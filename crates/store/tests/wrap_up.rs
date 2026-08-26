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
    Archiving, Ask, Event, Finished, Lifecycle, Settlements, Steer, Steering, Submission,
    WAITED_ON, WaitingOn, addressed_comments, archive_set, ask, finish_wrap_up, fix_attempts,
    forget_addressed_comments, forget_fix_attempts, implement_again, last_batch_proposal,
    last_proposal, load_conversation, load_response, open_database, pick_direction,
    record_addressed_comments, record_fix_attempt, record_pull_request, register_repo, save_brief,
    settle_wrap_up, start_conversation, start_grilling, steer_conversation, submit_response,
    timeline, unsettle_wrap_up, wrap_up_settled,
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
    pick_direction(pool, id, verkstead_schema::Direction::Inline)
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

/// What Resume does. The human has read the Notice of what stopped and asked
/// for another go, and a count left standing would be a watcher that stopped all
/// over again on its next poll without dispatching anything.
#[tokio::test]
async fn resuming_gives_every_check_its_attempts_back() {
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

/// A second round wraps up from nothing. The round before it settled all three
/// and addressed whatever was said on the pull request; a round that inherited
/// the first would be over the moment it reached Wrapping.
///
/// The comments are what it keeps, and deliberately: a comment somebody wrote and
/// a session answered stays answered, where every check and every review belongs
/// to the round that ran them.
#[tokio::test]
async fn a_second_round_forgets_what_the_round_before_it_settled() {
    let (_dir, pool) = fresh_pool().await;
    let id = wrapping(&pool).await;

    for waiting_on in WAITED_ON {
        settle_wrap_up(&pool, id, waiting_on).await.unwrap();
    }
    record_fix_attempt(&pool, id, "build").await.unwrap();
    record_addressed_comments(&pool, id, &["IC_1".to_owned()])
        .await
        .unwrap();

    assert_eq!(finish_wrap_up(&pool, id).await.unwrap(), Finished::Done);

    assert_eq!(
        steer_conversation(
            &pool,
            id,
            Steer {
                target: Lifecycle::Grilling,
                pairing: None,
                brief: Some("# Rate limiting, per account\n"),
                instruction: None,
                direction: None,
                worktree: Some(Path::new("/state/worktrees/rate-limiting")),
                base_commit: None,
            },
        )
        .await
        .unwrap(),
        Steering::Steered
    );

    assert_eq!(
        wrap_up_settled(&pool, id).await.unwrap(),
        Vec::new(),
        "the new round waits on all three again"
    );
    assert_eq!(
        fix_attempts(&pool, id, "build").await.unwrap(),
        0,
        "and its checks start from no attempts spent"
    );
    assert_eq!(
        addressed_comments(&pool, id).await.unwrap(),
        vec!["IC_1".to_owned()],
        "but a comment already answered stays answered"
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
/// each offering a way to fix it beside leaving it alone.
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
"#,
    )
    .unwrap()
}

/// Answering the review moves nothing at all.
///
/// The session that raised the findings is still running, waiting on exactly
/// this Response: it fixes what was accepted and pushes, and its ending cleanly
/// is what settles the review. A store that settled it here would call the
/// review over at the moment the decisions were made rather than the moment they
/// were carried out.
///
/// What the Answers *are* is kept beside the Questions they answer, which is
/// what the human reads back and what the exchange is drawn from. Nothing here
/// acts on them: the session that asked is the one thing that reads a pick.
#[tokio::test]
async fn answering_the_review_settles_nothing_and_leaves_the_answers_on_the_set() {
    let (_dir, pool) = fresh_pool().await;
    let id = wrapping(&pool).await;

    let asked = ask(&pool, id, &reviewing(), Ask::Blocking)
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

    let Submission::Accepted(_) = taken else {
        panic!("the Response resolves the Set, so it should be taken: {taken:?}");
    };

    assert!(
        !wrap_up_settled(&pool, id)
            .await
            .unwrap()
            .contains(&WaitingOn::Review),
        "the review is still what wrap-up is waiting on: its session has the answers \
         and the fixing to do",
    );

    // And what they said is on the Set where the human can read it back: the
    // exchange the page draws, and the record of a decision that was made. What
    // it is not is anything Verkstead acts on — the session that asked is the one
    // thing that reads these picks, and it has them.
    let answered = load_response(&pool, asked.id)
        .await
        .unwrap()
        .expect("the Response was just stored")
        .response;

    assert_eq!(
        answered.answers[0].free_text.as_deref(),
        Some("Keep the signature."),
        "what they said beside their Answer is kept as they wrote it",
    );
}

/// Which Set the review is on is read off the Sets themselves, so that nothing
/// has to be written down twice — and it is whatever this wrap last asked,
/// because nothing marks a review's Set as one.
///
/// Which is the point of it: a review's Set is an ordinary Set, and what says
/// there is one asking is a Set of this wrap's standing on the Timeline. One
/// Worktree holds one agent and nothing advances past a stop, so the newest is
/// the one the session that is running put up.
#[tokio::test]
async fn the_review_is_whatever_set_this_wrap_last_asked() {
    let (_dir, pool) = fresh_pool().await;
    let id = wrapping(&pool).await;

    assert_eq!(
        last_proposal(&pool, id).await.unwrap(),
        None,
        "a wrap-up nobody has asked anything on has no review to find",
    );

    let first = ask(&pool, id, &reviewing(), Ask::Blocking)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(last_proposal(&pool, id).await.unwrap(), Some(first.id));

    let asked_since = ask(&pool, id, &answering_the_comments(), Ask::Blocking)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        last_proposal(&pool, id).await.unwrap(),
        Some(asked_since.id),
        "and anything asked since is what is standing now",
    );
}

