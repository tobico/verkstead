//! The Pauses a Verkstead of before put on a Conversation's Timeline, read back
//! by this one.
//!
//! Nothing writes one any more: an account out of window stops a run the way
//! everything else does. What is asked of the store here is ADR-0006's rule —
//! *the record is kept and read rather than rewritten* — so the rows are written
//! by hand, as a database from before holds them, and the tests are that a
//! Timeline carrying one still reads whatever those rows say about a wait that
//! is long over.

use std::path::Path;

use sqlx::SqlitePool;
use verkstead_store::{Event, Pause, open_database, register_repo, start_conversation, timeline};

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

/// What a stored Pause has to say is what the human reads on the Timeline: which
/// account ran out, and the sentence the session printed about it.
#[tokio::test]
async fn a_stored_pause_still_names_the_account_and_what_the_session_said() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool).await;

    stored(&pool, id, "fable", SAID, Some("2026-08-24T05:00:00Z")).await;

    assert_eq!(
        on_the_timeline(&pool, id).await,
        vec![Pause {
            profile: "fable".to_owned(),
            said: SAID.to_owned(),
        }],
        "the account that ran out and its own sentence, exactly as they were read",
    );
}

/// A row a Verkstead of before ended still reads, and reads as the same thing:
/// what a wait was is on the record, and what became of it was a card's to say.
/// That card has gone, and the row keeps every word it was written with.
#[tokio::test]
async fn a_wait_a_verkstead_of_before_ended_reads_back_the_same() {
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
        on_the_timeline(&pool, id).await,
        vec![Pause {
            profile: "fable".to_owned(),
            said: SAID.to_owned(),
        }],
    );

    let ended: (Option<String>, Option<String>) =
        sqlx::query_as("SELECT resumed_by, resumed_at FROM pauses WHERE event_id = ?")
            .bind(event)
            .fetch_one(&pool)
            .await
            .unwrap();

    assert_eq!(
        ended,
        (
            Some("reset".to_owned()),
            Some("2026-08-24T05:00:00.000Z".to_owned())
        ),
        "and the row itself is untouched: rewriting the record is what nothing does",
    );
}

/// A Pause is one Conversation's, so another's Timeline does not carry it.
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

    stored(&pool, one, "fable", SAID, None).await;

    assert_eq!(on_the_timeline(&pool, other).await, vec![]);
    assert_eq!(on_the_timeline(&pool, one).await.len(), 1);
}
