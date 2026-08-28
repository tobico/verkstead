//! What happens the moment a Conversation's own work is finished: the pull
//! request the session opened is found, and the Conversation moves into
//! Wrapping.
//!
//! Three endings arrive here, because three kinds of work end on a pull request.
//! A backlog worked to empty ends at its finish step, from Implementing. An
//! inline implementation ends at its one session, from Implementing too — the
//! whole of the work was that session's, so what ends it is the session ending.
//! A roadmap ends at the roadmap commit, from Grilling — the session that
//! settled the work wrote it and carried the branch on without ever leaving the
//! grilling, and there was no Implementing to leave, because on a roadmap the
//! building belongs to the Stages.
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
//! stops, leaving the Conversation where it is with the reason on the Timeline
//! as a Notice. That is the honest shape of it: the run has stopped, Verkstead
//! cannot resolve it, and what to do about it is the human's — install `gh`, log
//! in, or open the PR by hand, and resume.
//!
//! With one thing tried first, and only for the one answer a session could do
//! anything about. Every run that ends here commits its work and pushes it
//! afterwards, so any of them can land the whole of what it was sent for and
//! still stop short of the push — which leaves the work built, committed and
//! unreviewable. So *no PR on the branch* is asked for once more, by a session of
//! its own sent to push and open one, and the stop is what is left if that comes
//! back to the same missing thing. A pressed Resume takes the same go rather than
//! the Notice again. See [`crate::runner`], which is where the deciding is done,
//! and [`record`], which is this module's half of it. The other troubles are
//! walls a session would walk into in the same place, so they stop as they always
//! have.
//!
//! Which is advice Resume then has to be able to take: a pull request opened in
//! a browser is one nothing on the branch knows about, so Resume asks GitHub
//! about it before it spends anything — see [`asked`], and [`crate::runner`] for
//! what an inline run makes of the answer.

use std::path::PathBuf;

use verkstead_schema::Nudge;

use crate::AppState;
use crate::github;
use crate::store;

/// Find the pull request `conversation_id`'s last session opened, and move the
/// Conversation on to wrapping it up.
///
/// `writing` is the Timeline Event that session printed into, so that a stop
/// written here carries the tail of what it last said — which is usually where
/// the reason it opened nothing is written down.
///
/// Nothing is refused for and nothing is returned: this runs at the end of an
/// unattended run with nobody watching, and what it has to say it says on the
/// Timeline.
pub(crate) async fn opened(state: &AppState, conversation_id: i64, writing: Option<i64>) {
    let Some((branch, found)) = asked(state, conversation_id).await else {
        return;
    };

    record(state, conversation_id, &branch, found, writing).await
}

/// Make of an answer `gh` has already given what [`opened`] makes of its own.
///
/// The same move with the asking taken out of it, for the one caller that has to
/// look at the answer before this does: a finish step that left no pull request
/// is sent back to open one, and only a branch GitHub says has none is worth
/// spending a session on — see [`crate::runner`]. Everything else it may say is
/// this function's to record or to stop over, exactly as it always was, so the
/// deciding stays in one place and the caller hands back what it was given.
///
/// `branch` is the branch that was asked about, which is what the stop is logged
/// against.
pub(crate) async fn record(
    state: &AppState,
    conversation_id: i64,
    branch: &str,
    found: Result<store::PullRequest, github::Trouble>,
    writing: Option<i64>,
) {
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

            // And the devices are told, because this is one of the moments the
            // work moved on with nobody watching: the whole run from the Brief
            // to here happened unattended, and the pull request is where the
            // human's own part of it starts. Behind the record, which the store
            // has already taken — a push service that cannot be reached costs a
            // notification and never the pull request.
            crate::push::told(
                &state.pool,
                conversation_id,
                crate::push::News::OnAPullRequest {
                    number: opened.number,
                },
            );

            // And the wrap-up itself starts here. The branch has just been
            // pushed, so GitHub is already running the checks and nobody else is
            // going to look — and nobody has read the branch at all.
            watching(state, conversation_id, Reviewing::AsFound);
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

            // `{error:#}` rather than `{error}`, because what a Notice is for is
            // the human working out what to do about it. Displayed plainly, an
            // anyhow error is its outermost context and nothing else — *putting
            // a pull request on the Timeline of Conversation 43* — which names
            // the step and withholds the reason it failed. The alternate form
            // carries the chain down to what the database actually said.
            stopped(
                state,
                conversation_id,
                &format!("the pull request could not be recorded: {error:#}"),
                writing,
            )
            .await;
        }
    }
}

/// Which review a wrap-up's watchers start.
///
/// The one thing that differs between the two ways of starting a wrap-up's
/// watchers, and it differs because a press is the human saying something. The
/// other three watchers each read the record and decide for themselves either
/// way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Reviewing {
    /// Whatever there is to see to: a branch nobody has read, or a review whose
    /// session is gone, which is a stop. What the finish step and a server
    /// coming back up both mean — see [`crate::review::run`].
    AsFound,

    /// The branch read from the start, whatever the last review left behind.
    /// What a press means — see [`crate::review::afresh`].
    Afresh,
}

