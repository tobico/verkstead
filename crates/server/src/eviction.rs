//! The sweep that keeps every language's store under its size: whole units out,
//! oldest first, while nothing is running.
//!
//! **What it takes is a unit, never a file of one** — see
//! [`crate::languages::Unit`], and [`crate::units::listed`], which finds them.
//! A store whose descriptor says its tool evicts for itself (sccache) and one
//! naming no unit are never touched. A language's size bounds every store of it
//! the sweep takes units out of, together: Rust's cargo half is held to Rust's
//! size, and its sccache is held to the same size again by sccache.
//!
//! **What is held to the size is the units, and only the units.** What a store
//! holds beside them — an index, metadata, the parts of Gradle's home no unit
//! names — is nothing the sweep can take, so counting it against the size would
//! have a language whose unswept part alone was over it lose every unit on
//! every pass and still be over. The page shows it beside the size instead —
//! see [`held_in_units`].
//!
//! **Hourly, and only while nothing runs.** The pace is
//! [`crate::Pace::evicting`], and *nothing runs* is no session and no
//! Conversation Terminal: both are launched through
//! [`BuildCache::compiling`], whose hold is what counts them. A pass that comes
//! due while something runs waits for the first moment nothing does — see
//! [`BuildCache::idle`] — rather than for the next hour, so a machine that is
//! idle for ten minutes a day is swept every day. **A machine that is never
//! idle is never swept**, which was decided rather than overlooked: the pane
//! says when each store was last swept, so it is seen.
//!
//! **Never half a unit.** A unit is renamed aside, into a directory of
//! Verkstead's own at the top of the Build Cache or of the directory beside the
//! Worktrees — see [`crate::languages::Machine::aside`] — and only then deleted,
//! so a tool reading the store sees a package or no package. The rename is
//! under the lock a launch is counted under — see [`BuildCache::moving`] — so a
//! session or terminal starting meanwhile waits for it, and the sweep stops
//! between units the moment anything has started. A rename across filesystems
//! (a store an installer mounted separately) is refused by the system rather
//! than turned into a copy: that unit is passed over and said in the log.
//! Whatever a pass that died left aside is deleted at the start of the next.
//!
//! **Read-only units are swept.** Go's module cache is directories of mode 0555
//! holding files of 0444, and cargo's git packs are 0444; a directory needs its
//! own write bit to be moved to another parent, and its contents need theirs to
//! be deleted, so both are given before. **No symlink is followed**, here or in
//! the walks that find and measure a unit.
//!
//! **Oldest is the newest time anywhere in the unit.** A Maven jar's own
//! modification time is the Last-Modified the server sent rather than when it
//! was downloaded, so a fresh download of an old artifact would look old by its
//! jar alone; the directory it landed in, and the files beside it, say when.
//! Each file is read by its access time where the store's filesystem keeps one
//! as well as its modification time, and by its modification time alone where
//! it does not — `noatime` — which turns *least recently used* into *least
//! recently written*. A directory is read by its modification time only, on
//! every filesystem: listing one is an access, and the sweep and the disk use
//! list every directory of every store, so a directory's access time says only
//! when Verkstead last walked it. Whether a filesystem keeps access times is
//! measured rather than read off its mount options — see
//! [`keeps_access_times`].
//!
//! **And Clear, which is the same machinery pressed by a human** — see
//! [`emptied`]: every store of one language emptied, sccache's included, and
//! **refused while anything runs** rather than waiting for it, because a human
//! pressing a button is there to be told why it did nothing.

use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use time::OffsetDateTime;

use crate::AppState;
use crate::build_cache::BuildCache;
use crate::disk_use::{Walking, walk};
use crate::languages::{self, Bounded, Descriptor, Languages};

/// How often the stores are swept, as [`crate::Pace`] has it by default.
///
/// An hour. What fills a store is sessions installing, which is minutes of
/// work at a time, so a store an hour over its size is a small fraction over
/// it; and a pass that finds every store under its size is a walk of each and
/// nothing more.
pub(crate) const SWEPT_EVERY: Duration = Duration::from_secs(60 * 60);

/// What sweeping one language did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Swept {
    /// What the units of its swept stores held before, in bytes — each file
    /// once per unit, so a little over what a fresh measurement would say where
    /// a file was linked twice inside one. **Only the units**: what is beside
    /// them in a store, its indexes and its metadata and whatever its
    /// descriptor names no unit for, is nothing the sweep can take, so it is
    /// nothing the sweep holds to the size — see [`swept`].
    pub held: u64,

    /// And what they hold after, as the units taken out add up.
    pub left: u64,

    /// The units taken out, oldest first, where they were.
    pub removed: Vec<PathBuf>,

    /// Whether it stopped before it was done because something started.
    pub stopped: bool,
}

