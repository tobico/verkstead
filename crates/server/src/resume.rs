//! Start driving a Conversation again: one press, and what it recomputes.
//!
//! **One standing way in.** Not something answered on the Event that stopped
//! the run, and not a step run again: Resume asks what *ought* to be running now,
//! from the lifecycle the Conversation is in and what the branch has written,
//! and starts that. Which is why it is offered on a Conversation that is merely
//! undriven as much as on one that has stopped — a run with nothing behind it is
//! the same condition however it got there, and a restarted server has a page
//! full of them.
//!
//! Read now rather than off whatever stopped. A stop may be answered the next
//! morning, and a Conversation moves on in the meantime — the human works the
//! Worktree by hand, a Manual Task lands a commit, the backlog it stopped in the
//! middle of is finished with. The step that failed is not the question; what
//! there is left to do is.
//!
//! **It is never silent.** Either something starts — which needs no
//! announcement, the session showing up on the Timeline — or it refuses by
//! name, and the page says which. The recompute's own bails become those
//! refusals: what used to be a line in the log about a backlog that had gone is
//! now a sentence in front of the human who pressed the button. A press that
//! decided there was nothing to do and said nothing is the whole of what this
//! replaces.
//!
//! **The checks happen here and the work happens after.** Everything that can
//! refuse is asked before anything is spawned, because a refusal is the press's
//! answer and the browser is holding the request open for it. What follows the
//! spawn reads the record again for itself — where an agent is about to be let
//! loose is the one thing that must not be guessed at, and a spawn is a moment
//! later.
//!
//! **And a restart presses it for itself.** No driver survives the process, so a
//! server coming up holds a page full of Conversations nothing is driving, and
//! every one of them wants exactly what the button does. So it does it unasked,
//! on each in turn — see [`at_startup`]. What it leaves alone is a stop somebody
//! decided on, that being the one kind waiting for a press rather than for a
//! server.

use verkstead_render::Resumed;
use verkstead_schema::{Direction, Nudge};

use crate::AppState;
use crate::store::{self, Decision, Lifecycle};

/// Who asked for the run to start again.
///
/// One thing turns on it, and only one: the fix attempts a wrapping
/// Conversation's checks have already spent. A human who has read what stopped
/// and pressed Resume is asking for another go, so the counters are forgotten;
/// a server coming back up has read nothing and asked for nothing, and an
/// attempt spent before the restart is one it must not spend again — see
/// [`crate::checks`]. Everything else about the recompute is the same either
/// way, which is the whole point of there being one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Resuming {
    /// The human pressed the button.
    Pressed,

    /// A server came up over a Conversation the last one was driving.
    Restarted,
}

