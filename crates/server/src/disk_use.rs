//! How much each language's store holds on disk, which the settings page draws
//! beside the size it is allowed.
//!
//! **Measured in the background and read from what was last measured.** A store
//! is many thousands of files — a year's Maven repository, a pnpm store — and a
//! walk of them is seconds of disk, which is not something a settings read can
//! be made to wait on. So the figures are held here, per language, and the read
//! takes whatever is held: the last figure, or nothing yet, which the page says
//! as *not measured yet* — see [`DiskUse::of`]. The measuring is a loop of its
//! own, the [`crate::cleanup`] shape: spawned at startup, never on a server that
//! runs no sessions, a pass every [`MEASURED_EVERY`] and another whenever
//! something that changes a store asks for one — see [`DiskUse::refresh`].
//!
//! **A store is the directories its descriptor names** — see
//! [`crate::languages::Descriptor::stores`] — and a language's figure is all of
//! them together, which is what its one size bounds.
//!
//! **The walk never follows a symlink.** pnpm's `projects/` holds links to the
//! Worktrees that installed out of it, bun keeps links into its own cache, and
//! uv's built wheels are links into its archive: following any of them would be
//! counting a project, or counting a file twice. And **a file with several
//! links is counted once** per language, because pnpm, deno, bun and uv link
//! out of their stores where they can and a link inside the store is one file
//! on the disk. Where the platform does not say which file a link is — Windows,
//! through the standard library — each is counted; a store there links out to a
//! project rather than within itself, so what that costs is little.
//!
//! **The walk is the sweep's too**, which wants the same files with their times
//! and stops descending at a unit — see [`walk`], whose visitor is handed each
//! entry's metadata and says whether to go into a directory.

use std::collections::{HashMap, HashSet};
use std::fs::Metadata;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::AppState;

/// How often every store is measured again where nothing has asked sooner.
///
/// A quarter of an hour: what fills a store is a session installing, which is
/// minutes of work at a time, and a figure that lagged a session by a quarter of
/// an hour is still the right order of magnitude for a page asking how close a
/// store is to its size. Slower would be a page that looked stuck; faster is a
/// disk walked for an answer nobody is reading.
pub(crate) const MEASURED_EVERY: Duration = Duration::from_secs(15 * 60);

/// One entry the walk reached: where it is, how deep under the directory the
/// walk started at, and its own metadata — a symlink's, never what it points
/// at.
pub(crate) struct Entry<'a> {
    pub path: &'a Path,

    /// One for what is directly in the directory walked, and so on down.
    pub depth: usize,

    pub metadata: &'a Metadata,
}

/// What the visitor says of a directory it was handed: go into it, or leave it
/// whole — which is how the sweep stops at a unit. Said of anything else, it
/// means nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Walking {
    Into,
    Past,
}

/// Every entry under `dir`, depth first, handed to `visit` — the directory
/// itself not among them.
///
/// **No symlink is followed**: each entry's metadata is the link's own, so a
/// link is an entry that is neither a file nor a directory and is never gone
/// into. A `dir` that is not there is nothing to walk rather than an error —
/// a store nothing has installed into yet is an empty one. What cannot be read
/// further down — a directory removed under the walk, one without the
/// permission to list it — is passed over, said in the log, and the walk goes
/// on: a figure missing one directory is a better answer than none.
pub(crate) fn walk(dir: &Path, visit: &mut impl FnMut(&Entry) -> Walking) -> io::Result<()> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };

    let mut pending: Vec<(std::fs::ReadDir, usize)> = vec![(entries, 1)];

    while let Some((entries, depth)) = pending.last_mut() {
        let depth = *depth;

        let Some(entry) = entries.next() else {
            pending.pop();
            continue;
        };

        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                tracing::debug!(error = ?error, dir = %dir.display(), "an entry of a store could not be read, so it was passed over");
                continue;
            }
        };

        let path = entry.path();

        let metadata = match std::fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) => {
                tracing::debug!(error = ?error, path = %path.display(), "an entry of a store went before it could be read, so it was passed over");
                continue;
            }
        };

        let walking = visit(&Entry {
            path: &path,
            depth,
            metadata: &metadata,
        });

        if metadata.is_dir() && walking == Walking::Into {
            match std::fs::read_dir(&path) {
                Ok(entries) => pending.push((entries, depth + 1)),
                Err(error) => {
                    tracing::debug!(error = ?error, path = %path.display(), "a directory of a store could not be listed, so it was passed over")
                }
            }
        }
    }

    Ok(())
}

/// How many bytes the files under `dirs` hold together: every regular file's
/// length, a file with several links counted once however many of them are
/// here, and nothing a symlink points at.
///
/// A directory inside another of `dirs` is walked once, as part of the outer
/// one, so two stores an installer named one inside the other are not one
/// store's files counted twice.
pub(crate) fn measure(dirs: &[PathBuf]) -> u64 {
    let mut linked = Linked::default();
    let mut bytes = 0;

    for dir in dirs {
        if dirs
            .iter()
            .any(|outer| outer != dir && dir.starts_with(outer))
        {
            continue;
        }

        let walked = walk(dir, &mut |entry| {
            if entry.metadata.is_file() && linked.first(entry.metadata) {
                bytes += entry.metadata.len();
            }

            Walking::Into
        });

        if let Err(error) = walked {
            tracing::warn!(error = ?error, dir = %dir.display(), "a store could not be measured, so its disk use leaves it out");
        }
    }

    bytes
}

