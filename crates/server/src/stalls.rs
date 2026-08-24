//! The check that finds a Conversation nothing is driving, and writes down that
//! it has stopped.
//!
//! **Stalled** is three things at once: the state is Grilling, Implementing or
//! Wrapping; nothing is registered as driving it — see [`crate::drivers`]; and
//! it is not halted already. Each of the three is doing work. The state is
//! what says something ought to be happening, so Draft and Direction waiting on
//! the human, Done finished and Aborted stopped are none of them a Conversation
//! standing still. The register is what says nothing is, rather than a stopwatch
//! — a wrapping Conversation idles for days under live watchers and is perfectly
//! healthy, and so are the gaps between an unattended run's steps. And a halt is
//! already the record of a Conversation that stopped, so one that has one is one
//! that has been written down.
//!
//! What it records is a **halt** — see [`crate::halts`] — of the kind nobody
//! chose: a stall is a driver that went away rather than a decision anybody
//! took, so a restarting server is free to start the work again unasked. The
//! Notice beside it reads as a report of a Conversation standing still rather
//! than of a session that failed, because nothing failed and nothing exited:
//! there was no session there at all.
//!
//! **When it looks.** At startup, every [`crate::Pace::stalls`] while the server
//! runs, and the moment a Manual Task's session ends. Startup is the one that
//! matters least, and deliberately: no driver survives the process, so a server
//! coming back holds no registrations at all — and what puts that right is the
//! restart's own resume, which runs first and takes up everything it can. See
//! [`crate::resume::at_startup`] and [`sweeping`], which waits for it. What is
//! left over when the sweep looks is what genuinely has nobody.

use std::time::Duration;

use tokio::task::JoinHandle;

use crate::AppState;
use crate::drivers::Driving;
use crate::store::{self, Lifecycle};

/// How often every Conversation is looked over, as [`crate::Pace`] has it by
/// default.
///
/// A minute. A stall is a Conversation standing still rather than something on
/// fire, so noticing one a minute late costs nothing — and the sweep is a list
/// of Conversations and a register read, which is not a thing to do in a tight
/// loop for the years a server is up.
pub(crate) const SWEPT_EVERY: Duration = Duration::from_secs(60);

/// Sweep for stalled Conversations from now until the process stops: once as
/// soon as `resumed` is done, and every [`crate::Pace::stalls`] after that.
///
/// `resumed` is whatever the startup takes up again before anything is judged —
/// the restart's own resume over every Conversation it was left driving, which
/// registers as it goes. Waited for rather than raced, because the two answer the
/// same question from opposite ends: a Conversation left mid-run has nothing
/// driving it for exactly as long as it takes to take it up again, and a sweep
/// that got there first would call every healthy one of them stalled.
pub(crate) fn sweeping(state: &AppState, resumed: Vec<JoinHandle<()>>) {
    let state = state.clone();

    tokio::spawn(async move {
        for taking_up in resumed {
            if let Err(error) = taking_up.await {
                tracing::error!(error = ?error, "taking up what was left running failed, so the sweep judges what it finds");
            }
        }

        loop {
            sweep(&state).await;

            tokio::time::sleep(state.sessions.pace().stalls).await;
        }
    });
}

/// One look over every Conversation: halt each one that has stalled.
///
/// Called on its own the moment a Manual Task's session ends, which is the one
/// time a stall is worth noticing without waiting for the next sweep: the human
/// set that session going by hand because nothing was moving, and what they want
/// to know when it stops is whether anything is moving now.
///
/// Nothing is refused for and nothing is returned. This runs unattended with
/// nobody watching, and what it has to say it says on the Timeline or in the
/// log.
pub(crate) async fn sweep(state: &AppState) {
    let conversations = match store::conversations(&state.pool).await {
        Ok(conversations) => conversations,
        Err(error) => {
            tracing::error!(error = ?error, "listing the Conversations to look for a stall among failed");
            return;
        }
    };

    // Read once for the whole sweep, which is what it is shaped for: a sweep is
    // a list of Conversations, and asking the register per Conversation would be
    // a list of lock acquisitions — see [`crate::sessions::Sessions::working`].
    let working = state.sessions.working();

    for conversation in conversations {
        if state
            .drivers
            .driven(&working, conversation.id, conversation.state)
        {
            continue;
        }

        // Asked before the evidence is gathered rather than left to
        // [`store::halt`] to refuse, which would answer the same way: gathering
        // it is a `git status` on a Worktree and a Timeline read, and a
        // Conversation already halted is not one to spend either on. It is also
        // not stalled — being written down is the half of a stall that is
        // missing.
        match store::halted(&state.pool, conversation.id).await {
            Ok(Some(_)) => continue,
            Ok(None) => {}
            Err(error) => {
                tracing::error!(error = ?error, conversation_id = conversation.id, "asking whether a Conversation had already stopped failed");
                continue;
            }
        }

        stalled(state, conversation.id, conversation.state).await;
    }
}

