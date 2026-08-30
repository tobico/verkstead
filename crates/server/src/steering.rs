//! Steer: the human saying where the work goes, from wherever it has got to.
//!
//! Two presses rather than one, and the gap between them is the point. Clicking
//! **Steer** stops the drive and answers with what it found running; the modal
//! then opens over a Conversation nothing is going to launch into while the
//! human composes. What they submit lands as the move.
//!
//! **The click stops it, and that is deliberate.** It is the ordinary Stop —
//! nothing new starts, and a session already running is left exactly where it
//! is — so the world the modal is drawn against is the world the submit arrives
//! in. **Cancel leaves the Conversation stopped**, with Resume on offer: the
//! click is what froze it, and unfreezing is a press of its own rather than
//! something a dismissed modal does behind the human's back.
//!
//! **What ends that session is the submit rather than the click.** One Worktree
//! holds one agent, so the session a steer starts takes the Worktree from
//! whatever is still in it — see the runner's `launch_in_turn`, which waits for
//! a session that cannot be displaced and ends one that can. So a steer into a
//! target something runs in ends what was running, at once or once a review
//! waiting on an ask has finished; a steer into Done launches nothing and ends
//! nothing. **Interrupt current task** is what ends it where it stands instead:
//! the wait saved in the first case, and the only ending there is in the second.
//!
//! **Every state is a source.** Unlike every other move here, a steer answers to
//! no rung of the ladder: a draft nothing has run in, a run in flight and a
//! Conversation Verkstead has finished with are all somewhere to be steered
//! from. What the targets are is [`SteerTarget`]'s to say, and it is the short
//! list — the states work is *done in*, with Draft and Closed left out because
//! each has a way in of its own, and Follow-up in because a steer is the only
//! way into it at all. So every refusal here is about the *target*: work that
//! cannot be set going from what the record holds.
//!
//! **What is missing is made again**, and the further from a running state the
//! source is the more of it there is to make. A Worktree whose directory has
//! gone is checked out afresh from the branch, exactly as a pressed Resume does
//! it — the branch holds everything that was committed, and a Conversation stuck
//! behind a deleted directory is the very thing this button is for. A closed
//! Conversation kept its branch and lost its Worktree, so it gets one back on
//! the branch. And a Draft has neither, so the branch is cut where a grill start
//! would have cut it: off the base the human fixed, resolved at this moment.
//!
//! **Every companion checkout the record says is missing is made again too**,
//! beside the Conversation's own, because the two sources with nothing on disk
//! have companions with nothing on disk: a Draft's were recorded on the setup
//! card and never checked out, and a Conversation steered back out of Closed had
//! its directories removed and its rows forgotten while its branches were kept.
//! Without this either would reach a running state with companions the sandbox
//! skips in silence — a session quietly missing the repository it was given. See
//! [`plan`], which is the one place any of it is decided, and [`make`], which is
//! the one place any of it is created.
//!
//! **And a steer may widen the set it works alongside, and open a row of it
//! up.** The modal's companion section puts another registered Repo in and ticks
//! a read-only one up to read-write, and the steer checks each out as part of
//! the move: fetch, resolve, cut or detach, bind — the grill start's shape,
//! including the fetch this deliberately skips for the Conversation's own
//! repository, a companion joining now being new work rather than an old
//! checkout being put back. An upgrade is fresh for that reason and not pinned:
//! the commit its detached checkout stands at is where that repository was when
//! the Conversation started, and its branch is cut from where the base stands
//! at the steer instead. The detached directory is then replaced, one companion
//! being one checkout.
//!
//! **One direction, and never the other.** Nothing here removes a companion or
//! narrows one, and the payload cannot spell either: what a session was once
//! given is never taken back mid-Conversation.
//!
//! **The record is two Events, a Pairing and whatever was made.** The Steer is
//! the human's — *I moved this* — the machine's plain Moved line stands under
//! it, which is the order the moment happened in, and beside them go the Pairing
//! the modal settled and the Worktree and base commit the steer had to make:
//! steering re-settles what runs the work rather than picking for one session.
//! The Steer carries what was written with it, too: an instruction, or the brief
//! a follow-up is opened on, is the Event's own body, so reading it back is
//! reading the job that was set. See [`store::steer_conversation`], which writes
//! all of it in one transaction.
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
//! forgotten, which is what that press does there. Into Follow-up it is a
//! session on the follow-up skill, started on the brief the human wrote and
//! required to have one — see [`crate::runner::following_up`]. Into Done there
//! is nothing to start at all.
//!
//! Nothing is reverted, reset or stashed, here or anywhere a stop is written:
//! the Worktree is left exactly as whatever was running left it.

use std::path::{Path, PathBuf};