/// The files already counted that have more than one link, by the file they
/// are rather than the name they were reached by.
///
/// Only those, because they are the only ones a second name can reach: a set of
/// every file in a store of a million would be tens of megabytes held for a
/// question nine in ten of them cannot raise.
#[derive(Default)]
struct Linked(HashSet<(u64, u64)>);

impl Linked {
    /// Whether this is the first time the file behind `metadata` was counted.
    #[cfg(unix)]
    fn first(&mut self, metadata: &Metadata) -> bool {
        use std::os::unix::fs::MetadataExt;

        metadata.nlink() < 2 || self.0.insert((metadata.dev(), metadata.ino()))
    }

    /// Every one, on a platform whose standard library does not say which
    /// file a link is — see this module's header.
    #[cfg(not(unix))]
    fn first(&mut self, _metadata: &Metadata) -> bool {
        let _ = &self.0;
        true
    }
}

/// The figures, held: each language's bytes on disk as last measured, and a
/// way to ask for them to be measured again.
///
/// Cloned into the state like every other register there, the clones sharing
/// one set of figures.
#[derive(Debug, Clone, Default)]
pub(crate) struct DiskUse {
    figures: Arc<Mutex<HashMap<String, u64>>>,
    again: Arc<tokio::sync::Notify>,
}

impl DiskUse {
    /// What `language`'s stores held when last measured, and `None` where they
    /// have not been measured since the server started.
    ///
    /// **Never waits on a measurement.** The lock is held by a measurement only
    /// for the moment it writes a figure it already has — see
    /// [`DiskUse::measuring_with`] — so a read in the middle of a walk is
    /// answered at once, with the figure from before it.
    pub(crate) fn of(&self, language: &str) -> Option<u64> {
        self.held().get(language).copied()
    }

    /// Ask for every store to be measured again as soon as the one in hand is
    /// done — which is what anything that has just changed a store says, so the
    /// page does not go on drawing what it held before.
    #[allow(
        dead_code,
        reason = "the sweep and Clear call it, neither of which is written yet"
    )]
    pub(crate) fn refresh(&self) {
        self.again.notify_one();
    }

    /// One pass: each of `stores`' languages measured by `measuring` in turn,
    /// and its figure written the moment it is had — so the first language's
    /// is on the page while the last is still being walked. A language that is
    /// no longer in `stores` has its figure taken away.
    ///
    /// `measuring` is [`measure`] everywhere but the suite, which holds one up
    /// to prove a read does not wait on it.
    pub(crate) fn measuring_with(
        &self,
        stores: &[(String, Vec<PathBuf>)],
        measuring: impl Fn(&[PathBuf]) -> u64,
    ) {
        self.held()
            .retain(|language, _| stores.iter().any(|(name, _)| name == language));

        for (language, dirs) in stores {
            let bytes = measuring(dirs);

            self.held().insert(language.clone(), bytes);
        }
    }

    fn held(&self) -> std::sync::MutexGuard<'_, HashMap<String, u64>> {
        self.figures.lock().unwrap_or_else(|held| held.into_inner())
    }
}

