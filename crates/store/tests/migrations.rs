//! What opening a database written by an older Verkstead does to it.
//!
//! Two shapes of before, and both become the one stop. The Interruptions that
//! were a table of their own become the Notices and stops they are now; the
//! halts and open Pauses that were tables beside the Conversations are read onto
//! the Conversations themselves as the columns arrive. What is worth a test
//! about either is what a promise could not keep — that a database from before
//! opens at all, that every stop on a Timeline is still readable, that the ones
//! still open still stop their Conversation, and that opening the same database
//! twice does not do it twice.
//!
//! And one rename: the state off the ladder was stored as `aborted` while the
//! press was called Abort, in the state column and in the body of the move that
//! says the work got there. Both become `closed`.
//!
//! And one table rebuilt: a commit used to be the Conversation's and the sha's,
//! there being one repository per Conversation to be in. It is now the
//! Conversation's, the Repo's and the sha's — so the rows already there are
//! attributed to the Conversation's own repository, and the rule that keeps one
//! commit per Conversation is rebuilt around the new column.
//!
//! And another rebuilt, for the same reason and against the same rule: a pull
//! request used to be the Conversation's alone, and is now the Conversation's and
//! the Repo's — a Conversation ends on one per repository it was worked in.
//!
//! And two more that follow from that one: a fix session was counted against the
//! Conversation and a check, and a settled suite was the Conversation's, both of
//! which are a pull request's now. What was already written down is the
//! Conversation's own repository's, that being the only pull request it could
//! have been about.
//!
//! Both old shapes are written here by hand rather than by the code that used to
//! write them: that code has gone, and what has to keep working is a database
//! rather than a function.

use std::path::Path;

use sqlx::SqlitePool;
use verkstead_store::{
    Commit, Decision, Event, Finished, Lifecycle, PullRequest, WaitingOn, asked_to_stop,
    clear_stop, commit_repo, conversations, finish_wrap_up, fix_attempts, load_conversation,
    open_database, pull_request, pull_request_repo, record_another_pull_request, record_commit,
    record_fix_attempt, recorded_commits, register_repo, start_conversation, start_grilling, stop,
    stopped, timeline, wrap_up_settled,
};

/// A database with the old table in it, and a Conversation to hang stops off.
///
/// The pool is handed back so the old rows can be written, and the path so the
/// database can be opened again — which is when the rewrite runs.
async fn before(dir: &Path) -> (SqlitePool, i64) {
    let pool = open_database(&dir.join("verkstead.db")).await.unwrap();

    let repo = register_repo(&pool, Path::new("/watched/verkstead"), "verkstead", "main")
        .await
        .unwrap()
        .unwrap()
        .id;

    let id = start_conversation(&pool, repo, "rate-limiting")
        .await
        .unwrap()
        .unwrap();

    start_grilling(
        &pool,
        id,
        "6f32b11a0c4d1e8f5b3a97c2d0e4f6a8b1c3d5e7",
        Path::new("/data/worktrees/verkstead-rate-limiting"),
        &[],
    )
    .await
    .unwrap();

    // The table as a Verkstead of before declared it, index and all: the
    // migration finds a database rather than a call, so this is the database.
    sqlx::query(
        "CREATE TABLE interruptions (
             event_id        INTEGER PRIMARY KEY REFERENCES timeline_events(id),
             conversation_id INTEGER NOT NULL REFERENCES conversations(id),
             step            TEXT NOT NULL,
             what            TEXT NOT NULL,
             how             TEXT NOT NULL,
             git_status      TEXT NOT NULL,
             tail            TEXT NOT NULL,
             remedy          TEXT,
             note            TEXT,
             settled_at      TEXT
         ) STRICT",
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "CREATE UNIQUE INDEX interruptions_open
             ON interruptions (conversation_id) WHERE remedy IS NULL",
    )
    .execute(&pool)
    .await
    .unwrap();

    (pool, id)
}

/// One Interruption as the old shape held it: an Event of the old kind, and the
/// row of facts beside it.
///
/// `settled` is the remedy and the note where the human answered it, or `None`
/// where the stop was left open — which is the one that was stopping the run.
async fn interruption(
    pool: &SqlitePool,
    conversation_id: i64,
    what: &str,
    settled: Option<(&str, &str)>,
) -> i64 {
    let (event_id,): (i64,) = sqlx::query_as(
        "INSERT INTO timeline_events (conversation_id, at, kind, body)
         VALUES (?, '2026-08-01T09:14:22.000Z', 'interruption', '')
         RETURNING id",
    )
    .bind(conversation_id)
    .fetch_one(pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO interruptions
             (event_id, conversation_id, step, what, how, git_status, tail,
              remedy, note, settled_at)
         VALUES (?, ?, 'task', ?, 'the session exited with status 1',
                 '## rate-limiting\n M limiter.rs', 'error: could not compile',
                 ?, ?, ?)",
    )
    .bind(event_id)
    .bind(conversation_id)
    .bind(what)
    .bind(settled.map(|(remedy, _)| remedy))
    .bind(settled.map(|(_, note)| note))
    .bind(settled.map(|_| "2026-08-02T11:02:04.000Z"))
    .execute(pool)
    .await
    .unwrap();

    event_id
}

