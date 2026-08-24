//! Deferred Asks in the store: what marks one, what it does to the reading a
//! driver makes of a quiet session, and the folding of its Answers into a later
//! session's prompt.
//!
//! The folding is the half worth reading twice. It is a record rather than a
//! reading of what happens to be answered — so what these ask is that a Set is
//! folded once, that recording it is what makes that true, and that nothing is
//! offered up before the human has answered it.

use std::path::Path;

use sqlx::SqlitePool;
use verkstead_schema::{Answer, Question, QuestionOption, QuestionSet, Response};
use verkstead_store::{
    Ask, Event, Settlements, archive_set, ask, insert_response, open_database, record_folded,
    register_repo, start_conversation, submit_response, timeline, unanswered_set_since, unfolded,
};

/// A pool over a fresh database, plus the directory keeping it alive.
async fn fresh_pool() -> (tempfile::TempDir, SqlitePool) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();
    (dir, pool)
}

/// The Conversation everything here is asked from.
async fn conversation(pool: &SqlitePool) -> i64 {
    let repo = register_repo(pool, Path::new("/srv/verkstead"), "verkstead", "main")
        .await
        .unwrap()
        .expect("nothing is registered at that path yet");

    start_conversation(pool, repo.id, "deferred-asks")
        .await
        .unwrap()
        .expect("the Repo was just registered")
}

/// A Set with one Question, so an Answer to it has something to be an Answer to.
fn set(title: &str) -> QuestionSet {
    QuestionSet {
        title: title.to_owned(),
        preface: None,
        questions: vec![Question {
            label: "Q1".to_owned(),
            text: "Which way round?".to_owned(),
            columns: Vec::new(),
            options: vec![
                QuestionOption {
                    n: 1,
                    text: "This way".to_owned(),
                    recommended: false,
                    cells: Vec::new(),
                },
                QuestionOption {
                    n: 2,
                    text: "That way".to_owned(),
                    recommended: false,
                    cells: Vec::new(),
                },
            ],
            subquestions: Vec::new(),
        }],
        postscript: None,
        proposal: None,
        review: None,
        project: Some("verkstead".to_owned()),
        branch: Some("deferred-asks".to_owned()),
        diff: None,
    }
}

/// An Answer picking the first Option, which is a Response that resolves [`set`].
fn picked() -> Response {
    Response {
        answers: vec![Answer {
            label: "Q1".to_owned(),
            selected: Some(1),
            free_text: None,
            unanswered: false,
        }],
        comment: None,
        direction: None,
    }
}

/// Ask a Set, one kind or the other, and hand back its id.
async fn asked(pool: &SqlitePool, conversation: i64, title: &str, kind: Ask) -> i64 {
    ask(pool, conversation, &set(title), kind)
        .await
        .unwrap()
        .expect("the Conversation is there to ask from")
        .id
}

/// The titles of what is waiting to be folded, in the order it comes back.
async fn folding(pool: &SqlitePool, conversation: i64) -> Vec<String> {
    unfolded(pool, conversation)
        .await
        .unwrap()
        .into_iter()
        .map(|answered| {
            answered
                .set
                .set()
                .expect("the stored Set reads back")
                .title
                .clone()
        })
        .collect()
}

/// Which Sets on the Timeline were asked deferred, oldest first.
async fn deferrals(pool: &SqlitePool, conversation: i64) -> Vec<(String, bool)> {
    timeline(pool, conversation)
        .await
        .unwrap()
        .into_iter()
        .filter_map(|event| match event.event {
            Event::QuestionSet(asked) => Some((
                asked
                    .set
                    .set()
                    .expect("the stored Set reads back")
                    .title
                    .clone(),
                asked.deferred,
            )),
            _ => None,
        })
        .collect()
}

/// Both kinds land on the Timeline, and the Timeline says which is which: both
/// are something to answer, and only one of them has a session standing still
/// behind it.
#[tokio::test]
async fn the_timeline_tells_a_deferred_set_from_a_blocking_one() {
    let (_dir, pool) = fresh_pool().await;
    let conversation = conversation(&pool).await;

    asked(&pool, conversation, "blocking", Ask::Blocking).await;
    asked(&pool, conversation, "deferred", Ask::Deferred).await;

    assert_eq!(
        deferrals(&pool, conversation).await,
        [
            ("blocking".to_owned(), false),
            ("deferred".to_owned(), true)
        ],
    );
}

