//! What a device holds of an **Agent Profile it is not bound on**, so that a
//! session can run here under it: a copy of the account, fetched from a device
//! the Profile is bound on before each launch — see [`account`] — and the
//! memory store carried with it — see [`memory`].
//!
//! **The Profiles themselves are records** (ADR-0028), replicated like any
//! other — see [`crate::replicating::profiles`]. The rows this module was named
//! for, a member's Profile held as a mirror of its home's and refreshed off its
//! `/api/ui/profiles`, are retired: every device holds the one record, an edit
//! or a removal made on any device is a local write that replicates, and there
//! is no home to relay a press to. What is left here is the account a session
//! away from it is given.

pub mod account;
pub mod memory;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// How many files this process has written beside their own names, which is what
/// tells two such writes apart — see [`aside`].
static WRITTEN_ASIDE: AtomicU64 = AtomicU64::new(0);

/// The name a file of a mirror is written under before it is renamed over its
/// own: beside it, and told apart from every other write this process makes.
///
/// **Beside and renamed** is what lets a session already running keep the file
/// it was given: a rename replaces the name and leaves the inode a bind is
/// holding alone. Both halves of a mirror are written that way — the account's
/// files in [`account`] and the memory store's in [`memory`] — and each keeps its
/// own rule about the mode, which is why this is the name rather than the write.
///
/// **And the process id is not enough to tell two of them apart**, which is the
/// case a mirror is squarely in: two Conversations under one member's Profile,
/// and a Terminal opened beside a running session, fetch into the **same** mirror
/// — and they are writes inside this one process, so a name built out of the pid
/// alone is one name for both. What that costs is not theoretical: whichever
/// renames first takes the name away, the other's rename fails for want of it,
/// and a launch that had nothing wrong with it is refused by
/// [`account::fetched`] — or, in the memory sync, a store is left unpulled with a
/// Notice about it. So the count goes on the end, and no two writes ever name one
/// file.
pub(crate) fn aside(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_owned();

    name.push(format!(
        ".verkstead-{}-{}",
        std::process::id(),
        WRITTEN_ASIDE.fetch_add(1, Ordering::Relaxed),
    ));

    PathBuf::from(name)
}

/// The most a member's Profiles may be before the answer is dropped rather than
/// merged: **one megabyte**.
///
/// A Profile is a name, a harness and a handful of model ids, and a binding an
/// account's paths — some hundreds of bytes — so a megabyte is thousands of
/// accounts, which is more than one person has. What it is for is the other
/// case: a machine on the far end of a relay that answers and then writes
/// without stopping, which this device makes one read of per member. The merged list's own bound, for its
/// reason — see [`crate::merging::MOST_A_LIST_IS`].
pub(crate) const MOST_THE_PROFILES_ARE: usize = 1024 * 1024;

#[cfg(test)]
mod tests {
    use super::*;

    /// Two writes beside one path are two names, so neither can take the other's
    /// away: which is the whole of what the count on the end is for — two
    /// Conversations under one member's Profile fetch into the same mirror, and
    /// they are two writes inside this one process.
    #[test]
    fn two_writes_beside_one_path_are_two_names() {
        let login = Path::new("/var/lib/verkstead/accounts/7/.claude/.credentials.json");

        let one = aside(login);
        let two = aside(login);

        assert_ne!(
            one, two,
            "a name two writers share is a name one of them loses"
        );

        for beside in [&one, &two] {
            assert_eq!(
                beside.parent(),
                login.parent(),
                "and it is written beside the file rather than anywhere else, so \
                 the rename that follows stays on the one filesystem",
            );
            assert_ne!(beside, login, "and never over it");
        }
    }
}
