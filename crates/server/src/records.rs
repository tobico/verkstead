//! The store an OpenCode session keeps of itself, read from outside the program
//! that is writing it.
//!
//! Three backends before this one keep a log: a file of lines, appended to
//! while the session runs, followed by remembering a byte offset and splitting
//! whole lines off the end of what has arrived since (see [`crate::transcript`]).
//! opencode keeps a database instead — one per account, under the account's own
//! data directory, in write-ahead-log mode — with a row per session and a row
//! per record within it. None of a byte offset survives that.
//!
//! What does survive is the idea. The store numbers every record within its
//! session, so the cursor is the highest sequence already taken and each poll
//! takes what has arrived past it. That is the file follower's bargain in the
//! store's own terms, and it is the whole of what following means here.
//!
//! **ADR-0006 is unchanged, which is why it survives the change of medium.**
//! Every record's payload is JSON, so a record reaches the Transcript verbatim
//! and is parsed at render time exactly as a Claude line or a Codex rollout line
//! is. What is stored is the payload byte for byte, with the record's own kind
//! and its place in the session's sequence around it — the two things the
//! renderer needs and nothing this invented on the way in. Nothing here parses
//! a payload.
//!
//! **The session is found rather than named.** opencode takes no session id at
//! launch, so nothing Verkstead knows beforehand names its session; what
//! identifies it is what the session wrote about itself. That is the directory
//! it opened in — the Conversation's Worktree, bound into the sandbox at the
//! path it has outside one, so the two are the same string — and the moment it
//! was created. The rule is the Codex rollout finder's; only the mechanism
//! differs.
//!
//! **Read as an outsider.** The database belongs to a program that is writing
//! it while this reads, so it is opened read-only and never written to. A poll
//! that cannot read is a poll that looks again on the next cadence, rather than
//! a failure: the store is not there for the first seconds of a session, and
//! the writer holds the lock from time to time after that.
//!
//! **And a store this build cannot read leaves the session Capture-only.** The
//! layout is opencode's own and it moves between releases — a table renamed, a
//! column gone. None of that may fail a session, because the Capture is a
//! complete record on its own and a session with no Transcript has always been
//! an ordinary thing here (ADR 0006). So it is the deliberate failure mode:
//! this stops reading, says so in the log, and the session runs on untouched.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use serde::Serialize;
use serde_json::value::RawValue;
use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

/// One OpenCode session's records, read as far as they have been written.
pub(crate) struct Reader {
    /// The store itself: the database under the Profile's data directory, whose
    /// name the sandbox pinned so that this opens a file Verkstead chose rather
    /// than guessing which of opencode's several it is — see
    /// [`crate::sandbox`].
    database: PathBuf,

    /// The Worktree this Conversation's session was launched in, which is half
    /// of what says which session in the store is this one.
    worktree: PathBuf,

    /// And the moment it was launched, which is the other half — as
    /// milliseconds since the epoch, which is how the store keeps its own
    /// times.
    launched: i64,

    /// The connection, once one has been made. Lazy, so that a store which is
    /// not there yet costs nothing and an unreachable one fails at the query
    /// rather than at the open.
    reading: Option<SqlitePool>,

    /// Which of the store's sessions is this one, once it has been found.
    ///
    /// `None` for the first polls of a session's life, which is the store not
    /// existing yet and then the session row not being in it yet.
    session: Option<String>,

    /// The highest sequence number already taken, which is the cursor.
    taken: i64,

    /// Whether this store turned out to be a shape this build cannot read, in
    /// which case nothing is asked of it again and the Capture is the whole
    /// record.
    unreadable: bool,
}

/// What the cursor is before anything has been taken.
///
/// One below the store's first record rather than zero: opencode numbers a
/// session's records from zero, so zero is a record that has arrived rather
/// than a session nothing has been read from.
const NOTHING_TAKEN: i64 = -1;

impl Reader {
    /// Read the session that opened in `worktree` at or after `launched`, out
    /// of the store at `database`.
    pub(crate) fn of(database: PathBuf, worktree: PathBuf, launched: SystemTime) -> Reader {
        Reader {
            database,
            worktree,
            launched: to_the_second(launched),
            reading: None,
            session: None,
            taken: NOTHING_TAKEN,
            unreadable: false,
        }
    }