/// Everything a wrapping Conversation has going on: its pull request's checks
/// watched, its comments read, its branch reviewed where nobody has read it yet,
/// and the rule that ends the whole thing waiting to be true.
///
/// One place that says what a wrap-up *is*, because everything that starts one
/// has to start the whole of it: the finish step opening the pull request, a
/// server coming back up over a Conversation it left wrapping, and a Resume
/// pressed on a wrap-up that stopped — which stopped the rest too, since nothing
/// advances past an open one. See [`crate::resume`] for the last two.
///
/// Each of them decides for itself whether there is anything to do, so starting
/// them twice is not starting two of anything: a review that has already settled
/// returns, a second of anything queues on the Worktree behind the first and
/// finds the work done, and a Conversation that has stopped wrapping up stops
/// every one of them.
///
/// Which is also why a restart and a Resume both come through here rather than
/// picking their own step. What either of them is looking at is a wrap-up with
/// nothing running, and that is one situation with several possible causes: a
/// branch nobody has read, a review whose session went between its ask and the
/// answers, a batch's proposal in the same state. Each of the four asks the
/// record what it is looking at rather than being told — see
/// [`crate::review::run`] and [`crate::responding::unattended`].
///
/// `reviewing` is the one thing the two ways of starting them differ over, and
/// it is the human's press that makes the difference: a review already asking is
/// something to stop over where a server found it, and something to read past
/// where they have read the Notice and asked for another go.
pub(crate) fn watching(state: &AppState, conversation_id: i64, reviewing: Reviewing) {
    driving(state, conversation_id, crate::checks::watch);
    driving(state, conversation_id, crate::comments::watch);

    match reviewing {
        Reviewing::AsFound => driving(state, conversation_id, crate::review::run),
        Reviewing::Afresh => driving(state, conversation_id, crate::review::afresh),
    }

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
/// first — which is what Resume on a stopped wrap-up does — does not have the
/// first of them to finish taking the Conversation off the register. See
/// [`crate::drivers`].
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

/// Whether the Conversation is still wrapping up, which is the only state
/// anything a wrap-up dispatches belongs to.
///
/// Asked on the far side of every wait for the Worktree: a Conversation closed
/// while something queued has nowhere left to work, and a queue is where most of
/// a wrap-up's waiting happens.
///
/// A store that will not answer reads as *no*, which is the right way round for
/// the one thing this decides: on the other side of it is an agent being let
/// loose in a Worktree.
pub(crate) async fn still_going(state: &AppState, conversation_id: i64) -> bool {
    match store::load_conversation(&state.pool, conversation_id).await {
        Ok(Some(conversation)) => conversation.state == store::Lifecycle::Wrapping,
        Ok(None) => {
            tracing::error!(conversation_id, "there is no Conversation left wrapping up");
            false
        }
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading whether a Conversation was wrapping up failed");
            false
        }
    }
}

/// Leave the Conversation where it is, with the reason on the Timeline.
///
/// A stop rather than a line of its own, because that is exactly what this is: a
/// run that has stopped on something Verkstead cannot resolve itself. Resume
/// once `gh` is logged in, or open the pull request by hand, or close the
/// Conversation.
///
/// [`store::Decision::Deliberate`]: the work ran and left no pull request, so what
/// is wrong is out here rather than in a driver that went away, and a restart
/// looking again would find the same missing thing.
///
/// Which is also why Resume on an inline implementation stops here rather than
/// launching over it — see [`crate::runner`]. A `gh` that cannot answer now is
/// one the session would push into later, so the account it would spend is
/// spent on the same missing thing.
pub(crate) async fn stopped(
    state: &AppState,
    conversation_id: i64,
    why: &str,
    writing: Option<i64>,
) {
    if let Err(error) = crate::stopping::stop(
        &state.pool,
        &state.nudges,
        conversation_id,
        crate::stopping::Decided::Verkstead,
        "finding the pull request the work ended on",
        why,
        writing,
    )
    .await
    {
        tracing::error!(
            error = ?error,
            conversation_id,
            "a finish step left no pull request and the stop saying so could not be recorded"
        );
    }
}

/// Ask the host's `gh` what pull request `conversation_id`'s branch has, and
/// hand back the branch it asked about alongside the answer.
///
/// Whether there is one is a question two phases ask. [`opened`] asks it at the
/// end of a run, where a pull request is what the work was carried to, and
/// Resume on an inline implementation asks it before spending a session,
/// because a branch that is already on a pull request has nothing left to
/// implement — see [`crate::runner`]. What each makes of the answer is its own,
/// so the answer is what this hands back rather than anything it does about it.
///
/// `None` where there was nothing to ask about or the asking itself fell over,
/// both of which are in the log already: neither leaves a caller anything to
/// say.
pub(crate) async fn asked(
    state: &AppState,
    conversation_id: i64,
) -> Option<(String, Result<store::PullRequest, github::Trouble>)> {
    let (repo, branch) = branch(state, conversation_id).await?;

    let asked = {
        let gh = state.github.clone();
        let branch = branch.clone();

        // Off the runtime's threads: this is a process, and one that goes to the
        // network.
        tokio::task::spawn_blocking(move || github::pull_request(&gh, &repo, &branch)).await
    };

    match asked {
        Ok(found) => Some((branch, found)),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "asking gh for a pull request failed");
            None
        }
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
