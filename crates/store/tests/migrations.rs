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
//! Both old shapes are written here by hand rather than by the code that used to
//! write them: that code has gone, and what has to keep working is a database
//! rather than a function.

use std::path::Path;

use sqlx::SqlitePool;
use verkstead_store::{
    Decision, Event, Lifecycle, asked_to_stop, clear_stop, conversations, load_conversation,
    open_database, register_repo, start_conversation, start_grilling, stop, stopped, timeline,
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

    assert_eq!(it.decision, Decision::Deliberate);
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

/// An open Pause reads back as the one stop too, deliberate — Verkstead pulled
/// the brake on the window — and carrying the words about when the account comes
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

    assert_eq!(it.decision, Decision::Deliberate, "it waits for a press");
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
