//! The mark saying Verkstead has told the human something about a Conversation
//! that they have not looked at yet.
//!
//! What is being asked of the store here is *there is news on this one*. The
//! row being there is the whole of the mark, so the rules worth a test are the
//! ones a promise could not keep: stamping twice is one mark, looking takes it
//! away for good, and the sidebar reads it back beside — never instead of —
//! what is waiting on the human.

use std::path::Path;

use sqlx::SqlitePool;
use verkstead_store::{
    conversations, open_database, register_repo, see_conversation, stamp_unseen, start_conversation,
};

/// A pool over a fresh database, plus the directory keeping it alive.
async fn fresh_pool() -> (tempfile::TempDir, SqlitePool) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();
    (dir, pool)
}

/// A Conversation to leave news on.
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

/// Whether the sidebar says there is news on one.
async fn unseen(pool: &SqlitePool, id: i64) -> bool {
    conversations(pool)
        .await
        .unwrap()
        .into_iter()
        .find(|row| row.id == id)
        .expect("the Conversation is on the sidebar")
        .unseen
}

/// The mark goes on, the sidebar says so, and looking takes it off.
#[tokio::test]
async fn news_stands_on_the_row_until_the_conversation_is_looked_at() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool, "rate-limiting").await;

    assert!(
        !unseen(&pool, id).await,
        "a Conversation nobody has been told anything about has no news on it",
    );

    stamp_unseen(&pool, id).await.unwrap();
    assert!(unseen(&pool, id).await, "and now there is");

    assert!(
        see_conversation(&pool, id).await.unwrap(),
        "looking at it found a mark to take away",
    );
    assert!(!unseen(&pool, id).await, "which is off the row");
}

/// Told twice is one mark, and looked at twice is one clearing.
///
/// The answer to the second look is what the server reads to decide whether to
/// tell the other devices: a Conversation opened again in a session of reading
/// should not send every sidebar back to the list for nothing.
#[tokio::test]
async fn there_is_no_such_thing_as_twice_as_unseen() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool, "rate-limiting").await;

    stamp_unseen(&pool, id).await.unwrap();
    stamp_unseen(&pool, id).await.unwrap();

    assert!(
        see_conversation(&pool, id).await.unwrap(),
        "one look clears what two tellings wrote",
    );
    assert!(!unseen(&pool, id).await);

    assert!(
        !see_conversation(&pool, id).await.unwrap(),
        "and the second look finds nothing left to take away",
    );
}

/// Looking at something that was never marked, or that is not there at all, is
/// not a failure: opening a Conversation is not a claim about it.
#[tokio::test]
async fn looking_at_what_carries_no_mark_is_nothing_happening() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool, "rate-limiting").await;

    assert!(!see_conversation(&pool, id).await.unwrap());
    assert!(!see_conversation(&pool, 404).await.unwrap());
}

/// The mark is one Conversation's. Telling the human about one leaves every
/// other row exactly as it was.
#[tokio::test]
async fn the_news_is_on_the_conversation_it_is_about() {
    let (_dir, pool) = fresh_pool().await;
    let told = conversation(&pool, "rate-limiting").await;
    let quiet = conversation(&pool, "usage-limits").await;

    stamp_unseen(&pool, told).await.unwrap();

    assert!(unseen(&pool, told).await);
    assert!(!unseen(&pool, quiet).await);

    see_conversation(&pool, told).await.unwrap();

    assert!(!unseen(&pool, told).await);
    assert!(!unseen(&pool, quiet).await);
}
