//! The watchers a pick arms: one per Conversation, watching for the artifact the
//! latest pick asked for.
//!
//! A pick does not move a Conversation — the artifact does — so what a pick
//! leaves behind is a watcher on the session that proposed. And a grilling may be
//! picked on more than once: the agent judges from the whole Response whether
//! everything is clear, and coming back with another Set, with a fresh proposal
//! on it, is one of the things it may decide. When that second proposal is picked
//! on, the first pick's watcher is watching for the wrong artifact.
//!
//! So the watchers are held here rather than spawned and forgotten. Arming one
//! cancels whatever this Conversation had armed before it, which is what makes
//! *latest pick wins* true of the running server as well as of the row the pick
//! wrote: exactly one watcher is live, and it is watching for the latest pick's
//! artifact.
//!
//! Nothing is ever taken out. A finished watcher's handle costs a pointer, and
//! cancelling one that has already returned does nothing at all — where a
//! register that forgot its own entries would have to tell *this* watcher
//! finishing apart from the one that superseded it.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tokio::task::JoinHandle;

/// The armed watchers, as whoever arms one holds them.
#[derive(Clone, Default)]
pub(crate) struct Followers {
    armed: Arc<Mutex<HashMap<i64, JoinHandle<()>>>>,
}

impl Followers {
    pub(crate) fn new() -> Followers {
        Followers::default()
    }

    /// Arm `watching` as this Conversation's watcher, cancelling the one it had.
    ///
    /// Cancelled rather than left to finish: the two would be watching one
    /// session for two different artifacts, and whichever landed first would end
    /// the session out from under the other. What the human last picked is what
    /// the grilling is being watched for.
    pub(crate) fn arm(&self, conversation_id: i64, watching: JoinHandle<()>) {
        let superseded = self
            .armed
            .lock()
            .expect("the followers registry is not poisoned")
            .insert(conversation_id, watching);

        if let Some(superseded) = superseded {
            tracing::debug!(
                conversation_id,
                "a later pick supersedes the one before it, so the watcher it armed is cancelled"
            );

            superseded.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    /// Arming a second watcher cancels the first, which is the whole of what this
    /// register is for: a Conversation has one watcher, and it is the latest
    /// pick's.
    #[tokio::test]
    async fn a_second_watcher_cancels_the_one_before_it() {
        let followers = Followers::new();
        let ran = Arc::new(AtomicUsize::new(0));

        let first = {
            let ran = ran.clone();
            tokio::spawn(async move {
                // Long enough that nothing but the cancellation ends it.
                tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                ran.fetch_add(1, Ordering::SeqCst);
            })
        };
        followers.arm(7, first);

        let second = {
            let ran = ran.clone();
            tokio::spawn(async move {
                ran.fetch_add(1, Ordering::SeqCst);
            })
        };
        followers.arm(7, second);

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        assert_eq!(
            ran.load(Ordering::SeqCst),
            1,
            "the second watcher ran and the first was cancelled before it could",
        );
    }

    /// And a watcher armed for another Conversation is nobody else's to cancel.
    #[tokio::test]
    async fn arming_one_conversations_watcher_leaves_anothers_alone() {
        let followers = Followers::new();
        let ran = Arc::new(AtomicUsize::new(0));

        for conversation_id in [7, 8] {
            let ran = ran.clone();
            followers.arm(
                conversation_id,
                tokio::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                    ran.fetch_add(1, Ordering::SeqCst);
                }),
            );
        }

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        assert_eq!(ran.load(Ordering::SeqCst), 2, "both watchers ran");
    }
}