/// Press Resume: recompute what should be driving this Conversation, clear the
/// stop, and start it.
///
/// `resuming` is who pressed it — the human, or a server coming up over a
/// Conversation the last one was driving. The recompute is the same for both, and
/// [`Resuming`] says what the one difference is.
///
/// The registration is taken here rather than by whatever is spawned, and that
/// is the whole reason the two are separate: the launch is the slow part, and
/// every moment of it is a Conversation being driven with nothing on the
/// register — which the stall sweep would read as a Conversation standing still
/// and stop all over again. So it is held across the decision and handed on
/// whole. See [`crate::drivers`].
///
/// Clearing the stop goes last of the things done before the spawn, and it has
/// to: the run does not advance past one — see [`crate::stopping::stopped`] — so a
/// driver launched over one would find the Conversation stopped and launch
/// nothing.
pub(crate) async fn resume(
    state: &AppState,
    conversation_id: i64,
    resuming: Resuming,
) -> anyhow::Result<Resumed> {
    let Some(conversation) = store::load_conversation(&state.pool, conversation_id).await? else {
        return Ok(Resumed::NoSuchConversation);
    };

    // Nothing drives a Conversation that is drafting, done or aborted, so there
    // is nothing to start driving again. The button is not drawn on one — this
    // is the same rule asked again on arrival, the way every named refusal here
    // is.
    if !matches!(
        conversation.state,
        Lifecycle::Grilling | Lifecycle::Implementing | Lifecycle::Wrapping
    ) {
        return Ok(Resumed::NotDriven);
    }

    // Whether the run has already stopped, which is the record's answer to the
    // same question the register answers — and the one that wins where they
    // disagree. A stop is Verkstead saying *nothing is driving this any more*,
    // written as the run stopped; what may still be registered behind one is a
    // watcher winding down or a poll loop with nothing left to do, and a
    // Conversation the human is told to press Resume on must not refuse the
    // press on account of one.
    let stopped = store::stopped(&state.pool, conversation_id)
        .await?
        .is_some();

    // A session running in the Worktree is the one thing nothing may be started
    // over, stop or no stop: two agents in one Worktree is the failure every
    // launch here is arranged to make impossible. So the stop is taken away —
    // which is the whole of what Resume can add to a Conversation something is
    // already working in — and nothing is launched here.
    //
    // Every stop is written with nothing running or ends what was, an exhausted
    // window included, so a stop that reaches this is one caught in the moment
    // between the two: a session asked to end and not yet reaped. Nothing
    // advances out of that moment either, a session Verkstead ended advancing
    // nothing — so what this press buys is the badge going, and the press after
    // it starts the work.
    if state.sessions.working().contains(&conversation_id) {
        if stopped {
            clear(state, conversation_id).await?;
        }

        return Ok(Resumed::AlreadyDriven);
    }

    // Otherwise it is the register that says it, because that is where the
    // answer is: a Conversation is driven by tasks and sessions, and neither
    // leaves a row behind. The second press of the button lands here — the first
    // took the stop away and started something.
    if !stopped
        && state.drivers.driven(
            &state.sessions.working(),
            conversation_id,
            conversation.state,
        )
    {
        return Ok(Resumed::AlreadyDriven);
    }

    // Taken before the reading that follows it and before the spawn, for the
    // reason above. Dropped on every path out that refuses — which is what
    // leaving it to the `?` and the early returns does.
    let driving = state.drivers.driving(conversation_id);

    // Every state past drafting has a Worktree, so one missing from the record
    // is a record that cannot be true. There is nowhere for a session to run and
    // nothing to make a worktree from either — the path is Verkstead's own to
    // have chosen, and nothing here knows which one it chose.
    let Some(worktree) = conversation.worktree.clone() else {
        return Ok(Resumed::NowhereToWork);
    };

    // What the record names, though, may be nowhere: a directory deleted,
    // hollowed out, or dropped from the repository's list of worktrees. Which is
    // a Conversation stuck for good under a button whose whole job is to unstick
    // one, so Resume makes it again from the branch rather than refusing on it.
    // Nothing is lost by that — a worktree is derived state — and nothing
    // healthy is touched, uncommitted changes and all. See
    // [`crate::worktrees::healthy`].
    //
    // Off the runtime's threads: a checkout of a large repository is not a quick
    // call, and every part of this blocks. Under the registration taken above,
    // because a Conversation being rebuilt is a Conversation being driven — a
    // sweep that found it undriven meanwhile would stop it all over again.
    let usable = tokio::task::spawn_blocking({
        let repo = conversation.repo.path.clone();
        let branch = conversation.branch.clone();
        let worktree = worktree.clone();

        move || {
            crate::worktrees::healthy(&repo, &worktree, &branch)
                || crate::worktrees::rebuild(&repo, &worktree, &branch)
        }
    })
    .await?;

    if !usable {
        return Ok(Resumed::WorktreeRefused);
    }

    match conversation.state {
        // A fresh grilling on the Brief and the digest of what has already been
        // asked and answered — the interview itself went with the process it ran
        // in. See [`crate::grillings::again`].
        Lifecycle::Grilling => {
            if conversation.grilling_pairing.is_none() {
                return Ok(Resumed::NoGrillingPairing);
            }

            clear(state, conversation_id).await?;

            tokio::spawn(crate::grillings::again(
                state.clone(),
                conversation_id,
                driving,
                // Everything the human has already answered, always: this press
                // is a relaunch of the interview that died, and one that opened
                // by asking again what they had already decided would cost them
                // the interview twice. A steer is where that becomes a choice —
                // see [`crate::steering`].
                crate::grillings::Digest::Prime,
            ));
        }

        // The next step read off the branch, which is the direction's to say.
        Lifecycle::Implementing => {
            // A Conversation implements because a direction was picked, so a
            // missing one is another record that cannot be true — and the one
            // thing that says which run it is that stopped.
            let Some(direction) = conversation.direction else {
                return Ok(Resumed::NoDirection);
            };

            // What every session of the work itself runs under. Checked here
            // rather than left to the launch, which would log it and start
            // nothing — see [`crate::runner`].
            if conversation.implementation_pairing.is_none() {
                return Ok(Resumed::NoImplementationPairing);
            }

            // And the backlog's own answer to what is next, asked of `.tasks/`
            // exactly as every other turn of the run asks it. Nothing left in it
            // is a breakdown that never landed or a feature that is finished
            // with, and neither is a thing to launch a session for.
            if direction == Direction::TaskList && !crate::runner::anything_to_work(&worktree).await
            {
                return Ok(Resumed::NothingToWork);
            }

            clear(state, conversation_id).await?;

            tokio::spawn(crate::runner::implementing_again(
                state.clone(),
                conversation_id,
                driving,
            ));
        }

        // The wrap-up's watchers over the top of nothing, which is what a
        // restarting server does with a Conversation it left wrapping up. Each
        // of the four decides for itself whether there is anything left to do,
        // so there is nothing here that can come to nothing.
        //
        // The fix attempts are forgotten first: the human has read what stopped
        // and asked for another go, and a count left standing would be a watcher
        // that stopped again on its next poll without dispatching anything. See
        // [`crate::checks::afresh`].
        Lifecycle::Wrapping => {
            clear(state, conversation_id).await?;

            match resuming {
                Resuming::Pressed => {
                    let state = state.clone();

                    tokio::spawn(async move {
                        // Held until the four watchers have registrations of
                        // their own, which is what
                        // [`crate::wrapping::watching`] takes as it spawns them:
                        // dropping first would leave a moment where a sweep
                        // could find the Conversation undriven all over again.
                        let _driving = driving;

                        crate::checks::afresh(state, conversation_id).await;
                    });
                }
                // The same four watchers with the counters left standing — see
                // [`Resuming`]. Registered before this one is let go, each of
                // the four taking a registration of its own as it is spawned,
                // which is the handover the press makes by holding its across
                // the spawn.
                Resuming::Restarted => {
                    crate::wrapping::watching(state, conversation_id);
                    drop(driving);
                }
            }
        }

        // Refused above, where the refusal belongs: before the stop is read and
        // before a registration is taken. Answered again here because the match
        // has to be whole, and answered the same way.
        Lifecycle::Draft | Lifecycle::Done | Lifecycle::Aborted => {
            return Ok(Resumed::NotDriven);
        }
    }

    tracing::info!(
        conversation_id,
        state = ?conversation.state,
        ?resuming,
        "the Conversation is being driven again"
    );

    Ok(Resumed::Resumed)
}

