//! The sweep that keeps watching a pull request after the work on it is Done.
//!
//! A wrap-up watches its pull requests every half minute and stops the moment
//! the Conversation reaches Done — see [`crate::checks`]. What is left after
//! that is a branch sitting on GitHub waiting for the human to merge it, and a
//! base that goes on moving under it: a pull request that merged cleanly at Done
//! conflicts a day later without anybody having touched it, and nothing would
//! notice.
//!
//! So this sweep does, on a pace of its own. Every [`crate::Pace::merges`] it
//! walks every Done Conversation's pull requests that nothing has recorded
//! merged or closed, and asks the host's `gh` two things about each: whether it
//! still merges, and where it has got to. Both land in the store beside what the
//! wrap-up's own watcher wrote there — see [`crate::store::record_merging`] and
//! [`crate::store::record_standing`] — so what a card draws about a Done
//! Conversation goes on being the last thing anybody asked GitHub, however long
//! ago the work finished.
//!
//! **Done alone.** A Conversation still Wrapping has a watcher asking this every
//! half minute already, and a Closed one is the human finished with the work —
//! which takes an Archived one with it, archiving being a Closed Conversation
//! off the sidebar rather than a state of its own. See
//! [`crate::store::unfinished_pull_requests`], which is where that is decided.
//!
//! **And it ends per pull request rather than all at once.** A pull request
//! recorded merged or closed is an answer that will not change, so it drops out
//! of the walk the moment one is recorded and is never asked about again. That
//! is learned from the same call that watches for the conflict, which is what
//! makes the ending free: a sweep that had to ask a second question to find out
//! whether to stop asking would be two calls for every one it saved.
//!
//! **Nothing is dispatched from here and nothing moves.** After Done this is
//! watching and nothing else: no session, no Notice, no push to a device. A
//! conflict on a pull request whose Conversation is finished is the human's to
//! decide about — Verkstead's part is that the fact is there to be drawn when
//! they look. The one thing that goes out is a Nudge on the Conversation where
//! the word changed, which is a page already open being told to read again
//! rather than anybody being told anything.
//!
//! A `gh` that cannot answer changes nothing at all, exactly as it changes
//! nothing for a wrap-up: *Verkstead does not know* is a third thing beside
//! merges and conflicts, and the next sweep asks again of a `gh` that may by
//! then have been logged in.
//!
//! **Opening the details pane sweeps that one pull request too.** It asks GitHub
//! about the pull request on its way to listing the checks, and both readings
//! ride that question — so a human who opens the pane freshens what is written
//! down here in the same act, whatever state the Conversation is in and whether
//! or not anything is sweeping it. See [`crate::github::details`] and
//! [`remember`].

use std::time::Duration;

use verkstead_schema::Nudge;

use crate::AppState;
use crate::github::{Landing, Mergeable, Stands};
use crate::store;

/// How often a Done Conversation's pull requests are asked about, as
/// [`crate::Pace`] has it by default.
///
/// Fifteen minutes, which is thirty times slower than a wrap-up's own watcher
/// and deliberately so: what this is watching for is a base moving under a
/// branch nobody is working on, and noticing one a quarter of an hour late costs
/// nothing at all. Nothing here is dispatched off the answer, so there is
/// nothing that being slow delays.
pub(crate) const SWEPT_EVERY: Duration = Duration::from_secs(15 * 60);

/// Sweep every Done Conversation's pull requests from now until the process
/// stops: once at startup, and every [`crate::Pace::merges`] after that.
///
/// At startup rather than after anything, unlike [`crate::stalls::sweeping`]:
/// what this looks at is work that is already finished, so there is nothing a
/// resume could be in the middle of putting right and nothing to wait for.
///
/// And never, on a server that runs no sessions — the stall sweep's own reason,
/// which is that only the tests' routers are built that way: a fixture standing a
/// router up over a store held still would otherwise go to somebody's network
/// about whatever pull requests it had written into it.
pub(crate) fn sweeping(state: &AppState) {
    if !state.sessions.runs_sessions() {
        return;
    }

    let state = state.clone();

    tokio::spawn(async move {
        loop {
            sweep(&state).await;

            tokio::time::sleep(state.sessions.pace().merges).await;
        }
    });
}

