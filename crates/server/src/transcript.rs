//! Following the log a session keeps of its own conversation, and putting every
//! line of it on the Transcript as it is written.
//!
//! The log is the agent's own file, in the agent's own format, and Verkstead is
//! a reader of it rather than a party to it. So it is found rather than
//! computed: Verkstead named the session before it started it (see
//! [`crate::sessions`]), and the file named for that session is looked for
//! inside the Agent Profile's `projects` directory. The alternative is working
//! out where the backend would have put it, which means reimplementing a private
//! algorithm belonging to somebody else's program (ADR 0006).
//!
//! Lines go to the store exactly as they were written, and nothing here reads
//! one. That is what holds the coupling to somebody else's file format down to
//! whoever renders it: a format that changes can leave a line nothing knows how
//! to draw, and it can never lose what was said.
//!
//! Following is plain polling, on the cadence the byte relay already flushes on
//! — the relay is awake on that interval anyway, and a file watcher would be a
//! second mechanism to get wrong for a file the same loop is already waiting on.
//!
//! A session that writes no log is followed the same way and stores nothing.
//! That is the stub agents the test suite runs, every backend that keeps no such
//! record, and every session Verkstead could not name: all of them are read back
//! off the Capture instead, which is a complete record on its own.

use std::path::{Path, PathBuf};

use sqlx::SqlitePool;
use tokio::io::{AsyncReadExt, AsyncSeekExt};

use crate::nudge::Nudges;
use crate::store;

/// One session's log, read as far as it has been written.
///
/// The whole of what following means is remembering where it got to: the log is
/// appended to while a session runs, and each poll takes what has arrived since
/// the last one.
pub(crate) struct Tail {
    /// Where the Agent Profile keeps its logs, one directory per project.
    projects: PathBuf,

    /// What Verkstead named this session, which is what the file is called.
    session: String,

    /// The log itself, once it has been found. A session writes its log when it
    /// starts talking rather than when it starts, so the first few polls of a
    /// real session ordinarily find nothing.
    log: Option<PathBuf>,

    /// How much of it has been read.
    read: u64,

    /// The beginning of a line whose end has not been written yet.
    ///
    /// A poll lands wherever the session had got to, which is regularly the
    /// middle of a line — and half a line is not something to keep, because a
    /// reader would take it for a whole one. Held in bytes rather than text
    /// because the same cut goes through characters as well as lines.
    ///
    /// Nothing bounds it but the line itself. A cap would have to either lose
    /// bytes or store a torn line, and both are worse than holding one line of
    /// a file the agent wrote.
    partial: Vec<u8>,

    /// Lines that are whole and not yet stored. Kept rather than dropped where
    /// the store refuses them, for the reason the Capture's flush keeps its own:
    /// a store that is briefly unwritable should cost latency rather than a hole
    /// in a record nothing can go back and fill.
    pending: Vec<String>,
}

impl Tail {
    /// Follow the log of the session named `session`, run under `profile`.
    pub(crate) fn of(profile: &store::Profile, session: &str) -> Tail {
        Tail {
            projects: profile.claude_dir.join("projects"),
            session: session.to_owned(),
            log: None,
            read: 0,
            partial: Vec::new(),
            pending: Vec::new(),
        }
    }

    /// Take whatever the session has written since the last poll, and put the
    /// whole lines of it on the Transcript.
    pub(crate) async fn poll(&mut self, pool: &SqlitePool, nudges: &Nudges, event_id: i64) {
        if self.log.is_none() {
            self.log = self.find().await;
        }

        if let Some(log) = self.log.clone() {
            self.take(&log).await;
        }

        self.store(pool, nudges, event_id).await;
    }

    /// Look for the log, and hand back where it is.
    ///
    /// `None` is the ordinary answer for most of a session's life: it is a
    /// session that has not written anything yet, a backend that keeps no log,
    /// and a Profile directory that has never been used. None of the three is
    /// worth saying anything about, which is why nothing here is logged.
    async fn find(&self) -> Option<PathBuf> {
        let named = format!("{}.jsonl", self.session);

        // Beside the project directories as well as inside one of them, because
        // which of the two the backend chooses is the backend's business — what
        // Verkstead knows is the directory it keeps them under and the name it
        // gave the session.
        let beside = self.projects.join(&named);
        if is_file(&beside).await {
            return Some(beside);
        }

        let mut projects = tokio::fs::read_dir(&self.projects).await.ok()?;

        while let Ok(Some(project)) = projects.next_entry().await {
            let inside = project.path().join(&named);

            if is_file(&inside).await {
                return Some(inside);
            }
        }

        None
    }

    /// Read what has been appended to the log since last time, and split off the
    /// lines of it that are finished.
    async fn take(&mut self, log: &Path) {
        let Ok(mut file) = tokio::fs::File::open(log).await else {
            // The file was there when it was found and is not now, which is a
            // Profile directory something else is tidying. The next poll looks
            // again.
            return;
        };

        if file
            .seek(std::io::SeekFrom::Start(self.read))
            .await
            .is_err()
        {
            return;
        }

        let mut arrived = Vec::new();

        if let Err(error) = file.read_to_end(&mut arrived).await {
            tracing::warn!(error = ?error, log = %log.display(), "reading a session's log failed");
            return;
        }

        self.read += arrived.len() as u64;
        self.partial.extend_from_slice(&arrived);

        // A line ends at its newline, and the newline is the framing rather than
        // anything the agent said — so it is what the line is split on and the
        // only byte not kept. Decoded a whole line at a time, which is also what
        // keeps a character the read cut in half out of the store.
        while let Some(ends) = self.partial.iter().position(|byte| *byte == b'\n') {
            let line: Vec<u8> = self.partial.drain(..=ends).collect();

            self.pending
                .push(String::from_utf8_lossy(&line[..ends]).into_owned());
        }
    }

    /// Put the finished lines on the Transcript, and tell whoever is watching
    /// that they are there.
    ///
    /// One contentless Nudge for the batch, because that is all there is to say:
    /// an open pane's answer to being nudged is to read everything again (ADR
    /// 0005), so nothing finer would change what it does.
    async fn store(&mut self, pool: &SqlitePool, nudges: &Nudges, event_id: i64) {
        if self.pending.is_empty() {
            return;
        }

        match store::append_transcript(pool, event_id, &self.pending).await {
            Err(error) => {
                tracing::error!(error = ?error, event_id, "keeping a session's Transcript failed")
            }
            Ok(()) => {
                self.pending.clear();
                nudges.announce();
            }
        }
    }
}

/// Whether there is a file at `path`, asked without blocking the loop doing the
/// asking.
async fn is_file(path: &Path) -> bool {
    tokio::fs::metadata(path)
        .await
        .is_ok_and(|found| found.is_file())
}
