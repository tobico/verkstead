//! What the human picks on a grilling's proposal, carried to the interview
//! waiting on its session: one channel per Conversation, holding the latest
//! pick.
//!
//! A pick does not move a Conversation — the artifact does — so what a pick
//! changes is what the grilling session's Done signal is checked against. The
//! interview is waiting on that session through the host, launched expecting
//! `picked` (see [`crate::lending`]), and it is the wait that reads the pick
//! here and holds the signal to the artifact it asked for.
//!
//! **Latest pick wins.** A grilling may be picked on more than once: the agent
//! judges from the whole Response whether everything is clear, and coming back
//! with another Set, with a fresh proposal on it, is one of the things it may
//! decide. When that second proposal is picked on, the first pick's artifact is
//! the wrong one to be watching for, so a pick replaces the one before it and
//! the wait moves onto the new one.
//!
//! **One interview listens at a time.** Listening opens a fresh channel and
//! drops the one before it, so a grilling relaunched never starts out holding
//! the last one's pick, and the wait it replaced hears nothing more.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tokio::sync::watch;
use verkstead_schema::Direction;

/// The picks being listened for, by Conversation.
#[derive(Clone, Default)]
pub(crate) struct Followers {
    picks: Arc<Mutex<HashMap<i64, watch::Sender<Option<Direction>>>>>,
}

impl Followers {
    pub(crate) fn new() -> Followers {
        Followers::default()
    }

    /// Listen for this Conversation's picks from now on, with nothing picked
    /// yet, in place of whoever listened before.
    pub(crate) fn listen(&self, conversation_id: i64) -> watch::Receiver<Option<Direction>> {
        let (picking, picks) = watch::channel(None);

        self.picks
            .lock()
            .expect("the followers registry is not poisoned")
            .insert(conversation_id, picking);

        picks
    }

    /// Say the human picked `direction`. Whether anything was listening: a
    /// Conversation whose interview is not running has nothing to hear it.
    pub(crate) fn picked(&self, conversation_id: i64, direction: Direction) -> bool {
        let picks = self
            .picks
            .lock()
            .expect("the followers registry is not poisoned");

        match picks.get(&conversation_id) {
            Some(picking) => picking.send(Some(direction)).is_ok(),
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A second pick replaces the first, which is the whole of what this
    /// register is for: what the wait reads is the latest pick.
    #[tokio::test]
    async fn a_second_pick_replaces_the_one_before_it() {
        let followers = Followers::new();
        let mut picks = followers.listen(7);

        assert!(followers.picked(7, Direction::Inline));
        assert!(followers.picked(7, Direction::Roadmap));

        picks.changed().await.unwrap();
        assert_eq!(*picks.borrow_and_update(), Some(Direction::Roadmap));
    }

    /// And a pick on another Conversation is nobody else's to hear, and one
    /// nothing listens for says so.
    #[tokio::test]
    async fn a_pick_reaches_its_own_conversations_listener_alone() {
        let followers = Followers::new();
        let picks = followers.listen(7);

        assert!(
            !followers.picked(8, Direction::Inline),
            "nobody listens on 8"
        );
        assert_eq!(*picks.borrow(), None);
    }

    /// Listening again starts from nothing picked, and the listener it
    /// replaced hears nothing more.
    #[tokio::test]
    async fn listening_again_starts_from_nothing_picked() {
        let followers = Followers::new();
        let mut before = followers.listen(7);
        followers.picked(7, Direction::TaskList);

        let after = followers.listen(7);

        assert_eq!(*after.borrow(), None);
        before.borrow_and_update();
        assert!(before.changed().await.is_err(), "the old channel is closed");
    }
}
