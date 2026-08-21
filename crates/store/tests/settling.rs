//! How a Question Set settles: answered, or closed unanswered by the human —
//! and what it will no longer accept once it has.
//!
//! Read through the Timeline the Set is on, which is the one way there is to
//! reach one. The standalone pending and archive lists that used to be the other
//! way are gone: a Set belongs to the Conversation it was asked from, and a
//! second route into it would be a second thing to keep true.

use std::path::Path;

use sqlx::SqlitePool;
use verkstead_schema::{QuestionSet, Response, SetCreated};
use verkstead_store::{
    Archiving, Event, Settlement, Settlements, Submission, archive_set, ask, conversations,
    insert_response, open_database, register_repo, start_conversation, submit_response, timeline,
};

/// A pool over a fresh database, plus the directory keeping it alive.
async fn fresh_pool() -> (tempfile::TempDir, SqlitePool) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();
    (dir, pool)
}

/// The channel a settling is announced on. Nothing here listens: what these
/// tests are about is what the store did, and the announcement is the server's
/// business.
fn settlements() -> Settlements {
    Settlements::new(4)
}

fn set(title: &str) -> QuestionSet {
    QuestionSet {
        title: title.to_owned(),
        preface: None,
        questions: Vec::new(),
        postscript: None,
        proposal: None,
        review: None,
        project: Some("verkstead".to_owned()),
        branch: Some("answering-conveniences".to_owned()),
        diff: None,
    }
}

/// Put a Set to the human, which is the one way there is to store one: every Set
/// is asked from a Conversation and lands on its Timeline.
///
/// The Conversation is made on the first ask and reused after it, so everything
/// asked here lands on the one Timeline these tests read.
async fn asked(pool: &SqlitePool, set: &QuestionSet) -> anyhow::Result<SetCreated> {
    Ok(ask(pool, conversation(pool).await?, set)
        .await?
        .expect("the Conversation is there to ask from"))
}

/// The Conversation everything here is asked from, made on the first ask.
async fn conversation(pool: &SqlitePool) -> anyhow::Result<i64> {
    if let Some(row) = conversations(pool).await?.first() {
        return Ok(row.id);
    }

    let repo = register_repo(pool, Path::new("/srv/verkstead"), "verkstead", "main")
        .await?
        .expect("nothing is registered at that path yet");

    Ok(start_conversation(pool, repo.id, "answering-conveniences")
        .await?
        .expect("the Repo was just registered"))
}

/// Every Set on the Conversation's Timeline, oldest first, as a title against
/// how it settled — which is the whole of what the Timeline says about one
/// without opening it.
async fn settlings(pool: &SqlitePool) -> Vec<(String, Option<Settlement>)> {
    let id = conversation(pool).await.unwrap();

    timeline(pool, id)
        .await
        .unwrap()
        .into_iter()
        .filter_map(|event| match event.event {
            Event::QuestionSet(asked) => Some((asked.set.title.clone(), asked.settlement)),
            _ => None,
        })
        .collect()
}

/// How one Set settled, off the Timeline it is on.
async fn settling(pool: &SqlitePool, set_id: i64) -> Option<Settlement> {
    let id = conversation(pool).await.unwrap();

    timeline(pool, id)
        .await
        .unwrap()
        .into_iter()
        .find_map(|event| match event.event {
            Event::QuestionSet(asked) if asked.set_id == set_id => Some(asked.settlement),
            _ => None,
        })
        .expect("the Set is on the Timeline it was asked from")
}

/// Store a Set and answer it, which is one of the two ways a Set settles.
async fn answer(pool: &SqlitePool, title: &str) -> i64 {
    let stored = asked(pool, &set(title)).await.unwrap();
    insert_response(pool, stored.id, &Response::default())
        .await
        .unwrap()
        .expect("the Set had no Response yet");
    stored.id
}

#[tokio::test]
async fn a_set_nobody_has_answered_is_still_waiting_on_the_human() {
    let (_dir, pool) = fresh_pool().await;

    asked(&pool, &set("still waiting")).await.unwrap();

    assert_eq!(
        settlings(&pool).await,
        [("still waiting".to_owned(), None)],
        "a Set settles by being answered or archived, not by anyone filing it",
    );
}

