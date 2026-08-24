//! Pauses on a Conversation's Timeline: raised when an account runs out of
//! window, ended once, and open exactly one at a time.
//!
//! What is being asked of the store here is *the run waits here*. The rules worth
//! a test are the two a promise could not keep: a Conversation has at most one
//! open Pause however many times the banner redraws, and the first thing to end
//! the wait is the one that stands — the human's press and the reset arriving are
//! two devices pressing the same button.

use std::path::Path;

use sqlx::SqlitePool;
use verkstead_store::{
    By, Event, Pause, Resumed, Resuming, conversations, open_database, open_pause, pause,
    record_pause, register_repo, resume_pause, start_conversation, timeline, waiting_pauses,
};

/// The sentence claude prints when an account is out of window, as this build
/// reads it back off the terminal.
const SAID: &str = "Usage limit reached · continuing automatically at 3pm · esc to cancel";

/// A pool over a fresh database, plus the directory keeping it alive.
async fn fresh_pool() -> (tempfile::TempDir, SqlitePool) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();
    (dir, pool)
}

/// A Conversation for a run to wait in.
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
async fn a_pause_lands_on_the_timeline_naming_the_account_and_the_reset() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool).await;

    let event = record_pause(&pool, id, "fable", SAID, Some("2026-08-24T05:00:00Z"))
        .await
        .unwrap()
        .expect("a Conversation with nothing waiting takes a Pause");

    assert_eq!(
        pause(&pool, id, event).await.unwrap(),
        Some(Pause {
            profile: "fable".to_owned(),
            said: SAID.to_owned(),
            resets_at: Some("2026-08-24T05:00:00Z".to_owned()),
            resumed: None,
        }),
        "the account that ran out and when it comes back, kept as they were read",
    );

    assert_eq!(
        on_the_timeline(&pool, id).await.len(),
        1,
        "and it is on the Timeline, which is where the human looks",
    );
}

/// The reset time is the half a display may not carry. A Pause without one is a
/// whole record rather than a broken one: the wait is the human's to end.
#[tokio::test]
async fn a_sentence_with_no_reset_time_in_it_still_pauses_the_run() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool).await;

    let event = record_pause(&pool, id, "fable", "Usage limit reached", None)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        pause(&pool, id, event).await.unwrap().unwrap().resets_at,
        None,
    );
    assert_eq!(open_pause(&pool, id).await.unwrap(), Some(event));
}

/// The banner redraws for as long as the wait lasts, so a second reading of it is
/// the ordinary case. One Pause per Conversation is the database's rule rather
/// than a promise made by whatever noticed.
#[tokio::test]
async fn a_conversation_waits_on_one_pause_however_often_it_is_told() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool).await;

    let first = record_pause(&pool, id, "fable", SAID, None)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        record_pause(
            &pool,
            id,
            "fable",
            "Usage limit reached · continuing shortly",
            None
        )
        .await
        .unwrap(),
        None,
        "a run that is already waiting takes no second Pause",
    );

    assert_eq!(open_pause(&pool, id).await.unwrap(), Some(first));
    assert_eq!(on_the_timeline(&pool, id).await.len(), 1);
}

/// And once the wait is over the next one is a Pause of its own, which is what a
/// long run against a busy account collects.
#[tokio::test]
async fn a_run_that_waited_once_can_wait_again() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool).await;

    let first = record_pause(&pool, id, "fable", SAID, None)
        .await
        .unwrap()
        .unwrap();
    resume_pause(&pool, id, first, By::Human).await.unwrap();

    let again = record_pause(&pool, id, "fable", "Usage limit reached again", None)
        .await
        .unwrap()
        .expect("the first wait is over, so a second one can start");

    assert_ne!(again, first);
    assert_eq!(open_pause(&pool, id).await.unwrap(), Some(again));
    assert_eq!(on_the_timeline(&pool, id).await.len(), 2, "both are kept");
}