/// Halt a stalled Conversation, and say so on its own Timeline.
///
/// The evidence is the ordinary evidence with nothing invented to fill it: the
/// state it is in, that nothing was driving it, what git makes of the Worktree,
/// and the tail of whatever the last session to run said — which is usually the
/// reason there is no session running now.
///
/// [`store::Halt::Circumstance`], because that is what a stall is: nobody
/// decided to stop, a driver went away. What that buys is a restart free to
/// start the work again without asking — a deliberate halt is the one that
/// waits for a press.
async fn stalled(state: &AppState, conversation_id: i64, lifecycle: Lifecycle) {
    tracing::warn!(
        conversation_id,
        state = ?lifecycle,
        "nothing is driving a Conversation that says it is being worked on",
    );

    let halted = crate::halts::halt(
        state,
        conversation_id,
        store::Halt::Circumstance,
        driving(lifecycle),
        "nothing is driving it: no session is running, and nothing is left to start one",
        said_last(state, conversation_id).await,
    )
    .await;

    if let Err(error) = halted {
        tracing::error!(
            error = ?error,
            conversation_id,
            "a Conversation has stalled and the halt saying so could not be recorded"
        );
    }
}

/// Start driving the Conversation again, because the human pressed Retry on the
/// stall that said nothing was.
///
/// For the stalls already on Timelines, and only those: the sweep halts now
/// rather than raising anything, so what reaches this is a Conversation that was
/// stopped by a Verkstead of before — which still has a card, a sheet and a
/// press behind it until they are taken away.
///
/// The one Retry that is not a step run again. Every other Interruption is
/// raised by a session launched for something, so the record names what to
/// launch; this one is raised about a Conversation with no session at all, and
/// what ought to be driving it is the state's to say. So the state is read —
/// now rather than when the stall was raised, because the human answers when
/// they get to it and a Conversation moves on in the meantime.
///
/// `note` is what they wrote alongside, and it reaches the session the relaunch
/// starts, exactly where a retried step's note goes. A wrap-up starts no session
/// of its own — its watchers dispatch whatever the pull request turns out to
/// need — so there the note has nowhere to go and is left on the record beside
/// the Remedy.
///
/// `driving` is the registration [`crate::runner::retry`] took as the press
/// arrived, handed on rather than taken again at the far end: the whole of what
/// a relaunch fixes is a Conversation nothing is registered against, and a gap
/// in the middle of fixing it would be a second stall raised about the very
/// thing that was putting the first one right.
pub(crate) async fn retried(state: AppState, conversation_id: i64, note: String, driving: Driving) {
    let lifecycle = match store::load_conversation(&state.pool, conversation_id).await {
        Ok(Some(conversation)) => conversation.state,
        Ok(None) => {
            tracing::error!(
                conversation_id,
                "there is no Conversation left to start driving again"
            );
            return;
        }
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading the Conversation to start driving again failed");
            return;
        }
    };

    match lifecycle {
        // Whichever run it was that stopped, taken up from where the repository
        // now stands — see [`crate::runner::implementing_again`].
        Lifecycle::Implementing => {
            crate::runner::implementing_again(state, conversation_id, note, driving).await;
        }
        // The watcher set started over the top of nothing, which is what a
        // restarting server does with a Conversation it left wrapping up. Each
        // of the four decides for itself whether there is anything left to do,
        // so respawning them asks the pull request the same questions again
        // rather than doing anything twice — see [`crate::wrapping::watching`].
        Lifecycle::Wrapping => {
            tracing::info!(
                conversation_id,
                "a stalled wrap-up was retried, so what watches one is started again"
            );

            // Registered before this one is let go, the four of them taking
            // registrations of their own as they are spawned: the handover is
            // the point, and dropping first would leave a moment where a sweep
            // could find the Conversation undriven all over again.
            crate::wrapping::watching(&state, conversation_id);
            drop(driving);
        }
        // A fresh grilling on the Brief, which is the only thing a retried
        // grilling can be: the dead session's interview went with the process it
        // ran in. What it is given beside the Brief is the log of what has
        // already been settled, so that it does not open by asking again — see
        // [`crate::grillings::again`].
        Lifecycle::Grilling => {
            crate::grillings::again(state, conversation_id, note, driving).await;
        }
        // None of these is a state anything was supposed to be driving, so none
        // of them is one a stall was raised about — this is a Conversation that
        // moved between the stall and the human getting to it, which the move
        // itself has already answered.
        Lifecycle::Draft | Lifecycle::Done | Lifecycle::Aborted => {
            tracing::info!(
                conversation_id,
                state = ?lifecycle,
                "the Conversation has moved on since it stalled, so nothing was started again",
            );
        }
    }
}

