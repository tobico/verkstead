//! Following the record a session keeps of its own conversation, and putting
//! every line of it on the Transcript as it is written.
//!
//! The record is the agent's own, in the agent's own format, and Verkstead is a
//! reader of it rather than a party to it. So it is found rather than computed:
//! working out where the backend would have put it means reimplementing a
//! private algorithm belonging to somebody else's program (ADR 0006).
//!
//! **Three of the four backends keep a file of lines, and the fourth keeps a
//! database** — see [`Following`]. What that changes is the bookkeeping and
//! nothing else: a file is followed by remembering how far into it the reading
//! got, a store by remembering the last record taken, and both take what has
//! arrived since the last poll. opencode's half of it is [`crate::records`].
//!
//! **How it is found is the backend's own** — see [`Search`]. Claude Code takes
//! the name Verkstead gave the session before it started it (see
//! [`crate::sessions`]) and writes a file called that, so its log is a lookup
//! inside the Agent Profile's `projects` directory. Codex takes no session id at
//! all, so nothing known before it starts names its log: what identifies a
//! rollout is what the session wrote in it about itself, which is the Worktree
//! it opened in — so the session's log is the one naming this Worktree that
//! appeared after this session was launched. Grok Build takes one too, so its
//! log is a lookup again — of a directory called the name, inside a directory
//! grok named by encoding the working directory, which is why the store is
//! walked rather than the encoding reproduced. opencode takes none either and
//! keeps no file: the session to follow is the row of its account's database
//! that records this Worktree and was created after this session was launched,
//! which is Codex's rule against a store of another shape.
//!
//! Lines go to the store exactly as they were written, and nothing here parses
//! one — a database's records included, which reach it as their payload
//! verbatim with the kind and the sequence the store filed them under around
//! it. That is what holds the coupling to somebody else's file format down to
//! whoever renders it: a format that changes can leave a line nothing knows how
//! to draw, and it can never lose what was said. What is read back out of a
//! batch on its way past is the two things the Timeline row is summarised by —
//! the last thing the agent said, and how many turns the batch was — and both
//! are read by the crate with the parser in it rather than here. So is the one
//! line of a rollout that says whose session it is, for the same reason: the
//! shape of somebody else's file is known in one place.
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
//! A session that writes no record is followed the same way and stores nothing.
//! That is the stub agents the test suite runs, every backend that keeps no such
//! record, every session Verkstead could not name, and every store of a shape
//! this build cannot read: all of them are read back off the Capture instead,
//! which is a complete record on its own.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use sqlx::SqlitePool;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncSeekExt, BufReader};
use verkstead_schema::Nudge;

use crate::nudge::Nudges;
use crate::records;
use crate::store;

/// One session's record, read as far as it has been written.
///
/// The whole of what following means is remembering where it got to: the record
/// is added to while a session runs, and each poll takes what has arrived since
/// the last one.
pub(crate) struct Tail {
    /// Whose session it is. Kept because what a Nudge about the Transcript has
    /// to say is where it moved (ADR-0009), and the log itself says nothing
    /// about that.
    conversation: i64,

    /// How this session's record is found — see [`Search`].
    search: Search,

    /// The record itself, once it has been found, and how far it has been
    /// followed — see [`Following`]. A session writes its record when it starts
    /// talking rather than when it starts, so the first few polls of a real
    /// session ordinarily find nothing.
    following: Following,

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

/// How a session's own record of itself is found.
///
/// Four backends, four answers, and the difference between them is not an
/// implementation detail. Claude takes the name Verkstead gave the session and
/// writes a file called that, so its log is a lookup. Codex takes no session id
/// at launch at all — so nothing Verkstead knows before the session starts
/// names its log, and what identifies it afterwards is what the session wrote
/// about itself: the Worktree it opened in, in a file that appeared after it
/// was launched. Grok Build takes one as Claude does, so its log is named again
/// — but it is a file of a fixed name inside a directory called the name, and
/// what that directory sits in is grok's own encoding of the working directory.
/// opencode takes none either, and keeps no log at all: it writes its sessions
/// into a database, and the one to follow is found in it the way a rollout is —
/// see [`crate::records`].
enum Search {
    /// Claude's: the Profile's directory of projects, and the name Verkstead
    /// gave the session, which is what the file is called.
    Named { projects: PathBuf, session: String },