/// A proposal closed unanswered is one nothing is left to act on, so it stops
/// counting as this wrap's review.
///
/// Which is what lets a review be run again after the session that asked has
/// gone: Verkstead closes the Set nobody is behind, and the branch is read afresh
/// by a session whose findings are then the review's. A closed Set left counting
/// would make that second reading a review nothing recognised — and the first,
/// which nobody will ever answer, the one every later question was asked of.
#[tokio::test]
async fn a_proposal_closed_unanswered_stops_being_the_review() {
    let (_dir, pool) = fresh_pool().await;
    let settlements = Settlements::new(8);
    let id = wrapping(&pool).await;

    let abandoned = ask(&pool, id, &reviewing(), Ask::Blocking)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(last_proposal(&pool, id).await.unwrap(), Some(abandoned.id));

    assert!(
        matches!(
            archive_set(&pool, &settlements, abandoned.id)
                .await
                .unwrap(),
            Archiving::Archived(_),
        ),
        "the Set nobody is behind is closed unanswered",
    );

    assert_eq!(
        last_proposal(&pool, id).await.unwrap(),
        None,
        "so this wrap reads as one nobody has reviewed, and as one nothing has been \
         proposed about",
    );

    let read_again = ask(&pool, id, &reviewing(), Ask::Blocking)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        last_proposal(&pool, id).await.unwrap(),
        Some(read_again.id),
        "and what the session that read the branch again asks is the review's",
    );
}

/// The Set a batch session asked with, which is the review's shape about what
/// somebody said rather than about the branch.
fn answering_the_comments() -> verkstead_schema::QuestionSet {
    verkstead_schema::QuestionSet::from_yaml(
        r#"
title: What was said on the rate limiter's pull request
questions:
  - label: Q1
    text: You said the reset is the wrong way round. It is.
    options:
      - n: 1
        text: Do it
        recommended: true
      - n: 2
        text: Leave it
"#,
    )
    .unwrap()
}

