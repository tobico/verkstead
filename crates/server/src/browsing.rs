//! Reading one directory for a path field's dropdown: what is in it, and what
//! each of those things is.
//!
//! One directory per ask and no walking: a field browses by asking again for
//! each level somebody drills into, so nothing here ever recurses and a
//! directory holding ten thousand files costs one `read_dir`.
//!
//! The two scopes are two different questions about where the ask may look, and
//! only one of them consults the boundary. A [`BrowseScope::Watched`] ask is
//! decided by [`crate::watched`] — the same admission the save behind that field
//! is going to make, on the resolved path, so the dropdown cannot offer what the
//! save would turn down. A [`BrowseScope::Anywhere`] ask is bounded by nothing
//! but what the server can read, which is a wider disclosure than anything else
//! here makes and was settled as one: the fields it serves take paths the
//! boundary says nothing about, and a dropdown that could not reach them would
//! be a dropdown nobody could use.
//!
//! Nothing here refuses by status code. A path that is relative, missing, not a
//! directory, outside the boundary or unreadable is a named outcome the dropdown
//! draws where its rows would be — see [`DirectoryListing`]. A field is typed
//! into a character at a time, so most of those are the ordinary state of a
//! field halfway through a word rather than anything that went wrong.

use std::path::{Path, PathBuf};

use verkstead_render::{BrowseScope, DirectoryEntry, DirectoryListing, EntryKind};

use crate::watched::{Admission, Boundary, WatchedPaths};

/// What `path` holds, asked in `scope` — or the named reason it holds nothing
/// this ask may have.
///
/// No path at all is the field standing empty, and the two scopes answer it
/// differently: the watched scope hands back the Watched Paths themselves,
/// which is where a browse bounded by them begins, and the anywhere scope hands
/// back the top of the machine, which is where a browse bounded by nothing
/// does — and which is the one thing here the platforms disagree about, so see
/// [`topmost`] for what each of them calls it.
///
/// Blocking: the directory is opened, and — in the scope bounded by them — so
/// is the file the Watched Paths are half said in.
pub(crate) fn list(
    watched: &WatchedPaths,
    scope: BrowseScope,
    path: Option<PathBuf>,
) -> DirectoryListing {
    match (scope, path) {
        // The boundary as a set of directories rather than as a decision about
        // one, which is the only thing an ask with nothing typed in the field
        // could be about.
        (BrowseScope::Watched, None) => roots(&watched.standing()),

        (BrowseScope::Watched, Some(path)) => match watched.admit(&path) {
            Admission::Inside(real) => entries_of(&real),
            Admission::NotAbsolute => DirectoryListing::NotAbsolute,
            Admission::Missing => DirectoryListing::Missing,
            Admission::Outside => DirectoryListing::OutsideWatchedPaths,
        },

        // Rooted at the top of the machine, and resolved here rather than by
        // the boundary: this is the scope no boundary is consulted for, and the
        // two questions the admission would have answered on the way — absolute,
        // and there — are the same two questions asked of any path.
        (BrowseScope::Anywhere, None) => topmost(),

        (BrowseScope::Anywhere, Some(path)) => {
            if !path.is_absolute() {
                return DirectoryListing::NotAbsolute;
            }

            match path.canonicalize() {
                Ok(real) => entries_of(&real),
                Err(_) => DirectoryListing::Missing,
            }
        }
    }
}

/// The Watched Paths themselves, as a listing with no directory above it.
///
/// Both halves of the boundary, in one list and deduplicated: what a field
/// bounded by the Watched Paths may browse is their union, and a root said on
/// the command line *and* in the settings is one directory rather than a row
/// twice. Which of the two said it is the settings page's question rather than
/// this one's — see [`crate::paths`], which is where that distinction is drawn.
fn roots(boundary: &Boundary<'_>) -> DirectoryListing {
    let mut entries: Vec<DirectoryEntry> = boundary
        .roots()
        .into_iter()
        .filter_map(|root| entry(root_name(&root)?, root))
        .collect();

    // Deduplicated on the whole path, which is the resolved one: two spellings
    // of one directory are one row.
    entries.sort_by(|left, right| left.path.cmp(&right.path));
    entries.dedup_by(|left, right| left.path == right.path);

    ordered(&mut entries);

    DirectoryListing::Listed {
        path: None,
        entries,
    }
}

/// Where a browse bounded by nothing begins, which is the one thing about this
/// scope the platforms disagree on.
///
/// A Unix has one filesystem root and the browse opens on what `/` holds. A
/// Windows machine has one root per drive and nothing above them — `/` is not
/// even a path [`Path::is_absolute`] accepts there, having a root but no prefix
/// — so the browse opens on the drives themselves, as a listing with no
/// directory above it. Which is the shape [`roots`] hands the other scope back:
/// the two scopes begin in different places, and both of them begin somewhere.
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
                why: format!("the server cannot read it: {error}"),
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
/// the boundary does with one, since it decides on the resolved path. Whatever
/// cannot be followed is a leaf, and the field pointed at it will be told so
/// when it asks for the listing.
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

