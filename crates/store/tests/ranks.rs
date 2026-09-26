//! The Rank every Conversation carries: where it sits in the sidebar, as a
//! fractional-indexing key with the device that issued it suffixed after it
//! (ADR-0020, *Ranks*).
//!
//! What is worth a test here is what a promise could not keep. A database
//! written before the column existed opens with every row of it ranked, and
//! ranked in the order that human's sidebar has been drawing them in all along —
//! their own hand-made order rearranged by an upgrade is the one thing this must
//! not do. A Conversation started now is ranked above everything, so the newest
//! is the first row whether or not anybody has ever dragged one, and a hundred
//! started in a row are a hundred distinct ranks rather than a key that ran out.
//! And two devices ranking above their own list are two ranks rather than a tie,
//! which is what the suffix is for.
//!
//! The arithmetic itself is tested where it lives, over a few thousand random
//! inserts — see the store's own `ranks` module. What is tested here is what the
//! database does with it.

use std::path::Path;

use sqlx::SqlitePool;
use verkstead_store::{
    archive_conversation, close_conversation, conversations, open_database, place_conversations,
    rank_the_conversations, register_repo, start_conversation,
};

/// The device every Conversation started here is ranked by, named the way a
/// cluster names one (ADR-0020, *Ranks*).
const THIS_DEVICE: &str = "aa00bb11cc22dd33ee44ff5566778899";

/// And a second machine, for the one question that takes two: what two devices
/// ranking above their own empty list mint.
const ANOTHER_DEVICE: &str = "0011223344556677889900aabbccddee";

/// A pool over a fresh database, plus the directory keeping it alive.
async fn fresh_pool() -> (tempfile::TempDir, SqlitePool) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();
    (dir, pool)
}

/// A Repo to hang Conversations off.
async fn repo(pool: &SqlitePool) -> i64 {
    register_repo(pool, Path::new("/watched/verkstead"), "verkstead", "main")
        .await
        .unwrap()
        .expect("nothing was registered at that path yet")
        .id
}

/// A Conversation on that Repo, started by this device.
async fn start(pool: &SqlitePool, repo: i64, branch: &str) -> i64 {
    start_conversation(pool, repo, branch, THIS_DEVICE)
        .await
        .unwrap()
        .expect("the Repo was just registered")
}

/// Every Conversation in rank order, which is the order the sidebar will be
/// drawn in.
async fn by_rank(pool: &SqlitePool) -> Vec<i64> {
    sqlx::query_scalar("SELECT id FROM conversations ORDER BY rank")
        .fetch_all(pool)
        .await
        .unwrap()
}

/// And the sidebar as it stands today, which is still the place order.
async fn sidebar(pool: &SqlitePool) -> Vec<i64> {
    conversations(pool)
        .await
        .unwrap()
        .into_iter()
        .map(|row| row.id)
        .collect()
}

/// The rank of each Conversation, by id.
async fn ranks(pool: &SqlitePool) -> Vec<(i64, Option<String>)> {
    sqlx::query_as("SELECT id, rank FROM conversations ORDER BY id")
        .fetch_all(pool)
        .await
        .unwrap()
}

/// A Conversation is ranked as it is started, so the newest is the first row
/// with nobody having dragged anything — which is what makes *the unplaced float
/// to the top* a rule the sidebar will not need.
#[tokio::test]
async fn back_to_back_starts_are_newest_first() {
    let (_dir, pool) = fresh_pool().await;
    let repo = repo(&pool).await;

    let first = start(&pool, repo, "first").await;
    let second = start(&pool, repo, "second").await;

    assert_eq!(
        by_rank(&pool).await,
        vec![second, first],
        "the second was ranked above the first, which is where a start ranks",
    );

    let ranks = ranks(&pool).await;

    assert_ne!(
        ranks[0].1, ranks[1].1,
        "two starts a moment apart are two ranks: {ranks:?}",
    );
}

/// And a hundred of them are a hundred ranks in that same order, which is what
/// says the key grows rather than running out: ranking above the top walks the
/// alphabet down and then carries into the head.
#[tokio::test]
async fn a_hundred_starts_are_a_hundred_ranks_newest_first() {
    let (_dir, pool) = fresh_pool().await;
    let repo = repo(&pool).await;

    let mut started = Vec::new();

    for branch in 0..100 {
        started.push(start(&pool, repo, &format!("branch-{branch}")).await);
    }

    started.reverse();

    assert_eq!(by_rank(&pool).await, started, "newest first, all hundred");

    let mut minted: Vec<String> = ranks(&pool)
        .await
        .into_iter()
        .map(|(id, rank)| {
            rank.unwrap_or_else(|| panic!("Conversation {id} was ranked at its start"))
        })
        .collect();

    minted.sort();
    minted.dedup();

    assert_eq!(minted.len(), 100, "and no two of them share a rank");
}