/// The Notices on a Conversation's Timeline, in Timeline order.
async fn notices(pool: &SqlitePool, id: i64) -> Vec<String> {
    timeline(pool, id)
        .await
        .unwrap()
        .into_iter()
        .filter_map(|event| match event.event {
            Event::Notice(markdown) => Some(markdown),
            _ => None,
        })
        .collect()
}

/// Every stop is readable afterwards, whether it was answered or not: the
/// Timeline is the record, and a rewrite that lost one would be a run nobody
/// could account for.
#[tokio::test]
async fn every_stop_of_before_is_a_notice_afterwards() {
    let dir = tempfile::tempdir().unwrap();
    let (pool, id) = before(dir.path()).await;

    interruption(
        &pool,
        id,
        "the task in .tasks/02-window.md",
        Some(("retry", "try again but leave the migration alone")),
    )
    .await;
    interruption(&pool, id, "the task in .tasks/03-counter.md", None).await;

    pool.close().await;

    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    let said = notices(&pool, id).await;

    assert_eq!(said.len(), 2, "both of them, in the order they happened");

    assert!(
        said[0].starts_with("**The task in .tasks/02-window.md** stopped."),
        "{:?}",
        said[0],
    );
    assert!(
        said[0].contains("the session exited with status 1"),
        "the reason survives: {:?}",
        said[0],
    );
    assert!(
        said[0].contains("    ## rate-limiting")
            && said[0].contains("    error: could not compile"),
        "and both pieces of evidence: {:?}",
        said[0],
    );
    assert!(
        said[0].contains("The step was run again, on 2026-08-02T11:02:04.000Z.")
            && said[0].contains("try again but leave the migration alone"),
        "and what the human did about it: {:?}",
        said[0],
    );

    assert!(
        !said[1].contains("What was done about it"),
        "the open one had nothing done about it: {:?}",
        said[1],
    );
}

/// And the one that was still open still stops the Conversation — as the one
/// stop, which is now the only thing that does. Deliberate, because an open stop
/// was a run waiting on the human, and a restart must not drive past one.
#[tokio::test]
async fn a_stop_that_was_open_becomes_the_one_stop_it_now_is() {
    let dir = tempfile::tempdir().unwrap();
    let (pool, id) = before(dir.path()).await;

    interruption(
        &pool,
        id,
        "the task in .tasks/02-window.md",
        Some(("abort", "")),
    )
    .await;
    let open = interruption(&pool, id, "the task in .tasks/03-counter.md", None).await;

    pool.close().await;

    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    let it = stopped(&pool, id).await.unwrap().expect("it was open");

    assert_eq!(
        it.decision,
        Decision::Verkstead,
        "a step of before that failed was Verkstead pulling the brake, so it \
         keeps the marks a stop the human pressed would not",
    );
    assert_eq!(
        it.notice, open,
        "the badge points at the Notice the open stop became",
    );
    assert_eq!(
        it.at, "2026-08-01T09:14:22.000Z",
        "stamped when the run stopped rather than when the server was upgraded",
    );

    assert!(
        conversations(&pool)
            .await
            .unwrap()
            .into_iter()
            .any(|row| row.id == id && row.waiting),
        "and it is still waiting on the human",
    );
}

/// A stop the human answered is a run that got going again or one they ended,
/// so nothing is waiting on it: no stop, and no badge.
#[tokio::test]
async fn a_stop_that_was_settled_stops_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let (pool, id) = before(dir.path()).await;

    interruption(
        &pool,
        id,
        "the task in .tasks/02-window.md",
        Some(("take-over", "")),
    )
    .await;

    pool.close().await;

    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    assert!(stopped(&pool, id).await.unwrap().is_none());
    assert!(
        !conversations(&pool)
            .await
            .unwrap()
            .into_iter()
            .any(|row| row.id == id && row.waiting),
    );
}

/// A Conversation that has stopped since keeps the stop it has: there is one
/// per Conversation, and the first Notice is the one that explains it.
#[tokio::test]
async fn a_conversation_already_stopped_keeps_the_stop_it_has() {
    let dir = tempfile::tempdir().unwrap();
    let (pool, id) = before(dir.path()).await;

    interruption(&pool, id, "the task in .tasks/03-counter.md", None).await;

    let now = stop(
        &pool,
        id,
        Decision::Circumstance,
        "**Implementing the work** stopped.",
        None,
    )
    .await
    .unwrap()
    .unwrap();

    pool.close().await;

    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    let it = stopped(&pool, id).await.unwrap().unwrap();

    assert_eq!(it.notice, now);
    assert_eq!(it.decision, Decision::Circumstance);
}