/// A Watched Path's own name: its last segment, or the whole of it where it has
/// no last segment to take.
///
/// `/` is the one path that has none, and a machine watching the whole
/// filesystem is a legal thing to have configured.
fn root_name(root: &Path) -> Option<String> {
    match root.file_name() {
        Some(name) => name.to_str().map(str::to_owned),
        None => root.to_str().map(str::to_owned),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::settings::Settings;

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

    fn watching(dir: &Path) -> WatchedPaths {
        WatchedPaths::resolve(&[dir.to_owned()]).unwrap()
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

        let listing = list(
            &watching(dir.path()),
            BrowseScope::Watched,
            Some(dir.path().to_owned()),
        );

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

        let listing = list(
            &watching(dir.path()),
            BrowseScope::Anywhere,
            Some(dir.path().to_owned()),
        );

        assert_eq!(names(listing), ["alpaca", "zebra", "alpaca.md", "zebra.md"]);
    }

    /// The client decides what to draw; this decides nothing.
    #[test]
    fn dotfiles_are_listed() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".claude")).unwrap();
        std::fs::write(dir.path().join(".claude.json"), "{}\n").unwrap();

        let listing = list(
            &watching(dir.path()),
            BrowseScope::Watched,
            Some(dir.path().to_owned()),
        );

        assert_eq!(names(listing), [".claude", ".claude.json"]);
    }

    #[test]
    fn the_watched_scope_with_no_path_answers_the_roots() {
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        let watched =
            WatchedPaths::resolve(&[first.path().to_owned(), second.path().to_owned()]).unwrap();

        let listing = list(&watched, BrowseScope::Watched, None);

        let DirectoryListing::Listed { path, entries } = listing else {
            panic!("the roots are a listing");
        };

        // No directory above them: the boundary is a set of directories rather
        // than a place.
        assert_eq!(path, None);
        assert_eq!(entries.len(), 2);
        assert!(
            entries
                .iter()
                .all(|row| row.kind == EntryKind::Directory && Path::new(&row.path).is_absolute())
        );
    }

    /// The union of the two halves, said once: a directory the command line and
    /// the settings both name is one root.
    #[test]
    fn the_roots_are_both_halves_of_the_boundary_deduplicated() {
        let data_dir = tempfile::tempdir().unwrap();
        let shared = tempfile::tempdir().unwrap();
        let said = tempfile::tempdir().unwrap();

        std::fs::write(
            data_dir.path().join("config.yaml"),
            format!(
                "watched_paths:\n  - {}\n  - {}\n",
                shared.path().display(),
                said.path().display()
            ),
        )
        .unwrap();

        let watched = watching(shared.path()).reading(Settings::in_data_dir(data_dir.path()));

        assert_eq!(listed(list(&watched, BrowseScope::Watched, None)).len(), 2);
    }

    /// The whole point of the scope: a path the save would refuse is a path the
    /// dropdown will not offer.
    #[test]
    fn a_path_outside_every_watched_root_is_refused_in_the_watched_scope() {
        let watched_dir = tempfile::tempdir().unwrap();
        let elsewhere = tempfile::tempdir().unwrap();

        assert_eq!(
            list(
                &watching(watched_dir.path()),
                BrowseScope::Watched,
                Some(elsewhere.path().to_owned()),
            ),
            DirectoryListing::OutsideWatchedPaths
        );
    }

    /// And the whole point of the other one: the same path, asked by a field the
    /// boundary says nothing about.
    #[test]
    fn the_same_path_lists_in_the_anywhere_scope() {
        let watched_dir = tempfile::tempdir().unwrap();
        let elsewhere = tempfile::tempdir().unwrap();
        std::fs::create_dir(elsewhere.path().join("src")).unwrap();

        assert_eq!(
            names(list(
                &watching(watched_dir.path()),
                BrowseScope::Anywhere,
                Some(elsewhere.path().to_owned()),
            )),
            ["src"]
        );
    }

    /// Watching nothing admits nothing, here as everywhere else.
    #[test]
    fn the_watched_scope_of_a_boundary_around_nothing_is_empty_and_refuses_everything() {
        let dir = tempfile::tempdir().unwrap();

        assert_eq!(
            list(&WatchedPaths::none(), BrowseScope::Watched, None),
            DirectoryListing::Listed {
                path: None,
                entries: Vec::new()
            }
        );
        assert_eq!(
            list(
                &WatchedPaths::none(),
                BrowseScope::Watched,
                Some(dir.path().to_owned())
            ),
            DirectoryListing::OutsideWatchedPaths
        );
    }

    /// The anywhere scope with nothing typed is `/`, which is where a browse
    /// bounded by nothing begins — on a machine that has a `/`, which is both
    /// Unixes and not Windows. What that scope opens on there is the drives,
    /// and the case below says so without needing one.
    #[cfg(unix)]
    #[test]
    fn the_anywhere_scope_with_no_path_reads_the_filesystem_root() {
        let listing = list(&WatchedPaths::none(), BrowseScope::Anywhere, None);

        let DirectoryListing::Listed { path, entries } = listing else {
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
    fn a_path_with_nothing_at_it_is_missing_in_either_scope() {
        let dir = tempfile::tempdir().unwrap();
        let never_made = dir.path().join("never-made");

        for scope in [BrowseScope::Watched, BrowseScope::Anywhere] {
            assert_eq!(
                list(&watching(dir.path()), scope, Some(never_made.clone())),
                DirectoryListing::Missing
            );
        }
    }

    #[test]
    fn a_relative_path_is_refused_without_being_resolved() {
        let dir = tempfile::tempdir().unwrap();

        for scope in [BrowseScope::Watched, BrowseScope::Anywhere] {
            assert_eq!(
                list(&watching(dir.path()), scope, Some(PathBuf::from("src"))),
                DirectoryListing::NotAbsolute
            );
        }
    }

    #[test]
    fn a_file_is_not_a_directory_rather_than_an_empty_listing() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("notes.md");
        std::fs::write(&file, "# notes\n").unwrap();

        assert_eq!(
            list(&watching(dir.path()), BrowseScope::Watched, Some(file)),
            DirectoryListing::NotADirectory
        );
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

        let listing = list(
            &watching(dir.path()),
            BrowseScope::Watched,
            Some(shut.clone()),
        );

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
