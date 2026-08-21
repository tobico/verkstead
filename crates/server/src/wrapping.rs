//! What happens the moment a backlog is worked to empty: the pull request the
//! finish step opened is found, and the Conversation moves into Wrapping.
//!
//! The finish itself is the session's. It follows the target repository's own
//! review process — read out of that repository's `docs/agents/git-workflow.md`
//! by the bundled fork of next-task — because pushing and opening a PR is the
//! project's process rather than Verkstead's. What is Verkstead's is knowing
//! that it happened, and the only way to know is to ask GitHub: an agent's word
//! for it would be the one report it can most easily be wrong about.
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

use crate::AppState;
use crate::github;
use crate::store;

/// Find the pull request `conversation_id`'s finish step opened, and move the
/// Conversation on to wrapping it up.
///
/// `writing` is the Timeline Event the finish session printed into, so that an
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
                "the finish step left no pull request Verkstead could find",
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
            state.nudges.announce();

            // And the wrap-up itself starts here, which for now is the checks:
            // the branch has just been pushed, so GitHub is already running them
            // and nobody else is going to look.
            tokio::spawn(crate::checks::watch(state.clone(), conversation_id));
        }
        // The run was stopped from outside while the finish step was landing, or
        // this is a second attempt at a finish that already moved the
        // Conversation. Neither is a failure and neither is anything to record
        // twice.
        Ok(store::Wrapping::NotImplementing) => tracing::info!(
            conversation_id,
            number = opened.number,
            "the Conversation is not implementing any more, so nothing was recorded",
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