/// Opening it again does nothing: the table has gone, which is what says the
/// rewrite has run — and the Notices are left exactly as they were rather than
/// rewritten from rows that are no longer there.
#[tokio::test]
async fn a_database_opened_twice_is_rewritten_once() {
    let dir = tempfile::tempdir().unwrap();
    let (pool, id) = before(dir.path()).await;

    interruption(&pool, id, "the task in .tasks/03-counter.md", None).await;

    pool.close().await;

    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();
    let once = notices(&pool, id).await;
    pool.close().await;

    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    assert_eq!(notices(&pool, id).await, once);

    let table: Option<(String,)> =
        sqlx::query_as("SELECT name FROM sqlite_master WHERE type = 'table' AND name = ?")
            .bind("interruptions")
            .fetch_optional(&pool)
            .await
            .unwrap();

    assert!(table.is_none(), "the table the stops were in has gone");
}

/// And a database that never had one opens with nothing to do, which is every
/// database made from now on.
#[tokio::test]
async fn a_database_made_today_has_nothing_to_rewrite() {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    assert!(conversations(&pool).await.unwrap().is_empty());
}

/// A database whose stops were kept in tables beside the Conversations, as one
/// written before the merge holds them.
///
/// Built by taking the columns back off a database this build made, which is
/// exactly what a database from before is: the tables were there and the columns
/// were not. The pool is handed back so the old rows can be written, and the
/// path so the database can be opened again.
async fn beside(dir: &Path) -> (SqlitePool, i64) {
    let pool = open_database(&dir.join("verkstead.db")).await.unwrap();

    let repo = register_repo(&pool, Path::new("/watched/verkstead"), "verkstead", "main")
        .await
        .unwrap()
        .unwrap()
        .id;

    let id = start_conversation(&pool, repo, "rate-limiting")
        .await
        .unwrap()
        .unwrap();

    start_grilling(
        &pool,
        id,
        "6f32b11a0c4d1e8f5b3a97c2d0e4f6a8b1c3d5e7",
        Path::new("/data/worktrees/verkstead-rate-limiting"),
        &[],
    )
    .await
    .unwrap();

    // The two tables as a Verkstead of before declared them, index and all.
    for statement in [
        "CREATE TABLE halts (
             conversation_id INTEGER PRIMARY KEY REFERENCES conversations(id),
             at              TEXT NOT NULL,
             halt            TEXT NOT NULL,
             event_id        INTEGER NOT NULL REFERENCES timeline_events(id)
         ) STRICT",
        "CREATE TABLE stops_asked (
             conversation_id INTEGER PRIMARY KEY REFERENCES conversations(id),
             at              TEXT NOT NULL
         ) STRICT",
        "CREATE UNIQUE INDEX pauses_open
             ON pauses (conversation_id) WHERE resumed_at IS NULL",
    ] {
        sqlx::query(statement).execute(&pool).await.unwrap();
    }

    // And the columns off, which is the whole of what says this database is one
    // from before: the copying runs as they arrive.
    for column in [
        "stopped_at",
        "stopped_by",
        "stopped_notice",
        "stopped_resets",
        "stop_asked_at",
    ] {
        sqlx::query(&format!("ALTER TABLE conversations DROP COLUMN {column}"))
            .execute(&pool)
            .await
            .unwrap();
    }

    (pool, id)
}

/// One Event on a Conversation's Timeline, for a halt or a Pause to hang off.
async fn event(pool: &SqlitePool, conversation_id: i64, kind: &str, body: &str) -> i64 {
    let (event_id,): (i64,) = sqlx::query_as(
        "INSERT INTO timeline_events (conversation_id, at, kind, body)
         VALUES (?, '2026-08-01T09:14:22.000Z', ?, ?)
         RETURNING id",
    )
    .bind(conversation_id)
    .bind(kind)
    .bind(body)
    .fetch_one(pool)
    .await
    .unwrap();

    event_id
}

/// An open halt reads back as the one stop, kind, Notice, stamp and all.
#[tokio::test]
async fn an_open_halt_reads_back_as_a_stop() {
    let dir = tempfile::tempdir().unwrap();
    let (pool, id) = beside(dir.path()).await;

    let notice = event(&pool, id, "notice", "**Implementing the work** stopped.").await;

    sqlx::query(
        "INSERT INTO halts (conversation_id, at, halt, event_id)
         VALUES (?, '2026-08-01T09:14:22.000Z', 'circumstance', ?)",
    )
    .bind(id)
    .bind(notice)
    .execute(&pool)
    .await
    .unwrap();

    pool.close().await;

    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    let it = stopped(&pool, id).await.unwrap().expect("it was halted");

    assert_eq!(it.decision, Decision::Circumstance, "nobody chose it");
    assert_eq!(
        it.notice, notice,
        "and the badge still points at its Notice"
    );
    assert_eq!(
        it.at, "2026-08-01T09:14:22.000Z",
        "stamped when the run stopped rather than when the server was upgraded",
    );

    let rows: Vec<(i64,)> = sqlx::query_as("SELECT conversation_id FROM halts")
        .fetch_all(&pool)
        .await
        .unwrap();

    assert_eq!(
        rows.len(),
        1,
        "and the table it was in is left exactly where it was",
    );
}

