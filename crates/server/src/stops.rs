//! The human stopping a Conversation on purpose: the two presses beside Abort,
//! and what each of them costs.
//!
//! Resume's opposite number, and shaped the same way — one act, recomputed from
//! where the work now stands, that either does something or refuses by name. The
//! difference between the two presses is only what they are willing to wait for:
//!
//! - **Stop** pauses after the current task. Nothing new is started, whatever is
//!   running now runs to its own end, and the Conversation stops before the next
//!   launch. Pressed with nothing running, there is nothing to see out and it
//!   stops where it stands.
//! - **Force stop** ends what is running now and stops at once. The step is left
//!   wherever the session had got to, uncommitted work and all, because that is
//!   what stopping immediately means.
//!
//! Both are **deliberate** — see [`crate::stopping::Decided`] — so neither
//! is one a restart picks up unasked, and neither reaches a phone: the human is
//! the one person a notification about their own press would be telling nothing.
//! Both are undone by the one Resume, which recomputes what ought to be running
//! and starts it.
//!
//! Nothing here touches the Worktree, the branch, or a Question Set left
//! standing. A stop is driving stopping rather than work being thrown away —
//! that is Abort, which is a different press and says so.

use anyhow::Result;
use verkstead_render::ConversationStopped;

use crate::AppState;
use crate::stopping::Decided;
use crate::store::{self, Lifecycle};

/// Why the Notice says the run stopped, for each way of pressing.
///
/// The human's own words back to them, because that is the whole of the reason:
/// nothing failed, nothing was decided about the work, somebody pressed a
/// button. Written as the *how* half of a stop's Notice — see
/// [`crate::stopping::stop`].
const AFTER_THE_STEP: &str = "you pressed Stop, so the session that was running \
     was carried to its end and nothing was started after it";
const NOTHING_RUNNING: &str =
    "you pressed Stop, and nothing was running to see out, so it stopped where it stood";
const AT_ONCE: &str = "you pressed Force stop, so what was running was ended where it stood";

/// Press Stop: pause after the current task.
///
/// Where a session is running, that session is left alone and the stop is
/// recorded for the run to find at its next launch — see [`asked`], which is
/// where it lands. Where none is, there is nothing to see out and the stop is
/// written now.
///
/// The session is what is asked about rather than the register of drivers, and
/// deliberately: a run between steps has a driver and no session, and what Stop
/// promises there is a Conversation that has already stopped rather than one
/// that stops when a poll loop next gets round to it.
pub(crate) async fn stop(state: &AppState, conversation_id: i64) -> Result<ConversationStopped> {
    let Some(standing) = standing(state, conversation_id).await? else {
        return Ok(ConversationStopped::NoSuchConversation);
    };

    let lifecycle = match standing {
        Standing::Driven(lifecycle) => lifecycle,
        Standing::Refused(refusal) => return Ok(refusal),
    };

    if !state.sessions.working().contains(&conversation_id) {
        by_hand(state, conversation_id, lifecycle, NOTHING_RUNNING).await?;

        return Ok(ConversationStopped::Stopped);
    }

    store::ask_to_stop(&state.pool, conversation_id).await?;

    tracing::info!(
        conversation_id,
        state = ?lifecycle,
        "the human asked the run to stop after the step it is on",
    );

    Ok(ConversationStopped::Stopping)
}

/// Press Force stop: stop now, and end whatever is running.
///
/// The stop goes first and the ending second, which is the one thing here that
/// has to be that way round. A session ended by Verkstead advances nothing —
/// see [`crate::sessions::Ended::on_purpose`] — so the driver that was seeing it
/// out goes straight to its next launch, and what must be there when it looks is
/// the stop. Ending first would be a race with the very run being stopped.
///
/// One session, because a Conversation has one: [`crate::sessions::Sessions`]
/// keeps a session per Conversation and starting a second ends the first, so
/// ending the Conversation's is ending everything of its that runs — the
/// grilling included.
pub(crate) async fn force(state: &AppState, conversation_id: i64) -> Result<ConversationStopped> {
    let Some(standing) = standing(state, conversation_id).await? else {
        return Ok(ConversationStopped::NoSuchConversation);
    };

    let lifecycle = match standing {
        Standing::Driven(lifecycle) => lifecycle,
        Standing::Refused(refusal) => return Ok(refusal),
    };

    by_hand(state, conversation_id, lifecycle, AT_ONCE).await?;

    state.sessions.end(conversation_id).await;

    tracing::info!(
        conversation_id,
        state = ?lifecycle,
        "the human stopped the run where it stood, and what was running was ended",
    );

    Ok(ConversationStopped::Stopped)
}

