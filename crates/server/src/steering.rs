//! Steer: the human saying where the work goes, from wherever it has got to.
//!
//! Two presses rather than one, and the gap between them is the point. Clicking
//! **Steer** stops the drive and answers with what it found running; the modal
//! then opens over a Conversation nothing is going to launch into while the
//! human composes. What they submit lands as the move.
//!
//! **The click stops it, and that is deliberate.** It is the ordinary Stop —
//! nothing new starts, and a session already running is seen out — so the world
//! the modal is drawn against is the world the submit arrives in. **Cancel
//! leaves the Conversation stopped**, with Resume on offer: the click is what
//! froze it, and unfreezing is a press of its own rather than something a
//! dismissed modal does behind the human's back.
//!
//! **Every state is a source.** Unlike every other move here, a steer answers to
//! no rung of the ladder: a draft nothing has run in, a run in flight and a
//! Conversation Verkstead has finished with are all somewhere to be steered
//! from. What the targets are is [`SteerTarget`]'s to say, and it is the short
//! list — the states work is *done in*, with Draft and Closed left out because
//! each has a way in of its own.
//!
//! **The record is two Events.** The Steer is the human's — *I moved this* —
//! and the machine's plain Moved line stands under it, which is the order the
//! moment happened in. See [`store::steer_conversation`], which writes both in
//! one transaction.
//!
//! Nothing is reverted, reset or stashed, here or anywhere a stop is written:
//! the Worktree is left exactly as whatever was running left it.

use verkstead_render::{ConversationSteered, SteerOpened, SteerSubmission, SteerTarget};
use verkstead_schema::Nudge;

use crate::AppState;
use crate::store::{self, Lifecycle};

/// Click Steer: stop the drive, and say what was running when it stopped.
///
/// The ordinary Stop, through the ordinary press — see [`crate::stops::stop`],
/// which writes the stop where nothing is running and records the request to
/// stop where something is. Its refusals are not this press's refusals: a
/// Conversation that has already stopped is already still, and one in a state
/// nothing drives never had a drive to stop. Both are a modal that opens.
///
/// What comes back is whether a session is still running, which is the one thing
/// the modal cannot work out for itself and the one thing **Interrupt current
/// task** is offered for.
pub(crate) async fn click(state: &AppState, conversation_id: i64) -> anyhow::Result<SteerOpened> {
    // Read back rather than trusted from the page, the way every press here
    // reads it back: this is the only thing that can say there is a
    // Conversation to steer at all.
    if store::load_conversation(&state.pool, conversation_id)
        .await?
        .is_none()
    {
        return Ok(SteerOpened::NoSuchConversation);
    }

    let stopped = crate::stops::stop(state, conversation_id).await?;

    // Asked after the stop rather than before it. A Stop pressed with nothing
    // running writes its stop where it stands and leaves nothing to see out, so
    // the checkbox is offered against what is running *now* — which is what the
    // submit a moment later will find.
    let working = state.sessions.working().contains(&conversation_id);

    tracing::info!(
        conversation_id,
        ?stopped,
        working,
        "the human is steering a Conversation, so the drive has stopped while they compose",
    );

    Ok(SteerOpened::Opened { working })
}

/// Submit the modal: move the Conversation where the human said, and record that
/// they said it.
///
/// In order: end what is running where **Interrupt current task** was ticked;
/// clear the stop the click left; move the Conversation and record the Steer;
/// and then start whatever the target needs starting.
///
/// The stop goes before the move rather than after, because what it is in front
/// of is the launch: a run does not advance past a stop — see
/// [`crate::stopping::stopped`] — so anything started behind one would find the
/// Conversation stopped and start nothing. Into Done there is nothing to start,
/// and the stop is cleared all the same: a Conversation Verkstead has finished
/// with wearing *blocked on you* would be a badge with no press to answer it.
pub(crate) async fn submit(
    state: &AppState,
    conversation_id: i64,
    submission: &SteerSubmission,
) -> anyhow::Result<ConversationSteered> {
    let Some(conversation) = store::load_conversation(&state.pool, conversation_id).await? else {
        return Ok(ConversationSteered::NoSuchConversation);
    };

    let target = target(submission.target);

    if submission.interrupt {
        // Ended rather than force-stopped, because the stop is already written:
        // the click wrote one, or recorded the request that the next launch
        // turns into one. A session Verkstead ended advances nothing — see
        // [`crate::sessions::Ended::on_purpose`] — so what was following it
        // reads its ending as the ending it is, and what it leaves in the
        // Worktree is left there.
        state.sessions.end(conversation_id).await;

        tracing::info!(
            conversation_id,
            "the steer's Interrupt was ticked, so the session running was ended where it stood",
        );
    }

    clear(state, conversation_id).await?;

    match store::steer_conversation(&state.pool, conversation_id, target).await? {
        store::Steering::NoSuchConversation => return Ok(ConversationSteered::NoSuchConversation),
        store::Steering::Steered => {}
    }

    // Nothing is launched: a steer into Done is the move alone, and the targets
    // that start something arrive with the tasks that build what each of them
    // starts.
    match submission.target {
        SteerTarget::Done => {}
    }

    state.nudges.announce(Nudge::Conversation {
        conversation: conversation_id,
    });

    tracing::info!(
        conversation_id,
        was = ?conversation.state,
        ?target,
        interrupted = submission.interrupt,
        "the human steered a Conversation",
    );

    Ok(ConversationSteered::Steered)
}

/// The state a target names.
///
/// The one place the modal's vocabulary and the record's are held to each other,
/// the way [`crate::ui`] holds the lifecycle's two spellings together — and the
/// reason the two are separate lists at all: every state is somewhere to steer
/// *from*, and only some of them are somewhere to steer *to*.
fn target(target: SteerTarget) -> Lifecycle {
    match target {
        SteerTarget::Done => Lifecycle::Done,
    }
}

/// Take the stop the click wrote away, along with any request to stop that has
/// not landed yet.
///
/// Both, for the reason a Resume clears both: the request is what the *next*
/// launch turns into a stop, so one left behind would stop the Conversation all
/// over again on the far side of the steer. See [`crate::resume`], which does
/// the same two things for the same reason.
///
/// Nothing to clear is an ordinary outcome. A steer from a state nothing drives
/// found nothing to stop at the click, and there is nothing here to take away.
async fn clear(state: &AppState, conversation_id: i64) -> anyhow::Result<()> {
    store::clear_stop(&state.pool, conversation_id).await?;
    store::forget_stop(&state.pool, conversation_id).await?;

    Ok(())
}