/// Bring the units of the stores `descriptor` sweeps by unit under `size` bytes
/// together, taking whole ones out oldest first — or nothing at all where they
/// are under it already, or where anything is running.
///
/// **The units are what is held to the size, and nothing else in the store.**
/// A store holds what no unit names as well — an index, metadata, and in
/// Gradle's home the distributions, the JDKs and the transforms — and none of
/// that is the sweep's to take. Counted against the size, a language whose
/// unswept part alone was over it would lose every unit on every pass and
/// still be over, which is a store cleared whole each hour by another route.
/// So it is measured on the page beside the size rather than held to it.
///
/// **Blocks**, on a walk of every unit in every store; the loop hands it to a
/// blocking thread.
///
/// Public for the proofs' sake: `tests/package_stores.rs` sweeps a store a real
/// tool filled, and then has the tool install out of what is left.
pub fn swept(cache: &BuildCache, descriptor: &Descriptor, size: u64) -> Swept {
    let Some(machine) = cache.machine() else {
        return Swept::default();
    };

    let mut found: Vec<Found> = Vec::new();

    for (dir, units) in by_unit(descriptor, &machine) {
        let Some(aside) = machine.aside(&dir) else {
            continue;
        };

        for unit in crate::units::listed(&dir, units) {
            if let Some(survey) = surveyed(&unit) {
                found.push(Found {
                    path: unit,
                    aside: aside.clone(),
                    survey,
                });
            }
        }
    }

    let held = found.iter().map(|unit| unit.survey.bytes).sum();

    let mut swept = Swept {
        held,
        left: held,
        ..Swept::default()
    };

    if held <= size {
        return swept;
    }

    // Where something is already running there is nothing to take, so the
    // probes below are not worth making.
    if cache.holding() > 0 {
        swept.stopped = true;

        return swept;
    }

    // Asked once per filesystem a unit is moved aside on, which is once per
    // placeholder's directory — and only now, with something to take.
    let mut keeping: HashMap<PathBuf, bool> = HashMap::new();

    let mut dated: Vec<(SystemTime, Found)> = found
        .into_iter()
        .map(|unit| {
            let atime = *keeping
                .entry(unit.aside.clone())
                .or_insert_with(|| keeps_access_times(&unit.aside));

            (unit.survey.newest(atime), unit)
        })
        .collect();

    dated.sort_by(|(one, one_unit), (other, other_unit)| {
        one.cmp(other).then(one_unit.path.cmp(&other_unit.path))
    });

    for (_, unit) in dated {
        if swept.left <= size {
            break;
        }

        let Some(moved) = cache.moving(|| moved_aside(&unit.path, &unit.aside)) else {
            tracing::info!(
                unit = %unit.path.display(),
                "something started while a store was being swept, so the sweep stops and goes on \
                 at the next moment nothing runs",
            );

            swept.stopped = true;

            break;
        };

        let Some(at) = moved else {
            continue;
        };

        // Out of the store, so out of reach of anything that runs from here:
        // what is left is Verkstead's own to take its time over, and a launch
        // no longer waits on it.
        if let Err(error) = deleted(&at) {
            tracing::warn!(
                error = ?error,
                aside = %at.display(),
                "a unit swept out of a store could not be deleted, so the next sweep deletes it",
            );
        }

        swept.left = swept.left.saturating_sub(unit.survey.bytes);
        swept.removed.push(unit.path);
    }

    swept
}

/// How many bytes the units of the stores `descriptor` sweeps by unit hold on
/// `machine` — what [`swept`] holds to the language's size — or `None` for a
/// language with no such store, which nothing holds to it.
///
/// What the settings page draws beside the store's whole disk use, so the part
/// the size bounds and the part it does not are both seen. **Blocks**, on the
/// same walk of every unit the sweep makes.
pub(crate) fn held_in_units(descriptor: &Descriptor, machine: &languages::Machine) -> Option<u64> {
    let stores = by_unit(descriptor, machine);

    if stores.is_empty() {
        return None;
    }

    Some(
        stores
            .iter()
            .flat_map(|(dir, units)| crate::units::listed(dir, units))
            .filter_map(|unit| surveyed(&unit))
            .map(|survey| survey.bytes)
            .sum(),
    )
}

/// The stores `descriptor` sweeps by unit on `machine`, each with the rules its
/// units are found by — not the one its tool evicts, nor one naming no unit.
fn by_unit<'a>(
    descriptor: &'a Descriptor,
    machine: &languages::Machine,
) -> Vec<(PathBuf, &'a [languages::Unit])> {
    descriptor
        .stores(machine)
        .into_iter()
        .filter_map(|(name, dir)| match descriptor.bounded(&name) {
            Bounded::ByUnit(units) => Some((dir, units)),
            Bounded::ByItsTool | Bounded::NotAtAll => None,
        })
        .collect()
}

