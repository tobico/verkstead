//! Where the server's `tracing` goes when nobody is watching a terminal.
//!
//! `verkstead serve` writes to the stdout of the shell that started it, which
//! is the right answer for a binary somebody started from a shell. A Verkstead
//! started from an icon has no such shell: its stdout goes to whatever the
//! desktop happened to launch it from, which on most is nowhere at all. So this
//! binary writes to a file instead — in the **Log Directory**, which is the
//! directory that exists for this and nothing else, and which *this* binary
//! makes, stage 01 having deliberately resolved it without creating it.
//!
//! **Bounded rather than endless.** A machine that has been running Verkstead
//! for months should not be handed a log nobody can open, so the file rolls over
//! at [`ROLL_AT`] and what it rolls over to is the one file behind it — at most
//! two files, at most twice the bound, whatever the machine has been up to.
//!
//! **Nowhere to put one is not a failure.** A machine that names no Log
//! Directory, or one whose Log Directory cannot be opened, gets no file: the app
//! says so, logs to stderr instead and goes on running, because putting a tray
//! up is still the job it was launched to do. That is the opposite of what the
//! Data Directory does with the same misfortune, and deliberately — a Verkstead
//! with nowhere to keep its database has nothing to serve, while one with
//! nowhere to keep a log file has only lost the log.

use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::MakeWriter;

/// What is logged when `RUST_LOG` says nothing: what `verkstead serve` logs —
/// the server's own startup line and whatever else it has to report — this
/// crate's own account of the tray, the browser and this file beside it, and
/// the verb's account of what stopped the app, with nothing from the crates
/// beneath any of them. The app's half is here because it is the half that says
/// why there is no icon in the tray, and a reason nobody is shown is no reason
/// at all; `verkstead_cli` is here because the tray app is reached through that
/// crate's `desktop` verb, and the log file is the only place a tray app's last
/// words are read.
const DEFAULT_FILTER: &str = "verkstead_server=info,verkstead_desktop=info,verkstead_cli=info";

/// What the log file is called inside the Log Directory.
const FILE: &str = "verkstead.log";

/// And what it rolls over to. One file behind the live one and no more: two
/// runs' worth of the recent past is what somebody reporting a problem is asked
/// for, and the whole point of rolling over is that this directory has a size
/// nobody has to think about.
const PREVIOUS: &str = "verkstead.log.1";

/// How large the live file may get before it rolls over.
///
/// Big enough to hold a long run of an ordinary machine's logging, small enough
/// to open in a text editor and to attach to a report of what went wrong.
const ROLL_AT: u64 = 4 * 1024 * 1024;

/// Where this run's logging went, which is what **View Logs** opens — or says
/// instead of opening.
#[derive(Debug, Clone)]
pub enum Kept {
    /// In the log file at this path.
    In(PathBuf),
    /// Nowhere: there is no file, for the reason this carries, and the logging
    /// is going to stderr. The reason is worded for a human because it is shown
    /// to one — see [`crate::dialog::note`].
    Nowhere(String),
}

/// Send this process's `tracing` to the log file, and say where that turned out
/// to be.
///
/// Called once, at startup, before anything has anything to report. `RUST_LOG`
/// filters what is written exactly as it filters `verkstead serve`'s stdout:
/// where the events *go* is the starting binary's call, and which of them are
/// worth writing is not.
pub fn start() -> Kept {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| DEFAULT_FILTER.into());

    let Some(dir) = verkstead_server::platform::log_dir() else {
        return nowhere(
            filter,
            "Verkstead has nowhere to keep a log file on this machine, so this run's log is \
             going to the standard error of whatever started it."
                .to_owned(),
        );
    };

    let rolling = match Bounded::in_dir(&dir) {
        Ok(rolling) => rolling,
        Err(why) => {
            return nowhere(
                filter,
                format!(
                    "Verkstead could not open its log file in {} ({why}), so this run's log \
                     is going to the standard error of whatever started it.",
                    dir.display()
                ),
            );
        }
    };

    let file = rolling.live.clone();

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        // A file rather than a terminal, and escape sequences in one are noise
        // to whoever opens it.
        .with_ansi(false)
        .with_writer(Rolling(Arc::new(Mutex::new(rolling))))
        .init();

    Kept::In(file)
}

/// The other ending: no file, the logging on stderr, and `why` said there as
/// well as kept for the menu item that would have opened one.
fn nowhere(filter: EnvFilter, why: String) -> Kept {
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(io::stderr)
        .init();

    tracing::warn!("{why}");

    Kept::Nowhere(why)
}

/// The log file itself: appended to, and rolled over where a write would take
/// it past [`ROLL_AT`].
struct Bounded {
    /// The file being written, which is the one **View Logs** opens.
    live: PathBuf,
    /// What it becomes when it rolls over, overwriting whatever roll came
    /// before — this is where the bound on the whole directory comes from.
    previous: PathBuf,
    /// The live file, held open for as long as the app runs.
    file: File,
    /// How much is in it, kept as it is written rather than asked of the
    /// filesystem: a `stat` per logged event would be a syscall for nothing.
    written: u64,
}

