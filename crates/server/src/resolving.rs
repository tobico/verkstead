//! The press that gets a finished Conversation's conflict resolved.
//!
//! A wrap-up ends when GitHub can merge every pull request the work is on. What
//! it cannot end is the base going on moving afterwards: a branch that merged
//! cleanly at Done conflicts a week later without anybody having touched it, and
//! the sweep that watches for that writes the fact down and dispatches nothing —
//! a conflict on work Verkstead has finished with is the human's to decide about.
//! See [`crate::merges`].
//!
//! This is them deciding. One press on the pull request's details pane, offered
//! only while the recorded fact says the branch conflicts, and what it does is
//! put the Conversation back into Wrapping so that the machine has another go at
//! it: the resolution session, by the same two-goes-and-a-Notice rules a wrap-up
//! resolves a conflict under — see [`crate::checks`].
//!
//! **It is a move of its own rather than a steer.** A steer into Wrapping
//! deliberately reads the branch again: it is the human saying *look at this
//! afresh*, so the review's settle goes and a review session runs. This is not
//! that. The work was reviewed, the human read the review, and the Conversation
//! was carried to Done on the strength of it — and a base that moved underneath
//! it since is not a reason to read the same branch a second time. So the
//! review's settle is left standing, and the wrap-up that starts here finds it
//! settled and runs nothing for it.
//!
//! **What does go back to waiting is the merge**, on the pull requests the
//! record says conflict. Which is the store's own doing, in the same transaction
//! as the move — see [`store::resolve_conflicts`], where the reason is written
//! down: a settle left standing over a conflict would be a wrap-up that reached
//! Done again on the first turn of its settling loop, before its watchers had
//! asked GitHub anything.
//!
//! **And the goes are forgotten**, both counts of them, exactly as Resume
//! forgets them: the human has read the record and asked for another round, and
//! a count left standing would be a watcher that stopped all over again on its
//! next poll without dispatching anything. See [`store::forget_fix_attempts`],
//! which is the same forgetting from the same reasoning.
//!
//! **And the checkout is seen to before any of it**, which is the one thing
//! this press has to do that the rest of the wrap-up does not. Every other way
//! into a Worktree is a way into one something was working in minutes ago; this
//! one is a way into one nothing has touched since the work was finished with,
//! which may be weeks. So it is read and made again where it has gone, exactly
//! as [`crate::resume`] reads and makes one — a worktree being derived state —
//! and the press refuses rather than moving where it cannot be made. A
//! resolution session dispatched at a directory that is not there would spend
//! both of the pull request's goes on a sandbox nothing could build, and stop
//! the run with a Notice blaming the conflict.
//!
//! The watchers then go on as found. There is nothing here that decides what
//! runs: each of them reads the record for itself a moment later, which is what
//! makes this press the same wrap-up every other way into one starts — the
//! review settled and silent, the checks watched, the comments read, and the
//! conflict dispatched at because GitHub says the branch will not merge.
//!
//! Nothing is pushed to a device for it. Every other thing that reaches for a
//! phone is Verkstead telling somebody about a moment they were not there for;
//! this is a button they just pressed.

use verkstead_render::Resolved;
use verkstead_schema::Nudge;

use crate::AppState;
use crate::store;

