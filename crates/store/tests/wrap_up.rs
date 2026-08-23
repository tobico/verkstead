//! What wrap-up is still waiting on, how many goes the machine has had at a red
//! check, which comments it has already dispatched about — and the move to Done
//! that having settled all three is.
//!
//! The first three are bookkeeping rather than Timeline Events, and every one of
//! them has to survive a restart — which is the whole reason they are in the
//! database at all. So what these ask is what a second reader sees: a fresh pool
//! over the same file, as a restarted server has.

use std::path::Path;

use sqlx::SqlitePool;
use verkstead_store::{
    Event, Finished, Fixing, Lifecycle, Reviewed, Settlements, Submission, WAITED_ON, WaitingOn,
    addressed_comments, ask, finish_wrap_up, fix_attempts, forget_fix_attempts, load_conversation,
    open_database, pick_direction, record_addressed_comments, record_fix_attempt,
    record_pull_request, register_repo, review_asked, save_brief, settle_wrap_up,
    start_conversation, start_grilling, submit_response, timeline, unsettle_wrap_up,
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
    pick_direction(pool, id, verkstead_schema::Direction::TaskList)
        .await
        .unwrap();
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

/// Which comments have already had a session dispatched about them, and the
/// whole reason it is in the database: a server that came back up having
/// forgotten would dispatch a session about feedback that was addressed
/// yesterday.
#[tokio::test]
async fn which_comments_have_been_dispatched_for_survives_a_restart() {
    let dir = tempfile::tempdir().unwrap();
    let database = dir.path().join("verkstead.db");

    let id = {
        let pool = open_database(&database).await.unwrap();
        let id = wrapping(&pool).await;

        assert_eq!(
            addressed_comments(&pool, id).await.unwrap(),
            Vec::<String>::new(),
            "nothing has been dispatched for on a pull request nobody has said anything on",
        );

        // One batch, one write: three replies in a minute are one point being
        // made, and one session is dispatched about all of them.
        record_addressed_comments(&pool, id, &["IC_1".to_owned(), "IC_2".to_owned()])
            .await
            .unwrap();

        pool.close().await;
        id
    };

    let restarted = open_database(&database).await.unwrap();
    let mut already = addressed_comments(&restarted, id).await.unwrap();
    already.sort();

    assert_eq!(already, vec!["IC_1".to_owned(), "IC_2".to_owned()]);

    // And a batch that overlaps one already written down is the same comments
    // rather than a refusal: the poll that dispatched for `IC_2` may have been a
    // server that then restarted, and what matters is that it is written once.
    record_addressed_comments(&restarted, id, &["IC_2".to_owned(), "IC_3".to_owned()])
        .await
        .unwrap();

    let mut already = addressed_comments(&restarted, id).await.unwrap();
    already.sort();

    assert_eq!(
        already,
        vec!["IC_1".to_owned(), "IC_2".to_owned(), "IC_3".to_owned()],
    );
}

/// The rule that ends a wrap-up: the checks green, the review answered and
/// nothing said left unaddressed, all three together.
///
/// Verkstead decides it itself and records the move like every other — there is
/// nobody at the workbench to press anything, which is the whole of what running
/// unattended means.
#[tokio::test]
async fn a_wrap_up_with_all_three_settled_is_done_and_the_move_is_on_the_timeline() {
    let (_dir, pool) = fresh_pool().await;
    let id = wrapping(&pool).await;

    for waiting_on in WAITED_ON {
        settle_wrap_up(&pool, id, waiting_on).await.unwrap();
    }

    assert_eq!(finish_wrap_up(&pool, id).await.unwrap(), Finished::Done);

    let conversation = load_conversation(&pool, id).await.unwrap().unwrap();
    assert_eq!(conversation.state, Lifecycle::Done);

    let timeline = timeline(&pool, id).await.unwrap();

    assert_eq!(
        timeline.iter().rev().find_map(|event| match &event.event {
            Event::Moved(state) => Some(*state),
            _ => None,
        }),
        Some(Lifecycle::Done),
        "the move is the last thing to have happened, on the record like every other",
    );

    // Nothing waited on the pull request being merged: the PR this walked through
    // is open, nothing here ever asked whether it was not, and the Conversation
    // is finished all the same. Done means Verkstead has finished with the work,
    // not that it is on `main`.

    // And a second watcher asking finds the move already made rather than making
    // it twice.
    assert_eq!(
        finish_wrap_up(&pool, id).await.unwrap(),
        Finished::NotWrapping,
    );
}

/// Any one of the three missing keeps it in Wrapping — each of them in turn,
/// because a rule that held for two of the three would be a wrap-up that
/// finished with work outstanding.
#[tokio::test]
async fn missing_any_one_of_the_three_keeps_the_conversation_wrapping() {
    for missing in WAITED_ON {
        let (_dir, pool) = fresh_pool().await;
        let id = wrapping(&pool).await;

        for waiting_on in WAITED_ON.into_iter().filter(|one| *one != missing) {
            settle_wrap_up(&pool, id, waiting_on).await.unwrap();
        }

        assert_eq!(
            finish_wrap_up(&pool, id).await.unwrap(),
            Finished::StillWaiting,
            "{missing:?} is outstanding, so the wrap-up is not over",
        );

        assert_eq!(
            load_conversation(&pool, id).await.unwrap().unwrap().state,
            Lifecycle::Wrapping,
        );

        // And settling the last of them is what finishes it.
        settle_wrap_up(&pool, id, missing).await.unwrap();
        assert_eq!(finish_wrap_up(&pool, id).await.unwrap(), Finished::Done);
    }
}

/// A wrap-up that settled everything and then had one of them come undone — a
/// commit landing on the pull request is a new run to wait on — is a wrap-up
/// still going.
#[tokio::test]
async fn checks_that_stop_being_settled_leave_a_finished_wrap_up_unfinishable() {
    let (_dir, pool) = fresh_pool().await;
    let id = wrapping(&pool).await;

    for waiting_on in WAITED_ON {
        settle_wrap_up(&pool, id, waiting_on).await.unwrap();
    }

    unsettle_wrap_up(&pool, id, WaitingOn::Checks)
        .await
        .unwrap();

    assert_eq!(
        finish_wrap_up(&pool, id).await.unwrap(),
        Finished::StillWaiting,
    );
}

/// The review's Set as the reviewing skill writes one: a Question per finding,
/// and the block that says which Answer to each means *fix it*.
fn reviewing() -> verkstead_schema::QuestionSet {
    verkstead_schema::QuestionSet::from_yaml(
        r#"
title: Review of the rate limiter branch
questions:
  - label: Q1
    text: The window counter is never reset between windows.
    options:
      - n: 1
        text: Fix it
        recommended: true
      - n: 2
        text: Leave it
  - label: Q2
    text: Two clocks now, and the tests pin both.
    options:
      - n: 1
        text: Fix it
      - n: 2
        text: Leave it
        recommended: true
review:
  findings:
    - fix: Q1.1
      what: Reset the counter as the window rolls.
    - fix: Q2.1
      what: Collapse the two clocks onto one.
"#,
    )
    .unwrap()
}

/// Answering the review is what settles it, and what the human accepted is
/// handed back as work to dispatch.
///
/// The findings come back in the order the review raised them rather than the
/// order they were answered in: that is the order it thought about them.
#[tokio::test]
async fn answering_the_review_settles_it_and_hands_back_what_to_fix() {
    let (_dir, pool) = fresh_pool().await;
    let id = wrapping(&pool).await;

    let asked = ask(&pool, id, &reviewing())
        .await
        .unwrap()
        .expect("the Conversation is there to ask from");

    let taken = submit_response(
        &pool,
        &Settlements::new(8),
        asked.id,
        &verkstead_schema::Response::from_yaml(
            "answers:\n  \
             - label: Q1\n    selected: 1\n    free_text: Keep the signature.\n  \
             - label: Q2\n    selected: 2\n",
        )
        .unwrap(),
    )
    .await
    .unwrap();

    let Submission::Accepted(taken) = taken else {
        panic!("the Response resolves the Set, so it should be taken: {taken:?}");
    };

    assert_eq!(
        taken.reviewed,
        Some(Reviewed::Answered {
            conversation_id: id,
            fixing: vec![Fixing {
                what: "Reset the counter as the window rolls.".to_owned(),
                said: "Keep the signature.".to_owned(),
            }],
        }),
        "the accepted finding is work, with what they said alongside it; the declined \
         one is nothing at all",
    );

    assert!(
        wrap_up_settled(&pool, id)
            .await
            .unwrap()
            .contains(&WaitingOn::Review),
        "and the review has stopped being something wrap-up waits on",
    );
}

/// A review the human declined every finding of is an answered review, not an
/// unanswered one: they read it and decided, which is the whole of what wrap-up
/// was waiting for.
#[tokio::test]
async fn a_review_answered_with_nothing_to_fix_is_still_answered() {
    let (_dir, pool) = fresh_pool().await;
    let id = wrapping(&pool).await;

    let asked = ask(&pool, id, &reviewing()).await.unwrap().unwrap();

    let taken = submit_response(
        &pool,
        &Settlements::new(8),
        asked.id,
        &verkstead_schema::Response::from_yaml(
            "answers:\n  - label: Q1\n    selected: 2\n  - label: Q2\n    unanswered: true\n",
        )
        .unwrap(),
    )
    .await
    .unwrap();

    let Submission::Accepted(taken) = taken else {
        panic!("the Response resolves the Set: {taken:?}");
    };

    assert_eq!(
        taken.reviewed,
        Some(Reviewed::Answered {
            conversation_id: id,
            fixing: Vec::new(),
        }),
        "nothing to dispatch",
    );
    assert!(
        wrap_up_settled(&pool, id)
            .await
            .unwrap()
            .contains(&WaitingOn::Review),
        "and the review is over either way",
    );
}

/// An ordinary Set is not the review's, however it is answered — otherwise every
/// question an agent asked during a wrap-up would settle it.
#[tokio::test]
async fn answering_an_ordinary_set_settles_no_review() {
    let (_dir, pool) = fresh_pool().await;
    let id = wrapping(&pool).await;

    let ordinary = verkstead_schema::QuestionSet {
        review: None,
        ..reviewing()
    };

    let asked = ask(&pool, id, &ordinary).await.unwrap().unwrap();

    let taken = submit_response(
        &pool,
        &Settlements::new(8),
        asked.id,
        &verkstead_schema::Response::from_yaml(
            "answers:\n  - label: Q1\n    selected: 1\n  - label: Q2\n    selected: 1\n",
        )
        .unwrap(),
    )
    .await
    .unwrap();

    let Submission::Accepted(taken) = taken else {
        panic!("the Response resolves the Set: {taken:?}");
    };

    assert_eq!(taken.reviewed, None);
    assert_eq!(wrap_up_settled(&pool, id).await.unwrap(), Vec::new());
}

/// Which Set the review is on is read off the Sets themselves, so that nothing
/// has to be written down twice — and the review is the Set carrying findings,
/// not whichever one came first.
#[tokio::test]
async fn the_review_is_found_by_the_block_it_carries() {
    let (_dir, pool) = fresh_pool().await;
    let id = wrapping(&pool).await;

    assert_eq!(
        review_asked(&pool, id).await.unwrap(),
        None,
        "a wrap-up nobody has reviewed has no review to find",
    );

    let ordinary = verkstead_schema::QuestionSet {
        review: None,
        ..reviewing()
    };
    ask(&pool, id, &ordinary).await.unwrap().unwrap();

    assert_eq!(
        review_asked(&pool, id).await.unwrap(),
        None,
        "and an ordinary Set is not one",
    );

    let asked = ask(&pool, id, &reviewing()).await.unwrap().unwrap();

    assert_eq!(review_asked(&pool, id).await.unwrap(), Some(asked.id));
}