impl Bounded {
    /// Open the log file in `dir`, **making the directory**, and pick up where a
    /// previous run left off.
    ///
    /// Appended to rather than truncated: a crash is exactly when the run before
    /// this one is worth reading, and it is the rolling over rather than the
    /// starting that keeps this bounded.
    fn in_dir(dir: &Path) -> io::Result<Bounded> {
        std::fs::create_dir_all(dir)?;

        let live = dir.join(FILE);
        let file = OpenOptions::new().create(true).append(true).open(&live)?;
        let written = file.metadata()?.len();

        Ok(Bounded {
            live,
            previous: dir.join(PREVIOUS),
            file,
            written,
        })
    }

    /// The live file becomes the previous one, and a new live file is opened
    /// behind it.
    ///
    /// A rename rather than a copy, so nothing is ever half-written: what a
    /// reader has open goes on being the file it opened, and the roll costs one
    /// directory entry whatever the file grew to.
    fn roll(&mut self) -> io::Result<()> {
        self.file.flush()?;
        std::fs::rename(&self.live, &self.previous)?;
        self.file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.live)?;
        self.written = 0;

        Ok(())
    }
}

impl Write for Bounded {
    /// One call per logged event — the formatting layer hands over a whole
    /// event at a time — which is what makes rolling over here safe: the file
    /// is cut between events rather than through the middle of one.
    fn write(&mut self, event: &[u8]) -> io::Result<usize> {
        if self.written > 0 && self.written + event.len() as u64 > ROLL_AT {
            self.roll()?;
        }

        let written = self.file.write(event)?;
        self.written += written as u64;

        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}

/// The one file, shared: the server logs from every thread its runtime has, and
/// they all write into it.
#[derive(Clone)]
struct Rolling(Arc<Mutex<Bounded>>);

/// What a thread writes an event with, which is the lock over the file held for
/// as long as that event takes.
struct Pen<'a>(MutexGuard<'a, Bounded>);

impl Write for Pen<'_> {
    fn write(&mut self, event: &[u8]) -> io::Result<usize> {
        self.0.write(event)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

impl<'a> MakeWriter<'a> for Rolling {
    type Writer = Pen<'a>;

    /// A poisoned lock is taken anyway: a panic on some other thread while it
    /// held this is a thing to be logging *about*, and refusing to log would
    /// throw the account of it away.
    fn make_writer(&'a self) -> Pen<'a> {
        Pen(self
            .0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An event of `bytes` bytes, which is all the writer here cares about one.
    fn event(bytes: usize) -> Vec<u8> {
        vec![b'x'; bytes]
    }

    /// Filling the file rolls it over, and what was in it is behind rather than
    /// gone: this is the whole of what "rotating" means here.
    #[test]
    fn a_full_file_rolls_over_to_the_one_behind_it() {
        let dir = tempfile::tempdir().unwrap();
        let mut log = Bounded::in_dir(dir.path()).unwrap();

        let event = event(64 * 1024);
        let events = (ROLL_AT / event.len() as u64) + 1;
        for _ in 0..events {
            log.write_all(&event).unwrap();
        }

        let live = dir.path().join(FILE);
        let previous = dir.path().join(PREVIOUS);

        assert!(
            previous.exists(),
            "the full file should be behind the live one"
        );
        assert!(
            previous.metadata().unwrap().len() <= ROLL_AT,
            "the file rolls over at the bound rather than past it"
        );
        assert!(
            live.metadata().unwrap().len() < ROLL_AT,
            "the live file is the one started after the roll"
        );
    }

    /// And rolling over again overwrites what was behind, which is where the
    /// bound on the directory as a whole comes from: two files and no more,
    /// however long the machine has been running.
    #[test]
    fn rolling_over_twice_keeps_two_files_and_no_more() {
        let dir = tempfile::tempdir().unwrap();
        let mut log = Bounded::in_dir(dir.path()).unwrap();

        let event = event(64 * 1024);
        let events = 2 * ((ROLL_AT / event.len() as u64) + 1);
        for _ in 0..events {
            log.write_all(&event).unwrap();
        }

        let mut kept: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        kept.sort();

        assert_eq!(kept, [FILE, PREVIOUS]);
    }

    /// A run picks up where the one before it left off rather than throwing it
    /// away — and picks up its *size* with it, so a file that was full when the
    /// app was stopped rolls over rather than growing past the bound.
    #[test]
    fn a_second_run_appends_to_what_the_first_one_wrote() {
        let dir = tempfile::tempdir().unwrap();

        let mut first = Bounded::in_dir(dir.path()).unwrap();
        first.write_all(b"the first run\n").unwrap();
        drop(first);

        let mut second = Bounded::in_dir(dir.path()).unwrap();
        second.write_all(b"the second run\n").unwrap();
        drop(second);

        let written = std::fs::read_to_string(dir.path().join(FILE)).unwrap();

        assert_eq!(written, "the first run\nthe second run\n");
    }

    /// The Log Directory is made by the binary that opens a file in it, which is
    /// this one — stage 01's resolution deliberately makes nothing.
    #[test]
    fn opening_the_file_makes_the_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("state").join("verkstead");

        Bounded::in_dir(&dir).unwrap();

        assert!(
            dir.join(FILE).exists(),
            "{} should hold a log file",
            dir.display()
        );
    }
}
