//! What is driving each Conversation, so that something can say when nothing
//! is.
//!
//! A Conversation in a driven state — Grilling, Implementing, Wrapping,
//! Follow-up — is supposed to have a task of Verkstead's own seeing it along:
//! the runner working a backlog, the driver following an inline run or a
//! roadmap, the set of watchers a wrap-up has going, the one seeing a follow-up
//! session out. When one of those dies the Conversation is
//! left saying it is being worked on with nothing working on it, and there is
//! nothing on the page for the human to press. This is the half of detecting
//! that which knows what is alive.
//!
//! **Registration rather than a time window.** A driver puts itself on here for
//! as long as its task runs and is off it the moment the task stops, so the
//! question is answered by what exists rather than by how long it has been
//! since anything happened. That matters most for a wrap-up: a Wrapping
//! Conversation with no session running for days is its normal healthy
//! condition — the watchers are polling a pull request nobody has merged — so a
//! window would either flag every one of them or never notice a watcher that
//! had died.
//!
//! It also gives the restart behaviour for nothing: none of these tasks survive
//! the process, so a server that has just come back holds no registrations at
//! all, which is exactly the truth about it.
//!
//! **A grilling is driven by its session until it is picked on.** Starting one
//! launches the session and keeps nothing that follows it, so until the human
//! picks there is nothing to hold a registration — see
//! [`crate::conversations::start_grilling`]. That is why [`Drivers::driven`] is
//! asked with the sessions register in hand rather than answering out of this
//! one alone.
//!
//! The launch itself is the exception, and for the reason every other launch
//! here holds one: the Conversation says it is grilling from before the session
//! exists, and the gap in between is exactly the slow part. So the press holds a
//! registration across it and lets go once there is a session — or once there is
//! not, which is a stall the next sweep should find.
//!
//! A pick changes that: what it arms is a watcher of Verkstead's own, which
//! follows the same session to the artifact it asked for and goes on holding the
//! Conversation through the move that follows — see
//! [`crate::runner::follow_the_tail`]. And a retried tail is the same thing by
//! the other door: the Conversation is still Grilling while the fresh session is
//! launched, and the retry has registered before it starts waiting. So a
//! grilling counts as driven by either.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use crate::store::Lifecycle;

/// Which Conversations have something driving them, and how many things.
///
/// Cloned into [`crate::AppState`] the way the sessions register is, and shared
/// by every clone: what is running is a fact about the process rather than
/// about any one handler.
///
/// A count rather than a flag, because a wrap-up is four tasks at once — six and
/// up where the work ended on a pull request in a companion as well, each of
/// those having its checks watched and its comments read by tasks of its own —
/// and any one of them still going is a wrap-up still being driven. And because
/// a Resume starts a second set over the top of the first without stopping it.
/// Counting is what keeps the first of them to finish from taking the whole
/// Conversation off the register.
#[derive(Clone, Default)]
pub(crate) struct Drivers {
    driving: Arc<Mutex<HashMap<i64, usize>>>,
}

/// One driver's registration, held for as long as its task runs.
///
/// A guard rather than a pair of calls, and that is the whole of why it is a
/// type: deregistration has to survive the task dying as well as returning. A
/// driver that panicked halfway and left its registration behind would be a
/// stalled Conversation nothing could ever detect — which is precisely the
/// failure this register exists to catch.
///
/// Passed along rather than taken again where one driver hands over to another
/// — a chosen direction to the run that follows it, a retry to the loop it
/// starts — so that the registration is continuous across the handover. Taking
/// a fresh one at the far end would leave a gap exactly where the launch that
/// takes the longest is.
pub(crate) struct Driving {
    drivers: Drivers,
    conversation_id: i64,
}

impl Drivers {
    /// A register with nothing on it, which is what a server starts with.
    pub(crate) fn new() -> Drivers {
        Drivers::default()
    }

    /// Say that something is driving `conversation_id`, until the guard is
    /// dropped.
    #[must_use = "a driver is registered for as long as its registration is held, \
                  so dropping one on the spot registers nothing"]
    pub(crate) fn driving(&self, conversation_id: i64) -> Driving {
        *self
            .driving
            .lock()
            .expect("the drivers register is not poisoned")
            .entry(conversation_id)
            .or_insert(0) += 1;

        Driving {
            drivers: self.clone(),
            conversation_id,
        }
    }