/// Press **Resolve conflicts**: send a Done Conversation back to its wrap-up,
/// and start the watchers that will resolve the conflict.
///
/// The refusals about the record are asked by the store, inside the transaction
/// that makes the move — which is what keeps two presses a moment apart from
/// both moving one Conversation, and what makes *nothing conflicts* an answer
/// about the record rather than about a reading taken beside it.
///
/// The refusals about the checkout are asked here, before it: what they are
/// about is a directory rather than a row, and a Conversation moved into
/// Wrapping over a Worktree nothing could work in would be one left standing
/// there for the stall sweep to stop. So the checkout is seen to first and the
/// move is made over one that is there.
///
/// The registration is taken before the move and held across the spawn, for
/// [`crate::resume`]'s reason: a Conversation that says Wrapping with nothing on
/// the drivers' register is one the stall sweep reads as standing still and
/// stops. It is handed on to the watchers, each of which takes one of its own as
/// it is spawned.
pub(crate) async fn resolve(state: &AppState, conversation_id: i64) -> anyhow::Result<Resolved> {
    // Taken before the move rather than after it, and dropped by the early
    // returns on every path that refuses.
    let driving = state.drivers.driving(conversation_id);

    let Some(conversation) = store::load_conversation(&state.pool, conversation_id).await? else {
        return Ok(Resolved::NoSuchConversation);
    };

    // Asked here as well as in the transaction below, and cheaply: what follows
    // it is a checkout of somebody's repository, and making one for a
    // Conversation the move is about to refuse would be slow work done for
    // nothing. The store's own answer is still the one that decides — see the
    // match below, which is what two presses a moment apart race on.
    if conversation.state != store::Lifecycle::Done {
        return Ok(Resolved::NotDone);
    }

    // And nothing is sent back to a wrap-up on a build that runs no sessions:
    // what this press is for is the resolution session the watchers dispatch,
    // and there is none to dispatch here. In front of the checkout below for the
    // reason the state is: what follows is somebody's repository being worked
    // on for a move that is about to be refused. See [`crate::sessions::run_on`].
    if state.sessions.here().absent() {
        return Ok(Resolved::NotOnWindowsYet);
    }

    // Every state past drafting has a Worktree, so one missing from the record
    // is a record that cannot be true: there is nowhere for the resolution
    // session to work and nothing to make a checkout from either, the path being
    // Verkstead's own to have chosen.
    let Some(worktree) = conversation.worktree.clone() else {
        return Ok(Resolved::NowhereToWork);
    };

    // The branch first, which may not be called what the record says — see
    // [`crate::renames`], and [`crate::resume`], which reads it here for the
    // same reason: the sweep that follows a rename runs only while a session
    // does, so a rename nothing saw would have the checkout below read as broken
    // and rebuilt from a branch that is not there.
    let branch = crate::renames::follow(
        &state.pool,
        conversation_id,
        &conversation.repo.path,
        &worktree,
        &conversation.branch,
    )
    .await
    .unwrap_or_else(|| conversation.branch.clone());

    // And then the checkout itself, because this press is the one way into a
    // wrap-up that starts work in a Worktree nothing has touched for weeks. A
    // Conversation stays Done for as long as nobody merges its pull request, and
    // in that time a directory is deleted, hollowed out or dropped from the
    // repository's list of worktrees — and a resolution session dispatched at a
    // path that is not there spends a go on a sandbox that cannot be built, then
    // spends the other, and stops the run with a Notice blaming the conflict.
    //
    // So it is made again from the branch, exactly as Resume makes it and for
    // Resume's reason: a worktree is derived state. Nothing healthy is touched,
    // uncommitted changes and all — see [`crate::worktrees::healthy`], which is
    // what answers on nearly every press and costs a git read.
    //
    // The Conversation's own alone, as Resume rebuilds its own alone: a
    // companion whose directory has gone is a sandbox no session of any kind
    // could be built for, and that is the same everywhere rather than this
    // press's to answer.
    //
    // Off the runtime's threads, a checkout of a large repository being no quick
    // call, and under the registration taken above, a Conversation being rebuilt
    // being a Conversation being driven.
    let usable = tokio::task::spawn_blocking({
        let repo = conversation.repo.path.clone();
        let worktree = worktree.clone();

        move || {
            crate::worktrees::healthy(&repo, &worktree, &branch)
                || crate::worktrees::rebuild(&repo, &worktree, &branch)
        }
    })
    .await?;

    if !usable {
        return Ok(Resolved::WorktreeRefused);
    }

    match store::resolve_conflicts(&state.pool, conversation_id).await? {
        store::Resolving::NoSuchConversation => return Ok(Resolved::NoSuchConversation),
        store::Resolving::NotDone => return Ok(Resolved::NotDone),
        store::Resolving::NothingConflicts => return Ok(Resolved::NothingConflicts),
        store::Resolving::Wrapping => {}
    }

    // Both counts, because either of them is what the last round of this wrap-up
    // spent: a Conversation that reached Done had its checks go green and its
    // conflicts resolved, and one that has been through this press before has
    // spent goes on the conflict itself.
    if let Err(error) = store::forget_fix_attempts(&state.pool, conversation_id).await {
        tracing::error!(error = ?error, conversation_id, "forgetting what a finished Conversation's conflicts had been given failed");
    }

    tracing::info!(
        conversation_id,
        "the human asked for a finished Conversation's conflict to be resolved, so it is \
         wrapping up again from no attempts spent",
    );

    // As found rather than afresh, which is the whole of what makes this press
    // different from a steer into Wrapping: the review is settled, so
    // [`crate::review::run`] reads that and launches nothing.
    //
    // Each of the watchers takes a registration of its own as it is spawned,
    // which is the handover this one is held across.
    crate::wrapping::watching(state, conversation_id, crate::wrapping::Reviewing::AsFound);
    drop(driving);

    // The Timeline has the press and the move on it, and the sidebar row has a
    // state that has changed — so every open page reads both again.
    state.nudges.announce(Nudge::Conversation {
        conversation: conversation_id,
    });

    Ok(Resolved::Resolving)
}
