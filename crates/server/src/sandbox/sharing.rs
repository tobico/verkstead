//! A root something is still running in, shared by the next launch rather than
//! built again under it — and, on the two platforms whose HOME is a real
//! directory, that HOME as well.
//!
//! **What emptying a root would take with it.** On Linux a root is a directory
//! under the Data Directory that the namespace binds in, and what is joined
//! into it is bound onto mount points inside it. Emptying that directory as a
//! Conversation Terminal starts would unlink those mount points from under a
//! session still running, which the kernel answers by unmounting them inside
//! that session: its transcript, its memory and its login would all be gone
//! from where it writes them. And a Claude session's `.claude.json` copy would
//! be a deleted file, whose changes are merged against a copy it never had.
//!
//! On a Mac and on Windows nothing is mounted, so emptying a root does not
//! unmount anything — it deletes. Where the Profile shares no memory that is
//! worse rather than better: the root holds the session's own store, which is
//! the only copy there is of its memory and its transcript, and an OpenCode
//! root holds the database it is writing — see [`super::root`].
//!
//! So a launch into a root that already has something running in it is given
//! that root as it is: nothing emptied, nothing written, and the same copy of
//! `.claude.json` with the same baseline to merge against. The root is built
//! afresh only once everything running in it has ended.
//!
//! **One register entry per Conversation and root**, rather than per
//! Conversation. A Conversation's grilling session and its terminal can run
//! under Profiles of two harnesses — a Claude session with a terminal under a
//! Codex account beside it — and each is given a root of its own, `.claude` and
//! `.codex`, side by side in the Conversation's directory. A launch into one is
//! nothing to the other.
//!
//! **And the HOME the roots are in is one Conversation's**, which is why a
//! launch is told whether it may empty that too — see [`Launch::empties`]. On
//! Linux it is a directory made inside the namespace and nothing of the host's
//! is removed with it, so there is nothing there to keep. On the other two it
//! is the Conversation's own directory, holding every root of it: emptying it
//! as a terminal starts would take the running session's root away whichever
//! harness that terminal is under, so nothing of the Conversation may be
//! running in it. See [`super::Homes`].

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

/// How many launches are running in each of a Conversation's roots, and the
/// baseline their copy is merged against. One per server, shared by every
/// session and every terminal.
#[derive(Debug, Clone, Default)]
pub(crate) struct Sharing(Arc<Mutex<HashMap<Root, Running>>>);

/// Which root: the Conversation's id, and what the root is called in its
/// directory — `.claude` or `.codex`.
type Root = (i64, &'static str);

#[derive(Debug, Default)]
struct Running {
    launches: usize,
    baseline: Baseline,
}

/// What a launch is given: whether it builds the root or shares one, whether
/// it may empty the HOME that root is in, and the baseline its copy is merged
/// against either way.
pub(crate) struct Launch {
    /// `true` where nothing else is running in the root, so this launch
    /// empties and builds it.
    pub(crate) builds: bool,

    /// `true` where nothing else is running in *any* of the Conversation's
    /// roots, so the HOME holding them may be emptied — which on the two
    /// platforms that make a real one is what would take a running session's
    /// root with it.
    pub(crate) empties: bool,

    pub(crate) baseline: Baseline,
}

/// One launch running in one of a Conversation's roots, until this is dropped —
/// which is when the [`super::Closing`] holding it has been seen to.
#[derive(Debug)]
pub(crate) struct Share {
    sharing: Sharing,
    root: Root,
}

impl Sharing {
    /// Launch into `conversation`'s root called `named`: `launch` is told whether
    /// it builds the root or shares it, and what it comes back with is held as
    /// one more launch running there until the [`Share`] beside it is dropped.
    ///
    /// **`launch` runs under the lock**, so a launch that shares a root never
    /// sees it half built, and two launches never both build it. And the lock is
    /// one register rather than one per Conversation, so what runs under it is
    /// what every other Conversation's start waits for.
    ///
    /// **Which is the whole of a launch, and on Windows that is more than a
    /// directory.** Building a root is emptying a small directory and writing a
    /// file or two, and on the two platforms with a wrapper that is all there is.
    /// On Windows the rendering joins the account in by hand and the boundary is
    /// written after it — a logon as the session account, and an access-control
    /// entry on every path the description names — so a second Conversation
    /// starting at that moment waits for those as well. Held all the same:
    /// sessions start one at a time on a machine with one human at it, and what
    /// the lock is keeping is a profile from being emptied out from under a
    /// session already running in it.
    pub(crate) fn launched<T>(
        &self,
        conversation: i64,
        named: &'static str,
        launch: impl FnOnce(Launch) -> std::io::Result<T>,
    ) -> std::io::Result<(T, Share)> {
        let mut held = self.0.lock().expect("the root register is not poisoned");
        let root = (conversation, named);

        // Whether anything of the Conversation is running in any root of it,
        // which is what says whether the HOME they are all in may be emptied.
        // Asked before the entry below, which would count this launch's own
        // root as running in a moment.
        let empties = !held
            .iter()
            .any(|((held, _), running)| *held == conversation && running.launches > 0);

        let running = held.entry(root).or_default();

        let launched = launch(Launch {
            builds: running.launches == 0,
            empties,
            baseline: running.baseline.clone(),
        })?;

        running.launches += 1;

        Ok((
            launched,
            Share {
                sharing: self.clone(),
                root,
            },
        ))
    }
}

impl Drop for Share {
    fn drop(&mut self) {
        let Ok(mut held) = self.sharing.0.lock() else {
            return;
        };

        if let Some(running) = held.get_mut(&self.root) {
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
                .launched(conversation, ".claude", |launch| Ok(launch.builds))
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

    /// A terminal under a Codex account beside a Claude session is a launch
    /// into a root nothing is running in, so it builds its own.
    #[test]
    fn two_roots_of_one_conversation_are_built_apart() {
        let sharing = Sharing::default();

        let (built, _session) = sharing
            .launched(7, ".claude", |launch| Ok(launch.builds))
            .unwrap();
        assert!(built);

        let (built, _terminal) = sharing
            .launched(7, ".codex", |launch| Ok(launch.builds))
            .unwrap();
        assert!(
            built,
            "nothing is running in the Codex root, so it is built"
        );
    }

    #[test]
    fn a_launch_that_fails_is_not_left_running() {
        let sharing = Sharing::default();

        assert!(
            sharing
                .launched(7, ".claude", |_| Err::<(), _>(std::io::Error::other(
                    "refused"
                )))
                .is_err()
        );

        let (built, _) = sharing
            .launched(7, ".claude", |launch| Ok(launch.builds))
            .unwrap();
        assert!(built);
    }

    #[test]
    fn every_launch_in_one_root_is_given_one_baseline() {
        let sharing = Sharing::default();

        let (first, _one) = sharing
            .launched(7, ".claude", |launch| Ok(launch.baseline))
            .unwrap();
        *first.lock().unwrap() = b"given\n".to_vec();

        let (second, _two) = sharing
            .launched(7, ".claude", |launch| Ok(launch.baseline))
            .unwrap();
        assert_eq!(*second.lock().unwrap(), b"given\n");
    }
}
