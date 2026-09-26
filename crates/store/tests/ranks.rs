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
//! And the sidebar's own order is the rank, so what the placement tests used to
//! ask is asked here: the order the human made comes back to them, a reorder
//! names one row rather than the list, and a neighbour that has gone since the
//! list was drawn is passed over rather than refused.
//!
//! The places themselves go once they have become ranks, which is the rest of
//! that rewrite: a database from before comes out of it with the table taken
//! away, and one that never had the table is nothing to do rather than a query
//! that fails. So the old shape is written out here by hand — that code has
//! gone, and what has to keep working is a database rather than a function.
//!
//! The arithmetic itself is tested where it lives, over a few thousand random
//! inserts — see the store's own `ranks` module. What is tested here is what the
//! database does with it.

use std::path::Path;

use sqlx::SqlitePool;
use verkstead_store::{
    archive_conversation, close_conversation, conversations, open_database, rank_conversation,
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

/// And the sidebar as the human sees it, which is that same order with what has
/// been put away left out.
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

/// The table the sidebar's order was kept in before ranks, with the rows the
/// human had dragged into it: the shape and the places both, written out the way
/// `migrations.rs`'s tests write out the old shapes they are about.
async fn placed(pool: &SqlitePool, order: &[i64]) {
    sqlx::query(
        "CREATE TABLE placements (
             conversation_id INTEGER PRIMARY KEY REFERENCES conversations(id),
             place           INTEGER NOT NULL
         ) STRICT",
    )
    .execute(pool)
    .await
    .unwrap();

    for (place, id) in order.iter().enumerate() {
        sqlx::query("INSERT INTO placements (conversation_id, place) VALUES (?, ?)")
            .bind(id)
            .bind(i64::try_from(place).unwrap())
            .execute(pool)
            .await
            .unwrap();
    }
}

/// Whether that table is there at all, which is what the rewrite reads to decide
/// there is anything to do — and what it takes away when it is finished.
async fn places(pool: &SqlitePool) -> bool {
    sqlx::query_scalar::<_, String>(
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'placements'",
    )
    .fetch_optional(pool)
    .await
    .unwrap()
    .is_some()
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
    placed(&pool, &[third, first]).await;

    // And one closed and put away, which is a row the sidebar does not draw and
    // still a row a rank has to reach: a Conversation unarchived later would
    // otherwise be one with nowhere to be.
    close_conversation(&pool, second).await.unwrap();
    archive_conversation(&pool, second).await.unwrap();

    // What the sidebar shows, which is what the old rule drew and what the
    // starts ranked both: the unplaced newest first, above the placed. Which is
    // what makes it the thing to hold the rewrite against.
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

    assert!(
        !places(&pool).await,
        "and the table those places were kept in has gone with them",
    );
}

/// And a database that never had that table — every one made from now on — is
/// nothing for the rewrite to do rather than a query against a table that is not
/// there. It runs at every start, so this is the case it meets on every start but
/// the first.
#[tokio::test]
async fn ranking_a_database_that_never_had_places_ranks_nothing() {
    let (_dir, pool) = fresh_pool().await;
    let repo = repo(&pool).await;

    let first = start(&pool, repo, "first").await;
    let second = start(&pool, repo, "second").await;

    assert!(
        !places(&pool).await,
        "a database made now has no places to read",
    );

    let before = ranks(&pool).await;

    rank_the_conversations(&pool, THIS_DEVICE).await.unwrap();

    assert_eq!(ranks(&pool).await, before, "nothing was rewritten");
    assert_eq!(
        by_rank(&pool).await,
        vec![second, first],
        "and the order is the one the starts made",
    );
}

