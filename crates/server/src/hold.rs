//! The Hold: the human at a live session's keyboard.
//!
//! It begins with the first keystroke typed into a Screen and ends only by being
//! handed back. Not by a timeout, not by the socket dropping, not by the tab
//! closing — resuming over a half-finished intervention is worse than a stalled
//! run, so the only thing that ends one is somebody saying so.
//!
//! **While it lasts Verkstead records and nothing else.** The relay goes on
//! reading the terminal, the Capture goes on being written, the Transcript goes
//! on being followed and the Timeline goes on being nudged. What stops is ending
//! the session and advancing the run — and that is a gate each driver asks
//! rather than one flag in one place, because each of them ends or advances
//! something of its own: a backlog step ended on landed-plus-quiet, a fix
//! session ended on committed-plus-quiet, a review session ended once it has
//! asked, and the wrap-up that starts the next roadmap stage. Every one of them
//! waits on [`Holds::until_handed_back`] before it acts.
//!
//! **A Hold outlives the session it was taken on.** A session that exits while
//! held advances nothing until the hand-back, and the hand-back then runs the
//! ordinary end-of-session evaluation on whatever the human left. So this is a
//! register of its own rather than a flag on a running session, keyed by the
//! Conversation and remembering which of its sessions the keyboard was taken at.
//!
//! **Nothing here is written down.** A Hold leaves no Timeline Event — the
//! Timeline records the work rather than the watching — and it lives exactly as
//! long as the process does, which is as long as the sessions it is about: a
//! restarted server has no sessions at all, so it has nothing left to be held.

use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::sync::{Arc, Mutex};

use tokio::sync::broadcast;

/// How many hand-backs a driver waiting on one may fall behind before it looks
/// at the register instead of listening.
///
/// Falling behind is not a failure here: what a waiter does with the word is
/// look again, so a missed one costs nothing as long as the looking still
/// happens — see [`Holds::until_handed_back`]. Generous all the same, this being
/// one message per Hold ever handed back by one human.
const HANDING_BACK: usize = 64;

/// The Holds in force on this server, by the Conversation each is on.
///
/// One per Conversation at most, because one Conversation has one session and a
/// Hold is the keyboard of one.
#[derive(Clone)]
pub(crate) struct Holds {
    /// Which Conversations are held, and the Timeline Event of the session the
    /// keyboard was taken at — which is where the *blocked on you* badge points,
    /// a badge with nowhere to go being one the human cannot answer.
    held: Arc<Mutex<HashMap<i64, i64>>>,

    /// Word to whoever is waiting that a Conversation has been handed back.
    ///
    /// A broadcast rather than a notification per waiter: the drivers waiting on
    /// one are few and each of them re-reads the register on hearing anything at
    /// all, so what this carries is *look again* rather than an instruction.
    handed_back: broadcast::Sender<i64>,
}

impl Holds {
    /// A server with nothing held, which is every server as it starts.
    pub(crate) fn none() -> Holds {
        Holds {
            held: Arc::new(Mutex::new(HashMap::new())),
            handed_back: broadcast::Sender::new(HANDING_BACK),
        }
    }

    /// The human typed into the Screen of `event_id`'s session: take the Hold.
    ///
    /// `true` where this is the keystroke that took it, which is what the caller
    /// tells the open pages about — every keystroke after it lands on a Hold
    /// that is already in force and changes nothing.
    ///
    /// A Conversation already held stays held on the session it was taken at.
    /// Nothing can reach a second one: the run does not advance past a Hold, so
    /// there is no next session for a keystroke to arrive at.
    pub(crate) fn take(&self, conversation_id: i64, event_id: i64) -> bool {
        match self.register().entry(conversation_id) {
            Entry::Occupied(_) => false,
            Entry::Vacant(vacant) => {
                vacant.insert(event_id);
                true
            }
        }
    }

    /// Which session's keyboard the human has, or `None` where this Conversation
    /// is Verkstead's again.
    pub(crate) fn holding(&self, conversation_id: i64) -> Option<i64> {
        self.register().get(&conversation_id).copied()
    }

