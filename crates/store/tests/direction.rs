//! How a grilling ends and how the work it settled gets built: the wrap-up
//! proposal, and the direction the human picks on it.
//!
//! The move is driven by a Response arriving rather than by anything anyone
//! pressed, which is the whole shape of the thing: a grilling ends by the
//! agent's own closing move, and picking a direction on that Set is what accepts
//! it. There is no rung between the two — a pick takes a Conversation from
//! Grilling straight to Implementing — so what these ask is what a
//! Conversation's state and its Timeline say afterwards.

use std::path::Path;

use sqlx::SqlitePool;
use verkstead_schema::{Direction, Proposal, Question, QuestionOption, QuestionSet, Response};
use verkstead_store::{
    Directing, Event, Lifecycle, Proposed, Settlements, Submission, ask, load_conversation,
    open_database, pick_direction, record_handoff, register_repo, save_brief, start_conversation,
    start_grilling, submit_response, timeline,
};

/// A pool over a fresh database, plus the directory keeping it alive.
async fn fresh_pool() -> (tempfile::TempDir, SqlitePool) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();
    (dir, pool)
}

/// The channel a settling is announced on. Nothing here listens: what these ask
/// is what the store did, and the announcement is the server's business.
fn settlements() -> Settlements {
    Settlements::new(4)
}

/// A Conversation that is grilling, which is where a proposal can reach one.
///
/// Started for real rather than moved by hand: `start_grilling` is what records
/// the base commit and the worktree beside the state, and a Conversation moved
/// straight into Grilling would be one nothing else in the store agrees about.
async fn grilling(pool: &SqlitePool) -> i64 {
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

    id
}

/// A Set asking one answerable question, with a wrap-up proposal on it.
fn proposing(direction: Direction) -> QuestionSet {
    QuestionSet {
        title: "Ready to build the rate limiter".to_owned(),
        preface: None,
        questions: vec![Question {
            label: "Q14".to_owned(),
            text: "Ready to build it this way?".to_owned(),
            columns: Vec::new(),
            options: vec![
                QuestionOption {
                    n: 1,
                    text: "Yes, go ahead".to_owned(),
                    recommended: true,
                    cells: Vec::new(),
                },
                QuestionOption {
                    n: 2,
                    text: "Not yet — more to work through".to_owned(),
                    recommended: false,
                    cells: Vec::new(),
                },
            ],
            subquestions: Vec::new(),
        }],
        postscript: None,
        proposal: Some(Proposal {
            direction,
            rationale: "Six changes, each independently testable.".to_owned(),
        }),
        review: None,
        project: None,
        branch: None,
        diff: None,
    }
}

/// And the same Set without one, which is every ordinary round of grilling.
fn ordinary() -> QuestionSet {
    QuestionSet {
        proposal: None,
        review: None,
        title: "Where the request counter lives".to_owned(),
        ..proposing(Direction::Inline)
    }
}

/// Answer a Set the way both halves of the server do, through the one path a
/// Response takes.
async fn answer(pool: &SqlitePool, set_id: i64) -> Submission {
    answered(pool, set_id, &accepting(Direction::TaskList)).await
}

/// The same, with a Response of the test's own choosing.
async fn answered(pool: &SqlitePool, set_id: i64, response: &Response) -> Submission {
    submit_response(pool, &settlements(), set_id, response)
        .await
        .unwrap()
}

/// What became of the proposal on the Set this Response answered.
async fn proposed(pool: &SqlitePool, set_id: i64, response: &Response) -> Option<Proposed> {
    let Submission::Accepted(taken) = answered(pool, set_id, response).await else {
        panic!("the Set was there and the Response resolves it");
    };

    taken.proposed
}

/// Picking a direction on the chooser, which is the one thing that accepts a
/// proposal — and which direction is picked is the human's, never the agent's.
fn accepting(direction: Direction) -> Response {
    Response {
        direction: Some(direction),
        ..Response::from_yaml("answers:\n  - label: Q14\n    unanswered: true\n").unwrap()
    }
}

/// What state a Conversation is in.
async fn state(pool: &SqlitePool, id: i64) -> Lifecycle {
    load_conversation(pool, id)
        .await
        .unwrap()
        .expect("the Conversation is there")
        .state
}

/// The states a Conversation's Timeline says it has moved through, in order.
async fn moves(pool: &SqlitePool, id: i64) -> Vec<Lifecycle> {
    timeline(pool, id)
        .await
        .unwrap()
        .iter()
        .filter_map(|event| match event.event {
            Event::Moved(state) => Some(state),
            _ => None,
        })
        .collect()
}