/// Measure every language's stores now and on [`MEASURED_EVERY`] after, or
/// sooner when [`DiskUse::refresh`] asks — on a server that runs sessions and
/// has a Build Cache, which is a server that has stores at all.
///
/// The descriptors are read off `config.yaml` on every pass, the way the
/// Cleanup reads its settings, so a store an installer named this morning is
/// measured on the next one. Every language's, switched on or off: a language
/// switched off still holds what it downloaded, and the page is where that is
/// seen.
pub(crate) fn measuring(state: &AppState) {
    let Some(machine) = state
        .sessions
        .build_cache()
        .and_then(|cache| cache.machine())
    else {
        return;
    };

    let state = state.clone();

    tokio::spawn(async move {
        loop {
            let languages = crate::languages::configured(&state.settings.config());

            let stores: Vec<(String, Vec<PathBuf>)> = languages
                .iter()
                .filter(|(_, descriptor)| descriptor.names_a_store())
                .map(|(name, descriptor)| {
                    (
                        name.to_owned(),
                        descriptor
                            .stores(&machine)
                            .into_iter()
                            .map(|(_, dir)| dir)
                            .collect(),
                    )
                })
                .collect();

            let disk_use = state.disk_use.clone();

            if let Err(error) =
                tokio::task::spawn_blocking(move || disk_use.measuring_with(&stores, measure)).await
            {
                tracing::error!(error = ?error, "measuring the languages' stores failed");
            }

            tokio::select! {
                () = tokio::time::sleep(MEASURED_EVERY) => {}
                () = state.disk_use.again.notified() => {}
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::mpsc;

    fn write(path: &Path, bytes: usize) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, vec![b'x'; bytes]).unwrap();
    }

    /// Every file's length, under however many directories.
    #[test]
    fn a_store_is_the_bytes_of_every_file_under_it() {
        let dir = tempfile::tempdir().unwrap();

        write(&dir.path().join("a/one"), 100);
        write(&dir.path().join("a/b/two"), 20);
        write(&dir.path().join("c/three"), 3);

        assert_eq!(measure(&[dir.path().join("a"), dir.path().join("c")]), 123);
    }

    /// A store nothing has installed into yet is an empty one.
    #[test]
    fn a_store_that_is_not_there_yet_is_nothing() {
        let dir = tempfile::tempdir().unwrap();

        assert_eq!(measure(&[dir.path().join("never")]), 0);
    }

    /// A symlink to a large directory outside the store adds nothing, and nor
    /// does one to a file.
    #[cfg(unix)]
    #[test]
    fn a_symlink_out_of_the_store_adds_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let store = dir.path().join("store");

        write(&store.join("own"), 10);
        write(&dir.path().join("project/huge"), 10_000);

        std::os::unix::fs::symlink(dir.path().join("project"), store.join("linked")).unwrap();
        std::os::unix::fs::symlink(dir.path().join("project/huge"), store.join("file")).unwrap();

        assert_eq!(measure(&[store]), 10);
    }

    /// A file with two names in the store is one file on the disk.
    #[cfg(unix)]
    #[test]
    fn a_hardlinked_file_is_counted_once() {
        let dir = tempfile::tempdir().unwrap();
        let store = dir.path().join("store");

        write(&store.join("files/blob"), 1000);
        std::fs::create_dir_all(store.join("index")).unwrap();
        std::fs::hard_link(store.join("files/blob"), store.join("index/blob")).unwrap();

        // And one linked out into a project still counts in the store.
        write(&store.join("files/other"), 7);
        std::fs::hard_link(store.join("files/other"), dir.path().join("out")).unwrap();

        assert_eq!(measure(&[store]), 1007);
    }

    /// Two stores, one inside the other, are the outer one's files once.
    #[test]
    fn a_store_inside_another_is_not_counted_twice() {
        let dir = tempfile::tempdir().unwrap();

        write(&dir.path().join("yarn/cache/zip"), 50);

        assert_eq!(
            measure(&[dir.path().join("yarn/cache"), dir.path().join("yarn")]),
            50,
        );
    }

    /// The walk hands each entry its depth and stops going down where it is
    /// told to — the shape the sweep needs of it.
    #[test]
    fn the_walk_stops_where_it_is_told_and_says_how_deep() {
        let dir = tempfile::tempdir().unwrap();

        write(&dir.path().join("a/b/c"), 1);
        write(&dir.path().join("d/e"), 1);

        let mut seen = Vec::new();

        walk(dir.path(), &mut |entry| {
            let name = entry
                .path
                .strip_prefix(dir.path())
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");

            seen.push((name.clone(), entry.depth));

            match name.as_str() {
                "a" => Walking::Past,
                _ => Walking::Into,
            }
        })
        .unwrap();

        seen.sort();

        assert_eq!(
            seen,
            [
                (String::from("a"), 1),
                (String::from("d"), 1),
                (String::from("d/e"), 2),
            ],
        );
    }

    /// A read in the middle of a measurement is answered at once: with nothing
    /// before the first figure, and with the last one after it.
    #[test]
    fn a_read_never_waits_on_a_measurement() {
        let disk_use = DiskUse::default();
        let stores = vec![(String::from("go"), vec![PathBuf::from("/nowhere")])];

        let (started, starting) = mpsc::channel();
        let (release, released) = mpsc::channel::<u64>();
        let released = Mutex::new(released);

        std::thread::scope(|scope| {
            let measuring = scope.spawn(|| {
                disk_use.measuring_with(&stores, |_| {
                    started.send(()).unwrap();
                    released.lock().unwrap().recv().unwrap()
                });
            });

            starting.recv().unwrap();

            assert_eq!(
                disk_use.of("go"),
                None,
                "a measurement in flight and none before it is not measured yet",
            );

            release.send(4096).unwrap();
            measuring.join().unwrap();

            assert_eq!(disk_use.of("go"), Some(4096));

            let measuring = scope.spawn(|| {
                disk_use.measuring_with(&stores, |_| {
                    started.send(()).unwrap();
                    released.lock().unwrap().recv().unwrap()
                });
            });

            starting.recv().unwrap();

            assert_eq!(
                disk_use.of("go"),
                Some(4096),
                "and one in flight after it is the last figure",
            );

            release.send(8192).unwrap();
            measuring.join().unwrap();
        });

        assert_eq!(disk_use.of("go"), Some(8192));
    }

    /// A language no longer loaded has no figure left over.
    #[test]
    fn a_language_gone_from_the_descriptors_takes_its_figure_with_it() {
        let disk_use = DiskUse::default();

        disk_use.measuring_with(&[(String::from("go"), Vec::new())], |_| 1);
        disk_use.measuring_with(&[(String::from("node"), Vec::new())], |_| 2);

        assert_eq!(disk_use.of("go"), None);
        assert_eq!(disk_use.of("node"), Some(2));
    }
}
