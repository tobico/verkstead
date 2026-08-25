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
//! **What is missing is made again**, and the further from a running state the
//! source is the more of it there is to make. A Worktree whose directory has
//! gone is checked out afresh from the branch, exactly as a pressed Resume does
//! it — the branch holds everything that was committed, and a Conversation stuck
//! behind a deleted directory is the very thing this button is for. A closed
//! Conversation kept its branch and lost its Worktree, so it gets one back on
//! the branch. And a Draft has neither, so the branch is cut where a grill start
//! would have cut it: off the base the human fixed, resolved at this moment. See
//! [`somewhere`], which is the one place any of it is decided.
//!
//! **The record is two Events, a Pairing and whatever was made.** The Steer is
//! the human's — *I moved this* — the machine's plain Moved line stands under
//! it, which is the order the moment happened in, and beside them go the Pairing
//! the modal settled and the Worktree and base commit the steer had to make:
//! steering re-settles what runs the work rather than picking for one session.
//! The Steer carries what was written with it, too: an instruction is the Event's
//! own body, so reading it back is reading the job that was set. See
//! [`store::steer_conversation`], which writes all of it in one transaction.
//!
//! **What each target starts is the ordinary recompute.** Into Grilling it is a
//! fresh grilling on the round's own Brief, primed with the digest of what has
//! already been answered where the human asked for it — which is
//! [`crate::grillings::again`], the relaunch a pressed Resume makes, reused
//! rather than forked. Into Implementing it is one of two things: the next step
//! read off the branch — the backlog's own answer to what is next, or the
//! roadmap it has written, which is [`crate::runner::implementing_again`], the
//! same relaunch — or, where the human wrote one, the instruction itself, in a
//! session that drives the Conversation and hands the pipeline on when it is
//! done; see [`crate::runner::instructed`]. One or the other and never neither:
//! an instruction is required exactly where nothing stands to be carried on,
//! which is what [`standing`] answers. Into Wrapping it is the wrap-up's own
//! watchers over whatever the branch now holds, with the fix attempts
//! forgotten, which is what that press does there. Into Done there is nothing
//! to start at all.
//!
//! Nothing is reverted, reset or stashed, here or anywhere a stop is written:
//! the Worktree is left exactly as whatever was running left it.

use std::path::PathBuf;

use verkstead_render::{ConversationSteered, SteerOpened, SteerSubmission, SteerTarget};
use verkstead_schema::{Direction, Nudge};