/// One look over every pull request there is still something to ask about.
///
/// Nothing is refused for and nothing is returned. This runs unattended with
/// nobody watching, and what it has to say it says in the log — there being
/// nothing here worth a line on a Timeline, the record itself being what it
/// writes.
async fn sweep(state: &AppState) {
    let unfinished = match store::unfinished_pull_requests(&state.pool).await {
        Ok(unfinished) => unfinished,
        Err(error) => {
            tracing::error!(error = ?error, "listing the pull requests still waiting to land failed");
            return;
        }
    };

    for pull_request in unfinished {
        let asked = {
            let gh = state.github.clone();
            let repo = pull_request.repo.path.clone();
            let number = pull_request.number;

            // Off the runtime's threads: this is a process, and one that goes to
            // the network.
            tokio::task::spawn_blocking(move || crate::github::landing(&gh, &repo, number)).await
        };

        let landing = match asked {
            Ok(Ok(landing)) => landing,
            // GitHub could not be asked, so nothing is concluded and nothing is
            // written down. The next sweep asks again.
            Ok(Err(trouble)) => {
                tracing::warn!(
                    conversation_id = pull_request.conversation_id,
                    repo = pull_request.repo.name,
                    number = pull_request.number,
                    why = trouble.why(),
                    "a pull request that has not landed could not be asked about",
                );

                continue;
            }
            Err(error) => {
                tracing::error!(error = ?error, conversation_id = pull_request.conversation_id, "asking gh about a pull request that has not landed failed");
                continue;
            }
        };

        remember(
            state,
            pull_request.conversation_id,
            &pull_request.repo,
            landing,
        )
        .await;
    }
}

/// Write down what one look at a pull request found: whether it merges, and
/// where it has got to.
///
/// The sweep's, and the details pane's too — the pane asks GitHub the same two
/// things on its way to listing the checks, so it is what freshens both on a
/// pull request nothing is sweeping. Which is [`crate::checks::remember`]'s
/// arrangement exactly, one fact along.
///
/// **Only what GitHub actually said.** A `mergeable` it has not worked out yet
/// and a `state` in a word this does not know are each *not known*, and not
/// knowing is written down nowhere: what stands is the last thing GitHub did
/// say. The second matters more than the first, because a standing is what ends
/// the asking — a word read as an ending in error would be a pull request nobody
/// ever looked at again.
pub(crate) async fn remember(
    state: &AppState,
    conversation_id: i64,
    repo: &store::Repo,
    landing: Landing,
) {
    let merging = match landing.mergeable {
        Mergeable::Cleanly => Some(store::Merging::Cleanly),
        Mergeable::Conflicting => Some(store::Merging::Conflicting),
        Mergeable::Unknown => None,
    };

    // Nudged where the word changed, exactly as the wrap-up's own watcher does
    // it: the card draws a mark off this, and a page open on a Done Conversation
    // is how a conflict that appeared after the work finished catches the eye.
    // A sweep that found the same word a quarter of an hour later has nothing to
    // tell anybody.
    if let Some(merging) = merging {
        match store::record_merging(&state.pool, conversation_id, repo.id, merging).await {
            Ok(true) => state.nudges.announce(Nudge::Conversation {
                conversation: conversation_id,
            }),
            Ok(false) => {}
            Err(error) => {
                tracing::error!(error = ?error, conversation_id, repo = repo.name, "recording whether a pull request that has not landed merges failed");
            }
        }
    }

    let standing = match landing.stands {
        Stands::Open => Some(store::Standing::Open),
        Stands::Merged => Some(store::Standing::Merged),
        Stands::Closed => Some(store::Standing::Closed),
        Stands::Unknown => None,
    };

    let Some(standing) = standing else {
        return;
    };

    if let Err(error) =
        store::record_standing(&state.pool, conversation_id, repo.id, standing).await
    {
        tracing::error!(error = ?error, conversation_id, repo = repo.name, "recording where a pull request has got to failed");
        return;
    }

    if standing != store::Standing::Open {
        tracing::info!(
            conversation_id,
            repo = repo.name,
            standing = ?standing,
            "a pull request has been finished with, so nothing asks about it again",
        );
    }
}
