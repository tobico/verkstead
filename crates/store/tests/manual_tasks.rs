//! The Manual Tasks a Verkstead of before put on a Conversation's Timeline, read
//! back by this one.
//!
//! Nothing writes one any more: what a human sets going by hand is a steer into
//! Implementing, whose instruction rides the Steer Event. What is asked of the
//! store here is ADR-0006's rule — *the record is kept and read rather than
//! rewritten* — so the rows are written by hand, as a database from before holds
//! them, and the tests are that a Timeline carrying one still reads back the
//! instruction it was written with.

use std::path::Path;

use sqlx::SqlitePool;
use verkstead_store::{Event, open_database, register_repo, start_conversation, timeline};

/// A pool over a fresh database, plus the directory keeping it alive.
async fn fresh_pool() -> (tempfile::TempDir, SqlitePool) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();
    (dir, pool)
}

/// A Conversation for an instruction to have been asked of.
async fn conversation(pool: &SqlitePool) -> i64 {
    let repo = register_repo(pool, Path::new("/watched/verkstead"), "verkstead", "main")
        .await
        .unwrap()
        .expect("nothing was registered at that path yet")
        .id;

    start_conversation(pool, repo, "rate-limiting")
        .await
        .unwrap()
        .expect("the Repo was just registered")
}

/// One Manual Task as a Verkstead of before wrote it: the Event of its own kind,
/// with the instruction whole in the body column.
///
/// Written here rather than by the call that used to write it — that call has
/// gone with the feature, and what has to keep working is a database rather than
/// a function.
async fn stored(pool: &SqlitePool, conversation_id: i64, instruction: &str) -> i64 {
    let (event_id,): (i64,) = sqlx::query_as(
        "INSERT INTO timeline_events (conversation_id, at, kind, body)
         VALUES (?, '2026-08-24T04:00:00.000Z', 'manual-task', ?)
         RETURNING id",
    )
    .bind(conversation_id)
    .bind(instruction)
    .fetch_one(pool)
    .await
    .unwrap();

    event_id
}

/// The instructions a Conversation was asked by hand, in Timeline order.
async fn instructions(pool: &SqlitePool, id: i64) -> Vec<String> {
    timeline(pool, id)
        .await
        .unwrap()
        .into_iter()
        .filter_map(|event| match event.event {
            Event::ManualTask(instruction) => Some(instruction),
            _ => None,
        })
        .collect()
}

/// What a stored Manual Task has to say is the whole of what the human typed:
/// the markdown they wrote, read back word for word.
#[tokio::test]
async fn a_stored_instruction_reads_back_as_the_markdown_it_was_typed_as() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool).await;

    let typed = "Rebase onto `main` and fix the conflicts.\n\n- keep the tests green\n";

    stored(&pool, id, typed).await;

    assert_eq!(instructions(&pool, id).await, [typed]);
}

/// And a Timeline that carries several carries all of them, in the order they
/// were asked: each was a moment of its own, and nothing rewrites a record.
#[tokio::test]
async fn each_stored_instruction_is_a_moment_of_its_own() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool).await;

    stored(&pool, id, "Rebase onto `main`.\n").await;
    stored(&pool, id, "Now push the branch.\n").await;

    assert_eq!(
        instructions(&pool, id).await,
        ["Rebase onto `main`.\n", "Now push the branch.\n"],
    );
}

/// A Manual Task is one Conversation's, so another's Timeline does not carry it.
#[tokio::test]
async fn a_manual_task_belongs_to_the_conversation_it_is_on() {
    let (_dir, pool) = fresh_pool().await;
    let one = conversation(&pool).await;

    let repo = register_repo(&pool, Path::new("/watched/askance"), "askance", "trunk")
        .await
        .unwrap()
        .unwrap()
        .id;
    let other = start_conversation(&pool, repo, "deferred-asks")
        .await
        .unwrap()
        .unwrap();

    stored(&pool, one, "Rebase onto `main`.\n").await;

    assert_eq!(instructions(&pool, other).await, Vec::<String>::new());
    assert_eq!(instructions(&pool, one).await.len(), 1);
}

/// And it is still there when the database is opened again, which is the whole
/// of what *the record is kept* means.
#[tokio::test]
async fn a_manual_task_survives_the_database_being_reopened() {
    let dir = tempfile::tempdir().unwrap();
    let database = dir.path().join("verkstead.db");

    let id = {
        let pool = open_database(&database).await.unwrap();
        let id = conversation(&pool).await;
        stored(&pool, id, "Rebase onto `main`.\n").await;
        pool.close().await;
        id
    };

    let pool = open_database(&database).await.unwrap();

    assert_eq!(instructions(&pool, id).await, ["Rebase onto `main`.\n"]);
}