/// A batch's own proposal is told from the review's by the review's settle, and
/// by nothing on the Set itself.
///
/// Two proposals stand on a wrap-up that has been commented on: the review's,
/// which settled first, and the batch's, which is only ever asked afterwards.
/// Which is which is the question this decides — and nothing about a Set says
/// it, so the moment the review settled is the line: put up before it and the
/// Set is the review's own, after it and it is a batch's.
#[tokio::test]
async fn a_batchs_own_proposal_is_told_from_the_reviews_by_the_reviews_settle() {
    let (_dir, pool) = fresh_pool().await;
    let id = wrapping(&pool).await;

    assert_eq!(
        last_proposal(&pool, id).await.unwrap(),
        None,
        "a wrap-up nobody has proposed anything on has no proposal to find",
    );
    assert_eq!(
        last_batch_proposal(&pool, id).await.unwrap(),
        None,
        "and none of it is a batch's",
    );

    let reviewed = ask(&pool, id, &reviewing(), Ask::Blocking)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        last_batch_proposal(&pool, id).await.unwrap(),
        None,
        "a review still running is no batch's proposal: nothing is dispatched about \
         what was said until it is over",
    );

    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    settle_wrap_up(&pool, id, WaitingOn::Review).await.unwrap();

    assert_eq!(
        last_proposal(&pool, id).await.unwrap(),
        Some(reviewed.id),
        "the review's is the newest proposal on the Timeline until a batch asks",
    );
    assert_eq!(
        last_batch_proposal(&pool, id).await.unwrap(),
        None,
        "and it is still not a batch's, however it reads",
    );

    tokio::time::sleep(std::time::Duration::from_millis(10)).await;

    let batch = ask(&pool, id, &answering_the_comments(), Ask::Blocking)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        last_proposal(&pool, id).await.unwrap(),
        Some(batch.id),
        "what a batch asks after the settle is the newest proposal",
    );
    assert_eq!(
        last_batch_proposal(&pool, id).await.unwrap(),
        Some(batch.id),
        "and it is the batch's own",
    );
}

/// A batch nobody answered is one the comments are forgotten for, so that the
/// session a retry starts is about the same words rather than about nothing.
#[tokio::test]
async fn comments_forgotten_are_dispatched_for_again() {
    let (dir, pool) = fresh_pool().await;
    let id = wrapping(&pool).await;

    record_addressed_comments(&pool, id, &["IC_1".to_owned(), "IC_2".to_owned()])
        .await
        .unwrap();

    forget_addressed_comments(&pool, id, &["IC_1".to_owned(), "IC_2".to_owned()])
        .await
        .unwrap();

    pool.close().await;

    // A second reader over the same file, as a restarted server has: forgetting
    // is a fact about the database rather than about the process.
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    assert_eq!(
        addressed_comments(&pool, id).await.unwrap(),
        Vec::<String>::new(),
        "nothing has been dispatched about, so the next poll dispatches again",
    );
}

/// *Settled once and stays settled* is a rule about one wrap. A review that
/// split its findings out into a backlog sends the work back to be built, and
/// what comes back is a branch nobody has read — so leaving Wrapping takes the
/// review's settle with it.
///
/// The checks it leaves alone: they are asked of GitHub on every poll, so a
/// second wrap settles them from the answers it gets rather than from what the
/// first one wrote down.
#[tokio::test]
async fn leaving_wrapping_puts_the_review_back_to_waiting_and_nothing_else() {
    let (_dir, pool) = fresh_pool().await;
    let id = wrapping(&pool).await;

    settle_wrap_up(&pool, id, WaitingOn::Review).await.unwrap();
    settle_wrap_up(&pool, id, WaitingOn::Checks).await.unwrap();

    implement_again(&pool, id).await.unwrap();

    assert_eq!(
        wrap_up_settled(&pool, id).await.unwrap(),
        vec![WaitingOn::Checks],
        "the review is waiting again and the checks are left where they were",
    );
}

/// And the first wrap's proposals are not the second's. The review Set it asked
/// was answered and acted on, and a second wrap that counted it would be one
/// that never reviewed anything: what it found asking was the wrap before.
#[tokio::test]
async fn the_first_wraps_review_is_not_the_second_wraps() {
    let (_dir, pool) = fresh_pool().await;
    let id = wrapping(&pool).await;

    let asked = ask(&pool, id, &reviewing(), Ask::Blocking)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(last_proposal(&pool, id).await.unwrap(), Some(asked.id));

    implement_again(&pool, id).await.unwrap();
    record_pull_request(
        &pool,
        id,
        &verkstead_store::PullRequest {
            number: 41,
            title: "Rate limiting".to_owned(),
            url: "https://github.com/tobico/verkstead/pull/41".to_owned(),
        },
    )
    .await
    .unwrap();

    assert_eq!(
        last_proposal(&pool, id).await.unwrap(),
        None,
        "the second wrap has been reviewed by nobody, so a review runs over the branch \
         and nothing of the first wrap's is what a batch session is measured against",
    );

    let again = ask(&pool, id, &reviewing(), Ask::Blocking)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        last_proposal(&pool, id).await.unwrap(),
        Some(again.id),
        "the second wrap's own review is the one it finds",
    );
}