/// And the handoffs a grilling has handed over, in order.
async fn handoffs(pool: &SqlitePool, id: i64) -> Vec<String> {
    timeline(pool, id)
        .await
        .unwrap()
        .into_iter()
        .filter_map(|event| match event.event {
            Event::Handoff(markdown) => Some(markdown),
            _ => None,
        })
        .collect()
}

/// The latest pick on a Conversation — what it says it is being built as.
async fn direction(pool: &SqlitePool, id: i64) -> Option<Direction> {
    load_conversation(pool, id)
        .await
        .unwrap()
        .expect("the Conversation is there")
        .direction
}

#[tokio::test]
async fn answering_a_set_that_carries_a_proposal_ends_the_grilling() {
    let (_dir, pool) = fresh_pool().await;
    let id = grilling(&pool).await;

    let created = ask(&pool, id, &proposing(Direction::TaskList))
        .await
        .unwrap()
        .unwrap();

    assert!(matches!(
        answer(&pool, created.id).await,
        Submission::Accepted(_)
    ));

    assert_eq!(
        state(&pool, id).await,
        Lifecycle::Implementing,
        "picking a direction on the closing proposal is what ends a grilling",
    );
    assert_eq!(
        moves(&pool, id).await,
        [Lifecycle::Grilling, Lifecycle::Implementing],
        "and the one move is on the Timeline, with no rung in between",
    );
    assert_eq!(
        direction(&pool, id).await,
        Some(Direction::TaskList),
        "and what was picked is the Conversation's latest pick",
    );
}

#[tokio::test]
async fn answering_an_ordinary_grilling_set_leaves_it_grilling() {
    let (_dir, pool) = fresh_pool().await;
    let id = grilling(&pool).await;

    let created = ask(&pool, id, &ordinary()).await.unwrap().unwrap();
    answer(&pool, created.id).await;

    assert_eq!(
        state(&pool, id).await,
        Lifecycle::Grilling,
        "an ordinary round of grilling is not the end of one",
    );
    assert_eq!(
        moves(&pool, id).await,
        [Lifecycle::Grilling],
        "and nothing moved, so nothing is on the Timeline saying it did",
    );
}

#[tokio::test]
async fn the_acceptance_says_what_the_proposal_moved() {
    let (_dir, pool) = fresh_pool().await;
    let id = grilling(&pool).await;

    let ordinary = ask(&pool, id, &ordinary()).await.unwrap().unwrap();
    assert_eq!(
        proposed(
            &pool,
            ordinary.id,
            &Response::from_yaml("answers:\n  - label: Q14\n    selected: 1\n").unwrap()
        )
        .await,
        None,
        "a Set carrying no proposal settled none, and says so by having nothing to say",
    );

    let proposing = ask(&pool, id, &proposing(Direction::Inline))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        proposed(&pool, proposing.id, &accepting(Direction::TaskList)).await,
        Some(Proposed::Accepted {
            direction: Direction::TaskList,
            directing: Directing::Moved,
        }),
        "the store hands back what happened, and which direction was picked — \
         the human's rather than the recommended one",
    );
}

/// Every way of answering a proposal that is not picking a direction on it.
///
/// One test over the four, because they are one rule: only a pick ends a
/// grilling, and everything else is the human disagreeing. Each gets its own
/// Conversation, so none of them is standing on what the last one left behind.
#[tokio::test]
async fn any_answer_without_a_pick_leaves_the_conversation_grilling() {
    for (how, response) in [
        (
            "an Option beside the chooser",
            "answers:\n  - label: Q14\n    selected: 2\n",
        ),
        (
            "an answer in their own words",
            "answers:\n  - label: Q14\n    free_text: Not until the migration is settled.\n",
        ),
        (
            "the question left open",
            "answers:\n  - label: Q14\n    unanswered: true\ncomment: |\n  Say more about the migration first.\n",
        ),
        (
            "an Option with words beside it",
            "answers:\n  - label: Q14\n    selected: 2\n    free_text: The counter still worries me.\n",
        ),
    ] {
        let (_dir, pool) = fresh_pool().await;
        let id = grilling(&pool).await;
        let created = ask(&pool, id, &proposing(Direction::TaskList))
            .await
            .unwrap()
            .unwrap();

        let response = Response::from_yaml(response).unwrap();

        assert_eq!(
            proposed(&pool, created.id, &response).await,
            Some(Proposed::SentBack),
            "answering with {how} is how a human disagrees",
        );
        assert_eq!(
            state(&pool, id).await,
            Lifecycle::Grilling,
            "so the grilling carries on after {how}",
        );
        assert_eq!(
            moves(&pool, id).await,
            [Lifecycle::Grilling],
            "and nothing is on the Timeline saying it moved, after {how}",
        );
    }
}