    /// The human has handed the keyboard back.
    ///
    /// `true` where there was a Hold to end. `false` is a hand-back that arrived
    /// twice — two devices, or a press repeated — which is the same answer
    /// arriving again rather than a failure.
    pub(crate) fn hand_back(&self, conversation_id: i64) -> bool {
        let held = self.register().remove(&conversation_id).is_some();

        if held {
            // A send that fails is nobody waiting, which is the ordinary case:
            // the driver that would have been listening has usually already been
            // and gone.
            let _ = self.handed_back.send(conversation_id);
        }

        held
    }

    /// Wait until nothing is holding this Conversation's keyboard.
    ///
    /// The gate every driver asks before it ends a session or advances a run.
    /// It returns at once where there is no Hold, which is every run nobody has
    /// touched — the ordinary case, and one read of a map.
    ///
    /// Subscribed before the register is asked, so that a hand-back landing
    /// between the two is heard rather than missed: the other way round is a
    /// driver that waits for ever on a keyboard that has already been given
    /// back.
    pub(crate) async fn until_handed_back(&self, conversation_id: i64) {
        loop {
            let mut handed_back = self.handed_back.subscribe();

            let Some(event_id) = self.holding(conversation_id) else {
                return;
            };

            tracing::info!(
                conversation_id,
                event_id,
                "the human has this Conversation's keyboard, so nothing is ended or advanced \
                 until they hand it back",
            );

            match handed_back.recv().await {
                // Somebody was handed back to. Which Conversation is not read
                // off the message: the register above is what decides, and this
                // only says it is worth asking again.
                Ok(_) => {}
                // Too far behind to be told one by one, which is the same
                // instruction: look again.
                Err(broadcast::error::RecvError::Lagged(_)) => {}
                // Impossible while this holds a sender of its own, and a
                // Conversation nothing can ever be handed back to is not one to
                // wait on for ever.
                Err(broadcast::error::RecvError::Closed) => return,
            }
        }
    }

    /// The register, locked.
    fn register(&self) -> std::sync::MutexGuard<'_, HashMap<i64, i64>> {
        self.held
            .lock()
            .expect("the Holds register is not poisoned")
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    /// The first keystroke takes it and the ones after it find it taken.
    #[test]
    fn the_first_keystroke_is_the_one_that_takes_the_hold() {
        let holds = Holds::none();

        assert!(holds.take(7, 41), "the first keystroke takes the Hold");
        assert!(!holds.take(7, 41), "and the second lands on one in force");
        assert_eq!(holds.holding(7), Some(41));
        assert_eq!(holds.holding(8), None, "one Conversation, not the next");
    }

    /// And handing back ends it, once.
    #[test]
    fn handing_back_ends_it_and_saying_so_twice_is_the_same_answer() {
        let holds = Holds::none();
        holds.take(7, 41);

        assert!(holds.hand_back(7), "the keyboard goes back");
        assert_eq!(holds.holding(7), None);
        assert!(
            !holds.hand_back(7),
            "and a second press is the same answer arriving again",
        );
    }

    /// A driver asking the gate on a Conversation nobody is holding walks
    /// straight through it, which is every run nobody has touched.
    #[tokio::test]
    async fn a_conversation_nobody_is_holding_is_not_a_wait_at_all() {
        let holds = Holds::none();

        tokio::time::timeout(Duration::from_secs(5), holds.until_handed_back(7))
            .await
            .expect("an unheld Conversation to be no wait at all");
    }

    /// And one that is held waits — until the hand-back, and no sooner.
    #[tokio::test]
    async fn the_gate_opens_when_the_keyboard_goes_back() {
        let holds = Holds::none();
        holds.take(7, 41);

        let waiting = tokio::spawn({
            let holds = holds.clone();
            async move { holds.until_handed_back(7).await }
        });

        // Long enough to be sure it is waiting rather than about to be told.
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(
            !waiting.is_finished(),
            "the gate opened with a Hold in force"
        );

        holds.hand_back(7);

        tokio::time::timeout(Duration::from_secs(5), waiting)
            .await
            .expect("the gate to open once the keyboard went back")
            .expect("the waiting task not to panic");
    }
}
