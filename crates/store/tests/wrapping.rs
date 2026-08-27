//! The pull request a Conversation's work was carried to, and the move into
//! Wrapping that recording one is.
//!
//! Recording the PR is not a thing that happens beside the move — it *is* the
//! move, in one transaction. So what these ask is what a Conversation and its
//! Timeline say afterwards: the state, the PR Event, and the move under it.
//!
//! Two states get here, because two kinds of work end on a pull request: a
//! backlog worked to empty, from Implementing, and a roadmap, from Grilling.
//!
//! And a Conversation can arrive twice. A review that split its findings out
//! into a backlog sends the work back to be built, and its finish step wraps up
//! again on the pull request it already had.

use std::path::Path;

use sqlx::SqlitePool;
use verkstead_store::{
    Event, Lifecycle, PullRequest, Rebuilding, Wrapping, close_conversation, implement_again,
    load_conversation, open_database, pick_direction, pull_request, record_pull_request,
    register_repo, save_brief, start_conversation, start_grilling, timeline,
};

/// A pool over a fresh database, plus the directory keeping it alive.
async fn fresh_pool() -> (tempfile::TempDir, SqlitePool) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();
    (dir, pool)
}

/// A Conversation that is grilling, which is where every direction is picked and
/// where a roadmap Conversation still is when its pull request opens.
///
/// Walked there rather than moved by hand: every state on the way records
/// something — the base commit, the worktree — and a Conversation dropped
/// straight into one would be one nothing else in the store agrees about.
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

/// And the same one carried on to having its work built, which is where a finish
/// step reaches one. Inline, because that is the direction whose pick makes the
/// move.
async fn implementing(pool: &SqlitePool) -> i64 {
    let id = grilling(pool).await;

    pick_direction(pool, id, verkstead_schema::Direction::Inline)
        .await
        .unwrap();

    id
}

