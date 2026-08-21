//! Interruptions on a Conversation's Timeline: raised with their evidence,
//! settled once, and open exactly one at a time.
//!
//! What is being asked of the store here is *the run stops here*. An Interruption
//! is the one Event that holds a run up, so the two rules worth a test are the
//! ones a promise could not keep: a Conversation has at most one open, and the
//! first remedy chosen is the one that stands.

use std::path::Path;

use sqlx::SqlitePool;
use verkstead_store::{
    Event, Evidence, Interruption, Remedy, Settled, Settling, Step, interruption, open_database,
    open_interruption, record_interruption, register_repo, settle_interruption, start_conversation,
    timeline,
};

/// A pool over a fresh database, plus the directory keeping it alive.
async fn fresh_pool() -> (tempfile::TempDir, SqlitePool) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();
    (dir, pool)
}

/// A Conversation for a run to stop in.
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

/// What a run that stopped on a task gathered.
fn evidence() -> Evidence {
    Evidence {
        step: Step::Task,
        what: "task 03 of the backlog".to_owned(),
        how: "the session exited with status 1".to_owned(),
        git_status: " M crates/limiter/src/lib.rs\n?? crates/limiter/src/window.rs\n".to_owned(),
        tail: "error[E0432]: unresolved import `crate::window`".to_owned(),
    }
}

/// The Interruptions on a Conversation's Timeline, in Timeline order.
async fn on_the_timeline(pool: &SqlitePool, id: i64) -> Vec<Interruption> {
    timeline(pool, id)
        .await
        .unwrap()
        .into_iter()
        .filter_map(|event| match event.event {
            Event::Interruption(interruption) => Some(*interruption),
            _ => None,
        })
        .collect()
}

#[tokio::test]
async fn an_interruption_lands_on_the_timeline_with_its_evidence() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool).await;

    let event = record_interruption(&pool, id, &evidence())
        .await
        .unwrap()
        .expect("a Conversation that is there takes one");

    assert_eq!(
        on_the_timeline(&pool, id).await,
        vec![Interruption {
            evidence: evidence(),
            settled: None,
        }],
        "the Timeline carries all four pieces of evidence, and nothing settled",
    );

    assert_eq!(
        interruption(&pool, id, event).await.unwrap(),
        Some(Interruption {
            evidence: evidence(),
            settled: None,
        }),
        "and reading the one Event says the same",
    );

    assert_eq!(
        open_interruption(&pool, id).await.unwrap(),
        Some(event),
        "an Interruption nobody has answered is what stops the run",
    );
}

/// The rule the partial unique index is there for. A run stops at its first
/// Interruption, so a second raised against the same Conversation is something
/// that got past the guard — two watchers noticing the same dead session — and
/// the first is the one the human is being asked about.
#[tokio::test]
async fn a_conversation_has_at_most_one_open_interruption() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool).await;

    let first = record_interruption(&pool, id, &evidence())
        .await
        .unwrap()
        .expect("the first one is raised");

    assert_eq!(
        record_interruption(&pool, id, &evidence()).await.unwrap(),
        None,
        "the second is refused rather than raised",
    );

    assert_eq!(
        on_the_timeline(&pool, id).await.len(),
        1,
        "and the Timeline holds one, not two",
    );

    assert_eq!(open_interruption(&pool, id).await.unwrap(), Some(first));
}

/// Settling one closes it, which is what lets the next thing that goes wrong be
/// raised: a Conversation collects any number of settled Interruptions over a
/// long run, and exactly one that is stopping it.
#[tokio::test]
async fn a_settled_interruption_stops_holding_the_run_up() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool).await;

    let event = record_interruption(&pool, id, &evidence())
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        settle_interruption(&pool, id, event, Remedy::Retry, "leave the migration alone")
            .await
            .unwrap(),
        Settling::Settled,
    );

    assert_eq!(
        open_interruption(&pool, id).await.unwrap(),
        None,
        "nothing is stopping the run any more",
    );

    let Some(Interruption {
        settled: Some(Settled { remedy, note, at }),
        ..
    }) = interruption(&pool, id, event).await.unwrap()
    else {
        panic!("a settled Interruption reads back settled");
    };

    assert_eq!(remedy, Remedy::Retry);
    assert_eq!(
        note, "leave the migration alone",
        "what the human wrote is what reaches the agent that can act on it",
    );
    assert!(!at.is_empty(), "and when they chose");

    // And now the run can stop again, which is the whole reason the index is
    // partial.
    assert!(
        record_interruption(&pool, id, &evidence())
            .await
            .unwrap()
            .is_some(),
        "a fresh one is raised once the last is answered",
    );
}

