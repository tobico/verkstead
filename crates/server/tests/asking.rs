//! Question Sets on a Conversation's Timeline: what attributes one, what the
//! Timeline says about it, and what answering it does to the session waiting.
//!
//! The whole of the agent contract hangs off the Conversation a session is
//! running for, because that is what its `VERKSTEAD_SERVER` says — see
//! `tests/sandbox.rs` for the other end of that, where a probe inside a sandbox
//! reads the variable back. What is proved here is the half the sandbox cannot
//! see: that a Set posted to one Conversation's own base URL lands there and
//! nowhere else, that two Conversations grilling the same Repo stay apart, and
//! that a Response taken through the browser's route ends the wait the agent is
//! holding on the agents' one.

use std::path::Path;
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use serde::de::DeserializeOwned;
use sqlx::SqlitePool;
use tower::ServiceExt;
use verkstead_render::{
    ConversationEntry, ConversationView, QuestionSetEvent, SetRow, Standing, Submitted,
    TimelineEvent,
};
use verkstead_schema::{Liveness, SetCreated};
use verkstead_server::{open_database, router, store};

/// What a grilling session asks: a Question, a Sub-question under it, and a
/// Heading over them — the three shapes a row of the Timeline's table can take.
const SET: &str = r#"
title: Retry policy for the outbound queue
preface: Failed deliveries are retried forever.
questions:
  - label: Q1
    text: Where should the retry counter live?
    options:
      - n: 1
        text: On the delivery row
      - n: 2
        text: In a table of its own
        recommended: true
  - label: Q2
    text: How should a dead endpoint be given up on?
    subquestions:
      - letter: a
        text: After how many failures?
        options:
          - n: 1
            text: Five
            recommended: true
          - n: 2
            text: Fifty
      - letter: b
        text: And what happens to what is queued behind it?
"#;

/// A Response resolving it, one Answer of each kind: an Option chosen, an Option
/// chosen with a word about why, and a question left open.
///
/// `Q2` is not among them and must not be: it heads its Sub-questions and asks
/// nothing of its own.
const ANSWERED: &str = r#"
answers:
  - label: Q1
    selected: 2
  - label: Q2a
    selected: 1
    free_text: five, then stop
  - label: Q2b
    unanswered: true
"#;

/// Two Conversations against one Repo, which is the arrangement nothing but an
/// explicit scope can tell apart: the CLI derives the project and the branch
/// from the working directory, and both would say the same thing here.
async fn two_conversations() -> (tempfile::TempDir, SqlitePool, Router, i64, i64) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    let repo = store::register_repo(&pool, Path::new("/srv/verkstead"), "verkstead", "main")
        .await
        .unwrap()
        .expect("nothing is registered at that path yet");

    let first = store::start_conversation(&pool, repo.id, "outbound-retries")
        .await
        .unwrap()
        .unwrap();
    let second = store::start_conversation(&pool, repo.id, "rate-limiting")
        .await
        .unwrap()
        .unwrap();

    (dir, pool.clone(), router(pool), first, second)
}

/// Submit a Set the way a session's CLI does: to the base URL its sandbox was
/// given, which is this Conversation's own.
async fn ask(app: &Router, conversation: i64, yaml: &str) -> (StatusCode, String) {
    fetch(
        app,
        Request::builder()
            .method("POST")
            .uri(format!("/conversations/{conversation}/api/v1/sets"))
            .header(header::CONTENT_TYPE, "application/yaml")
            .body(Body::from(yaml.to_owned()))
            .unwrap(),
    )
    .await
}

/// The same, deferred: the flag the CLI sends when the session is not going to
/// wait, which is a query parameter rather than anything in the Set — see the
/// server's `sets` module.
async fn ask_deferred(app: &Router, conversation: i64, yaml: &str) -> (StatusCode, String) {
    fetch(
        app,
        Request::builder()
            .method("POST")
            .uri(format!(
                "/conversations/{conversation}/api/v1/sets?deferred=true"
            ))
            .header(header::CONTENT_TYPE, "application/yaml")
            .body(Body::from(yaml.to_owned()))
            .unwrap(),
    )
    .await
}

/// The same, insisting it was taken, and handing back the id the agent then
/// waits on.
async fn asked(app: &Router, conversation: i64, yaml: &str) -> i64 {
    let (status, body) = ask(app, conversation, yaml).await;
    assert_eq!(status, StatusCode::CREATED, "{body}");

    let created: SetCreated = serde_saphyr::from_str(&body).unwrap();
    created.id
}

