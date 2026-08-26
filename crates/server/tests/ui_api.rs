//! The viewer's own namespace: what the human's browser asks of the server, over
//! JSON, under `/api/ui/`.
//!
//! What is worth proving here is that this namespace is a way through to the
//! same store and the same held waits the agents' half uses — that a submit from
//! the browser ends a wait an agent is genuinely holding, and not merely that a
//! row appeared. The rendering it answers with is `ui_content.rs`'s subject, and
//! the two things under the same prefix that answer out of neither the store
//! nor the waits are their own files': the Nudge stream, which is listened on
//! rather than asked, is `nudges.rs`'s, and `/api/ui/update` is `updates.rs`'s.

use std::path::Path;
use std::time::{Duration, Instant};

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use serde::de::DeserializeOwned;
use sqlx::SqlitePool;
use tower::ServiceExt;
use verkstead_render::{
    ConversationView, Locked, PushKey, QuestionSetEvent, SetReading, SetView, Standing, Submitted,
    Subscribed, Subscription, TimelineEvent,
};
use verkstead_schema::{Answer, ApiError, Liveness, QuestionSet, Response, SetCreated};
use verkstead_server::{open_database, router, store};

/// Two Questions, one with Sub-questions, so a Response has to account for
/// `Q1`, `Q2`, `Q2a` and `Q2b`.
const SET: &str = r#"
title: Rate limiting for the public API
project: verkstead
branch: solid-viewer
questions:
  - label: Q1
    text: Where should the request counter live?
    options:
      - n: 1
        text: In-process, per instance.
      - n: 2
        text: In Redis, shared across instances.
        recommended: true
  - label: Q2
    text: How should a throttled client be told to back off?
    subquestions:
      - letter: a
        text: What should Retry-After say?
        options:
          - n: 1
            text: The exact number of seconds.
      - letter: b
        text: Anything else about the headers?
"#;

/// Everything in [`SET`] there is to answer, in the order a Response has to
/// account for it. `Q2` is not among them: it heads its Sub-questions and asks
/// nothing of its own, so no entry comes back for it.
const QUESTIONS: [&str; 3] = ["Q1", "Q2a", "Q2b"];

/// The Conversation every Set in this file is asked from.
///
/// Every Set is asked from one — that is what the base URL a session is given
/// says — so a test that wants a Set needs somewhere for it to land. [`fresh_app`]
/// makes it over a database with nothing in it, so it is always the first
/// Conversation there is, which is what lets the helpers below name it without
/// threading it through every test. What it is about matters to nothing here.
const ASKING_FROM: i64 = 1;

/// One router over a fresh database, shared by every request in a test: a wait
/// held through one clone has to hear a submit made through another, which is
/// the whole point of the two namespaces sharing their state.
async fn fresh_app() -> (tempfile::TempDir, SqlitePool, Router) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    let repo = store::register_repo(&pool, Path::new("/srv/verkstead"), "verkstead", "main")
        .await
        .unwrap()
        .expect("nothing is registered at that path yet");

    let conversation = store::start_conversation(&pool, repo.id, "solid-viewer")
        .await
        .unwrap()
        .expect("the Repo was just registered");
    assert_eq!(conversation, ASKING_FROM);

    (dir, pool.clone(), router(pool))
}

async fn body_text(response: axum::response::Response) -> String {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    String::from_utf8(bytes.to_vec()).unwrap()
}

/// Ask the viewer's namespace for something, and read back what it answered.
async fn get<T: DeserializeOwned>(app: &Router, path: &str) -> T {
    let (status, body) = fetch(
        app,
        Request::builder().uri(path).body(Body::empty()).unwrap(),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "GET {path} failed: {body}");
    read(&body)
}

/// Tell the viewer's namespace something, in the JSON a browser would send.
async fn post<T: DeserializeOwned>(app: &Router, path: &str, body: serde_json::Value) -> T {
    let (status, body) = fetch(
        app,
        Request::builder()
            .method("POST")
            .uri(path)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(serde_json::to_vec(&body).unwrap()))
            .unwrap(),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "POST {path} failed: {body}");
    read(&body)
}