#[tokio::test]
async fn an_answered_set_carries_its_response_and_the_time_it_was_answered() {
    let (_dir, pool) = fresh_pool().await;

    let created = asked(&pool, &set("Where the request counter lives"))
        .await
        .unwrap();
    let accepted = insert_response(&pool, created.id, &Response::default())
        .await
        .unwrap()
        .unwrap();

    let Some(Settlement::Answered(answered)) = settling(&pool, created.id).await else {
        panic!("an answered Set reads as answered on its Timeline");
    };

    assert_eq!(answered.set_id, created.id);
    assert_eq!(
        answered.submitted_at, accepted.submitted_at,
        "the day the decision was made is the Set's own, stamped as it was taken",
    );
    assert_eq!(answered.response, Response::default());
}

#[tokio::test]
async fn archiving_a_set_settles_it_unanswered_and_leaves_the_rest_waiting() {
    let (_dir, pool) = fresh_pool().await;
    let orphan = asked(&pool, &set("the one whose agent died"))
        .await
        .unwrap();
    asked(&pool, &set("still waiting")).await.unwrap();

    assert!(
        settlings(&pool)
            .await
            .iter()
            .all(|(_, settlement)| settlement.is_none()),
        "both are waiting on the human before either is closed",
    );

    let archiving = archive_set(&pool, &settlements(), orphan.id).await.unwrap();
    let Archiving::Archived(archived) = archiving else {
        panic!("a Set nobody has answered should archive: {archiving:?}");
    };
    assert_eq!(archived.set_id, orphan.id);

    assert_eq!(
        settlings(&pool).await,
        [
            (
                "the one whose agent died".to_owned(),
                Some(Settlement::ArchivedUnanswered(archived)),
            ),
            ("still waiting".to_owned(), None),
        ],
        "the point of archiving is that the Set stops reading as waiting — and \
         that it says so without a Response, because there never was one",
    );
}

#[tokio::test]
async fn an_answered_set_cannot_be_archived_unanswered() {
    let (_dir, pool) = fresh_pool().await;
    let decided = answer(&pool, "already answered").await;
    let before = settling(&pool, decided).await;

    assert_eq!(
        archive_set(&pool, &settlements(), decided).await.unwrap(),
        Archiving::AlreadyAnswered,
        "a decision is not an orphan, and archiving is not a way to unmake one",
    );
    assert_eq!(
        settling(&pool, decided).await,
        before,
        "and the decision already filed was not touched",
    );
}

#[tokio::test]
async fn an_archived_set_will_not_take_a_response() {
    let (_dir, pool) = fresh_pool().await;
    let orphan = asked(&pool, &set("closed unanswered")).await.unwrap();
    archive_set(&pool, &settlements(), orphan.id).await.unwrap();

    assert_eq!(
        submit_response(&pool, &settlements(), orphan.id, &Response::default())
            .await
            .unwrap(),
        Submission::Archived,
        "archiving closes a Set for good: it must not also become an answered one",
    );

    assert!(
        matches!(
            settling(&pool, orphan.id).await,
            Some(Settlement::ArchivedUnanswered(_))
        ),
        "and it is still the Set nobody ever answered",
    );
}

#[tokio::test]
async fn archiving_a_set_twice_leaves_the_first_archiving_standing() {
    let (_dir, pool) = fresh_pool().await;
    let orphan = asked(&pool, &set("closed unanswered")).await.unwrap();

    let Archiving::Archived(first) = archive_set(&pool, &settlements(), orphan.id).await.unwrap()
    else {
        panic!("the first archiving should have taken");
    };

    // Two devices, or one page left open in a second tab.
    assert_eq!(
        archive_set(&pool, &settlements(), orphan.id).await.unwrap(),
        Archiving::AlreadyArchived,
    );

    assert_eq!(
        settling(&pool, orphan.id).await,
        Some(Settlement::ArchivedUnanswered(first)),
        "the Set was closed once, at the time it was closed",
    );
}

#[tokio::test]
async fn there_is_nothing_to_archive_about_a_set_that_does_not_exist() {
    let (_dir, pool) = fresh_pool().await;

    assert_eq!(
        archive_set(&pool, &settlements(), 404).await.unwrap(),
        Archiving::NoSuchSet,
    );
}
