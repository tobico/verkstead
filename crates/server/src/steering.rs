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
//! each has a way in of its own. So every refusal here is about the *target*:
//! work that cannot be set going from what the record holds.
//!
//! **What is missing is made again.** A Worktree whose directory has gone is
//! checked out afresh from the branch, exactly as a pressed Resume does it — the
//! branch holds everything that was committed, and a Conversation stuck behind a
//! deleted directory is the very thing this button is for.
//!
//! **The record is two Events and a Pairing.** The Steer is the human's — *I
//! moved this* — the machine's plain Moved line stands under it, which is the
//! order the moment happened in, and the Pairing the modal settled is written
//! beside them: steering re-settles what runs the work rather than picking for
//! one session. See [`store::steer_conversation`], which writes all of it in one
//! transaction.
//!
//! **What each target starts is the ordinary recompute.** Into Wrapping it is
//! the wrap-up's own watchers over whatever the branch now holds, with the fix
//! attempts forgotten — which is what a pressed Resume already does there, and
//! it is reused rather than forked. Into Done there is nothing to start at all.
//!
//! Nothing is reverted, reset or stashed, here or anywhere a stop is written:
//! the Worktree is left exactly as whatever was running left it.

use verkstead_render::{ConversationSteered, SteerOpened, SteerSubmission, SteerTarget};
use verkstead_schema::Nudge;

use crate::AppState;
use crate::profiles::Unlisted;
use crate::store::{self, Conversation, Lifecycle, Role, Settling};

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
/// In order: refuse what cannot be done at all; end what is running where
/// **Interrupt current task** was ticked; make the Worktree again where the
/// directory it names has gone; clear the stop the click left; move the
/// Conversation, recording the Steer and the Pairing as one act; and then start
/// whatever the target needs starting.
///
/// **The refusals go first**, before anything is ended, rebuilt or cleared. The
/// browser is holding the request open for the answer, and a refusal that had
/// already ended somebody's session would be a press that half happened — see
/// [`refusal`], which is every one of them asked of the record alone. What is
/// after them either happens or is a failure.
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

    if let Some(refusal) = refusal(state, &conversation, submission).await? {
        return Ok(refusal);
    }

    // Taken before the Worktree is touched and held across the spawn, which is
    // what [`crate::resume`] does with its own and for the same reason: from
    // here until something is registered the Conversation is being driven with
    // nothing on the register, and a stall sweep that looked in between would
    // read it as standing still and stop it all over again. Dropped on every
    // path out that does not launch — which is what leaving it to the `?` and
    // the early returns does.
    let driving = state.drivers.driving(conversation_id);

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

    if !usable(&conversation, submission.target).await? {
        return Ok(ConversationSteered::WorktreeRefused);
    }

    clear(state, conversation_id).await?;

    let pairing = settling(&conversation, submission);

    match store::steer_conversation(&state.pool, conversation_id, target, pairing).await? {
        store::Steering::NoSuchConversation => return Ok(ConversationSteered::NoSuchConversation),
        store::Steering::NoSuchProfile => return Ok(ConversationSteered::NoSuchProfile),
        store::Steering::Steered => {}
    }

    match submission.target {
        // The wrap-up's four watchers over the top of whatever the branch now
        // holds, with the fix attempts forgotten: exactly what a pressed Resume
        // does for a wrapping Conversation, reused rather than forked. Each of
        // the four decides for itself whether there is anything left to do, so
        // there is nothing here that can come to nothing.
        SteerTarget::Wrapping => {
            let state = state.clone();

            tokio::spawn(async move {
                // Held until the four watchers have registrations of their own,
                // which is what [`crate::wrapping::watching`] takes as it spawns
                // them: dropping first would leave a moment where a sweep could
                // find the Conversation undriven all over again.
                let _driving = driving;

                crate::checks::afresh(state, conversation_id).await;
            });
        }

        // And nothing at all: a steer into Done is the move alone.
        SteerTarget::Done => drop(driving),
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

/// Why this steer cannot be made at all, or `None` where it can.
///
/// Everything asked of the record and of nothing else, so that all of it can be
/// asked before a session is ended or a stop taken away. The state the
/// Conversation is *in* is not among them and never will be — every state is a
/// source — so each of these is about the state it is going *to*: work that
/// cannot be set going from what the record holds.
async fn refusal(
    state: &AppState,
    conversation: &Conversation,
    submission: &SteerSubmission,
) -> anyhow::Result<Option<ConversationSteered>> {
    // A wrapping Conversation is defined by the pull request under it, so a steer
    // into Wrapping is a move onto one that is already there. The modal does not
    // offer the target where there is none; this is the same rule asked again on
    // arrival.
    if submission.target == SteerTarget::Wrapping
        && store::pull_request(&state.pool, conversation.id)
            .await?
            .is_none()
    {
        return Ok(Some(ConversationSteered::NoPullRequest));
    }

    // And somewhere to run: every state past drafting has a Worktree, so one
    // missing from the record is a record that cannot be true. There is nothing
    // to make one from either — the path is Verkstead's own to have chosen, and
    // nothing here knows which one it chose. Whether the directory is still
    // *there* is a different question, asked after the refusals and answered by
    // making it again — see [`usable`].
    if submission.target.runs() && conversation.worktree.is_none() {
        return Ok(Some(ConversationSteered::NowhereToWork));
    }

    let Some(role) = role(submission.target) else {
        return Ok(None);
    };

    // What the human picked, judged the way the drafting pickers judge theirs: a
    // Profile that has gone and a model it no longer lists are both a list that
    // was edited between the read and the pick.
    if let Some(choice) = &submission.pairing {
        return Ok(
            match crate::profiles::unlisted(&state.pool, choice).await? {
                Some(Unlisted::NoSuchProfile) => Some(ConversationSteered::NoSuchProfile),
                Some(Unlisted::NoSuchModel) => Some(ConversationSteered::NoSuchModel),
                None => None,
            },
        );
    }

    // Nothing picked, which is the human leaving the picker on what the
    // Conversation already had — and a refusal where it had none. A session is
    // launched under a Pairing, so a state something runs in with none settled is
    // a move into work nothing could start.
    Ok(fixed(conversation, role)
        .is_none()
        .then_some(ConversationSteered::NoPairing))
}

/// The Pairing to write with the move, or `None` where there is none to write.
///
/// `None` twice over: a target nothing runs in settles nothing, and a human who
/// left the picker alone has changed nothing — [`refusal`] has already made sure
/// that the Conversation's own is there in that case.
fn settling<'a>(
    conversation: &Conversation,
    submission: &'a SteerSubmission,
) -> Option<Settling<'a>> {
    let role = role(submission.target)?;
    let choice = submission.pairing.as_ref()?;

    // The Conversation's own is left exactly as it is rather than rewritten with
    // itself: a re-choice takes the model row away and puts it back, and a
    // Pairing that did not change is not something to rewrite.
    if fixed(conversation, role) == Some(choice.profile_id) {
        return None;
    }

    Some(Settling {
        role,
        profile_id: choice.profile_id,
        model: &choice.model,
    })
}