async fn fetch(app: &Router, request: Request<Body>) -> (StatusCode, String) {
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    (status, body_text(response).await)
}

fn read<T: DeserializeOwned>(body: &str) -> T {
    serde_json::from_str(body).unwrap_or_else(|err| panic!("reading {body:?}: {err}"))
}

/// One Set as the viewer reads it, insisting this build can still read the
/// stored body.
///
/// Every Set here was written by this build a moment ago, so a reading that
/// cannot be rendered is a broken test rather than the case
/// `/api/ui/sets/{id}` says the other half of — see `ui_content.rs`, which is
/// where the unreadable half is asked about.
async fn get_set(app: &Router, id: i64) -> SetView {
    let reading: SetReading = get(app, &format!("/api/ui/sets/{id}")).await;

    match reading {
        SetReading::Set(view) => *view,
        SetReading::Unreadable(unreadable) => {
            panic!("Set {id} came back unreadable: {}", unreadable.why)
        }
    }
}

/// Send a Set the way the CLI does, and return the id the agent then waits on.
async fn post_set(app: &Router, yaml: &str) -> i64 {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/conversations/{ASKING_FROM}/api/v1/sets"))
                .header(header::CONTENT_TYPE, "application/yaml")
                .body(Body::from(yaml.to_owned()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    let created: SetCreated = serde_saphyr::from_str(&body_text(response).await).unwrap();
    created.id
}

/// Open a wait on a Set the way the CLI does, held for `hold` seconds.
async fn wait_for_response(app: &Router, id: i64, hold: u64) -> axum::response::Response {
    app.clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/conversations/{ASKING_FROM}/api/v1/sets/{id}/response?hold={hold}"
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
}

/// Hold a wait on a Set the way the CLI does, on a task the test can drop.
fn hold_a_wait(app: &Router, id: i64) -> tokio::task::JoinHandle<()> {
    let app = app.clone();

    tokio::spawn(async move {
        let _held = wait_for_response(&app, id, 60).await;
    })
}

/// The Question Sets on [`ASKING_FROM`]'s Timeline, oldest first.
///
/// The one way there is to reach a Set from the viewer now: a Set belongs to the
/// Conversation it was asked from, and the standalone lists that used to be the
/// second way in have gone.
async fn asked_sets(app: &Router) -> Vec<QuestionSetEvent> {
    let view: ConversationView = get(app, &format!("/api/ui/conversations/{ASKING_FROM}")).await;

    view.timeline
        .into_iter()
        .filter_map(|event| match event {
            TimelineEvent::QuestionSet(asked) => Some(asked),
            _ => None,
        })
        .collect()
}

/// The Timeline, asked for until the Set `id` on it reads as `liveness`.
///
/// A wait takes its slot as its handler starts running, which is a moment after
/// the request opening it goes in — so the Timeline is asked again rather than
/// once after a guessed pause.
async fn asked_where(app: &Router, id: i64, liveness: Liveness) -> Vec<QuestionSetEvent> {
    let deadline = Instant::now() + Duration::from_secs(5);

    loop {
        let asked = asked_sets(app).await;
        if asked
            .iter()
            .any(|row| row.set_id == id && row.standing == Standing::Waiting(liveness))
        {
            return asked;
        }

        assert!(
            Instant::now() < deadline,
            "waited for Set {id} to read as {liveness:?} in vain: {asked:?}"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

fn answered(label: &str, selected: Option<u32>) -> Answer {
    Answer {
        label: label.to_owned(),
        selected,
        free_text: None,
        unanswered: false,
    }
}

fn left_open(label: &str) -> Answer {
    Answer {
        label: label.to_owned(),
        selected: None,
        free_text: None,
        unanswered: true,
    }
}

/// What the viewer builds when the human answers `Q1` and confirms the warning
/// about the three they left alone.
fn confirmed_with_three_left_open() -> Response {
    Response {
        answers: vec![answered("Q1", Some(2)), left_open("Q2a"), left_open("Q2b")],
        comment: None,
        direction: None,
    }
}

/// A Response resolving every question of [`SET`], for the tests that only need
/// a Set to be answered.
fn decided() -> Response {
    Response {
        answers: QUESTIONS.iter().map(|label| left_open(label)).collect(),
        comment: Some("Neither — why is this not just a cache in front?".to_owned()),
        direction: None,
    }
}

/// Put a Set on [`ASKING_FROM`]'s Timeline, which is the one way there is to
/// store one — the endpoint above is the same thing with a router in front.
async fn asked(pool: &SqlitePool, set: &QuestionSet) -> anyhow::Result<SetCreated> {
    Ok(store::ask(pool, ASKING_FROM, set, store::Ask::Blocking)
        .await?
        .expect("the Conversation is there to ask from"))
}

/// A Set with nothing but a title, for the lists, which never look inside one.
fn bare(title: &str) -> QuestionSet {
    QuestionSet {
        title: title.to_owned(),
        preface: None,
        questions: Vec::new(),
        postscript: None,
        proposal: None,
        review: None,
        project: Some("verkstead".to_owned()),
        branch: Some("solid-viewer".to_owned()),
        diff: None,
    }
}

#[tokio::test]
async fn every_waiting_set_is_on_its_conversations_timeline_with_its_liveness() {
    let (_dir, pool, app) = fresh_app().await;
    for title in ["the older ask", "the newer ask"] {
        asked(&pool, &bare(title)).await.unwrap();
    }

    let asked = asked_sets(&app).await;

    let titles: Vec<&str> = asked.iter().map(|row| row.title.as_str()).collect();
    assert_eq!(
        titles,
        ["the older ask", "the newer ask"],
        "oldest first, which is reading order: a Timeline is read down, where a \
         list was scanned from the top",
    );

    // The badge arrives decided rather than as a timestamp: this is the side
    // with the registry of held waits. A Set this new is one whose agent is on
    // its way to its first wait.
    assert_eq!(asked[0].standing, Standing::Waiting(Liveness::Waiting));
}

#[tokio::test]
async fn a_set_nothing_has_waited_on_for_long_enough_reads_as_disconnected() {
    let (_dir, pool, app) = fresh_app().await;
    let stored = asked(&pool, &bare("nobody is listening")).await.unwrap();
    // Old enough that the window measured from its creation has closed, and no
    // wait was ever held on it.
    backdate_created(&pool, stored.id, "2026-08-03T09:00:00.000Z").await;

    let asked = asked_sets(&app).await;

    assert_eq!(
        asked[0].standing,
        Standing::Waiting(Liveness::Disconnected),
        "a Set nothing is waiting on is still waiting on the human",
    );
}

#[tokio::test]
async fn the_badge_is_the_wait_the_agents_half_is_genuinely_holding() {
    let (_dir, pool, app) = fresh_app().await;
    let waited_on = post_set(&app, SET).await;
    let orphan = asked(&pool, &bare("the one whose agent went"))
        .await
        .unwrap()
        .id;
    // Both old enough that the window measured from their creation has closed, so
    // the only thing that can make either of them read as waiting is a wait.
    for id in [waited_on, orphan] {
        backdate_created(&pool, id, "2026-08-03T09:00:00.000Z").await;
    }

    let agent = hold_a_wait(&app, waited_on);
    let asked = asked_where(&app, waited_on, Liveness::Waiting).await;

    // And a wait held on one Set says nothing about another: the registry is
    // asked per Set, not read as "somebody is about".
    let other = asked
        .iter()
        .find(|row| row.set_id == orphan)
        .expect("the Set nothing is waiting on is still on the Timeline");
    assert_eq!(
        other.standing,
        Standing::Waiting(Liveness::Disconnected),
        "the Set nothing is waiting on keeps its own badge: {asked:?}"
    );

    // The set view is fed from the same registry, because the two badges are one
    // fact: the human locks a Set from its own page, with the badge in view.
    let set = get_set(&app, waited_on).await;
    assert_eq!(set.standing, Standing::Waiting(Liveness::Waiting));
    let orphaned = get_set(&app, orphan).await;
    assert_eq!(
        orphaned.standing,
        Standing::Waiting(Liveness::Disconnected),
        "a Set nothing is waiting on is still waiting on the human"
    );

    agent.abort();
}

#[tokio::test]
async fn a_disconnected_set_is_still_answerable_from_the_viewer() {
    let (_dir, pool, app) = fresh_app().await;
    let id = post_set(&app, SET).await;
    backdate_created(&pool, id, "2026-08-03T09:00:00.000Z").await;

    let asked = asked_sets(&app).await;
    assert_eq!(asked[0].standing, Standing::Waiting(Liveness::Disconnected));

    // Display state only (ADR-0001): the badge never gates an answer, and a Set
    // whose agent has gone is neither withdrawn nor closed on its own.
    let outcome: Submitted = post(
        &app,
        &format!("/api/ui/sets/{id}/response"),
        serde_json::to_value(decided()).unwrap(),
    )
    .await;
    assert_eq!(outcome, Submitted::Accepted);

    let asked = asked_sets(&app).await;
    assert!(
        matches!(asked[0].standing, Standing::Answered(_)),
        "the answered Set should read as the decision it now is: {asked:?}"
    );
}

#[tokio::test]
async fn an_answered_set_reads_as_a_decision_while_the_rest_still_wait() {
    let (_dir, pool, app) = fresh_app().await;
    let answered = asked(&pool, &bare("already answered")).await.unwrap();
    asked(&pool, &bare("still waiting")).await.unwrap();
    store::insert_response(&pool, answered.id, &Response::default())
        .await
        .unwrap()
        .expect("the Set had no Response yet");

    let asked = asked_sets(&app).await;

    let standings: Vec<(&str, bool)> = asked
        .iter()
        .map(|row| {
            (
                row.title.as_str(),
                matches!(row.standing, Standing::Answered(_)),
            )
        })
        .collect();
    assert_eq!(
        standings,
        [("already answered", true), ("still waiting", false)],
        "a settled Set stays where it was asked, saying what became of it: \
         nothing leaves a Timeline",
    );
}

#[tokio::test]
async fn the_timeline_tells_a_decision_from_a_set_nobody_ever_answered() {
    let (_dir, pool, app) = fresh_app().await;

    let decided_set = asked(&pool, &bare("decided")).await.unwrap();
    store::insert_response(&pool, decided_set.id, &Response::default())
        .await
        .unwrap()
        .unwrap();
    settle_at(&pool, decided_set.id, "2025-08-03T09:07:00.000Z").await;

    let orphan = asked(&pool, &bare("nobody ever answered")).await.unwrap();
    store::lock_set(&pool, &store::Settlements::new(1), orphan.id)
        .await
        .unwrap();

    let asked = asked_sets(&app).await;

    // The two kinds of settling are not the same thing to read back — one is a
    // decision that was made, the other is a Set nobody answered — and each
    // carries the moment it was settled. The stamp travels raw: this is the one
    // time on this wire the viewer words itself, because it belongs to the
    // Standing rather than to the Event.
    let Standing::Answered(answered) = &asked[0].standing else {
        panic!("expected a decision, got {:?}", asked[0].standing);
    };
    assert_eq!(answered.submitted_at, "2025-08-03T09:07:00.000Z");

    let Standing::LockedUnanswered(closed_at) = &asked[1].standing else {
        panic!(
            "expected a Set closed unanswered, got {:?}",
            asked[1].standing
        );
    };
    assert!(!closed_at.is_empty(), "expected when it was closed");
}

#[tokio::test]
async fn a_set_carries_where_it_stands_along_with_its_own_material() {
    let (_dir, _pool, app) = fresh_app().await;
    let id = post_set(&app, SET).await;

    let set = get_set(&app, id).await;

    assert_eq!(set.id, id);
    assert_eq!(set.title, "Rate limiting for the public API");
    assert_eq!(set.questions.len(), 2);
    // Which view the human gets turns on this, so it travels with the Set
    // rather than being asked for once the page is already up.
    assert_eq!(set.standing, Standing::Waiting(Liveness::Waiting));
}

#[tokio::test]
async fn a_set_that_does_not_exist_says_so_however_it_was_asked_for() {
    let (_dir, _pool, app) = fresh_app().await;

    // A number that names no Set, and an id that could never name one: the
    // viewer's URLs are typed by hand as often as they are followed.
    for path in ["/api/ui/sets/404", "/api/ui/sets/not-a-number"] {
        let (status, body) = fetch(
            &app,
            Request::builder().uri(path).body(Body::empty()).unwrap(),
        )
        .await;

        assert_eq!(status, StatusCode::NOT_FOUND, "{path} answered {body}");
        let refused: ApiError = read(&body);
        assert!(
            refused.error.contains("no Question Set"),
            "expected a refusal that says what is missing, got {:?}",
            refused.error
        );
    }
}

#[tokio::test]
async fn a_submit_from_the_viewer_settles_a_wait_that_is_genuinely_being_held() {
    let (_dir, _pool, app) = fresh_app().await;
    let id = post_set(&app, SET).await;

    // The agent's wait goes up first and finds nothing, so the only thing that
    // can end it is word of the submit — not a row it happened to read.
    let agent = tokio::spawn({
        let app = app.clone();
        async move { wait_for_response(&app, id, 30).await }
    });
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert!(
        !agent.is_finished(),
        "the wait should still be held: nothing has answered the Set yet",
    );

    let response = confirmed_with_three_left_open();
    let submitted = Instant::now();
    let outcome: Submitted = post(
        &app,
        &format!("/api/ui/sets/{id}/response"),
        serde_json::to_value(&response).unwrap(),
    )
    .await;
    assert_eq!(outcome, Submitted::Accepted);

    let waited = agent.await.unwrap();
    let woken_in = submitted.elapsed();

    assert_eq!(waited.status(), StatusCode::OK);
    assert!(
        woken_in < Duration::from_secs(5),
        "the wait should have been woken by the submit, not left to time out; \
         it took {woken_in:?} of its 30s hold",
    );

    // And what the agent gets is the Response the human confirmed, unanswered
    // markers and all.
    assert_eq!(
        Response::from_yaml(&body_text(waited).await).unwrap(),
        response
    );
}

#[tokio::test]
async fn an_answered_set_reads_back_as_the_decision_that_was_made() {
    let (_dir, _pool, app) = fresh_app().await;
    let id = post_set(&app, SET).await;
    let response = decided();

    let outcome: Submitted = post(
        &app,
        &format!("/api/ui/sets/{id}/response"),
        serde_json::to_value(&response).unwrap(),
    )
    .await;
    assert_eq!(outcome, Submitted::Accepted);

    let set = get_set(&app, id).await;
    let Standing::Answered(answered) = set.standing else {
        panic!("expected an answered Set, got {:?}", set.standing);
    };
    assert_eq!(answered.response, response);
    assert!(
        !answered.submitted_at.is_empty(),
        "expected the time the Response landed"
    );
}

#[tokio::test]
async fn a_second_submit_is_told_the_set_was_already_answered() {
    let (_dir, _pool, app) = fresh_app().await;
    let id = post_set(&app, SET).await;
    let body = serde_json::to_value(decided()).unwrap();

    let first: Submitted = post(&app, &format!("/api/ui/sets/{id}/response"), body.clone()).await;
    let second: Submitted = post(&app, &format!("/api/ui/sets/{id}/response"), body).await;

    assert_eq!(first, Submitted::Accepted);
    assert_eq!(second, Submitted::AlreadyAnswered);
}

#[tokio::test]
async fn a_response_that_misses_a_question_comes_back_naming_it() {
    let (_dir, _pool, app) = fresh_app().await;
    let id = post_set(&app, SET).await;

    // Q2b is unaccounted for, which the viewer should never let happen — so it
    // is carried back and shown rather than swallowed.
    let short = Response {
        answers: vec![answered("Q1", Some(1)), left_open("Q2a")],
        comment: None,
        direction: None,
    };

    let outcome: Submitted = post(
        &app,
        &format!("/api/ui/sets/{id}/response"),
        serde_json::to_value(&short).unwrap(),
    )
    .await;

    let Submitted::Rejected(violations) = outcome else {
        panic!("expected the Response refused, got {outcome:?}");
    };
    assert!(
        violations.iter().any(|said| said.contains("Q2b")),
        "expected the missed question named, got {violations:?}"
    );
}

#[tokio::test]
async fn a_submit_to_a_set_that_is_gone_says_which_way_it_went() {
    let (_dir, _pool, app) = fresh_app().await;
    let id = post_set(&app, SET).await;
    let body = serde_json::to_value(decided()).unwrap();

    // Locked first, which closes it for good: a Response to one cannot make
    // it an answered Set instead.
    let locked: Locked = post(&app, &format!("/api/ui/sets/{id}/lock"), body.clone()).await;
    assert_eq!(locked, Locked::Closed);

    let refused: Submitted = post(&app, &format!("/api/ui/sets/{id}/response"), body.clone()).await;
    assert_eq!(refused, Submitted::Locked);

    let missing: Submitted = post(&app, "/api/ui/sets/9999/response", body).await;
    assert_eq!(missing, Submitted::NoSuchSet);
}

#[tokio::test]
async fn locking_a_set_ends_a_wait_held_on_it_and_files_it_unanswered() {
    let (_dir, _pool, app) = fresh_app().await;
    let id = post_set(&app, SET).await;

    let agent = tokio::spawn({
        let app = app.clone();
        async move { wait_for_response(&app, id, 30).await }
    });
    tokio::time::sleep(Duration::from_millis(200)).await;

    let outcome: Locked = post(
        &app,
        &format!("/api/ui/sets/{id}/lock"),
        serde_json::json!({}),
    )
    .await;
    assert_eq!(outcome, Locked::Closed);

    // Nothing is ever coming, and the CLI is told so rather than left polling.
    assert_eq!(agent.await.unwrap().status(), StatusCode::GONE);

    let asked = asked_sets(&app).await;
    assert!(
        matches!(asked[0].standing, Standing::LockedUnanswered(_)),
        "the Timeline stops claiming the Set is waiting, and says it was never \
         answered rather than showing a decision: {asked:?}"
    );

    let set = get_set(&app, id).await;
    assert!(
        matches!(set.standing, Standing::LockedUnanswered(_)),
        "expected the Set closed unanswered, got {:?}",
        set.standing
    );
}

#[tokio::test]
async fn locking_says_what_it_found_when_the_set_had_already_gone() {
    let (_dir, _pool, app) = fresh_app().await;
    let answered_set = post_set(&app, SET).await;
    post::<Submitted>(
        &app,
        &format!("/api/ui/sets/{answered_set}/response"),
        serde_json::to_value(decided()).unwrap(),
    )
    .await;

    let orphan = post_set(&app, SET).await;
    let empty = serde_json::json!({});
    post::<Locked>(&app, &format!("/api/ui/sets/{orphan}/lock"), empty.clone()).await;

    // A decision is not something to close, and one closing is enough.
    for (id, expected) in [
        (answered_set, Locked::AlreadyAnswered),
        (orphan, Locked::AlreadyLocked),
        (9999, Locked::NoSuchSet),
    ] {
        let outcome: Locked = post(&app, &format!("/api/ui/sets/{id}/lock"), empty.clone()).await;
        assert_eq!(outcome, expected, "locking {id} again");
    }
}

#[tokio::test]
async fn a_device_is_handed_the_public_key_and_never_the_private_half() {
    let (_dir, pool, app) = fresh_app().await;
    let keys = store::vapid_keys(&pool).await.unwrap();

    let (status, body) = fetch(
        &app,
        Request::builder()
            .uri("/api/ui/push/key")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let handed: PushKey = read(&body);
    assert_eq!(handed.key, keys.public_key);
    assert!(
        !body.contains(&keys.private_key),
        "the private half must never leave the server: {body}"
    );
}

#[tokio::test]
async fn a_device_asking_to_be_told_is_stored_and_can_ask_to_stop() {
    let (_dir, pool, app) = fresh_app().await;
    let subscription = Subscription {
        endpoint: "https://push.example/abc".to_owned(),
        p256dh: "a-public-key".to_owned(),
        auth: "an-auth-secret".to_owned(),
    };

    let stored: Subscribed = post(
        &app,
        "/api/ui/push/subscribe",
        serde_json::to_value(&subscription).unwrap(),
    )
    .await;
    assert_eq!(stored, Subscribed::Stored);
    assert_eq!(store::push_subscriptions(&pool).await.unwrap().len(), 1);

    // Subscribing again is the same device, not a second one.
    let again: Subscribed = post(
        &app,
        "/api/ui/push/subscribe",
        serde_json::to_value(&subscription).unwrap(),
    )
    .await;
    assert_eq!(again, Subscribed::Stored);
    assert_eq!(store::push_subscriptions(&pool).await.unwrap().len(), 1);

    let (status, body) = fetch(
        &app,
        Request::builder()
            .method("POST")
            .uri("/api/ui/push/unsubscribe")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                serde_json::json!({ "endpoint": subscription.endpoint }).to_string(),
            ))
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT, "{body}");
    assert!(store::push_subscriptions(&pool).await.unwrap().is_empty());
}

#[tokio::test]
async fn a_subscription_with_nothing_to_send_to_is_refused() {
    let (_dir, pool, app) = fresh_app().await;

    let refused: Subscribed = post(
        &app,
        "/api/ui/push/subscribe",
        serde_json::json!({ "endpoint": "", "p256dh": "a-key", "auth": "a-secret" }),
    )
    .await;

    assert_eq!(refused, Subscribed::Incomplete);
    assert!(store::push_subscriptions(&pool).await.unwrap().is_empty());
}

#[tokio::test]
async fn turning_notifications_off_on_a_device_the_server_never_heard_of_is_no_error() {
    let (_dir, _pool, app) = fresh_app().await;

    let (status, body) = fetch(
        &app,
        Request::builder()
            .method("POST")
            .uri("/api/ui/push/unsubscribe")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                serde_json::json!({ "endpoint": "https://push.example/never-stored" }).to_string(),
            ))
            .unwrap(),
    )
    .await;

    // What was asked for holds either way: nothing is sent there.
    assert_eq!(status, StatusCode::NO_CONTENT, "{body}");
}

#[tokio::test]
async fn the_agents_contract_still_speaks_its_own_language() {
    let (_dir, _pool, app) = fresh_app().await;
    let id = post_set(&app, SET).await;

    // The viewer's namespace is JSON and the agents' is YAML, over the same
    // store: neither has taken the other's shape.
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/conversations/{ASKING_FROM}/api/v1/sets/{id}/response?hold=0"
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    let health = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(body_text(health).await, "ok");
}

/// Age a Set, so the window Liveness is measured in has closed.
///
/// Both stamps, because there are two and they are one fact: the Set's own
/// creation time, which its page is read against, and the stamp of the Timeline
/// Event it landed on, which its row on that Timeline is. The one transaction
/// that puts a Set to a Conversation writes them together, so a test that moved
/// only one would be asking about an arrangement that cannot happen.
async fn backdate_created(pool: &SqlitePool, id: i64, created_at: &str) {
    sqlx::query("UPDATE question_sets SET created_at = ? WHERE id = ?")
        .bind(created_at)
        .bind(id)
        .execute(pool)
        .await
        .unwrap();

    sqlx::query(
        "UPDATE timeline_events SET at = ?
         WHERE id = (SELECT event_id FROM set_events WHERE set_id = ?)",
    )
    .bind(created_at)
    .bind(id)
    .execute(pool)
    .await
    .unwrap();
}

/// Move when a Response landed, so a settling's date is a test's to decide.
async fn settle_at(pool: &SqlitePool, id: i64, submitted_at: &str) {
    sqlx::query("UPDATE responses SET submitted_at = ? WHERE set_id = ?")
        .bind(submitted_at)
        .bind(id)
        .execute(pool)
        .await
        .unwrap();
}