/// And the same for a Deferred Ask, which is taken and answered exactly as one
/// that waits.
async fn deferred(app: &Router, conversation: i64, yaml: &str) -> i64 {
    let (status, body) = ask_deferred(app, conversation, yaml).await;
    assert_eq!(status, StatusCode::CREATED, "{body}");

    let created: SetCreated = serde_saphyr::from_str(&body).unwrap();
    created.id
}

/// [`SET`] under another title, for a test that asks twice and has to tell the
/// two apart on the page.
fn retitled(title: &str) -> String {
    SET.replace("Retry policy for the outbound queue", title)
}

/// Whether the sidebar says this Conversation wants the human.
async fn waiting(app: &Router, conversation: i64) -> bool {
    let (status, body) = fetch(
        app,
        Request::builder()
            .uri("/api/ui/conversations")
            .body(Body::empty())
            .unwrap(),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "{body}");

    let sidebar: Vec<ConversationEntry> = serde_json::from_str(&body).unwrap();

    sidebar
        .into_iter()
        .find(|row| row.id == conversation)
        .expect("the Conversation is on the sidebar")
        .waiting
}

/// Open a wait the way the CLI does, through the Conversation the Set was asked
/// from.
async fn wait(app: &Router, conversation: i64, id: i64, hold: u64) -> (StatusCode, String) {
    fetch(
        app,
        Request::builder()
            .uri(format!(
                "/conversations/{conversation}/api/v1/sets/{id}/response?hold={hold}"
            ))
            .body(Body::empty())
            .unwrap(),
    )
    .await
}

/// A Conversation as the workbench reads it.
async fn view(app: &Router, conversation: i64) -> ConversationView {
    let (status, body) = fetch(
        app,
        Request::builder()
            .uri(format!("/api/ui/conversations/{conversation}"))
            .body(Body::empty())
            .unwrap(),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "{body}");
    serde_json::from_str(&body).unwrap_or_else(|err| panic!("reading {body:?}: {err}"))
}

/// Every Question Set on a Conversation's Timeline, in the order it was asked.
fn sets(view: &ConversationView) -> Vec<&QuestionSetEvent> {
    view.timeline
        .iter()
        .filter_map(|event| match event {
            TimelineEvent::QuestionSet(asked) => Some(asked),
            _ => None,
        })
        .collect()
}

/// Answer a Set the way the workbench and the phone both do: the viewer's own
/// route, in JSON. There is one, and it is the same one from either device.
async fn answer<T: DeserializeOwned>(app: &Router, id: i64, response: serde_json::Value) -> T {
    let (status, body) = fetch(
        app,
        Request::builder()
            .method("POST")
            .uri(format!("/api/ui/sets/{id}/response"))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(serde_json::to_vec(&response).unwrap()))
            .unwrap(),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "{body}");
    serde_json::from_str(&body).unwrap_or_else(|err| panic!("reading {body:?}: {err}"))
}

async fn fetch(app: &Router, request: Request<Body>) -> (StatusCode, String) {
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8(bytes.to_vec()).unwrap())
}

/// The Response `ANSWERED` is, as a browser sends one.
fn decided() -> serde_json::Value {
    serde_json::json!({
        "answers": [
            { "label": "Q1", "selected": 2 },
            { "label": "Q2a", "selected": 1, "free_text": "five, then stop" },
            { "label": "Q2b", "unanswered": true },
        ],
    })
}

/// The whole point of the scope: a Set is that Conversation's, said outright
/// rather than worked out from what the CLI derived.
#[tokio::test]
async fn a_set_lands_on_the_timeline_of_the_conversation_it_was_asked_from() {
    let (_dir, _pool, app, first, second) = two_conversations().await;

    let id = asked(&app, first, SET).await;

    let asked_here = sets(&view(&app, first).await)
        .first()
        .map(|asked| (asked.set_id, asked.title.clone()))
        .expect("the Set is on the Timeline it was asked from");

    assert_eq!(
        asked_here,
        (id, "Retry policy for the outbound queue".to_owned())
    );
    assert!(
        sets(&view(&app, second).await).is_empty(),
        "the other Conversation was not asked anything"
    );
}

/// Two Conversations against one Repo, on branches of their own, are the case
/// the derived project and branch cannot tell apart.
#[tokio::test]
async fn two_conversations_on_one_repo_each_receive_only_their_own_sets() {
    let (_dir, _pool, app, first, second) = two_conversations().await;

    let mine = asked(&app, first, SET).await;
    let yours = asked(
        &app,
        second,
        &SET.replace(
            "Retry policy for the outbound queue",
            "Rate limiting for the public API",
        ),
    )
    .await;

    let on = |view: &ConversationView| -> Vec<i64> {
        sets(view).into_iter().map(|asked| asked.set_id).collect()
    };

    assert_eq!(on(&view(&app, first).await), [mine]);
    assert_eq!(on(&view(&app, second).await), [yours]);
}

