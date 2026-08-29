//! Stops on a Conversation: written with the Notice that explains them, held
//! one at a time however the run stopped, and cleared when driving starts
//! again.
//!
//! What is being asked of the store here is *this Conversation is stopped*. The
//! one stop is the one thing that says so, so the rules worth a test are the
//! ones a promise could not keep: a Conversation has at most one, its Notice is
//! on the Timeline beside it, and clearing it leaves the Notice where it is.

use std::path::Path;

use sqlx::SqlitePool;
use verkstead_store::{
    ConversationRow, Decision, Event, Stopped, Stopping, ask_to_stop, asked_to_stop, clear_stop,
    close_conversation, conversations, forget_stop, open_database, register_repo,
    start_conversation, start_grilling, stop, stop_as_asked, stopped, timeline,
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

/// What Verkstead said about the stop, as its Notice carries it.
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
async fn a_stop_lands_with_the_notice_that_explains_it() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool).await;

    let notice = stop(&pool, id, Decision::Circumstance, SAID, None)
        .await
        .unwrap()
        .expect("a Conversation that is there stops");

    let it = stopped(&pool, id)
        .await
        .unwrap()
        .expect("it is stopped, which is what was just written");

    assert_eq!(
        it,
        Stopped {
            decision: Decision::Circumstance,
            notice,
            at: it.at.clone(),
            resets: None,
        },
        "the kind of stop it was, and the Event that says what it was",
    );
    assert!(
        it.at.starts_with("20"),
        "with when it stopped, RFC 3339: {:?}",
        it.at,
    );

    assert_eq!(
        notices(&pool, id).await,
        vec![SAID.to_owned()],
        "and what Verkstead has to say about it is an ordinary Notice",
    );
}

/// An account out of window stops a Conversation the same way, and the only
/// thing that tells it apart is the words it carries about the window coming
/// back.
#[tokio::test]
async fn a_stop_for_a_window_is_the_same_stop_with_reset_words_on_it() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool).await;

    let notice = stop(
        &pool,
        id,
        Decision::Verkstead,
        SAID,
        Some("2026-08-24T05:00:00Z"),
    )
    .await
    .unwrap()
    .expect("a Conversation that is there stops");

    let it = stopped(&pool, id).await.unwrap().expect("it is stopped");

    assert_eq!(
        (it.decision, it.notice, it.resets.as_deref()),
        (Decision::Verkstead, notice, Some("2026-08-24T05:00:00Z"),),
        "one Notice and one badge behind it, with the reset words beside them",
    );
}

/// The rule the one-stop-per-Conversation record is there for. A Conversation
/// that is stopped is stopped once: the sweep looks again a minute later and
/// finds the same Conversation standing just as still, and that is not news.
#[tokio::test]
async fn a_conversation_is_stopped_once_however_often_it_is_noticed() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool).await;

    let first = stop(&pool, id, Decision::Circumstance, SAID, None)
        .await
        .unwrap()
        .expect("a Conversation that is there stops");

    assert_eq!(
        stop(&pool, id, Decision::Human, "and again", None)
            .await
            .unwrap(),
        None,
        "the second one is the same stop arriving twice",
    );

    assert_eq!(
        stopped(&pool, id).await.unwrap().map(|it| it.decision),
        Some(Decision::Circumstance),
        "so the first stop is the one that stands, kind and all",
    );
    assert_eq!(
        notices(&pool, id).await,
        vec![SAID.to_owned()],
        "and the Timeline is told once, not once a minute",
    );

    assert_eq!(
        stopped(&pool, id).await.unwrap().map(|it| it.notice),
        Some(first),
        "pointing at the Notice that explained it in the first place",
    );
}

/// Clearing it is what starting to drive again does — and it takes away the
/// stop alone. The Notice is a record of a stop that really happened, and a
/// Timeline that took yesterday's back would be one nobody could read.
#[tokio::test]
async fn driving_again_clears_the_stop_and_leaves_the_notice() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool).await;

    stop(&pool, id, Decision::Verkstead, SAID, Some("3pm"))
        .await
        .unwrap();
    clear_stop(&pool, id).await.unwrap();

    assert_eq!(
        stopped(&pool, id).await.unwrap(),
        None,
        "nothing is stopping it any more, and no reset words are left over",
    );
    assert_eq!(
        notices(&pool, id).await,
        vec![SAID.to_owned()],
        "and what stopped it is still on the record",
    );

    assert!(
        stop(&pool, id, Decision::Human, "stopped again", None)
            .await
            .unwrap()
            .is_some(),
        "which is what leaves room for the next stop to be written",
    );
}