/// The two ways a wait ends are one row, and the record keeps which it was.
#[tokio::test]
async fn what_ended_the_wait_is_on_the_record() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool).await;

    let event = record_pause(&pool, id, "fable", SAID, None)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        resume_pause(&pool, id, event, By::Reset).await.unwrap(),
        Resuming::Resumed,
    );

    let Some(Resumed { by, at }) = pause(&pool, id, event).await.unwrap().unwrap().resumed else {
        panic!("the Pause reads as still waiting");
    };

    assert_eq!(by, By::Reset, "the window came back on its own");
    assert!(!at.is_empty(), "and it is stamped");
    assert_eq!(
        open_pause(&pool, id).await.unwrap(),
        None,
        "nothing is holding the run up any more",
    );
}

/// The human presses from a phone while the sweep is closing the same Pause. The
/// second one is not an error and not something to act on twice.
#[tokio::test]
async fn a_wait_that_is_over_stays_over() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool).await;

    let event = record_pause(&pool, id, "fable", SAID, None)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        resume_pause(&pool, id, event, By::Human).await.unwrap(),
        Resuming::Resumed,
    );
    assert_eq!(
        resume_pause(&pool, id, event, By::Reset).await.unwrap(),
        Resuming::AlreadyResumed,
        "the first ending stands",
    );

    assert_eq!(
        pause(&pool, id, event)
            .await
            .unwrap()
            .unwrap()
            .resumed
            .map(|resumed| resumed.by),
        Some(By::Human),
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

    let event = record_pause(&pool, one, "fable", SAID, None)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(pause(&pool, other, event).await.unwrap(), None);
    assert_eq!(
        resume_pause(&pool, other, event, By::Human).await.unwrap(),
        Resuming::NoSuchPause,
    );
    assert_eq!(
        open_pause(&pool, one).await.unwrap(),
        Some(event),
        "and the Pause it does belong to is untouched",
    );
}

/// What the sweep reads: every wait still on, whichever Conversation it is in,
/// with the time it is waiting for. That is what makes a reset survive a restart
/// — nothing holds a clock across the process.
#[tokio::test]
async fn every_run_still_waiting_is_readable_at_once() {
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

    let waiting = record_pause(&pool, one, "fable", SAID, Some("2026-08-24T05:00:00Z"))
        .await
        .unwrap()
        .unwrap();
    let over = record_pause(&pool, other, "opus", SAID, None)
        .await
        .unwrap()
        .unwrap();
    resume_pause(&pool, other, over, By::Human).await.unwrap();

    let still = waiting_pauses(&pool).await.unwrap();

    assert_eq!(still.len(), 1, "the one that is over is not waiting");
    assert_eq!(still[0].conversation_id, one);
    assert_eq!(still[0].event_id, waiting);
    assert_eq!(still[0].resets_at.as_deref(), Some("2026-08-24T05:00:00Z"));
}

/// The sidebar says *this one wants you* for a paused run, exactly as it does for
/// a run stopped on an Interruption: what it is saying is that the work has
/// stopped, and whether the human has to do anything about it is the
/// Conversation's own page to show.
#[tokio::test]
async fn a_paused_run_is_waiting_on_the_human_in_the_sidebar() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool).await;

    verkstead_store::save_brief(&pool, id, "# Rate limiting\n")
        .await
        .unwrap();
    verkstead_store::start_grilling(
        &pool,
        id,
        "6f32b11a0c4d1e8f5b3a97c2d0e4f6a8b1c3d5e7",
        Path::new("/state/worktrees/verkstead-rate-limiting"),
    )
    .await
    .unwrap();

    let waiting = |rows: Vec<verkstead_store::ConversationRow>| {
        rows.into_iter()
            .find(|row| row.id == id)
            .expect("the Conversation is in the list")
            .waiting
    };

    assert!(
        !waiting(conversations(&pool).await.unwrap()),
        "nothing is waiting before the account runs out",
    );

    let event = record_pause(&pool, id, "fable", SAID, None)
        .await
        .unwrap()
        .unwrap();

    assert!(waiting(conversations(&pool).await.unwrap()));

    resume_pause(&pool, id, event, By::Reset).await.unwrap();

    assert!(
        !waiting(conversations(&pool).await.unwrap()),
        "and the row is quiet again once the window comes back",
    );
}