/// And it runs at every start, so running it twice has to be running it once: a
/// second pass that re-ranked would be a sidebar that moved on a restart. The
/// first pass takes the places away with it, so the second is the case above.
#[tokio::test]
async fn ranking_a_database_twice_ranks_it_once() {
    let dir = tempfile::tempdir().unwrap();
    let database = dir.path().join("verkstead.db");
    let pool = open_database(&database).await.unwrap();

    let repo = repo(&pool).await;
    let first = start(&pool, repo, "first").await;
    let second = start(&pool, repo, "second").await;

    placed(&pool, &[first, second]).await;

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

    // A row from before, which is a place and no rank — and the table being
    // there is what says this database is one the rewrite has anything to do to.
    placed(&pool, &[before]).await;

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

/// Three Conversations on one Repo, newest first, which is the order a start
/// ranks them into — and the Repo, for the one test that starts a fourth.
async fn three(pool: &SqlitePool) -> (i64, i64, i64, i64) {
    let repo = repo(pool).await;

    let first = start(pool, repo, "first").await;
    let second = start(pool, repo, "second").await;
    let third = start(pool, repo, "third").await;

    (repo, first, second, third)
}

/// The rank of one Conversation, whole — the key and the device on it.
async fn rank(pool: &SqlitePool, id: i64) -> String {
    sqlx::query_scalar("SELECT rank FROM conversations WHERE id = ?")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}

/// The whole of what a drag says: this Conversation now sits under that one.
async fn dropped(pool: &SqlitePool, id: i64, below: Option<i64>) {
    rank_conversation(pool, id, below, THIS_DEVICE)
        .await
        .unwrap();
}

/// A card let go at the top of the list, which is the one drop with no row to
/// name: the key minted is outside the range rather than between two of them.
#[tokio::test]
async fn a_row_dropped_at_the_top_is_ranked_above_everything() {
    let (_dir, pool) = fresh_pool().await;
    let (_repo, first, second, third) = three(&pool).await;

    dropped(&pool, first, None).await;

    assert_eq!(sidebar(&pool).await, vec![first, third, second]);
    assert!(
        rank(&pool, first).await < rank(&pool, third).await,
        "the row that moved is above the one that was the top",
    );
}

/// And one let go at the foot, which is the other end of the same thing: the row
/// it lands under is the last one there is, so there is nothing under the gap.
#[tokio::test]
async fn a_row_dropped_at_the_foot_is_ranked_below_everything() {
    let (_dir, pool) = fresh_pool().await;
    let (_repo, first, second, third) = three(&pool).await;

    dropped(&pool, third, Some(first)).await;

    assert_eq!(sidebar(&pool).await, vec![second, first, third]);
    assert!(
        rank(&pool, third).await > rank(&pool, first).await,
        "the row that moved is under the one that was the foot",
    );
}

/// The drop everything else is a special case of: the key minted sorts between
/// its two neighbours **with the suffixes on**, which is what the separator is
/// there to buy — the arithmetic never sees a device, and the strings the
/// database sorts always carry one.
#[tokio::test]
async fn a_row_dropped_between_two_sorts_between_them() {
    let (_dir, pool) = fresh_pool().await;
    let (_repo, first, second, third) = three(&pool).await;

    // The top row, dropped into the gap the other two leave.
    dropped(&pool, third, Some(second)).await;

    assert_eq!(sidebar(&pool).await, vec![second, third, first]);

    let moved = rank(&pool, third).await;

    assert!(
        rank(&pool, second).await < moved && moved < rank(&pool, first).await,
        "the minted rank sorts between its neighbours, suffixes and all: {moved}",
    );
    assert!(
        moved.ends_with(THIS_DEVICE),
        "and the row that moved carries its own device: {moved}",
    );
}

/// A viewer sends the list it drew, and a row can be gone by the time it lands —
/// the way an id in the whole-list order it replaces was passed over rather than
/// refusing the drag it was only partly about. There is nothing left to rank
/// against, so the list stays as the rest of it says.
#[tokio::test]
async fn a_neighbour_that_has_gone_is_not_a_refusal() {
    let (_dir, pool) = fresh_pool().await;
    let (_repo, first, second, third) = three(&pool).await;

    dropped(&pool, third, Some(9_999)).await;

    assert_eq!(sidebar(&pool).await, vec![third, second, first]);
}

/// And an id naming no Conversation at all is the same non-event from the other
/// side: nothing is written, and nothing is refused.
#[tokio::test]
async fn ranking_a_conversation_that_is_not_there_changes_nothing() {
    let (_dir, pool) = fresh_pool().await;
    let (_repo, first, second, third) = three(&pool).await;

    dropped(&pool, 9_999, Some(second)).await;

    assert_eq!(sidebar(&pool).await, vec![third, second, first]);
}

/// What the whole feature is for: the order the human made is still theirs after
/// the database has been closed and opened again. A reload reads the same rows,
/// and a restart reads the same column.
#[tokio::test]
async fn the_order_a_drag_made_survives_a_restart() {
    let dir = tempfile::tempdir().unwrap();
    let database = dir.path().join("verkstead.db");
    let pool = open_database(&database).await.unwrap();

    let (_repo, first, second, third) = three(&pool).await;

    dropped(&pool, second, Some(first)).await;

    let after = vec![third, first, second];
    assert_eq!(sidebar(&pool).await, after);

    pool.close().await;

    let pool = open_database(&database).await.unwrap();

    assert_eq!(
        sidebar(&pool).await,
        after,
        "the order is a column, so a restart reads what the drag wrote",
    );
}

/// A row dropped where it already is has to be a row that has not moved. It is
/// its own neighbour's neighbour, and a key minted between a row and itself is
/// one the arithmetic has nothing to compute.
#[tokio::test]
async fn a_row_dropped_where_it_already_sits_stays_there() {
    let (_dir, pool) = fresh_pool().await;
    let (_repo, first, second, third) = three(&pool).await;

    dropped(&pool, second, Some(third)).await;

    assert_eq!(sidebar(&pool).await, vec![third, second, first]);
}

/// And a Conversation started while the sidebar is open arrives at the top of
/// whatever the human has dragged it into, rather than under it: every start
/// ranks above everything, which is the rule that replaces *the unplaced float
/// to the top*.
#[tokio::test]
async fn a_conversation_started_after_a_drag_lands_at_the_top() {
    let (_dir, pool) = fresh_pool().await;
    let (repo, first, second, third) = three(&pool).await;

    dropped(&pool, third, Some(first)).await;

    let fourth = start(&pool, repo, "fourth").await;

    assert_eq!(sidebar(&pool).await, vec![fourth, second, first, third]);
}
