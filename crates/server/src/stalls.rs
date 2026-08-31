//! The check that finds a Conversation nothing is driving, and writes down that
//! it has stopped.
//!
//! **Stalled** is three things at once: the state is one work goes on in —
//! Grilling, Implementing, Wrapping or Follow-up; nothing is registered as
//! driving it — see [`crate::drivers`]; and nothing has stopped it on purpose.
//! Each of the four is doing work. The state is
//! what says something ought to be happening, so Draft and Direction waiting on
//! the human, Done finished and Closed stopped are none of them a Conversation
//! standing still. The register is what says nothing is, rather than a stopwatch
//! — a wrapping Conversation idles for days under live watchers and is perfectly
//! healthy, and so are the gaps between an unattended run's steps. And a stop is
//! already the record of a Conversation that stopped, so one that has one is one
//! that has been written down — an account out of window included, that being a
//! run stopped on purpose and said on its own Timeline. It is all the one
//! question [`crate::stopping::stopped`] answers.
//!
//! What it records is a **stop** — see [`crate::stopping`] — of the kind nobody
//! chose: a stall is a driver that went away rather than a decision anybody
//! took, so a restarting server is free to start the work again unasked. The
//! Notice beside it reads as a report of a Conversation standing still rather
//! than of a session that failed, because nothing failed and nothing exited:
//! there was no session there at all.
//!
//! **When it looks.** At startup, and every [`crate::Pace::stalls`] while the
//! server runs. Startup is the one that matters least, and deliberately: no
//! driver survives the process, so a server coming back holds no registrations
//! at all — and what puts that right is the restart's own resume, which runs
//! first and takes up everything it can. See [`crate::resume::at_startup`] and
//! [`sweeping`], which waits for it. What is left over when the sweep looks is
//! what genuinely has nobody.
//!
//! And never, on a server that runs no sessions: there, nobody is what every
//! Conversation mid-run has and always will have, so the sweep has nothing to
//! tell the human. See [`sweeping`].

use std::time::Duration;

use tokio::task::JoinHandle;

use crate::AppState;
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
    // Nothing to sweep for on a server that runs no sessions — see
    // [`crate::sessions::Sessions::runs_sessions`]. A stall is a Conversation
    // *nothing is driving*, and a server with no agents drives nothing by
    // construction: every Conversation mid-run is one, forever. So the sweep
    // would be a minute-by-minute stop on each of them saying so.
    //
    // Only the tests' routers are ever built that way, and what it costs them
    // is what it would cost a server: `router()` over a store held still is
    // exactly how the viewer's fixtures are written, and a sweep landing
    // mid-write puts a stall on the Timeline being serialised.
    if !state.sessions.runs_sessions() {
        return;
    }

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

/// One look over every Conversation: stop each one that has stalled.
///
/// Nothing is refused for and nothing is returned. This runs unattended with
/// nobody watching, and what it has to say it says on the Timeline or in the
/// log.
async fn sweep(state: &AppState) {
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
        // As the resume sweep does: a state word nobody can read is not one to
        // weigh a stall against. The sidebar carries the row so it can be
        // ended, and nothing unattended acts on it.
        let Some(lifecycle) = conversation.state.known() else {
            continue;
        };

        if state.drivers.driven(&working, conversation.id, lifecycle) {
            continue;
        }

        // Asked before the evidence is gathered rather than left to
        // [`store::stop`] to refuse, which would answer the same way: gathering
        // it is a `git status` on a Worktree and a Timeline read, and a
        // Conversation already stopped is not one to spend either on. It is also
        // not stalled — being written down is the half of a stall that is
        // missing.
        //
        // One question, an account out of window included: a run waiting a window
        // out is stopped on purpose, said on its own Timeline and already pushed,
        // and stopping it again would be telling the human twice about one wait
        // and calling a deliberate stop a failure.
        match store::stopped(&state.pool, conversation.id).await {
            Ok(Some(_)) => continue,
            Ok(None) => {}
            Err(error) => {
                tracing::error!(error = ?error, conversation_id = conversation.id, "asking whether a Conversation had already stopped failed");
                continue;
            }
        }

        // A Stop the human pressed while something was running, whose run then
        // came to rest without launching anything else — a backlog that finished
        // its last task, a session that was the whole of the work. There was no
        // next launch for the request to land at, so it lands here, and what is
        // written is their stop rather than a stall: nobody is owed a Notice
        // saying nothing was driving a Conversation they stopped themselves. See
        // [`crate::stops::asked`].
        if crate::stops::asked(state, conversation.id).await {
            continue;
        }

        stalled(state, conversation.id, lifecycle).await;
    }
}

/// Stop a stalled Conversation, and say so on its own Timeline.
///
/// The evidence is the ordinary evidence with nothing invented to fill it: the
/// state it is in, that nothing was driving it, what git makes of the Worktree,
/// and the tail of whatever the last session to run said — which is usually the
/// reason there is no session running now.
///
/// [`store::Decision::Circumstance`], because that is what a stall is: nobody
/// decided to stop, a driver went away. What that buys is a restart free to
/// start the work again without asking — a stop somebody decided is the one
/// that waits for a press.
async fn stalled(state: &AppState, conversation_id: i64, lifecycle: Lifecycle) {
    tracing::warn!(
        conversation_id,
        state = ?lifecycle,
        "nothing is driving a Conversation that says it is being worked on",
    );

    let stopped = crate::stopping::stop(
        &state.pool,
        &state.nudges,
        conversation_id,
        crate::stopping::Decided::Nobody,
        driving(lifecycle),
        "nothing is driving it: no session is running, and nothing is left to start one",
        said_last(state, conversation_id).await,
    )
    .await;

    if let Err(error) = stopped {
        tracing::error!(
            error = ?error,
            conversation_id,
            "a Conversation has stalled and the stop saying so could not be recorded"
        );
    }
}

/// What ought to have been happening, in the words the rest of the evidence uses
/// — the sentence the Timeline draws above how it ended.
///
/// Read off the state, because for a stall the state is the whole of what says
/// it. Every other stop names a step a session was launched for; this one names
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
        Lifecycle::FollowUp => "following the work up",
        Lifecycle::Draft | Lifecycle::Done | Lifecycle::Closed => "driving the Conversation",
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
/// terms, having likewise found one to stop about.
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
        .find_map(|event| matches!(event.event, store::Event::AgentOutput(..)).then_some(event.id))
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
            Lifecycle::FollowUp,
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
                "following the work up",
            ],
        );
    }
}
