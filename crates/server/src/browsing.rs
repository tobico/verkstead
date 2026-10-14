//! Reading one directory for a path field's dropdown: what is in it, and what
//! each of those things is.
//!
//! One directory per ask and no walking: a field browses by asking again for
//! each level somebody drills into, so nothing here ever recurses and a
//! directory holding ten thousand files costs one `read_dir`.
//!
//! One question, asked of every field: what is at this path. Nothing bounds it
//! but what the server can read, which is a wider disclosure than anything else
//! here makes and was settled as one — a field that could not reach the
//! directory it is about to be pointed at would be a field nobody could fill
//! in, and every path field is now one of those.
//!
//! **An ask with no path opens on the server's own home directory**, that being
//! where the human's repositories and the account an Agent Profile names both
//! most often are. A starting point rather than a ceiling: the listing walks up
//! out of it like any other directory — see [`list`], and [`topmost`] for what a
//! server with no home to read opens on instead.
//!
//! Nothing here refuses by status code. A path that is relative, missing, not a
//! directory or unreadable is a named outcome the dropdown draws where its rows
//! would be — see [`DirectoryListing`]. A field is typed into a character at a
//! time, so most of those are the ordinary state of a field halfway through a
//! word rather than anything that went wrong.

use std::path::{Path, PathBuf};

use verkstead_render::{DirectoryEntry, DirectoryListing, EntryKind};

use crate::platform::{Environment, Platform};
use crate::resolved::{Resolved, resolve};

/// What `path` holds, or the named reason it holds nothing.
///
/// No path at all is the field standing empty, and what that opens on is the
/// server's own home — see [`opening`], which is also where a server with no
/// home to read falls back to the top of the machine.
///
/// Resolved through [`crate::resolved`], which is what the save behind the
/// field resolves through: *absolute* and *there* are one pair of questions
/// rather than one pair per caller, so a dropdown cannot come to answer them
/// differently from the save it is filling a field in for.
///
/// Blocking: the directory is opened, and every entry in it is asked what it is.
pub(crate) fn list(path: Option<PathBuf>) -> DirectoryListing {
    let Some(path) = path else {
        return opening(home());
    };

    match resolve(&path) {
        Resolved::At(real) => entries_of(&real),
        Resolved::NotAbsolute => DirectoryListing::NotAbsolute,
        Resolved::Missing => DirectoryListing::Missing,
    }
}

/// The home directory of whoever is running the server, read from the
/// environment the way every other use of it is — see
/// [`crate::platform::home_dir`], which is where Windows' `%USERPROFILE%` is
/// read instead of `$HOME`.
fn home() -> Option<PathBuf> {
    crate::platform::home_dir(Platform::HERE, &Environment::of_the_process())
}

/// Where a browse with nothing typed in the field opens.
///
/// `home` where it lists, and the top of the machine where it does not: a home
/// nothing says, a home that will not resolve, and a home that is not a
/// directory are one answer between them, because none of them is something the
/// human could correct from a dropdown. The fallback is what this endpoint
/// answered an empty field with before there was a home in it at all, so
/// nothing is out of reach either way.
///
/// Resolved the way a typed path is — see [`list`] — so a `HOME` that is
/// relative is one of the homes there is no listing to be had of, rather than a
/// directory read from wherever the server happened to be started.
fn opening(home: Option<PathBuf>) -> DirectoryListing {
    let Some(Resolved::At(real)) = home.as_deref().map(resolve) else {
        return topmost();
    };

    match entries_of(&real) {
        listed @ DirectoryListing::Listed { .. } => listed,
        _ => topmost(),
    }
}

/// Where a browse opens when there is no home to open on, which is the one thing
/// about it the platforms disagree on.
///
/// A Unix has one filesystem root and the browse opens on what `/` holds. A
/// Windows machine has one root per drive and nothing above them — `/` is not
/// even a path [`Path::is_absolute`] accepts there, having a root but no prefix
/// — so the browse opens on the drives themselves, as a listing with no
/// directory above it.
#[cfg(not(windows))]
fn topmost() -> DirectoryListing {
    entries_of(Path::new("/"))
}