/// What the difference is *for*, on the store's side: a session that has gone
/// quiet behind a Deferred Ask has finished, where one behind a blocking Ask is
/// mid-question and must not be reaped.
#[tokio::test]
async fn a_deferred_set_is_not_a_session_still_asking() {
    let (_dir, pool) = fresh_pool().await;
    let conversation = conversation(&pool).await;

    // Everything asked after the Conversation's own first Event, which is what
    // "this session's" means here.
    let since = 0;

    let deferred = asked(&pool, conversation, "deferred", Ask::Deferred).await;

    assert_eq!(
        unanswered_set_since(&pool, conversation, since)
            .await
            .unwrap(),
        None,
        "nothing is idling on a Deferred Ask, so nothing is waiting to be answered \
         before the session can be ended",
    );

    let blocking = asked(&pool, conversation, "blocking", Ask::Blocking).await;

    assert_eq!(
        unanswered_set_since(&pool, conversation, since)
            .await
            .unwrap(),
        Some(blocking),
        "and a blocking one is exactly that",
    );

    assert_ne!(blocking, deferred);
}

/// Only what the human has actually answered: an unanswered Deferred Ask is
/// still theirs to decide, and one they closed unanswered is a decision they
/// declined to make.
#[tokio::test]
async fn only_answered_deferred_sets_are_waiting_to_be_folded() {
    let (_dir, pool) = fresh_pool().await;
    let conversation = conversation(&pool).await;

    let answered = asked(&pool, conversation, "answered", Ask::Deferred).await;
    asked(&pool, conversation, "unanswered", Ask::Deferred).await;
    let archived = asked(&pool, conversation, "archived", Ask::Deferred).await;
    let blocking = asked(&pool, conversation, "blocking", Ask::Blocking).await;

    for settled in [answered, blocking] {
        insert_response(&pool, settled, &picked())
            .await
            .unwrap()
            .expect("neither had a Response yet");
    }

    archive_set(&pool, &Settlements::new(4), archived)
        .await
        .unwrap();

    assert_eq!(
        folding(&pool, conversation).await,
        ["answered".to_owned()],
        "an answered blocking Set went to the session that asked it, and the \
         other two have no Answers to fold",
    );
}

/// Folded once and never again — and it is the recording that makes it so,
/// rather than anything about the Set having been answered.
#[tokio::test]
async fn a_folded_set_is_not_offered_up_again() {
    let (_dir, pool) = fresh_pool().await;
    let conversation = conversation(&pool).await;

    let first = asked(&pool, conversation, "first", Ask::Deferred).await;
    let second = asked(&pool, conversation, "second", Ask::Deferred).await;

    // Answered out of order, to prove the order they come back in is the order
    // they were asked rather than the order they were decided.
    for settled in [second, first] {
        submit_response(&pool, &Settlements::new(4), settled, &picked())
            .await
            .unwrap();
    }

    assert_eq!(
        folding(&pool, conversation).await,
        ["first".to_owned(), "second".to_owned()],
        "oldest first, which is the order the human was asked in",
    );

    record_folded(&pool, &[first]).await.unwrap();

    assert_eq!(
        folding(&pool, conversation).await,
        ["second".to_owned()],
        "the one that has been into a prompt does not go into the next one",
    );

    record_folded(&pool, &[second]).await.unwrap();

    assert!(
        folding(&pool, conversation).await.is_empty(),
        "and a session started after both has nothing of theirs to be told",
    );
}

/// A Conversation's own, and nobody else's: two Conversations may be running at
/// once, and an Answer folded into the wrong prompt is an instruction about
/// somebody else's work.
#[tokio::test]
async fn only_this_conversations_deferred_answers_are_folded_into_its_prompts() {
    let (_dir, pool) = fresh_pool().await;
    let mine = conversation(&pool).await;

    let repo = verkstead_store::registered_repos(&pool).await.unwrap()[0].id;
    let yours = start_conversation(&pool, repo, "somebody-elses-work")
        .await
        .unwrap()
        .unwrap();

    let theirs = asked(&pool, yours, "theirs", Ask::Deferred).await;
    submit_response(&pool, &Settlements::new(4), theirs, &picked())
        .await
        .unwrap();

    assert!(folding(&pool, mine).await.is_empty());
    assert_eq!(folding(&pool, yours).await, ["theirs".to_owned()]);
}
