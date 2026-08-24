//! Halts on a Conversation: written with the Notice that explains them, held
//! one at a time, and cleared when driving starts again.
//!
//! What is being asked of the store here is *this Conversation is stopped*. A
//! halt is the one thing that says so, so the rules worth a test are the ones a
//! promise could not keep: a Conversation has at most one, its Notice is on the
//! Timeline beside it, and clearing it leaves the Notice where it is.

use std::path::Path;

use sqlx::SqlitePool;
use verkstead_store::{
    Event, Halt, Halted, clear_halt, halt, halted, open_database, register_repo,
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

/// A Conversation for driving to stop on.
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

/// What Verkstead said about the stop, as a halt's Notice carries it.
const SAID: &str = "**Implementing the work** stopped.\n\nnothing is driving it";

/// The notices on a Conversation's Timeline, in Timeline order.
async fn notices(pool: &SqlitePool, id: i64) -> Vec<String> {
    timeline(pool, id)
        .await
        .unwrap()
        .into_iter()
        .filter_map(|event| match event.event {
            Event::Notice(markdown) => Some(markdown),
            _ => None,
        })
        .collect()
}

#[tokio::test]
async fn a_halt_lands_with_the_notice_that_explains_it() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool).await;

    let event = halt(&pool, id, Halt::Circumstance, SAID)
        .await
        .unwrap()
        .expect("a Conversation that is there stops");

    let stopped = halted(&pool, id)
        .await
        .unwrap()
        .expect("it is halted, which is what was just written");

    assert_eq!(
        stopped,
        Halted {
            halt: Halt::Circumstance,
            event_id: event,
            at: stopped.at.clone(),
        },
        "the kind of stop it was, and the Event that says what it was",
    );
    assert!(
        stopped.at.starts_with("20"),
        "with when it stopped, RFC 3339: {:?}",
        stopped.at,
    );

    assert_eq!(
        notices(&pool, id).await,
        vec![SAID.to_owned()],
        "and what Verkstead has to say about it is an ordinary Notice",
    );
}

/// The rule the primary key is there for. A Conversation that is stopped is
/// stopped once: the sweep looks again a minute later and finds the same
/// Conversation standing just as still, and that is not news.
#[tokio::test]
async fn a_conversation_is_halted_once_however_often_it_is_noticed() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool).await;

    let first = halt(&pool, id, Halt::Circumstance, SAID)
        .await
        .unwrap()
        .expect("a Conversation that is there stops");

    assert_eq!(
        halt(&pool, id, Halt::Deliberate, "and again")
            .await
            .unwrap(),
        None,
        "the second one is the same stop arriving twice",
    );

    assert_eq!(
        halted(&pool, id).await.unwrap().map(|stopped| stopped.halt),
        Some(Halt::Circumstance),
        "so the first halt is the one that stands, kind and all",
    );
    assert_eq!(
        notices(&pool, id).await,
        vec![SAID.to_owned()],
        "and the Timeline is told once, not once a minute",
    );

    assert_eq!(
        halted(&pool, id)
            .await
            .unwrap()
            .map(|stopped| stopped.event_id),
        Some(first),
        "pointing at the Notice that explained it in the first place",
    );
}

/// Clearing it is what starting to drive again does — and it takes away the
/// halt alone. The Notice is a record of a stop that really happened, and a
/// Timeline that took yesterday's back would be one nobody could read.
#[tokio::test]
async fn driving_again_clears_the_halt_and_leaves_the_notice() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool).await;

    halt(&pool, id, Halt::Deliberate, SAID).await.unwrap();
    clear_halt(&pool, id).await.unwrap();

    assert_eq!(
        halted(&pool, id).await.unwrap(),
        None,
        "nothing is stopping it any more",
    );
    assert_eq!(
        notices(&pool, id).await,
        vec![SAID.to_owned()],
        "and what stopped it is still on the record",
    );

    assert!(
        halt(&pool, id, Halt::Deliberate, "stopped again")
            .await
            .unwrap()
            .is_some(),
        "which is what leaves room for the next stop to be written",
    );
}

/// Both kinds read back as themselves. Which one a halt is decides whether a
/// restarting server starts the work again or leaves it alone, so a word the
/// store could not read back would be a decision nothing could act on.
#[tokio::test]
async fn every_kind_of_stop_reads_back_as_itself() {
    let (_dir, pool) = fresh_pool().await;
    let repo = register_repo(&pool, Path::new("/watched/verkstead"), "verkstead", "main")
        .await
        .unwrap()
        .expect("nothing was registered at that path yet")
        .id;

    for (kind, branch) in [
        (Halt::Deliberate, "asked-to-stop"),
        (Halt::Circumstance, "left-mid-run"),
    ] {
        let id = start_conversation(&pool, repo, branch)
            .await
            .unwrap()
            .expect("the Repo was just registered");

        halt(&pool, id, kind, SAID).await.unwrap();

        assert_eq!(
            halted(&pool, id).await.unwrap().map(|stopped| stopped.halt),
            Some(kind),
        );
    }
}

/// A Conversation that is not there is not a failure to halt: it is a
/// Conversation nobody could ever start again, and nothing to write a Notice
/// about.
#[tokio::test]
async fn a_conversation_that_is_gone_takes_no_halt() {
    let (_dir, pool) = fresh_pool().await;

    assert_eq!(
        halt(&pool, 404, Halt::Circumstance, SAID).await.unwrap(),
        None
    );
    assert_eq!(halted(&pool, 404).await.unwrap(), None);
}

/// And clearing one that was never there does nothing at all, which is the
/// ordinary case for a Conversation being driven perfectly well.
#[tokio::test]
async fn clearing_a_halt_nothing_wrote_is_nothing_to_do() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool).await;

    clear_halt(&pool, id).await.unwrap();

    assert_eq!(halted(&pool, id).await.unwrap(), None);
}