/// A Conversation that is not there has no Timeline for a Set to land on and
/// nobody who would ever see it, so the Set is refused rather than stored
/// somewhere nothing reads.
#[tokio::test]
async fn a_set_asked_from_no_conversation_at_all_is_refused() {
    let (_dir, _pool, app, _first, _second) = two_conversations().await;

    let (status, body) = ask(&app, 404, SET).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(
        body.contains("404"),
        "the refusal should name the Conversation that is not there: {body}"
    );
}

/// A scope written into the path and never read would be no scope at all.
#[tokio::test]
async fn another_conversations_set_is_not_reachable_through_this_one() {
    let (_dir, _pool, app, first, second) = two_conversations().await;

    let id = asked(&app, first, SET).await;

    let (waiting, _) = wait(&app, second, id, 0).await;
    assert_eq!(
        waiting,
        StatusCode::NOT_FOUND,
        "a session may wait on its own Conversation's Sets and no others"
    );

    let (submitted, _) = fetch(
        &app,
        Request::builder()
            .method("POST")
            .uri(format!("/conversations/{second}/api/v1/sets/{id}/response"))
            .header(header::CONTENT_TYPE, "application/yaml")
            .body(Body::from(ANSWERED))
            .unwrap(),
    )
    .await;
    assert_eq!(submitted, StatusCode::NOT_FOUND);

    // And through its own, it is exactly where it was.
    let (its_own, _) = wait(&app, first, id, 0).await;
    assert_eq!(
        its_own,
        StatusCode::NO_CONTENT,
        "still waiting on the human"
    );
}

