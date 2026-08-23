//! What happens the moment a Conversation's own work is finished: the pull
//! request the session opened is found, and the Conversation moves into
//! Wrapping.
//!
//! Two endings arrive here, because two kinds of work end on a pull request. A
//! backlog worked to empty ends at its finish step, from Implementing. A roadmap
//! ends at the roadmap commit, from Grilling — the session that settled the work
//! wrote it and carried the branch on without ever leaving the grilling, and
//! there was no Implementing to leave, because on a roadmap the building belongs
//! to the Stages.
//!
//! The push and the pull request are the session's either way. It follows the
//! target repository's own review process — read out of that repository's
//! `docs/agents/git-workflow.md` by the bundled fork it is running inside —
//! because pushing and opening a PR is the project's process rather than
//! Verkstead's. What is Verkstead's is knowing that it happened, and the only way
//! to know is to ask GitHub: an agent's word for it would be the one report it
//! can most easily be wrong about.
//!
//! So this asks the host's `gh` for the PR on the Conversation's branch — see
//! [`crate::github`] — and records what it finds. Recording it *is* the move,
//! which is why nothing here sets a state: the store does both in one
//! transaction, so a Wrapping with no PR under it cannot exist.
//!
//! Nothing waits on approval. There is no gate in front of the finish and none
//! in front of the PR: merging stays the human act, and everything up to it runs
//! unattended.
//!
//! A `gh` that cannot answer — absent, not logged in, no PR on the branch —
//! leaves the Conversation where it is with the reason on the Timeline, as an
//! Interruption. That is the honest shape of it: the run has stopped, Verkstead
//! cannot resolve it, and what to do about it is the human's — install `gh`, log
//! in, open the PR by hand and retry.

use std::path::PathBuf;

use verkstead_schema::Nudge;

use crate::AppState;
use crate::github;
use crate::store;

/// Find the pull request `conversation_id`'s last session opened, and move the
/// Conversation on to wrapping it up.
///
/// `writing` is the Timeline Event that session printed into, so that an
/// Interruption raised here carries the tail of what it last said — which is
/// usually where the reason it opened nothing is written down.
///
/// Nothing is refused for and nothing is returned: this runs at the end of an
/// unattended run with nobody watching, and what it has to say it says on the
/// Timeline.
pub(crate) async fn opened(state: &AppState, conversation_id: i64, writing: Option<i64>) {
    let Some((repo, branch)) = branch(state, conversation_id).await else {
        return;
    };

    let asked = {
        let gh = state.github.clone();
        let branch = branch.clone();

        // Off the runtime's threads: this is a process, and one that goes to the
        // network.
        tokio::task::spawn_blocking(move || github::pull_request(&gh, &repo, &branch)).await
    };

    let found = match asked {
        Ok(found) => found,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "asking gh for a pull request failed");
            return;
        }
    };

    let opened = match found {
        Ok(opened) => opened,
        Err(trouble) => {
            tracing::warn!(
                conversation_id,
                branch,
                why = trouble.why(),
                "the last session left no pull request Verkstead could find",
            );

            stopped(state, conversation_id, &trouble.why(), writing).await;
            return;
        }
    };

    match store::record_pull_request(&state.pool, conversation_id, &opened).await {
        Ok(store::Wrapping::Started) => {
            tracing::info!(
                conversation_id,
                number = opened.number,
                url = opened.url,
                "the work is on a pull request, so the Conversation is wrapping up",
            );

            // The Timeline has a move on it and something new pinned above it, and
            // an open page should say so without being reloaded.
            state.nudges.announce(Nudge::Conversation {
                conversation: conversation_id,
            });

            // And the wrap-up itself starts here. The branch has just been
            // pushed, so GitHub is already running the checks and nobody else is
            // going to look — and nobody has read the branch at all.
            watching(state, conversation_id);
        }
        // The run was stopped from outside while the last step was landing, or
        // this is a second attempt at an ending that already moved the
        // Conversation. Neither is a failure and neither is anything to record
        // twice.
        Ok(store::Wrapping::NothingToWrap) => tracing::info!(
            conversation_id,
            number = opened.number,
            "the Conversation has nothing left to wrap up, so nothing was recorded",
        ),
        Ok(store::Wrapping::NoSuchConversation) => tracing::error!(
            conversation_id,
            "there is no Conversation left to record a pull request against"
        ),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "recording a pull request failed");

            stopped(
                state,
                conversation_id,
                &format!("the pull request could not be recorded: {error}"),
                writing,
            )
            .await;
        }
    }
}

