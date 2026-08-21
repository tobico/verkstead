//! How a grilling ends and how the work it settled gets built: the wrap-up
//! proposal that moves a Conversation out of Grilling, and the direction the
//! human chooses once it is there.
//!
//! The move is driven by a Response arriving rather than by anything anyone
//! pressed, which is the whole shape of the thing: a grilling ends by the
//! agent's own closing move, and answering that Set is what accepts it. So what
//! these ask is what a Conversation's state and its Timeline say afterwards.

use std::path::Path;

use sqlx::SqlitePool;
use verkstead_schema::{Direction, Proposal, Question, QuestionOption, QuestionSet, Response};
use verkstead_store::{
    Directed, Directing, Event, Implementing, Lifecycle, Proposed, Settlements, Submission, ask,
    choose_direction, load_conversation, move_to_direction, open_database, record_handoff,
    register_repo, save_brief, start_conversation, start_grilling, start_implementing,
    submit_response, timeline,
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
            accepted_by: "Q14.1".to_owned(),
            rationale: "Six changes, each independently testable.".to_owned(),
        }),
        project: None,
        branch: None,
        diff: None,
    }
}

/// And the same Set without one, which is every ordinary round of grilling.
fn ordinary() -> QuestionSet {
    QuestionSet {
        proposal: None,
        title: "Where the request counter lives".to_owned(),
        ..proposing(Direction::Inline)
    }
}

/// Answer a Set the way both halves of the server do, through the one path a
/// Response takes.
async fn answer(pool: &SqlitePool, set_id: i64) -> Submission {
    answered(pool, set_id, &accepting()).await
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

/// Picking the Option the proposal names, which is the one thing that accepts
/// one — `accepted_by: Q14.1`.
fn accepting() -> Response {
    Response::from_yaml("answers:\n  - label: Q14\n    selected: 1\n").unwrap()
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

/// And the directions it says were chosen, in order.
async fn directions(pool: &SqlitePool, id: i64) -> Vec<Direction> {
    timeline(pool, id)
        .await
        .unwrap()
        .iter()
        .filter_map(|event| match event.event {
            Event::Directed(direction) => Some(direction),
            _ => None,
        })
        .collect()
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
        Lifecycle::Direction,
        "answering the closing proposal is what ends a grilling",
    );
    assert_eq!(
        moves(&pool, id).await,
        [Lifecycle::Grilling, Lifecycle::Direction],
        "and the move is on the Timeline, where everything that happens lands",
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
        proposed(&pool, ordinary.id, &accepting()).await,
        None,
        "a Set carrying no proposal settled none, and says so by having nothing to say",
    );

    let proposing = ask(&pool, id, &proposing(Direction::Inline))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        proposed(&pool, proposing.id, &accepting()).await,
        Some(Proposed::Accepted(Directing::Moved)),
        "the store hands back what happened rather than logging it",
    );
}