    /// Whether anything is driving `conversation_id`, given the state it is in
    /// and `working`, the Conversations a session is registered for.
    ///
    /// The sessions register is handed in whole rather than asked per
    /// Conversation, because the one thing that asks this is a sweep over every
    /// Conversation there is — see [`crate::sessions::Sessions::working`] for
    /// why that is the cheaper way round.
    ///
    /// A state nothing is meant to be driving answers that it is fine as it
    /// stands. Draft is waiting on the human, Done is finished and Closed is
    /// stopped: none of them is a Conversation standing still, so none of them
    /// is one this question is really about.
    pub(crate) fn driven(
        &self,
        working: &HashSet<i64>,
        conversation_id: i64,
        state: Lifecycle,
    ) -> bool {
        match state {
            // A grilling session, or the watcher a pick armed on one — see this
            // module's own documentation for why it is both.
            Lifecycle::Grilling => {
                working.contains(&conversation_id) || self.registered(conversation_id)
            }
            Lifecycle::Implementing | Lifecycle::Wrapping | Lifecycle::FollowUp => {
                self.registered(conversation_id)
            }
            Lifecycle::Draft | Lifecycle::Done | Lifecycle::Closed => true,
        }
    }

    /// Whether any driver is registered for `conversation_id`.
    ///
    /// The raw reading of the register, which is what the Conversation page
    /// reports as `driven` — see [`crate::ui`]. [`Drivers::driven`] is this
    /// question put through the rule about which states are supposed to have
    /// one; this is the register itself.
    pub(crate) fn registered(&self, conversation_id: i64) -> bool {
        self.driving
            .lock()
            .expect("the drivers register is not poisoned")
            .contains_key(&conversation_id)
    }
}