use verkstead_render::{
    CompanionMode, ConversationSteered, SteerCompanionRefusal, SteerOpened, SteerSubmission,
    SteerTarget,
};
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
/// In order: refuse what cannot be done at all; work out every checkout the
/// work needs and has not got, without making any of it; end what is running
/// where **Interrupt current task** was ticked; make those checkouts and replace
/// the ones they take the place of; clear the stop the click left; move the
/// Conversation, recording the Steer, the Brief, the Pairing, the companions —
/// the ones that came in and the ones that were opened up — and what was just
/// made as one act; and then start whatever the target needs starting.
///
/// **Every question goes first**, before anything is ended, rebuilt or cleared —
/// the ones asked of the record and the ones asked of git alike. The browser is
/// holding the request open for the answer, and a refusal that had already ended
/// somebody's session would be a press that half happened. See [`refusal`],
/// which is every one asked of the record alone, [`additions`] and [`upgrades`],
/// which are what the record is asked about the companion section's two halves,
/// and [`plan`], which asks git everything a directory turns on and makes none
/// of it. What is after them either happens or is a failure.
///
/// The stop goes before the move rather than after, because what it is in front
/// of is the launch: a run does not advance past a stop — see
/// [`crate::stopping::stopped`] — so anything started behind one would find the
/// Conversation stopped and start nothing. Into Done there is nothing to start,
/// and the stop is cleared all the same: a Conversation Verkstead has finished
/// with wearing *blocked on you* would be a word with nothing behind it to
/// answer.
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

    // Which registered Repos the modal named, read back as rows and held against
    // what the Conversation already has — and then everything git is asked about
    // a directory: what has to be made, and whether any of it can be.
    //
    // Both in front of the interrupt below, which is what makes a steer refused
    // for a companion a press that did not happen: no directory, no branch, no
    // row, nothing ended and no stop cleared. The registration above is dropped
    // on each of these by the early return.
    let added = match additions(state, &conversation, submission).await? {
        Additions::Refused(refusal) => return Ok(refusal),
        Additions::Ready(added) => added,
    };

    // And which of the companions it already has are being opened up, read back
    // as the rows they will become: read-write, with the branch the modal typed
    // or mirroring where it typed none.
    let opened = match upgrades(state, &conversation, submission).await? {
        Upgrades::Refused(refusal) => return Ok(refusal),
        Upgrades::Ready(opened) => opened,
    };

    let planned = match plan(state, &conversation, submission.target, &added, &opened).await? {
        Planning::Refused(refusal) => return Ok(refusal),
        Planning::Ready(planned) => planned,
    };

    if submission.interrupt {
        // Ended rather than force-stopped, because the stop is already written:
        // the click wrote one, or recorded the request that the next launch
        // turns into one. A session Verkstead ended advances nothing — see
        // [`crate::sessions::Ended::on_purpose`] — so what was following it
        // reads its ending as the ending it is, and what it leaves in the
        // Worktree is left there.
        //
        // Here rather than left to the launch, which is what ends it otherwise
        // and which cannot end every session: a review waiting on an ask holds
        // the Worktree against being displaced, and into Done nothing launches
        // at all. This is the one ending that answers to neither.
        state.sessions.end(conversation_id).await;

        tracing::info!(
            conversation_id,
            "the steer's Interrupt was ticked, so the session running was ended where it stood",
        );
    }

    // From here to the record naming what this makes, as a grill start holds it
    // and for its reason: a directory made and not yet recorded is one the sweep
    // of orphaned worktrees would read as nobody's. See
    // [`crate::AppState::checkouts`].
    let making = state.checkouts.lock().await;

    let made = match make(planned).await? {
        Making::Refused(refusal) => return Ok(refusal),
        Making::Ready(made) => made,
    };

    clear(state, conversation_id).await?;

    let instruction = instruction(submission);
    let follow_up = follow_up(submission);

    // The rows the added companions become, borrowed off what was read back
    // above rather than off the submit: an empty branch name is *mirroring* and
    // a whitespace base is the rule, and both were settled once already.
    let joining: Vec<store::Joining<'_>> = added
        .iter()
        .map(|companion| store::Joining {
            repo_id: companion.repo.id,
            mode: companion.mode,
            base_ref: companion.base_ref.as_deref(),
            branch: &companion.branch,
        })
        .collect();

    // And the rows the opened companions become, borrowed the same way: the
    // branch each was given, mirroring where the field was left empty.
    let opening: Vec<store::Opening<'_>> = opened
        .iter()
        .map(|companion| store::Opening {
            repo_id: companion.repo.id,
            branch: &companion.branch,
        })
        .collect();

    let said = announced(&added, &opened, &conversation.branch);

    let settling = settling(&conversation, submission);

    let steer = store::Steer {
        target,
        pairings: &settling,
        brief: brief(submission),
        // Whichever of the two the target takes, both landing in the one place:
        // the Steer Event's own body is what the human wrote to steer it with.
        instruction: instruction.or(follow_up),
        direction: directing(&conversation, instruction),
        worktree: made.worktree.as_deref(),
        base_commit: made.base_commit.as_deref(),
        companions: &joining,
        opened: &opening,
        checkouts: &made.checkouts,
        said: said.as_deref(),
    };

    match store::steer_conversation(&state.pool, conversation_id, steer).await? {
        store::Steering::NoSuchConversation => return Ok(ConversationSteered::NoSuchConversation),
        store::Steering::NoSuchProfile => return Ok(ConversationSteered::NoSuchProfile),
        store::Steering::Steered => {}
    }

    // Recorded, so the sweep would keep them. What follows is a launch, and
    // holding a lock across one would hold every other start behind it.
    drop(making);

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
            // this a driver rather than an errand standing beside the work.
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

        // The wrap-up's five watchers over the top of whatever the branch now
        // holds, with the fix attempts forgotten: exactly what a pressed Resume
        // does for a wrapping Conversation, reused rather than forked. Each of
        // the five decides for itself whether there is anything left to do, so
        // there is nothing here that can come to nothing.
        SteerTarget::Wrapping => {
            let state = state.clone();

            tokio::spawn(async move {
                // Held until the five watchers have registrations of their own,
                // which is what [`crate::wrapping::watching`] takes as it spawns
                // them: dropping first would leave a moment where a sweep could
                // find the Conversation undriven all over again.
                let _driving = driving;

                crate::checks::afresh(state, conversation_id).await;
            });
        }

        // A session on the follow-up skill, started on the brief the human
        // steered with — which is required, so [`refusal`] has already made sure
        // there is one and this cannot come to nothing. See
        // [`crate::runner::following_up`], which is what drives the Conversation
        // while it runs.
        SteerTarget::FollowUp => match follow_up {
            Some(follow_up) => {
                tokio::spawn(crate::runner::following_up(
                    state.clone(),
                    conversation_id,
                    crate::follow_ups::FollowUp::opening(follow_up.to_owned()),
                    driving,
                ));
            }

            // Refused above, so nothing reaches here. Said rather than
            // unwrapped — a panic in a handler is a request the browser is left
            // holding — and nothing is refused at this end: the move is written
            // by now, and an answer saying it was refused would be an answer
            // about a Conversation that had already moved. What is left is a
            // Conversation nothing is driving, which the stall sweep says out
            // loud a minute later.
            None => {
                tracing::error!(
                    conversation_id,
                    "a steer into Follow-up got past the refusals with no brief, so nothing \
                     was started",
                );

                drop(driving);
            }
        },

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
    // into Wrapping is a move onto one that is already there — and a follow-up is
    // the human taking something up about work that is already pushed, so it
    // turns on the same fact and is refused by the same name. The Conversation's
    // own repository's either way: a companion's is something a wrap-up covers
    // rather than something that makes one. The modal offers neither where there
    // is none; this is the same rule asked again on arrival.
    if matches!(
        submission.target,
        SteerTarget::Wrapping | SteerTarget::FollowUp
    ) && store::pull_request(&state.pool, conversation.id, conversation.repo.id)
        .await?
        .is_none()
    {
        return Ok(Some(ConversationSteered::NoPullRequest));
    }

    // And a follow-up is whatever the human wrote it about. Nothing on the branch
    // could stand in for it — a follow-up is not a step of the run to be picked
    // up — so it is the one written payload with no quiet meaning, and the modal
    // holds the submit shut without one rather than offering it.
    if submission.target == SteerTarget::FollowUp && follow_up(submission).is_none() {
        return Ok(Some(ConversationSteered::NoFollowUpBrief));
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
        && !standing(
            conversation.direction,
            conversation.worktree.clone(),
            conversation.base_commit.clone(),
        )
        .await
        .offerable()
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

    let roles = roles(submission.target);

    if roles.is_empty() {
        return Ok(None);
    }

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
    // a move into work nothing could start. Every role the target runs under,
    // because any one of them missing is a session the wrap-up could not start.
    //
    // A role picked away is settled: what it says is that the state runs no
    // session there, which is not a session that could not be started.
    Ok(roles
        .iter()
        .any(|role| !fixed(conversation, *role).picked())
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

/// What the human wants followed up on, or `None` where they wrote nothing.
///
/// Read exactly as the instruction above it is, and it lands in the same place —
/// the Steer Event's own body. What differs is that `None` is a refusal rather
/// than an ordinary case: an instruction left empty carries on what the branch
/// holds, and there is nothing a follow-up could carry on instead. See
/// [`refusal`].
///
/// Only where a follow-up could be started on it. A brief that arrived beside
/// another target is a page sending a field it should not have drawn.
fn follow_up(submission: &SteerSubmission) -> Option<&str> {
    if submission.target != SteerTarget::FollowUp {
        return None;
    }

    submission
        .follow_up
        .as_deref()
        .map(str::trim)
        .filter(|follow_up| !follow_up.is_empty())
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

/// The Pairings to write with the move, one per role the target runs under —
/// empty where there are none to write.
///
/// Empty twice over: a target nothing runs in settles nothing, and a human who
/// left the picker alone has changed nothing — [`refusal`] has already made sure
/// that the Conversation's own is there in that case.
///
/// And a role the human picked away is left picked away. The modal's one pick is
/// what the sessions run under, and a role that runs none is not among them:
/// writing a Pairing over it would turn a review back on that nobody asked for.
///
/// **The review role is filled rather than rewritten**, which is the one role
/// this treats differently and the reason is what the picker says. It is
/// labelled for the state's own work and prefilled with what builds, so a human
/// who steers a wrap-up and changes nothing has picked nothing about the
/// review — and writing that prefill over an account they chose on the setup
/// card to be a fresh set of eyes would undo the whole point of picking it
/// apart. So it takes the pick only where nothing was picked for it, which is
/// still what lets a Conversation that never fixed one — a steered Draft — be
/// steered into a wrap-up at all.
fn settling<'a>(conversation: &Conversation, submission: &'a SteerSubmission) -> Vec<Settling<'a>> {
    let Some(choice) = submission.pairing.as_ref() else {
        return Vec::new();
    };

    roles(submission.target)
        .iter()
        .copied()
        .filter(|role| {
            let fixed = fixed(conversation, *role);

            // Nothing picked or nothing at all: the review takes this pick only
            // to fill a role that has none, never to replace one that has.
            if *role == Role::Review {
                return !fixed.picked();
            }

            // The Conversation's own is left exactly as it is rather than
            // rewritten with itself: a re-choice takes the model row away and
            // puts it back, and a Pairing that did not change is not something
            // to rewrite.
            //
            // Both halves, because both halves are what a pick is. The picker
            // offers one row per Profile-and-model, so the same Profile on
            // another of its models is a different pick — and a comparison that
            // asked about the Profile alone would answer *Steered* to a change
            // of model and write none of it.
            !fixed.skipped()
                && !fixed.pairing().is_some_and(|pairing| {
                    pairing.profile.id == choice.profile_id
                        && pairing.model.as_deref() == Some(&choice.model)
                })
        })
        .map(|role| Settling {
            role,
            profile_id: choice.profile_id,
            model: &choice.model,
        })
        .collect()
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
    /// [`plan`] — so a directory that has gone says nothing about what the
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
/// [`ConversationSteered::NoInstruction`], which is what a steer into
/// Implementing is refused with where nothing stands and nothing was written.
/// Two things can stand, and the
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
/// **One `stat` for whether there is a directory to read**, rather than asking
/// git whether it would still call the place a worktree. Two reasons, and the
/// second is the one that decides it: what is read below is `.tasks/` and what
/// the branch has written, and a directory that answers those answers them
/// however git feels about its registration — and this runs on every draw of
/// every Conversation, which a workbench re-reads on every Nudge, so three git
/// processes here would be three git processes a second on a page nobody is
/// touching. [`plan`] asks git the harder question a moment later, where it
/// is asked once and a rebuild turns on it.
///
/// Off the runtime's threads, every one of these readings being a filesystem or
/// git call.
pub(crate) async fn standing(
    direction: Option<Direction>,
    worktree: Option<PathBuf>,
    base_commit: Option<String>,
) -> Standing {
    // A Conversation that has never said how its work is built has nothing to
    // carry on whatever is on its branch: what says what is next is the
    // direction, and there is none.
    let Some(direction) = direction else {
        return Standing::Nothing;
    };

    if direction == Direction::Inline {
        return Standing::Nothing;
    }

    let Some(worktree) = worktree else {
        return Standing::Unreadable;
    };

    let there = {
        let worktree = worktree.clone();

        tokio::task::spawn_blocking(move || worktree.is_dir())
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
            let Some(base) = base_commit else {
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

/// Which roles' Pairings a target's sessions run under, empty where nothing
/// runs there.
///
/// Implementing, Wrapping and Follow-up share the implementation Pairing,
/// because they are one run seen at three moments: the task sessions build the
/// work, the wrap-up's watchers dispatch the fix and comment sessions that see
/// it through, and a follow-up session does whatever the human wants doing
/// about it afterwards. Every one of those is the work itself, so all of them
/// run under what builds. A grilling is its own, which is what an interview runs
/// under whatever else has happened since.
///
/// Wrapping is the one target that names two, because a wrap-up both builds and
/// reviews. What the human's one pick does to each of them is not the same
/// thing, though — see [`settling`]: it settles what builds, and it fills the
/// review role only where nothing was picked for it, which is what lets a
/// Conversation that has never fixed a review Pairing — a steered draft — be
/// steered into a wrap-up at all without replacing one that was chosen.
fn roles(target: SteerTarget) -> &'static [Role] {
    match target {
        SteerTarget::Grilling => &[Role::Grilling],
        SteerTarget::Implementing | SteerTarget::FollowUp => &[Role::Implementation],
        SteerTarget::Wrapping => &[Role::Implementation, Role::Review],
        SteerTarget::Done => &[],
    }
}

/// What the Conversation has settled for that role already, whole.
///
/// Both halves rather than the Profile it names, because both are what a pick is
/// held against — see [`settling`]. A Pairing carrying no model is a Profile
/// chosen before pairings existed, which is half a choice and so never the same
/// choice as one made now.
///
/// And whether the role was picked away rather than paired, which is settled
/// too: a Conversation that will not be grilled or will not be reviewed is not
/// one with a Pairing missing, and the difference is what the two readers below
/// turn on.
fn fixed(conversation: &Conversation, role: Role) -> store::Picked {
    match role {
        Role::Grilling => conversation.grilling_pairing.clone(),
        Role::Implementation => carried(conversation.implementation_pairing.as_ref()),
        Role::Review => conversation.review_pairing.clone(),
    }
}

/// A role that can only be paired or unpicked, read the same way as one that
/// can be picked away.
fn carried(pairing: Option<&store::Pairing>) -> store::Picked {
    pairing
        .cloned()
        .map_or(store::Picked::Nothing, store::Picked::Under)
}

/// The line that goes under the Steer, naming every companion the steer put in
/// and the mode it went in at, and every one it opened up and the branch it was
/// given — or `None` where it did neither.
///
/// What a Conversation was configured with is read on the Brief's details pane
/// ever after, and that pane says only what the set *is*. This is what says when
/// it changed and who changed it: it sits under the human's own Event, so the
/// Timeline reads as one moment rather than as a machine's aside beside it.
///
/// The mode with each addition, because that is the difference between a
/// repository a session may read and one it may commit in — and the whole of
/// what a companion costs whoever reads the work afterwards. The branch with
/// each upgrade, for the same reason turned the other way: a companion that may
/// now be committed in is a pull request somebody will have to look at, and the
/// branch is where to find it.
///
/// Two sentences where a steer did both, because they are two different things
/// to have done: one widened the set and the other opened a row that was
/// already in it.
fn announced(
    added: &[store::Companion],
    opened: &[store::Companion],
    branch: &str,
) -> Option<String> {
    let mut said = Vec::new();

    if !added.is_empty() {
        let named: Vec<String> = added
            .iter()
            .map(|companion| {
                let mode = match companion.mode {
                    store::CompanionMode::ReadOnly => "read-only",
                    store::CompanionMode::ReadWrite => "read-write",
                };

                format!("`{}` {mode}", companion.repo.name)
            })
            .collect();

        said.push(format!("Steered into the sandbox: {}.", named.join(", ")));
    }

    if !opened.is_empty() {
        // Mirroring resolved, which is the record's own business rather than
        // this line's: what the human reads is the branch that was cut, whether
        // they typed the name or left the field to follow the Conversation's.
        let named: Vec<String> = opened
            .iter()
            .map(|companion| {
                let cut = companion.branch_for(branch).unwrap_or_default();

                format!("`{}` on `{cut}`", companion.repo.name)
            })
            .collect();

        said.push(format!("Opened up for writing: {}.", named.join(", ")));
    }

    (!said.is_empty()).then(|| said.join(" "))
}

/// Which registered Repos the steer is putting into the sandbox, read back as
/// rows — or why one of them cannot go in.
///
/// The three questions the setup card asks the moment a row is pressed, asked
/// here because a steer is where they are asked past drafting: a Conversation is
/// not a companion of itself, a Repo added twice would be one repository with
/// two checkouts in one sandbox, and a Repo that is not registered is outside
/// the boundary Verkstead may operate in.
///
/// The Conversation's own and the set it already has come before the registry,
/// for the reason [`store::add_companion`] asks them in that order: they are the
/// cheaper questions and the more specific answers, and each of them can say
/// which repository it was. The registry cannot — see
/// [`ConversationSteered::NoSuchCompanionRepo`], which is the one refusal here
/// with no repository in it.
///
/// **Nothing here changes a row that is already there.** A submit naming a
/// companion the Conversation already has is refused rather than obeyed: the
/// frozen set only widens, and an add that landed on an existing row would be a
/// downgrade dressed as an add.
///
/// **And nothing at all where the target runs nothing.** Done has no sandbox to
/// set up, so the modal draws no section there and a submit carrying one is
/// answered the way [`brief`] and [`instruction`] answer a payload beside the
/// wrong target: as a page sending a field it should not have drawn.
///
/// What comes back is a [`store::Companion`] apiece, the shape the record holds
/// — so everything past this treats a companion the modal has just named and one
/// the Conversation has had all along as the one kind of thing.
async fn additions(
    state: &AppState,
    conversation: &Conversation,
    submission: &SteerSubmission,
) -> anyhow::Result<Additions> {
    // Nothing at all where the target runs nothing, whatever arrived: Done has
    // no sandbox to set up and nothing a companion could be for, so the modal
    // does not draw the section there and a submit carrying one is a page
    // sending a field it should not have drawn. The rule [`brief`] and
    // [`instruction`] follow, asked of the third payload.
    if !submission.target.runs() {
        return Ok(Additions::Ready(Vec::new()));
    }

    let mut added: Vec<store::Companion> = Vec::new();

    for addition in &submission.added {
        let refused = |repo: &str, why| {
            Additions::Refused(ConversationSteered::Companion {
                repo: repo.to_owned(),
                why,
            })
        };

        if addition.repo_id == conversation.repo.id {
            return Ok(refused(
                &conversation.repo.name,
                SteerCompanionRefusal::OwnRepo,
            ));
        }

        // The set as the record holds it and the set this submit has built up so
        // far, because one page sending the same Repo twice is the same mistake
        // as one sending a Repo that is there already.
        if let Some(already) = conversation
            .companions
            .iter()
            .map(|companion| &companion.repo)
            .chain(added.iter().map(|companion| &companion.repo))
            .find(|repo| repo.id == addition.repo_id)
        {
            return Ok(refused(&already.name, SteerCompanionRefusal::AlreadyAdded));
        }

        // On the registry, not merely in the table: a steer may widen what a
        // Conversation works alongside, and what it may widen to is what the
        // human has registered. One that has been taken off it is refused by the
        // same name as one that never existed.
        let Some(repo) = store::registered_repo(&state.pool, addition.repo_id).await? else {
            return Ok(Additions::Refused(ConversationSteered::NoSuchCompanionRepo));
        };

        added.push(store::Companion {
            repo,
            mode: mode(addition.mode),

            // Whitespace alone is the rule rather than a branch called nothing,
            // exactly as the setup card's picker records it: what the dropdown's
            // first entry sends is the override taken away.
            base_ref: addition
                .base_ref
                .as_deref()
                .map(str::trim)
                .filter(|base| !base.is_empty())
                .map(str::to_owned),
            branch: addition.branch.trim().to_owned(),

            // Nothing on disk yet, which is what makes it one of the checkouts
            // [`plan`] has to make.
            worktree: None,
            base_commit: None,
        });
    }

    Ok(Additions::Ready(added))
}

/// What reading the companions the modal named came to.
enum Additions {
    Ready(Vec<store::Companion>),

    /// One of them cannot go in, and the human is told which and why.
    Refused(ConversationSteered),
}

/// How far into a companion the modal said the work may reach.
///
/// The wire's word and the record's held to each other, which is what the two
/// vocabularies always need between them — see [`target`], which does the same
/// job for the states.
fn mode(mode: CompanionMode) -> store::CompanionMode {
    match mode {
        CompanionMode::ReadOnly => store::CompanionMode::ReadOnly,
        CompanionMode::ReadWrite => store::CompanionMode::ReadWrite,
    }
}

/// Which companions the steer is opening up, read back as the rows they will
/// become — or why one of them cannot be opened.
///
/// **One direction, and the payload is what makes it one.** An upgrade carries
/// a Repo and a branch name and nothing else, so read-only is not something
/// this can be asked for and neither is removal: what a session was once given
/// is never taken back mid-Conversation, and the wire cannot spell the taking
/// back. What is left to refuse is the two ways of naming the wrong row.
///
/// The Conversation's own set comes before the registry, for [`additions`]'
/// reason: it is the cheaper question and the more specific answer, and a Repo
/// this Conversation holds carries its name on the row. The registry is asked
/// only about an id the set does not answer to, and only to say which
/// repository the human was talking about — a registered Repo that is no
/// companion of this Conversation is named, and an unregistered one is
/// [`ConversationSteered::NoSuchCompanionRepo`], the refusal with no repository
/// in it.
///
/// **Nothing here changes a row that is already open.** A companion that is
/// read-write has nothing left to open, and obeying such a submit would be
/// cutting its branch a second time over whatever has been committed to the
/// first — see [`SteerCompanionRefusal::AlreadyReadWrite`]. Which is also what
/// answers a page naming one Repo twice: the second row meets the first one's
/// upgrade.
///
/// **And nothing at all where the target runs nothing**, for [`additions`]'
/// reason: Done has no sandbox to open up.
///
/// What comes back is a [`store::Companion`] apiece — the row as it will stand
/// after the steer, mode moved and branch set, with the worktree it is about to
/// lose still on it. That directory is what [`alongside`] replaces.
async fn upgrades(
    state: &AppState,
    conversation: &Conversation,
    submission: &SteerSubmission,
) -> anyhow::Result<Upgrades> {
    if !submission.target.runs() {
        return Ok(Upgrades::Ready(Vec::new()));
    }

    let mut opened: Vec<store::Companion> = Vec::new();

    for upgrade in &submission.upgraded {
        let refused = |repo: &str, why| {
            Upgrades::Refused(ConversationSteered::Companion {
                repo: repo.to_owned(),
                why,
            })
        };

        // The set this submit has opened so far before the record's own, so
        // that one page naming a Repo twice meets the row it has already
        // opened rather than the read-only one the record still holds.
        if let Some(already) = opened
            .iter()
            .find(|companion| companion.repo.id == upgrade.repo_id)
        {
            return Ok(refused(
                &already.repo.name,
                SteerCompanionRefusal::AlreadyReadWrite,
            ));
        }

        let Some(companion) = conversation
            .companions
            .iter()
            .find(|companion| companion.repo.id == upgrade.repo_id)
        else {
            // The Conversation's own repository first, which is the one Repo a
            // page could plausibly send here by mistake and the one the record
            // has already named; then the registry, which can say what any
            // other id is called; and then the refusal with nothing to name.
            let named = match upgrade.repo_id == conversation.repo.id {
                true => Some(conversation.repo.name.clone()),
                false => store::load_repo(&state.pool, upgrade.repo_id)
                    .await?
                    .map(|repo| repo.name),
            };

            let Some(named) = named else {
                return Ok(Upgrades::Refused(ConversationSteered::NoSuchCompanionRepo));
            };

            return Ok(refused(&named, SteerCompanionRefusal::NotACompanion));
        };

        if companion.mode == store::CompanionMode::ReadWrite {
            return Ok(refused(
                &companion.repo.name,
                SteerCompanionRefusal::AlreadyReadWrite,
            ));
        }

        opened.push(store::Companion {
            mode: store::CompanionMode::ReadWrite,
            branch: upgrade.branch.trim().to_owned(),

            // The base stays exactly as the human picked it while drafting.
            // What moves is where that name points: the upgrade fetches and
            // resolves it again, the companion joining the work now rather than
            // where that repository stood when the Conversation started.
            //
            // And the worktree it has now comes with it, which is the detached
            // directory this upgrade replaces.
            ..companion.clone()
        });
    }

    Ok(Upgrades::Ready(opened))
}

/// What reading the companions the modal opened up came to.
enum Upgrades {
    Ready(Vec<store::Companion>),

    /// One of them cannot be opened, and the human is told which and why.
    Refused(ConversationSteered),
}

/// Whether a companion is joining the work now, being opened up, or coming back
/// to a checkout it lost.
///
/// The one thing that reads differently between them, and it is the whole of why
/// this is a distinction: a branch already taken in that repository is somebody
/// else's work where a companion is joining, and is *this* companion's own work
/// where it is coming back. So the first is refused and the second is checked
/// out again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Joining {
    /// The modal has just named it. Everything about its checkout is new.
    Added,

    /// The modal has just opened it up. It is joining the work now as much as an
    /// added one is — the commit its detached checkout stands at is where that
    /// repository was when the Conversation started, and this is a companion
    /// beginning to be worked in *now* — so git is asked exactly what
    /// [`Self::Added`] is asked, in the same order.
    ///
    /// What is different is on the far side of it: the detached directory it
    /// had is replaced rather than left beside the new one. One companion is
    /// one checkout, and a detached checkout has nothing left to be.
    Opened,

    /// The record holds it and nothing on disk does: a steered Draft's
    /// companion, recorded on the setup card and never checked out, or one of a
    /// Conversation steered back out of Closed, whose directory was removed and
    /// whose row was forgotten while its branch was kept.
    Recorded,
}

/// Work out every checkout this steer has to make, and ask git everything each
/// of them turns on — without making any of it.
///
/// Nothing planned and nothing looked at where the target runs nothing: Done
/// needs no directory, and a steer into it must not turn on whether one is still
/// there.
///
/// **The Conversation's own** is the three cases, cheapest first, and which one
/// a Conversation is in is a fact about it rather than a choice:
///
/// - **A Worktree git can still answer about is left exactly as it stands**,
///   uncommitted changes and all — see [`crate::worktrees::healthy`], which is
///   the same reading a pressed Resume makes. There is nothing to make, so
///   nothing is planned for it.
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
/// **And every companion with nothing on disk**, which is three sources between
/// them: the ones the modal has just added, a steered Draft's — recorded on the
/// setup card and never checked out — and those of a Conversation steered back
/// out of Closed, whose directories were removed and whose rows were forgotten
/// while their branches were kept. All three would otherwise reach a running
/// state with companions the sandbox skips in silence, which is a session
/// quietly missing the repository it was given. A companion the record already
/// holds a directory for is left exactly where it is.
///
/// **Every question is asked before any of them is answered**, as at a grill
/// start, which is what lets a steer refused over one companion leave no
/// directory and no branch anywhere. See [`make`], which is the one place any of
/// this creates anything.
///
/// Off the runtime's threads, a checkout of a large repository being no quick
/// call and every part of this blocking.
async fn plan(
    state: &AppState,
    conversation: &Conversation,
    target: SteerTarget,
    added: &[store::Companion],
    opened: &[store::Companion],
) -> anyhow::Result<Planning> {
    if !target.runs() {
        return Ok(Planning::Ready(Planned::default()));
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
    let id = conversation.id;
    let data_dir = state.data_dir.clone();

    // What a branch that has never been cut comes off, named rather than
    // resolved: a drafting Conversation's column holds the *branch* the human
    // picked, and where they picked none the rule is the Repo's default branch.
    let named = conversation
        .base_commit
        .clone()
        .unwrap_or_else(|| conversation.repo.default_branch.clone());

    // Every companion this steer has to check out: the ones just added, the
    // ones just opened up, and the ones the record holds with nowhere on disk.
    // The added and the opened first because that is what the human is standing
    // at; nothing else turns on the order.
    //
    // A companion being opened is left out of the last group whether or not it
    // has a directory, because the upgrade is what says what its checkout will
    // be: one recorded with nowhere on disk — a steered Draft's read-only
    // companion, ticked up in the same modal — would otherwise be planned twice
    // and handed two directories.
    let opening: Vec<i64> = opened.iter().map(|companion| companion.repo.id).collect();

    let checking_out: Vec<(Joining, store::Companion)> = added
        .iter()
        .cloned()
        .map(|companion| (Joining::Added, companion))
        .chain(
            opened
                .iter()
                .cloned()
                .map(|companion| (Joining::Opened, companion)),
        )
        .chain(
            conversation
                .companions
                .iter()
                .filter(|companion| companion.worktree.is_none())
                .filter(|companion| !opening.contains(&companion.repo.id))
                .cloned()
                .map(|companion| (Joining::Recorded, companion)),
        )
        .collect();

    let planned = tokio::task::spawn_blocking({
        let path = path.clone();
        let branch = branch.clone();

        move || {
            let mut checkouts = Vec::new();
            let mut base_commit = None;

            // The Conversation's own first, so that whatever directory it takes
            // is taken before any companion asks for one.
            if !crate::worktrees::healthy(&repo, &path, &branch) {
                let holding = match crate::worktrees::branch_taken(&repo, &branch) {
                    true => Holding::Kept(branch.clone()),
                    false => {
                        let Some(commit) = crate::worktrees::resolve(&repo, &named) else {
                            return Err(ConversationSteered::NoBaseCommit);
                        };

                        base_commit = Some(commit.clone());

                        Holding::Cut(branch.clone(), commit)
                    }
                };

                checkouts.push(Checkout {
                    companion: None,
                    repo,
                    path: path.clone(),
                    holding,
                    replacing: None,
                });
            }

            for (joining, companion) in checking_out {
                // Whatever the Conversation's own holds and whatever has been
                // planned so far: until they exist the filesystem cannot tell
                // two directories apart, so two companions coming off one branch
                // name would otherwise be handed the same one. See
                // [`crate::worktrees::unclaimed_path`].
                let claimed: Vec<PathBuf> = std::iter::once(path.clone())
                    .chain(checkouts.iter().map(|checkout| checkout.path.clone()))
                    .collect();

                checkouts.push(alongside(
                    &data_dir, id, &branch, joining, companion, &claimed,
                )?);
            }

            Ok(Planned {
                worktree: Some(path),
                base_commit,
                checkouts,
            })
        }
    })
    .await?;

    Ok(match planned {
        Ok(planned) => Planning::Ready(planned),
        Err(refusal) => Planning::Refused(refusal),
    })
}

/// Ask git everything one companion's checkout turns on, and come back with what
/// it will be.
///
/// Fetch, then resolve, then check the branch — the grill start's order, for the
/// grill start's reasons, and each failure refused by the same name with the
/// repository said. Including the fetch that this module deliberately skips for
/// the Conversation's own repository: a companion joining now is new work rather
/// than an old checkout being put back, so it comes off what the remote is
/// holding at this moment. A companion whose repository has no remote has
/// nothing to fetch and is never refused for it.
///
/// **A companion being opened up is asked all of it too**, and for the same
/// reason twice over: the commit its detached checkout stands at is where that
/// repository was when the Conversation started, and it is joining the work
/// now. So the base on its row is re-resolved against what the fetch has just
/// brought down, and the branch is cut from that tip. What it had is not a
/// branch and cannot be carried forward — a detached checkout holds none — so
/// there is nothing here that could be kept.
///
/// **A companion coming back to a branch it still holds asks git none of it.**
/// That branch was cut for this Conversation and holds everything that was
/// committed to it, so it is checked out again rather than cut, and there is no
/// base for a branch that is already there to come off — which is why the row
/// that comes back records no base commit. See [`Joining`], which is the whole
/// of the difference between that and the same branch under a companion joining
/// now: there, a name already taken is somebody else's work and refuses the
/// steer.
///
/// Otherwise a read-write companion is cut a branch of its own from its base,
/// mirroring the Conversation's where the row names none, and a read-only one is
/// checked out detached at whatever that base comes to at this moment — that
/// being the only commit anything can still name.
///
/// Nothing is made here. What comes back is a plan, and the making waits until
/// every checkout has one: that is what lets a steer that cannot deliver one
/// companion refuse without having made another.
fn alongside(
    data: &Path,
    id: i64,
    branch: &str,
    joining: Joining,
    companion: store::Companion,
    claimed: &[PathBuf],
) -> Result<Checkout, ConversationSteered> {
    let repo = companion.repo.path.clone();
    let refused = |why| ConversationSteered::Companion {
        repo: companion.repo.name.clone(),
        why,
    };

    // A read-write companion is cut a branch of its own: the one that was typed,
    // or the Conversation's where nothing was, which is what mirroring is. A
    // read-only one takes no name at all.
    let cut = companion.branch_for(branch);

    // The one checkout that asks git nothing else: the branch is there, it is
    // this companion's own, and what is missing is only the directory it was
    // worked in.
    if joining == Joining::Recorded
        && let Some(cut) = &cut
        && crate::worktrees::branch_taken(&repo, cut)
    {
        return Ok(Checkout {
            companion: Some((companion.repo.id, companion.repo.name.clone())),
            path: crate::worktrees::unclaimed_path(data, id, &companion.repo.name, cut, claimed),
            repo,
            holding: Holding::Kept(cut.clone()),
            replacing: None,
        });
    }

    // The detached directory an upgrade takes the place of, carried through to
    // [`make`] and removed there rather than here: nothing is unmade until every
    // checkout has been planned and made, which is what leaves a companion the
    // steer then refuses over read-only with its checkout exactly where it was.
    //
    // `None` on anything else, and on an opened companion the record holds no
    // directory for — a steered Draft's, which was never checked out.
    let replacing = match joining {
        Joining::Opened => companion.worktree.clone(),
        Joining::Added | Joining::Recorded => None,
    };

    if let crate::worktrees::Fetched::Failed(said) = crate::worktrees::fetch(&repo) {
        tracing::error!(
            said,
            repo = %repo.display(),
            "fetching a companion Repo's remotes failed, so the steer is not being made",
        );

        return Err(refused(SteerCompanionRefusal::FetchFailed));
    }

    // The branch of that repository's own the human picked, or its default
    // branch as origin holds it — the rule the Conversation's base follows,
    // asked of the companion's repository.
    let named = match companion.base_ref.clone() {
        Some(picked) => picked,
        None => crate::worktrees::default_ref(&repo, &companion.repo.default_branch),
    };

    let Some(commit) = crate::worktrees::resolve(&repo, &named) else {
        return Err(refused(SteerCompanionRefusal::NoBaseCommit));
    };

    if let Some(cut) = &cut
        && crate::worktrees::branch_exists(&repo, cut)
    {
        return Err(refused(SteerCompanionRefusal::BranchExists));
    }

    // Named for the Repo and what the checkout holds, as the Conversation's own
    // is: the branch where there is one, and otherwise the base it stands at —
    // a read-only companion holds no branch to be named for.
    let holds = cut.clone().unwrap_or(named);

    Ok(Checkout {
        companion: Some((companion.repo.id, companion.repo.name.clone())),
        path: crate::worktrees::unclaimed_path(data, id, &companion.repo.name, &holds, claimed),
        repo,
        holding: match cut {
            Some(cut) => Holding::Cut(cut, commit),
            None => Holding::Detached(commit),
        },
        replacing,
    })
}

/// Make every checkout the plan holds, or unmake the ones already made and say
/// which one would not be.
///
/// The one place a steer creates anything, which is what makes *leaves nothing
/// behind* something to hold rather than something to hope for. What is unwound
/// is directory and branch together for a branch this steer cut — see
/// [`crate::worktrees::unmake`], a branch cut moments ago by a steer that then
/// refused holding nothing worth keeping — and the directory alone for one it
/// did not: a branch that was already there is work somebody committed, and a
/// checkout taken away again is the nothing this Conversation already had.
///
/// **And then, and only then, the directories the new ones replace**: the
/// detached checkout of every companion the steer opened up. Last because a
/// refusal anywhere in the plan has to leave those companions read-only with
/// their checkouts where they were, and a directory removed on the way through
/// could not be put back.
///
/// Off the runtime's threads, every part of this blocking.
async fn make(planned: Planned) -> anyhow::Result<Making> {
    let Planned {
        worktree,
        base_commit,
        checkouts,
    } = planned;

    let made = tokio::task::spawn_blocking(move || {
        for (nth, checkout) in checkouts.iter().enumerate() {
            let made = match &checkout.holding {
                Holding::Cut(branch, commit) => {
                    crate::worktrees::add(&checkout.repo, &checkout.path, branch, commit)
                }
                Holding::Kept(branch) => {
                    crate::worktrees::rebuild(&checkout.repo, &checkout.path, branch)
                }
                Holding::Detached(commit) => {
                    crate::worktrees::add_detached(&checkout.repo, &checkout.path, commit)
                }
            };

            if made {
                continue;
            }

            // This one included, and first: an `add` that fell over may have
            // made the directory, or the branch, or neither, and what is being
            // unwound is whatever it did get as far as. The rest newest first,
            // which is the order they were made in reversed.
            for done in checkouts[..=nth].iter().rev() {
                crate::worktrees::unmake(&done.repo, &done.path, done.holding.cut());
            }

            return Err(checkout.refused());
        }

        // And only now the directories the new ones take the place of: the
        // detached checkout of every companion that was opened up. After the
        // whole plan is made rather than beside each one, which is what leaves
        // a companion the steer went on to refuse over read-only with its
        // checkout exactly where it stood.
        //
        // Nothing is refused for a directory that will not go. The branch is
        // cut, the new worktree is there and the record is about to name it, so
        // what is left is a directory nothing points at — worth the log it gets
        // and not worth undoing the steer for.
        for checkout in &checkouts {
            let Some(replacing) = &checkout.replacing else {
                continue;
            };

            if !crate::worktrees::remove(&checkout.repo, replacing) {
                tracing::error!(
                    path = %replacing.display(),
                    "the detached checkout of a companion a steer opened up could not be removed",
                );
            }
        }

        Ok(recorded(&checkouts))
    })
    .await?;

    Ok(match made {
        // The path either way, healthy or made: the record may never have held
        // one, and writing back the one it did hold changes nothing.
        Ok(checkouts) => Making::Ready(Made {
            worktree,
            base_commit,
            checkouts,
        }),
        Err(refusal) => Making::Refused(refusal),
    })
}

/// Where each companion checkout of a steer went and what it came off, for the
/// record that follows the work.
///
/// The Conversation's own is not among them: it goes on the row the store has
/// always kept for it, one per Conversation.
fn recorded(checkouts: &[Checkout]) -> Vec<store::CompanionWorktree> {
    checkouts
        .iter()
        .filter_map(|checkout| {
            let (repo_id, _) = checkout.companion.as_ref()?;

            Some(store::CompanionWorktree {
                repo_id: *repo_id,
                path: checkout.path.clone(),
                base_commit: checkout.holding.commit().map(str::to_owned),
            })
        })
        .collect()
}

/// One checkout a steer is about to make: which repository, where it goes and
/// what it will hold.
///
/// The Conversation's own and each companion in the one shape, because from the
/// moment they are planned they are the same thing — a worktree of a registered
/// repository. What differs between them is which repository is named where git
/// will not make it.
struct Checkout {
    /// The companion Repo this is a checkout of — its id and what it is called —
    /// or `None` for the Conversation's own.
    companion: Option<(i64, String)>,

    /// The repository the worktree is made from.
    repo: PathBuf,

    /// Where the checkout goes, under the Data Directory.
    path: PathBuf,

    holding: Holding,

    /// The directory this checkout takes the place of, where it takes the place
    /// of one: the detached checkout of a companion being opened up.
    ///
    /// Removed once every checkout is made rather than before this one is — see
    /// [`make`]. One companion is one checkout, and a detached directory has
    /// nothing left to be once the branch is cut; but a steer refused over
    /// another companion has to leave this one exactly where it was, and a
    /// directory removed early could not be put back.
    ///
    /// `None` on everything else, including an opened companion the record
    /// holds no directory for.
    replacing: Option<PathBuf>,
}

impl Checkout {
    /// How git refusing to make this checkout is refused back: the
    /// Conversation's own repository says only that git would not, and a
    /// companion says which repository it was.
    fn refused(&self) -> ConversationSteered {
        match &self.companion {
            Some((_, repo)) => ConversationSteered::Companion {
                repo: repo.clone(),
                why: SteerCompanionRefusal::WorktreeRefused,
            },
            None => ConversationSteered::WorktreeRefused,
        }
    }
}

/// What a checkout will hold, which is the whole of how it is made.
enum Holding {
    /// A branch to cut, and the commit its base resolved to.
    Cut(String, String),

    /// A branch that is already there, checked out again into a directory that
    /// has gone — the Conversation's own after a close, and a read-write
    /// companion's the same way. Nothing was resolved for it: the branch holds
    /// what was committed to it, and what it was cut from is not this steer's to
    /// say.
    Kept(String),

    /// No branch at all: a read-only companion, detached at the commit its base
    /// resolved to.
    Detached(String),
}

impl Holding {
    /// The branch this steer would be cutting, which is the only one an unwind
    /// may take away — see [`make`].
    fn cut(&self) -> Option<&str> {
        match self {
            Self::Cut(branch, _) => Some(branch.as_str()),
            Self::Kept(_) | Self::Detached(_) => None,
        }
    }

    /// The commit a base resolved to, where anything resolved one.
    fn commit(&self) -> Option<&str> {
        match self {
            Self::Cut(_, commit) | Self::Detached(commit) => Some(commit.as_str()),
            Self::Kept(_) => None,
        }
    }
}

/// What working out where a steer will run came to.
enum Planning {
    Ready(Planned),

    /// Something the record or git was asked says it cannot run there, and the
    /// human is told which way — see [`ConversationSteered::NoBaseCommit`] and
    /// [`ConversationSteered::Companion`].
    Refused(ConversationSteered),
}

/// Everywhere the steer will work, before any of it exists.
///
/// All of it empty for a target nothing runs in, which is what [`Default`] is
/// here for: a steer into Done plans nothing and writes nothing about a
/// directory.
#[derive(Default)]
struct Planned {
    /// Where the Conversation's own work goes on.
    worktree: Option<PathBuf>,

    /// What its branch will be cut from, where this steer is what cuts it.
    base_commit: Option<String>,

    /// Every checkout to make, the Conversation's own first.
    checkouts: Vec<Checkout>,
}

/// What making them came to.
enum Making {
    Ready(Made),

    /// Git would not, and the human is told which way it would not — see
    /// [`ConversationSteered::WorktreeRefused`] and
    /// [`ConversationSteered::Companion`].
    Refused(ConversationSteered),
}

/// What the steer has to record about where the work will run.
///
/// All of it empty for a target nothing runs in: a steer into Done makes nothing
/// and writes nothing about a directory.
struct Made {
    /// Where the work goes on.
    worktree: Option<PathBuf>,

    /// What the branch was cut from, where this is what cut it. `None` on every
    /// Conversation that had a branch already: what it branched from was
    /// resolved once, and it is not resolved again.
    base_commit: Option<String>,

    /// And where each companion's checkout went.
    checkouts: Vec<store::CompanionWorktree>,
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
        SteerTarget::FollowUp => Lifecycle::FollowUp,
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