/// The drives, for the reason above.
#[cfg(windows)]
fn topmost() -> DirectoryListing {
    let mut entries: Vec<DirectoryEntry> = drives(|drive| drive.is_dir())
        .into_iter()
        .filter_map(|drive| entry(root_name(&drive)?, drive))
        .collect();

    ordered(&mut entries);

    DirectoryListing::Listed {
        path: None,
        entries,
    }
}

/// The drive letters a machine answers to, as the roots they are: `C:\` rather
/// than `C:`, which is the difference between the top of a drive and whatever
/// directory that drive was last at.
///
/// Asked of the filesystem a letter at a time rather than of Win32: the call
/// that hands back the whole set is a dependency this crate does not otherwise
/// have, and twenty-six questions about a directory cost less than the one
/// `read_dir` whichever answer is picked is about to get. Compiled on the
/// platforms that have no drives as well, so the Linux runner tests it — which
/// is how the platform's own directories are tested too, and what varies is
/// `present` rather than anything this decides.
#[cfg(any(windows, test))]
fn drives(present: impl Fn(&Path) -> bool) -> Vec<PathBuf> {
    (b'A'..=b'Z')
        .map(|letter| PathBuf::from(format!("{}:\\", letter as char)))
        .filter(|drive| present(drive))
        .collect()
}

/// What one resolved directory holds.
///
/// Every refusal the filesystem can make here is a row rather than a failure: a
/// path naming a file is a browse that has gone as deep as it goes, and a
/// directory that will not open — permissions, or one that went between the ask
/// and the reading — is the filesystem's answer to draw rather than the
/// server's error to report.
fn entries_of(real: &Path) -> DirectoryListing {
    if !real.is_dir() {
        return DirectoryListing::NotADirectory;
    }

    let reading = match std::fs::read_dir(real) {
        Ok(reading) => reading,
        Err(error) => {
            return DirectoryListing::Unreadable {
                why: format!("The server can't read it: {error}."),
            };
        }
    };

    let mut entries: Vec<DirectoryEntry> = reading
        .filter_map(|read| {
            // An entry that will not read is left out rather than failing the
            // listing: what the human is looking for is almost certainly one of
            // the ones that did.
            let read = read.ok()?;

            // And so is one whose name is not UTF-8. It could be lossily
            // spelled, but what came back would be a path nothing is at — a row
            // that cannot be browsed into or submitted is worse than a row that
            // is not there.
            entry(read.file_name().to_str()?.to_owned(), read.path())
        })
        .collect();

    ordered(&mut entries);

    DirectoryListing::Listed {
        path: Some(real.display().to_string()),
        entries,
    }
}

/// One row, or nothing where the path is not one this can be put on the wire —
/// which is the same reading the name above gets, one level up.
fn entry(name: String, path: PathBuf) -> Option<DirectoryEntry> {
    Some(DirectoryEntry {
        kind: kind(&path),
        path: path.into_os_string().into_string().ok()?,
        name,
    })
}

/// Directories first, then by name.
///
/// A repository sorts as the directory it is: the mark says what it holds
/// rather than putting it somewhere else in the list, and a field looking for
/// one finds it where the eye is already going.
fn ordered(entries: &mut [DirectoryEntry]) {
    entries.sort_by(|left, right| {
        let below = |entry: &DirectoryEntry| entry.kind == EntryKind::File;

        below(left)
            .cmp(&below(right))
            .then_with(|| left.name.cmp(&right.name))
    });
}

/// What the field drawing a row does with it: drill in, mark it, or treat it as
/// a leaf.
///
/// Followed rather than read off the link: a symlink to a directory browses
/// into that directory, which is what the filesystem itself would do — and what
/// the save behind the field does with one, since it resolves the path too.
/// Whatever cannot be followed is a leaf, and the field pointed at it will be
/// told so when it asks for the listing.
fn kind(path: &Path) -> EntryKind {
    if !path.is_dir() {
        return EntryKind::File;
    }

    // A `.git` of either shape: a directory in a clone, and a file in a
    // worktree — both of which are repositories to register.
    match path.join(".git").exists() {
        true => EntryKind::Repository,
        false => EntryKind::Directory,
    }
}