impl Drop for Driving {
    fn drop(&mut self) {
        let mut driving = self
            .drivers
            .driving
            .lock()
            .expect("the drivers register is not poisoned");

        let Some(count) = driving.get_mut(&self.conversation_id) else {
            // Nothing to take off, which cannot happen while a guard exists: it
            // put itself on and nothing else can take another driver's entry
            // away. Said rather than panicked, because a drop that panics while
            // a task is already unwinding aborts the process.
            tracing::error!(
                conversation_id = self.conversation_id,
                "a driver went off a register it was never on"
            );
            return;
        };

        *count -= 1;

        // The last one out takes the Conversation with it, so *driven* is a key
        // being there rather than a count being read.
        if *count == 0 {
            driving.remove(&self.conversation_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The Conversation these are asked about. Any number does; what is being
    /// tested is a register, and a register knows nothing else about one.
    const CONVERSATION: i64 = 7;

    /// Which Conversations have a session running, as [`Drivers::driven`] is
    /// handed it — see [`crate::sessions::Sessions::working`].
    fn working(ids: &[i64]) -> HashSet<i64> {
        ids.iter().copied().collect()
    }

    /// The whole of what a driver's registration is for: while its task runs the
    /// Conversation is being driven, and the moment the task is over it is not.
    #[tokio::test]
    async fn a_driver_is_on_the_register_for_as_long_as_its_task_runs() {
        let drivers = Drivers::new();
        let (started, running) = tokio::sync::oneshot::channel();
        let (finish, finished) = tokio::sync::oneshot::channel();

        let driver = tokio::spawn({
            let driving = drivers.driving(CONVERSATION);

            async move {
                let _driving = driving;

                started.send(()).unwrap();
                finished.await.unwrap();
            }
        });

        running.await.unwrap();
        assert!(
            drivers.registered(CONVERSATION),
            "its task is running, so something is driving the Conversation",
        );

        finish.send(()).unwrap();
        driver.await.unwrap();

        assert!(
            !drivers.registered(CONVERSATION),
            "its task returned, so nothing is driving the Conversation any more",
        );
    }

    /// And the reason it is a guard rather than a pair of calls. A driver that
    /// died halfway and left its registration behind would be a Conversation
    /// nothing was driving that nothing could ever notice — which is the exact
    /// failure the register is here to catch.
    #[tokio::test]
    async fn a_driver_that_panicked_is_off_the_register_too() {
        let drivers = Drivers::new();

        let driver = tokio::spawn({
            let driving = drivers.driving(CONVERSATION);

            async move {
                let _driving = driving;

                panic!("a driver falling over mid-run");
            }
        });

        assert!(driver.await.is_err(), "the task panicked");
        assert!(
            !drivers.registered(CONVERSATION),
            "a driver that died is not one still driving anything",
        );
    }

    /// A wrap-up is five tasks at once and each of them ends in its own time, so
    /// a Conversation is driven while any one of them is left. Which is also
    /// what Resume needs: it starts a second set over the top of
    /// the first without stopping it.
    #[test]
    fn a_conversation_is_driven_while_any_one_of_its_drivers_is_left() {
        let drivers = Drivers::new();
        let mut watchers: Vec<Driving> = (0..4).map(|_| drivers.driving(CONVERSATION)).collect();

        while watchers.len() > 1 {
            watchers.pop();

            assert!(
                drivers.registered(CONVERSATION),
                "{} watchers are still going",
                watchers.len(),
            );
        }

        watchers.pop();

        assert!(
            !drivers.registered(CONVERSATION),
            "the last one out takes the Conversation with it",
        );
    }

    /// Drivers are registered per Conversation, so one Conversation's dead
    /// runner says nothing about another's live one.
    #[test]
    fn one_conversations_driver_drives_only_that_conversation() {
        let drivers = Drivers::new();
        let driving = drivers.driving(CONVERSATION);

        assert!(drivers.registered(CONVERSATION));
        assert!(!drivers.registered(CONVERSATION + 1));

        drop(driving);
    }

    /// The registration is handed from the task that took it to the task that
    /// goes on driving — a chosen direction to the run that follows it, a retry
    /// to the loop it starts — and the point of handing it over rather than
    /// taking a fresh one at the far end is that there is no moment in between
    /// where the Conversation reads as undriven.
    #[tokio::test]
    async fn a_registration_handed_on_is_never_off_the_register_in_between() {
        let drivers = Drivers::new();
        let driving = drivers.driving(CONVERSATION);

        // What the chooser does: the launch it waits out is the slowest part of
        // starting a run, and the Conversation says it is implementing
        // throughout.
        tokio::task::yield_now().await;
        assert!(
            drivers.registered(CONVERSATION),
            "the launch is still going"
        );

        let (over, ended) = tokio::sync::oneshot::channel();

        let run = tokio::spawn(async move {
            let _driving = driving;

            // A run is a session per step with a gap between each: the sessions
            // come and go and the registration does not.
            for _ in 0..3 {
                tokio::task::yield_now().await;
            }

            over.send(()).unwrap();
        });

        ended.await.unwrap();
        run.await.unwrap();

        assert!(
            !drivers.registered(CONVERSATION),
            "the loop has ended, so nothing is driving it",
        );
    }

    /// A grilling is driven by its session or by the watcher a pick armed on
    /// one, and by nothing else: a grilling with neither is a Conversation
    /// standing still.
    #[test]
    fn a_grilling_is_driven_by_its_session_or_by_the_watcher_on_it() {
        let drivers = Drivers::new();

        assert!(
            drivers.driven(&working(&[CONVERSATION]), CONVERSATION, Lifecycle::Grilling),
            "its grilling session is running",
        );
        assert!(
            !drivers.driven(&working(&[]), CONVERSATION, Lifecycle::Grilling),
            "its session has gone and it was never picked on, so nothing is grilling it",
        );

        let driving = drivers.driving(CONVERSATION);

        assert!(
            drivers.driven(&working(&[]), CONVERSATION, Lifecycle::Grilling),
            "a pick's watcher follows the grilling to the artifact it asked for",
        );

        drop(driving);

        assert!(
            !drivers.driven(&working(&[]), CONVERSATION, Lifecycle::Grilling),
            "and with the watcher gone too there is nothing left driving it",
        );
    }

    /// And the other way round for the states a task drives. A session comes and
    /// goes several times over one run, so what says the work is being driven is
    /// the task that keeps starting them rather than any one of them.
    #[test]
    fn implementing_and_wrapping_are_driven_by_the_task_that_runs_them() {
        let drivers = Drivers::new();

        for state in [
            Lifecycle::Implementing,
            Lifecycle::Wrapping,
            Lifecycle::FollowUp,
        ] {
            assert!(
                !drivers.driven(&working(&[CONVERSATION]), CONVERSATION, state),
                "{state:?} with a session running and no driver is a run nothing is seeing out",
            );
        }

        let driving = drivers.driving(CONVERSATION);

        for state in [
            Lifecycle::Implementing,
            Lifecycle::Wrapping,
            Lifecycle::FollowUp,
        ] {
            assert!(
                drivers.driven(&working(&[]), CONVERSATION, state),
                "{state:?} between one session and the next is still being driven",
            );
        }

        drop(driving);

        for state in [
            Lifecycle::Implementing,
            Lifecycle::Wrapping,
            Lifecycle::FollowUp,
        ] {
            assert!(
                !drivers.driven(&working(&[]), CONVERSATION, state),
                "{state:?} with the loop ended is not",
            );
        }
    }

    /// The states nothing is meant to be driving. Draft is waiting on the
    /// human, Done is finished and Closed is stopped: none of them is a
    /// Conversation standing still with nobody to move it.
    #[test]
    fn a_state_nothing_drives_is_never_one_standing_still() {
        let drivers = Drivers::new();

        for state in [Lifecycle::Draft, Lifecycle::Done, Lifecycle::Closed] {
            assert!(
                drivers.driven(&working(&[]), CONVERSATION, state),
                "{state:?} is not a state anything is supposed to be driving",
            );
        }
    }
}