/// Which role's Pairing a target's sessions run under, or `None` where nothing
/// runs there.
///
/// The wrap-up's watchers dispatch fix sessions, review sessions and comment
/// sessions, and every one of them is the work itself: they run under the
/// implementation Pairing, the same one the backlog was worked under.
fn role(target: SteerTarget) -> Option<Role> {
    match target {
        SteerTarget::Wrapping => Some(Role::Implementation),
        SteerTarget::Done => None,
    }
}

/// What the Conversation has settled for that role already, as the Profile it
/// names.
fn fixed(conversation: &Conversation, role: Role) -> Option<i64> {
    let pairing = match role {
        Role::Grilling => &conversation.grilling_pairing,
        Role::Implementation => &conversation.implementation_pairing,
    };

    pairing.as_ref().map(|pairing| pairing.profile.id)
}

/// Make sure there is a Worktree to work in, and say whether there is.
///
/// `true` without looking at anything where the target runs nothing: Done needs
/// no directory, and a steer into it must not turn on whether one is still
/// there.
///
/// Otherwise the same two calls Resume makes, in the same order and for the same
/// reason: a directory deleted, hollowed out or dropped from the repository's
/// list of worktrees is a Conversation stuck for good, and a worktree is derived
/// state — the branch holds everything that was committed. Nothing healthy is
/// touched, uncommitted changes and all. See [`crate::worktrees::healthy`].
///
/// Off the runtime's threads, a checkout of a large repository being no quick
/// call and every part of this blocking.
async fn usable(conversation: &Conversation, target: SteerTarget) -> anyhow::Result<bool> {
    if !target.runs() {
        return Ok(true);
    }

    let Some(worktree) = conversation.worktree.clone() else {
        // Refused before this by [`refusal`], which reads the same field: there
        // is no path to make one at, Verkstead's own choice of path being
        // nothing this knows.
        return Ok(false);
    };

    let repo = conversation.repo.path.clone();
    let branch = conversation.branch.clone();

    Ok(tokio::task::spawn_blocking(move || {
        crate::worktrees::healthy(&repo, &worktree, &branch)
            || crate::worktrees::rebuild(&repo, &worktree, &branch)
    })
    .await?)
}

/// The state a target names.
///
/// The one place the modal's vocabulary and the record's are held to each other,
/// the way [`crate::ui`] holds the lifecycle's two spellings together — and the
/// reason the two are separate lists at all: every state is somewhere to steer
/// *from*, and only some of them are somewhere to steer *to*.
fn target(target: SteerTarget) -> Lifecycle {
    match target {
        SteerTarget::Wrapping => Lifecycle::Wrapping,
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