/// The human answers from whichever device is to hand, so the second press of a
/// button is the first choice arriving again rather than a new decision.
#[tokio::test]
async fn the_first_remedy_chosen_is_the_one_that_stands() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool).await;

    let event = record_interruption(&pool, id, &evidence())
        .await
        .unwrap()
        .unwrap();

    settle_interruption(&pool, id, event, Remedy::Retry, "try again")
        .await
        .unwrap();

    assert_eq!(
        settle_interruption(&pool, id, event, Remedy::Abort, "")
            .await
            .unwrap(),
        Settling::AlreadySettled,
        "the second press is not a second decision",
    );

    let settled = interruption(&pool, id, event)
        .await
        .unwrap()
        .unwrap()
        .settled
        .unwrap();

    assert_eq!(settled.remedy, Remedy::Retry, "the first choice stands");
    assert_eq!(settled.note, "try again");
}

/// An Interruption is reached through the Timeline it is on, exactly as a
/// transcript and a commit are: an Event id belonging to another Conversation
/// names nothing here.
#[tokio::test]
async fn an_interruption_belongs_to_the_conversation_it_stopped() {
    let (_dir, pool) = fresh_pool().await;
    let stopped = conversation(&pool).await;

    let elsewhere = start_conversation(&pool, 1, "other-work")
        .await
        .unwrap()
        .expect("the same Repo takes a second Conversation");

    let event = record_interruption(&pool, stopped, &evidence())
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        interruption(&pool, elsewhere, event).await.unwrap(),
        None,
        "another Conversation's Event names nothing",
    );

    assert_eq!(
        settle_interruption(&pool, elsewhere, event, Remedy::Abort, "")
            .await
            .unwrap(),
        Settling::NoSuchInterruption,
        "and cannot be settled from there either",
    );

    assert_eq!(
        open_interruption(&pool, elsewhere).await.unwrap(),
        None,
        "nor does it stop a run it was never part of",
    );
}

/// Every step a session is launched for, so that a retry of any of them knows
/// what to launch again.
#[tokio::test]
async fn every_step_a_run_can_stop_on_reads_back_as_itself() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool).await;

    for step in [Step::Planning, Step::Task, Step::Finish, Step::Inline] {
        let event = record_interruption(&pool, id, &Evidence { step, ..evidence() })
            .await
            .unwrap()
            .unwrap();

        assert_eq!(
            interruption(&pool, id, event)
                .await
                .unwrap()
                .unwrap()
                .evidence
                .step,
            step,
        );

        settle_interruption(&pool, id, event, Remedy::TakeOver, "")
            .await
            .unwrap();
    }
}

/// And every remedy, for the same reason: what the human chose is what the
/// server acts on, and a word the store could not read back would be a choice
/// nothing acted on.
#[tokio::test]
async fn every_remedy_reads_back_as_itself() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool).await;

    for remedy in [Remedy::Retry, Remedy::TakeOver, Remedy::Abort] {
        let event = record_interruption(&pool, id, &evidence())
            .await
            .unwrap()
            .unwrap();

        settle_interruption(&pool, id, event, remedy, "")
            .await
            .unwrap();

        assert_eq!(
            interruption(&pool, id, event)
                .await
                .unwrap()
                .unwrap()
                .settled
                .unwrap()
                .remedy,
            remedy,
        );
    }
}

/// A Conversation that is not there is not a failure to raise one against: it is
/// a run whose record has gone, and there is nobody left to ask.
#[tokio::test]
async fn an_interruption_against_no_conversation_is_raised_against_nothing() {
    let (_dir, pool) = fresh_pool().await;

    assert_eq!(
        record_interruption(&pool, 404, &evidence()).await.unwrap(),
        None,
    );
}
