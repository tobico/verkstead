//! Deferred Asks in the store: what marks one, what it does to the reading made
//! of an idle session, and the folding of its Answers into a later session's
//! prompt.
//!
//! The folding is the half worth reading twice. It is a record rather than a
//! reading of what happens to be answered — so what these ask is that a Set is
//! folded once, that recording it is what makes that true, and that nothing is
//! offered up before the human has answered it.

use std::path::Path;

use sqlx::SqlitePool;
use verkstead_schema::{Answer, Question, QuestionOption, QuestionSet, Response};
use verkstead_store::{
    Ask, Event, Locking, OwnAsking, PlacedWork, Settlements, ask, ask_on_our_own, insert_response,
    lock_line, lock_lines, lock_set, lock_set_saying, open_database, open_set, own_asking,
    own_set_ids, own_sets, record_folded, register_repo, start_conversation, submit_response,
    timeline, unfolded,
};

/// The device every Conversation started here is ranked by, named the way a
/// cluster names one (ADR-0020, *Ranks*).
const THIS_DEVICE: &str = "aa00bb11cc22dd33ee44ff5566778899";

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

    start_conversation(pool, repo.id, "deferred-asks", THIS_DEVICE, None)
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
        project: Some("verkstead".to_owned()),
        branch: Some("deferred-asks".to_owned()),
        diff: None,
        diffs: Vec::new(),
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
            attachments: Vec::new(),
            added: Vec::new(),
        }],
        comment: None,
        direction: None,
        nothing_else: false,
        added: Vec::new(),
        attachments: Vec::new(),
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

/// How each Set on the Timeline was asked, oldest first.
async fn deferrals(pool: &SqlitePool, conversation: i64) -> Vec<(String, Ask)> {
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
                asked.ask,
            )),
            _ => None,
        })
        .collect()
}

/// All three kinds land on the Timeline, and the Timeline says which is which:
/// every one of them is something to answer, and what differs is who is standing
/// still behind it.
#[tokio::test]
async fn the_timeline_tells_a_deferred_set_from_a_blocking_one() {
    let (_dir, pool) = fresh_pool().await;
    let conversation = conversation(&pool).await;

    asked(&pool, conversation, "blocking", Ask::Blocking).await;
    asked(&pool, conversation, "deferred", Ask::Deferred).await;
    asked(&pool, conversation, "stored", Ask::StoreAndNudge).await;

    assert_eq!(
        deferrals(&pool, conversation).await,
        [
            ("blocking".to_owned(), Ask::Blocking),
            ("deferred".to_owned(), Ask::Deferred),
            ("stored".to_owned(), Ask::StoreAndNudge),
        ],
    );
}

/// What the difference is *for*, on the store's side: a session idle behind a
/// Deferred Ask is idle with nothing open, where one behind a blocking Ask is
/// mid-question and must not be prodded.
#[tokio::test]
async fn a_deferred_set_is_not_a_session_still_asking() {
    let (_dir, pool) = fresh_pool().await;
    let conversation = conversation(&pool).await;

    let deferred = asked(&pool, conversation, "deferred", Ask::Deferred).await;

    assert_eq!(
        open_set(&pool, conversation).await.unwrap(),
        None,
        "nothing is idling on a Deferred Ask, so the human is holding nothing that \
         should hold the rescue off",
    );

    let blocking = asked(&pool, conversation, "blocking", Ask::Blocking).await;

    assert_eq!(
        open_set(&pool, conversation).await.unwrap(),
        Some(blocking),
        "and a blocking one is exactly that",
    );

    assert_ne!(blocking, deferred);
}