/// Picking a direction *and* saying something beside it is still acceptance.
///
/// Words beside a pick are the qualification the Guide says they are — the agent
/// reads them and the work goes ahead — where words instead of a pick are the
/// human's own answer, which is the proposal sent back. A human who picked a
/// direction and added a note meant the direction.
#[tokio::test]
async fn accepting_with_something_to_add_is_still_accepting() {
    let (_dir, pool) = fresh_pool().await;
    let id = grilling(&pool).await;
    let created = ask(&pool, id, &proposing(Direction::TaskList))
        .await
        .unwrap()
        .unwrap();

    let qualified = Response {
        direction: Some(Direction::TaskList),
        ..Response::from_yaml(
            "answers:\n  - label: Q14\n    free_text: Keep the config key as it is.\n",
        )
        .unwrap()
    };

    assert_eq!(
        proposed(&pool, created.id, &qualified).await,
        Some(Proposed::Accepted {
            direction: Direction::TaskList,
            directing: Directing::Moved,
        }),
    );
    assert_eq!(state(&pool, id).await, Lifecycle::Implementing);
}

/// A grilling sent back can propose again, and that one can be accepted.
///
/// The whole way back, end to end: the point of a refusal is that the work can
/// still be built afterwards, and by the same door.
#[tokio::test]
async fn a_proposal_put_again_after_a_refusal_can_be_accepted() {
    let (_dir, pool) = fresh_pool().await;
    let id = grilling(&pool).await;

    let first = ask(&pool, id, &proposing(Direction::Inline))
        .await
        .unwrap()
        .unwrap();
    let refusing = Response::from_yaml("answers:\n  - label: Q14\n    selected: 2\n").unwrap();
    assert_eq!(
        proposed(&pool, first.id, &refusing).await,
        Some(Proposed::SentBack),
    );

    // The agent read the Response, went back down the branch, and proposed
    // again — a second Set, because the first is answered and stays answered.
    let second = ask(&pool, id, &proposing(Direction::TaskList))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        proposed(&pool, second.id, &accepting(Direction::TaskList)).await,
        Some(Proposed::Accepted {
            direction: Direction::TaskList,
            directing: Directing::Moved,
        }),
    );

    assert_eq!(state(&pool, id).await, Lifecycle::Implementing);
    assert_eq!(
        moves(&pool, id).await,
        [Lifecycle::Grilling, Lifecycle::Implementing],
        "the refusal moved nothing, so the Timeline says it got here once",
    );
}

#[tokio::test]
async fn a_second_proposal_answered_finds_the_move_already_made() {
    let (_dir, pool) = fresh_pool().await;
    let id = grilling(&pool).await;

    let first = ask(&pool, id, &proposing(Direction::Inline))
        .await
        .unwrap()
        .unwrap();
    let second = ask(&pool, id, &proposing(Direction::TaskList))
        .await
        .unwrap()
        .unwrap();

    answer(&pool, first.id).await;

    assert_eq!(
        proposed(&pool, second.id, &accepting(Direction::Roadmap)).await,
        Some(Proposed::Accepted {
            direction: Direction::Roadmap,
            directing: Directing::NotGrilling,
        }),
        "the first acceptance moved it, and the second has nothing left to move",
    );
    assert_eq!(
        moves(&pool, id).await,
        [Lifecycle::Grilling, Lifecycle::Implementing],
        "so the Timeline does not say it moved twice",
    );
    assert_eq!(
        direction(&pool, id).await,
        Some(Direction::TaskList),
        "and the pick that moved it is the one that stands",
    );
}

#[tokio::test]
async fn a_conversation_that_is_not_grilling_has_no_grilling_to_end() {
    let (_dir, pool) = fresh_pool().await;
    let repo = register_repo(&pool, Path::new("/srv/verkstead"), "verkstead", "main")
        .await
        .unwrap()
        .unwrap();
    let id = start_conversation(&pool, repo.id, "still-drafting")
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        pick_direction(&pool, id, Direction::Inline).await.unwrap(),
        Directing::NotGrilling,
    );
    assert_eq!(state(&pool, id).await, Lifecycle::Draft);
    assert_eq!(direction(&pool, id).await, None);
}

#[tokio::test]
async fn there_is_no_conversation_to_move() {
    let (_dir, pool) = fresh_pool().await;

    assert_eq!(
        pick_direction(&pool, 404, Direction::Inline).await.unwrap(),
        Directing::NoSuchConversation,
    );
}

