//! What a session may reach, as an access-control entry on each real directory
//! the description names.
//!
//! The Windows half of a rendering, and the half that is the boundary. There is
//! nothing to mount and nothing to write a policy at: a session reaches what
//! its identity has been granted and nothing else, so a
//! [`Surface`](super::surface::Surface) becomes a list of entries written on the
//! human's own directories.
//!
//! **Whose identity is given rather than assumed.** A list worked out here
//! names no SID at all — [`writing::write`] is handed one — so what these
//! entries are for is the caller's to say. What it says today is the local
//! account of Verkstead's own that every session runs as (see
//! [`super::account`]), and the list did not change a word when it stopped
//! being an AppContainer profile's.

//!
//! **The list is worked out here and written next door.** What a description
//! comes to is a fact about the description rather than about Win32, so
//! [`entries`] is built and read on every platform and
//! [`writing`](self::writing) — which is compiled where there is an
//! access-control list to write — is the only part that touches the machine.
//! Which is what lets a test on a Linux box ask what a Windows session would be
//! granted, the way [`super::bwrap`]'s tests read flags nothing runs.
//!
//! **The vocabulary, as the probe found this platform answers it** (ADR-0014,
//! *What the probe answered*):
//!
//! - **`Own` and `Elsewhere`** are a grant at the Surface's reach on the real
//!   path — the *host* side of an `Elsewhere`, because a junction is followed
//!   and the grant is checked on its target, so what is granted is the account
//!   itself rather than the name a session finds it under.
//! - **`Empty` and `Temporary`** are a grant read-write: they are the session's
//!   own profile and the directory it throws things away in, which nobody else
//!   has any business in and which a session cannot do without.
//!
//! Read for the identity every session shares, they are what a Conversation's
//! boundary *is*: one account for the installation means the entries alone say
//! what one Conversation's session may reach that the next one's may not — see
//! [`super::entries`], which is what holds a Conversation's and takes them off
//! the machine when its work stops.

//! - **`Nothing`** is refused — see [`Wanted::Refused`], and
//!   [`writing::refuse`] for the mechanism, which is the one part of this the
//!   probe could not settle from outside.
//! - **`ProcessTable` and `Devices`** are nothing at all here. There is no
//!   process table in the filesystem on this platform, and the devices a
//!   program opens by name are the machine's own.
//!
//! **And one thing that is in no description**: each `PATH` entry under the
//! human's own profile, read-only. Program Files, the system directory and
//! Windows PowerShell are all readable by an ordinary local account with no
//! entry at all — the probe ran `node`, `git` and PowerShell out of them — but a
//! per-user tool install is not, and an agent installed by npm is exactly that.
//! So the directories a session is told to look for a program in are granted
//! where they are the human's own, and nothing else of that profile is.

//!
//! **And a step through every directory on the way to a granted path** — see
//! [`Wanted::Stepped`], which is the one thing here that is about *resolving* a
//! path rather than about reaching one. *Reaching* a path deep under the
//! human's profile needs no entry above it: an ordinary account holds the
//! privilege that skips the traverse check, which is why a probe reached one
//! with nothing anywhere above it. But *resolving* a path walks its prefixes and
//! asks each for its attributes, and an agent resolves a path before it reads it
//! — so a session granted its Worktree and nothing on the way to it refuses its
//! own Worktree the moment it checks. One entry on each directory along the way
//! answers that, and says nothing whatever about what is inside: the human's
//! profile is walked through in the same breath as staying unlistable.

#[cfg(windows)]
pub(crate) mod writing;

// And what was written down about what was written: the record under the Data
// Directory that lets a server which did not write an entry take it away all
// the same. Built everywhere, for this module's own reason — a record is a
// file, and only the taking-back of an entry is a Win32 call.
//
// Half of it is a Win32 machine's all the same: what *writes* a record is a
// Conversation's boundary being begun, which happens on one platform only, so a
// build for either of the others carries the writing of one for its tests and
// for the shape of the thing rather than for anything it does.

