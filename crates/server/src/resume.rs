//! Start driving a Conversation again: one press, and what it recomputes.
//!
//! **One standing way in.** Not a remedy taken on the Event that stopped the
//! run, and not a step run again: Resume asks what *ought* to be running now,
//! from the lifecycle the Conversation is in and what the branch has written,
//! and starts that. Which is why it is offered on a Conversation that is merely
//! undriven as much as on one that has halted — a run with nothing behind it is
//! the same condition however it got there, and a restarted server has a page
//! full of them.
//!
//! Read now rather than off whatever stopped. A halt may be answered the next
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

use verkstead_render::Resumed;
use verkstead_schema::{Direction, Nudge};

use crate::AppState;
use crate::store::{self, Lifecycle};

/// Press Resume: recompute what should be driving this Conversation, clear the
/// halt, and start it.
///
/// The registration is taken here rather than by whatever is spawned, and that
/// is the whole reason the two are separate: the launch is the slow part, and
/// every moment of it is a Conversation being driven with nothing on the
/// register — which the stall sweep would read as a Conversation standing still
/// and halt all over again. So it is held across the decision and handed on
/// whole. See [`crate::drivers`].
///
/// The halt goes last of the things done before the spawn, and it has to: the
/// run does not advance past a halt — see [`crate::halts::stopped`] — so a
/// driver launched over one would find the Conversation stopped and launch
/// nothing.
pub(crate) async fn resume(state: &AppState, conversation_id: i64) -> anyhow::Result<Resumed> {
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
    // disagree. A halt is Verkstead saying *nothing is driving this any more*,
    // written as the run stopped; what may still be registered behind one is a
    // watcher winding down or a poll loop with nothing left to do, and a
    // Conversation the human is told to press Resume on must not refuse the
    // press on account of one.
    let halted = store::halted(&state.pool, conversation_id).await?.is_some();

    // Otherwise it is the register that says it, because that is where the
    // answer is: a Conversation is driven by tasks and sessions, and neither
    // leaves a row behind. The second press of the button lands here — the first
    // took the halt away and started something.
    if !halted
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
    // sweep that found it undriven meanwhile would halt it all over again.
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
                String::new(),
                driving,
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
                String::new(),
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
        // that halted again on its next poll without dispatching anything. See
        // [`crate::checks::retried`].
        Lifecycle::Wrapping => {
            clear(state, conversation_id).await?;

            let state = state.clone();

            tokio::spawn(async move {
                // Held until the four watchers have registrations of their own,
                // which is what [`crate::wrapping::watching`] takes as it spawns
                // them: dropping first would leave a moment where a sweep could
                // find the Conversation undriven all over again.
                let _driving = driving;

                crate::checks::retried(state, conversation_id).await;
            });
        }

        // Refused above, where the refusal belongs: before the halt is read and
        // before a registration is taken. Answered again here because the match
        // has to be whole, and answered the same way.
        Lifecycle::Draft | Lifecycle::Done | Lifecycle::Aborted => {
            return Ok(Resumed::NotDriven);
        }
    }

    tracing::info!(
        conversation_id,
        state = ?conversation.state,
        "the human pressed Resume, so the Conversation is being driven again"
    );

    Ok(Resumed::Resumed)
}

/// Take the halt away, and tell the open pages the badge has gone.
///
/// The Notice stays where it is: it is a record of a stop that really happened.
/// What goes is the state — the badge, and the guard that stops anything being
/// launched behind it.
///
/// Nothing to clear is the ordinary case rather than a mistake: Resume is
/// offered on a Conversation that is merely undriven as much as on one that
/// halted, and a restarted server's are all of the first kind.
async fn clear(state: &AppState, conversation_id: i64) -> anyhow::Result<()> {
    store::clear_halt(&state.pool, conversation_id).await?;

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
/// has gone* would leave them exactly as stuck as an interruption did.
pub(crate) fn ready(
    state: &AppState,
    conversation_id: i64,
    lifecycle: Lifecycle,
    halted: bool,
) -> bool {
    matches!(
        lifecycle,
        Lifecycle::Grilling | Lifecycle::Implementing | Lifecycle::Wrapping
    ) && (halted
        || !state
            .drivers
            .driven(&state.sessions.working(), conversation_id, lifecycle))
}