/// A second pick on a Conversation already being built changes nothing.
///
/// Not a rule about picking twice so much as the one rule stated once: only a
/// grilling can be ended, and this one is over. What the human picked stands,
/// and the work carries on being built as it.
#[tokio::test]
async fn a_conversation_that_is_already_being_built_takes_no_second_pick() {
    let (_dir, pool) = fresh_pool().await;
    let id = grilling(&pool).await;

    pick_direction(&pool, id, Direction::TaskList)
        .await
        .unwrap();

    assert_eq!(
        pick_direction(&pool, id, Direction::Inline).await.unwrap(),
        Directing::NotGrilling,
    );
    assert_eq!(
        direction(&pool, id).await,
        Some(Direction::TaskList),
        "the pick that ended the grilling is the one in force",
    );
    assert_eq!(
        moves(&pool, id).await,
        [Lifecycle::Grilling, Lifecycle::Implementing],
        "and the second pick records nothing",
    );
}

#[tokio::test]
async fn a_handoff_lands_on_the_timeline_as_the_document_it_is() {
    let (_dir, pool) = fresh_pool().await;
    let id = grilling(&pool).await;

    assert!(
        record_handoff(&pool, id, "# What we settled\n")
            .await
            .unwrap()
    );

    assert_eq!(handoffs(&pool, id).await, ["# What we settled\n"]);

    // A reopened round grills again, and its handoff is a second Event rather
    // than a rewrite of the first: nothing leaves a Timeline.
    record_handoff(&pool, id, "# What we settled, again\n")
        .await
        .unwrap();

    assert_eq!(
        handoffs(&pool, id).await,
        ["# What we settled\n", "# What we settled, again\n"],
    );
}

#[tokio::test]
async fn there_is_no_conversation_to_hand_over_from() {
    let (_dir, pool) = fresh_pool().await;

    assert!(
        !record_handoff(&pool, 404, "# What we settled\n")
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn a_direction_survives_a_restart() {
    let dir = tempfile::tempdir().unwrap();
    let database = dir.path().join("verkstead.db");

    let id = {
        let pool = open_database(&database).await.unwrap();
        let id = grilling(&pool).await;
        pick_direction(&pool, id, Direction::TaskList)
            .await
            .unwrap();
        pool.close().await;
        id
    };

    let pool = open_database(&database).await.unwrap();
    let conversation = load_conversation(&pool, id).await.unwrap().unwrap();

    assert_eq!(conversation.state, Lifecycle::Implementing);
    assert_eq!(conversation.direction, Some(Direction::TaskList));
}

/// A database written before the Direction state left the ladder, opened by this
/// Verkstead.
///
/// The state is gone and so are the two Events about it, and a Conversation
/// found sitting in it comes out Aborted — its grilling session is over and the
/// chooser it was waiting on does not exist any more. Written by hand, because
/// this code cannot write that database any more: that is the whole point of it.
#[tokio::test]
async fn a_conversation_caught_choosing_a_direction_comes_out_aborted() {
    let dir = tempfile::tempdir().unwrap();
    let database = dir.path().join("verkstead.db");

    let id = {
        let pool = open_database(&database).await.unwrap();
        let id = grilling(&pool).await;

        // Back to where the old ladder left it: sitting in Direction, with the
        // move that put it there and the choice it had made on its Timeline.
        for statement in [
            "UPDATE conversations SET state = 'direction'",
            "INSERT INTO timeline_events (conversation_id, at, kind, body)
             VALUES (1, '2026-08-20T09:00:00.000Z', 'moved', 'direction')",
            "INSERT INTO timeline_events (conversation_id, at, kind, body)
             VALUES (1, '2026-08-20T09:01:00.000Z', 'directed', 'task-list')",
        ] {
            sqlx::query(statement).execute(&pool).await.unwrap();
        }

        pool.close().await;
        id
    };

    let pool = open_database(&database).await.unwrap();
    let conversation = load_conversation(&pool, id).await.unwrap().unwrap();

    assert_eq!(
        conversation.state,
        Lifecycle::Aborted,
        "there is no session to receive anything and no chooser left to press",
    );
    assert_eq!(
        conversation.worktree, None,
        "and an aborted Conversation has no worktree, as every other abort leaves none",
    );
    assert_eq!(
        moves(&pool, id).await,
        [Lifecycle::Grilling, Lifecycle::Aborted],
        "the retired move is off the Timeline and the abort is on it",
    );
}