/// An open Pause reads back as the one stop too, Verkstead's — it pulled the
/// brake on the window — and carrying the words about when the account comes
/// back. The Event stays the Pause it always was.
#[tokio::test]
async fn an_open_pause_reads_back_as_a_stop_with_reset_words() {
    let dir = tempfile::tempdir().unwrap();
    let (pool, id) = beside(dir.path()).await;

    let waiting = event(&pool, id, "pause", "").await;

    sqlx::query(
        "INSERT INTO pauses (event_id, conversation_id, profile, said, resets_at)
         VALUES (?, ?, 'fable', 'Usage limit reached', '2026-08-24T05:00:00Z')",
    )
    .bind(waiting)
    .bind(id)
    .execute(&pool)
    .await
    .unwrap();

    pool.close().await;

    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    let it = stopped(&pool, id).await.unwrap().expect("it was waiting");

    assert_eq!(
        it.decision,
        Decision::Verkstead,
        "it waits for a press, and for a reason the human has to be shown",
    );
    assert_eq!(it.notice, waiting, "the badge points at the Pause Event");
    assert_eq!(it.resets.as_deref(), Some("2026-08-24T05:00:00Z"));
    assert_eq!(
        it.at, "2026-08-01T09:14:22.000Z",
        "stamped when the run stopped",
    );

    assert!(
        conversations(&pool)
            .await
            .unwrap()
            .into_iter()
            .any(|row| row.id == id && row.waiting),
        "and the sidebar still says it is waiting on the human",
    );

    let rows: Vec<(Option<String>,)> = sqlx::query_as("SELECT resumed_at FROM pauses")
        .fetch_all(&pool)
        .await
        .unwrap();

    assert_eq!(
        rows,
        vec![(None,)],
        "with the Pause left open exactly as it was written: nothing rewrote it",
    );
}

/// A Pause that had already ended stops nothing, and neither does one on a
/// Conversation that had a halt of its own — the halt is the stop that names
/// its own Notice.
#[tokio::test]
async fn a_halt_wins_over_a_pause_and_an_ended_pause_stops_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let (pool, id) = beside(dir.path()).await;

    let notice = event(&pool, id, "notice", "**Implementing the work** stopped.").await;
    let waiting = event(&pool, id, "pause", "").await;

    sqlx::query(
        "INSERT INTO halts (conversation_id, at, halt, event_id)
         VALUES (?, '2026-08-01T09:14:22.000Z', 'deliberate', ?)",
    )
    .bind(id)
    .bind(notice)
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO pauses (event_id, conversation_id, profile, said, resets_at)
         VALUES (?, ?, 'fable', 'Usage limit reached', '2026-08-24T05:00:00Z')",
    )
    .bind(waiting)
    .bind(id)
    .execute(&pool)
    .await
    .unwrap();

    pool.close().await;

    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    let it = stopped(&pool, id).await.unwrap().unwrap();

    assert_eq!(it.notice, notice, "the halt is the stop that stands");
    assert_eq!(
        it.resets, None,
        "and it carries nothing about a window, because it is not one",
    );
}

/// A Stop the human pressed that never landed is still pressed afterwards: a
/// server that read nothing here would take the run up as though nobody had
/// asked.
#[tokio::test]
async fn a_stop_asked_for_before_is_still_asked_for() {
    let dir = tempfile::tempdir().unwrap();
    let (pool, id) = beside(dir.path()).await;

    sqlx::query(
        "INSERT INTO stops_asked (conversation_id, at) VALUES (?, '2026-08-01T09:14:22.000Z')",
    )
    .bind(id)
    .execute(&pool)
    .await
    .unwrap();

    pool.close().await;

    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    assert!(asked_to_stop(&pool, id).await.unwrap());
}

/// And opening it again copies nothing: the columns are what says the reading
/// has happened, so a stop the human has since resumed stays resumed rather than
/// arriving all over again on the next restart.
#[tokio::test]
async fn a_stop_read_across_is_not_read_across_twice() {
    let dir = tempfile::tempdir().unwrap();
    let (pool, id) = beside(dir.path()).await;

    let notice = event(&pool, id, "notice", "**Implementing the work** stopped.").await;

    sqlx::query(
        "INSERT INTO halts (conversation_id, at, halt, event_id)
         VALUES (?, '2026-08-01T09:14:22.000Z', 'deliberate', ?)",
    )
    .bind(id)
    .bind(notice)
    .execute(&pool)
    .await
    .unwrap();

    pool.close().await;

    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    clear_stop(&pool, id).await.unwrap();
    pool.close().await;

    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    assert_eq!(
        stopped(&pool, id).await.unwrap(),
        None,
        "the run the human started again is still going",
    );
}

/// The states a Conversation's moves say it went through, in Timeline order.
async fn moves(pool: &SqlitePool, id: i64) -> Vec<Lifecycle> {
    timeline(pool, id)
        .await
        .unwrap()
        .into_iter()
        .filter_map(|event| match event.event {
            Event::Moved(state) => Some(state),
            _ => None,
        })
        .collect()
}

