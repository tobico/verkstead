//! Commits on a Conversation's Timeline: recorded once each, in the order they
//! were offered, and read back as the line the Timeline draws.
//!
//! What is being asked of the store here is *exactly once*. Whatever watches a
//! branch sweeps the whole of it every time it looks — a branch is not a queue
//! — so nearly every commit it offers has been recorded already, and the store
//! is what makes offering one twice cost nothing.

use std::path::Path;

use sqlx::SqlitePool;
use verkstead_store::{
    Commit, Event, commit, open_database, record_commit, recorded_commits, register_repo,
    start_conversation, timeline,
};

/// A pool over a fresh database, plus the directory keeping it alive.
async fn fresh_pool() -> (tempfile::TempDir, SqlitePool) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();
    (dir, pool)
}

/// A Conversation to land commits on.
async fn conversation(pool: &SqlitePool) -> i64 {
    let repo = register_repo(pool, Path::new("/watched/verkstead"), "verkstead", "main")
        .await
        .unwrap()
        .expect("nothing was registered at that path yet")
        .id;

    start_conversation(pool, repo, "rate-limiting")
        .await
        .unwrap()
        .expect("the Repo was just registered")
}

/// A commit as git would have described it.
fn landed(sha: &str, subject: &str) -> Commit {
    Commit {
        sha: sha.to_owned(),
        subject: subject.to_owned(),
        files: 2,
        insertions: 31,
        deletions: 4,
    }
}

/// The commits on a Conversation's Timeline, in Timeline order.
async fn on_the_timeline(pool: &SqlitePool, id: i64) -> Vec<Commit> {
    timeline(pool, id)
        .await
        .unwrap()
        .into_iter()
        .filter_map(|event| match event.event {
            Event::Commit(commit) => Some(commit),
            _ => None,
        })
        .collect()
}

#[tokio::test]
async fn a_commit_lands_on_the_timeline_as_what_it_changed() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool).await;

    let event = record_commit(&pool, id, &landed("a1b2c3d", "feat: rate limiting"))
        .await
        .unwrap()
        .expect("a commit on a Conversation that is there is recorded");

    assert_eq!(
        on_the_timeline(&pool, id).await,
        vec![landed("a1b2c3d", "feat: rate limiting")],
        "the Timeline says what the commit was called and how much it moved",
    );

    assert_eq!(
        commit(&pool, id, event).await.unwrap(),
        Some(landed("a1b2c3d", "feat: rate limiting")),
        "and the Event is what the details pane fetches the commit by",
    );
}

/// The rule the whole design of this table is for. A sweep offers the branch
/// whole, so the second sweep offers everything the first one already recorded.
#[tokio::test]
async fn the_same_commit_offered_twice_is_recorded_once() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool).await;

    let first = record_commit(&pool, id, &landed("a1b2c3d", "feat: rate limiting"))
        .await
        .unwrap();
    let again = record_commit(&pool, id, &landed("a1b2c3d", "feat: rate limiting"))
        .await
        .unwrap();

    assert!(first.is_some(), "the first sweep records it");
    assert_eq!(again, None, "and the second finds nothing left to do");

    assert_eq!(
        on_the_timeline(&pool, id).await.len(),
        1,
        "so the Timeline has it once",
    );
}

/// Two Conversations can be working on branches that share a commit — one
/// stacked on the other, or both branched from the same place — and each of
/// their Timelines is its own record.
#[tokio::test]
async fn one_commit_can_be_on_two_conversations() {
    let (_dir, pool) = fresh_pool().await;
    let one = conversation(&pool).await;

    let repo = register_repo(&pool, Path::new("/watched/other"), "other", "main")
        .await
        .unwrap()
        .expect("nothing was registered there yet")
        .id;
    let two = start_conversation(&pool, repo, "stacked")
        .await
        .unwrap()
        .unwrap();

    assert!(
        record_commit(&pool, one, &landed("a1b2c3d", "feat: rate limiting"))
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        record_commit(&pool, two, &landed("a1b2c3d", "feat: rate limiting"))
            .await
            .unwrap()
            .is_some(),
        "the rule is one per Conversation, not one per repository",
    );

    assert_eq!(on_the_timeline(&pool, one).await.len(), 1);
    assert_eq!(on_the_timeline(&pool, two).await.len(), 1);
}

/// The order Events are read back in is the order they were recorded, which for
/// commits is the order they landed on the branch — a sweep offers them oldest
/// first.
#[tokio::test]
async fn commits_come_back_in_the_order_they_were_recorded() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool).await;

    for (sha, subject) in [
        ("1111111", "test: a failing test"),
        ("2222222", "feat: rate limiting"),
        ("3333333", "docs: say what it does"),
    ] {
        record_commit(&pool, id, &landed(sha, subject))
            .await
            .unwrap()
            .expect("each of these is new");
    }

    let subjects: Vec<String> = on_the_timeline(&pool, id)
        .await
        .into_iter()
        .map(|commit| commit.subject)
        .collect();

    assert_eq!(
        subjects,
        vec![
            "test: a failing test",
            "feat: rate limiting",
            "docs: say what it does"
        ],
    );
}

/// What a sweep asks before it goes reading git: everything already recorded is
/// a commit there is nothing left to do about.
#[tokio::test]
async fn what_is_already_recorded_can_be_asked_for_by_sha() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool).await;

    assert_eq!(
        recorded_commits(&pool, id).await.unwrap(),
        Vec::<String>::new(),
        "a Conversation that has committed nothing has nothing recorded",
    );

    record_commit(&pool, id, &landed("a1b2c3d", "feat: rate limiting"))
        .await
        .unwrap();

    assert_eq!(
        recorded_commits(&pool, id).await.unwrap(),
        vec!["a1b2c3d".to_owned()],
    );
}

/// A commit attributed to a Conversation that is not there would be on nobody's
/// Timeline, which is why the insert selects the Conversation rather than
/// trusting the id.
#[tokio::test]
async fn a_commit_on_no_conversation_is_not_recorded() {
    let (_dir, pool) = fresh_pool().await;

    assert_eq!(
        record_commit(&pool, 404, &landed("a1b2c3d", "feat: rate limiting"))
            .await
            .unwrap(),
        None,
    );
}

/// A commit is reached through the Timeline it is on. An Event id belonging to
/// another Conversation names nothing, which is what makes the details pane's
/// route conversation-scoped rather than only conversation-shaped.
#[tokio::test]
async fn a_commit_is_not_readable_through_another_conversation() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool).await;

    let event = record_commit(&pool, id, &landed("a1b2c3d", "feat: rate limiting"))
        .await
        .unwrap()
        .unwrap();

    assert_eq!(commit(&pool, id + 1, event).await.unwrap(), None);
}
