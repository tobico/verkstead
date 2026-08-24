//! The order the sidebar is in: what the human said by dragging, and what
//! happens to a Conversation nobody has placed.
//!
//! What is being asked of the store here is *the list stays where it was put*.
//! The rules worth a test are the ones a promise could not keep: an order
//! survives being written and read back, one that was placed and then started
//! again is replaced rather than added to, and a Conversation with no place of
//! its own lands somewhere stated instead of wherever the join happens to put
//! it.

use std::path::Path;

use sqlx::SqlitePool;
use verkstead_store::{
    conversations, open_database, place_conversations, register_repo, start_conversation,
};

/// A pool over a fresh database, plus the directory keeping it alive.
async fn fresh_pool() -> (tempfile::TempDir, SqlitePool) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();
    (dir, pool)
}

/// Three Conversations in one Repo, in the order they were started.
async fn three(pool: &SqlitePool) -> (i64, i64, i64) {
    let repo = register_repo(pool, Path::new("/watched/verkstead"), "verkstead", "main")
        .await
        .unwrap()
        .expect("nothing was registered at that path yet")
        .id;

    let mut started = Vec::new();
    for branch in ["first", "second", "third"] {
        started.push(
            start_conversation(pool, repo, branch)
                .await
                .unwrap()
                .expect("the Repo was just registered"),
        );
    }

    (started[0], started[1], started[2])
}

/// The sidebar as it stands, by id.
async fn sidebar(pool: &SqlitePool) -> Vec<i64> {
    conversations(pool)
        .await
        .unwrap()
        .into_iter()
        .map(|row| row.id)
        .collect()
}

#[tokio::test]
async fn nothing_placed_is_newest_first() {
    let (_dir, pool) = fresh_pool().await;
    let (first, second, third) = three(&pool).await;

    assert_eq!(
        sidebar(&pool).await,
        vec![third, second, first],
        "with nobody having placed anything, the list is the order they were started in, reversed",
    );
}

#[tokio::test]
async fn the_order_the_human_gave_is_the_order_they_get_back() {
    let (_dir, pool) = fresh_pool().await;
    let (first, second, third) = three(&pool).await;

    place_conversations(&pool, &[second, third, first])
        .await
        .unwrap();

    assert_eq!(
        sidebar(&pool).await,
        vec![second, third, first],
        "the list is what was placed, in the order it was placed in",
    );
}

#[tokio::test]
async fn a_second_order_replaces_the_first() {
    let (_dir, pool) = fresh_pool().await;
    let (first, second, third) = three(&pool).await;

    place_conversations(&pool, &[second, third, first])
        .await
        .unwrap();
    place_conversations(&pool, &[first, second, third])
        .await
        .unwrap();

    assert_eq!(
        sidebar(&pool).await,
        vec![first, second, third],
        "a drag says where the whole list goes, so the order before it is gone rather than under it",
    );
}

#[tokio::test]
async fn what_was_never_placed_goes_to_the_top() {
    let (_dir, pool) = fresh_pool().await;
    let (first, second, third) = three(&pool).await;

    place_conversations(&pool, &[third, first]).await.unwrap();

    assert_eq!(
        sidebar(&pool).await,
        vec![second, third, first],
        "the one nobody placed is above the two who were, and the two keep their order",
    );
}

/// A viewer sends the list it drew, which is a moment old by the time it lands.
#[tokio::test]
async fn an_id_naming_no_conversation_is_passed_over() {
    let (_dir, pool) = fresh_pool().await;
    let (first, second, third) = three(&pool).await;

    place_conversations(&pool, &[second, 9_999, first, third])
        .await
        .unwrap();

    assert_eq!(
        sidebar(&pool).await,
        vec![second, first, third],
        "the order stands for the Conversations that are there, and the id that names none is dropped",
    );
}

/// A viewer with a mistake in it, rather than anything the human did.
#[tokio::test]
async fn an_id_sent_twice_keeps_the_place_it_was_first_given() {
    let (_dir, pool) = fresh_pool().await;
    let (first, second, third) = three(&pool).await;

    place_conversations(&pool, &[second, first, second, third])
        .await
        .unwrap();

    assert_eq!(
        sidebar(&pool).await,
        vec![second, first, third],
        "the first place it was given stands, and every row is still placed",
    );
}
