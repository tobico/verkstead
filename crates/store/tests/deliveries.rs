//! The Delivery in the store: whether a Set's Answers ever reached a session.
//!
//! One fact, and the whole of why it is written down separately from the
//! folding: a Blocking Ask has no folding record, because a wait that ends is a
//! wait that delivered what it was holding. A wait that is *killed* ends too,
//! and delivers nothing — so *answered* and *collected* stopped being the same
//! thing, and something had to say which.
//!
//! So these are mostly about a blocking ask, which is the kind that had no
//! record before and the kind the nudge now reads one for.

use std::path::Path;

use sqlx::SqlitePool;
use verkstead_schema::{Answer, Question, QuestionSet, Response};
use verkstead_store::{
    Ask, Settlements, Submission, ask, asked_as, delivered, open_database, record_delivery,
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
async fn conversation(pool: &SqlitePool) -> i64 {
    let repo = register_repo(pool, Path::new("/srv/verkstead"), "verkstead", "main")
        .await
        .unwrap()
        .expect("nothing is registered at that path yet");

    start_conversation(pool, repo.id, "dropped-wait-recovery")
        .await
        .unwrap()
        .expect("the Repo was just registered")
}

/// One question, which is all a Delivery needs to hang off.
fn round() -> QuestionSet {
    QuestionSet {
        title: "Where should the counter live?".to_owned(),
        preface: None,
        questions: vec![Question {
            label: "Q1".to_owned(),
            text: "In process, or in Redis?".to_owned(),
            columns: Vec::new(),
            options: Vec::new(),
            subquestions: Vec::new(),
        }],
        postscript: None,
        proposal: None,
        project: Some("verkstead".to_owned()),
        branch: Some("dropped-wait-recovery".to_owned()),
        diff: None,
        diffs: Vec::new(),
    }
}

/// Put a Set on the Timeline, asked the given way, and hand back its id.
async fn asked(pool: &SqlitePool, conversation: i64, how: Ask) -> i64 {
    ask(pool, conversation, &round(), how)
        .await
        .unwrap()
        .expect("the Conversation is there to ask from")
        .id
}

/// The human answering it.
fn answering() -> Response {
    Response {
        answers: vec![Answer {
            label: "Q1".to_owned(),
            selected: None,
            free_text: Some("In process for now.".to_owned()),
            unanswered: false,
            attachments: Vec::new(),
        }],
        comment: None,
        direction: None,
        nothing_else: false,
    }
}

/// When the Delivery says the Answers reached somebody, straight off the row.
async fn delivered_at(pool: &SqlitePool, set_id: i64) -> String {
    let (at,): (String,) = sqlx::query_as("SELECT delivered_at FROM deliveries WHERE set_id = ?")
        .bind(set_id)
        .fetch_one(pool)
        .await
        .unwrap();

    at
}

/// The state every Set starts in, and the state the ones that matter stay in:
/// answered by the human, and handed to nobody.
#[tokio::test]
async fn a_set_nobody_has_been_handed_has_no_delivery() {
    let (_dir, pool) = fresh_pool().await;
    let conversation = conversation(&pool).await;
    let set_id = asked(&pool, conversation, Ask::Blocking).await;

    assert!(
        !delivered(&pool, set_id).await.unwrap(),
        "a Set nobody has answered has nothing to have delivered",
    );

    let taken = submit_response(&pool, &settlements(), set_id, &answering())
        .await
        .unwrap();
    assert!(matches!(taken, Submission::Accepted(_)));

    assert!(
        !delivered(&pool, set_id).await.unwrap(),
        "and answering it is not handing it over — which is the whole condition \
         the nudge reads: answered, with no Delivery",
    );
}

/// The ordinary ending, and the one the killed wait is told apart from.
#[tokio::test]
async fn handing_the_response_over_is_the_delivery() {
    let (_dir, pool) = fresh_pool().await;
    let conversation = conversation(&pool).await;
    let set_id = asked(&pool, conversation, Ask::Blocking).await;

    record_delivery(&pool, set_id).await.unwrap();

    assert!(delivered(&pool, set_id).await.unwrap());
}

/// Because a Response may be fetched more than once — by a session told to come
/// back for it, or by one that was handed it and asked again — and none of those
/// is a second delivery.
#[tokio::test]
async fn the_first_handing_over_is_the_one_that_counts() {
    let (_dir, pool) = fresh_pool().await;
    let conversation = conversation(&pool).await;
    let set_id = asked(&pool, conversation, Ask::Blocking).await;

    record_delivery(&pool, set_id).await.unwrap();
    let first = delivered_at(&pool, set_id).await;

    record_delivery(&pool, set_id).await.unwrap();

    assert_eq!(
        delivered_at(&pool, set_id).await,
        first,
        "a second fetch is the same Answers reaching the same place, so the row \
         is left saying when they first did",
    );
}

/// The kind the record exists for. A blocking ask has no deferral row at all, so
/// before there was a Delivery there was nothing about one to read: *answered*
/// and *collected* were the same record.
#[tokio::test]
async fn a_blocking_ask_has_a_delivery_where_it_has_no_folding() {
    let (_dir, pool) = fresh_pool().await;
    let conversation = conversation(&pool).await;
    let set_id = asked(&pool, conversation, Ask::Blocking).await;

    assert_eq!(
        asked_as(&pool, set_id).await.unwrap(),
        Ask::Blocking,
        "no deferral row, which is what makes one a blocking ask",
    );

    record_delivery(&pool, set_id).await.unwrap();

    assert!(
        delivered(&pool, set_id).await.unwrap(),
        "and the Delivery is written for it all the same",
    );
}

/// Kept for every kind rather than for the one that needed it, so that one
/// question about a Set has one answer however it was asked.
#[tokio::test]
async fn every_kind_of_ask_keeps_one() {
    let (_dir, pool) = fresh_pool().await;
    let conversation = conversation(&pool).await;

    for how in [Ask::Blocking, Ask::StoreAndNudge, Ask::Deferred] {
        let set_id = asked(&pool, conversation, how).await;

        assert!(!delivered(&pool, set_id).await.unwrap());
        record_delivery(&pool, set_id).await.unwrap();
        assert!(
            delivered(&pool, set_id).await.unwrap(),
            "a {how:?} ask's Response is handed over on the same door as any other",
        );
    }
}
