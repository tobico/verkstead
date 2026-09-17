//! The Done signal: a session saying its work is finished, by running
//! `verkstead done`.
//!
//! Every session Verkstead launches is an interactive agent, which idles when
//! its work is over rather than exiting, so something has to end it. What used
//! to was a guess read from outside — the work on the branch plus a few seconds
//! of quiet — and an agent that waits on something in the background wears
//! exactly that shape while it is still at work. So the agent says it is done,
//! and that is what ends it. See ADR-0018.
//!
//! **The repository is still read, at that moment and only then.** What *done*
//! means is the kind's own reading, unchanged — see
//! [`crate::runner::Landing`] — and what it is for now is the check on a signal
//! rather than the trigger for an ending. A signal the evidence does not bear
//! out is refused, naming what is missing, and the session stays alive to put
//! it right in the same turn.
//!
//! **The signal is a bare verb**, because the server already knows which
//! session is running in the Conversation and what it was sent for: whatever is
//! seeing that session out writes down here what it is waiting for — see
//! [`Signals::expecting`] — and the route reads it back. A session nothing has
//! written anything down for is refused by name: one still grilling with no
//! Direction picked has no artifact to have finished.
//!
//! The ending is the driver's rather than the route's. An accepted signal is
//! remembered against the session, and the driver seeing it out ends it once it
//! is next idle — so the closing words an agent prints after the command reach
//! the Transcript. See [`crate::runner`]'s `see_out`.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use tokio::sync::watch;
use verkstead_schema::ApiError;

use crate::AppState;
use crate::reply::yaml;
use crate::runner::Landing;
use crate::store::{self, Lifecycle};

/// What each Conversation's running session is being seen out for, where a
/// driver is seeing one out on the Done signal.
///
/// Cloned into [`crate::AppState`] the way the other registers are, and shared
/// by every clone: what a session was sent for is a fact about the process
/// rather than about any one handler.
#[derive(Clone, Default)]
pub(crate) struct Signals {
    expected: Arc<Mutex<HashMap<i64, Expected>>>,

    /// Which registration is which, so that one taken off the register is only
    /// ever the one that put itself there — see [`Expecting`].
    issued: Arc<AtomicU64>,
}

/// One session's entry: which session, what landed looks like for it, and the
/// word that it signalled and was believed.
struct Expected {
    token: u64,
    event_id: i64,
    worktree: PathBuf,
    landing: Landing,
    given: watch::Sender<bool>,
}

/// A driver's hold on its entry, for as long as it is seeing the session out.
///
/// A guard rather than a pair of calls, for the reason [`crate::drivers`]'s
/// registration is one: a driver cancelled mid-watch — a later pick superseding
/// the one that armed it, above all — has to take its entry with it, or a
/// signal would be checked against an artifact nobody is waiting for any more.
pub(crate) struct Expecting {
    signals: Signals,
    conversation_id: i64,
    token: u64,
    given: watch::Receiver<bool>,
}

/// Whether a session has given the Done signal, as whoever is waiting on it
/// holds that.
#[derive(Debug, Clone)]
pub(crate) struct Signal {
    given: watch::Receiver<bool>,
}

impl Signals {
    pub(crate) fn new() -> Signals {
        Signals::default()
    }

    /// Write down that the session printing into `event_id` is to be ended on
    /// its Done signal, once `landing` has landed in `worktree`.
    ///
    /// Replaces whatever the Conversation had written down: one Worktree holds
    /// one session, and the driver seeing it out now is the one that knows what
    /// it was sent for.
    pub(crate) fn expecting(
        &self,
        conversation_id: i64,
        event_id: i64,
        worktree: PathBuf,
        landing: Landing,
    ) -> Expecting {
        let token = self.issued.fetch_add(1, Ordering::Relaxed);
        let (given, receiver) = watch::channel(false);

        self.register().insert(
            conversation_id,
            Expected {
                token,
                event_id,
                worktree,
                landing,
                given,
            },
        );

        Expecting {
            signals: self.clone(),
            conversation_id,
            token,
            given: receiver,
        }
    }

    fn register(&self) -> std::sync::MutexGuard<'_, HashMap<i64, Expected>> {
        self.expected
            .lock()
            .expect("the done signals register is not poisoned")
    }
}

impl Expecting {
    /// The signal this entry is waiting on, to be handed to whatever waits.
    pub(crate) fn signal(&self) -> Signal {
        Signal {
            given: self.given.clone(),
        }
    }
}

impl Drop for Expecting {
    fn drop(&mut self) {
        let mut register = self.signals.register();

        if register
            .get(&self.conversation_id)
            .is_some_and(|expected| expected.token == self.token)
        {
            register.remove(&self.conversation_id);
        }
    }
}

impl Signal {
    /// Whether the signal has been given and accepted yet.
    pub(crate) fn given(&self) -> bool {
        *self.given.borrow()
    }

    /// Wait until it has been. Never returns where it never is, which is what
    /// makes it an arm of a `select!`.
    pub(crate) async fn arrived(mut self) {
        if self.given.wait_for(|given| *given).await.is_err() {
            // The entry went without the signal ever being given: nothing will
            // give it now, and nothing is waiting for this to say otherwise.
            std::future::pending::<()>().await;
        }
    }
}