/// **Clear**: empty every store `descriptor` names — swept by unit, evicted by
/// its tool or not bounded at all — and say how many things were taken out of
/// them, or `None` where anything is running, which a Clear is refused over.
///
/// The sweep's machinery, all of it at once: everything in a store is renamed
/// aside under [`BuildCache::clearing`], so a launch waits for the whole Clear
/// rather than finding a store half emptied, and is deleted after — read-only
/// directories given their write bits, and no link followed. The store
/// directories themselves stay, empty, so what a session is handed is still
/// there. A language naming the Compile Server has it stopped first, its store
/// being sccache's.
///
/// `others` is every other language's store: one inside this language's is
/// left where it is, and only what is beside it goes.
///
/// **Blocks**, on the deletes.
pub fn emptied(cache: &BuildCache, descriptor: &Descriptor, others: &[PathBuf]) -> Option<usize> {
    let Some(machine) = cache.machine() else {
        return Some(0);
    };

    let stores = descriptor.stores(&machine);

    // And where units are moved aside to, which is in nobody's store.
    let kept: Vec<PathBuf> = others.iter().cloned().chain(machine.asides()).collect();

    let moved = cache.clearing(descriptor.names(languages::SCCACHE), || {
        let mut moved = Vec::new();

        for (_, dir) in &stores {
            if let Some(aside) = machine.aside(dir) {
                emptying(dir, &aside, &kept, &mut moved);
            }
        }

        moved
    })?;

    for at in &moved {
        if let Err(error) = deleted(at) {
            tracing::warn!(
                error = ?error,
                aside = %at.display(),
                "what a Clear took out of a store could not be deleted, so the next sweep deletes it",
            );
        }
    }

    Some(moved.len())
}

/// Move everything in `dir` into `aside`, onto `moved` — except what `kept`
/// names, and what holds something it names, which is gone into instead.
fn emptying(dir: &Path, aside: &Path, kept: &[PathBuf], moved: &mut Vec<PathBuf>) {
    // A store nothing has written yet is empty already.
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();

        if kept.contains(&path) {
            continue;
        }

        let holds_kept = kept.iter().any(|kept| kept.starts_with(&path));

        if holds_kept && entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            emptying(&path, aside, kept, moved);

            continue;
        }

        moved.extend(moved_aside(&path, aside));
    }
}

/// One unit the sweep may take, with what it is judged and counted by.
struct Found {
    path: PathBuf,

    /// Where it is moved aside to, on the store's own filesystem.
    aside: PathBuf,

    survey: Survey,
}

/// What a walk of one unit found: the newest time anything in it was written,
/// and read, and the bytes of its files.
struct Survey {
    written: SystemTime,

    /// Files only — see this module's header for why a directory's access
    /// time says nothing.
    read: SystemTime,

    bytes: u64,
}

impl Survey {
    /// How new the unit is: its newest access time as well where `atime` says
    /// its filesystem keeps them, and its newest modification time alone where
    /// it does not.
    fn newest(&self, atime: bool) -> SystemTime {
        match atime {
            true => self.written.max(self.read),
            false => self.written,
        }
    }
}

/// The times and the bytes of the unit at `unit` — `None` for one gone before
/// it was read. A link inside is neither followed nor read.
fn surveyed(unit: &Path) -> Option<Survey> {
    let metadata = std::fs::symlink_metadata(unit).ok()?;

    let mut survey = Survey {
        written: SystemTime::UNIX_EPOCH,
        read: SystemTime::UNIX_EPOCH,
        bytes: 0,
    };

    let mut add = |metadata: &std::fs::Metadata| {
        if let Ok(modified) = metadata.modified() {
            survey.written = survey.written.max(modified);
        }

        if metadata.is_file() {
            if let Ok(accessed) = metadata.accessed() {
                survey.read = survey.read.max(accessed);
            }

            survey.bytes += metadata.len();
        }
    };

    add(&metadata);

    if metadata.is_dir() {
        let walked = walk(unit, &mut |entry| {
            if !entry.metadata.is_symlink() {
                add(entry.metadata);
            }

            Walking::Into
        });

        if let Err(error) = walked {
            tracing::debug!(error = ?error, unit = %unit.display(), "a unit could not be read, so it was passed over");

            return None;
        }
    }

    Some(survey)
}