#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) mod remembering;

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::surface::{Access, Reach, Surface};

/// What one path is to be, once the entries are written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Wanted {
    /// Reachable that far, and everything under it with it: a grant inherits
    /// down a tree, which is what makes one entry the answer for a Worktree
    /// rather than one per file in it.
    Granted(Reach),

    /// Stepped through on the way to somewhere else: its attributes readable
    /// and a walk through it allowed, and **not inherited**, so it says nothing
    /// whatever about what is inside.
    ///
    /// **What resolving a path needs and reaching one does not** — see this
    /// module's own documentation. Nothing here is a reach: a directory a
    /// session may only step through is one it cannot list, cannot read a file
    /// out of and cannot write to, and the entry stops at that directory rather
    /// than propagating to anything under it.
    Stepped,

    /// And refused, whatever a grant above it says — the account's own skills,
    /// which a session is to find nothing at.
    Refused,
}

/// One path and what it is to be.
///
/// Read by name rather than through accessors, because the one thing that reads
/// one is [`writing`], which is inside this module: an entry is a pair, and a
/// pair with two functions in front of it would be two functions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Entry {
    /// The real path on the host the entry goes on.
    path: PathBuf,

    /// And what it says.
    wanted: Wanted,
}

/// Everything `surface` says, as the entries that make it true — in the order
/// the description said it.
///
/// **The order is the description's**, for the reason [`Surface`] keeps one: a
/// path said twice is the second one, and what covers the account's own skills
/// is said after the account it is inside. Written in that order, the refusal
/// lands over a grant rather than under it.
///
/// **And the steps come last**, after every path the description names is in
/// the list, because what a step is wanted on is what nothing else has spoken
/// for — see [`stepping`]. Written last too, which costs nothing: a step
/// inherits to nowhere, so no list below it is walked and no earlier entry is
/// disturbed.
///
/// `profile` is the human's own — where the account running the server keeps
/// its things — and is what the `PATH` rule above is measured against. `None`
/// where the machine will not say, which costs a session the per-user tools on
/// its `PATH` and nothing else.
pub(crate) fn entries(surface: &Surface, profile: Option<&Path>) -> Vec<Entry> {
    let mut entries = Vec::new();

    // First, because these are the machine's floor rather than this session's:
    // a description that goes on to grant one of them at a wider reach is a
    // description whose word is the later one.
    if let Some(profile) = profile {
        for directory in super::open::looked_in(surface) {
            if beneath(directory, profile) {
                entries.push(granted(directory, Reach::ReadOnly));
            }
        }
    }

    for access in surface.reaches() {
        match access {
            Access::Own { path, reach } => entries.push(granted(path, *reach)),

            // The host's side of it, which is where the grant belongs: a
            // junction is followed and the grant is checked on its target, so
            // an entry on the name a session finds the account under would be
            // an entry on the junction rather than on the account.
            Access::Elsewhere { host, reach, .. } => entries.push(granted(host, *reach)),

            // The session's own profile and what it throws away, both of them
            // made by the rendering a moment before this — see
            // [`super::open::command`], which is what puts the directory there
            // for the entry to go on.
            Access::Empty(path) | Access::Temporary(path) => {
                entries.push(granted(path, Reach::ReadWrite));
            }

            Access::Nothing { inside, .. } => entries.push(Entry {
                path: inside.clone(),
                wanted: Wanted::Refused,
            }),

            // Neither of which is a path on this platform — see this module's
            // own documentation.
            Access::ProcessTable | Access::Devices => {}
        }
    }

    // Last, and with every path the description names already in the list:
    // what a step is wanted on is what nothing else has spoken for — see
    // [`stepping`].
    let steps = stepping(&entries);
    entries.extend(steps);

    entries
}

