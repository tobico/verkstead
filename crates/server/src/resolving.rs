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
/// The refusals are asked by the store, inside the transaction that makes the
/// move — which is what keeps two presses a moment apart from both moving one
/// Conversation, and what makes *nothing conflicts* an answer about the record
/// rather than about a reading taken beside it.
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