/// What ought to have been happening, in the words the rest of the evidence uses
/// — the sentence the Timeline draws above how it ended.
///
/// Read off the state, because for a stall the state is the whole of what says
/// it. Every other halt names a step a session was launched for; this one names
/// the thing nobody was doing.
///
/// The three states nothing drives never reach here — [`sweep`] leaves them
/// alone, and [`crate::drivers::Drivers::driven`] is where that is decided — so
/// what they answer is only ever read by a database somebody has been in by
/// hand.
///
/// Shared with [`crate::resume::refused`], which is the same sentence about the
/// same Conversation: what nobody was doing, said as a startup resume gives up on
/// starting it.
pub(crate) fn driving(lifecycle: Lifecycle) -> &'static str {
    match lifecycle {
        Lifecycle::Grilling => "grilling the work",
        Lifecycle::Implementing => "implementing the work",
        Lifecycle::Wrapping => "wrapping the work up",
        Lifecycle::Draft | Lifecycle::Done | Lifecycle::Aborted => "driving the Conversation",
    }
}

/// Which Timeline Event the last session to run printed into, so the evidence
/// carries the tail of what it said — or `None` where no session has ever run.
///
/// Off the Timeline rather than off the sessions register, which is the one
/// place it could not be: what makes this a stall is that the register holds
/// nothing. The newest agent output is the nearest thing there is to a last
/// word, and on a Conversation whose session died it is where the reason is
/// written down.
///
/// A Timeline read per stalled Conversation, which is a read per Conversation
/// nothing is driving rather than per Conversation — the sweep asks this only
/// once it has one to raise about. [`crate::resume::refused`] asks it on the same
/// terms, having likewise found one to halt about.
pub(crate) async fn said_last(state: &AppState, conversation_id: i64) -> Option<i64> {
    let timeline = match store::timeline(&state.pool, conversation_id).await {
        Ok(timeline) => timeline,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading back what a stalled Conversation last had running failed");
            return None;
        }
    };

    timeline
        .iter()
        .rev()
        .find_map(|event| matches!(event.event, store::Event::AgentOutput(_)).then_some(event.id))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every state a stall can be found in says what was not being done, in its
    /// own words. The evidence is read on a phone, and "nothing is driving it"
    /// is only half an answer without what *it* was meant to be doing.
    #[test]
    fn each_driven_state_says_what_nobody_was_doing() {
        let said: Vec<&str> = [
            Lifecycle::Grilling,
            Lifecycle::Implementing,
            Lifecycle::Wrapping,
        ]
        .into_iter()
        .map(driving)
        .collect();

        assert_eq!(
            said,
            vec![
                "grilling the work",
                "implementing the work",
                "wrapping the work up",
            ],
        );
    }
}