/// The PR the finish step opened, as the host's `gh` read it back.
fn opened() -> PullRequest {
    PullRequest {
        number: 41,
        title: "Rate limiting".to_owned(),
        url: "https://github.com/tobico/verkstead/pull/41".to_owned(),
    }
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

/// The whole of it: the PR is recorded, the Conversation is wrapping, and both
/// the PR and the move are on the Timeline.
#[tokio::test]
async fn recording_the_pull_request_moves_the_conversation_into_wrapping() {
    let (_dir, pool) = fresh_pool().await;
    let id = implementing(&pool).await;

    assert_eq!(
        record_pull_request(&pool, id, &opened()).await.unwrap(),
        Wrapping::Started,
    );

    let conversation = load_conversation(&pool, id).await.unwrap().unwrap();
    assert_eq!(conversation.state, Lifecycle::Wrapping);

    let events = events(&pool, id).await;

    assert_eq!(
        events.last(),
        Some(&Event::Moved(Lifecycle::Wrapping)),
        "the move is the last thing on the Timeline: {events:?}",
    );
    assert_eq!(
        events[events.len() - 2],
        Event::PullRequest(opened()),
        "and the PR is what it moved on: {events:?}",
    );
}

/// A roadmap Conversation gets here from Grilling, with no Implementing on the
/// way: the session that settled the work wrote the roadmap and carried the
/// branch to a pull request without ever leaving the grilling, because the
/// building belongs to the Stages it planned.
#[tokio::test]
async fn a_roadmap_conversation_wraps_up_straight_out_of_its_grilling() {
    let (_dir, pool) = fresh_pool().await;
    let id = grilling(&pool).await;

    pick_direction(&pool, id, verkstead_schema::Direction::Roadmap)
        .await
        .unwrap();

    assert_eq!(
        load_conversation(&pool, id).await.unwrap().unwrap().state,
        Lifecycle::Grilling,
        "the pick moved nothing: the grilling is what writes the roadmap",
    );

    assert_eq!(
        record_pull_request(&pool, id, &opened()).await.unwrap(),
        Wrapping::Started,
    );

    let conversation = load_conversation(&pool, id).await.unwrap().unwrap();
    assert_eq!(conversation.state, Lifecycle::Wrapping);

    assert_eq!(
        events(&pool, id)
            .await
            .into_iter()
            .filter_map(|event| match event {
                Event::Moved(state) => Some(state),
                _ => None,
            })
            .collect::<Vec<_>>(),
        [Lifecycle::Grilling, Lifecycle::Wrapping],
        "and the ladder skips Implementing rather than idling in it",
    );
}

/// A second attempt at the same finish finds the move already made. Nothing is
/// recorded twice — the state check is what says so, and it is read inside the
/// same transaction the insert is in.
#[tokio::test]
async fn a_conversation_that_is_already_wrapping_records_no_second_pull_request() {
    let (_dir, pool) = fresh_pool().await;
    let id = implementing(&pool).await;

    record_pull_request(&pool, id, &opened()).await.unwrap();

    assert_eq!(
        record_pull_request(&pool, id, &opened()).await.unwrap(),
        Wrapping::NothingToWrap,
    );

    let requests = events(&pool, id)
        .await
        .into_iter()
        .filter(|event| matches!(event, Event::PullRequest(_)))
        .count();

    assert_eq!(requests, 1, "one Conversation, one pull request");
}

/// A Conversation closed out from under the run is not one to move into
/// Wrapping, however far the session it was running had got.
#[tokio::test]
async fn a_closed_conversation_is_not_moved_on_by_a_pull_request() {
    let (_dir, pool) = fresh_pool().await;
    let id = implementing(&pool).await;

    close_conversation(&pool, id).await.unwrap();

    assert_eq!(
        record_pull_request(&pool, id, &opened()).await.unwrap(),
        Wrapping::NothingToWrap,
    );

    let conversation = load_conversation(&pool, id).await.unwrap().unwrap();
    assert_eq!(conversation.state, Lifecycle::Closed);
}

#[tokio::test]
async fn a_conversation_that_is_not_there_records_nothing() {
    let (_dir, pool) = fresh_pool().await;

    assert_eq!(
        record_pull_request(&pool, 404, &opened()).await.unwrap(),
        Wrapping::NoSuchConversation,
    );
}

/// A second wrap records nothing new. The Conversation left Wrapping to build a
/// backlog its review split out, and what its finish step opened is the pull
/// request it already had — so the record is reused, and what says the work came
/// round again is the lifecycle moves either side of it.
#[tokio::test]
async fn a_second_wrap_reuses_the_pull_request_the_first_one_recorded() {
    let (_dir, pool) = fresh_pool().await;
    let id = implementing(&pool).await;

    record_pull_request(&pool, id, &opened()).await.unwrap();

    assert_eq!(
        implement_again(&pool, id).await.unwrap(),
        Rebuilding::Started,
    );
    assert_eq!(
        load_conversation(&pool, id).await.unwrap().unwrap().state,
        Lifecycle::Implementing,
        "the split-out work is built, and building is Implementing",
    );

    assert_eq!(
        record_pull_request(&pool, id, &opened()).await.unwrap(),
        Wrapping::Started,
        "and the finish that follows the backlog wraps it up again",
    );

    let events = events(&pool, id).await;

    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, Event::PullRequest(_)))
            .count(),
        1,
        "one Conversation, one pull request, however many times it wraps up: {events:?}",
    );
    assert_eq!(
        events
            .into_iter()
            .filter_map(|event| match event {
                Event::Moved(state) => Some(state),
                _ => None,
            })
            .collect::<Vec<_>>(),
        [
            Lifecycle::Grilling,
            Lifecycle::Wrapping,
            Lifecycle::Implementing,
            Lifecycle::Wrapping,
        ],
        "and the moves are what tell the re-entry's story",
    );
    assert_eq!(
        pull_request(&pool, id).await.unwrap(),
        Some(opened()),
        "with the one record still the one the watchers read",
    );
}

/// A Conversation that is not wrapping up has no wrap-up to leave, whether it is
/// being built already or was closed out from under the session that would have
/// split the work out.
#[tokio::test]
async fn only_a_wrapping_conversation_can_be_sent_back_to_be_built() {
    let (_dir, pool) = fresh_pool().await;
    let id = implementing(&pool).await;

    assert_eq!(
        implement_again(&pool, id).await.unwrap(),
        Rebuilding::NotWrapping,
    );

    record_pull_request(&pool, id, &opened()).await.unwrap();
    close_conversation(&pool, id).await.unwrap();

    assert_eq!(
        implement_again(&pool, id).await.unwrap(),
        Rebuilding::NotWrapping,
    );
    assert_eq!(
        load_conversation(&pool, id).await.unwrap().unwrap().state,
        Lifecycle::Closed,
    );

    assert_eq!(
        implement_again(&pool, 404).await.unwrap(),
        Rebuilding::NoSuchConversation,
    );
}