/// Whether a Stop the human pressed earlier is waiting to land, and the stop
/// that answers it where one is.
///
/// Asked in front of every launch, through [`crate::stopping::stopped`]: this
/// *is* the next launch the press asked to come before, so the stop is written
/// here and the launch does not happen.
///
/// `true` is *do not launch*, which is also what a store that will not answer
/// gets, for the reason [`crate::stopping::stopped`] reads its own failures that
/// way: what is on the other side of this is spending an account, and something
/// that cannot tell whether the human asked it to stop should wait. Nothing is
/// written down in that case — the sweep finds the Conversation standing still a
/// minute later and says so in its own words.
pub(crate) async fn asked(state: &AppState, conversation_id: i64) -> bool {
    match store::asked_to_stop(&state.pool, conversation_id).await {
        Ok(false) => return false,
        Ok(true) => {}
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading whether the human had asked the run to stop failed");
            return true;
        }
    }

    let lifecycle = match store::load_conversation(&state.pool, conversation_id).await {
        Ok(Some(conversation)) => conversation.state,
        Ok(None) => return true,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading the Conversation the human asked to stop failed");
            return true;
        }
    };

    tracing::info!(
        conversation_id,
        state = ?lifecycle,
        "the step the human asked to stop after has finished, so the run stops here",
    );

    if let Err(error) = by_hand(state, conversation_id, lifecycle, AFTER_THE_STEP).await {
        tracing::error!(error = ?error, conversation_id, "a run the human stopped could not be recorded as stopped");
    }

    true
}

/// Whether Stop and Force stop are worth offering: the Conversation is in a
/// state something ought to be driving, and it has not stopped already.
///
/// The rule the two buttons are drawn by, said once here so that the page and
/// the presses cannot come to different answers about it — the same job
/// [`crate::resume::ready`] does for the button beside them.
///
/// Nothing about what is running. A Conversation between one step and the next
/// is one to stop as much as a busy one: what Stop is for is the run, and the
/// run is what will launch the next session the moment it can.
pub(crate) fn ready(lifecycle: Lifecycle, stopped: bool) -> bool {
    matches!(
        lifecycle,
        Lifecycle::Grilling | Lifecycle::Implementing | Lifecycle::Wrapping
    ) && !stopped
}

/// What a press finds when it arrives: a Conversation to stop, or the refusal to
/// answer with.
enum Standing {
    Driven(Lifecycle),
    Refused(ConversationStopped),
}

/// The rule [`ready`] draws the buttons by, asked again as the press arrives —
/// which is the same order Resume does it in, and for the same reason: the page
/// was drawn a moment ago and the record is what decides.
///
/// `None` is a Conversation that is not there at all.
async fn standing(state: &AppState, conversation_id: i64) -> Result<Option<Standing>> {
    let Some(conversation) = store::load_conversation(&state.pool, conversation_id).await? else {
        return Ok(None);
    };

    let stopped = store::stopped(&state.pool, conversation_id)
        .await?
        .is_some();

    Ok(Some(if !ready(conversation.state, stopped) {
        Standing::Refused(if stopped {
            ConversationStopped::AlreadyStopped
        } else {
            ConversationStopped::NotDriven
        })
    } else {
        Standing::Driven(conversation.state)
    }))
}

/// Write the stop the human's press is, and forget the asking.
///
/// The ordinary stop with the ordinary evidence — what ought to have been
/// happening, why it is not, what git makes of the Worktree and the tail of what
/// the last session said. Nothing about a press deserves a Notice of its own
/// shape: the human reading this tomorrow wants what every other stop tells
/// them, and the reason line is what says this one was theirs.
///
/// The asking is forgotten whether or not a Notice was written. A Conversation
/// that had already stopped keeps the stop it has — there is one per
/// Conversation — and leaving the request behind either way would be a stop that
/// landed again on the far side of the next Resume.
async fn by_hand(
    state: &AppState,
    conversation_id: i64,
    lifecycle: Lifecycle,
    how: &str,
) -> Result<()> {
    let stopped = crate::stopping::stop(
        &state.pool,
        &state.nudges,
        conversation_id,
        Decided::Human,
        crate::stalls::driving(lifecycle),
        how,
        crate::stalls::said_last(state, conversation_id).await,
    )
    .await;

    store::forget_stop(&state.pool, conversation_id).await?;

    stopped?;

    Ok(())
}