/// One entry on every directory on the way to a granted path, so that the path
/// resolves at all.
///
/// **Outermost first**, which is the order a resolution walks a path in and so
/// the order the entries are for.
///
/// **Nothing is spoken for twice.** A directory that is on the way to two
/// granted paths takes one entry, and one the description already names —
/// granted or refused — takes none at all: a step is the narrowest thing
/// written here, and one landing on a path that is already granted would be a
/// session behind a boundary narrower than its description. Which is why this
/// is worked out over the whole list rather than as each entry is made.
///
/// **A refusal is stepped to by nothing.** What a description refuses is inside
/// something it grants, so the way to it is already here — and a step of its
/// own would be an entry on the very directories a refusal exists to keep a
/// session out of.
fn stepping(entries: &[Entry]) -> Vec<Entry> {
    let named: HashSet<String> = entries.iter().map(|entry| folded(&entry.path)).collect();

    let mut already = HashSet::new();
    let mut stepping = Vec::new();

    for entry in entries
        .iter()
        .filter(|entry| entry.wanted != Wanted::Refused)
    {
        for directory in ancestors(&entry.path) {
            let folded = folded(&directory);

            if named.contains(&folded) || !already.insert(folded) {
                continue;
            }

            stepping.push(Entry {
                path: directory,
                wanted: Wanted::Stepped,
            });
        }
    }

    stepping
}

/// Every directory on the way to `path`, outermost first and `path` itself not
/// among them.
///
/// **Read off the string rather than off [`Path`]**, for the reason [`folded`]
/// is: this is worked out wherever the tests run, and `Path` on a Unix reads
/// `C:\Users\ada` as one component with nothing above it — so an ancestor walk
/// through it would answer about Windows paths only on Windows, which is the
/// one thing this module is arranged not to do.
///
/// A drive and a root are the directory *with* the separator on it: `C:\` is a
/// directory where `C:` is a drive-relative nothing, and `/` is the root where
/// the empty string is no path at all.
///
/// Nothing here is clever about the prefixes Windows has that are not drives —
/// a UNC share comes out as a step on `\` and a step on the share, and both are
/// paths the machine will simply refuse an entry on. Which is an answer rather
/// than a fault: see [`writing::write`], where an ancestor that will not take
/// an entry is counted and the session goes on.
fn ancestors(path: &Path) -> Vec<PathBuf> {
    let whole = path.to_string_lossy();
    let whole = whole.trim_end_matches(['\\', '/']);

    whole
        .match_indices(['\\', '/'])
        .map(|(at, _)| {
            let so_far = &whole[..at];

            if so_far.is_empty() || so_far.ends_with(':') {
                PathBuf::from(&whole[..=at])
            } else {
                PathBuf::from(so_far)
            }
        })
        .collect()
}

/// One path granted that far.
fn granted(path: impl Into<PathBuf>, reach: Reach) -> Entry {
    Entry {
        path: path.into(),
        wanted: Wanted::Granted(reach),
    }
}

/// Whether `path` is somewhere under `directory`, as this platform reads two
/// paths.
///
/// Case-folded, because a Windows filesystem is: a `PATH` written
/// `C:\Users\Ada\AppData` and a profile read back as `C:\Users\ada` are one
/// directory, and a comparison that said otherwise would leave the tools a
/// session needs ungranted on half the machines there are.
///
/// **Strictly under**, which is the whole rule rather than a nicety: a `PATH`
/// entry that *is* the profile would otherwise grant the human's whole account
/// read-only, and the account is the one thing this platform's boundary is
/// about.
fn beneath(path: &Path, directory: &Path) -> bool {
    let (path, directory) = (folded(path), folded(directory));

    path.len() > directory.len()
        && path.starts_with(&directory)
        && matches!(path.as_bytes().get(directory.len()), Some(b'\\' | b'/'))
}