/// Every kind reads back as itself. Which one a stop is decides two things a
/// promise could not keep — whether a restarting server starts the work again
/// or leaves it alone, and whether the human is marked as being waited on — so
/// a word the store could not read back would be two decisions nothing could
/// act on.
///
/// The stored word is asserted beside the kind, because it is written into a
/// database that outlives this build: a word quietly renamed would leave every
/// stop written before it unreadable, and the migration reading yesterday's
/// Pauses writes one of these by hand.
#[tokio::test]
async fn every_kind_of_stop_reads_back_as_itself() {
    let (_dir, pool) = fresh_pool().await;
    let repo = register_repo(&pool, Path::new("/watched/verkstead"), "verkstead", "main")
        .await
        .unwrap()
        .expect("nothing was registered at that path yet")
        .id;

    for (kind, word, branch) in [
        (Decision::Verkstead, "verkstead", "brake-pulled"),
        (Decision::Human, "human", "asked-to-stop"),
        (Decision::Deliberate, "deliberate", "stopped-long-ago"),
        (Decision::Circumstance, "circumstance", "left-mid-run"),
    ] {
        let id = start_conversation(&pool, repo, branch)
            .await
            .unwrap()
            .expect("the Repo was just registered");

        stop(&pool, id, kind, SAID, None).await.unwrap();

        assert_eq!(
            stopped(&pool, id).await.unwrap().map(|it| it.decision),
            Some(kind),
        );

        let (stored,): (String,) =
            sqlx::query_as("SELECT stopped_by FROM conversations WHERE id = ?")
                .bind(id)
                .fetch_one(&pool)
                .await
                .unwrap();

        assert_eq!(stored, word, "the word the column holds for {kind:?}");
    }
}

/// And each kind answers the two questions the same way every time.
///
/// A restart asks the first: everything somebody decided waits for a press, and
/// only what nobody decided is taken up unasked. The marks ask the second, and
/// it is the narrower one — the human's own press waits for their press like
/// the rest and is not something they have to be told about, so the two rows
/// here that are theirs answer *yes* and *no*.
#[test]
fn each_kind_says_who_waits_for_it_and_who_is_waited_on() {
    for (kind, decided, waits_on_the_human) in [
        (Decision::Verkstead, true, true),
        (Decision::Human, true, false),
        (Decision::Deliberate, true, false),
        (Decision::Circumstance, false, true),
    ] {
        assert_eq!(kind.decided(), decided, "who a {kind:?} stop waits for");
        assert_eq!(
            kind.waits_on_the_human(),
            waits_on_the_human,
            "and whether a {kind:?} stop is marked as waiting on them",
        );
    }
}

/// A Conversation that is not there is not a failure to stop: it is a
/// Conversation nobody could ever start again, and nothing to write a Notice
/// about.
#[tokio::test]
async fn a_conversation_that_is_gone_takes_no_stop() {
    let (_dir, pool) = fresh_pool().await;

    assert_eq!(
        stop(&pool, 404, Decision::Circumstance, SAID, None)
            .await
            .unwrap(),
        None
    );
    assert_eq!(stopped(&pool, 404).await.unwrap(), None);
}

/// And clearing one that was never there does nothing at all, which is the
/// ordinary case for a Conversation being driven perfectly well.
#[tokio::test]
async fn clearing_a_stop_nothing_wrote_is_nothing_to_do() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool).await;

    clear_stop(&pool, id).await.unwrap();

    assert_eq!(stopped(&pool, id).await.unwrap(), None);
}

/// The Stop asked for that has not landed yet hangs off the Conversation the
/// same way, and for the same reason: one Conversation asks to stop once, and a
/// server restarted in the gap has to find that it did.
#[tokio::test]
async fn a_conversation_asks_to_stop_once() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool).await;

    assert!(!asked_to_stop(&pool, id).await.unwrap());

    ask_to_stop(&pool, id).await.unwrap();
    ask_to_stop(&pool, id).await.unwrap();

    assert!(
        asked_to_stop(&pool, id).await.unwrap(),
        "the second press is the first one arriving again",
    );

    forget_stop(&pool, id).await.unwrap();

    assert!(
        !asked_to_stop(&pool, id).await.unwrap(),
        "and what Resume overtakes is forgotten",
    );
}

/// A stop that stands on the human's request stands on it still being there
/// when it is written — and the write is what takes it away.
///
/// Nothing else here would say so. The run reads the request in front of a
/// launch and writes the stop a moment later, and a Steer or a Resume in that
/// moment takes the request back: what they are overtaking is a stop nothing has
/// written yet. Written anyway, it lands on the far side of the press that undid
/// it — the Conversation has moved, a fresh run is starting, and it stops that
/// run before it has launched anything.
#[tokio::test]
async fn a_stop_asked_for_and_taken_back_is_not_written() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool).await;

    assert_eq!(
        stop_as_asked(&pool, id, Decision::Human, SAID, None)
            .await
            .unwrap(),
        Stopping::Withdrawn,
        "nobody asked, so there is nothing here to land",
    );
    assert_eq!(stopped(&pool, id).await.unwrap(), None);
    assert!(notices(&pool, id).await.is_empty());

    ask_to_stop(&pool, id).await.unwrap();

    let landed = stop_as_asked(&pool, id, Decision::Human, SAID, None)
        .await
        .unwrap();

    assert!(
        matches!(landed, Stopping::Stopped(_)),
        "and asked for, it lands: {landed:?}",
    );
    assert_eq!(notices(&pool, id).await, vec![SAID.to_owned()]);
    assert!(
        !asked_to_stop(&pool, id).await.unwrap(),
        "the request goes with the stop it became, rather than landing again at \
         the next launch",
    );

    // And the stop an ordinary run makes is not held to any of that: what it
    // stands on is what the run has to say, and nobody pressed anything.
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool).await;

    assert!(
        stop(&pool, id, Decision::Verkstead, SAID, None)
            .await
            .unwrap()
            .is_some(),
        "Verkstead's own brake needs no request behind it",
    );
}