/// A drive's own name: its last segment, or the whole of it where it has no
/// last segment to take, which is what the top of a drive is.
///
/// Only the drives are named this way — every other row is read out of the
/// directory holding it, which is where a name already is.
#[cfg(windows)]
fn root_name(root: &Path) -> Option<String> {
    match root.file_name() {
        Some(name) => name.to_str().map(str::to_owned),
        None => root.to_str().map(str::to_owned),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The entries of a listing, or a panic saying what came back instead.
    fn listed(listing: DirectoryListing) -> Vec<DirectoryEntry> {
        match listing {
            DirectoryListing::Listed { entries, .. } => entries,
            other => panic!("expected a listing, got {other:?}"),
        }
    }

    /// The rows' names, which is what a dropdown draws.
    fn names(listing: DirectoryListing) -> Vec<String> {
        listed(listing).into_iter().map(|row| row.name).collect()
    }

    /// A directory holding a `.git`, which is what a clone looks like from
    /// outside it. Made rather than cloned: what this reads is the presence of
    /// the name, and git is not asked anything.
    fn repository(at: &Path) {
        std::fs::create_dir_all(at.join(".git")).unwrap();
    }

    #[test]
    fn a_directory_lists_what_is_in_it_with_each_entry_saying_what_it_is() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("README.md"), "# a directory\n").unwrap();
        repository(&dir.path().join("verkstead"));

        let listing = list(Some(dir.path().to_owned()));

        assert_eq!(
            listed(listing)
                .into_iter()
                .map(|row| (row.name, row.kind))
                .collect::<Vec<_>>(),
            vec![
                ("src".to_owned(), EntryKind::Directory),
                ("verkstead".to_owned(), EntryKind::Repository),
                ("README.md".to_owned(), EntryKind::File),
            ]
        );
    }

    /// Directories first and then by name, whatever order the filesystem hands
    /// them back in.
    #[test]
    fn directories_come_before_files_and_each_half_is_by_name() {
        let dir = tempfile::tempdir().unwrap();
        for made in ["zebra", "alpaca"] {
            std::fs::create_dir(dir.path().join(made)).unwrap();
        }
        for written in ["zebra.md", "alpaca.md"] {
            std::fs::write(dir.path().join(written), "\n").unwrap();
        }

        let listing = list(Some(dir.path().to_owned()));

        assert_eq!(names(listing), ["alpaca", "zebra", "alpaca.md", "zebra.md"]);
    }

    /// The client decides what to draw; this decides nothing.
    #[test]
    fn dotfiles_are_listed() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".claude")).unwrap();
        std::fs::write(dir.path().join(".claude.json"), "{}\n").unwrap();

        let listing = list(Some(dir.path().to_owned()));

        assert_eq!(names(listing), [".claude", ".claude.json"]);
    }

    /// Nothing bounds a browse: a directory nobody told Verkstead about lists
    /// like any other, which is what an open boundary means from a dropdown.
    #[test]
    fn a_directory_nothing_was_ever_said_about_lists() {
        let elsewhere = tempfile::tempdir().unwrap();
        std::fs::create_dir(elsewhere.path().join("src")).unwrap();

        assert_eq!(names(list(Some(elsewhere.path().to_owned()))), ["src"]);
    }

    /// Where a browse with nothing typed opens: the server's own home, listed
    /// like any other directory — the path resolved, and the entries its own.
    #[test]
    fn no_path_opens_on_the_servers_home() {
        let home = tempfile::tempdir().unwrap();
        std::fs::create_dir(home.path().join("src")).unwrap();

        let DirectoryListing::Listed { path, entries } = opening(Some(home.path().to_owned()))
        else {
            panic!("a home that is there lists");
        };

        assert_eq!(
            path.as_deref(),
            home.path().canonicalize().unwrap().to_str()
        );
        assert_eq!(
            entries.into_iter().map(|row| row.name).collect::<Vec<_>>(),
            ["src"]
        );
    }

    /// And a home there is no listing to be had of falls back to the top of the
    /// machine — one answer for the four ways that happens, none of them
    /// something the human could correct from a dropdown.
    ///
    /// The relative one is there because the home is resolved the way a typed
    /// path is: it is refused rather than read from wherever the server was
    /// started.
    #[test]
    fn a_home_that_cannot_be_read_opens_on_the_topmost_listing_instead() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("notes.md");
        std::fs::write(&file, "# notes\n").unwrap();

        for home in [
            None,
            Some(dir.path().join("never-made")),
            Some(file),
            Some(PathBuf::from("src")),
        ] {
            assert_eq!(opening(home), topmost());
        }
    }

    /// The topmost listing on a Unix is `/`, which is where a browse with
    /// nowhere else to start begins. What it is on a machine with drives instead
    /// is the case below, which needs none.
    #[cfg(unix)]
    #[test]
    fn the_topmost_listing_is_the_filesystem_root() {
        let DirectoryListing::Listed { path, entries } = topmost() else {
            panic!("the root lists");
        };

        assert_eq!(path.as_deref(), Some("/"));
        assert!(!entries.is_empty(), "there is something in /");
    }

    /// Every letter is asked about and the ones that answer are the roots, each
    /// spelled as the top of its drive rather than as the drive.
    ///
    /// Run wherever the suite runs, because what varies between the platforms
    /// is which letters answer rather than any of the reasoning: the machine
    /// stands in as the closure.
    #[test]
    fn the_drives_are_the_letters_something_is_mounted_on() {
        let mounted = |drive: &Path| matches!(drive.to_str(), Some("C:\\") | Some("Z:\\"));

        assert_eq!(
            drives(mounted)
                .into_iter()
                .map(|drive| drive.display().to_string())
                .collect::<Vec<_>>(),
            ["C:\\", "Z:\\"]
        );
    }

    /// And a machine with nothing mounted lists nothing, rather than offering a
    /// letter that is not there.
    #[test]
    fn a_machine_with_no_drives_has_no_roots() {
        assert!(drives(|_| false).is_empty());
    }

    /// A field halfway through a word, which is the ordinary state of one.
    #[test]
    fn a_path_with_nothing_at_it_is_missing() {
        let dir = tempfile::tempdir().unwrap();

        assert_eq!(
            list(Some(dir.path().join("never-made"))),
            DirectoryListing::Missing
        );
    }

    #[test]
    fn a_relative_path_is_refused_without_being_resolved() {
        assert_eq!(
            list(Some(PathBuf::from("src"))),
            DirectoryListing::NotAbsolute
        );
    }

    #[test]
    fn a_file_is_not_a_directory_rather_than_an_empty_listing() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("notes.md");
        std::fs::write(&file, "# notes\n").unwrap();

        assert_eq!(list(Some(file)), DirectoryListing::NotADirectory);
    }

    /// A directory that is there and will not open. Drawn as a row rather than
    /// reported as a failure — the same answer a directory that went between one
    /// ask and the next gets, which is the case this stands in for.
    #[test]
    #[cfg(unix)]
    fn a_directory_that_cannot_be_read_says_so_rather_than_failing() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let shut = dir.path().join("shut");
        std::fs::create_dir(&shut).unwrap();
        std::fs::set_permissions(&shut, std::fs::Permissions::from_mode(0o000)).unwrap();

        let listing = list(Some(shut.clone()));

        // Root reads it whatever the mode says, and CI runs as somebody. Both
        // answers are the endpoint behaving — what this refuses to be is a
        // failure.
        assert!(
            matches!(
                listing,
                DirectoryListing::Unreadable { .. } | DirectoryListing::Listed { .. }
            ),
            "an unreadable directory answers a listing or a refusal, got {listing:?}"
        );

        std::fs::set_permissions(&shut, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
}
