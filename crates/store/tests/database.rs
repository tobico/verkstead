//! The database itself: the mode it is opened in, and what happens when two
//! writes want it at once.
//!
//! Neither of these is about what a table holds. They are about the two settings
//! everything else in the store rests on — and both were wrong, in a way that
//! cost a Conversation its ending. A finish step recorded its pull request while
//! the session that opened it was still writing its Capture, SQLite refused the
//! write, and the Conversation was left implementing an emptied backlog with the
//! work out on a pull request nothing knew about.
//!
//! So they are asserted here rather than left to be true: a `journal_mode` that
//! silently went back to a rollback journal, or a `BEGIN` that went back to
//! being deferred, would each be a change nothing else in the suite would
//! notice until a run failed the same way again.

use std::path::Path;

use sqlx::{Row, SqlitePool};
use verkstead_store::{
    Lifecycle, PullRequest, Wrapping, load_conversation, open_database, register_repo, save_brief,
    start_conversation, start_grilling,
};

/// A pool over a fresh database, plus the directory keeping it alive.
async fn fresh_pool() -> (tempfile::TempDir, SqlitePool) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();
    (dir, pool)
}

/// A Conversation on `branch`, walked as far as grilling — which is one of the
/// two states a pull request can be recorded against.
async fn grilling(pool: &SqlitePool, repo: i64, branch: &str) -> i64 {
    let id = start_conversation(pool, repo, branch)
        .await
        .unwrap()
        .expect("the Repo is registered");

    save_brief(pool, id, "# Rate limiting\n").await.unwrap();
    start_grilling(
        pool,
        id,
        "c0ffee",
        Path::new("/state/worktrees").join(branch).as_path(),
        &[],
    )
    .await
    .unwrap();

    id
}

/// The pull request a finish step opened, numbered so that each Conversation's
/// is its own.
fn opened(number: i64) -> PullRequest {
    PullRequest {
        number,
        title: "Rate limiting".to_owned(),
        url: format!("https://github.com/tobico/verkstead/pull/{number}"),
        repo: None,
    }
}

/// The database is opened in write-ahead logging.
///
/// sqlx will not do this by itself — it leaves `journal_mode` alone, because
/// switching a database into or out of WAL takes an exclusive lock no busy
/// timeout can wait on and it will not do that behind an application's back. So
/// it is Verkstead's to ask for, and the asking is what this pins.
///
/// Under the rollback journal that is the default, a reader and a writer cannot
/// hold the file at once: every poll of a Timeline is something a session's
/// Capture write has to queue behind. Verkstead writes continuously while a
/// session runs and reads on every open page, which is the shape of use WAL is
/// for.
#[tokio::test]
async fn the_database_is_opened_in_write_ahead_logging() {
    let (_dir, pool) = fresh_pool().await;

    let mode: String = sqlx::query("PRAGMA journal_mode")
        .fetch_one(&pool)
        .await
        .unwrap()
        .get(0);

    assert_eq!(
        mode.to_lowercase(),
        "wal",
        "a Verkstead database is a WAL one, so a page being read never holds a \
         session's writes up",
    );
}

/// Writes that arrive at once all land, rather than one of them being refused
/// for the other holding the database.
///
/// The failure this is written against, in the shape it really happened in.
/// Every transaction in the store reads before it writes — a state read before
/// the move it authorises, a count read before the row that changes it — and a
/// deferred `BEGIN` takes no lock for the read, so the first write has to promote
/// one. **SQLite will not wait for that promotion**: where another connection is
/// holding its own read, promoting would deadlock the pair, so rather than call
/// the busy handler it fails the statement at once with *database is locked*. No
/// busy timeout covers it, however long.
///
/// `BEGIN IMMEDIATE` takes the write lock before the first read, so there is no
/// promotion to fail and the wait becomes an ordinary one the busy timeout does
/// cover. Which is what this asks: sixteen recordings at once, and every one of
/// them lands.
///
/// Sixteen Conversations rather than one, so that what is being asked about is
/// the database being held rather than the store's own rule that a Conversation
/// has one pull request.
#[tokio::test]
async fn writes_that_arrive_at_once_all_land() {
    let (_dir, pool) = fresh_pool().await;

    let repo = register_repo(&pool, Path::new("/srv/verkstead"), "verkstead", "main")
        .await
        .unwrap()
        .expect("nothing is registered at that path yet")
        .id;

    let mut conversations = Vec::new();

    for number in 0..16 {
        conversations.push(grilling(&pool, repo, &format!("rate-limiting-{number}")).await);
    }

    let recording: Vec<_> = conversations
        .iter()
        .enumerate()
        .map(|(number, &id)| {
            let pool = pool.clone();

            tokio::spawn(async move {
                verkstead_store::record_pull_request(&pool, id, repo, &opened(number as i64 + 1))
                    .await
            })
        })
        .collect();

    for (number, recorded) in recording.into_iter().enumerate() {
        assert_eq!(
            recorded.await.unwrap().unwrap(),
            Wrapping::Started,
            "recording {number} was refused while the others were writing",
        );
    }

    for &id in &conversations {
        assert_eq!(
            load_conversation(&pool, id).await.unwrap().unwrap().state,
            Lifecycle::Wrapping,
            "and every Conversation made the move its recording carries",
        );
    }
}