/// A Conversation stopped by something outside the human is one the sidebar
/// draws as waiting on them.
///
/// The badge on its own page points at the Notice; the list has no Notice to
/// point at and only the dot, so what it needs is the fact — and a stop the
/// sidebar said nothing about would be one the human found by opening every
/// Conversation they have.
#[tokio::test]
async fn a_stop_from_outside_the_human_is_waiting_on_them_in_the_sidebar() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool).await;

    start_grilling(
        &pool,
        id,
        "abc1234",
        Path::new("/data/worktrees/rate-limiting"),
        &[],
    )
    .await
    .unwrap();

    let waiting = |rows: Vec<ConversationRow>| {
        rows.into_iter()
            .find(|row| row.id == id)
            .expect("the Conversation is on the list")
            .waiting
    };

    assert!(
        !waiting(conversations(&pool).await.unwrap()),
        "a Conversation being grilled is not waiting on anybody",
    );

    stop(&pool, id, Decision::Verkstead, SAID, None)
        .await
        .unwrap();

    assert!(
        waiting(conversations(&pool).await.unwrap()),
        "and one Verkstead pulled the brake on is",
    );

    clear_stop(&pool, id).await.unwrap();

    assert!(
        !waiting(conversations(&pool).await.unwrap()),
        "and starting to drive again takes the dot with it, leaving the Notice \
         where it is",
    );
}

/// And a stop the human made themselves is not, however plainly it is a stop.
///
/// It still stands there waiting for their press — nothing about that changes —
/// but the dot means *something happened without you*, and a dot on the work
/// they pressed Stop on last is the one that teaches them to stop reading the
/// dots. A row stored before the two were told apart reads the same way: it
/// cannot be told apart now either, and their own presses are what nearly all
/// of those rows are.
#[tokio::test]
async fn the_humans_own_stop_is_not_waiting_on_them_in_the_sidebar() {
    let (_dir, pool) = fresh_pool().await;
    let repo = register_repo(&pool, Path::new("/watched/verkstead"), "verkstead", "main")
        .await
        .unwrap()
        .expect("nothing was registered at that path yet")
        .id;

    for (kind, branch) in [
        (Decision::Human, "asked-to-stop"),
        (Decision::Deliberate, "stopped-long-ago"),
        (Decision::Circumstance, "left-mid-run"),
        (Decision::Verkstead, "brake-pulled"),
    ] {
        let id = start_conversation(&pool, repo, branch)
            .await
            .unwrap()
            .expect("the Repo was just registered");

        start_grilling(
            &pool,
            id,
            "abc1234",
            Path::new("/data/worktrees/rate-limiting"),
            &[],
        )
        .await
        .unwrap();

        stop(&pool, id, kind, SAID, None).await.unwrap();

        let waiting = conversations(&pool)
            .await
            .unwrap()
            .into_iter()
            .find(|row| row.id == id)
            .expect("the Conversation is on the list")
            .waiting;

        assert_eq!(
            waiting,
            kind.waits_on_the_human(),
            "the sidebar's dot follows the word the stop was written with, and \
             this one is {kind:?}",
        );
    }
}

/// And a stop on a Conversation the human has closed is not either, however it
/// stopped.
///
/// Closing is them saying the work is over wherever it had got to, so the stop
/// stops being something to come back to: the dot means *there is something here
/// for you*, and there is not. The record is left exactly as it was — it is what
/// happened, and the Notice explaining it is still on the Timeline — so what
/// changes is only what the sidebar makes of it.
#[tokio::test]
async fn a_closed_conversation_is_not_waiting_on_them_whatever_stopped_it() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool).await;

    start_grilling(
        &pool,
        id,
        "abc1234",
        Path::new("/data/worktrees/rate-limiting"),
        &[],
    )
    .await
    .unwrap();

    stop(&pool, id, Decision::Verkstead, SAID, None)
        .await
        .unwrap();

    let waiting = |rows: Vec<ConversationRow>| {
        rows.into_iter()
            .find(|row| row.id == id)
            .expect("the Conversation is on the list")
            .waiting
    };

    assert!(
        waiting(conversations(&pool).await.unwrap()),
        "Verkstead pulled the brake, so until it is closed this is waiting on them",
    );

    close_conversation(&pool, id).await.unwrap();

    assert!(
        !waiting(conversations(&pool).await.unwrap()),
        "and closing takes the dot away, whatever the stop was",
    );
    assert!(
        stopped(&pool, id).await.unwrap().is_some(),
        "leaving the stop itself where it is: closing reads it, and writes nothing",
    );
}
