//! The Previews a building session suggests for trying its work by hand
//! (ADR-0024), as the store keeps them.

use std::path::Path;

use sqlx::SqlitePool;
use verkstead_store::{
    open_database, register_repo, retire_preview, standing_previews, start_conversation,
    suggest_preview, timeline,
};

/// The device every Conversation started here is ranked by, named the way a
/// cluster names one (ADR-0020, *Ranks*).
const THIS_DEVICE: &str = "aa00bb11cc22dd33ee44ff5566778899";

/// A pool over a fresh database, plus the directory keeping it alive.
async fn fresh_pool() -> (tempfile::TempDir, SqlitePool) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();
    (dir, pool)
}

/// A Conversation to suggest Previews on.
async fn conversation(pool: &SqlitePool, branch: &str) -> i64 {
    let repo = register_repo(pool, Path::new("/watched/verkstead"), "verkstead", "main")
        .await
        .unwrap()
        .map(|repo| repo.id)
        .unwrap_or(1);

    start_conversation(pool, repo, branch, THIS_DEVICE)
        .await
        .unwrap()
        .expect("the Repo is registered")
}

/// The titles and commands standing, in order.
async fn standing(pool: &SqlitePool, id: i64) -> Vec<(String, String)> {
    standing_previews(pool, id)
        .await
        .unwrap()
        .into_iter()
        .map(|preview| (preview.title, preview.command))
        .collect()
}

#[tokio::test]
async fn a_preview_stands_oldest_first_with_its_stamp() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool, "rate-limiting").await;

    for (title, command) in [("Build and test", "cargo test"), ("Serve", "cargo run")] {
        assert!(
            suggest_preview(&pool, id, title, command, "the session that worked task 01")
                .await
                .unwrap()
        );
    }

    let previews = standing_previews(&pool, id).await.unwrap();

    assert_eq!(previews.len(), 2, "{previews:?}");
    assert_eq!(previews[0].title, "Build and test");
    assert_eq!(previews[0].command, "cargo test");
    assert_eq!(previews[0].left_by, "the session that worked task 01");
    assert!(!previews[0].at.is_empty());
    assert_eq!(previews[1].title, "Serve");
}

/// The same title again replaces the command and the stamp, and keeps its
/// place in the list.
#[tokio::test]
async fn the_same_title_again_replaces_it_in_place() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool, "rate-limiting").await;

    suggest_preview(
        &pool,
        id,
        "Build and test",
        "cargo test",
        "the session that worked task 01",
    )
    .await
    .unwrap();
    suggest_preview(
        &pool,
        id,
        "Serve",
        "cargo run",
        "the session that worked task 01",
    )
    .await
    .unwrap();
    suggest_preview(
        &pool,
        id,
        "Build and test",
        "cargo test --workspace",
        "the session that worked task 02",
    )
    .await
    .unwrap();

    let previews = standing_previews(&pool, id).await.unwrap();

    assert_eq!(
        standing(&pool, id).await,
        [
            (
                "Build and test".to_owned(),
                "cargo test --workspace".to_owned()
            ),
            ("Serve".to_owned(), "cargo run".to_owned()),
        ],
    );
    assert_eq!(previews[0].left_by, "the session that worked task 02");
}

/// A Preview is not an Event: nothing lands on the Timeline.
#[tokio::test]
async fn a_preview_puts_nothing_on_the_timeline() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool, "rate-limiting").await;
    let before = timeline(&pool, id).await.unwrap().len();

    suggest_preview(
        &pool,
        id,
        "Build and test",
        "cargo test",
        "the finish session",
    )
    .await
    .unwrap();

    assert_eq!(timeline(&pool, id).await.unwrap().len(), before);
}

#[tokio::test]
async fn retiring_takes_only_that_title_back() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool, "rate-limiting").await;
    let other = conversation(&pool, "other-work").await;

    suggest_preview(
        &pool,
        id,
        "Build and test",
        "cargo test",
        "the finish session",
    )
    .await
    .unwrap();
    suggest_preview(
        &pool,
        other,
        "Build and test",
        "npm test",
        "the finish session",
    )
    .await
    .unwrap();

    assert!(
        !retire_preview(&pool, id, "Serve").await.unwrap(),
        "none stands"
    );
    assert!(retire_preview(&pool, id, "Build and test").await.unwrap());
    assert!(
        !retire_preview(&pool, id, "Build and test").await.unwrap(),
        "and once gone, it is gone",
    );

    assert!(standing(&pool, id).await.is_empty());
    assert_eq!(
        standing(&pool, other).await,
        [("Build and test".to_owned(), "npm test".to_owned())],
        "another Conversation's of the same title stands",
    );
}

#[tokio::test]
async fn a_preview_on_no_conversation_is_said_so() {
    let (_dir, pool) = fresh_pool().await;

    assert!(
        !suggest_preview(
            &pool,
            42,
            "Build and test",
            "cargo test",
            "the finish session"
        )
        .await
        .unwrap()
    );
}
