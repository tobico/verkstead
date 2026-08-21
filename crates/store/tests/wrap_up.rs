//! What wrap-up is still waiting on, and how many goes the machine has had at a
//! red check.
//!
//! Both are bookkeeping rather than Timeline Events, and both have to survive a
//! restart — which is the whole reason they are in the database at all. So what
//! these ask is what a second reader sees: a fresh pool over the same file, as a
//! restarted server has.

use std::path::Path;

use sqlx::SqlitePool;
use verkstead_store::{
    WaitingOn, choose_direction, fix_attempts, forget_fix_attempts, move_to_direction,
    open_database, record_fix_attempt, record_pull_request, register_repo, save_brief,
    settle_wrap_up, start_conversation, start_grilling, start_implementing, unsettle_wrap_up,
    wrap_up_settled,
};

/// A Conversation whose work is on a pull request, which is the only state any
/// of this is about.
///
/// Walked there rather than moved by hand, exactly as the wrapping tests walk
/// one: every state on the way records something, and a Conversation dropped
/// straight into Wrapping would be one nothing else in the store agrees about.
async fn wrapping(pool: &SqlitePool) -> i64 {
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
    )
    .await
    .unwrap();
    move_to_direction(pool, id).await.unwrap();
    choose_direction(pool, id, verkstead_schema::Direction::TaskList)
        .await
        .unwrap();
    start_implementing(pool, id).await.unwrap();
    record_pull_request(
        pool,
        id,
        &verkstead_store::PullRequest {
            number: 41,
            title: "Rate limiting".to_owned(),
            url: "https://github.com/tobico/verkstead/pull/41".to_owned(),
        },
    )
    .await
    .unwrap();

    id
}

/// A pool over a fresh database, plus the directory keeping it alive.
async fn fresh_pool() -> (tempfile::TempDir, SqlitePool) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();
    (dir, pool)
}

/// A wrap-up that has settled nothing, which is every wrap-up the moment its
/// pull request opens.
#[tokio::test]
async fn a_wrap_up_that_has_just_started_is_waiting_on_everything() {
    let (_dir, pool) = fresh_pool().await;
    let id = wrapping(&pool).await;

    assert_eq!(wrap_up_settled(&pool, id).await.unwrap(), Vec::new());
}

/// The green suite, and the same green suite on the next poll: settling is a
/// statement about how things are rather than an event, so saying it twice says
/// the same thing twice.
#[tokio::test]
async fn settling_the_checks_twice_settles_them_once() {
    let (_dir, pool) = fresh_pool().await;
    let id = wrapping(&pool).await;

    settle_wrap_up(&pool, id, WaitingOn::Checks).await.unwrap();
    settle_wrap_up(&pool, id, WaitingOn::Checks).await.unwrap();

    assert_eq!(
        wrap_up_settled(&pool, id).await.unwrap(),
        vec![WaitingOn::Checks],
    );
}

/// And the reason settling is written this way round: a commit pushed to the
/// pull request is a new run to wait on, so yesterday's green must be able to
/// stop standing.
#[tokio::test]
async fn checks_that_go_red_again_stop_being_settled() {
    let (_dir, pool) = fresh_pool().await;
    let id = wrapping(&pool).await;

    settle_wrap_up(&pool, id, WaitingOn::Checks).await.unwrap();
    unsettle_wrap_up(&pool, id, WaitingOn::Checks)
        .await
        .unwrap();

    assert_eq!(wrap_up_settled(&pool, id).await.unwrap(), Vec::new());

    // And unsettling what was never settled is the ordinary case for as long as
    // a suite is running, rather than anything to refuse.
    unsettle_wrap_up(&pool, id, WaitingOn::Checks)
        .await
        .unwrap();
    assert_eq!(wrap_up_settled(&pool, id).await.unwrap(), Vec::new());
}

/// The count is per check rather than per Conversation: a suite where one job
/// fails and is fixed and then a different one fails has not spent its attempts.
#[tokio::test]
async fn fix_attempts_are_counted_against_the_check_rather_than_the_conversation() {
    let (_dir, pool) = fresh_pool().await;
    let id = wrapping(&pool).await;

    assert_eq!(
        fix_attempts(&pool, id, "Rust").await.unwrap(),
        0,
        "nothing has been tried about a check that has only just gone red",
    );

    assert_eq!(record_fix_attempt(&pool, id, "Rust").await.unwrap(), 1);
    assert_eq!(record_fix_attempt(&pool, id, "Rust").await.unwrap(), 2);

    assert_eq!(fix_attempts(&pool, id, "Rust").await.unwrap(), 2);
    assert_eq!(
        fix_attempts(&pool, id, "Viewer").await.unwrap(),
        0,
        "and the job beside it has spent nothing",
    );
}

/// The whole reason the count is in the database: a server that came back up
/// having forgotten would dispatch fix sessions at the same check for ever,
/// which is exactly what *two attempts, then ask the human* exists to prevent.
#[tokio::test]
async fn what_a_check_has_already_been_given_survives_a_restart() {
    let dir = tempfile::tempdir().unwrap();
    let database = dir.path().join("verkstead.db");

    let id = {
        let pool = open_database(&database).await.unwrap();
        let id = wrapping(&pool).await;

        record_fix_attempt(&pool, id, "Rust").await.unwrap();
        settle_wrap_up(&pool, id, WaitingOn::Checks).await.unwrap();

        pool.close().await;
        id
    };

    let restarted = open_database(&database).await.unwrap();

    assert_eq!(fix_attempts(&restarted, id, "Rust").await.unwrap(), 1);
    assert_eq!(
        wrap_up_settled(&restarted, id).await.unwrap(),
        vec![WaitingOn::Checks],
    );
}

/// What a retried Interruption does. The human has read the evidence and asked
/// for another go, and a count left standing would be a watcher that raised the
/// same Interruption on its next poll without dispatching anything.
#[tokio::test]
async fn a_retry_gives_every_check_its_attempts_back() {
    let (_dir, pool) = fresh_pool().await;
    let id = wrapping(&pool).await;

    record_fix_attempt(&pool, id, "Rust").await.unwrap();
    record_fix_attempt(&pool, id, "Rust").await.unwrap();
    record_fix_attempt(&pool, id, "Viewer").await.unwrap();

    forget_fix_attempts(&pool, id).await.unwrap();

    assert_eq!(fix_attempts(&pool, id, "Rust").await.unwrap(), 0);
    assert_eq!(fix_attempts(&pool, id, "Viewer").await.unwrap(), 0);
}
