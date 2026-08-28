//! The rows that fix where a backlog and a roadmap landed on a Conversation's
//! branch.
//!
//! Neither row carries anything. What a Timeline draws at one is the list as
//! the Worktree holds it when somebody looks — the repository owns those files
//! — so what is stored is the position alone: the moment the work stopped being
//! a plan and became a list to work through.
//!
//! Which makes *once* the whole of the rule. A run that is seen out twice, or
//! one taken up again after a stop, reaches the same landing again and finds
//! the row already there.

use std::path::Path;

use sqlx::SqlitePool;
use verkstead_store::{
    Event, Landed, open_database, record_backlog, record_roadmap, register_repo, save_brief,
    start_conversation, start_grilling, timeline,
};

/// A pool over a fresh database, plus the directory keeping it alive.
async fn fresh_pool() -> (tempfile::TempDir, SqlitePool) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();
    (dir, pool)
}

/// A Conversation with a branch under it, which is what a list lands on.
///
/// Walked there rather than moved by hand: every state on the way records
/// something, and a Conversation dropped straight into one would be one nothing
/// else in the store agrees about.
async fn grilling(pool: &SqlitePool) -> i64 {
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
        &[],
    )
    .await
    .unwrap();

    id
}

/// What a Conversation's Timeline holds, in order.
async fn events(pool: &SqlitePool, id: i64) -> Vec<Event> {
    timeline(pool, id)
        .await
        .unwrap()
        .into_iter()
        .map(|event| event.event)
        .collect()
}

/// The backlog landing puts a row on the record, at the end of what has
/// happened so far.
#[tokio::test]
async fn a_backlog_landing_is_stamped_where_it_landed() {
    let (_dir, pool) = fresh_pool().await;
    let id = grilling(&pool).await;

    assert_eq!(
        record_backlog(&pool, id).await.unwrap(),
        Landed::Stamped,
        "the branch did not carry a backlog until now",
    );

    let events = events(&pool, id).await;

    assert_eq!(
        events.last(),
        Some(&Event::TaskList),
        "the row is the last thing to have happened: {events:?}",
    );
}

/// And the roadmap's, which is the same thing one level up and its own row.
#[tokio::test]
async fn a_roadmap_landing_is_stamped_beside_it() {
    let (_dir, pool) = fresh_pool().await;
    let id = grilling(&pool).await;

    assert_eq!(record_roadmap(&pool, id).await.unwrap(), Landed::Stamped);
    assert_eq!(record_backlog(&pool, id).await.unwrap(), Landed::Stamped);

    let events = events(&pool, id).await;

    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, Event::TaskList | Event::StageList))
            .collect::<Vec<_>>(),
        [&Event::StageList, &Event::TaskList],
        "two rows, in the order the two lists landed: {events:?}",
    );
}

/// A list lands once. A run seen out a second time — or taken up again after a
/// stop — finds the row already on the record, and nothing is written.
#[tokio::test]
async fn a_second_landing_writes_nothing() {
    let (_dir, pool) = fresh_pool().await;
    let id = grilling(&pool).await;

    assert_eq!(record_backlog(&pool, id).await.unwrap(), Landed::Stamped);
    assert_eq!(
        record_backlog(&pool, id).await.unwrap(),
        Landed::Already,
        "the second sighting of the same landing",
    );
    assert_eq!(record_roadmap(&pool, id).await.unwrap(), Landed::Stamped);
    assert_eq!(record_roadmap(&pool, id).await.unwrap(), Landed::Already);

    assert_eq!(
        events(&pool, id)
            .await
            .iter()
            .filter(|event| matches!(event, Event::TaskList | Event::StageList))
            .count(),
        2,
        "one row each, however many times the landing is seen",
    );
}

/// And a Conversation that is not there is refused by name rather than written
/// against nothing.
#[tokio::test]
async fn there_is_nothing_to_stamp_on_a_conversation_that_is_not_there() {
    let (_dir, pool) = fresh_pool().await;

    assert_eq!(
        record_backlog(&pool, 404).await.unwrap(),
        Landed::NoSuchConversation,
    );
    assert_eq!(
        record_roadmap(&pool, 404).await.unwrap(),
        Landed::NoSuchConversation,
    );
}