/// The state and the move that says the work got there both move to the word
/// the press is called now, so the Conversation is described in one vocabulary
/// rather than two.
#[tokio::test]
async fn a_conversation_that_was_aborted_reads_as_closed() {
    let dir = tempfile::tempdir().unwrap();
    let (pool, id) = before(dir.path()).await;

    // The state and the Event as a Verkstead of before left them: the word in
    // the column, and the same word as the body of the move.
    sqlx::query("UPDATE conversations SET state = 'aborted' WHERE id = ?")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();

    sqlx::query(
        "INSERT INTO timeline_events (conversation_id, at, kind, body)
         VALUES (?, '2026-08-01T09:14:22.000Z', 'moved', 'aborted')",
    )
    .bind(id)
    .execute(&pool)
    .await
    .unwrap();

    pool.close().await;

    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    assert_eq!(
        load_conversation(&pool, id).await.unwrap().unwrap().state,
        Lifecycle::Closed,
    );
    assert_eq!(
        moves(&pool, id).await,
        [Lifecycle::Grilling, Lifecycle::Closed]
    );

    let (stored,): (String,) = sqlx::query_as("SELECT state FROM conversations WHERE id = ?")
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();

    assert_eq!(
        stored, "closed",
        "the stored word moved rather than the reading"
    );

    let left: Vec<(String,)> = sqlx::query_as(
        "SELECT body FROM timeline_events WHERE kind = 'moved' AND body = 'aborted'",
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    assert!(left.is_empty(), "and the move's body moved with it");
}

/// A Brief that happens to hold the word is prose somebody wrote, not a state:
/// the rename is the `moved` Events and the state column, and nothing wider.
#[tokio::test]
async fn a_brief_that_says_aborted_is_left_exactly_as_it_was() {
    let dir = tempfile::tempdir().unwrap();
    let (pool, id) = before(dir.path()).await;

    sqlx::query(
        "INSERT INTO timeline_events (conversation_id, at, kind, body)
         VALUES (?, '2026-08-01T09:14:22.000Z', 'brief', 'aborted')",
    )
    .bind(id)
    .execute(&pool)
    .await
    .unwrap();

    pool.close().await;

    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    let briefs: Vec<String> = timeline(&pool, id)
        .await
        .unwrap()
        .into_iter()
        .filter_map(|event| match event.event {
            Event::Brief(markdown) => Some(markdown),
            _ => None,
        })
        .collect();

    // The empty one the Conversation started with, and then this one.
    assert_eq!(briefs.last().unwrap(), "aborted");
}

/// And the old word is still read, for a row no migration reached: a database
/// restored from a backup taken before it ran, or one somebody wrote by hand.
#[tokio::test]
async fn the_old_word_is_still_a_state_this_verkstead_can_read() {
    let dir = tempfile::tempdir().unwrap();
    let (pool, id) = before(dir.path()).await;

    sqlx::query("UPDATE conversations SET state = 'aborted' WHERE id = ?")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();

    assert_eq!(
        load_conversation(&pool, id).await.unwrap().unwrap().state,
        Lifecycle::Closed,
    );
}

/// A database whose commits are the Conversation's and the sha's, which is what
/// every commit recorded before Verkstead swept more than one repository is.
///
/// The table is written out as the Verkstead that made it declared it — the
/// migration finds a database rather than a call — and one commit put in it, with
/// the Commit Summary that hangs off the same Event.
async fn commits_of_before(dir: &Path) -> (i64, i64) {
    let pool = open_database(&dir.join("verkstead.db")).await.unwrap();

    let repo = register_repo(&pool, Path::new("/watched/verkstead"), "verkstead", "main")
        .await
        .unwrap()
        .unwrap()
        .id;

    let id = start_conversation(&pool, repo, "rate-limiting")
        .await
        .unwrap()
        .unwrap();

    sqlx::query("DROP TABLE commits")
        .execute(&pool)
        .await
        .unwrap();

    sqlx::query(
        "CREATE TABLE commits (
             event_id        INTEGER PRIMARY KEY REFERENCES timeline_events(id),
             conversation_id INTEGER NOT NULL REFERENCES conversations(id),
             sha             TEXT NOT NULL,
             subject         TEXT NOT NULL,
             files           INTEGER NOT NULL,
             insertions      INTEGER NOT NULL,
             deletions       INTEGER NOT NULL,
             UNIQUE (conversation_id, sha)
         ) STRICT",
    )
    .execute(&pool)
    .await
    .unwrap();

    let (event_id,): (i64,) = sqlx::query_as(
        "INSERT INTO timeline_events (conversation_id, at, kind, body)
         VALUES (?, '2026-08-01T09:14:22.000Z', 'commit', '')
         RETURNING id",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO commits
             (event_id, conversation_id, sha, subject, files, insertions, deletions)
         VALUES (?, ?, 'a1b2c3d', 'feat: rate limiting', 2, 31, 4)",
    )
    .bind(event_id)
    .bind(id)
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO commit_summaries (event_id, summary) VALUES (?, 'A bucket per account.')",
    )
    .bind(event_id)
    .execute(&pool)
    .await
    .unwrap();

    pool.close().await;

    (id, repo)
}

