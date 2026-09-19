//! A Claude root on Linux that something is still running in, shared by the
//! next launch rather than built again under it.
//!
//! **Why Linux alone.** On Linux a root is a directory under the Data Directory
//! that the namespace binds in, and what is joined into it is bound onto mount
//! points inside it. Emptying that directory as a Conversation Terminal starts
//! would unlink those mount points from under a session still running, which
//! the kernel answers by unmounting them inside that session: its transcript,
//! its memory and its login would all be gone from where it writes them. And
//! the `.claude.json` copy it is bound to would be a deleted file, whose
//! changes are merged against a copy it never had.
//!
//! So a launch into a Conversation that already has something running in its
//! root is given that root as it is: nothing emptied, nothing written, and the
//! same copy of `.claude.json` with the same baseline to merge against. The
//! root is built afresh only once everything running in it has ended.
//!
//! On a Mac and on Windows the profile is the HOME itself, and is emptied as it
//! always was — see [`super::Homes`].

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// The copy of `.claude.json` as the account last had it, which is what a
/// merge back compares the session's copy against — see
/// [`super::root::merged_back`].
///
/// **Shared**, where a root is: every launch in one root has one copy, and each
/// merge moves this on to what it merged, so the next ending carries only what
/// changed after it.
pub(crate) type Baseline = Arc<Mutex<Vec<u8>>>;

/// How many launches are running in each Conversation's root, and the baseline
/// their copy is merged against. One per server, shared by every session and
/// every terminal.
#[derive(Debug, Clone, Default)]
pub(crate) struct Sharing(Arc<Mutex<HashMap<i64, Running>>>);

#[derive(Debug, Default)]
struct Running {
    launches: usize,
    baseline: Baseline,
}

/// What a launch is given: whether it builds the root or shares one, and the
/// baseline its copy is merged against either way.
pub(crate) struct Launch {
    /// `true` where nothing else is running in the root, so this launch
    /// empties and builds it.
    pub(crate) builds: bool,

    pub(crate) baseline: Baseline,
}

/// One launch running in a Conversation's root, until this is dropped — which
/// is when the [`super::Closing`] holding it has been seen to.
#[derive(Debug)]
pub(crate) struct Share {
    sharing: Sharing,
    conversation: i64,
}

impl Sharing {
    /// Launch into `conversation`'s root: `launch` is told whether it builds the
    /// root or shares it, and what it comes back with is held as one more launch
    /// running there until the [`Share`] beside it is dropped.
    ///
    /// **`launch` runs under the lock**, so a launch that shares a root never
    /// sees it half built, and two launches never both build it. Building a root
    /// is emptying a small directory and writing two files, so every other
    /// Conversation waits no longer than that.
    pub(crate) fn launched<T>(
        &self,
        conversation: i64,
        launch: impl FnOnce(Launch) -> std::io::Result<T>,
    ) -> std::io::Result<(T, Share)> {
        let mut held = self.0.lock().expect("the root register is not poisoned");
        let running = held.entry(conversation).or_default();

        let launched = launch(Launch {
            builds: running.launches == 0,
            baseline: running.baseline.clone(),
        })?;

        running.launches += 1;

        Ok((
            launched,
            Share {
                sharing: self.clone(),
                conversation,
            },
        ))
    }
}

impl Drop for Share {
    fn drop(&mut self) {
        let Ok(mut held) = self.sharing.0.lock() else {
            return;
        };

        if let Some(running) = held.get_mut(&self.conversation) {
            running.launches = running.launches.saturating_sub(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_launch_builds_and_the_rest_share_until_every_one_has_ended() {
        let sharing = Sharing::default();

        let builds = |sharing: &Sharing, conversation| {
            sharing
                .launched(conversation, |launch| Ok(launch.builds))
                .unwrap()
        };

        let (built, session) = builds(&sharing, 7);
        assert!(built, "nothing else is running, so this one builds");

        let (built, terminal) = builds(&sharing, 7);
        assert!(
            !built,
            "a session is running in it, so a terminal shares it"
        );

        let (built, _elsewhere) = builds(&sharing, 8);
        assert!(built, "and another Conversation's root is its own");

        drop(session);
        let (built, second) = builds(&sharing, 7);
        assert!(!built, "the terminal is still running in it");

        drop(terminal);
        drop(second);
        let (built, _) = builds(&sharing, 7);
        assert!(built, "and once nothing is, it is built afresh");
    }

    #[test]
    fn a_launch_that_fails_is_not_left_running() {
        let sharing = Sharing::default();

        assert!(
            sharing
                .launched(7, |_| Err::<(), _>(std::io::Error::other("refused")))
                .is_err()
        );

        let (built, _) = sharing.launched(7, |launch| Ok(launch.builds)).unwrap();
        assert!(built);
    }

    #[test]
    fn every_launch_in_one_root_is_given_one_baseline() {
        let sharing = Sharing::default();

        let (first, _one) = sharing.launched(7, |launch| Ok(launch.baseline)).unwrap();
        *first.lock().unwrap() = b"given\n".to_vec();

        let (second, _two) = sharing.launched(7, |launch| Ok(launch.baseline)).unwrap();
        assert_eq!(*second.lock().unwrap(), b"given\n");
    }
}