/// Start driving again everything a restart left, which is what a server does as
/// it comes up.
///
/// The whole of what a restart is: no driver survives the process, so every
/// Conversation the last server was driving is one nothing is driving now — a
/// grilling whose session was killed, a backlog between tasks, a wrap-up whose
/// watchers went with the process. Each of them gets the recompute the button
/// gives, and starts driving again with nobody asked. There is nothing for a
/// human to press here: a restart is not a decision anybody took, and a
/// Conversation left standing still because Verkstead was upgraded is one that
/// would wait for however long it took somebody to notice.
///
/// **Except where somebody decided to stop.** A [`Decision::Deliberate`] is
/// Verkstead or the human pulling the brake — the checks that would not go green,
/// a finish step with no pull request, a Stop pressed from the menu, an account
/// out of window — and a server coming back up is no reason to think differently
/// about any of them. Those keep their badge and wait for the press. A
/// [`Decision::Circumstance`] is the other half of the same record: nobody chose
/// it, so it is taken up here.
///
/// A refusal is written down rather than logged, because a Conversation that
/// cannot be started is exactly the one somebody has to look at: it stops, with
/// the refusal as its Notice — see [`refused`].
///
/// The task is handed back rather than let go, because the stall sweep waits for
/// it: every Conversation here is undriven until this has taken it up, and a
/// sweep that looked first would call each of them stalled. See
/// [`crate::stalls::sweeping`].
#[must_use = "the sweep waits for what a restart takes up before it judges \
              whether anything is driving it"]
