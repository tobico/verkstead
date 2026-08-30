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
//! Lines go to the store exactly as they were written, and nothing here parses
//! one. That is what holds the coupling to somebody else's file format down to
//! whoever renders it: a format that changes can leave a line nothing knows how
//! to draw, and it can never lose what was said. What is read back out of a
//! batch on its way past is the two things the Timeline row is summarised by —
//! the last thing the agent said, and how many turns the batch was — and both
//! are read by the crate with the parser in it rather than here.
//!
//! Both are kept as the log is followed rather than worked out when the row is
//! read. A Timeline is read every time an open page hears the world moved, and
//! a count taken then would be every line of every session's log parsed to draw
//! one row of it.
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
use verkstead_schema::Nudge;

use crate::nudge::Nudges;
use crate::store;

/// One session's log, read as far as it has been written.
///
/// The whole of what following means is remembering where it got to: the log is
/// appended to while a session runs, and each poll takes what has arrived since
/// the last one.
pub(crate) struct Tail {
    /// Whose session it is. Kept because what a Nudge about the Transcript has
    /// to say is where it moved (ADR-0009), and the log itself says nothing
    /// about that.
    conversation: i64,

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

    /// The last thing the agent said in its own words, as far as the log has
    /// been followed. What the Timeline row reads — see
    /// [`crate::capture::Reading::summary`].
    ///
    /// `None` until it says something, which is most of a session's first
    /// minute and the whole of a session whose backend keeps no log.
    latest: Option<String>,

    /// How many turns the conversation has taken, as far as the log has been
    /// followed — the other half of what the row reads.
    ///
    /// Kept as it goes rather than counted on the way out, for the reason the
    /// line count is: a Timeline is read every time an open page hears the
    /// world moved, and a count worked out then would be every line of every
    /// session's log parsed to draw one row. Each batch is read once, here, by
    /// the loop that was following the log anyway.
    ///
    /// `None` until a line reaches the Transcript, which is what says there is
    /// no Transcript to count: a session whose backend keeps no log stays at
    /// `None` for its whole life, and its row shows no metric at all.
    turns: Option<i64>,
}

impl Tail {
    /// Follow the log of the session named `session`, run under `profile` for
    /// `conversation`.
    pub(crate) fn of(conversation: i64, profile: &store::Profile, session: &str) -> Tail {
        // Where Claude Code keeps its logs, under the directory the account is.
        // Each backend after it names its own place under its own account, so
        // this is one arm per type rather than one path every type is assumed
        // to keep.
        let store::Account::Claude { claude_dir, .. } = &profile.account;

        Tail {
            conversation,
            projects: claude_dir.join("projects"),
            session: session.to_owned(),
            log: None,
            read: 0,
            partial: Vec::new(),
            pending: Vec::new(),
            latest: None,
            turns: None,
        }
    }

    /// The last thing the agent said, for whoever is summarising the session.
    pub(crate) fn latest(&self) -> Option<&str> {
        self.latest.as_deref()
    }

    /// How many turns are on the Transcript, for the same summary — `None`
    /// where there is no Transcript at all.
    pub(crate) fn turns(&self) -> Option<i64> {
        self.turns
    }

    /// Take whatever the session has written since the last poll, and put the
    /// whole lines of it on the Transcript.
    ///
    /// Whether [`Tail::latest`] moved on, so that whoever is summarising the
    /// session writes a row only where there is a new one to write.
    pub(crate) async fn poll(&mut self, pool: &SqlitePool, nudges: &Nudges, event_id: i64) -> bool {
        if self.log.is_none() {
            self.log = self.find().await;
        }

        if let Some(log) = self.log.clone() {
            self.take(&log).await;
        }

        self.store(pool, nudges, event_id).await
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
    /// One Nudge for the batch, because that is all there is to say: it names
    /// the Transcript and the Conversation it moved in, and a second one would
    /// name the same two things again.
    ///
    /// Whether the batch moved what the Timeline row shows — something the
    /// agent said, or turns to add to the count. A batch of nothing but the
    /// backend's own bookkeeping has moved neither.
    async fn store(&mut self, pool: &SqlitePool, nudges: &Nudges, event_id: i64) -> bool {
        if self.pending.is_empty() {
            return false;
        }

        match store::append_transcript(pool, event_id, &self.pending).await {
            Err(error) => {
                tracing::error!(error = ?error, event_id, "keeping a session's Transcript failed");
                false
            }
            Ok(()) => {
                let said = latest(&self.pending);
                let counted = verkstead_render::turns(&self.pending) as i64;

                // The count starts at the first batch that lands rather than at
                // the Tail: what `Some` says is that there is a Transcript, and
                // there is one from the moment a line is on it.
                *self.turns.get_or_insert(0) += counted;

                let moved = said.is_some() || counted > 0;

                if said.is_some() {
                    self.latest = said;
                }

                self.pending.clear();
                nudges.announce(Nudge::Transcript {
                    conversation: self.conversation,
                });

                moved
            }
        }
    }
}

/// The newest thing the agent said in a batch of Transcript lines, or `None`
/// where the batch was all tools, thinking and bookkeeping.
///
/// The last line of the last statement rather than the whole of it. What reads
/// this is one row of a Timeline, and an agent that wrote three paragraphs
/// before asking a question has the question at the end of them — which is the
/// half somebody glancing at the row came for.
fn latest(lines: &[String]) -> Option<String> {
    verkstead_render::statements(lines)
        .last()?
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .next_back()
        .map(str::to_owned)
}

/// Whether there is a file at `path`, asked without blocking the loop doing the
/// asking.
async fn is_file(path: &Path) -> bool {
    tokio::fs::metadata(path)
        .await
        .is_ok_and(|found| found.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An agent writes what it is about to do, does it, and says how it went.
    /// The row reads the last of those, and the end of it: what a session is
    /// waiting on is the last thing it wrote, not the first.
    #[test]
    fn the_last_thing_said_is_the_end_of_the_last_thing_said() {
        let lines = vec![
            r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"Reading the brief."}]}}"#.to_owned(),
            r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"The counter has no home.\n\nWhere should it live?"}]}}"#.to_owned(),
        ];

        assert_eq!(latest(&lines).as_deref(), Some("Where should it live?"));
    }

    #[test]
    fn a_batch_of_nothing_but_tools_has_not_moved_what_was_said() {
        let lines = vec![
            r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"tool_use","id":"t1","name":"Bash","input":{"command":"ls"}}]}}"#.to_owned(),
            r#"{"type":"user","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"t1","content":"limiter.md"}]}}"#.to_owned(),
        ];

        assert_eq!(latest(&lines), None);
    }

    #[test]
    fn a_session_that_keeps_no_log_has_nothing_to_say_for_itself() {
        assert_eq!(latest(&[]), None);
    }
}