/// Whether the filesystem `aside` is on keeps access times: a file of
/// Verkstead's own there, dated two days back, read, and looked at again.
///
/// **Measured rather than read off the mount**, because the answer is in
/// different places on each platform — `noatime` in Linux's mount options, an
/// `MNT_NOATIME` flag on a Mac, a registry value on Windows — and the thing
/// actually wanted is whether a read moves the time. Two days back because
/// Linux's default, `relatime`, moves it on a read only where it is older than
/// the modification time or a day old, so this is a read it would record.
///
/// False where the probe could not be made, which is the reading that is never
/// wrong in the dangerous direction: by modification time alone, a package used
/// every day but written a year ago looks old, and is fetched again.
pub(crate) fn keeps_access_times(aside: &Path) -> bool {
    let probe = aside.join("access-time-probe");

    let probed = (|| -> io::Result<bool> {
        std::fs::create_dir_all(aside)?;

        let long_ago = SystemTime::now() - Duration::from_secs(2 * 24 * 60 * 60);

        {
            let mut file = std::fs::File::create(&probe)?;
            file.write_all(b"probe")?;
            file.set_times(
                std::fs::FileTimes::new()
                    .set_accessed(long_ago)
                    .set_modified(long_ago),
            )?;
        }

        std::fs::File::open(&probe)?.read_exact(&mut [0; 1])?;

        let accessed = std::fs::metadata(&probe)?.accessed()?;

        Ok(accessed > long_ago + Duration::from_secs(24 * 60 * 60))
    })();

    let _ = std::fs::remove_file(&probe);

    match probed {
        Ok(keeps) => {
            tracing::debug!(aside = %aside.display(), keeps, "whether a store's filesystem keeps access times");

            keeps
        }
        Err(error) => {
            tracing::warn!(
                error = ?error,
                aside = %aside.display(),
                "whether a store's filesystem keeps access times could not be measured, so its \
                 units are judged by when they were written",
            );

            false
        }
    }
}

/// Rename `unit` into `aside`, and say where it went — or `None` where it is
/// still where it was, which is said in the log.
///
/// A unit directory without its own write bit is given it first, which a move
/// to another parent needs, and has its mode put back where the move fails.
fn moved_aside(unit: &Path, aside: &Path) -> Option<PathBuf> {
    /// A name nothing else in `aside` has: when, and how many before it.
    static MOVED: AtomicU64 = AtomicU64::new(0);

    if let Err(error) = std::fs::create_dir_all(aside) {
        tracing::warn!(error = ?error, aside = %aside.display(), "the directory a unit is moved aside to could not be made, so nothing is swept out of the store");

        return None;
    }

    let at = aside.join(format!(
        "{}-{}",
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos(),
        MOVED.fetch_add(1, Ordering::Relaxed),
    ));

    let was = writable_to_move(unit);

    match std::fs::rename(unit, &at) {
        Ok(()) => Some(at),
        Err(error) => {
            if let Some(was) = was {
                let _ = std::fs::set_permissions(unit, was);
            }

            match error.kind() {
                io::ErrorKind::CrossesDevices => tracing::warn!(
                    unit = %unit.display(),
                    "a unit is on a filesystem of its own, and a rename aside is the only way it is \
                     swept, so it is left where it is",
                ),
                _ => {
                    tracing::warn!(error = ?error, unit = %unit.display(), "a unit could not be moved aside, so it is left where it is")
                }
            }

            None
        }
    }
}

/// Give a directory at `unit` its owner's write bit where it has not got it,
/// and say what its permissions were — `None` where nothing was changed.
#[cfg(unix)]
fn writable_to_move(unit: &Path) -> Option<std::fs::Permissions> {
    use std::os::unix::fs::PermissionsExt;

    let metadata = std::fs::symlink_metadata(unit).ok()?;
    let was = metadata.permissions();

    if !metadata.is_dir() || was.mode() & 0o200 != 0 {
        return None;
    }

    std::fs::set_permissions(unit, std::fs::Permissions::from_mode(was.mode() | 0o200)).ok()?;

    Some(was)
}

/// Nothing on Windows, where a directory's read-only attribute does not stop
/// it being moved.
#[cfg(not(unix))]
fn writable_to_move(_unit: &Path) -> Option<std::fs::Permissions> {
    None
}

/// Delete `at` and everything under it, read-only or not, following no link.
fn deleted(at: &Path) -> io::Result<()> {
    let metadata = std::fs::symlink_metadata(at)?;

    writable(at, &metadata);

    if !metadata.is_dir() {
        return std::fs::remove_file(at);
    }

    // The walk hands each directory over before it lists it, so one without
    // the bits to be listed is given them first.
    walk(at, &mut |entry| {
        if !entry.metadata.is_symlink() {
            writable(entry.path, entry.metadata);
        }

        Walking::Into
    })?;

    std::fs::remove_dir_all(at)
}

/// Give what is at `path` the permissions its deleting needs: a directory its
/// owner's read, write and search, so what is in it can be listed and taken
/// out, and a file its write — which is what Windows asks before it deletes
/// one.
fn writable(path: &Path, metadata: &std::fs::Metadata) {
    let mut permissions = metadata.permissions();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let wanted = match metadata.is_dir() {
            true => permissions.mode() | 0o700,
            false => permissions.mode() | 0o200,
        };

        if wanted == permissions.mode() {
            return;
        }

        permissions.set_mode(wanted);
    }

    #[cfg(not(unix))]
    {
        if !permissions.readonly() {
            return;
        }

        #[allow(
            clippy::permissions_set_readonly_false,
            reason = "Windows has no other bits: this clears the read-only attribute"
        )]
        permissions.set_readonly(false);
    }

    if let Err(error) = std::fs::set_permissions(path, permissions) {
        tracing::debug!(error = ?error, path = %path.display(), "what is being deleted could not be made writable");
    }
}

