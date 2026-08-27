//! The Nothing-else mark in the store: the human saying, on a follow-up's Set,
//! that there is nothing else to follow up.
//!
//! The mark rides the submitted Response and is kept beside the stored one, and
//! the second half of that is what these are mostly about: the Response a
//! waiting agent is handed has to be the same bytes whether the human ticked the
//! option or not, because how a follow-up ends is Verkstead's business and the
//! session is not allowed a way to act on it.
//!
//! What reads the mark back is a rule about a whole follow-up, which is not
//! here yet. What is here is the mark itself, per Set: written where the
//! Response landed, absent where it did not, and never sticky.

use std::path::Path;

use sqlx::SqlitePool;
use verkstead_schema::{Answer, Question, QuestionSet, Response};
use verkstead_store::{
    Ask, Settlements, Submission, ask, ended_on, load_response, lock_set, open_database,
    register_repo, start_conversation, submit_response,
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
