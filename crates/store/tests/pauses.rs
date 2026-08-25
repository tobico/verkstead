//! The Pauses a Verkstead of before put on a Conversation's Timeline, read back
//! by this one.
//!
//! Nothing writes one any more: an account out of window stops a run the way
//! everything else does. What is asked of the store here is ADR-0006's rule —
//! *the record is kept and read rather than rewritten* — so the rows are written
//! by hand, as a database from before holds them, and the tests are that a
//! Timeline carrying one still reads and that ending one is recorded once.

use std::path::Path;

use sqlx::SqlitePool;
use verkstead_store::{
    Event, Pause, Resuming, open_database, pause, register_repo, resume_pause, start_conversation,
    timeline,
};

/// The sentence claude prints when an account is out of window, as a Verkstead
/// of before read it back off the terminal.
const SAID: &str = "Usage limit reached · continuing automatically at 3pm · esc to cancel";

/// A pool over a fresh database, plus the directory keeping it alive.
async fn fresh_pool() -> (tempfile::TempDir, SqlitePool) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();
    (dir, pool)
}

/// A Conversation for a run to have waited in.
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

/// One Pause as a Verkstead of before wrote it: the Event of its own kind, and
/// the row of separate facts under it.
///
/// Written here rather than by the code that used to write it — that code has
/// gone, and what has to keep working is a database rather than a function.
async fn stored(
    pool: &SqlitePool,
    conversation_id: i64,
    profile: &str,
    said: &str,
    resets_at: Option<&str>,
) -> i64 {
    let (event_id,): (i64,) = sqlx::query_as(
        "INSERT INTO timeline_events (conversation_id, at, kind, body)
         VALUES (?, '2026-08-24T04:00:00.000Z', 'pause', '')
         RETURNING id",
    )
    .bind(conversation_id)
    .fetch_one(pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO pauses (event_id, conversation_id, profile, said, resets_at)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(event_id)
    .bind(conversation_id)
    .bind(profile)
    .bind(said)
    .bind(resets_at)
    .execute(pool)
    .await
    .unwrap();

    event_id
}

/// The Pauses on a Conversation's Timeline, in Timeline order.
async fn on_the_timeline(pool: &SqlitePool, id: i64) -> Vec<Pause> {
    timeline(pool, id)
        .await
        .unwrap()
        .into_iter()
        .filter_map(|event| match event.event {
            Event::Pause(pause) => Some(pause),
            _ => None,
        })
        .collect()
}

#[tokio::test]
async fn a_stored_pause_still_names_the_account_and_the_reset() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool).await;

    let event = stored(&pool, id, "fable", SAID, Some("2026-08-24T05:00:00Z")).await;

    assert_eq!(
        pause(&pool, id, event).await.unwrap(),
        Some(Pause {
            profile: "fable".to_owned(),
            said: SAID.to_owned(),
            resets_at: Some("2026-08-24T05:00:00Z".to_owned()),
            resumed: None,
        }),
        "the account that ran out and when it comes back, exactly as they were read",
    );

    assert_eq!(
        on_the_timeline(&pool, id).await.len(),
        1,
        "and it is on the Timeline, which is where the human looks",
    );
}

/// The reset time is the half a display may not have carried. A Pause without
/// one is a whole record rather than a broken one.
#[tokio::test]
async fn a_stored_pause_with_no_reset_time_reads_back_too() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool).await;

    let event = stored(&pool, id, "fable", "Usage limit reached", None).await;

    assert_eq!(
        pause(&pool, id, event).await.unwrap().unwrap().resets_at,
        None,
    );
    assert_eq!(on_the_timeline(&pool, id).await.len(), 1);
}

/// That a wait is over is on the record, and stamped. What ended it is not:
/// there is one way left, and every wait that ends ends by a press.
#[tokio::test]
async fn a_wait_that_ended_is_on_the_record() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool).await;

    let event = stored(&pool, id, "fable", SAID, None).await;

    assert_eq!(
        resume_pause(&pool, id, event).await.unwrap(),
        Resuming::Resumed,
    );

    let Some(at) = pause(&pool, id, event).await.unwrap().unwrap().resumed else {
        panic!("the Pause reads as still waiting");
    };

    assert!(!at.is_empty(), "and it is stamped");
}

/// A row a Verkstead of before ended reads as ended, whichever of its two
/// answers it was written with. The word it kept is nothing this build asks
/// about, and rewriting the row to drop it would be rewriting the record.
#[tokio::test]
async fn a_wait_a_verkstead_of_before_ended_still_reads_as_over() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool).await;

    let event = stored(&pool, id, "fable", SAID, None).await;

    sqlx::query(
        "UPDATE pauses SET resumed_by = 'reset', resumed_at = '2026-08-24T05:00:00.000Z'
         WHERE event_id = ?",
    )
    .bind(event)
    .execute(&pool)
    .await
    .unwrap();

    assert_eq!(
        pause(&pool, id, event).await.unwrap().unwrap().resumed,
        Some("2026-08-24T05:00:00.000Z".to_owned()),
    );
}

/// The human presses from a phone twice over. The second one is not an error and
/// not something to act on twice.
#[tokio::test]
async fn a_wait_that_is_over_stays_over() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool).await;

    let event = stored(&pool, id, "fable", SAID, None).await;

    assert_eq!(
        resume_pause(&pool, id, event).await.unwrap(),
        Resuming::Resumed,
    );
    assert_eq!(
        resume_pause(&pool, id, event).await.unwrap(),
        Resuming::AlreadyResumed,
        "the first ending stands",
    );
}

/// A Pause is reached through the Timeline it is on, so an Event belonging to
/// another Conversation names nothing here.
#[tokio::test]
async fn a_pause_belongs_to_the_conversation_it_is_on() {
    let (_dir, pool) = fresh_pool().await;
    let one = conversation(&pool).await;

    let repo = register_repo(&pool, Path::new("/watched/askance"), "askance", "trunk")
        .await
        .unwrap()
        .unwrap()
        .id;
    let other = start_conversation(&pool, repo, "deferred-asks")
        .await
        .unwrap()
        .unwrap();

    let event = stored(&pool, one, "fable", SAID, None).await;

    assert_eq!(pause(&pool, other, event).await.unwrap(), None);
    assert_eq!(
        resume_pause(&pool, other, event).await.unwrap(),
        Resuming::NoSuchPause,
    );
    assert_eq!(
        pause(&pool, one, event).await.unwrap().unwrap().resumed,
        None,
        "and the Pause it does belong to is untouched",
    );
}