/// And a store-and-nudge ask is a session still asking, stored though it is.
///
/// The whole point of the third state. Its session has ended its turn and is
/// waiting for the line Verkstead types when the Response lands, so a reader
/// that took the row beside it as *nobody is waiting* would type a rescue into
/// that session in the middle of a wait that is working.
#[tokio::test]
async fn a_store_and_nudge_set_is_a_session_still_asking() {
    let (_dir, pool) = fresh_pool().await;
    let conversation = conversation(&pool).await;

    let stored = asked(&pool, conversation, "stored", Ask::StoreAndNudge).await;

    assert_eq!(
        open_set(&pool, conversation).await.unwrap(),
        Some(stored),
        "a session is idling on it, so its own session is not one to prod",
    );

    submit_response(&pool, &Settlements::new(4), stored, &picked())
        .await
        .unwrap();

    assert_eq!(
        open_set(&pool, conversation).await.unwrap(),
        None,
        "and answering it settles it as it settles any other",
    );

    // And its Answers fold, exactly as a Deferred Ask's do: a session that has
    // gone before the nudge reaches it is the folding rule's own case.
    assert_eq!(folding(&pool, conversation).await, ["stored".to_owned()]);
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
    let locked = asked(&pool, conversation, "locked", Ask::Deferred).await;
    let blocking = asked(&pool, conversation, "blocking", Ask::Blocking).await;

    for settled in [answered, blocking] {
        insert_response(&pool, settled, &picked())
            .await
            .unwrap()
            .expect("neither had a Response yet");
    }

    lock_set(&pool, &Settlements::new(4), locked).await.unwrap();

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
    let yours = start_conversation(&pool, repo, "somebody-elses-work", THIS_DEVICE, None)
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

/// **A Set of Verkstead's own is never folded into a session's prompt**, stored
/// as a Deferred Ask though it is: its Answers are Verkstead's to act on, and
/// it reads back as Verkstead's, with what it asked about.
#[tokio::test]
async fn a_set_of_verksteads_own_is_never_folded() {
    let (_dir, pool) = fresh_pool().await;
    let conversation = conversation(&pool).await;

    let about = OwnAsking::Placing {
        work: PlacedWork::Task {
            number: 3,
            label: "03".to_owned(),
        },
        wants: "Windows".to_owned(),
        moves: vec!["0011223344556677889900aabbccddee".to_owned()],
        waiting_on: vec!["ffeeddccbbaa99887766554433221100".to_owned()],
    };
    let own = ask_on_our_own(&pool, conversation, &set("placing"), &about)
        .await
        .unwrap()
        .expect("the Conversation is there to ask from")
        .id;
    let agents = asked(&pool, conversation, "the agent's", Ask::Deferred).await;

    for answered in [own, agents] {
        submit_response(&pool, &Settlements::new(4), answered, &picked())
            .await
            .unwrap();
    }

    assert_eq!(folding(&pool, conversation).await, ["the agent's"]);

    assert_eq!(own_asking(&pool, own).await.unwrap(), Some(about.clone()));
    assert_eq!(own_asking(&pool, agents).await.unwrap(), None);
    assert_eq!(
        own_set_ids(&pool, conversation).await.unwrap(),
        [own].into_iter().collect(),
    );

    let owned = own_sets(&pool, conversation).await.unwrap();
    assert_eq!(owned.len(), 1);
    assert_eq!(owned[0].asking, about);
    assert_eq!(owned[0].response, Some(picked()));
    assert!(!owned[0].locked);
}

/// **A lock Verkstead takes carries its line**, read back by the Set and by
/// the Conversation; a human's lock carries none, and a Set already answered
/// takes no lock and so no line.
#[tokio::test]
async fn a_lock_with_a_line_reads_back_with_it() {
    let (_dir, pool) = fresh_pool().await;
    let conversation = conversation(&pool).await;
    let settlements = Settlements::new(4);

    let woke = asked(&pool, conversation, "woke", Ask::Deferred).await;
    let pressed = asked(&pool, conversation, "pressed", Ask::Deferred).await;
    let answered = asked(&pool, conversation, "answered", Ask::Deferred).await;

    let line = "nuc-win answered, so the work moved there.";

    assert!(matches!(
        lock_set_saying(&pool, &settlements, woke, line)
            .await
            .unwrap(),
        Locking::Locked(_),
    ));
    assert!(matches!(
        lock_set(&pool, &settlements, pressed).await.unwrap(),
        Locking::Locked(_),
    ));
    submit_response(&pool, &settlements, answered, &picked())
        .await
        .unwrap();
    assert_eq!(
        lock_set_saying(&pool, &settlements, answered, line)
            .await
            .unwrap(),
        Locking::AlreadyAnswered,
    );

    assert_eq!(lock_line(&pool, woke).await.unwrap().as_deref(), Some(line));
    assert_eq!(lock_line(&pool, pressed).await.unwrap(), None);
    assert_eq!(lock_line(&pool, answered).await.unwrap(), None);
    assert_eq!(
        lock_lines(&pool, conversation).await.unwrap(),
        [(woke, line.to_owned())].into_iter().collect(),
    );
}