/// Everything a wrapping Conversation has going on: its pull request's checks
/// watched, its comments read, its branch reviewed where nobody has read it yet,
/// and the rule that ends the whole thing waiting to be true.
///
/// One place that says what a wrap-up *is*, because four things start one and all
/// four have to start the whole of it: the finish step opening the pull request,
/// a server coming back up over a Conversation it left wrapping, and either of
/// the two Interruptions being retried — each of which stopped the rest too,
/// since nothing advances past an open one.
///
/// Each of them decides for itself whether there is anything to do, so starting
/// them twice is not starting two of anything: a review that has already asked
/// returns, a second settling watcher finds the move already made, and a
/// Conversation that has stopped wrapping up stops every one of them.
pub(crate) fn watching(state: &AppState, conversation_id: i64) {
    driving(state, conversation_id, crate::checks::watch);
    driving(state, conversation_id, crate::comments::watch);
    driving(state, conversation_id, crate::review::run);
    driving(state, conversation_id, crate::settling::watch);
}

/// Start one of them, registered as a driver of the Conversation for as long as
/// it runs.
///
/// The registration goes with the task rather than around the spawning, which
/// is the whole of what makes it worth a function: a wrap-up is driven while
/// any one of the four is still going, and each of them ends in its own time —
/// the review once it has asked, the rest once the Conversation stops wrapping
/// up. Counted rather than flagged, so a second set started over the top of the
/// first — which is what retrying either of the wrap-up's Interruptions does —
/// does not have the first of them to finish taking the Conversation off the
/// register. See [`crate::drivers`].
fn driving<W, F>(state: &AppState, conversation_id: i64, watcher: W)
where
    W: FnOnce(AppState, i64) -> F,
    F: Future<Output = ()> + Send + 'static,
{
    let driving = state.drivers.driving(conversation_id);
    let watching = watcher(state.clone(), conversation_id);

    tokio::spawn(async move {
        let _driving = driving;

        watching.await;
    });
}

/// Start watching every Conversation that was already wrapping up, and say when
/// it is done.
///
/// What a restarting server does. A pull request goes on being built while
/// Verkstead is down, and a review is a session that does not survive the process
/// that started it — so a server that came back up and watched nothing would
/// leave a Conversation wrapping for ever with nobody having said so.
///
/// The task is handed back rather than let go, because something waits on it:
/// the stall sweep judges a Conversation by whether anything is registered as
/// driving it, and every wrap-up taken up here registers as it is started — so a
/// sweep that looked first would call every one of them stalled. See
/// [`crate::stalls::sweeping`].
#[must_use = "the sweep waits for the wrap-ups to be taken up before it judges \
              whether anything is driving them"]
pub(crate) fn resume(state: &AppState) -> tokio::task::JoinHandle<()> {
    let state = state.clone();

    tokio::spawn(async move {
        let conversations = match store::conversations(&state.pool).await {
            Ok(conversations) => conversations,
            Err(error) => {
                tracing::error!(error = ?error, "listing the Conversations to resume wrapping up failed");
                return;
            }
        };

        for conversation in conversations
            .into_iter()
            .filter(|conversation| conversation.state == store::Lifecycle::Wrapping)
        {
            tracing::info!(
                conversation_id = conversation.id,
                "a Conversation was left wrapping up, so its wrap-up is taken up again",
            );

            watching(&state, conversation.id);
        }
    })
}

/// Leave the Conversation where it is, with the reason on the Timeline.
///
/// An Interruption rather than a line of its own, because that is exactly what
/// this is: a run that has stopped on something Verkstead cannot resolve itself.
/// The three remedies all mean something here — retry once `gh` is logged in,
/// take over and open the PR by hand, or abort the run.
async fn stopped(state: &AppState, conversation_id: i64, why: &str, writing: Option<i64>) {
    if let Err(error) = crate::interruptions::raise(
        state,
        conversation_id,
        store::Step::Finish,
        "finding the pull request the finish step opened",
        why,
        writing,
    )
    .await
    {
        tracing::error!(
            error = ?error,
            conversation_id,
            "a finish step left no pull request and the Interruption saying so could not be raised"
        );
    }
}

/// Which repository to ask `gh` in, and which branch to ask about.
///
/// The repository rather than the Worktree, exactly as the branch watcher asks
/// it: the remotes and the refs are the repository's, and a Worktree may have
/// been removed by the time this runs.
async fn branch(state: &AppState, conversation_id: i64) -> Option<(PathBuf, String)> {
    match store::load_conversation(&state.pool, conversation_id).await {
        Ok(Some(conversation)) => Some((conversation.repo.path, conversation.branch)),
        Ok(None) => {
            tracing::error!(
                conversation_id,
                "there is no Conversation left to find a pull request for"
            );
            None
        }
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading the Conversation to ask gh about failed");
            None
        }
    }
}