pub(crate) fn at_startup(state: &AppState) -> tokio::task::JoinHandle<()> {
    let state = state.clone();

    tokio::spawn(async move {
        let conversations = match store::conversations(&state.pool).await {
            Ok(conversations) => conversations,
            Err(error) => {
                tracing::error!(error = ?error, "listing the Conversations a restart left failed");
                return;
            }
        };

        for conversation in conversations {
            if !matches!(
                conversation.state,
                Lifecycle::Grilling | Lifecycle::Implementing | Lifecycle::Wrapping
            ) {
                continue;
            }

            match waiting_for_a_press(&state, conversation.id).await {
                Ok(true) => continue,
                Ok(false) => {}
                Err(error) => {
                    tracing::error!(error = ?error, conversation_id = conversation.id, "reading whether a Conversation was waiting on the human failed");
                    continue;
                }
            }

            tracing::info!(
                conversation_id = conversation.id,
                state = ?conversation.state,
                "a restart left a Conversation with nothing driving it, so it is being driven again",
            );

            match resume(&state, conversation.id, Resuming::Restarted).await {
                Ok(Resumed::Resumed) => {}
                Ok(refusal) => refused(&state, conversation.id, conversation.state, refusal).await,
                Err(error) => {
                    tracing::error!(error = ?error, conversation_id = conversation.id, "starting to drive a Conversation a restart left failed");
                }
            }
        }
    })
}

/// Whether this Conversation is stopped in a way only the human can undo, which
/// is the one thing a restart leaves alone.
///
/// A [`Decision::Deliberate`] is what says it: somebody decided, so nothing here
/// decides otherwise. See [`crate::stopping::stopped`], which asks the same
/// question in front of a launch.
async fn waiting_for_a_press(state: &AppState, conversation_id: i64) -> anyhow::Result<bool> {
    Ok(store::stopped(&state.pool, conversation_id)
        .await?
        .is_some_and(|stopped| stopped.decision == Decision::Deliberate))
}

/// Stop a Conversation a restart could not start anything for, with the refusal
/// as its Notice.
///
/// A refusal at startup has nobody in front of it: the press answers the browser
/// that is holding a request open, and this answers nothing at all. So it goes
/// where the human will find it — the Timeline, in the words the button would
/// have used — and the Conversation stops rather than being swept a minute later
/// under *nothing is driving it*, which is the same Conversation described by
/// something that knows less.
///
/// [`Decision::Deliberate`], which is what it is: Verkstead looked at this
/// Conversation and decided nothing could be started for it. Nothing but the
/// human can change that, so the next restart leaves it alone rather than
/// refusing all over again — and it reaches a phone, a Conversation nothing will
/// ever pick up unasked being exactly the kind worth being told about.
///
/// A Conversation already stopped keeps the stop it has: there is one per
/// Conversation, and the first Notice is the one that explains it.
async fn refused(state: &AppState, conversation_id: i64, lifecycle: Lifecycle, refusal: Resumed) {
    let Some(why) = why(refusal) else {
        tracing::info!(
            conversation_id,
            ?refusal,
            "a Conversation moved on before a restart could take it up, so nothing was written",
        );
        return;
    };

    let stopped = crate::stopping::stop(
        &state.pool,
        &state.nudges,
        conversation_id,
        crate::stopping::Decided::Verkstead,
        crate::stalls::driving(lifecycle),
        &format!("nothing could be started for it as the server came back up: {why}"),
        crate::stalls::said_last(state, conversation_id).await,
    )
    .await;

    if let Err(error) = stopped {
        tracing::error!(
            error = ?error,
            conversation_id,
            ?refusal,
            "nothing could be started for a Conversation and the stop saying so could not be recorded"
        );
    }
}