/// The commits on a Conversation's Timeline, in Timeline order.
async fn commits(pool: &SqlitePool, id: i64) -> Vec<Commit> {
    timeline(pool, id)
        .await
        .unwrap()
        .into_iter()
        .filter_map(|event| match event.event {
            Event::Commit(commit) => Some(commit),
            _ => None,
        })
        .collect()
}

/// A commit recorded before this is the Conversation's own repository's, which
/// is the only repository it was possible for it to be in — and it reads back
/// exactly as it always did, unlabeled, with what it said about itself.
#[tokio::test]
async fn every_commit_of_before_is_the_conversations_own_repositorys() {
    let dir = tempfile::tempdir().unwrap();
    let (id, repo) = commits_of_before(dir.path()).await;

    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    let landed = commits(&pool, id).await;

    assert_eq!(landed.len(), 1, "the commit is still on the Timeline");
    assert_eq!(landed[0].sha, "a1b2c3d");
    assert_eq!(landed[0].subject, "feat: rate limiting");
    assert_eq!(landed[0].files, 2);
    assert_eq!(landed[0].insertions, 31);
    assert_eq!(landed[0].deletions, 4);
    assert_eq!(
        landed[0].summary.as_deref(),
        Some("A bucket per account."),
        "and what it said about itself, which hangs off the same Event",
    );
    assert_eq!(
        landed[0].repo, None,
        "unlabeled, which is what the work's own repository draws as",
    );

    assert_eq!(
        recorded_commits(&pool, id, repo).await.unwrap(),
        vec!["a1b2c3d".to_owned()],
        "and the sweep of that repository knows it has it",
    );

    let event = timeline(&pool, id)
        .await
        .unwrap()
        .into_iter()
        .find(|event| matches!(event.event, Event::Commit(_)))
        .expect("the commit is on the Timeline")
        .id;

    assert_eq!(
        commit_repo(&pool, id, event)
            .await
            .unwrap()
            .map(|repo| repo.path),
        Some(Path::new("/watched/verkstead").to_owned()),
        "so the details pane reads its diff out of the Conversation's own repository",
    );
}

/// And the rule the table carries is the rebuilt one: the same commit offered
/// again is refused, and opening the database a second time rewrites nothing.
#[tokio::test]
async fn the_rebuilt_commits_table_keeps_one_commit_per_conversation_per_repo() {
    let dir = tempfile::tempdir().unwrap();
    let (id, repo) = commits_of_before(dir.path()).await;

    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    let again = Commit {
        sha: "a1b2c3d".to_owned(),
        subject: "feat: rate limiting".to_owned(),
        files: 2,
        insertions: 31,
        deletions: 4,
        summary: None,
        repo: None,
    };

    assert_eq!(
        record_commit(&pool, id, repo, &again).await.unwrap(),
        None,
        "the next sweep of that branch finds nothing left to do",
    );

    pool.close().await;

    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    assert_eq!(
        commits(&pool, id).await.len(),
        1,
        "and a database opened twice is rewritten once",
    );
}

/// A database whose pull requests are the Conversation's alone, which is what
/// every one recorded before a Conversation could end on more than one is.
///
/// The table is written out as the Verkstead that made it declared it — the
/// migration finds a database rather than a call — and one pull request put in
/// it, on the Event it hangs off.
async fn pull_requests_of_before(dir: &Path) -> (i64, i64) {
    let pool = open_database(&dir.join("verkstead.db")).await.unwrap();

    let repo = register_repo(&pool, Path::new("/watched/verkstead"), "verkstead", "main")
        .await
        .unwrap()
        .unwrap()
        .id;

    let id = start_conversation(&pool, repo, "rate-limiting")
        .await
        .unwrap()
        .unwrap();

    sqlx::query("DROP TABLE pull_requests")
        .execute(&pool)
        .await
        .unwrap();

    sqlx::query(
        "CREATE TABLE pull_requests (
             event_id        INTEGER PRIMARY KEY REFERENCES timeline_events(id),
             conversation_id INTEGER NOT NULL REFERENCES conversations(id),
             number          INTEGER NOT NULL,
             title           TEXT NOT NULL,
             url             TEXT NOT NULL,
             UNIQUE (conversation_id)
         ) STRICT",
    )
    .execute(&pool)
    .await
    .unwrap();

    let (event_id,): (i64,) = sqlx::query_as(
        "INSERT INTO timeline_events (conversation_id, at, kind, body)
         VALUES (?, '2026-08-01T09:14:22.000Z', 'pull-request', '')
         RETURNING id",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO pull_requests (event_id, conversation_id, number, title, url)
         VALUES (?, ?, 41, 'Rate limiting', 'https://github.com/tobico/verkstead/pull/41')",
    )
    .bind(event_id)
    .bind(id)
    .execute(&pool)
    .await
    .unwrap();

    pool.close().await;

    (id, repo)
}

