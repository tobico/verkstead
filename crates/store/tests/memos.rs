//! The Memos a building session leaves for the sessions after it (ADR-0023),
//! as the store keeps them and the Timeline reads them back.

use std::path::Path;

use sqlx::SqlitePool;
use verkstead_store::{
    Event, Memo, Retirement, Retiring, leave_memo, memo_of, open_database, register_repo,
    restore_memo, retire_memo, standing_memos, start_capture, start_conversation, timeline,
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

    // Written straight in, so the read is asked about apart from the retiring.
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

/// Retired by a session with a reason, a Memo leaves the standing list and
/// reads back retired with the stamp and the reason — and still one card on the
/// Timeline, the Event untouched.
#[tokio::test]
async fn a_retired_memo_leaves_the_standing_list_and_keeps_its_card() {
    let (_dir, pool) = fresh_pool().await;
    let (id, session) = worked(&pool, "rate-limiting").await;

    let first = leave_memo(
        &pool,
        id,
        MARKDOWN,
        "the session that worked task 03",
        session,
    )
    .await
    .unwrap()
    .unwrap();
    let second = leave_memo(
        &pool,
        id,
        "Run clippy first.\n",
        "the finish session",
        session,
    )
    .await
    .unwrap()
    .unwrap();

    assert_eq!(
        retire_memo(
            &pool,
            id,
            first,
            "the session that worked task 05",
            Some("fixed in 8f3c2a1")
        )
        .await
        .unwrap(),
        Retiring::Retired,
    );

    let standing: Vec<i64> = standing_memos(&pool, id)
        .await
        .unwrap()
        .into_iter()
        .map(|memo| memo.id)
        .collect();
    assert_eq!(standing, vec![second], "the rest still stand");

    let read = on_the_timeline(&pool, id).await;
    assert_eq!(read.len(), 2, "one card per memo: {read:?}");
    assert_eq!(read[0].1, MARKDOWN, "the markdown untouched");

    let retired = read[0].2.retired.clone().expect("retired");
    assert_eq!(retired.by, "the session that worked task 05");
    assert_eq!(retired.reason.as_deref(), Some("fixed in 8f3c2a1"));
}

/// Retiring one twice is refused naming who did it first, and one of another
/// Conversation's is refused as not this one's.
#[tokio::test]
async fn a_memo_already_retired_or_of_another_conversation_is_refused() {
    let (_dir, pool) = fresh_pool().await;
    let (one, session) = worked(&pool, "rate-limiting").await;
    let (other, _) = worked(&pool, "deferred-asks").await;

    let memo = leave_memo(&pool, one, MARKDOWN, "the inline session", session)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        retire_memo(&pool, other, memo, "the inline session", None)
            .await
            .unwrap(),
        Retiring::NotThisConversations,
    );
    assert_eq!(
        retire_memo(&pool, one, session, "the inline session", None)
            .await
            .unwrap(),
        Retiring::NotThisConversations,
        "an Event that is not a memo is not one",
    );

    retire_memo(&pool, one, memo, "you", None).await.unwrap();

    assert_eq!(
        retire_memo(&pool, one, memo, "the inline session", None)
            .await
            .unwrap(),
        Retiring::AlreadyRetired {
            by: "you".to_owned()
        },
    );
}

/// Restore clears all three columns and puts it back in the standing list; it
/// is one Conversation's to restore, and a memo standing restores to itself.
#[tokio::test]
async fn a_restored_memo_stands_again() {
    let (_dir, pool) = fresh_pool().await;
    let (id, session) = worked(&pool, "rate-limiting").await;
    let (other, _) = worked(&pool, "deferred-asks").await;

    let memo = leave_memo(&pool, id, MARKDOWN, "the inline session", session)
        .await
        .unwrap()
        .unwrap();

    retire_memo(&pool, id, memo, "you", Some("wrong"))
        .await
        .unwrap();

    assert!(!restore_memo(&pool, other, memo).await.unwrap());
    assert!(restore_memo(&pool, id, memo).await.unwrap());
    assert!(restore_memo(&pool, id, memo).await.unwrap(), "and again");

    let (_, markdown, read) = memo_of(&pool, id, memo).await.unwrap().unwrap();
    assert_eq!(markdown, MARKDOWN);
    assert_eq!(read.retired, None);
    assert_eq!(standing_memos(&pool, id).await.unwrap().len(), 1);
    assert_eq!(memo_of(&pool, other, memo).await.unwrap(), None);
}