    /// Take whatever the session has written since the last poll, as the lines
    /// it goes on the Transcript as.
    ///
    /// Empty is the ordinary answer rather than a fault: a store that is not
    /// there yet, a session that has not been created in it yet, a poll the
    /// writer's lock came out on top of, and a session that simply said nothing
    /// in the last half-second all come back with nothing.
    pub(crate) async fn take(&mut self) -> Vec<String> {
        if self.unreadable {
            return Vec::new();
        }

        match self.arrived().await {
            Ok(arrived) => arrived,
            Err(error) if unknown_shape(&error) => {
                self.unreadable = true;

                tracing::warn!(
                    error = ?error,
                    database = %self.database.display(),
                    "this OpenCode session's store is not a shape this build can read, so \
                     the session's own record is its Capture alone",
                );

                Vec::new()
            }
            Err(error) => {
                // The writer holding the lock, or the file half-made. Both are
                // conditions of the moment rather than of the store, and the
                // next poll asks again.
                tracing::debug!(
                    error = ?error,
                    database = %self.database.display(),
                    "an OpenCode session's store could not be read this poll",
                );

                Vec::new()
            }
        }
    }

    /// The same, with what went wrong left to the caller to judge.
    async fn arrived(&mut self) -> Result<Vec<String>, sqlx::Error> {
        if !is_file(&self.database).await {
            // opencode makes its store when it starts rather than when it is
            // installed, and a Profile that has never run a session has none.
            // Nothing to say about it: the next poll looks again.
            return Ok(Vec::new());
        }

        let reading = match &self.reading {
            Some(reading) => reading.clone(),
            None => self.reading.insert(opened(&self.database)).clone(),
        };

        if self.session.is_none() {
            self.session = whose(&reading, &self.worktree, self.launched).await?;

            if let Some(session) = &self.session {
                tracing::debug!(
                    session,
                    worktree = %self.worktree.display(),
                    "an OpenCode session was found in its account's store",
                );
            }
        }

        let Some(session) = self.session.clone() else {
            return Ok(Vec::new());
        };

        let arrived = past(&reading, &session, self.taken).await?;

        if let Some((seq, _, _)) = arrived.last() {
            self.taken = *seq;
        }

        Ok(arrived
            .iter()
            .map(|(seq, kind, payload)| record(*seq, kind, payload))
            .collect())
    }
}

/// Open the store to read it and nothing else.
///
/// **Read-only, and never opened eagerly.** Read-only because the database is
/// somebody else's and Verkstead is a reader of it (ADR 0006); lazily because
/// the store of a Profile whose first session is still starting does not exist
/// yet, and a connection made when there is something to read is one fewer
/// thing to get wrong when there is not.
///
/// One connection, because one poll of one session reads it and they are half a
/// second apart. And short waits on both halves of getting that connection,
/// because the loop that polls this is the loop that flushes the session's
/// Capture: a poll that waited out the default would stop a watched terminal
/// from moving for as long as it waited, where a poll that gives up asks again
/// on the next cadence and costs half a second.
fn opened(database: &Path) -> SqlitePool {
    let options = SqliteConnectOptions::new()
        .filename(database)
        .read_only(true)
        .create_if_missing(false)
        .busy_timeout(WAIT_FOR_THE_WRITER);

    SqlitePoolOptions::new()
        .max_connections(1)
        .acquire_timeout(WAIT_FOR_THE_WRITER)
        .connect_lazy_with(options)
}

/// How long a poll waits on a store somebody else is writing before it gives up
/// and asks again.
///
/// Under the cadence it is polled on, so that a poll blocked all the way
/// through is a poll that has finished by the time the next one is due.
const WAIT_FOR_THE_WRITER: Duration = Duration::from_millis(250);

/// Which of the store's sessions is the one launched in `worktree` at or after
/// `launched`.
///
/// Two tests rather than one, because neither is enough on its own — this is
/// the Codex rollout finder's rule, said in SQL. The Worktree alone would find
/// whatever earlier session last worked in it, and the moment alone would find
/// whichever Conversation happened to start a session at the same time.
/// Together they are one session: one Conversation per Worktree and one session
/// per Conversation.
///
/// The newest of them where more than one matches, for the reason the rollout
/// finder takes the newest — see [`to_the_second`].
///
/// **And the session opencode started for itself, rather than one it started
/// under that.** A sub-agent's session records the same directory and is newer
/// than the session that spawned it, so it would win a race it is not in.
async fn whose(
    reading: &SqlitePool,
    worktree: &Path,
    launched: i64,
) -> Result<Option<String>, sqlx::Error> {
    // opencode's own spelling of its own store, which is why it is written out
    // rather than built: what this asks for is the columns that program named,
    // and the day one of them moves is the day this build stops recognising the
    // store — see [`unknown_shape`].
    let found: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM session
          WHERE directory = ? AND parent_id IS NULL AND time_created >= ?
          ORDER BY time_created DESC, id DESC
          LIMIT 1",
    )
    .bind(worktree.to_string_lossy().into_owned())
    .bind(launched)
    .fetch_optional(reading)
    .await?;

    Ok(found.map(|(id,)| id))
}