/// Every way of answering a proposal that is not picking the Option it named.
///
/// One test over the four, because they are one rule: only `accepted_by` ends a
/// grilling, and everything else is the human disagreeing. Each gets its own
/// Conversation, so none of them is standing on what the last one left behind.
#[tokio::test]
async fn any_other_answer_leaves_the_conversation_grilling() {
    for (how, response) in [
        (
            "another Option",
            "answers:\n  - label: Q14\n    selected: 2\n",
        ),
        (
            "an answer in their own words instead of an Option",
            "answers:\n  - label: Q14\n    free_text: Not until the migration is settled.\n",
        ),
        (
            "the question left open",
            "answers:\n  - label: Q14\n    unanswered: true\ncomment: |\n  Say more about the migration first.\n",
        ),
        (
            "an Option the proposal did not name, with words beside it",
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

/// Picking the named Option *and* saying something about it is still acceptance.
///
/// The Guide's own reading: free text beside a selected Option is the rationale
/// or a qualification, and free text instead of one is an answer of the human's
/// own. Only the second of those is a refusal — a human who picked "yes, go
/// ahead" and added a note meant yes.
#[tokio::test]
async fn accepting_with_something_to_add_is_still_accepting() {
    let (_dir, pool) = fresh_pool().await;
    let id = grilling(&pool).await;
    let created = ask(&pool, id, &proposing(Direction::TaskList))
        .await
        .unwrap()
        .unwrap();

    let qualified = Response::from_yaml(
        "answers:\n  - label: Q14\n    selected: 1\n    free_text: Keep the config key as it is.\n",
    )
    .unwrap();

    assert_eq!(
        proposed(&pool, created.id, &qualified).await,
        Some(Proposed::Accepted(Directing::Moved)),
    );
    assert_eq!(state(&pool, id).await, Lifecycle::Direction);
}

/// A grilling sent back can propose again, and that one can be accepted.
///
/// The whole way back, end to end: the point of a refusal is that the work can
/// still reach Direction afterwards, and by the same door.
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
        proposed(&pool, second.id, &accepting()).await,
        Some(Proposed::Accepted(Directing::Moved)),
    );

    assert_eq!(state(&pool, id).await, Lifecycle::Direction);
    assert_eq!(
        moves(&pool, id).await,
        [Lifecycle::Grilling, Lifecycle::Direction],
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
        proposed(&pool, second.id, &accepting()).await,
        Some(Proposed::Accepted(Directing::NotGrilling)),
        "the first acceptance moved it, and the second has nothing left to move",
    );
    assert_eq!(
        moves(&pool, id).await,
        [Lifecycle::Grilling, Lifecycle::Direction],
        "so the Timeline does not say it moved twice",
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
        move_to_direction(&pool, id).await.unwrap(),
        Directing::NotGrilling,
    );
    assert_eq!(state(&pool, id).await, Lifecycle::Draft);
}

#[tokio::test]
async fn there_is_no_conversation_to_move() {
    let (_dir, pool) = fresh_pool().await;

    assert_eq!(
        move_to_direction(&pool, 404).await.unwrap(),
        Directing::NoSuchConversation,
    );
}

#[tokio::test]
async fn choosing_a_direction_records_it_and_lands_it_on_the_timeline() {
    let (_dir, pool) = fresh_pool().await;
    let id = grilling(&pool).await;
    move_to_direction(&pool, id).await.unwrap();

    assert_eq!(
        choose_direction(&pool, id, Direction::TaskList)
            .await
            .unwrap(),
        Directed::Chosen,
    );

    let conversation = load_conversation(&pool, id).await.unwrap().unwrap();
    assert_eq!(conversation.direction, Some(Direction::TaskList));
    assert_eq!(
        conversation.state,
        Lifecycle::Direction,
        "what was settled is how the work gets built, not that it has started",
    );
    assert_eq!(directions(&pool, id).await, [Direction::TaskList]);
}

#[tokio::test]
async fn choosing_again_replaces_the_choice_and_keeps_both_on_the_record() {
    let (_dir, pool) = fresh_pool().await;
    let id = grilling(&pool).await;
    move_to_direction(&pool, id).await.unwrap();

    choose_direction(&pool, id, Direction::Inline)
        .await
        .unwrap();
    choose_direction(&pool, id, Direction::TaskList)
        .await
        .unwrap();

    let conversation = load_conversation(&pool, id).await.unwrap().unwrap();
    assert_eq!(
        conversation.direction,
        Some(Direction::TaskList),
        "the latest choice is the one in force",
    );
    assert_eq!(
        directions(&pool, id).await,
        [Direction::Inline, Direction::TaskList],
        "and the Timeline keeps what it was changed from — nothing leaves a Timeline",
    );
}

#[tokio::test]
async fn a_conversation_that_is_not_choosing_has_nothing_to_choose() {
    let (_dir, pool) = fresh_pool().await;
    let id = grilling(&pool).await;

    assert_eq!(
        choose_direction(&pool, id, Direction::Inline)
            .await
            .unwrap(),
        Directed::NotChoosing,
        "a grilling that has not proposed wrapping up is not asking this yet",
    );

    let conversation = load_conversation(&pool, id).await.unwrap().unwrap();
    assert_eq!(conversation.direction, None);
    assert_eq!(directions(&pool, id).await, []);
}

#[tokio::test]
async fn there_is_no_conversation_to_direct() {
    let (_dir, pool) = fresh_pool().await;

    assert_eq!(
        choose_direction(&pool, 404, Direction::Inline)
            .await
            .unwrap(),
        Directed::NoSuchConversation,
    );
}

#[tokio::test]
async fn starting_the_implementation_moves_the_conversation_and_says_so() {
    let (_dir, pool) = fresh_pool().await;
    let id = grilling(&pool).await;
    move_to_direction(&pool, id).await.unwrap();
    choose_direction(&pool, id, Direction::Inline)
        .await
        .unwrap();

    assert_eq!(
        start_implementing(&pool, id).await.unwrap(),
        Implementing::Started
    );

    assert_eq!(state(&pool, id).await, Lifecycle::Implementing);
    assert_eq!(
        moves(&pool, id).await,
        [
            Lifecycle::Grilling,
            Lifecycle::Direction,
            Lifecycle::Implementing
        ],
        "the choice is an Event of its own and the work starting is a move",
    );
}

/// Choosing is what starts the work, so a Conversation that is not choosing has
/// nothing to start — including one already implementing, which is a second
/// press.
#[tokio::test]
async fn there_is_nothing_to_implement_where_nothing_is_being_chosen() {
    let (_dir, pool) = fresh_pool().await;
    let id = grilling(&pool).await;

    assert_eq!(
        start_implementing(&pool, id).await.unwrap(),
        Implementing::NotChoosing,
    );
    assert_eq!(state(&pool, id).await, Lifecycle::Grilling);

    move_to_direction(&pool, id).await.unwrap();
    start_implementing(&pool, id).await.unwrap();

    assert_eq!(
        start_implementing(&pool, id).await.unwrap(),
        Implementing::NotChoosing,
    );
    assert_eq!(
        moves(&pool, id).await,
        [
            Lifecycle::Grilling,
            Lifecycle::Direction,
            Lifecycle::Implementing
        ],
        "the second press records nothing",
    );

    assert_eq!(
        start_implementing(&pool, 404).await.unwrap(),
        Implementing::NoSuchConversation,
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
        move_to_direction(&pool, id).await.unwrap();
        choose_direction(&pool, id, Direction::TaskList)
            .await
            .unwrap();
        pool.close().await;
        id
    };

    let pool = open_database(&database).await.unwrap();
    let conversation = load_conversation(&pool, id).await.unwrap().unwrap();

    assert_eq!(conversation.state, Lifecycle::Direction);
    assert_eq!(conversation.direction, Some(Direction::TaskList));
    assert_eq!(directions(&pool, id).await, [Direction::TaskList]);
}
