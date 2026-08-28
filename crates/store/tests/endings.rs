//! The Nothing-else mark in the store: the human saying, on a follow-up's Set,
//! that there is nothing else to follow up.
//!
//! The mark rides the submitted Response and is kept beside the stored one, and
//! the second half of that is what these are mostly about: the Response a
//! waiting agent is handed has to be the same bytes whether the human ticked the
//! option or not, because how a follow-up ends is Verkstead's business and the
//! session is not allowed a way to act on it.
//!
//! Two readings of it, and the second is what the mark is for. Per Set, it is
//! written where the Response landed and absent where it did not. Per
//! Conversation, it is the rule about a whole follow-up: the newest round the
//! human answered, inside the window this follow-up's own steer opened — so a
//! round asked after a marked one puts the follow-up back to running, and a mark
//! left by the follow-up before this one ends nothing.
//!
//! And the landing that reading ends in: back to Wrapping over the pull request
//! the follow-up was opened about, with the checks put back to waiting where it
//! pushed.

use std::path::Path;

use sqlx::SqlitePool;
use verkstead_schema::{Answer, Question, QuestionSet, Response};
use verkstead_store::{
    Ask, Ending, Lifecycle, PullRequest, Settlements, Steer, Steering, Submission, WAITED_ON,
    WaitingOn, ask, ended_on, follow_up_over, load_conversation, load_response, lock_set,
    nothing_else, open_database, record_another_pull_request, register_repo, settle_wrap_up,
    start_conversation, steer_conversation, submit_response, wrap_up_settled,
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
/// is what the store wrote.
fn settlements() -> Settlements {
    Settlements::new(4)
}

/// The Conversation every Set here is asked from.
///
/// Left drafting, because the mark is not the state's business: what puts the
/// option on the page is where the Conversation stands, and that is decided
/// where the page is drawn. The store takes the mark it is handed.
async fn conversation(pool: &SqlitePool) -> i64 {
    let repo = register_repo(pool, Path::new("/srv/verkstead"), "verkstead", "main")
        .await
        .unwrap()
        .expect("nothing is registered at that path yet");

    start_conversation(pool, repo.id, "follow-ups")
        .await
        .unwrap()
        .expect("the Repo was just registered")
}

/// A round of a follow-up as a Set: one question, closed with a Postscript.
fn round(title: &str) -> QuestionSet {
    QuestionSet {
        title: title.to_owned(),
        preface: None,
        questions: vec![Question {
            label: "Q1".to_owned(),
            text: "Should the header say when the window resets?".to_owned(),
            columns: Vec::new(),
            options: Vec::new(),
            subquestions: Vec::new(),
        }],
        postscript: Some("That is the header added and pushed.\n".to_owned()),
        proposal: None,
        project: Some("verkstead".to_owned()),
        branch: Some("rate-limiting".to_owned()),
        diff: None,
        diffs: Vec::new(),
    }
}

/// Put a round on the Timeline and hand back its id.
async fn asked(pool: &SqlitePool, conversation: i64, title: &str) -> i64 {
    ask(pool, conversation, &round(title), Ask::Blocking)
        .await
        .unwrap()
        .expect("the Conversation is there to ask from")
        .id
}

/// An ordinary Response to [`round`]: the question answered, and a word about
/// the Set as a whole.
fn answering() -> Response {
    Response {
        answers: vec![Answer {
            label: "Q1".to_owned(),
            selected: None,
            free_text: Some("Yes, seconds rather than a timestamp.".to_owned()),
            unanswered: false,
        }],
        comment: Some("Reads well otherwise.".to_owned()),
        direction: None,
        nothing_else: false,
    }
}

/// The same Response with the option ticked, which is the human saying the
/// follow-up is over.
fn ending() -> Response {
    Response {
        nothing_else: true,
        ..answering()
    }
}

/// What the store handed back to whoever was waiting on this Set.
async fn handed_back(pool: &SqlitePool, set_id: i64) -> Response {
    load_response(pool, set_id)
        .await
        .unwrap()
        .expect("the Set has been answered")
        .response
}

/// And the row it was handed back out of, byte for byte.
async fn stored_body(pool: &SqlitePool, set_id: i64) -> String {
    let (body,): (String,) = sqlx::query_as("SELECT body FROM responses WHERE set_id = ?")
        .bind(set_id)
        .fetch_one(pool)
        .await
        .unwrap();

    body
}

#[tokio::test]
async fn a_response_carrying_the_mark_is_handed_to_the_agent_without_it() {
    let (_dir, pool) = fresh_pool().await;
    let conversation = conversation(&pool).await;

    let ordinary = asked(&pool, conversation, "the round before").await;
    let last = asked(&pool, conversation, "the last round").await;

    submit_response(&pool, &settlements(), ordinary, &answering())
        .await
        .unwrap();
    submit_response(&pool, &settlements(), last, &ending())
        .await
        .unwrap();

    assert_eq!(
        stored_body(&pool, last).await,
        stored_body(&pool, ordinary).await,
        "the mark is the system's, so the two Responses are stored as the same \
         bytes — which is what a waiting session is handed"
    );
    assert_eq!(
        handed_back(&pool, last).await,
        answering(),
        "and what those bytes say is the Response without the mark on it"
    );
    assert!(
        !stored_body(&pool, last).await.contains("nothing_else"),
        "there is no field there for an agent to find, absent or otherwise"
    );
}

#[tokio::test]
async fn the_mark_is_kept_beside_the_response_and_reads_back_per_set() {
    let (_dir, pool) = fresh_pool().await;
    let conversation = conversation(&pool).await;

    let ordinary = asked(&pool, conversation, "the round before").await;
    let last = asked(&pool, conversation, "the last round").await;

    submit_response(&pool, &settlements(), ordinary, &answering())
        .await
        .unwrap();
    submit_response(&pool, &settlements(), last, &ending())
        .await
        .unwrap();

    assert!(
        ended_on(&pool, last).await.unwrap(),
        "the Set whose Response ticked the option carries the mark"
    );
    assert!(
        !ended_on(&pool, ordinary).await.unwrap(),
        "and the one whose Response did not, does not"
    );
}

#[tokio::test]
async fn a_set_that_has_not_been_answered_carries_no_mark() {
    let (_dir, pool) = fresh_pool().await;
    let conversation = conversation(&pool).await;

    let waiting = asked(&pool, conversation, "still out with the human").await;
    let locked = asked(&pool, conversation, "closed unanswered").await;
    lock_set(&pool, &settlements(), locked).await.unwrap();

    assert!(!ended_on(&pool, waiting).await.unwrap());
    assert!(
        !ended_on(&pool, locked).await.unwrap(),
        "a Set nobody answered says nothing about whether the follow-up is over"
    );
}

#[tokio::test]
async fn a_second_response_marks_nothing_because_it_is_not_stored() {
    let (_dir, pool) = fresh_pool().await;
    let conversation = conversation(&pool).await;

    let set_id = asked(&pool, conversation, "the round").await;

    submit_response(&pool, &settlements(), set_id, &answering())
        .await
        .unwrap();

    assert!(
        matches!(
            submit_response(&pool, &settlements(), set_id, &ending())
                .await
                .unwrap(),
            Submission::AlreadyAnswered
        ),
        "a Set is answered once, and the first Response stands"
    );
    assert!(
        !ended_on(&pool, set_id).await.unwrap(),
        "so the refused Response's mark is refused with it"
    );
}

/// Steer the Conversation into Follow-up, which is what opens the window the
/// whole-follow-up read counts inside.
///
/// The real move rather than a state written over the top of one: what says
/// where this follow-up began is the Moved Event on the Timeline, and only the
/// steer writes one.
async fn following_up(pool: &SqlitePool, conversation: i64) {
    assert_eq!(
        steer_conversation(
            pool,
            conversation,
            Steer {
                target: Lifecycle::FollowUp,
                pairing: None,
                brief: None,
                instruction: Some("Does it count the 429s it sends?\n"),
                direction: None,
                worktree: None,
                base_commit: None,
                companions: &[],
                opened: &[],
                checkouts: &[],
                said: None,
            },
        )
        .await
        .unwrap(),
        Steering::Steered,
    );
}

#[tokio::test]
async fn the_follow_up_is_over_when_the_latest_round_answered_carries_the_mark() {
    let (_dir, pool) = fresh_pool().await;
    let conversation = conversation(&pool).await;

    following_up(&pool, conversation).await;

    assert!(
        !nothing_else(&pool, conversation).await.unwrap(),
        "a follow-up that has asked nothing is one nobody has ended"
    );

    let first = asked(&pool, conversation, "the round before").await;
    submit_response(&pool, &settlements(), first, &answering())
        .await
        .unwrap();

    assert!(
        !nothing_else(&pool, conversation).await.unwrap(),
        "and an answer without the option ticked is the human saying there is more"
    );

    let last = asked(&pool, conversation, "the last round").await;
    submit_response(&pool, &settlements(), last, &ending())
        .await
        .unwrap();

    assert!(
        nothing_else(&pool, conversation).await.unwrap(),
        "the newest round they answered carries the mark, so the follow-up is over"
    );
}

#[tokio::test]
async fn a_round_asked_after_the_mark_puts_the_follow_up_back_to_running() {
    let (_dir, pool) = fresh_pool().await;
    let conversation = conversation(&pool).await;

    following_up(&pool, conversation).await;

    let marked = asked(&pool, conversation, "the round they ticked").await;
    submit_response(&pool, &settlements(), marked, &ending())
        .await
        .unwrap();

    let again = asked(&pool, conversation, "one more thing then").await;

    assert!(
        nothing_else(&pool, conversation).await.unwrap(),
        "a round nobody has answered says nothing, so the marked one is still \
         the latest word"
    );

    submit_response(&pool, &settlements(), again, &answering())
        .await
        .unwrap();

    assert!(
        !nothing_else(&pool, conversation).await.unwrap(),
        "and its own Response decides: the mark is never sticky"
    );
}

#[tokio::test]
async fn a_mark_left_by_the_follow_up_before_this_one_ends_nothing() {
    let (_dir, pool) = fresh_pool().await;
    let conversation = conversation(&pool).await;

    following_up(&pool, conversation).await;

    let ended = asked(&pool, conversation, "the round that ended it").await;
    submit_response(&pool, &settlements(), ended, &ending())
        .await
        .unwrap();

    assert!(nothing_else(&pool, conversation).await.unwrap());

    // Back to the wrap-up, and steered into Follow-up a second time — which is a
    // follow-up that has asked nothing yet.
    assert_eq!(
        follow_up_over(&pool, conversation, false).await.unwrap(),
        Ending::Wrapped,
    );

    following_up(&pool, conversation).await;

    assert!(
        !nothing_else(&pool, conversation).await.unwrap(),
        "the window opens at the newest move into Follow-up, so last time's mark \
         is not this follow-up's word"
    );
}

#[tokio::test]
async fn a_locked_round_is_not_the_latest_word_and_a_deferred_one_is_never_counted() {
    let (_dir, pool) = fresh_pool().await;
    let conversation = conversation(&pool).await;

    following_up(&pool, conversation).await;

    let marked = asked(&pool, conversation, "the round they ticked").await;
    submit_response(&pool, &settlements(), marked, &ending())
        .await
        .unwrap();

    let locked = asked(&pool, conversation, "closed unanswered").await;
    lock_set(&pool, &settlements(), locked).await.unwrap();

    assert!(
        nothing_else(&pool, conversation).await.unwrap(),
        "a Set nobody answered carries no Response and so no word either way"
    );

    let aside = ask(
        &pool,
        conversation,
        &round("something for a later session"),
        Ask::Deferred,
    )
    .await
    .unwrap()
    .expect("the Conversation is there to ask from")
    .id;

    submit_response(&pool, &settlements(), aside, &answering())
        .await
        .unwrap();

    assert!(
        nothing_else(&pool, conversation).await.unwrap(),
        "and a Deferred Ask is nobody's round: its Answers are for a later \
         session by design"
    );
}

#[tokio::test]
async fn the_follow_up_lands_back_in_the_wrap_up_and_takes_the_checks_with_it() {
    let (_dir, pool) = fresh_pool().await;
    let conversation = conversation(&pool).await;

    // A pull request for the checks to be about. A Conversation ends on one per
    // repository it was worked in and each has a suite of its own, so what a
    // follow-up puts back to waiting is read off the record rather than named:
    // there is no *the* checks to settle without one.
    let repo = own_repo(&pool, conversation).await;
    record_another_pull_request(&pool, conversation, repo, &opened())
        .await
        .unwrap();

    for waiting_on in WAITED_ON {
        settle_wrap_up(&pool, conversation, waiting_on)
            .await
            .unwrap();
    }

    settle_wrap_up(&pool, conversation, WaitingOn::Checks(repo))
        .await
        .unwrap();

    following_up(&pool, conversation).await;

    assert_eq!(
        follow_up_over(&pool, conversation, true).await.unwrap(),
        Ending::Wrapped,
    );

    let conversation_row = load_conversation(&pool, conversation)
        .await
        .unwrap()
        .expect("the Conversation is there");

    assert_eq!(
        conversation_row.state,
        Lifecycle::Wrapping,
        "a follow-up ends where it started: the pull request it was opened about",
    );

    let settled = wrap_up_settled(&pool, conversation).await.unwrap();

    assert!(
        !settled.contains(&WaitingOn::Checks(repo)),
        "the follow-up pushed, so the green standing over the checks was the run \
         before it: {settled:?}",
    );
    assert!(
        settled.contains(&WaitingOn::Review),
        "and the review it was steered out of stays settled: this is the same \
         wrap, and the human has just been through it: {settled:?}",
    );
}

/// Which Repo a Conversation's own work is in, which is what its own pull
/// request is recorded against.
async fn own_repo(pool: &SqlitePool, conversation: i64) -> i64 {
    load_conversation(pool, conversation)
        .await
        .unwrap()
        .expect("the Conversation is there")
        .repo
        .id
}

/// The pull request the work ended up on, unlabeled: the Conversation's own
/// repository's, which is the one every one of these is about.
fn opened() -> PullRequest {
    PullRequest {
        number: 41,
        title: "Rate limiting".to_owned(),
        url: "https://github.com/tobico/verkstead/pull/41".to_owned(),
        repo: None,
    }
}

#[tokio::test]
async fn a_follow_up_that_pushed_nothing_lands_with_every_settle_standing() {
    let (_dir, pool) = fresh_pool().await;
    let conversation = conversation(&pool).await;

    for waiting_on in WAITED_ON {
        settle_wrap_up(&pool, conversation, waiting_on)
            .await
            .unwrap();
    }

    following_up(&pool, conversation).await;

    assert_eq!(
        follow_up_over(&pool, conversation, false).await.unwrap(),
        Ending::Wrapped,
    );

    let settled = wrap_up_settled(&pool, conversation).await.unwrap();

    assert!(
        WAITED_ON.iter().all(|one| settled.contains(one)),
        "there is no new run to wait on, so the wrap-up carries on from exactly \
         where the follow-up found it: {settled:?}",
    );
}

#[tokio::test]
async fn nothing_but_a_follow_up_can_be_landed_back_in_a_wrap_up() {
    let (_dir, pool) = fresh_pool().await;
    let conversation = conversation(&pool).await;

    assert_eq!(
        follow_up_over(&pool, conversation, false).await.unwrap(),
        Ending::NotFollowingUp,
        "a Conversation that is not following anything up has no follow-up to end",
    );
    assert_eq!(
        follow_up_over(&pool, 404, false).await.unwrap(),
        Ending::NoSuchConversation,
    );
}
