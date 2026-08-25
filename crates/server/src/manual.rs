//! The Manual Task: the one thing the human sets going by hand.
//!
//! Everything else that starts a session is the pipeline deciding — a direction
//! chosen, a step landed, a check gone red. This is the human typing an
//! instruction at the end of a Timeline, picking an Agent Profile beside it and
//! pressing submit, and it is the escape hatch: the way to move a Conversation
//! that nothing is driving, whatever state it says it is in.
//!
//! **Outside the pipeline in every sense.** No state changes on account of one.
//! A Done Conversation stays Done and does not re-enter wrapping when the
//! session commits; a stop stays where it is, with *blocked on you* still on it,
//! because starting to drive again is the only thing that clears one. What a
//! Manual Task leaves behind is its instruction on the record, whatever its
//! session printed, and whatever that committed — all of which land as the
//! ordinary Events they land as everywhere else.
//!
//! **It holds the Turn for as long as it runs.** Starting a session ends
//! whatever is registered, so the only thing that keeps a driver from killing a
//! manual session halfway through is that every driver waits for the Worktree
//! before it launches — see [`crate::runner`]. A submission that finds the Turn
//! taken loses rather than queueing: the composer that was pressed was drawn
//! against a world that has since moved, and an instruction written against that
//! world may no longer be the thing to do.
//!
//! **Ended on quiet with no open Set.** There is no done file to end one by and
//! no path to watch — an instruction can ask for anything — so silence is the
//! only signal there is. Which makes it a heavier signal than a backlog step's,
//! where quiet is the second of two, so the grace is longer: see
//! [`crate::runner::Pace::manual`]. And a session idling on a Blocking Ask
//! prints nothing for hours, so it is quiet *and* nothing of its own awaiting an
//! answer, or it is left where it is.

use verkstead_render::{ManualTaskStarted, ManualTaskSubmission};
use verkstead_schema::Nudge;

use crate::AppState;
use crate::sessions::{Session, Turn};
use crate::skills;
use crate::store;

