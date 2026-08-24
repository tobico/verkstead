//! What opening a database written by an older Verkstead does to it.
//!
//! One rewrite so far: the stops that used to be a table of their own become
//! the Notices and halts they are now. What is worth a test about it is what a
//! promise could not keep — that a database from before opens at all, that
//! every stop on a Timeline is still readable, that the ones still open still
//! stop their Conversation, and that opening the same database twice does not
//! do it twice.
//!
//! The old shape is written here by hand rather than by the code that used to
//! write it: that code has gone, and what has to keep working is a database
//! rather than a function.

use std::path::Path;

use sqlx::SqlitePool;
use verkstead_store::{
    Event, Halt, conversations, halt, halted, open_database, register_repo, start_conversation,
    start_grilling, timeline,
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

/// One stop as the old shape held it: an Event of the old kind, and the row of
/// facts beside it.
///
/// `settled` is the remedy and the note where the human answered it, or `None`
/// where the stop was left open — which is the one that was stopping the run.
async fn stop(
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

    stop(
        &pool,
        id,
        "the task in .tasks/02-window.md",
        Some(("retry", "try again but leave the migration alone")),
    )
    .await;
    stop(&pool, id, "the task in .tasks/03-counter.md", None).await;

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

/// And the one that was still open still stops the Conversation — as a halt,
/// which is now the only thing that does. Deliberate, because an open stop was
/// a run waiting on the human, and a restart must not drive past one.
#[tokio::test]
async fn a_stop_that_was_open_becomes_the_halt_it_now_is() {
    let dir = tempfile::tempdir().unwrap();
    let (pool, id) = before(dir.path()).await;

    stop(
        &pool,
        id,
        "the task in .tasks/02-window.md",
        Some(("abort", "")),
    )
    .await;
    let open = stop(&pool, id, "the task in .tasks/03-counter.md", None).await;

    pool.close().await;

    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    let halted = halted(&pool, id).await.unwrap().expect("it was open");

    assert_eq!(halted.halt, Halt::Deliberate);
    assert_eq!(
        halted.event_id, open,
        "the badge points at the Notice the open stop became",
    );
    assert_eq!(
        halted.at, "2026-08-01T09:14:22.000Z",
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
/// so nothing is waiting on it: no halt, and no badge.
#[tokio::test]
async fn a_stop_that_was_settled_stops_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let (pool, id) = before(dir.path()).await;

    stop(
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

    assert!(halted(&pool, id).await.unwrap().is_none());
    assert!(
        !conversations(&pool)
            .await
            .unwrap()
            .into_iter()
            .any(|row| row.id == id && row.waiting),
    );
}

/// A Conversation that has collected a halt since keeps the one it has: there
/// is one halt per Conversation, and the first Notice is the one that explains
/// it.
#[tokio::test]
async fn a_conversation_already_halted_keeps_the_halt_it_has() {
    let dir = tempfile::tempdir().unwrap();
    let (pool, id) = before(dir.path()).await;

    stop(&pool, id, "the task in .tasks/03-counter.md", None).await;

    let now = halt(
        &pool,
        id,
        Halt::Circumstance,
        "**Implementing the work** stopped.",
    )
    .await
    .unwrap()
    .unwrap();

    pool.close().await;

    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    let halted = halted(&pool, id).await.unwrap().unwrap();

    assert_eq!(halted.event_id, now);
    assert_eq!(halted.halt, Halt::Circumstance);
}

/// Opening it again does nothing: the table has gone, which is what says the
/// rewrite has run — and the Notices are left exactly as they were rather than
/// rewritten from rows that are no longer there.
#[tokio::test]
async fn a_database_opened_twice_is_rewritten_once() {
    let dir = tempfile::tempdir().unwrap();
    let (pool, id) = before(dir.path()).await;

    stop(&pool, id, "the task in .tasks/03-counter.md", None).await;

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