    /// Codex's: the account's store of rollouts, the Worktree this session is
    /// working in, and the moment it was launched — see [`rollout`].
    Rollout {
        sessions: PathBuf,
        worktree: PathBuf,
        launched: SystemTime,
    },

    /// Grok Build's: the account's store of sessions, and the name Verkstead
    /// gave this one, which is what the directory holding its log is called —
    /// see [`updates`].
    Updates { sessions: PathBuf, session: String },

    /// OpenCode's: the database its account keeps its sessions in, the Worktree
    /// this session is working in, and the moment it was launched. Codex's rule
    /// against a store that is not a file of lines — see [`crate::records`].
    Records {
        database: PathBuf,
        worktree: PathBuf,
        launched: SystemTime,
    },

    /// And nowhere at all, for a session there is nothing to look for. Nothing
    /// is looked for and the Capture is the whole record, which is ADR-0006's
    /// rule for a session with no log, unchanged.
    Nowhere,
}

/// What codex calls the directory it keeps its rollouts under, inside the one
/// directory its account is.
///
/// Named because it is somebody else's spelling, the same bargain the
/// usage-limit phrase and the idle signature make: one place to edit when it
/// moves.
const ROLLOUTS: &str = "sessions";

/// And what grok calls the directory it keeps its sessions under, inside the one
/// directory its account is.
///
/// The same word as [`ROLLOUTS`] and a constant of its own, because it is a
/// second backend's spelling of its own store rather than the same store: one of
/// the two moving is not both of them moving.
const SESSIONS: &str = "sessions";

/// What grok calls the log itself, inside the directory named for the session.
///
/// The authoritative record of the conversation, and the only file in there that
/// is: `summary.json` beside it is the store's index entry, and the rest is the
/// session's own furniture — its plan, its rewind points, its raw chat history.
const UPDATES: &str = "updates.jsonl";

impl Tail {
    /// Follow the record of the session named `session`, run under `profile`
    /// for `conversation` in `worktree`, and started at `launched`.
    ///
    /// The last two are what the two backends that take no session id are found
    /// *by*, and are nothing to the two that name their own: a Codex or an
    /// OpenCode session with neither is a session with nothing to look for —
    /// see [`Search`].
    pub(crate) fn of(
        conversation: i64,
        profile: &store::Profile,
        session: &str,
        worktree: Option<&Path>,
        launched: SystemTime,
    ) -> Tail {
        // One arm per agent type rather than one path every type is assumed to
        // keep: where a backend puts its record, and what it calls it, is that
        // backend's own business, and a backend arriving with a fourth answer
        // lands here.
        let search = match (&profile.account, worktree) {
            // Where Claude Code keeps its logs, under the directory the account
            // is.
            (store::Account::Claude { claude_dir, .. }, _) => Search::Named {
                projects: claude_dir.join("projects"),
                session: session.to_owned(),
            },

            // And where codex keeps its rollouts, under the one directory its
            // account is.
            (store::Account::Codex { home }, Some(worktree)) => Search::Rollout {
                sessions: home.join(ROLLOUTS),
                worktree: worktree.to_owned(),
                launched: to_the_second(launched),
            },

            // A Codex session with no Worktree has nothing to match a rollout
            // against, and a session with no Worktree never starts — see
            // [`crate::sessions::Sessions::start`]. So this is a case that
            // cannot happen rather than one that is given up on, and it is
            // given up on rather than guessed at.
            (store::Account::Codex { .. }, None) => Search::Nowhere,

            // And where grok keeps its sessions, under the one directory its
            // account is. Grok Build names its session at launch, so the log is
            // named rather than found — the Worktree and the moment are
            // nothing to it.
            (store::Account::Grok { home }, _) => Search::Updates {
                sessions: home.join(SESSIONS),
                session: session.to_owned(),
            },

            // And where opencode keeps its sessions, which is one database
            // under the data half of the two directories its account is. The
            // name of the file is the one the sandbox pinned rather than the
            // one opencode would have chosen for itself — see
            // [`crate::sandbox`].
            (store::Account::OpenCode { home }, Some(worktree)) => Search::Records {
                database: home
                    .join(crate::sandbox::OPENCODE_DATA_INSIDE_HOME)
                    .join(crate::sandbox::OPENCODE_DB_FILE),
                worktree: worktree.to_owned(),
                launched,
            },

            // And an OpenCode session with no Worktree has nothing to match a
            // session in that store against, which is the Codex case above word
            // for word: it cannot happen, and it is given up on rather than
            // guessed at.
            (store::Account::OpenCode { .. }, None) => Search::Nowhere,
        };

        Tail {
            conversation,
            search,
            following: Following::Looking,
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
        if matches!(self.following, Following::Looking) {
            self.following = self.find().await;
        }

        let arrived = match &mut self.following {
            Following::Looking => Vec::new(),
            Following::Log(log) => log.take().await,
            Following::Records(records) => records.take().await,
        };

        self.pending.extend(arrived);

        self.store(pool, nudges, event_id).await
    }

    /// Look for the session's own record, and hand back the following of it.
    ///
    /// [`Following::Looking`] is the ordinary answer for most of a session's
    /// life: it is a session that has not written anything yet, a backend whose
    /// record is not looked for, and a Profile directory that has never been
    /// used. None of the three is worth saying anything about, which is why
    /// nothing here is logged.
    ///
    /// A store is the exception, and it is not one: there is nothing to look
    /// for, because what is looked for is inside the database rather than
    /// beside it, and the reader does its own finding on every poll until it
    /// finds its session — see [`crate::records`].
    async fn find(&self) -> Following {
        match &self.search {
            Search::Nowhere => Following::Looking,
            Search::Named { projects, session } => following(named(projects, session).await),
            Search::Rollout {
                sessions,
                worktree,
                launched,
            } => following(rollout(sessions, worktree, *launched).await),
            Search::Updates { sessions, session } => following(updates(sessions, session).await),
            Search::Records {
                database,
                worktree,
                launched,
            } => Following::Records(records::Reader::of(
                database.clone(),
                worktree.clone(),
                *launched,
            )),
        }
    }
}

/// What a session's own record turned out to be, and how far it has been
/// followed.
///
/// The two shapes a backend keeps one in. Three of the four write a file of
/// lines and are followed by remembering how far into it the reading got; the
/// fourth writes a database and is followed by remembering which record was the
/// last taken. Both are the same bargain — take what has arrived since last
/// time — and neither's bookkeeping means anything to the other.
enum Following {
    /// Nothing found yet, which every poll looks again for.
    Looking,