/// Set a Manual Task going: the instruction on the Timeline, and a session under
/// the Pairing `choice` names doing what it says.
///
/// Returns as soon as the session is running. Seeing it out is a task of its own
/// — a manual task takes as long as it takes, and the browser that submitted it
/// is not waiting for the work.
///
/// The refusals are named rather than collapsed, the way every other press's
/// are: each of them is something different for the human to do about it.
pub(crate) async fn submit(
    state: &AppState,
    conversation_id: i64,
    submission: &ManualTaskSubmission,
) -> anyhow::Result<ManualTaskStarted> {
    let instruction = submission.instruction.trim();

    if instruction.is_empty() {
        return Ok(ManualTaskStarted::EmptyInstruction);
    }

    // Read back rather than trusted from the page, for the reason starting a
    // grilling reads it back: where an agent is about to be let loose is the one
    // thing that must not be guessed at.
    let Some(conversation) = store::load_conversation(&state.pool, conversation_id).await? else {
        return Ok(ManualTaskStarted::NoSuchConversation);
    };

    // The Worktree is the whole of the condition, and the two states without one
    // are the two the composer is never drawn in. Checked against the record
    // rather than the filesystem: a directory the human deleted by hand is a
    // Conversation with a problem — see [`crate::conversations::worktree`] — and
    // the sandbox is what refuses to build over one.
    if conversation.worktree.is_none() {
        return Ok(ManualTaskStarted::NowhereToWork);
    }

    let Some(profile) = store::load_profile(&state.pool, submission.profile_id).await? else {
        return Ok(ManualTaskStarted::NoSuchProfile);
    };

    // The model half of the pick, held to the Profile's list the way the two
    // Pairing choices are: a Profile whose list was edited between the composer
    // being drawn and the press is a session that would launch on a model that
    // account cannot run.
    if !profile.models.contains(&submission.model) {
        return Ok(ManualTaskStarted::NoSuchModel);
    }

    // The pick is one-off, and it is a Pairing rather than a row of the record:
    // nothing about this is written down as the Conversation's.
    let pairing = store::Pairing {
        profile,
        model: Some(submission.model.clone()),
    };

    // Tried for rather than waited for, and the two are different answers to the
    // human. A Turn somebody else holds is an agent already running, which is
    // the one thing that makes this submission stale; waiting for it would leave
    // a browser holding a request open for however long a backlog takes, and
    // then run an instruction written against a Worktree that had moved under
    // it. So this is refused, and the page says so in words.
    let Some(turn) = state.sessions.try_turn(conversation_id) else {
        return Ok(ManualTaskStarted::AlreadyRunning);
    };

    // And again by the register, because the two questions are not the same one.
    // The Turn is taken around a launch and let go; a session outlives the launch
    // that started it, and a driver that has launched and moved on leaves one
    // running with the Turn free.
    if state.sessions.writing(conversation_id).is_some() {
        return Ok(ManualTaskStarted::AlreadyRunning);
    }

    // On the record before anything runs, and above the session's own Event
    // because that is the order it happened in: this is what was asked for, and
    // what follows it is what came of it. A launch that then fails leaves the
    // instruction there as a thing that was asked for, which is what it was.
    if !store::record_manual_task(&state.pool, conversation_id, instruction).await? {
        return Ok(ManualTaskStarted::NoSuchConversation);
    }

    state.nudges.announce(Nudge::Conversation {
        conversation: conversation_id,
    });

    let prompt = skills::manual_task(instruction);

    let started = state
        .sessions
        .start(&state.pool, &state.nudges, &conversation, &pairing, &prompt)
        .await?;

    let Some(session) = started else {
        tracing::error!(
            conversation_id,
            "no session could be started for a manual task, so the instruction stands \
             on the Timeline with nothing running on it"
        );
        return Ok(ManualTaskStarted::NotStarted);
    };

    tracing::info!(
        conversation_id,
        event_id = session.event_id,
        profile = pairing.profile.name,
        model = submission.model,
        "a manual task is running"
    );

    tokio::spawn(follow(state.clone(), conversation_id, session, turn));

    Ok(ManualTaskStarted::Started)
}

/// See a manual session out: end it once it has gone quiet with nothing of its
/// own open, and ask the human about it where it fell over instead.
///
/// `turn` is held for the whole of this and dropped as it returns, which is what
/// makes a driver wanting to launch wait rather than end this session where it
/// stands.
///
/// Nothing is relaunched from here and no state moves. What a manual session
/// ends up having done is on the Timeline — what it printed, what it asked, what
/// it committed — and the Conversation is exactly where it was.
///
/// The one exception is a session that ended badly, which stops like every other
/// session that ended badly — see [`stop`]. The human submits from a phone and
/// walks away, so *blocked on you* and the Notice under it are what reach them.
///
/// What it does end with, either way, is the check for a Conversation nothing
/// is driving — see [`crate::stalls::sweep`]. Nothing is relaunched by it: the
/// human set this going by hand because the work was standing still, and if it
/// is standing still again now that it has finished, that is the thing to say.
async fn follow(state: AppState, conversation_id: i64, mut session: Session, turn: Turn) {
    let event_id = session.event_id;
    let quiet = session.quiet.clone();
    let grace = state.sessions.pace().manual;

    let ended = tokio::select! {
        ended = session.ended() => Some(ended),
        () = quiet_and_nothing_asked(&state, conversation_id, event_id, &quiet, grace) => None,
    };

    let ended = match ended {
        Some(ended) => ended,
        None => {
            tracing::info!(
                conversation_id,
                event_id,
                "a manual session has gone quiet with nothing of its own open, so it is \
                 being ended",
            );

            state.sessions.end(conversation_id).await;

            // And then asked how it ended, rather than taken to have ended the
            // way this branch meant it to. The grace runs out on a session that
            // fell over a moment ago as readily as on one sitting there
            // thinking: a session's last word reaches its driver behind a final
            // sweep of the branch, and a slow one is a session that has gone
            // with nobody told yet. Ending it settles the question — the relay
            // is finished by the time that returns, so how it ended is there to
            // read, and a session Verkstead ended is
            // [`crate::sessions::Ended::Stopped`] either way.
            session.ended().await
        }
    };

    // Nothing is read off the Worktree to decide this, unlike every step of a
    // run: what a manual task was for is a paragraph of free text, so there is
    // no landing to check and no commit that says it worked. An instruction may
    // legitimately change nothing — "have a look at what the migration left
    // behind" is a manual task done when it has been answered — so a clean exit
    // is a clean exit, and only the exit status is a reason to stop.
    if let Some(how) = ended.badly() {
        stop(&state, conversation_id, event_id, &how).await;
    }

    drop(turn);

    crate::stalls::sweep(&state).await;
}