use crate::AppState;
use crate::grillings::Digest;
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
/// **Interrupt current task** was ticked; make whatever the work needs and has
/// not got — a Worktree, and the branch under it; clear the stop the click left;
/// move the Conversation, recording the Steer, the Brief, the Pairing and what
/// was just made as one act; and then start whatever the target needs starting.
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

    let made = match somewhere(state, &conversation, submission.target).await? {
        Making::Refused(refusal) => return Ok(refusal),
        Making::Ready(made) => made,
    };

    clear(state, conversation_id).await?;

    let instruction = instruction(submission);

    let steer = store::Steer {
        target,
        pairing: settling(&conversation, submission),
        brief: brief(submission),
        instruction,
        direction: directing(&conversation, instruction),
        worktree: made.worktree.as_deref(),
        base_commit: made.base_commit.as_deref(),
    };

    match store::steer_conversation(&state.pool, conversation_id, steer).await? {
        store::Steering::NoSuchConversation => return Ok(ConversationSteered::NoSuchConversation),
        store::Steering::NoSuchProfile => return Ok(ConversationSteered::NoSuchProfile),
        store::Steering::Steered => {}
    }

    match submission.target {
        // A grilling from the beginning, which is the only kind there is: an
        // interview lives in the session having it, so there is never one to
        // pick up. What that session is primed with is the round's own Brief —
        // the one the modal just wrote, or the one that was already there — and
        // the digest of everything answered where the human asked for it. See
        // [`crate::grillings::again`], which is the same launch a pressed Resume
        // makes and which holds the registration until its session has one.
        SteerTarget::Grilling => {
            tokio::spawn(crate::grillings::again(
                state.clone(),
                conversation_id,
                driving,
                match submission.digest {
                    true => Digest::Prime,
                    false => Digest::Skip,
                },
            ));
        }

        // One of two, and which of them is what the human wrote rather than
        // anything read off the branch.
        //
        // Nothing here can come to nothing either way: [`standing`] has already
        // refused a steer with neither an instruction nor anything to carry on,
        // which is the reading this one is the second of.
        SteerTarget::Implementing => match instruction {
            // A session started on the instruction, driving the Conversation
            // while it runs and handing the pipeline on from whatever the branch
            // then holds. See [`crate::runner::instructed`], which is what makes
            // this a driver rather than the errand beside the work a Manual Task
            // is.
            Some(instruction) => {
                tokio::spawn(crate::runner::instructed(
                    state.clone(),
                    conversation_id,
                    instruction.to_owned(),
                    driving,
                ));
            }

            // Or the next step read off the branch, which is the direction's to
            // say: the backlog's own answer to what is next, or the roadmap the
            // branch has written. Exactly what a pressed Resume does for an
            // implementing Conversation, reused rather than forked — see
            // [`crate::runner::implementing_again`], which reads the branch
            // again for itself a moment after this and holds the registration
            // until its session has one.
            None => {
                tokio::spawn(crate::runner::implementing_again(
                    state.clone(),
                    conversation_id,
                    driving,
                ));
            }
        },

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
/// Everything asked of the record and of the branch as it already stands, so
/// that all of it can be asked before a session is ended, a Worktree is rebuilt
/// or a stop is taken away. The state the Conversation is *in* is not among them
/// and never will be — every state is a source — so each of these is about the
/// state it is going *to*: work that cannot be set going from what there is.
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

    // And a steer into Implementing either carries on what the branch already
    // holds or does what the human wrote, so a branch holding nothing to carry
    // on and a modal holding nothing written is a session with no job. The modal
    // requires the instruction there rather than offering the submit —
    // [`standing`] is what it draws that by, and this is that same reading asked
    // again on arrival.
    //
    // A Worktree that is not there to be read is not a branch holding nothing:
    // the steer checks one out of the branch a moment after this, and refusing
    // here would refuse the Conversation this button was written for. See
    // [`Standing::Unreadable`].
    if submission.target == SteerTarget::Implementing
        && instruction(submission).is_none()
        && !standing(conversation).await.offerable()
    {
        return Ok(Some(ConversationSteered::NoInstruction));
    }

    // And a grilling starts from a Brief, so a round opened with none written in
    // the modal and none already on the Timeline is an interview about nothing.
    // The rule a pressed *Start grilling* is refused by — see
    // [`crate::conversations::start_grilling`] — asked of the other way in, and
    // it has to be asked here rather than left to the session: the Brief a
    // steered round lands with is frozen where it lands, so there is no draft to
    // go back and write one in.
    if submission.target == SteerTarget::Grilling
        && brief(submission).is_none()
        && crate::conversations::brief(&state.pool, conversation.id)
            .await?
            .trim()
            .is_empty()
    {
        return Ok(Some(ConversationSteered::EmptyBrief));
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

/// The new round's Brief to write with the move, or `None` where there is none
/// to write.
///
/// Whitespace alone counts as none: a textarea the human tabbed through is a
/// steer without a brief, and a Brief Event holding a blank line would be a
/// round that says it was started from nothing in particular. What is written is
/// what they typed, though, exactly as the drafting field saves it — a document
/// is kept as its author left it.
///
/// Only where a round is being opened. A brief that arrived beside another
/// target is a page sending a field it should not have drawn, and what a Brief
/// Event under a wrap-up would mean is nothing at all.
fn brief(submission: &SteerSubmission) -> Option<&str> {
    if submission.target != SteerTarget::Grilling {
        return None;
    }

    submission
        .brief
        .as_deref()
        .filter(|brief| !brief.trim().is_empty())
}

/// The hand-written work to send a session off with, or `None` where the human
/// wrote none.
///
/// Whitespace alone counts as none, exactly as the brief above it: a textarea
/// somebody tabbed through is a steer that carries on what stands rather than an
/// instruction to do nothing. What is written is what they typed, trimmed at the
/// ends the way every document reaching a prompt is.
///
/// Only where a session could be started on it. An instruction that arrived
/// beside another target is a page sending a field it should not have drawn, and
/// what a wrap-up would do with one is nothing at all.
fn instruction(submission: &SteerSubmission) -> Option<&str> {
    if submission.target != SteerTarget::Implementing {
        return None;
    }

    submission
        .instruction
        .as_deref()
        .map(str::trim)
        .filter(|instruction| !instruction.is_empty())
}

/// How the work is built from here, or `None` where the Conversation has already
/// said.
///
/// The one thing a steer settles that the human did not pick, and it is settled
/// rather than asked because there is only one answer it could have: an
/// instruction session is the whole of the work in one session, which is what
/// **inline** means. A Conversation that has been grilled has already picked,
/// and what it picked is left exactly as it is — see
/// [`store::steer_conversation`], which will not write over one.
///
/// Written at all because of what comes after the steer rather than what comes
/// with it. A Conversation implementing with nothing saying how its work is
/// built is a record a pressed Resume refuses on by name, so a steer that set a
/// session going in one would leave the Conversation unable to be started again
/// the moment that session ended.
fn directing(conversation: &Conversation, instruction: Option<&str>) -> Option<Direction> {
    instruction?;

    conversation
        .direction
        .is_none()
        .then_some(Direction::Inline)
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
    //
    // Both halves, because both halves are what a pick is. The picker offers one
    // row per Profile-and-model, so the same Profile on another of its models is
    // a different pick — and a comparison that asked about the Profile alone
    // would answer *Steered* to a change of model and write none of it.
    if fixed(conversation, role).is_some_and(|pairing| {
        pairing.profile.id == choice.profile_id && pairing.model.as_deref() == Some(&choice.model)
    }) {
        return None;
    }

    Some(Settling {
        role,
        profile_id: choice.profile_id,
        model: &choice.model,
    })
}

/// What a steer into Implementing would find on the branch to carry on.
///
/// Three answers rather than two, because a Worktree that is not there is not
/// the same thing as one that is there and empty — see [`Standing`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Standing {
    /// The Worktree is there, and it holds work left to do.
    Stands,

    /// It is there, and it holds none — or the Conversation's work is not the
    /// kind anything on a branch could be carried on from.
    ///
    /// The one answer that refuses: a steer into Implementing here either does
    /// what the human wrote or does nothing at all.
    Nothing,

    /// There was nothing to read: no Worktree on the record, or one git will not
    /// answer about any more.
    ///
    /// **Not a refusal**, and that is the whole of why this is three answers.
    /// The steer checks the branch out again before anything runs in it — see
    /// [`somewhere`] — so a directory that has gone says nothing about what the
    /// branch holds, and the Conversation stuck behind a deleted one is the very
    /// thing this button is for. What decides it instead is
    /// [`crate::runner::implementing_again`], which reads the Worktree the steer
    /// has just made and starts nothing where there is nothing — exactly as it
    /// does for a pressed Resume.
    Unreadable,
}

impl Standing {
    /// Whether *carrying on* is worth offering, which is everything but a
    /// Worktree that was read and held nothing.
    ///
    /// What [`crate::ui`] draws the modal by and what [`refusal`] refuses by,
    /// said once so that the page and the press cannot come to different answers
    /// about it.
    pub(crate) fn offerable(self) -> bool {
        self != Self::Nothing
    }
}

/// Whether there is work standing on the branch for a steer into Implementing
/// to carry on.
///
/// The one question that target turns on, said once here because the modal
/// draws by it and the submit refuses by it — see
/// [`ConversationSteered::NoInstruction`]. Two things can stand, and the
/// direction is what says which of them this run would be:
///
/// - **a backlog with work left in it**, which is `.tasks/` asked exactly as
///   every turn of a task-list run asks it — the finish step included, a list
///   whose tasks are all worked still having the finish to run; and
/// - **a roadmap the branch has written**, which is the reading a stalled
///   roadmap run makes of itself before it decides whether to write one.
///
/// An inline run stands for nothing here, and that is the point of the
/// distinction rather than an omission: its work is the one session, so there
/// is nothing on the branch to pick up where it left off. What such a
/// Conversation is steered into Implementing with is an instruction of the
/// human's own, which is what [`Standing::Nothing`] asks the modal for — and it
/// is answered before the directory is looked at, no reading of one being able
/// to change it.
///
/// Read of the Worktree as it stands, which is how everything else pinned to a
/// Timeline is read. Nothing is made here to find out — this is asked in front
/// of a refusal, and a refusal that had rebuilt a Worktree first would be a
/// press that half happened — but a Worktree that is not there to be read is
/// [`Standing::Unreadable`] rather than nothing standing, because the steer
/// makes one out of the branch a moment later and the branch is what holds the
/// backlog.
///
/// [`crate::worktrees::healthy`] for whether there is one to read, which is the
/// same reading [`somewhere`] makes about the same directory a moment later.
///
/// Off the runtime's threads, every one of these readings being a filesystem or
/// git call.
pub(crate) async fn standing(conversation: &Conversation) -> Standing {
    // A Conversation that has never said how its work is built has nothing to
    // carry on whatever is on its branch: what says what is next is the
    // direction, and there is none.
    let Some(direction) = conversation.direction else {
        return Standing::Nothing;
    };

    if direction == Direction::Inline {
        return Standing::Nothing;
    }

    let Some(worktree) = conversation.worktree.clone() else {
        return Standing::Unreadable;
    };

    let there = {
        let repo = conversation.repo.path.clone();
        let branch = conversation.branch.clone();
        let worktree = worktree.clone();

        tokio::task::spawn_blocking(move || crate::worktrees::healthy(&repo, &worktree, &branch))
            .await
            .unwrap_or(false)
    };

    if !there {
        return Standing::Unreadable;
    }

    match direction {
        Direction::TaskList => match crate::runner::anything_to_work(&worktree).await {
            true => Standing::Stands,
            false => Standing::Nothing,
        },

        // What a roadmap Conversation has written since it branched, committed
        // or not — see [`crate::stages::touched`], which is the same reading
        // [`crate::runner::implementing_again`] makes a moment later to decide
        // what to start. No base commit is a Conversation that has never
        // branched, which has a Worktree it did not get from one.
        Direction::Roadmap => {
            let Some(base) = conversation.base_commit.clone() else {
                return Standing::Nothing;
            };

            let touched = tokio::task::spawn_blocking(move || {
                !crate::stages::touched(&worktree, &base).is_empty()
            })
            .await
            .unwrap_or(false);

            match touched {
                true => Standing::Stands,
                false => Standing::Nothing,
            }
        }

        // Answered above, before anything was read.
        Direction::Inline => Standing::Nothing,
    }
}

/// Which role's Pairing a target's sessions run under, or `None` where nothing
/// runs there.
///
/// Implementing and Wrapping are one answer between them, because they are one
/// run seen at two moments: the task sessions build the work, and the wrap-up's
/// watchers dispatch the fix, review and comment sessions that see it through.
/// Every one of those is the work itself, so all of them run under the
/// implementation Pairing. A grilling is the other one, which is what an
/// interview runs under whatever else has happened since.
fn role(target: SteerTarget) -> Option<Role> {
    match target {
        SteerTarget::Grilling => Some(Role::Grilling),
        SteerTarget::Implementing | SteerTarget::Wrapping => Some(Role::Implementation),
        SteerTarget::Done => None,
    }
}

/// What the Conversation has settled for that role already, whole.
///
/// Both halves rather than the Profile it names, because both are what a pick is
/// held against — see [`settling`]. A Pairing carrying no model is a Profile
/// chosen before pairings existed, which is half a choice and so never the same
/// choice as one made now.
fn fixed(conversation: &Conversation, role: Role) -> Option<&store::Pairing> {
    match role {
        Role::Grilling => conversation.grilling_pairing.as_ref(),
        Role::Implementation => conversation.implementation_pairing.as_ref(),
    }
}

/// Make sure there is somewhere to work, and say what had to be made.
///
/// Nothing made and nothing looked at where the target runs nothing: Done needs
/// no directory, and a steer into it must not turn on whether one is still
/// there.
///
/// Otherwise the three cases, cheapest first, and which one a Conversation is in
/// is a fact about it rather than a choice:
///
/// - **A Worktree git can still answer about is left exactly as it stands**,
///   uncommitted changes and all — see [`crate::worktrees::healthy`], which is
///   the same reading a pressed Resume makes.
/// - **A branch with no Worktree on it gets one**: a directory deleted, hollowed
///   out or dropped from the repository's list of worktrees, and a closed
///   Conversation whose Worktree was taken away and whose branch was kept. A
///   worktree is derived state, so it is made again rather than refused on.
/// - **A branch that was never made is cut**, off the base the human fixed while
///   drafting or the Repo's default branch where they fixed none — resolved at
///   this moment, exactly as [`crate::conversations::start_grilling`] resolves
///   it, because what they picked is a branch and what they meant by picking it
///   is wherever it stands now.
///
/// Which is why the path is chosen here where the record holds none: it is
/// chosen the way a first grilling chooses one, and this is a first grilling for
/// everything but the Conversation's name.
///
/// [`crate::worktrees::branch_taken`] rather than `branch_exists` for the one
/// question that decides between the last two: cutting a branch over somebody
/// else's work is the mistake pressing the button again cannot undo, so a read
/// git would not answer counts as a branch that is there.
///
/// Off the runtime's threads, a checkout of a large repository being no quick
/// call and every part of this blocking.
async fn somewhere(
    state: &AppState,
    conversation: &Conversation,
    target: SteerTarget,
) -> anyhow::Result<Making> {
    if !target.runs() {
        return Ok(Making::Ready(Made::default()));
    }

    let path = conversation.worktree.clone().unwrap_or_else(|| {
        crate::worktrees::worktree_path(
            &state.data_dir,
            conversation.id,
            &conversation.repo.name,
            &conversation.branch,
        )
    });

    let repo = conversation.repo.path.clone();
    let branch = conversation.branch.clone();

    // What a branch that has never been cut comes off, named rather than
    // resolved: a drafting Conversation's column holds the *branch* the human
    // picked, and where they picked none the rule is the Repo's default branch.
    let named = conversation
        .base_commit
        .clone()
        .unwrap_or_else(|| conversation.repo.default_branch.clone());

    let made = tokio::task::spawn_blocking({
        let path = path.clone();

        move || {
            if crate::worktrees::healthy(&repo, &path, &branch) {
                return Ok(None);
            }

            if crate::worktrees::branch_taken(&repo, &branch) {
                return crate::worktrees::rebuild(&repo, &path, &branch)
                    .then_some(None)
                    .ok_or(ConversationSteered::WorktreeRefused);
            }

            let Some(commit) = crate::worktrees::resolve(&repo, &named) else {
                return Err(ConversationSteered::NoBaseCommit);
            };

            crate::worktrees::add(&repo, &path, &branch, &commit)
                .then_some(Some(commit))
                .ok_or(ConversationSteered::WorktreeRefused)
        }
    })
    .await?;

    Ok(match made {
        // The path either way, healthy or made: the record may never have held
        // one, and writing back the one it did hold changes nothing.
        Ok(base_commit) => Making::Ready(Made {
            worktree: Some(path),
            base_commit,
        }),
        Err(refusal) => Making::Refused(refusal),
    })
}

/// What making somewhere to work came to.
enum Making {
    Ready(Made),

    /// Git would not, and the human is told which way it would not — see
    /// [`ConversationSteered::WorktreeRefused`] and
    /// [`ConversationSteered::NoBaseCommit`].
    Refused(ConversationSteered),
}

/// What the steer has to record about where the work will run.
///
/// Both `None` for a target nothing runs in, which is what [`Default`] is here
/// for: a steer into Done makes nothing and writes nothing about a directory.
#[derive(Default)]
struct Made {
    /// Where the work goes on.
    worktree: Option<PathBuf>,

    /// What the branch was cut from, where this is what cut it. `None` on every
    /// Conversation that had a branch already: what it branched from was
    /// resolved once, and it is not resolved again.
    base_commit: Option<String>,
}

/// The state a target names.
///
/// The one place the modal's vocabulary and the record's are held to each other,
/// the way [`crate::ui`] holds the lifecycle's two spellings together — and the
/// reason the two are separate lists at all: every state is somewhere to steer
/// *from*, and only some of them are somewhere to steer *to*.
fn target(target: SteerTarget) -> Lifecycle {
    match target {
        SteerTarget::Grilling => Lifecycle::Grilling,
        SteerTarget::Implementing => Lifecycle::Implementing,
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