/// Delete whatever a pass that died left in `aside`.
fn cleared(aside: &Path) {
    let Ok(entries) = std::fs::read_dir(aside) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();

        match deleted(&path) {
            Ok(()) => {
                tracing::info!(left = %path.display(), "a unit a sweep left aside has been deleted")
            }
            Err(error) => {
                tracing::warn!(error = ?error, left = %path.display(), "a unit a sweep left aside could not be deleted")
            }
        }
    }
}

/// What one pass over every language did.
#[derive(Debug, Default)]
struct Pass {
    /// The languages it swept to the end, which is what the pane's *last
    /// swept* is.
    swept: Vec<String>,

    /// Whether it took anything, which is what says the disk use is out of date.
    removed: bool,

    /// Whether it stopped because something started, which is a pass still due.
    stopped: bool,
}

/// One pass: what an earlier one left aside deleted, and then each language
/// with a store swept by unit brought under its size, in the order they were
/// written — stopping the moment something starts.
fn pass(cache: &BuildCache, languages: &Languages) -> Pass {
    let mut pass = Pass::default();

    let Some(machine) = cache.machine() else {
        return pass;
    };

    for aside in machine.asides() {
        cleared(&aside);
    }

    for (name, descriptor) in languages.iter() {
        let sweeps = descriptor
            .store_names()
            .any(|store| matches!(descriptor.bounded(store), Bounded::ByUnit(_)));

        if !sweeps {
            continue;
        }

        // [`Descriptor::size`] has already fallen back to the default where
        // what is written is not a size, so this is read.
        let size = languages::bytes(descriptor.size(name))
            .or_else(|_| languages::bytes(languages::default_size(name)))
            .unwrap_or(u64::MAX);

        let swept = swept(cache, descriptor, size);

        if !swept.removed.is_empty() {
            pass.removed = true;

            tracing::info!(
                language = name,
                held = swept.held,
                left = swept.left,
                size,
                units = swept.removed.len(),
                "a language's store was over its size and has been swept",
            );
        }

        if swept.stopped {
            pass.stopped = true;

            break;
        }

        pass.swept.push(name.to_owned());
    }

    pass
}

/// When each language's stores were last swept to the end, which the pane says
/// beside its size — held, like the disk use, for as long as the server runs.
#[derive(Debug, Clone, Default)]
pub(crate) struct Sweeps(Arc<Mutex<HashMap<String, OffsetDateTime>>>);

impl Sweeps {
    /// When `language`'s stores were last swept, and `None` where they have not
    /// been since the server started.
    pub(crate) fn of(&self, language: &str) -> Option<OffsetDateTime> {
        self.held().get(language).copied()
    }

    /// Say `languages` were swept to the end at `at`.
    pub(crate) fn swept(&self, languages: &[String], at: OffsetDateTime) {
        let mut held = self.held();

        for language in languages {
            held.insert(language.clone(), at);
        }
    }

    fn held(&self) -> std::sync::MutexGuard<'_, HashMap<String, OffsetDateTime>> {
        self.0.lock().unwrap_or_else(|held| held.into_inner())
    }
}