/// Put a manual session that fell over on the Timeline for the human to read.
///
/// The ordinary stop with the ordinary Notice: the evidence is gathered the way
/// every other one's is, and nothing reverts, resets or stashes — the Worktree
/// is left exactly as the session left it.
///
/// [`store::Decision::Deliberate`]. The human typed the instruction and walked away,
/// so a restart running it again on its own would be a machine repeating an act
/// of theirs unasked; whether it goes again is theirs to say.
///
/// Nothing is refused for. By the time this runs the session is gone, and a stop
/// that could not be recorded is a manual task that failed with nothing saying
/// so — which is a thing to see in the log, and the same thing either way.
async fn stop(state: &AppState, conversation_id: i64, writing: i64, how: &str) {
    let stopped = crate::stopping::stop(
        &state.pool,
        &state.nudges,
        conversation_id,
        crate::stopping::Decided::Verkstead,
        "doing what the manual task said",
        how,
        Some(writing),
    )
    .await;

    if let Err(error) = stopped {
        tracing::error!(
            error = ?error,
            conversation_id,
            "a manual task failed and the stop saying so could not be recorded"
        );
    }
}

/// Wait until the session has printed nothing for `grace` *and* has no Question
/// Set of its own still waiting to be answered.
///
/// Both, and the order matters only in that neither is sufficient. Quiet alone
/// would reap a session idling on a Blocking Ask, which is a session doing
/// exactly what it should — `verkstead ask` blocks until the human answers, and
/// that may be the next morning. No-open-Set alone would leave every manual
/// session running until something else ended it, there being no done file to
/// say the work landed.
///
/// Anything the session prints puts the whole grace back on the clock — see
/// [`crate::sessions::Quiet`] — so a session still talking is never one this
/// ends, however long it goes on for.
async fn quiet_and_nothing_asked(
    state: &AppState,
    conversation_id: i64,
    event_id: i64,
    quiet: &crate::sessions::Quiet,
    grace: std::time::Duration,
) {
    let poll = state.sessions.pace().poll;

    loop {
        // The grace first, because it is the cheap half: a session that is still
        // talking is not one to ask the store about at all.
        let owed = grace.saturating_sub(quiet.for_how_long());

        if !owed.is_zero() {
            tokio::time::sleep(owed).await;
            continue;
        }

        if asking(state, conversation_id, event_id).await {
            tokio::time::sleep(poll).await;
            continue;
        }

        return;
    }
}

/// Whether the manual session has a Set of its own still open.
///
/// A store that will not answer reads as *asking*, which is the right way round
/// for the one thing this decides: a session is ended on the strength of it, and
/// ending one mid-question would take the answer away from an agent that had
/// asked for it.
async fn asking(state: &AppState, conversation_id: i64, event_id: i64) -> bool {
    match store::unanswered_set_since(&state.pool, conversation_id, event_id).await {
        Ok(open) => open.is_some(),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading whether a manual session was still asking failed");
            true
        }
    }
}