/// Each refusal in the words the Notice says it in, or `None` where there is
/// nothing to write down.
///
/// The three that answer `None` are not refusals about the work at all: the
/// Conversation went, or moved out of a driven state, or something took it up
/// between the listing and the resuming. Whatever did that has already said so,
/// and a stop about it would be a stop nobody could act on.
///
/// The rest are the words the button's own refusals use, said as the reason half
/// of a Notice — see the viewer's `RESUME_REFUSAL` for the same list put to
/// somebody who is looking at the page.
fn why(refusal: Resumed) -> Option<&'static str> {
    Some(match refusal {
        Resumed::Resumed => return None,
        Resumed::NoSuchConversation | Resumed::NotDriven | Resumed::AlreadyDriven => return None,
        Resumed::NowhereToWork => "there is no Worktree on the record to work in",
        Resumed::WorktreeRefused => {
            "its Worktree is not one any more, and git would not make it again from the branch"
        }
        Resumed::NoDirection => "nothing on the record says how the work is being built",
        Resumed::NothingToWork => {
            "there is nothing left in `.tasks/` to work — the backlog was never written, \
             or it is finished with"
        }
        Resumed::NoGrillingPairing => "the grilling Profile and model it runs under have gone",
        Resumed::NoImplementationPairing => {
            "the implementation Profile and model the work runs under have gone"
        }
    })
}

/// Take the stop away, and tell the open pages the badge has gone.
///
/// The Notice stays where it is: it is a record of a stop that really happened.
/// What goes is the state — the badge, the reset words beside the button, and
/// the guard that stops anything being launched behind it.
///
/// And a Stop asked for that has not landed goes with it, for the same reason:
/// what is being started here is the very thing it asked to come before, so a
/// request left behind would stop the run again at its next step. See
/// [`crate::stops`].
///
/// Nothing to clear is the ordinary case rather than a mistake: Resume is
/// offered on a Conversation that is merely undriven as much as on one that
/// stopped, and a restarted server's are all of the first kind.
async fn clear(state: &AppState, conversation_id: i64) -> anyhow::Result<()> {
    store::clear_stop(&state.pool, conversation_id).await?;
    store::forget_stop(&state.pool, conversation_id).await?;

    state.nudges.announce(Nudge::Conversation {
        conversation: conversation_id,
    });

    Ok(())
}

/// Whether Resume is worth offering: the Conversation is in a state something
/// ought to be driving, and nothing is.
///
/// The rule the button is drawn by, said once here so that the page and the
/// press cannot come to different answers about it. It is deliberately the
/// smaller half of what [`resume`] asks — everything else is a refusal the human
/// gets to read, and a button that hid itself rather than saying *the backlog
/// has gone* would leave them exactly as stuck as the run stopping did.
pub(crate) fn ready(
    state: &AppState,
    conversation_id: i64,
    lifecycle: Lifecycle,
    stopped: bool,
) -> bool {
    matches!(
        lifecycle,
        Lifecycle::Grilling | Lifecycle::Implementing | Lifecycle::Wrapping
    ) && (stopped
        || !state
            .drivers
            .driven(&state.sessions.working(), conversation_id, lifecycle))
}
