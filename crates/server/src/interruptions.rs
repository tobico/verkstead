//! What the human does about an Interruption a Verkstead of before raised.
//!
//! Nothing raises one any more: where a run stops, it halts and a Notice says
//! why — see [`crate::halts`]. What is left here is the settling, and it is left
//! for the Interruptions already on Timelines, which still carry their card and
//! their three remedies until the migration rewrites them into Notices.
//!
//! [`settle`] runs whenever the human gets to it, which may be the next morning,
//! and acts on what they chose.
//!
//! Nothing here reverts, resets or stashes anything. In every case the repository
//! is left exactly as the session left it — that is what makes *take over
//! manually* a remedy at all, and it is why aborting from here does not remove
//! the Worktree the way aborting a Conversation from its menu does: the human is
//! being handed the wreckage on purpose.

use anyhow::Result;
use verkstead_render::{Remedy, RemedySettled};
use verkstead_schema::Nudge;

use crate::AppState;
use crate::store;

/// Take the human's remedy: record it, then do it.
///
/// Recorded before anything acts on it, for the reason a worktree is recorded
/// before a session is launched in it — an Interruption acted on without being
/// closed is one the human could answer twice, and a retry pressed twice is two
/// agents in one Worktree. [`RemedySettled::AlreadySettled`] is therefore an
/// ordinary answer rather than an error: the second press of a button is the
/// first choice arriving again.
///
/// Only the closing is refused for. What the remedy *does* — a session that would
/// not start, a Conversation that would not move — is logged and left on the
/// record, because by the time it runs the Interruption is closed and answering
/// the button with a failure would say that none of it happened.
pub(crate) async fn settle(
    state: &AppState,
    conversation_id: i64,
    event_id: i64,
    remedy: Remedy,
    note: &str,
) -> Result<RemedySettled> {
    // Read before it is closed, because the step is what a retry launches again
    // and a settled row is not the one to read it off.
    let step = match store::interruption(&state.pool, conversation_id, event_id).await? {
        Some(interruption) => interruption.evidence.step,
        None => return Ok(RemedySettled::NoSuchInterruption),
    };

    let settling =
        store::settle_interruption(&state.pool, conversation_id, event_id, stored(remedy), note)
            .await?;

    match settling {
        store::Settling::NoSuchInterruption => return Ok(RemedySettled::NoSuchInterruption),
        store::Settling::AlreadySettled => return Ok(RemedySettled::AlreadySettled),
        store::Settling::Settled => {}
    }

    tracing::info!(
        conversation_id,
        event_id,
        remedy = ?remedy,
        step = ?step,
        "an Interruption was settled"
    );

    match remedy {
        // Handed straight back to the runner, which is where a run is driven from
        // whether it is starting or picking up: what step to launch has to be
        // settled before the session is, and the runner is what settles it.
        Remedy::Retry => {
            tokio::spawn(crate::runner::retry(
                state.clone(),
                conversation_id,
                step,
                note.to_owned(),
            ));
        }
        // Nothing to do but stop, which is what settling already did: the run was
        // not advancing, and now nothing will restart it. The Worktree stays
        // exactly as the session left it — which is the whole of what taking over
        // manually needs.
        Remedy::TakeOver => tracing::info!(
            conversation_id,
            "Verkstead has stopped driving; the Worktree is the human's"
        ),
        Remedy::Abort => abort(state, conversation_id).await?,
    }

    Ok(RemedySettled::Settled)
}

/// The store's word for a remedy. One word either side, and this is where the two
/// vocabularies are held to each other.
fn stored(remedy: Remedy) -> store::Remedy {
    match remedy {
        Remedy::Retry => store::Remedy::Retry,
        Remedy::TakeOver => store::Remedy::TakeOver,
        Remedy::Abort => store::Remedy::Abort,
    }
}

/// End the run here: the Conversation stops wherever it had got to.
///
/// Unlike aborting from the Conversation's own menu, this leaves the Worktree
/// exactly where it is. Everything about an Interruption is built on the
/// repository being left as the session left it, and a remedy that swept the
/// evidence away as it ended the run would be the one thing the human could not
/// undo.
///
/// No session is ended, because there is none to end: an Interruption is raised
/// by a session that has already gone.
async fn abort(state: &AppState, conversation_id: i64) -> Result<()> {
    match store::abort_conversation(&state.pool, conversation_id).await? {
        store::Aborting::Aborted => tracing::info!(
            conversation_id,
            "the run was ended at an Interruption; the Worktree is left as the session left it"
        ),
        // Somebody aborted it from the menu between the Interruption being raised
        // and this being pressed, which is the same thing arriving twice.
        store::Aborting::AlreadyAborted => tracing::info!(
            conversation_id,
            "the run had already been ended when the Interruption was aborted"
        ),
        store::Aborting::NoSuchConversation => tracing::error!(
            conversation_id,
            "there is no Conversation left to end the run of"
        ),
    }

    state.nudges.announce(Nudge::Conversation {
        conversation: conversation_id,
    });

    Ok(())
}
