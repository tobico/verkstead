//! The Memos a building session leaves for the sessions after it (ADR-0023),
//! as the store keeps them and the Timeline reads them back.

use std::path::Path;

use sqlx::SqlitePool;
use verkstead_store::{
    Event, Memo, Retirement, leave_memo, open_database, register_repo, start_capture,
    start_conversation, timeline,
};

/// The device every Conversation started here is ranked by, named the way a
/// cluster names one (ADR-0020, *Ranks*).
const THIS_DEVICE: &str = "aa00bb11cc22dd33ee44ff5566778899";

/// What the session wrote.
const MARKDOWN: &str = "The sessions suite flakes under parallelism.\n";

/// A pool over a fresh database, plus the directory keeping it alive.
async fn fresh_pool() -> (tempfile::TempDir, SqlitePool) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();
    (dir, pool)
}

/// A Conversation, and the output of the session on it that leaves the Memo.
async fn worked(pool: &SqlitePool, branch: &str) -> (i64, i64) {
    let repo = register_repo(pool, Path::new("/watched/verkstead"), "verkstead", "main")
        .await
        .unwrap()
        .map(|repo| repo.id)
        .unwrap_or(1);

    let id = start_conversation(pool, repo, branch, THIS_DEVICE)
        .await
        .unwrap()
        .expect("the Repo is registered");

    let session = start_capture(pool, id, Some("task-03"), None)
        .await
        .unwrap();

    (id, session)
}

/// The Memos on a Conversation's Timeline, in Timeline order, by Event id.
async fn on_the_timeline(pool: &SqlitePool, id: i64) -> Vec<(i64, String, Memo)> {
    timeline(pool, id)
        .await
        .unwrap()
        .into_iter()
        .filter_map(|event| match event.event {
            Event::Memo(markdown, memo) => Some((event.id, markdown, memo)),
            _ => None,
        })
        .collect()
}

/// A Memo reads back off the Timeline as its own kind: the markdown, the words
/// it was stamped with, the session it was left out of, and no retirement.
#[tokio::test]
async fn a_memo_left_reads_back_as_its_own_kind_with_its_stamp() {
    let (_dir, pool) = fresh_pool().await;
    let (id, session) = worked(&pool, "rate-limiting").await;

    let memo = leave_memo(
        &pool,
        id,
        MARKDOWN,
        "the session that worked task 03",
        session,
    )
    .await
    .unwrap()
    .expect("the Conversation is there to leave one on");

    assert_eq!(
        on_the_timeline(&pool, id).await,
        vec![(
            memo,
            MARKDOWN.to_owned(),
            Memo {
                left_by: "the session that worked task 03".to_owned(),
                left_in: session,
                retired: None,
            },
        )],
    );
}

/// The retirement columns read back as one, once something has written them.
#[tokio::test]
async fn a_retired_memo_reads_back_retired_with_who_and_why() {
    let (_dir, pool) = fresh_pool().await;
    let (id, session) = worked(&pool, "rate-limiting").await;

    let memo = leave_memo(&pool, id, MARKDOWN, "the finish session", session)
        .await
        .unwrap()
        .unwrap();

    // Nothing retires one yet, so the row is written the way that will.
    sqlx::query(
        "UPDATE memos SET retired_at = '2026-10-14T09:00:00.000Z', retired_by = 'you',
                          reason = 'fixed on main'
         WHERE event_id = ?",
    )
    .bind(memo)
    .execute(&pool)
    .await
    .unwrap();

    let [(_, _, read)] = <[_; 1]>::try_from(on_the_timeline(&pool, id).await).unwrap();

    assert_eq!(
        read.retired,
        Some(Retirement {
            at: "2026-10-14T09:00:00.000Z".to_owned(),
            by: "you".to_owned(),
            reason: Some("fixed on main".to_owned()),
        }),
    );
}

/// A Memo is one Conversation's, and a Conversation that is not there has none
/// left on it.
#[tokio::test]
async fn a_memo_belongs_to_the_conversation_it_was_left_on() {
    let (_dir, pool) = fresh_pool().await;
    let (one, session) = worked(&pool, "rate-limiting").await;
    let (other, _) = worked(&pool, "deferred-asks").await;

    leave_memo(&pool, one, MARKDOWN, "the inline session", session)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(on_the_timeline(&pool, other).await, vec![]);
    assert_eq!(on_the_timeline(&pool, one).await.len(), 1);

    assert_eq!(
        leave_memo(&pool, 9_999, MARKDOWN, "the inline session", session)
            .await
            .unwrap(),
        None,
        "no Conversation, no Memo",
    );
}
