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
    stage_roadmap, start_conversation, start_grilling, timeline,
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

    assert_eq!(
        record_roadmap(&pool, id, None).await.unwrap(),
        Landed::Stamped
    );
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
    assert_eq!(
        record_roadmap(&pool, id, None).await.unwrap(),
        Landed::Stamped
    );
    assert_eq!(
        record_roadmap(&pool, id, None).await.unwrap(),
        Landed::Already
    );

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
        record_roadmap(&pool, 404, None).await.unwrap(),
        Landed::NoSuchConversation,
    );
}

/// The roadmap a branch wrote is recorded with the landing, and that name is
/// what every wrap-up from here carries the effort on into.
///
/// Writing a roadmap is choosing it — there is no other moment at which this
/// Conversation's roadmap is settled — so the name arrives with the row saying
/// the roadmap landed.
#[tokio::test]
async fn the_roadmap_a_branch_wrote_is_recorded_with_the_landing() {
    let (_dir, pool) = fresh_pool().await;
    let id = grilling(&pool).await;

    assert_eq!(
        stage_roadmap(&pool, id).await.unwrap(),
        None,
        "nothing is recorded before the roadmap lands",
    );

    assert_eq!(
        record_roadmap(&pool, id, Some("mvp")).await.unwrap(),
        Landed::Stamped,
    );

    assert_eq!(
        stage_roadmap(&pool, id).await.unwrap().as_deref(),
        Some("mvp")
    );
}

/// A branch that wrote no roadmap of its own — or wrote two, which is two
/// efforts planned in one go — has none recorded, and the landing is stamped
/// all the same.
///
/// Which of the two it was is the caller's reading of the branch. What reaches
/// here either way is nothing to record, and the wrap-up starts no stage.
#[tokio::test]
async fn a_landing_with_no_single_roadmap_records_no_name() {
    let (_dir, pool) = fresh_pool().await;
    let id = grilling(&pool).await;

    assert_eq!(
        record_roadmap(&pool, id, None).await.unwrap(),
        Landed::Stamped,
    );

    assert_eq!(stage_roadmap(&pool, id).await.unwrap(), None);
}

/// And the name is settled once. A run taken up again sees the same roadmap on
/// the same branch, and what is stored does not move.
#[tokio::test]
async fn a_second_landing_leaves_the_recorded_roadmap_alone() {
    let (_dir, pool) = fresh_pool().await;
    let id = grilling(&pool).await;

    record_roadmap(&pool, id, Some("mvp")).await.unwrap();

    assert_eq!(
        record_roadmap(&pool, id, Some("public-release"))
            .await
            .unwrap(),
        Landed::Already,
    );

    assert_eq!(
        stage_roadmap(&pool, id).await.unwrap().as_deref(),
        Some("mvp"),
        "the first sighting is the one that settled it",
    );
}

/// A Conversation that is not there has nothing to record a roadmap against,
/// and the refusal is the landing's own.
#[tokio::test]
async fn a_roadmap_is_not_recorded_against_a_conversation_that_is_not_there() {
    let (_dir, pool) = fresh_pool().await;

    assert_eq!(
        record_roadmap(&pool, 404, Some("mvp")).await.unwrap(),
        Landed::NoSuchConversation,
    );

    assert_eq!(stage_roadmap(&pool, 404).await.unwrap(), None);
}