/// A pull request recorded before this is the Conversation's own repository's,
/// which is the only repository it was possible for it to be in — and it reads
/// back exactly as it always did, unlabeled, in the repository the details pane
/// asks GitHub in.
#[tokio::test]
async fn every_pull_request_of_before_is_the_conversations_own_repositorys() {
    let dir = tempfile::tempdir().unwrap();
    let (id, repo) = pull_requests_of_before(dir.path()).await;

    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    assert_eq!(
        pull_request(&pool, id, repo).await.unwrap(),
        Some(PullRequest {
            number: 41,
            title: "Rate limiting".to_owned(),
            url: "https://github.com/tobico/verkstead/pull/41".to_owned(),
            repo: None,
        }),
        "the wrap-up's watchers still find the pull request they always did",
    );

    let event = timeline(&pool, id)
        .await
        .unwrap()
        .into_iter()
        .find(|event| matches!(event.event, Event::PullRequest(_)))
        .expect("the pull request is on the Timeline")
        .id;

    assert_eq!(
        pull_request_repo(&pool, id, event)
            .await
            .unwrap()
            .map(|repo| repo.path),
        Some(Path::new("/watched/verkstead").to_owned()),
        "so the details pane asks GitHub in the Conversation's own repository",
    );
}

/// And the rule the table carries is the rebuilt one: another repository's pull
/// request stands beside the one that is there, and opening the database a
/// second time rewrites nothing.
#[tokio::test]
async fn the_rebuilt_pull_requests_table_keeps_one_per_conversation_per_repo() {
    let dir = tempfile::tempdir().unwrap();
    let (id, _) = pull_requests_of_before(dir.path()).await;

    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    let beside = register_repo(&pool, Path::new("/watched/askance"), "askance", "main")
        .await
        .unwrap()
        .unwrap()
        .id;

    let companions = PullRequest {
        number: 7,
        title: "Rate limiting".to_owned(),
        url: "https://github.com/tobico/askance/pull/7".to_owned(),
        repo: None,
    };

    assert!(
        record_another_pull_request(&pool, id, beside, &companions)
            .await
            .unwrap(),
        "the old rule would have refused this outright",
    );
    assert!(
        record_another_pull_request(&pool, id, beside, &companions)
            .await
            .unwrap(),
        "and the new one keeps the row that repository already has",
    );

    pool.close().await;

    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    assert_eq!(
        timeline(&pool, id)
            .await
            .unwrap()
            .into_iter()
            .filter(|event| matches!(event.event, Event::PullRequest(_)))
            .count(),
        2,
        "and a database opened twice is rewritten once",
    );
}

/// A database whose wrap-up bookkeeping is the shape it was before a
/// Conversation could end on more than one pull request: a fix session counted
/// against the Conversation and a check, and a suite settled for the Conversation
/// itself.
///
/// The Conversation is walked to Wrapping first, so what the rewrite attributes
/// is a wrap-up that was really under way — and the pull request it is on is the
/// one its settled checks turn out to have been about.
async fn wrap_up_of_before(dir: &Path) -> (i64, i64) {
    let pool = open_database(&dir.join("verkstead.db")).await.unwrap();

    let repo = register_repo(&pool, Path::new("/watched/verkstead"), "verkstead", "main")
        .await
        .unwrap()
        .unwrap()
        .id;

    let id = start_conversation(&pool, repo, "rate-limiting")
        .await
        .unwrap()
        .unwrap();

    start_grilling(
        &pool,
        id,
        "c0ffee",
        Path::new("/state/worktrees/rate-limiting"),
        &[],
    )
    .await
    .unwrap();

    verkstead_store::pick_direction(&pool, id, verkstead_schema::Direction::Inline)
        .await
        .unwrap();

    verkstead_store::record_pull_request(
        &pool,
        id,
        repo,
        &PullRequest {
            number: 41,
            title: "Rate limiting".to_owned(),
            url: "https://github.com/tobico/verkstead/pull/41".to_owned(),
            repo: None,
        },
    )
    .await
    .unwrap();

    for table in [
        "check_fix_attempts",
        "wrap_up_settled",
        "addressed_comments",
    ] {
        sqlx::query(&format!("DROP TABLE {table}"))
            .execute(&pool)
            .await
            .unwrap();
    }

    sqlx::query(
        "CREATE TABLE check_fix_attempts (
             conversation_id INTEGER NOT NULL REFERENCES conversations(id),
             check_name      TEXT NOT NULL,
             attempts        INTEGER NOT NULL,
             PRIMARY KEY (conversation_id, check_name)
         ) STRICT",
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "CREATE TABLE wrap_up_settled (
             conversation_id INTEGER NOT NULL REFERENCES conversations(id),
             waiting_on      TEXT NOT NULL,
             at              TEXT NOT NULL,
             PRIMARY KEY (conversation_id, waiting_on)
         ) STRICT",
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "CREATE TABLE addressed_comments (
             conversation_id INTEGER NOT NULL REFERENCES conversations(id),
             comment_id      TEXT NOT NULL,
             at              TEXT NOT NULL,
             PRIMARY KEY (conversation_id, comment_id)
         ) STRICT",
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO check_fix_attempts (conversation_id, check_name, attempts)
         VALUES (?, 'Rust', 1)",
    )
    .bind(id)
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO addressed_comments (conversation_id, comment_id, at)
         VALUES (?, 'IC_1', '2026-08-01T09:14:22.000Z')",
    )
    .bind(id)
    .execute(&pool)
    .await
    .unwrap();

    for waiting_on in ["checks", "review", "comments"] {
        sqlx::query(
            "INSERT INTO wrap_up_settled (conversation_id, waiting_on, at)
             VALUES (?, ?, '2026-08-01T09:14:22.000Z')",
        )
        .bind(id)
        .bind(waiting_on)
        .execute(&pool)
        .await
        .unwrap();
    }

    pool.close().await;

    (id, repo)
}