/// Two devices ranking above their own empty list mint the same key — a fresh
/// device's first rows are exactly that rather than a rare coincidence — and the
/// suffix is what keeps them two ranks rather than a tie nothing could be dragged
/// between.
#[tokio::test]
async fn two_devices_ranking_above_nothing_sort_apart() {
    let (_mine, mine) = fresh_pool().await;
    let (_theirs, theirs) = fresh_pool().await;

    let here = repo(&mine).await;
    let there = repo(&theirs).await;

    start_conversation(&mine, here, "first", THIS_DEVICE)
        .await
        .unwrap()
        .unwrap();
    start_conversation(&theirs, there, "first", ANOTHER_DEVICE)
        .await
        .unwrap()
        .unwrap();

    let here = ranks(&mine).await[0].1.clone().unwrap();
    let there = ranks(&theirs).await[0].1.clone().unwrap();

    assert_ne!(here, there, "two devices, two ranks");
    assert!(
        here.ends_with(THIS_DEVICE) && there.ends_with(ANOTHER_DEVICE),
        "each rank names the device that issued it: {here} and {there}",
    );
    assert_eq!(
        here.trim_end_matches(THIS_DEVICE),
        there.trim_end_matches(ANOTHER_DEVICE),
        "and the keys under them are the same one, computed apart",
    );
}

/// The whole of what the rewrite is for: a database written before the column
/// existed opens with every Conversation ranked, and in rank order they are in
/// the order that sidebar has been drawing them in — what the human placed in
/// their place order, with what nobody placed above it newest first.
///
/// Read against the place order itself rather than against a list written out
/// here, because agreeing with it is the promise: the rule this replaces is the
/// only thing that says where those rows belong.
#[tokio::test]
async fn a_database_from_before_ranks_opens_in_the_order_it_was_drawn_in() {
    let dir = tempfile::tempdir().unwrap();
    let database = dir.path().join("verkstead.db");
    let pool = open_database(&database).await.unwrap();

    let repo = repo(&pool).await;
    let first = start(&pool, repo, "first").await;
    let second = start(&pool, repo, "second").await;
    let third = start(&pool, repo, "third").await;
    let fourth = start(&pool, repo, "fourth").await;

    // Two of them dragged into an order of the human's own, which leaves two
    // that nobody has ever placed.
    place_conversations(&pool, &[third, first]).await.unwrap();

    // And one closed and put away, which is a row the sidebar does not draw and
    // still a row a rank has to reach: a Conversation unarchived later would
    // otherwise be one with nowhere to be.
    close_conversation(&pool, second).await.unwrap();
    archive_conversation(&pool, second).await.unwrap();

    let drawn = sidebar(&pool).await;

    assert_eq!(
        drawn,
        vec![fourth, third, first],
        "the unplaced above the placed, and the archived one not drawn at all",
    );

    // The column off, which is the whole of what says this database is one from
    // before: the sidebar's order was the places and nothing else.
    sqlx::query("ALTER TABLE conversations DROP COLUMN rank")
        .execute(&pool)
        .await
        .unwrap();

    pool.close().await;

    let pool = open_database(&database).await.unwrap();

    assert!(
        ranks(&pool).await.iter().all(|(_, rank)| rank.is_none()),
        "the column arrives empty: the open cannot know which device this is",
    );

    rank_the_conversations(&pool, THIS_DEVICE).await.unwrap();

    assert_eq!(
        by_rank(&pool).await,
        vec![fourth, second, third, first],
        "every row ranked, in the order the place order puts them in",
    );
    assert_eq!(
        sidebar(&pool).await,
        drawn,
        "and the sidebar itself draws exactly what it drew before",
    );

    for (id, rank) in ranks(&pool).await {
        let rank = rank.unwrap_or_else(|| panic!("Conversation {id} was ranked"));

        assert!(
            rank.ends_with(THIS_DEVICE),
            "Conversation {id} was ranked by this device: {rank}",
        );
    }
}

/// And it runs at every start, so running it twice has to be running it once: a
/// second pass that re-ranked would be a sidebar that moved on a restart.
#[tokio::test]
async fn ranking_a_database_twice_ranks_it_once() {
    let dir = tempfile::tempdir().unwrap();
    let database = dir.path().join("verkstead.db");
    let pool = open_database(&database).await.unwrap();

    let repo = repo(&pool).await;
    let first = start(&pool, repo, "first").await;
    let second = start(&pool, repo, "second").await;

    place_conversations(&pool, &[first, second]).await.unwrap();

    sqlx::query("ALTER TABLE conversations DROP COLUMN rank")
        .execute(&pool)
        .await
        .unwrap();

    pool.close().await;

    let pool = open_database(&database).await.unwrap();

    rank_the_conversations(&pool, THIS_DEVICE).await.unwrap();

    let once = ranks(&pool).await;

    rank_the_conversations(&pool, THIS_DEVICE).await.unwrap();

    assert_eq!(
        ranks(&pool).await,
        once,
        "the second pass found nothing to do"
    );
}

/// A Conversation started before the rewrite gets to a database is ranked above
/// everything, as every start is — so the rows from before land underneath it,
/// in their own order, rather than on top of it.
///
/// Unreachable through a serve, which ranks the database before it answers
/// anything. It is the one thing that could put a ranked row and an unranked one
/// in the same table, and a rewrite that started from the top would have
/// shuffled them together.
#[tokio::test]
async fn rows_from_before_land_under_what_is_already_ranked() {
    let dir = tempfile::tempdir().unwrap();
    let database = dir.path().join("verkstead.db");
    let pool = open_database(&database).await.unwrap();

    let repo = repo(&pool).await;
    let before = start(&pool, repo, "before").await;

    sqlx::query("UPDATE conversations SET rank = NULL")
        .execute(&pool)
        .await
        .unwrap();

    let started_first = start(&pool, repo, "started-first").await;

    rank_the_conversations(&pool, THIS_DEVICE).await.unwrap();

    assert_eq!(
        by_rank(&pool).await,
        vec![started_first, before],
        "what was ranked keeps its place, and the row from before goes under it",
    );
}