/// A path as the comparison above reads one: lower case, and with a trailing
/// separator dropped so that a `PATH` entry written with one and a profile
/// read back without are still one directory and one under it.
fn folded(path: &Path) -> String {
    path.to_string_lossy()
        .to_lowercase()
        .trim_end_matches(['\\', '/'])
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::ffi::OsString;

    /// A description with one of everything in it, in the order a session's own
    /// is built in.
    fn described(account: &Path, home: &Path) -> Surface {
        let mut surface = Surface::starting_in(PathBuf::from(r"C:\repo"));

        surface.made(Access::Empty(home.to_owned()));
        surface.made(Access::Temporary(home.join("Temp")));
        surface.made(Access::ProcessTable);
        surface.made(Access::Devices);
        surface
            .own(r"C:\repo", Reach::ReadWrite)
            .own(r"C:\repo\.git", Reach::ReadWrite)
            .own(r"C:\ProgramData\verkstead\skills", Reach::ReadOnly)
            .elsewhere(account, home.join(".claude"), Reach::ReadWrite)
            .nothing(home.join(".claude").join("skills"), PathBuf::from("unused"));

        surface
    }

    /// Every part of the vocabulary, as the entry it comes to.
    ///
    /// The steps are left out here and asked for on their own below: they are
    /// about the way to a path rather than about the path, so a list of them
    /// mixed in would make this assertion about both at once.
    #[test]
    fn each_kind_of_access_is_the_entry_it_says() {
        let (account, home) = (Path::new(r"C:\Users\ada\.claude"), Path::new(r"D:\homes\7"));

        let entries = entries(&described(account, home), None);

        assert_eq!(
            entries
                .iter()
                .filter(|entry| entry.wanted != Wanted::Stepped)
                .map(|entry| (entry.path.clone(), entry.wanted))
                .collect::<Vec<_>>(),
            vec![
                (home.to_owned(), Wanted::Granted(Reach::ReadWrite)),
                (home.join("Temp"), Wanted::Granted(Reach::ReadWrite)),
                (PathBuf::from(r"C:\repo"), Wanted::Granted(Reach::ReadWrite)),
                (
                    PathBuf::from(r"C:\repo\.git"),
                    Wanted::Granted(Reach::ReadWrite)
                ),
                (
                    PathBuf::from(r"C:\ProgramData\verkstead\skills"),
                    Wanted::Granted(Reach::ReadOnly)
                ),
                // The account itself rather than the name it is found under,
                // and the refusal after it, which is the order the description
                // said them in.
                (account.to_owned(), Wanted::Granted(Reach::ReadWrite)),
                (home.join(".claude").join("skills"), Wanted::Refused),
            ],
            "the process table and the devices are nothing at all here, and \
             everything else is one entry",
        );
    }

    /// And the way to every one of them, which is the whole of what a step is:
    /// one entry on each directory a granted path is resolved through, and
    /// none on a directory the description has already spoken for.
    ///
    /// **Asserted whole and in order**, because both are the point: a step
    /// written twice on one directory would be a second entry saying what the
    /// first says, and one written on a path that is *granted* would be a
    /// grant narrowed to a step.
    #[test]
    fn every_granted_path_is_stepped_through_and_nothing_else_is() {
        let (account, home) = (Path::new(r"C:\Users\ada\.claude"), Path::new(r"D:\homes\7"));

        let entries = entries(&described(account, home), None);

        assert_eq!(
            entries
                .iter()
                .filter(|entry| entry.wanted == Wanted::Stepped)
                .map(|entry| entry.path.clone())
                .collect::<Vec<_>>(),
            vec![
                // The way to the session's own profile, and to the temporary
                // directory inside it — which is under the profile and so
                // needs nothing of its own.
                PathBuf::from(r"D:\"),
                PathBuf::from(r"D:\homes"),
                // The way to the Worktree, which is the drive and nothing else
                // — its `.git` is under a directory already granted, so the
                // way to that is already here.
                PathBuf::from(r"C:\"),
                // And the way to the skills of Verkstead's own.
                PathBuf::from(r"C:\ProgramData"),
                PathBuf::from(r"C:\ProgramData\verkstead"),
                // And the way to the account, which is the human's own profile
                // being walked through without being given.
                PathBuf::from(r"C:\Users"),
                PathBuf::from(r"C:\Users\ada"),
            ],
            "every directory on the way to a granted path is stepped through \
             once, outermost first, and a path the description already names \
             is not stepped through at all",
        );
    }

    /// And what a refusal is on the way to: nothing.
    ///
    /// The directories above a refused path are the ones a refusal exists to
    /// keep a session out of, and the way to it is the granted path it is
    /// inside — which is already in the list.
    #[test]
    fn a_refused_path_is_stepped_to_by_nothing() {
        let home = Path::new(r"D:\homes\7");
        let mut surface = Surface::starting_in(PathBuf::from(r"C:\repo"));

        surface.nothing(home.join(".claude").join("skills"), PathBuf::from("unused"));

        assert_eq!(
            entries(&surface, None)
                .iter()
                .filter(|entry| entry.wanted == Wanted::Stepped)
                .count(),
            0,
            "a description that refuses one path and grants none is one step \
             through nothing at all",
        );
    }

    /// The ancestor walk itself, which is read off the string so that it
    /// answers the same on every machine the tests run on.
    #[test]
    fn the_way_to_a_path_is_read_the_same_wherever_it_is_read() {
        assert_eq!(
            ancestors(Path::new(r"C:\Users\ada\AppData\Roaming\npm")),
            vec![
                PathBuf::from(r"C:\"),
                PathBuf::from(r"C:\Users"),
                PathBuf::from(r"C:\Users\ada"),
                PathBuf::from(r"C:\Users\ada\AppData"),
                PathBuf::from(r"C:\Users\ada\AppData\Roaming"),
            ],
            "a drive is the directory with the separator on it, and the path \
             itself is not on the way to itself",
        );

        assert_eq!(
            ancestors(Path::new(r"D:\repo\")),
            vec![PathBuf::from(r"D:\")],
            "and a trailing separator is not a directory of its own",
        );

        assert_eq!(
            ancestors(Path::new(r"C:\")),
            Vec::<PathBuf>::new(),
            "and a drive has nothing above it",
        );
    }

    /// And the `PATH` rule, which is in no description: what is under the
    /// human's own profile is granted read-only and what is not is left alone.
    #[test]
    fn only_the_path_entries_under_the_humans_profile_are_granted() {
        let (account, home) = (Path::new(r"C:\Users\ada\.claude"), Path::new(r"D:\homes\7"));
        let mut surface = described(account, home);

        surface.set(
            "Path",
            OsString::from(concat!(
                r"D:\verkstead\bin;",
                r"C:\Users\Ada\AppData\Roaming\npm;",
                r"C:\Program Files\Git\cmd;",
                r"C:\Users\ada",
            )),
        );

        let granted: Vec<_> = entries(&surface, Some(Path::new(r"C:\Users\ada")))
            .into_iter()
            .filter(|entry| entry.wanted == Wanted::Granted(Reach::ReadOnly))
            .map(|entry| entry.path.clone())
            .collect();

        assert!(
            granted.contains(&PathBuf::from(r"C:\Users\Ada\AppData\Roaming\npm")),
            "a per-user tool install is what this rule is for, whatever case \
             the machine wrote it in: {granted:?}"
        );
        assert!(
            !granted.contains(&PathBuf::from(r"C:\Program Files\Git\cmd")),
            "and Program Files needs no entry at all: {granted:?}"
        );
        assert!(
            !granted.contains(&PathBuf::from(r"D:\verkstead\bin")),
            "and Verkstead's own directory is granted by the description \
             rather than by this: {granted:?}"
        );
        assert!(
            !granted.contains(&PathBuf::from(r"C:\Users\ada")),
            "and a `PATH` entry that is the profile itself would be the whole \
             account read-only, which is the one thing this boundary is \
             about: {granted:?}"
        );
    }
}