/// The records of `session` past the sequence number `taken`, in the order the
/// session wrote them.
///
/// Everything that has arrived rather than a page of it: what bounds a poll is
/// that the one before it was half a second ago.
async fn past(
    reading: &SqlitePool,
    session: &str,
    taken: i64,
) -> Result<Vec<(i64, String, String)>, sqlx::Error> {
    sqlx::query_as(
        "SELECT seq, type, data FROM event
          WHERE aggregate_id = ? AND seq > ?
          ORDER BY seq",
    )
    .bind(session)
    .bind(taken)
    .fetch_all(reading)
    .await
}

/// One record as it goes on the Transcript.
///
/// The payload exactly as the store holds it, with the two things around it
/// that a reader of the Transcript cannot work out for itself: the kind
/// opencode filed the record under, and its place in the session's sequence.
///
/// **The payload is spliced rather than parsed and written out again.** JSON
/// read into a value and serialised back is the same document and not the same
/// bytes — the keys come back in another order — and verbatim is the whole
/// bargain ADR 0006 makes. A payload that is not JSON at all is kept as the
/// text it is, for the other half of that bargain: a record can arrive in a
/// shape nothing knows how to draw, and it can never be lost on the way in.
fn record(seq: i64, kind: &str, payload: &str) -> String {
    let quoted;

    let payload = match serde_json::from_str::<&RawValue>(payload) {
        Ok(payload) => payload,
        Err(_) => {
            quoted = serde_json::value::to_raw_value(payload)
                .expect("a string is a JSON document of its own");

            &quoted
        }
    };

    serde_json::to_string(&Record {
        kind,
        seq,
        record: payload,
    })
    .expect("a record of borrowed JSON serialises")
}

/// The line a record is written as — and the only thing this crate says about
/// the shape of an OpenCode Transcript, everything else about it being the
/// renderer's.
///
/// No `type` of its own, deliberately. The three backends before this one are
/// told apart by what their own lines carry, so a key of that name here would
/// be a fourth backend answering to the same question with somebody else's
/// vocabulary — see the render crate's `transcript` module.
#[derive(Serialize)]
struct Record<'a> {
    kind: &'a str,
    seq: i64,
    record: &'a RawValue,
}

/// Whether what came back says the store is a shape this build does not know,
/// rather than one it could not read at this moment.
///
/// The distinction is the whole of the Capture-only rule: a store that is
/// locked, half-made or not there yet is looked at again on the next poll, and
/// a store whose tables are not the tables this asks about is one to stop
/// asking. Both arrive as an error and only one of them is final.
///
/// **SQLite's own answer is what decides it.** A statement this build knows is
/// well formed, refused by the database itself — a table renamed, a column gone
/// — comes back under the generic error code, and every other code is a
/// condition rather than a shape: the file is not there, the writer holds the
/// lock, the disk said no. A column that is there but holds something this
/// cannot read out of it is the same finding said by the driver instead.
fn unknown_shape(error: &sqlx::Error) -> bool {
    match error {
        sqlx::Error::Database(refused) => primary(refused.code().as_deref()) == Some(SQLITE_ERROR),
        sqlx::Error::ColumnDecode { .. } | sqlx::Error::ColumnNotFound(_) => true,
        _ => false,
    }
}

/// The primary result code inside an extended one, which is its low byte.
///
/// The driver hands the code back as the decimal text of SQLite's *extended*
/// result code, and the extended codes are the primary one with a reason in the
/// high bits. What is being asked here is the primary question.
fn primary(code: Option<&str>) -> Option<i32> {
    Some(code?.parse::<i32>().ok()? & 0xff)
}

/// SQLite's generic error, which is what a statement no schema can answer comes
/// back as.
const SQLITE_ERROR: i32 = 1;

