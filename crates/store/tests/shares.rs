//! What the store keeps about handing a Conversation over: where its share was
//! published, and whether a share of it was ever commented on its pull
//! requests.
//!
//! The first is a link that is replaced every time somebody publishes again.
//! The second is the fact the automatic share is gated on — see
//! `share_to_pull_requests` in `crates/server/src/settling.rs` — and what is
//! worth a test about it is exactly what the gate leans on: it is off until a
//! comment lands, a second landing does not move it, and nothing takes it away
//! again.

use std::path::Path;

use sqlx::SqlitePool;
use verkstead_store::{
    open_database, record_share, record_share_comment, register_repo, share, share_commented,
    start_conversation,
};

/// A pool over a fresh database, plus the directory keeping it alive.
async fn fresh_pool() -> (tempfile::TempDir, SqlitePool) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();
    (dir, pool)
}

/// A Conversation to share.
async fn conversation(pool: &SqlitePool, branch: &str) -> i64 {
    let repo = register_repo(pool, Path::new("/watched/verkstead"), "verkstead", "main")
        .await
        .unwrap()
        .map(|repo| repo.id)
        .unwrap_or(1);

    start_conversation(pool, repo, branch)
        .await
        .unwrap()
        .expect("the Repo is registered")
}

/// The fact is off until a comment lands, and the landing is what writes it.
#[tokio::test]
async fn nothing_is_on_record_until_a_comment_lands() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool, "sharing").await;

    assert!(
        !share_commented(&pool, id).await.unwrap(),
        "a Conversation nobody has commented a share on has nothing on record",
    );

    record_share_comment(&pool, id).await.unwrap();

    assert!(share_commented(&pool, id).await.unwrap());
}

/// And it is one fact rather than a count: a second comment is not a second
/// record, and it does not move the moment the first was written at.
#[tokio::test]
async fn a_second_comment_leaves_the_first_where_it_was() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool, "sharing").await;

    record_share_comment(&pool, id).await.unwrap();
    let first = written_at(&pool, id).await;

    record_share_comment(&pool, id).await.unwrap();

    assert_eq!(
        written_at(&pool, id).await,
        first,
        "the record says a comment was left, not when the latest one was",
    );
    assert!(share_commented(&pool, id).await.unwrap());
}

/// And it is a fact about the Conversation rather than about the share that was
/// commented: publishing again replaces where the file went, and it does not
/// un-leave a comment.
#[tokio::test]
async fn publishing_again_leaves_the_comment_on_record() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool, "sharing").await;

    record_share(&pool, id, "https://gist.github.com/tobico/9f1")
        .await
        .unwrap();
    record_share_comment(&pool, id).await.unwrap();

    record_share(&pool, id, "https://gist.github.com/tobico/a20")
        .await
        .unwrap();

    assert_eq!(
        share(&pool, id).await.unwrap().map(|share| share.url),
        Some("https://gist.github.com/tobico/a20".to_owned()),
    );
    assert!(
        share_commented(&pool, id).await.unwrap(),
        "a fresh publish is not an unsaying of what was already said",
    );
}

/// And the fact belongs to the Conversation it was written on: another one is
/// not commented because this one is.
#[tokio::test]
async fn the_fact_belongs_to_the_conversation_it_was_written_on() {
    let (_dir, pool) = fresh_pool().await;
    let commented = conversation(&pool, "sharing").await;
    let quiet = conversation(&pool, "rate-limiting").await;

    record_share_comment(&pool, commented).await.unwrap();

    assert!(share_commented(&pool, commented).await.unwrap());
    assert!(!share_commented(&pool, quiet).await.unwrap());
}

/// When the first comment landed, straight out of the row: the store has no
/// reader for it, because nothing but this test has ever wanted it.
async fn written_at(pool: &SqlitePool, conversation_id: i64) -> String {
    let (at,): (String,) =
        sqlx::query_as("SELECT at FROM share_comments WHERE conversation_id = ?")
            .bind(conversation_id)
            .fetch_one(pool)
            .await
            .unwrap();

    at
}