    /// A log file, and how far into it has been read.
    Log(Log),

    /// A session inside a store, and how far its records have been taken.
    Records(records::Reader),
}

/// A found log, read as far as it has been written.
struct Log {
    /// The file itself.
    log: PathBuf,

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
}

/// A log that was looked for, as something to follow.
fn following(found: Option<PathBuf>) -> Following {
    match found {
        Some(log) => Following::Log(Log {
            log,
            read: 0,
            partial: Vec::new(),
        }),
        None => Following::Looking,
    }
}

impl Log {
    /// Read what has been appended since last time, and hand back the lines of
    /// it that are finished.
    async fn take(&mut self) -> Vec<String> {
        let Ok(mut file) = tokio::fs::File::open(&self.log).await else {
            // The file was there when it was found and is not now, which is a
            // Profile directory something else is tidying. The next poll looks
            // again.
            return Vec::new();
        };

        if file
            .seek(std::io::SeekFrom::Start(self.read))
            .await
            .is_err()
        {
            return Vec::new();
        }

        let mut arrived = Vec::new();

        if let Err(error) = file.read_to_end(&mut arrived).await {
            tracing::warn!(
                error = ?error,
                log = %self.log.display(),
                "reading a session's log failed",
            );
            return Vec::new();
        }

        self.read += arrived.len() as u64;
        self.partial.extend_from_slice(&arrived);

        let mut lines = Vec::new();

        // A line ends at its newline, and the newline is the framing rather than
        // anything the agent said — so it is what the line is split on and the
        // only byte not kept. Decoded a whole line at a time, which is also what
        // keeps a character the read cut in half out of the store.
        while let Some(ends) = self.partial.iter().position(|byte| *byte == b'\n') {
            let line: Vec<u8> = self.partial.drain(..=ends).collect();

            lines.push(String::from_utf8_lossy(&line[..ends]).into_owned());
        }

        lines
    }
}

impl Tail {
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

/// The log Claude Code keeps of a session, which is the file named for the name
/// Verkstead gave it.
async fn named(projects: &Path, session: &str) -> Option<PathBuf> {
    let named = format!("{session}.jsonl");

    // Beside the project directories as well as inside one of them, because
    // which of the two the backend chooses is the backend's business — what
    // Verkstead knows is the directory it keeps them under and the name it
    // gave the session.
    let beside = projects.join(&named);
    if is_file(&beside).await {
        return Some(beside);
    }

    let mut projects = tokio::fs::read_dir(projects).await.ok()?;

    while let Ok(Some(project)) = projects.next_entry().await {
        let inside = project.path().join(&named);

        if is_file(&inside).await {
            return Some(inside);
        }
    }

    None
}

/// The rollout a Codex session is keeping of itself: the log under the
/// account's session store whose first line names `worktree` and which has been
/// written to since `launched`.
///
/// Two tests rather than one, because neither is enough on its own. The
/// Worktree alone would find whatever earlier session last worked in it, and
/// the moment alone would find whichever Conversation happened to start a
/// session at the same time — Verkstead runs as many at once as the machine
/// will take. Together they are one session: one Conversation per Worktree and
/// one session per Conversation, so a rollout that names this Worktree and
/// appeared after this session started is this session's.
///
/// The newest of them where there is more than one, because the moment is only
/// as fine as the filesystem's clock — see [`to_the_second`].
async fn rollout(sessions: &Path, worktree: &Path, launched: SystemTime) -> Option<PathBuf> {
    let mut found: Option<(SystemTime, PathBuf)> = None;

    for day in days(sessions, launched).await {
        let Ok(mut entries) = tokio::fs::read_dir(&day).await else {
            continue;
        };

        while let Ok(Some(entry)) = entries.next_entry().await {
            let log = entry.path();

            if !is_rollout(&log) {
                continue;
            }

            let Some(written) = written(&log).await.filter(|written| *written >= launched) else {
                continue;
            };

            // The two cheap tests first, and the file opened only for what
            // survives them: reading somebody else's first line is the
            // expensive one, and by here it is being read of this session's own
            // log and hardly anything else.
            if found.as_ref().is_some_and(|(newest, _)| *newest >= written) {
                continue;
            }

            if names(&log, worktree).await {
                found = Some((written, log));
            }
        }
    }

    found.map(|(_, log)| log)
}

/// The day directories of `sessions` written into since `launched`, which is
/// everywhere a log of a session launched then can be.
///
/// Codex files its rollouts by date, `sessions/YYYY/MM/DD`, and creating a file
/// touches the directory it lands in — so the day this session's log was
/// written in is a directory at least as new as the log, and every other day in
/// the store is a day the log is not in. The dates themselves are never read:
/// which day codex thinks it is, in which timezone, is codex's business.
///
/// The years and the months above them are walked whole, because the same trick
/// does not work on them: a day directory created this morning left this
/// month's directory older than a session launched this afternoon. There are
/// twelve months in a year and a handful of years in a store, so what that
/// costs is a directory listing of a directory of directories.
async fn days(sessions: &Path, launched: SystemTime) -> Vec<PathBuf> {
    let mut days = Vec::new();

    for year in directories(sessions).await {
        for month in directories(&year).await {
            for day in directories(&month).await {
                if written(&day)
                    .await
                    .is_some_and(|written| written >= launched)
                {
                    days.push(day);
                }
            }
        }
    }

    days
}

/// Whether the first line of `log` says its session was working in `worktree`.
///
/// A rollout opens with the session's own metadata, and in it is the directory
/// codex was launched in — which for a Verkstead session is the Conversation's
/// Worktree, bound into the sandbox at the path it has outside one so that the
/// two are the same string.
///
/// One line rather than the file, because one line is the whole of what
/// identifies a rollout and the rest of it is the Transcript's to keep and the
/// renderer's to read. Bounded, because this runs against a file somebody else
/// is writing: a poll can land before the first line has its newline, which is
/// a rollout that is not identifiable *yet* and the next poll asks again. A
/// first line longer than the bound is one that never becomes identifiable, and
/// that session stays Capture-only — the same answer as a log that never
/// appeared.
async fn names(log: &Path, worktree: &Path) -> bool {
    let Ok(file) = tokio::fs::File::open(log).await else {
        return false;
    };

    let mut first = Vec::new();

    if BufReader::new(file.take(FIRST_LINE))
        .read_until(b'\n', &mut first)
        .await
        .is_err()
    {
        return false;
    }

    verkstead_render::rollout_cwd(&String::from_utf8_lossy(&first))
        .is_some_and(|cwd| Path::new(&cwd) == worktree)
}

/// How much of a rollout is read to identify it. Generous against the eighteen
/// kilobytes codex 0.149.0 opens one with — the session's whole system prompt
/// is in there — and small against a log that runs to megabytes.
const FIRST_LINE: u64 = 256 * 1024;

/// Whether `log` is a rollout being written rather than one of the other things
/// codex keeps beside them.
///
/// The store holds more than the live logs: codex compresses the older ones,
/// and it is moving its index of them into SQLite in the same tree. What this
/// needs is to ignore whatever is not a plain JSONL rollout, rather than to keep
/// up with either.
fn is_rollout(log: &Path) -> bool {
    log.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with(ROLLOUT) && name.ends_with(ROLLOUT_SUFFIX))
}

/// What codex calls one: `rollout-<timestamp>-<uuid>.jsonl`. Named for the
/// reason [`ROLLOUTS`] is.
const ROLLOUT: &str = "rollout-";
const ROLLOUT_SUFFIX: &str = ".jsonl";

/// A moment as the coarsest filesystem would have recorded it — the whole
/// second it fell in.
///
/// What the finder compares a rollout's modification time against, and that
/// comparison is only as fine as the clock the filesystem stamped it with. A
/// store on a filesystem keeping whole seconds would stamp a rollout written a
/// moment after launch with the second the session started in, and the
/// session's own log would then look older than the session.
///
/// A second of slack costs nothing. What it can let in is the rollout of an
/// earlier session in the same Worktree that ended in the very second this one
/// started, and [`rollout`] takes the newest of what it finds anyway.
fn to_the_second(moment: SystemTime) -> SystemTime {
    match moment.duration_since(SystemTime::UNIX_EPOCH) {
        Ok(since) => SystemTime::UNIX_EPOCH + Duration::from_secs(since.as_secs()),
        Err(_) => moment,
    }
}

/// The log a Grok session keeps of itself: the `updates.jsonl` inside the
/// directory named for the session id Verkstead gave it.
///
/// Grok organises its store by working directory and then by session — one
/// directory per directory it has been run in, and one per session inside that.
/// **What it calls the outer one is grok's own encoding of the path**: URL-
/// encoded where that fits and a slug with a hash of it where it does not, with
/// the original path left in a `.cwd` file beside the sessions. Working out
/// which of those a Worktree's path would have come out as means reimplementing
/// somebody else's private scheme (ADR 0006), and it would come apart the first
/// time either half of it moved.
///
/// So the store's own directories are what say where to look, and the session id
/// is what identifies the log inside them: one level of walking, and a name
/// Verkstead chose at the end of it. Beside them as well as inside one, for the
/// reason Claude's log is looked for both places — whether grok grouped this
/// session under an encoded directory at all is grok's business, and what
/// Verkstead knows is the store and the name it gave the session.
///
/// `None` while the session has not written it yet, which is every poll of its
/// first seconds: grok makes the directory when it starts talking rather than
/// when it starts.
async fn updates(sessions: &Path, session: &str) -> Option<PathBuf> {
    let beside = sessions.join(session).join(UPDATES);

    if is_file(&beside).await {
        return Some(beside);
    }

    for encoded in directories(sessions).await {
        let inside = encoded.join(session).join(UPDATES);

        if is_file(&inside).await {
            return Some(inside);
        }
    }

    None
}

/// The directories directly under `under`, and nothing else that is there.
async fn directories(under: &Path) -> Vec<PathBuf> {
    let Ok(mut entries) = tokio::fs::read_dir(under).await else {
        return Vec::new();
    };

    let mut directories = Vec::new();

    while let Ok(Some(entry)) = entries.next_entry().await {
        if entry.file_type().await.is_ok_and(|kind| kind.is_dir()) {
            directories.push(entry.path());
        }
    }

    directories
}

/// When `path` was last written to, asked without blocking the loop doing the
/// asking.
async fn written(path: &Path) -> Option<SystemTime> {
    tokio::fs::metadata(path).await.ok()?.modified().ok()
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

    /// A rollout as codex opens one, naming the directory the session was
    /// launched in.
    fn meta(cwd: &Path) -> String {
        format!(
            r#"{{"timestamp":"2026-08-30T07:47:01.017Z","ordinal":0,"type":"session_meta","payload":{{"session_id":"01a051a2","cwd":"{}"}}}}"#,
            cwd.display()
        )
    }

    /// One written into the day directory of a session store, under a name
    /// codex would have given it.
    fn wrote(sessions: &Path, day: &str, uuid: &str, cwd: &Path) -> PathBuf {
        let directory = sessions.join(day);
        std::fs::create_dir_all(&directory).unwrap();

        let log = directory.join(format!("rollout-2026-08-30T17-47-00-{uuid}.jsonl"));
        std::fs::write(&log, format!("{}\n", meta(cwd))).unwrap();

        log
    }

    /// A moment before anything the test wrote, which is what a session
    /// launched before its own log has.
    fn a_moment_ago() -> SystemTime {
        SystemTime::now() - Duration::from_secs(60)
    }

    /// A file or a directory made to look as though it was last written to `by`
    /// ago, which is what the store of a machine with history on it holds.
    fn aged(path: &Path, by: Duration) {
        let times = std::fs::FileTimes::new().set_modified(SystemTime::now() - by);

        std::fs::File::open(path).unwrap().set_times(times).unwrap();
    }

    /// Two Codex sessions started near-together in two Worktrees write their
    /// rollouts into one store, and each finder takes its own.
    ///
    /// Which is the whole of what identifying a rollout is for. Verkstead runs
    /// as many sessions at once as the machine will take, and a Profile is one
    /// account — so two of them writing into the same day of the same store,
    /// seconds apart, is the ordinary case rather than the awkward one.
    #[tokio::test]
    async fn two_sessions_in_two_worktrees_each_find_their_own_rollout() {
        let store = tempfile::tempdir().unwrap();
        let sessions = store.path().join("sessions");

        let launched = a_moment_ago();

        let rate = PathBuf::from("/srv/worktrees/rate-limiting");
        let tables = PathBuf::from("/srv/worktrees/tables");

        let of_rate = wrote(&sessions, "2026/08/30", "aaaa", &rate);
        let of_tables = wrote(&sessions, "2026/08/30", "bbbb", &tables);

        assert_eq!(
            rollout(&sessions, &rate, launched).await,
            Some(of_rate),
            "the session in the rate-limiting Worktree should follow the rollout \
             naming it"
        );
        assert_eq!(
            rollout(&sessions, &tables, launched).await,
            Some(of_tables),
            "and the one beside it should follow the other"
        );
    }

    /// And a Worktree worked in before is not this session's log: the same
    /// Conversation resumed is a second session in the same directory, and what
    /// tells the two apart is when each of their logs appeared.
    #[tokio::test]
    async fn a_rollout_written_before_the_session_started_is_a_previous_sessions() {
        let store = tempfile::tempdir().unwrap();
        let sessions = store.path().join("sessions");
        let worktree = PathBuf::from("/srv/worktrees/rate-limiting");

        let earlier = wrote(&sessions, "2026/08/30", "aaaa", &worktree);

        // Only the log is made old. The day directory is left as new as this
        // session, because a session launched today lands in a directory
        // today's earlier sessions already made.
        let launched = to_the_second(SystemTime::now());
        aged(&earlier, Duration::from_secs(600));

        assert_eq!(
            rollout(&sessions, &worktree, launched).await,
            None,
            "a log that was already there when the session started is not the log \
             the session is writing"
        );

        let now = wrote(&sessions, "2026/08/30", "bbbb", &worktree);

        assert_eq!(
            rollout(&sessions, &worktree, launched).await,
            Some(now),
            "and the one that appeared after it is"
        );
    }

    /// The store holds more than the logs being written — codex compresses the
    /// older ones and keeps an index of them in SQLite beside them — and a
    /// session whose own rollout is not there yet finds none of it.
    #[tokio::test]
    async fn nothing_in_the_store_but_a_rollout_is_taken_for_one() {
        let store = tempfile::tempdir().unwrap();
        let sessions = store.path().join("sessions");
        let worktree = PathBuf::from("/srv/worktrees/rate-limiting");

        let day = sessions.join("2026/08/30");
        std::fs::create_dir_all(&day).unwrap();

        for beside in [
            "rollout-2026-08-30T17-47-00-aaaa.jsonl.zst",
            "rollout-2026-08-30T17-47-00-aaaa.jsonl.gz",
            "sessions.sqlite",
            "sessions.sqlite-wal",
        ] {
            std::fs::write(day.join(beside), format!("{}\n", meta(&worktree))).unwrap();
        }

        assert_eq!(
            rollout(&sessions, &worktree, a_moment_ago()).await,
            None,
            "only a plain JSONL rollout is a log to follow"
        );
    }

    /// A rollout whose first line has not been finished yet is one that cannot
    /// be identified yet, and the poll after it asks again.
    #[tokio::test]
    async fn a_rollout_caught_mid_first_line_is_read_again_rather_than_guessed_at() {
        let store = tempfile::tempdir().unwrap();
        let sessions = store.path().join("sessions");
        let worktree = PathBuf::from("/srv/worktrees/rate-limiting");

        let day = sessions.join("2026/08/30");
        std::fs::create_dir_all(&day).unwrap();

        let log = day.join("rollout-2026-08-30T17-47-00-aaaa.jsonl");
        let whole = meta(&worktree);
        std::fs::write(&log, &whole[..40]).unwrap();

        assert_eq!(rollout(&sessions, &worktree, a_moment_ago()).await, None);

        std::fs::write(&log, format!("{whole}\n")).unwrap();

        assert_eq!(
            rollout(&sessions, &worktree, a_moment_ago()).await,
            Some(log)
        );
    }

    /// The store is filed by date, and which date is codex's own business —
    /// what the finder reads is which day directory has been written into since
    /// the session started, rather than what any of them is called.
    #[tokio::test]
    async fn a_rollout_is_found_whatever_day_codex_filed_it_under() {
        let store = tempfile::tempdir().unwrap();
        let sessions = store.path().join("sessions");
        let worktree = PathBuf::from("/srv/worktrees/rate-limiting");

        // Every day but this one left older than the session, which is what a
        // store with history in it looks like.
        let launched = to_the_second(SystemTime::now());
        let a_day = Duration::from_secs(86_400);

        for old in ["2025/12/31", "2026/07/14", "2026/08/29"] {
            let day = sessions.join(old);
            std::fs::create_dir_all(&day).unwrap();
            aged(&day, a_day);
        }

        let tomorrow = wrote(&sessions, "2026/08/31", "aaaa", &worktree);

        assert_eq!(
            rollout(&sessions, &worktree, launched).await,
            Some(tomorrow)
        );
    }

    /// A Grok session's log where grok keeps one: `updates.jsonl` inside a
    /// directory named for the session, inside the directory grok named by
    /// encoding the working directory.
    fn kept(sessions: &Path, encoded: &str, session: &str) -> PathBuf {
        let directory = sessions.join(encoded).join(session);
        std::fs::create_dir_all(&directory).unwrap();

        let log = directory.join("updates.jsonl");
        std::fs::write(&log, format!("{A_LINE}\n")).unwrap();

        log
    }

    /// One line of one, as grok writes them: the stream of session updates its
    /// own agent protocol is spoken in. What any of it means is the renderer's,
    /// and nothing here reads a line at all.
    const A_LINE: &str = r#"{"sessionUpdate":"agent_message_chunk"}"#;

    /// Two Grok sessions running at once in two Worktrees write into the one
    /// store their account keeps, and each finder takes its own.
    ///
    /// Which is what naming a session at launch buys: the two logs are told
    /// apart by the name Verkstead gave each of them, so neither the directory
    /// grok grouped them under nor the moment either started has to be read.
    #[tokio::test]
    async fn two_sessions_in_two_worktrees_each_find_their_own_log() {
        let store = tempfile::tempdir().unwrap();
        let sessions = store.path().join("sessions");

        let rate = "%2Fsrv%2Fworktrees%2Frate-limiting";
        let tables = "%2Fsrv%2Fworktrees%2Ftables";

        let of_rate = kept(&sessions, rate, "aaaa-1111");
        let of_tables = kept(&sessions, tables, "bbbb-2222");

        // And the session that worked in the rate-limiting Worktree yesterday,
        // beside today's in the same directory: one Worktree is worked in
        // session after session, and only one of them is this one.
        let yesterday = kept(&sessions, rate, "cccc-3333");

        assert_eq!(
            updates(&sessions, "aaaa-1111").await,
            Some(of_rate),
            "the log under the name Verkstead gave this session is the one followed"
        );
        assert_eq!(
            updates(&sessions, "bbbb-2222").await,
            Some(of_tables),
            "and the session beside it follows its own, in the directory grok \
             grouped that Worktree under"
        );
        assert!(
            yesterday.is_file(),
            "the earlier session's log is still there and was followed by neither"
        );
    }

    /// What the directory a session is grouped under is called is grok's own
    /// encoding of the working directory, and nothing here reads it: a path that
    /// URL-encodes to more than grok's limit is kept under a slug and a hash
    /// instead, with the path itself left in a `.cwd` file beside the sessions.
    ///
    /// So the store's directories are walked rather than the encoding
    /// reproduced, and a session grouped under no directory at all is found the
    /// same way — which of the two grok does is grok's business.
    #[tokio::test]
    async fn a_log_is_found_whatever_grok_encoded_the_working_directory_as() {
        let store = tempfile::tempdir().unwrap();
        let sessions = store.path().join("sessions");

        let hashed = "srv-worktrees-verkstead-rate-limiting-8f2c1d9e";
        let log = kept(&sessions, hashed, "aaaa-1111");
        std::fs::write(
            sessions.join(hashed).join(".cwd"),
            "/srv/worktrees/verkstead-rate-limiting\n",
        )
        .unwrap();

        assert_eq!(
            updates(&sessions, "aaaa-1111").await,
            Some(log),
            "a group directory named however grok had to name it holds the log all \
             the same"
        );

        let beside = sessions.join("bbbb-2222");
        std::fs::create_dir_all(&beside).unwrap();
        let unencoded = beside.join("updates.jsonl");
        std::fs::write(&unencoded, "{}\n").unwrap();

        assert_eq!(
            updates(&sessions, "bbbb-2222").await,
            Some(unencoded),
            "and a session the store held directly is found as well"
        );
    }

    /// Nothing in the session's own directory but the log is taken for the log.
    ///
    /// A Grok session fills that directory with the store's own furniture — the
    /// index entry, the raw chat history it sent the model, its plan, its rewind
    /// points — and one file of it is the conversation.
    #[tokio::test]
    async fn nothing_beside_the_log_is_taken_for_it() {
        let store = tempfile::tempdir().unwrap();
        let sessions = store.path().join("sessions");

        let directory = sessions.join("%2Fsrv%2Fworktrees%2Ftables/aaaa-1111");
        std::fs::create_dir_all(&directory).unwrap();

        for beside in [
            "summary.json",
            "chat_history.jsonl",
            "plan.json",
            "rewind_points.jsonl",
            "signals.json",
        ] {
            std::fs::write(directory.join(beside), "{}\n").unwrap();
        }

        std::fs::create_dir_all(directory.join("subagents")).unwrap();

        assert_eq!(
            updates(&sessions, "aaaa-1111").await,
            None,
            "the conversation is `updates.jsonl` and nothing else in there is it"
        );
    }

    /// And a session that has not written anything yet is a session with nothing
    /// to follow, which is every poll of its first seconds.
    #[tokio::test]
    async fn a_session_that_has_written_nothing_yet_has_no_log_to_follow() {
        let store = tempfile::tempdir().unwrap();
        let sessions = store.path().join("sessions");

        assert_eq!(
            updates(&sessions, "aaaa-1111").await,
            None,
            "a store that has never been written in holds no log"
        );

        let log = kept(&sessions, "%2Fsrv%2Fworktrees%2Ftables", "aaaa-1111");

        assert_eq!(
            updates(&sessions, "aaaa-1111").await,
            Some(log),
            "and the poll after the session started talking finds it"
        );
    }
}