/// A Set opened by its own id — which is what a push notification opens on a
/// phone — says which Conversation it belongs to, because that is the only way
/// back from it.
#[tokio::test]
async fn a_set_read_by_its_own_id_names_the_conversation_it_was_asked_from() {
    let (_dir, _pool, app, first, second) = two_conversations().await;

    let its_own = asked(&app, first, SET).await;
    let the_others = asked(&app, second, SET).await;

    for (id, expected) in [(its_own, first), (the_others, second)] {
        let (status, body) = fetch(
            &app,
            Request::builder()
                .uri(format!("/api/ui/sets/{id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");

        let reading: verkstead_render::SetReading =
            serde_json::from_str(&body).unwrap_or_else(|err| panic!("reading {body:?}: {err}"));
        let verkstead_render::SetReading::Set(view) = reading else {
            panic!("a Set this build just stored reads back: {body}");
        };
        assert_eq!(
            view.conversation, expected,
            "Set {id} should lead back to the Conversation it was asked from",
        );
    }
}

/// The design's summary for a Question Set: a table of number, question and
/// answer. Every Question and Sub-question gets a row, including the Heading
/// that asks nothing — its text is what the rows under it are read against.
#[tokio::test]
async fn the_timeline_summarises_a_set_as_a_number_question_answer_table() {
    let (_dir, _pool, app, first, _second) = two_conversations().await;

    let id = asked(&app, first, SET).await;

    let waiting = view(&app, first).await;
    let rows: Vec<(&str, bool, &str)> = sets(&waiting)[0]
        .rows
        .iter()
        .map(|row: &SetRow| (row.name.as_str(), row.nested, row.answer.as_str()))
        .collect();

    assert_eq!(
        rows,
        [
            ("Q1", false, ""),
            ("Q2", false, ""),
            ("Q2a", true, ""),
            ("Q2b", true, ""),
        ],
        "nothing is decided on a Set still waiting, and a Sub-question says it is one"
    );
    assert_eq!(
        sets(&waiting)[0].rows[1].question,
        "How should a dead endpoint be given up on?",
        "the question column is the agent's words, as words"
    );

    assert_eq!(
        answer::<Submitted>(&app, id, decided()).await,
        Submitted::Accepted
    );

    let answered = view(&app, first).await;
    let decided: Vec<&str> = sets(&answered)[0]
        .rows
        .iter()
        .map(|row| row.answer.as_str())
        .collect();

    assert_eq!(
        decided,
        [
            // The Option that was chosen…
            "In a table of its own",
            // …nothing, because a Heading was never asked…
            "",
            // …the Option and the words that qualified it…
            "Five — five, then stop",
            // …and a question the human left open.
            "",
        ],
    );
}

/// What closes the grill loop: the session idles on its wait, the human answers
/// from whatever device is to hand, and the wait ends with the Response in it.
///
/// One route for both devices, which is why there is one test: the workbench and
/// the phone are the same viewer posting to the same endpoint, and what would
/// make them differ is not something the server has.
#[tokio::test]
async fn answering_through_the_viewer_ends_the_wait_the_session_is_holding() {
    let (_dir, _pool, app, first, _second) = two_conversations().await;

    let id = asked(&app, first, SET).await;

    // The session, idling on a wait held open — through a clone of the router,
    // because what has to hold is that the browser's half reaches the agents'.
    let held = tokio::spawn({
        let app = app.clone();
        async move { wait(&app, first, id, 30).await }
    });

    // Long enough for the wait to be holding rather than about to be.
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(!held.is_finished(), "the session should still be idling");

    assert_eq!(
        answer::<Submitted>(&app, id, decided()).await,
        Submitted::Accepted
    );

    let (status, body) = tokio::time::timeout(Duration::from_secs(5), held)
        .await
        .expect("answering should have ended the wait")
        .unwrap();

    assert_eq!(status, StatusCode::OK);
    let response = verkstead_schema::Response::from_yaml(&body).unwrap();
    assert_eq!(response.answers[0].selected, Some(2));
    assert_eq!(
        response.answers[1].free_text.as_deref(),
        Some("five, then stop"),
    );
}

/// A grill loop is rounds: the session asks, the human answers, the session asks
/// again with the answers in hand. Every round is on the one Timeline, which is
/// what makes the Conversation readable as a conversation.
#[tokio::test]
async fn a_whole_loop_of_rounds_lands_on_the_one_timeline() {
    let (_dir, _pool, app, first, _second) = two_conversations().await;

    let mut asked_ids = Vec::new();

    for round in [
        "Retry policy for the outbound queue",
        "Where the counter goes",
    ] {
        let id = asked(
            &app,
            first,
            &SET.replace("Retry policy for the outbound queue", round),
        )
        .await;

        assert_eq!(
            answer::<Submitted>(&app, id, decided()).await,
            Submitted::Accepted
        );
        asked_ids.push(id);
    }

    let view = view(&app, first).await;
    let asked = sets(&view);

    assert_eq!(
        asked.iter().map(|asked| asked.set_id).collect::<Vec<_>>(),
        asked_ids,
        "both rounds are on the Timeline, in the order they were asked"
    );
    assert!(
        asked
            .iter()
            .all(|asked| matches!(asked.standing, verkstead_render::Standing::Answered(_))),
        "and both have been answered"
    );
}

/// Both kinds of ask land on the same Timeline and are answered from the same
/// page. What tells them apart is the standing on the one still waiting: nobody
/// is on the other end of a Deferred Ask, and reading it as an agent that had
/// disconnected would be reporting a failure where there is none.
#[tokio::test]
async fn a_deferred_set_says_so_where_a_blocking_one_says_who_is_waiting() {
    let (_dir, _pool, app, first, _second) = two_conversations().await;

    let blocking = asked(&app, first, SET).await;
    let deferred = deferred(&app, first, &retitled("Wording of the retry log line")).await;

    let view = view(&app, first).await;
    let standing = |set_id: i64| {
        sets(&view)
            .into_iter()
            .find(|asked| asked.set_id == set_id)
            .map(|asked| asked.standing.clone())
            .expect("the Set is on the Timeline it was asked from")
    };

    assert_eq!(
        standing(blocking),
        Standing::Waiting(Liveness::Waiting),
        "the blocking reading, which is the registry of held waits speaking: a \
         Set nobody has had time to walk away from",
    );
    assert_eq!(
        standing(deferred),
        Standing::Waiting(Liveness::Deferred),
        "and no wait was ever held on this one, by design",
    );
}

/// Both are something to answer, so both leave the Conversation *blocked on
/// you*: the human is the one being waited on either way, and the sidebar says
/// only that.
#[tokio::test]
async fn a_deferred_set_leaves_the_conversation_waiting_on_the_human() {
    let (_dir, pool, app, first, _second) = two_conversations().await;

    // Out of Draft, which is the state the badge is never drawn on: a draft is
    // drawn as a draft, and nothing has been asked from one.
    store::set_state(&pool, first, store::Lifecycle::Grilling)
        .await
        .unwrap();

    assert!(!waiting(&app, first).await, "nothing has been asked yet");

    let deferred = deferred(&app, first, SET).await;
    assert!(
        waiting(&app, first).await,
        "an unanswered Deferred Ask is an unanswered question to the human",
    );

    assert_eq!(
        answer::<Submitted>(&app, deferred, decided()).await,
        Submitted::Accepted,
        "and it is answered through the one route a Set is answered by",
    );
    assert!(
        !waiting(&app, first).await,
        "which is the whole of what it was waiting for",
    );
}