/// A comment a session was dispatched about before this was dispatched about on
/// the Conversation's own pull request, which is the only one it could have been
/// left on — so the comment stays answered there, and the same pull request's
/// watcher does not dispatch a second session about yesterday's feedback.
#[tokio::test]
async fn every_comment_dispatched_for_before_this_was_the_conversations_own_pull_requests() {
    let dir = tempfile::tempdir().unwrap();
    let (id, repo) = wrap_up_of_before(dir.path()).await;

    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    assert_eq!(
        verkstead_store::addressed_comments(&pool, id, repo)
            .await
            .unwrap(),
        vec!["IC_1".to_owned()],
        "the comment somebody was sent to deal with is still dealt with",
    );

    let beside = register_repo(&pool, Path::new("/watched/askance"), "askance", "main")
        .await
        .unwrap()
        .unwrap()
        .id;

    assert_eq!(
        verkstead_store::addressed_comments(&pool, id, beside)
            .await
            .unwrap(),
        Vec::<String>::new(),
        "and a companion's pull request has had nothing dispatched about it",
    );

    pool.close().await;

    // And a database opened twice is rewritten once.
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    assert_eq!(
        verkstead_store::addressed_comments(&pool, id, repo)
            .await
            .unwrap(),
        vec!["IC_1".to_owned()],
    );
}

/// A fix session counted before this was counted against the Conversation's own
/// repository, which is the only pull request it was possible for it to have been
/// about — so the check that had a go left has one, and the same check name on
/// another pull request has its own two.
#[tokio::test]
async fn every_fix_session_counted_before_this_was_the_conversations_own_repositorys() {
    let dir = tempfile::tempdir().unwrap();
    let (id, repo) = wrap_up_of_before(dir.path()).await;

    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    assert_eq!(
        fix_attempts(&pool, id, repo, "Rust").await.unwrap(),
        1,
        "the go it has already had is still spent",
    );

    let beside = register_repo(&pool, Path::new("/watched/askance"), "askance", "main")
        .await
        .unwrap()
        .unwrap()
        .id;

    assert_eq!(
        fix_attempts(&pool, id, beside, "Rust").await.unwrap(),
        0,
        "and the same check name on another pull request starts from its own two",
    );
    assert_eq!(
        record_fix_attempt(&pool, id, beside, "Rust").await.unwrap(),
        1,
        "which the rebuilt key is what makes room for",
    );
}

/// And a settled suite, and a pull request nothing was left unaddressed on, were
/// both about that same pull request — while the review stays what it always
/// was: one review, about no pull request in particular.
///
/// Which together are what keeps a wrap-up that was nearly over from starting
/// again: what it had settled is still settled, so a server that came back up to
/// this database is waiting on what it was waiting on before — which here is
/// nothing at all.
#[tokio::test]
async fn every_settled_suite_of_before_is_the_conversations_own_pull_requests() {
    let dir = tempfile::tempdir().unwrap();
    let (id, repo) = wrap_up_of_before(dir.path()).await;

    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    let mut settled = wrap_up_settled(&pool, id).await.unwrap();
    settled.sort_by_key(|one| format!("{one:?}"));

    assert_eq!(
        settled,
        vec![
            WaitingOn::Checks(repo),
            WaitingOn::Comments(repo),
            WaitingOn::Review,
        ],
        "the suite that was green and the pull request that was quiet are the one \
         pull request's they could have been",
    );

    // And the rule that ends a wrap-up reads them as it always did: everything
    // this wrap-up was waiting on it had already settled, so it is over.
    assert_eq!(finish_wrap_up(&pool, id).await.unwrap(), Finished::Done);

    pool.close().await;

    // And a database opened twice is rewritten once: a second run over the
    // rebuilt tables would find nothing to attribute and drop what is there.
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    assert_eq!(
        fix_attempts(&pool, id, repo, "Rust").await.unwrap(),
        1,
        "what a check had been given is where the first run left it",
    );
    assert_eq!(wrap_up_settled(&pool, id).await.unwrap().len(), 3);
}