/// What the route answers with, either way.
enum Verdict {
    Accepted,
    Refused(String),
}

/// `POST /conversations/{conversation}/api/v1/done` — the session running in
/// this Conversation says its work is finished.
///
/// 200 where the work has landed by its kind's own reading, and the session is
/// ended once it is next idle. 409 otherwise, with the reason in words the agent
/// can act on in the same turn: what is missing, no session here to end, or no
/// Direction picked yet. The session is left exactly as it was.
pub(crate) async fn signal(
    State(state): State<AppState>,
    Path(conversation_id): Path<i64>,
) -> Response {
    match verdict(&state, conversation_id).await {
        Verdict::Accepted => {
            tracing::info!(
                conversation_id,
                "a session said it is done and its work bears that out, so it is to be ended"
            );

            (StatusCode::OK, "accepted\n").into_response()
        }
        Verdict::Refused(why) => {
            tracing::info!(
                conversation_id,
                why,
                "a session said it is done and was refused"
            );

            yaml(StatusCode::CONFLICT, &ApiError::new(why))
        }
    }
}

/// Decide a signal: check it against what the session was sent for, and
/// remember it where it holds.
async fn verdict(state: &AppState, conversation_id: i64) -> Verdict {
    let Some(event_id) = state.sessions.writing(conversation_id) else {
        return Verdict::Refused(
            "there is no session running in this Conversation, so there is nothing here to end"
                .to_owned(),
        );
    };

    // Read under the lock and checked outside it: the check is git, and a lock
    // held across a process would hold every other Conversation's signal behind
    // it.
    let found = state
        .signals
        .register()
        .get(&conversation_id)
        .filter(|expected| expected.event_id == event_id)
        .map(|expected| {
            (
                expected.token,
                expected.worktree.clone(),
                expected.landing.clone(),
            )
        });

    let Some((token, worktree, landing)) = found else {
        return Verdict::Refused(unexpected(state, conversation_id).await);
    };

    if let Some(missing) = crate::runner::missing(&worktree, &landing).await {
        return Verdict::Refused(format!(
            "this session is not done yet: {missing}. Put that right, then run `verkstead done` \
             again"
        ));
    }

    // The same entry the check was made for, rather than whichever is there now:
    // one put there in the meantime is a driver waiting on something else.
    match state
        .signals
        .register()
        .get(&conversation_id)
        .filter(|expected| expected.token == token)
    {
        Some(expected) => {
            expected.given.send_replace(true);
            Verdict::Accepted
        }
        None => Verdict::Refused(
            "what this session was sent for changed while the signal was being checked, so run \
             `verkstead done` again"
                .to_owned(),
        ),
    }
}

/// Why a session nothing is waiting on a signal from was refused.
///
/// A grilling that has not been picked on is the one said by name: it has no
/// artifact to have finished, and the move that gives it one is the human's. A
/// Direction picked a moment ago is the watcher not yet armed, and says so.
async fn unexpected(state: &AppState, conversation_id: i64) -> String {
    let grilling = matches!(
        store::state(&state.pool, conversation_id).await,
        Ok(Some(Lifecycle::Grilling))
    );

    if !grilling {
        return "Verkstead ends this session by itself once its work is there, so there is \
                nothing for `verkstead done` to say"
            .to_owned();
    }

    match store::picked_direction(&state.pool, conversation_id).await {
        Ok(Some(_)) => "the Direction the human picked is still being taken up, so run \
                        `verkstead done` again in a moment"
            .to_owned(),
        _ => "no Direction has been picked yet, so there is nothing for this session to have \
              finished — propose one with `verkstead ask` and write what the human picks"
            .to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(signals: &Signals, event_id: i64) -> Expecting {
        signals.expecting(7, event_id, PathBuf::from("/nowhere"), Landing::Ticked(1))
    }

    /// Dropping a hold takes its entry off the register, which is what keeps a
    /// cancelled driver's artifact from being checked against.
    #[test]
    fn a_hold_let_go_of_takes_its_entry_with_it() {
        let signals = Signals::new();

        drop(entry(&signals, 1));

        assert!(signals.register().get(&7).is_none());
    }

    /// And never somebody else's: a driver that replaced it owns the entry now.
    #[test]
    fn a_superseded_hold_leaves_the_entry_that_replaced_it() {
        let signals = Signals::new();

        let first = entry(&signals, 1);
        let _second = entry(&signals, 2);

        drop(first);

        assert_eq!(signals.register().get(&7).map(|e| e.event_id), Some(2));
    }

    /// A signal given reaches whoever holds a copy of it.
    #[tokio::test]
    async fn a_given_signal_arrives() {
        let signals = Signals::new();
        let hold = entry(&signals, 1);
        let signal = hold.signal();

        assert!(!signal.given());

        signals.register().get(&7).unwrap().given.send_replace(true);

        assert!(signal.given());
        tokio::time::timeout(std::time::Duration::from_secs(1), signal.arrived())
            .await
            .expect("the signal arrives once given");
    }
}
