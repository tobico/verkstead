//! Manual Tasks: what the human asks for by hand at the end of a Timeline.
//!
//! One instruction in markdown, recorded as its own kind of Event and read back
//! whole. What the session it starts goes on to do is not this test's business
//! and not the Event's either — that lands beside it as the Events any work
//! lands as.

use std::path::Path;

use sqlx::SqlitePool;
use verkstead_store::{
    Event, open_database, record_manual_task, register_repo, save_brief, start_conversation,
    start_grilling, timeline,
};

/// A pool over a fresh database, plus the directory keeping it alive.
async fn fresh_pool() -> (tempfile::TempDir, SqlitePool) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();
    (dir, pool)
}

/// A Conversation with a Worktree, which is where a Manual Task can reach one.
///
/// Started for real rather than moved by hand: `start_grilling` is what records
/// the base commit and the worktree beside the state.
async fn working(pool: &SqlitePool) -> i64 {
    let repo = register_repo(pool, Path::new("/srv/verkstead"), "verkstead", "main")
        .await
        .unwrap()
        .expect("nothing is registered at that path yet");

    let id = start_conversation(pool, repo.id, "rate-limiting")
        .await
        .unwrap()
        .expect("the Repo was just registered");

    save_brief(pool, id, "# Rate limiting\n").await.unwrap();
    start_grilling(
        pool,
        id,
        "c0ffee",
        Path::new("/state/worktrees/rate-limiting"),
    )
    .await
    .unwrap();

    id
}

/// The instructions a Conversation has been asked by hand, in order.
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

#[tokio::test]
async fn an_instruction_lands_on_the_timeline_as_the_markdown_it_was_typed_as() {
    let (_dir, pool) = fresh_pool().await;
    let id = working(&pool).await;

    let typed = "Rebase onto `main` and fix the conflicts.\n\n- keep the tests green\n";

    assert!(record_manual_task(&pool, id, typed).await.unwrap());

    assert_eq!(instructions(&pool, id).await, [typed]);
}

#[tokio::test]
async fn each_manual_task_is_a_moment_of_its_own_rather_than_a_rewrite() {
    let (_dir, pool) = fresh_pool().await;
    let id = working(&pool).await;

    record_manual_task(&pool, id, "Rebase onto `main`.\n")
        .await
        .unwrap();
    record_manual_task(&pool, id, "Now push the branch.\n")
        .await
        .unwrap();

    assert_eq!(
        instructions(&pool, id).await,
        ["Rebase onto `main`.\n", "Now push the branch.\n"],
        "a second thought is a second Manual Task, and nothing leaves a Timeline",
    );
}

#[tokio::test]
async fn there_is_no_conversation_to_ask_anything_of() {
    let (_dir, pool) = fresh_pool().await;

    assert!(
        !record_manual_task(&pool, 404, "Rebase onto `main`.\n")
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn a_manual_task_survives_the_database_being_reopened() {
    let dir = tempfile::tempdir().unwrap();
    let database = dir.path().join("verkstead.db");

    let id = {
        let pool = open_database(&database).await.unwrap();
        let id = working(&pool).await;
        record_manual_task(&pool, id, "Rebase onto `main`.\n")
            .await
            .unwrap();
        pool.close().await;
        id
    };

    let pool = open_database(&database).await.unwrap();

    assert_eq!(instructions(&pool, id).await, ["Rebase onto `main`.\n"]);
}