/// A moment in the milliseconds the store keeps its own times in, floored to
/// the whole second it fell in.
///
/// The second of slack is the rollout finder's, for the same reason and against
/// a different clock: there, a coarse filesystem stamps a log written a moment
/// after launch with the second the session started in. Here the two clocks are
/// the same machine's, and the slack costs nothing while keeping the two
/// finders reading alike. What it can let in is a session of this Worktree's
/// that ended in the very second this one started, and the finder takes the
/// newest of what it finds anyway.
fn to_the_second(moment: SystemTime) -> i64 {
    match moment.duration_since(SystemTime::UNIX_EPOCH) {
        Ok(since) => (since.as_secs() as i64) * 1_000,
        // A clock before the epoch is a machine with no clock set. Nothing in
        // the store can be older than that, so everything in it is this
        // session's to consider and the Worktree is left to say which.
        Err(_) => 0,
    }
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
    use sqlx::Executor;
    use sqlx::sqlite::SqliteJournalMode;

    use super::*;

    /// A store shaped the way opencode 1.18.25 shapes one, as far as this reads
    /// it: the row per session, and the row per record within it.
    ///
    /// Only the columns this asks for. What the rest of that database holds is
    /// opencode's business, and a fixture that copied it would be this build
    /// claiming to know more of somebody else's schema than it reads.
    async fn store(at: &Path) -> SqlitePool {
        let writing = SqlitePool::connect_with(
            SqliteConnectOptions::new()
                .filename(at)
                .create_if_missing(true)
                // The mode opencode keeps its own store in, so that what these
                // read is a database of the shape the real one is.
                .journal_mode(SqliteJournalMode::Wal),
        )
        .await
        .unwrap();

        writing
            .execute(
                "CREATE TABLE session (
                     id           TEXT PRIMARY KEY,
                     parent_id    TEXT,
                     directory    TEXT NOT NULL,
                     time_created INTEGER NOT NULL
                 );
                 CREATE TABLE event (
                     id           TEXT PRIMARY KEY,
                     aggregate_id TEXT NOT NULL,
                     seq          INTEGER NOT NULL,
                     type         TEXT NOT NULL,
                     data         TEXT NOT NULL
                 );",
            )
            .await
            .unwrap();

        writing
    }

    /// A session opencode opened in `directory` at `created`.
    async fn opened(writing: &SqlitePool, session: &str, directory: &Path, created: i64) {
        sqlx::query(
            "INSERT INTO session (id, parent_id, directory, time_created) VALUES (?, NULL, ?, ?)",
        )
        .bind(session)
        .bind(directory.to_string_lossy().into_owned())
        .bind(created)
        .execute(writing)
        .await
        .unwrap();
    }

    /// And a record it wrote inside one.
    async fn wrote(writing: &SqlitePool, session: &str, seq: i64, kind: &str, data: &str) {
        sqlx::query("INSERT INTO event (id, aggregate_id, seq, type, data) VALUES (?, ?, ?, ?, ?)")
            .bind(format!("{session}-{seq}"))
            .bind(session)
            .bind(seq)
            .bind(kind)
            .bind(data)
            .execute(writing)
            .await
            .unwrap();
    }

    /// The moment a session under test was launched, and the same moment in the
    /// milliseconds the store keeps.
    fn launched() -> (SystemTime, i64) {
        let launched = SystemTime::now();
        let millis = launched
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        (launched, millis)
    }

    /// Two OpenCode sessions started near-together in two Worktrees write into
    /// one store, and each reader takes its own.
    ///
    /// Which is the whole of what identifying a session is for. Verkstead runs
    /// as many sessions at once as the machine will take, and a Profile is one
    /// account — so two of them writing into the same database, seconds apart,
    /// is the ordinary case rather than the awkward one.
    #[tokio::test]
    async fn two_sessions_in_two_worktrees_each_take_their_own_records() {
        let account = tempfile::tempdir().unwrap();
        let database = account.path().join("opencode.db");
        let writing = store(&database).await;

        let (launched, now) = launched();

        let rate = PathBuf::from("/srv/worktrees/rate-limiting");
        let tables = PathBuf::from("/srv/worktrees/tables");

        opened(&writing, "ses_rate", &rate, now).await;
        opened(&writing, "ses_tables", &tables, now + 20).await;

        wrote(
            &writing,
            "ses_rate",
            0,
            "session.created.1",
            r#"{"of":"rate"}"#,
        )
        .await;
        wrote(
            &writing,
            "ses_tables",
            0,
            "session.created.1",
            r#"{"of":"tables"}"#,
        )
        .await;

        let mut of_rate = Reader::of(database.clone(), rate, launched);
        let mut of_tables = Reader::of(database, tables, launched);

        assert_eq!(
            of_rate.take().await,
            vec![r#"{"kind":"session.created.1","seq":0,"record":{"of":"rate"}}"#.to_owned()],
            "the session in the rate-limiting Worktree should take the records of \
             the session opened there",
        );
        assert_eq!(
            of_tables.take().await,
            vec![r#"{"kind":"session.created.1","seq":0,"record":{"of":"tables"}}"#.to_owned()],
            "and the one beside it should take the other's",
        );
    }

    /// And a Worktree worked in before is not this session's: the same
    /// Conversation resumed is a second session in the same directory, and what
    /// tells the two apart is when each of them was created.
    #[tokio::test]
    async fn a_session_from_before_this_one_started_is_not_the_one_followed() {
        let account = tempfile::tempdir().unwrap();
        let database = account.path().join("opencode.db");
        let writing = store(&database).await;

        let (launched, now) = launched();
        let worktree = PathBuf::from("/srv/worktrees/rate-limiting");

        opened(&writing, "ses_earlier", &worktree, now - 600_000).await;
        wrote(
            &writing,
            "ses_earlier",
            0,
            "session.created.1",
            r#"{"of":"earlier"}"#,
        )
        .await;

        let mut reader = Reader::of(database.clone(), worktree.clone(), launched);

        assert!(
            reader.take().await.is_empty(),
            "a session that was already in the store when this one started is not \
             the session this one is writing",
        );

        opened(&writing, "ses_now", &worktree, now).await;
        wrote(
            &writing,
            "ses_now",
            0,
            "session.created.1",
            r#"{"of":"now"}"#,
        )
        .await;

        assert_eq!(
            reader.take().await,
            vec![r#"{"kind":"session.created.1","seq":0,"record":{"of":"now"}}"#.to_owned()],
            "and the one created after it is",
        );
    }

    /// A session opencode started under this one is not this one. It records the
    /// same directory and is newer, so nothing but its parentage keeps it out of
    /// a race it is not in.
    #[tokio::test]
    async fn a_session_started_under_this_one_is_not_the_one_followed() {
        let account = tempfile::tempdir().unwrap();
        let database = account.path().join("opencode.db");
        let writing = store(&database).await;

        let (launched, now) = launched();
        let worktree = PathBuf::from("/srv/worktrees/rate-limiting");

        opened(&writing, "ses_own", &worktree, now).await;
        wrote(
            &writing,
            "ses_own",
            0,
            "session.created.1",
            r#"{"of":"own"}"#,
        )
        .await;

        sqlx::query(
            "INSERT INTO session (id, parent_id, directory, time_created) VALUES (?, ?, ?, ?)",
        )
        .bind("ses_under")
        .bind("ses_own")
        .bind(worktree.to_string_lossy().into_owned())
        .bind(now + 1_000)
        .execute(&writing)
        .await
        .unwrap();

        let mut reader = Reader::of(database, worktree, launched);

        assert_eq!(
            reader.take().await,
            vec![r#"{"kind":"session.created.1","seq":0,"record":{"of":"own"}}"#.to_owned()],
            "the session Verkstead launched is the one followed, rather than the \
             newer one it started under itself",
        );
    }

    /// The cursor: each poll takes what has arrived past the highest sequence
    /// already taken, and nothing it has taken before.
    #[tokio::test]
    async fn a_second_poll_takes_only_the_records_that_arrived_since_the_first() {
        let account = tempfile::tempdir().unwrap();
        let database = account.path().join("opencode.db");
        let writing = store(&database).await;

        let (launched, now) = launched();
        let worktree = PathBuf::from("/srv/worktrees/rate-limiting");

        opened(&writing, "ses_own", &worktree, now).await;
        wrote(&writing, "ses_own", 0, "session.created.1", r#"{"at":0}"#).await;
        wrote(&writing, "ses_own", 1, "message.updated.1", r#"{"at":1}"#).await;

        let mut reader = Reader::of(database, worktree, launched);

        assert_eq!(
            reader.take().await,
            vec![
                r#"{"kind":"session.created.1","seq":0,"record":{"at":0}}"#.to_owned(),
                r#"{"kind":"message.updated.1","seq":1,"record":{"at":1}}"#.to_owned(),
            ],
            "the first poll takes the session's record from its beginning, which \
             is the record numbered zero rather than the one numbered one",
        );

        assert!(
            reader.take().await.is_empty(),
            "a poll that arrived before the session said anything more takes \
             nothing rather than the record again",
        );

        wrote(
            &writing,
            "ses_own",
            2,
            "message.part.updated.1",
            r#"{"at":2}"#,
        )
        .await;

        assert_eq!(
            reader.take().await,
            vec![r#"{"kind":"message.part.updated.1","seq":2,"record":{"at":2}}"#.to_owned()],
            "and the poll after that takes what arrived since, and only that",
        );
    }

    /// A payload reaches the Transcript byte for byte, keys in the order the
    /// store wrote them, which is what parsing it and writing it out again would
    /// have lost.
    #[test]
    fn a_records_payload_goes_on_the_transcript_as_the_store_holds_it() {
        let payload =
            r#"{"sessionID":"ses_own","part":{"type":"text","text":"Reading the brief."}}"#;

        assert_eq!(
            record(9, "message.part.updated.1", payload),
            format!(r#"{{"kind":"message.part.updated.1","seq":9,"record":{payload}}}"#),
        );
    }

    /// And a payload that is not JSON at all is kept as the text it is. A record
    /// can arrive in a shape nothing knows how to draw; it can never be lost on
    /// the way in (ADR 0006).
    #[test]
    fn a_payload_that_is_not_json_is_kept_rather_than_dropped() {
        assert_eq!(
            record(9, "message.updated.1", "not json at all"),
            r#"{"kind":"message.updated.1","seq":9,"record":"not json at all"}"#,
        );
    }

    /// A store of a shape this build does not know leaves the session
    /// Capture-only: nothing is taken, nothing is asked of it again, and the
    /// session itself is untouched.
    #[tokio::test]
    async fn a_store_this_build_cannot_read_leaves_the_session_capture_only() {
        let account = tempfile::tempdir().unwrap();
        let database = account.path().join("opencode.db");

        let writing = SqlitePool::connect_with(
            SqliteConnectOptions::new()
                .filename(&database)
                .create_if_missing(true),
        )
        .await
        .unwrap();

        // A release that renamed the column the directory is recorded under,
        // which is the whole class: the database is there and readable, and the
        // question this asks of it has no answer.
        writing
            .execute(
                "CREATE TABLE session (
                     id           TEXT PRIMARY KEY,
                     parent_id    TEXT,
                     cwd          TEXT NOT NULL,
                     time_created INTEGER NOT NULL
                 )",
            )
            .await
            .unwrap();

        let (launched, _) = launched();
        let mut reader = Reader::of(
            database,
            PathBuf::from("/srv/worktrees/rate-limiting"),
            launched,
        );

        assert!(
            reader.take().await.is_empty(),
            "a store whose shape this build does not know hands back no records",
        );
        assert!(
            reader.unreadable,
            "and is not asked again, the Capture being the session's whole record \
             from here",
        );
    }

    /// Where a store that is not there yet is asked again on the next poll. That
    /// is every session's first seconds — opencode writes its database when it
    /// starts rather than when it is installed.
    #[tokio::test]
    async fn a_store_that_is_not_there_yet_is_looked_at_again() {
        let account = tempfile::tempdir().unwrap();
        let database = account.path().join("opencode.db");

        let (launched, now) = launched();
        let worktree = PathBuf::from("/srv/worktrees/rate-limiting");

        let mut reader = Reader::of(database.clone(), worktree.clone(), launched);

        assert!(
            reader.take().await.is_empty(),
            "there is nothing to read before opencode has written anything",
        );
        assert!(
            !reader.unreadable,
            "and a store that has not appeared yet is not a store of a shape this \
             build cannot read",
        );

        let writing = store(&database).await;
        opened(&writing, "ses_own", &worktree, now).await;
        wrote(
            &writing,
            "ses_own",
            0,
            "session.created.1",
            r#"{"of":"own"}"#,
        )
        .await;

        assert_eq!(
            reader.take().await,
            vec![r#"{"kind":"session.created.1","seq":0,"record":{"of":"own"}}"#.to_owned()],
            "and the poll after it appeared reads it",
        );
    }
}