/// Sweep the stores from now until the process stops: at startup, and every
/// [`crate::Pace::evicting`] after that — each pass at the first moment nothing
/// runs, and gone on with at the next such moment where something starts
/// partway through.
///
/// Never on a server that runs no sessions, the other sweeps' reason, nor on
/// one with no Build Cache, which has no stores.
pub(crate) fn sweeping(state: &AppState) {
    if !state.sessions.runs_sessions() {
        return;
    }

    let Some(cache) = state
        .sessions
        .build_cache()
        .filter(|cache| cache.machine().is_some())
        .cloned()
    else {
        return;
    };

    let state = state.clone();

    tokio::spawn(async move {
        loop {
            loop {
                cache.idle().await;

                // Read off the file on every pass, like the Cleanup's settings:
                // a size typed on a phone is what the next pass goes by.
                let languages = languages::configured(&state.settings.config());
                let swept_with = cache.clone();

                let pass = match tokio::task::spawn_blocking(move || pass(&swept_with, &languages))
                    .await
                {
                    Ok(pass) => pass,
                    Err(error) => {
                        tracing::error!(error = ?error, "sweeping the languages' stores failed");

                        break;
                    }
                };

                state.sweeps.swept(&pass.swept, OffsetDateTime::now_utc());

                if pass.removed {
                    state.disk_use.refresh();
                }

                if !pass.stopped {
                    break;
                }
            }

            tokio::time::sleep(state.sessions.pace().evicting).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::mpsc;

    use crate::settings::Config;

    /// Where a unit is moved aside to, at the top of the Build Cache — see
    /// [`crate::languages::Machine::aside`].
    const ASIDE_NAME: &str = ".verkstead-swept";

    /// One language with a store of each of the three kinds, all under the
    /// Build Cache.
    const DEMO: &str = "\
languages:
  demo:
    stores:
      packages:
        dir: \"{cache}/demo/packages\"
        units:
          - depth: 1
      tool:
        dir: \"{cache}/demo/tool\"
        evicted: by-its-tool
      loose:
        dir: \"{cache}/demo/loose\"
";

    /// A Build Cache in a directory of its own, and the language above.
    struct Fixture {
        _dir: tempfile::TempDir,
        cache: BuildCache,
        root: PathBuf,
        languages: Languages,
    }

    impl Fixture {
        fn new() -> Fixture {
            let dir = tempfile::tempdir().unwrap();
            let root = dir.path().join("cache");

            Fixture {
                cache: BuildCache::at(root.clone(), None, dir.path().join("data")),
                root,
                _dir: dir,
                languages: Languages::read(DEMO).unwrap(),
            }
        }

        fn descriptor(&self) -> &Descriptor {
            self.languages.get("demo").unwrap()
        }

        /// A file of `bytes` at `rest` under the cache, written `days` ago.
        fn write(&self, rest: &str, bytes: usize, days: u64) -> PathBuf {
            let path = self.root.join(rest);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, vec![b'x'; bytes]).unwrap();
            dated(&path, days);

            path
        }

        fn swept(&self, size: u64) -> Swept {
            swept(&self.cache, self.descriptor(), size)
        }
    }

    /// Date what is at `path` `days` back, both of its times.
    fn dated(path: &Path, days: u64) {
        let when = SystemTime::now() - Duration::from_secs(days * 24 * 60 * 60);

        std::fs::File::options()
            .write(!path.is_dir())
            .read(true)
            .open(path)
            .unwrap()
            .set_times(
                std::fs::FileTimes::new()
                    .set_accessed(when)
                    .set_modified(when),
            )
            .unwrap();
    }

    /// Whole units go, oldest first, until the store is under its size — and
    /// not one more.
    #[test]
    fn a_store_over_its_size_loses_its_oldest_units_whole_until_it_is_under() {
        let fixture = Fixture::new();

        for (name, days) in [("old", 30), ("middling", 10), ("new", 1)] {
            fixture.write(&format!("demo/packages/{name}/a"), 100, days);
            fixture.write(&format!("demo/packages/{name}/b"), 100, days);
            dated(&fixture.root.join("demo/packages").join(name), days);
        }

        let swept = fixture.swept(450);

        assert_eq!(swept.held, 600);
        assert_eq!(swept.left, 400);
        assert_eq!(
            swept.removed,
            [fixture.root.join("demo/packages/old")],
            "the oldest unit, whole, and nothing once the store is under",
        );
        assert!(!fixture.root.join("demo/packages/old").exists());
        assert!(fixture.root.join("demo/packages/middling/a").exists());
        assert!(fixture.root.join("demo/packages/new/b").exists());

        // And nothing of it is left aside.
        let aside = fixture.root.join(ASIDE_NAME);
        assert!(
            std::fs::read_dir(&aside)
                .map(|mut entries| entries.next().is_none())
                .unwrap_or(true),
            "a unit moved aside is deleted",
        );
    }

    /// A store under its size, one its tool evicts for itself and one naming no
    /// unit are none of them touched, however full.
    #[test]
    fn under_its_size_by_its_tool_or_naming_no_unit_is_left_alone() {
        let fixture = Fixture::new();

        fixture.write("demo/packages/one/a", 100, 30);
        fixture.write("demo/tool/blob", 10_000, 30);
        fixture.write("demo/loose/thing/a", 10_000, 30);

        let swept = fixture.swept(100);

        assert_eq!(swept.held, 100, "only the store swept by unit is counted");
        assert!(swept.removed.is_empty());
        assert!(fixture.root.join("demo/packages/one/a").exists());

        // And over it, the other two are still not touched.
        let swept = fixture.swept(0);

        assert_eq!(swept.removed, [fixture.root.join("demo/packages/one")]);
        assert!(fixture.root.join("demo/tool/blob").exists());
        assert!(fixture.root.join("demo/loose/thing/a").exists());
        assert!(
            !fixture
                .root
                .join(ASIDE_NAME)
                .join("access-time-probe")
                .exists(),
            "and the probe is not left behind",
        );
    }

    /// A unit is as new as the newest thing in it: a Maven jar dated by its
    /// server long ago, downloaded today beside a file written today, is not
    /// the oldest unit.
    #[test]
    fn a_unit_is_as_new_as_the_newest_thing_in_it() {
        let fixture = Fixture::new();

        fixture.write("demo/packages/fresh/greet-1.0.jar", 100, 400);
        fixture.write("demo/packages/fresh/_remote.repositories", 1, 0);
        fixture.write("demo/packages/stale/other-1.0.jar", 100, 20);
        dated(&fixture.root.join("demo/packages/stale"), 20);

        let swept = fixture.swept(150);

        assert_eq!(swept.removed, [fixture.root.join("demo/packages/stale")]);
    }

    /// A file read lately is new where the filesystem keeps access times, and
    /// as old as its writing where it does not.
    #[test]
    fn a_file_is_judged_by_its_access_time_only_where_one_is_kept() {
        let dir = tempfile::tempdir().unwrap();

        let unit = dir.path().join("unit");
        std::fs::create_dir_all(&unit).unwrap();
        let file = unit.join("lib");
        std::fs::write(&file, b"x").unwrap();

        let long_ago = SystemTime::now() - Duration::from_secs(300 * 24 * 60 * 60);
        let lately = SystemTime::now() - Duration::from_secs(60);

        std::fs::File::options()
            .write(true)
            .open(&file)
            .unwrap()
            .set_times(
                std::fs::FileTimes::new()
                    .set_accessed(lately)
                    .set_modified(long_ago),
            )
            .unwrap();
        dated(&unit, 300);

        let survey = surveyed(&unit).unwrap();
        let kept = survey.newest(true);
        let unkept = survey.newest(false);

        assert!(kept >= lately - Duration::from_secs(1));
        assert!(unkept < lately - Duration::from_secs(24 * 60 * 60));
    }

    /// Nothing is taken while anything holds the cache, and the sweep says it
    /// stopped — which is what keeps it due.
    #[test]
    fn nothing_is_removed_while_a_session_or_terminal_runs() {
        let fixture = Fixture::new();

        fixture.write("demo/packages/one/a", 100, 30);

        let running = fixture.cache.compiling(&Config::default(), None);
        let swept = fixture.swept(0);

        assert!(swept.stopped);
        assert!(swept.removed.is_empty());
        assert!(fixture.root.join("demo/packages/one/a").exists());

        drop(running);

        let swept = fixture.swept(0);

        assert!(!swept.stopped);
        assert_eq!(swept.removed.len(), 1);
    }

    /// A launch arriving while a unit is moved waits for it, and the sweep
    /// moves nothing after it.
    #[test]
    fn a_launch_waits_for_the_unit_being_moved_and_stops_the_sweep() {
        let fixture = Fixture::new();
        let cache = fixture.cache.clone();

        let (moving, moved) = mpsc::channel();
        let (go_on, going_on) = mpsc::channel::<()>();
        let (launched, launching) = mpsc::channel();

        std::thread::scope(|scope| {
            let sweeping = &fixture.cache;
            let sweep = scope.spawn(move || {
                sweeping.moving(|| {
                    moving.send(()).unwrap();
                    going_on.recv().unwrap();
                })
            });

            moved.recv().unwrap();

            let launch = scope.spawn(move || {
                let hold = cache.compiling(&Config::default(), None);
                launched.send(()).unwrap();
                hold
            });

            assert!(
                launching.recv_timeout(Duration::from_millis(200)).is_err(),
                "a launch waits while a unit is being moved",
            );

            go_on.send(()).unwrap();

            assert_eq!(sweep.join().unwrap(), Some(()));
            launching.recv().unwrap();

            let hold = launch.join().unwrap();

            assert_eq!(
                fixture.cache.moving(|| ()),
                None,
                "and once it is counted, nothing more is moved",
            );

            drop(hold);
        });

        assert_eq!(fixture.cache.moving(|| ()), Some(()));
    }

    /// A due sweep waits for the first moment nothing runs, and not a moment
    /// longer.
    #[tokio::test]
    async fn a_sweep_waits_for_the_moment_nothing_runs() {
        let fixture = Fixture::new();

        let running = fixture.cache.compiling(&Config::default(), None);
        let cache = fixture.cache.clone();
        let waiting = tokio::spawn(async move { cache.idle().await });

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(
            !waiting.is_finished(),
            "nothing is idle while a session runs"
        );

        drop(running);

        tokio::time::timeout(Duration::from_secs(5), waiting)
            .await
            .expect("idle as soon as the last one lets go")
            .unwrap();

        // And at once where nothing runs.
        tokio::time::timeout(Duration::from_secs(5), fixture.cache.idle())
            .await
            .unwrap();
    }

    /// Go's module cache is read-only all the way down, and it is swept all the
    /// same.
    #[cfg(unix)]
    #[test]
    fn read_only_go_modules_are_swept() {
        use std::os::unix::fs::PermissionsExt;

        let fixture = Fixture::new();

        let module = fixture.root.join("demo/packages/greet@v1.0.0");
        fixture.write("demo/packages/greet@v1.0.0/sub/go.mod", 100, 30);
        fixture.write("demo/packages/greet@v1.0.0/greet.go", 100, 30);

        for path in [
            module.join("sub/go.mod"),
            module.join("greet.go"),
            module.join("sub"),
            module.clone(),
        ] {
            let mode = match path.is_dir() {
                true => 0o555,
                false => 0o444,
            };
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode)).unwrap();
        }

        let swept = fixture.swept(0);

        assert_eq!(swept.removed, std::slice::from_ref(&module));
        assert!(!module.exists());
        assert!(
            std::fs::read_dir(fixture.root.join(ASIDE_NAME))
                .unwrap()
                .next()
                .is_none(),
            "and nothing of it is left aside",
        );
    }

    /// A link in a unit goes with the unit, and what it points at stays.
    #[cfg(unix)]
    #[test]
    fn a_link_in_a_unit_is_never_followed() {
        let fixture = Fixture::new();

        let outside = fixture.write("project/precious", 10, 0);
        fixture.write("demo/packages/one/a", 100, 30);
        std::os::unix::fs::symlink(
            fixture.root.join("project"),
            fixture.root.join("demo/packages/one/linked"),
        )
        .unwrap();

        assert_eq!(fixture.swept(0).removed.len(), 1);
        assert!(outside.exists());
    }

    /// What a pass that died left aside is gone at the start of the next.
    #[test]
    fn what_a_pass_left_aside_is_deleted_by_the_next() {
        let fixture = Fixture::new();

        fixture.write(&format!("{ASIDE_NAME}/123-0/a/b"), 10, 0);
        fixture.write("demo/packages/one/a", 1, 0);

        let pass = pass(&fixture.cache, &fixture.languages);

        assert_eq!(pass.swept, ["demo"]);
        assert!(!pass.removed, "a store under its size loses nothing");
        assert!(!fixture.root.join(ASIDE_NAME).join("123-0").exists());
        assert!(fixture.root.join("demo/packages/one/a").exists());
    }

    /// A pass that finds something running stops, so it is still due.
    #[test]
    fn a_pass_stopped_by_a_launch_is_one_still_due() {
        let fixture = Fixture::new();

        fixture.write("demo/packages/one/a", 2048, 30);

        let languages =
            Languages::read(&DEMO.replace("    stores:", "    size: 1K\n    stores:")).unwrap();
        let _running = fixture.cache.compiling(&Config::default(), None);

        let pass = pass(&fixture.cache, &languages);

        assert!(pass.stopped);
        assert!(pass.swept.is_empty());
    }

    /// A Clear empties every store of the language, however each is bounded,
    /// and leaves another language's alone — one inside it included.
    #[test]
    fn a_clear_empties_every_store_of_one_language_and_no_other() {
        let fixture = Fixture::new();

        fixture.write("demo/packages/one/a", 100, 0);
        fixture.write("demo/packages/two/b", 100, 0);
        fixture.write("demo/tool/blob", 100, 0);
        fixture.write("demo/loose/thing/a", 100, 0);
        let others = fixture.write("demo/loose/theirs/kept", 100, 0);
        let beside = fixture.write("other/packages/one/a", 100, 0);

        let kept = [
            fixture.root.join("demo/loose/theirs"),
            fixture.root.join("other/packages"),
        ];

        assert_eq!(
            emptied(&fixture.cache, fixture.descriptor(), &kept),
            Some(4)
        );

        for store in ["packages", "tool"] {
            let dir = fixture.root.join("demo").join(store);

            assert!(
                std::fs::read_dir(&dir).unwrap().next().is_none(),
                "{} is empty, and still there",
                dir.display(),
            );
        }

        assert!(!fixture.root.join("demo/loose/thing").exists());
        assert!(others.exists(), "another language's store inside it stays");
        assert!(beside.exists(), "and so does another language's beside it");
        assert!(
            std::fs::read_dir(fixture.root.join(ASIDE_NAME))
                .unwrap()
                .next()
                .is_none(),
            "and nothing of it is left aside",
        );
    }

    /// A Clear is refused while anything runs, and takes nothing.
    #[test]
    fn a_clear_is_refused_while_a_session_or_terminal_runs() {
        let fixture = Fixture::new();

        let unit = fixture.write("demo/packages/one/a", 100, 0);

        let running = fixture.cache.compiling(&Config::default(), None);

        assert_eq!(emptied(&fixture.cache, fixture.descriptor(), &[]), None);
        assert!(unit.exists());

        drop(running);

        assert_eq!(emptied(&fixture.cache, fixture.descriptor(), &[]), Some(1));
        assert!(!unit.exists());
    }

    /// Go's read-only modules are cleared like anything else.
    #[cfg(unix)]
    #[test]
    fn a_clear_takes_read_only_go_modules() {
        use std::os::unix::fs::PermissionsExt;

        let fixture = Fixture::new();

        let module = fixture.root.join("demo/tool/greet@v1.0.0");
        fixture.write("demo/tool/greet@v1.0.0/greet.go", 100, 0);

        for (path, mode) in [(module.join("greet.go"), 0o444), (module.clone(), 0o555)] {
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode)).unwrap();
        }

        assert_eq!(emptied(&fixture.cache, fixture.descriptor(), &[]), Some(1));
        assert!(!module.exists());
    }
}
