//! The Nudge stream: telling the open viewer pages what moved.
//!
//! Read off the wire rather than off the channel behind it, because what has to
//! hold is that a page listening over HTTP hears each change — the three that
//! settle a Set or put a new one to the human, and nothing else. A Nudge now
//! says which kind of thing moved and where (ADR-0009), so the assertions are
//! about *how many* arrived, *when*, and *what each said* — the last of which is
//! the wire shape the viewer's own tests are fed from, below.

use std::path::Path;
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use tower::ServiceExt;
use verkstead_schema::{Nudge, SetCreated};
use verkstead_server::{open_database, router, store};

/// The Conversation every Set in this file is asked from, made by [`fresh_app`]
/// over a database with nothing in it.
const ASKING_FROM: i64 = 1;

const SET: &str = r#"
title: Rate limiting for the public API
project: verkstead
branch: nudge
questions:
  - label: Q1
    text: Where should the request counter live?
    options:
      - n: 1
        text: In-process, per instance.
      - n: 2
        text: In Redis, shared across instances.
"#;

/// A Response resolving every question of [`SET`].
fn decided() -> serde_json::Value {
    serde_json::json!({
        "answers": [{ "label": "Q1", "selected": 2, "unanswered": false }],
    })
}

/// How long a test will wait for a Nudge it expects: generous, because it is
/// only ever paid when the assertion is about to fail.
const PATIENCE: Duration = Duration::from_secs(5);

/// How long to keep listening after everything expected has arrived, to catch a
/// Nudge that should not have been sent at all.
const SETTLING: Duration = Duration::from_millis(300);

/// One router over a fresh database, shared by every request in a test: a page
/// listening through one clone has to hear a Set posted through another.
async fn fresh_app() -> (tempfile::TempDir, Router) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    // Somewhere for the Sets to land: every Set is asked from a Conversation,
    // and a Nudge is what tells an open page that one arrived on a Timeline.
    let repo = store::register_repo(&pool, Path::new("/srv/verkstead"), "verkstead", "main")
        .await
        .unwrap()
        .expect("nothing is registered at that path yet");

    let conversation = store::start_conversation(&pool, repo.id, "nudge")
        .await
        .unwrap()
        .expect("the Repo was just registered");
    assert_eq!(conversation, ASKING_FROM);

    (dir, router(pool))
}

/// An open page, listening on the Nudge stream.
struct Listening {
    body: Body,

    /// What has been read off the stream and is not a whole frame yet. SSE
    /// frames are not the chunks they arrive in.
    buffered: String,
}

impl Listening {
    /// Open the stream the way a page does. Returns once the response is in
    /// hand, which is after the handler has subscribed — so anything the test
    /// does next is something this page is listening for.
    async fn open(app: &Router) -> Self {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/ui/nudges")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get(header::CONTENT_TYPE)
                .and_then(|kind| kind.to_str().ok()),
            Some("text/event-stream"),
        );

        Self {
            body: response.into_body(),
            buffered: String::new(),
        }
    }

    /// The next whole frame off the stream, whatever kind it is.
    async fn frame(&mut self) -> String {
        loop {
            if let Some(end) = self.buffered.find("\n\n") {
                return self.buffered.drain(..end + 2).collect();
            }

            let chunk = self
                .body
                .frame()
                .await
                .expect("the stream ended")
                .unwrap()
                .into_data()
                .expect("the stream carries data frames");

            self.buffered.push_str(std::str::from_utf8(&chunk).unwrap());
        }
    }

    /// The next Nudge, past the keep-alives that are the stream's other
    /// traffic, as the page reads it: the JSON of the `data` line.
    async fn nudge(&mut self) -> Nudge {
        let waited_for = tokio::time::timeout(PATIENCE, async {
            loop {
                let frame = self.frame().await;
                if frame.starts_with("event: nudge") {
                    return said(&frame);
                }
            }
        });

        waited_for.await.expect("waited for a Nudge in vain")
    }

    /// Insist that nothing more is coming.
    async fn nothing_more(&mut self) {
        let arrived = tokio::time::timeout(SETTLING, async {
            loop {
                let frame = self.frame().await;
                if frame.starts_with("event: nudge") {
                    return frame;
                }
            }
        })
        .await;

        assert!(arrived.is_err(), "an unwanted Nudge arrived: {arrived:?}");
    }
}

/// What one SSE frame said, read the way the viewer reads it — off the `data`
/// line and through the type generated from `Nudge`.
///
/// Deserialising rather than comparing text is the whole assertion: a Nudge that
/// came out as something this cannot parse is one no page could act on.
fn said(frame: &str) -> Nudge {
    let data = frame
        .lines()
        .find_map(|line| line.strip_prefix("data: "))
        .unwrap_or_else(|| panic!("a Nudge frame carries a data line, not {frame:?}"));

    serde_json::from_str(data)
        .unwrap_or_else(|error| panic!("a Nudge should be readable as one: {data:?} — {error}"))
}

async fn body_text(response: axum::response::Response) -> String {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    String::from_utf8(bytes.to_vec()).unwrap()
}

/// Send a Set the way the CLI does, and return the id the agent then waits on.
async fn post_set(app: &Router) -> i64 {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/conversations/{ASKING_FROM}/api/v1/sets"))
                .header(header::CONTENT_TYPE, "application/yaml")
                .body(Body::from(SET))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    let created: SetCreated = serde_saphyr::from_str(&body_text(response).await).unwrap();
    created.id
}

/// Tell the viewer's namespace something, the way a browser would.
async fn post(app: &Router, path: &str, body: serde_json::Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(path)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK, "POST {path} failed");
}

#[tokio::test]
async fn a_set_arriving_nudges_the_open_pages() {
    let (_dir, app) = fresh_app().await;
    let mut page = Listening::open(&app).await;

    post_set(&app).await;

    assert_eq!(
        page.nudge().await,
        Nudge::Set {
            conversation: ASKING_FROM
        },
    );
    page.nothing_more().await;
}

#[tokio::test]
async fn a_response_landing_nudges_the_open_pages() {
    let (_dir, app) = fresh_app().await;
    let id = post_set(&app).await;

    // Opened after the Set exists, so the only durable change it can hear about
    // is the answering.
    let mut page = Listening::open(&app).await;

    post(&app, &format!("/api/ui/sets/{id}/response"), decided()).await;

    assert_eq!(
        page.nudge().await,
        Nudge::Set {
            conversation: ASKING_FROM
        },
    );
    page.nothing_more().await;
}

#[tokio::test]
async fn a_response_from_the_agents_half_nudges_the_open_pages_too() {
    let (_dir, app) = fresh_app().await;
    let id = post_set(&app).await;
    let mut page = Listening::open(&app).await;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/conversations/{ASKING_FROM}/api/v1/sets/{id}/response"
                ))
                .header(header::CONTENT_TYPE, "application/yaml")
                .body(Body::from("answers:\n  - label: Q1\n    selected: 2\n"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    assert_eq!(
        page.nudge().await,
        Nudge::Set {
            conversation: ASKING_FROM
        },
    );
    page.nothing_more().await;
}

#[tokio::test]
async fn a_set_archived_unanswered_nudges_the_open_pages() {
    let (_dir, app) = fresh_app().await;
    let id = post_set(&app).await;
    let mut page = Listening::open(&app).await;

    post(
        &app,
        &format!("/api/ui/sets/{id}/archive"),
        serde_json::json!({}),
    )
    .await;

    assert_eq!(
        page.nudge().await,
        Nudge::Set {
            conversation: ASKING_FROM
        },
    );
    page.nothing_more().await;
}

#[tokio::test]
async fn an_agents_wait_opening_and_closing_nudges_nobody() {
    let (_dir, app) = fresh_app().await;
    let id = post_set(&app).await;
    let mut page = Listening::open(&app).await;

    // A whole wait, from the agent taking its slot in the registry to the hold
    // window closing on it: the Liveness badge moves twice and the pending
    // world does not move at all.
    let held = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/conversations/{ASKING_FROM}/api/v1/sets/{id}/response?hold=1"
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(held.status(), StatusCode::NO_CONTENT);

    page.nothing_more().await;
}

#[tokio::test]
async fn every_open_page_hears_every_nudge() {
    let (_dir, app) = fresh_app().await;

    let mut phone = Listening::open(&app).await;
    let mut laptop = Listening::open(&app).await;

    let id = post_set(&app).await;
    post(&app, &format!("/api/ui/sets/{id}/response"), decided()).await;

    // The same two Nudges, saying the same two things, on both connections: the
    // arrival off the Nudges channel and the answering off the store's, which
    // this is what proves come out of the merge as one kind of thing.
    for page in [&mut phone, &mut laptop] {
        let asked = Nudge::Set {
            conversation: ASKING_FROM,
        };

        assert_eq!(page.nudge().await, asked);
        assert_eq!(page.nudge().await, asked);
        page.nothing_more().await;
    }
}

#[tokio::test]
async fn the_stream_says_something_while_nothing_is_happening() {
    let (_dir, app) = fresh_app().await;
    let mut page = Listening::open(&app).await;

    // The keep-alive interval is a quarter-minute of real time, which is a
    // quarter-minute nobody should spend watching a clock: from here the test
    // is on the runtime's clock, which jumps to the next timer the moment
    // there is nothing else to do.
    tokio::time::pause();

    let frame = page.frame().await;

    assert!(
        frame.starts_with(':'),
        "a quiet stream should have carried a keep-alive, not {frame:?}",
    );
}

/// The Conversation the fixture's scoped Nudges point at. Not [`ASKING_FROM`]:
/// what a fixture is for is being read by a test that made its own world, and a
/// number that means something here would read as a coincidence there.
const SCOPED_TO: i64 = 7;

/// Every kind of Nudge, as one array in the order they are declared in.
///
/// One of each rather than one per file: this is a vocabulary rather than a set
/// of payloads, and what a reader of it wants to see is the whole of it at once.
const KINDS: [Nudge; 8] = [
    Nudge::Transcript {
        conversation: SCOPED_TO,
    },
    Nudge::Screen {
        conversation: SCOPED_TO,
    },
    Nudge::Commit {
        conversation: SCOPED_TO,
    },
    Nudge::Set {
        conversation: SCOPED_TO,
    },
    Nudge::Conversation {
        conversation: SCOPED_TO,
    },
    Nudge::Conversations,
    Nudge::Repos,
    Nudge::Profiles,
];

/// Where the golden fixtures are written, relative to this crate — the same
/// directory `ui_content` writes the endpoints' payloads to.
const FIXTURES: &str = "../../web/tests/fixtures";

/// Leave the viewer's own tests one Nudge of every kind, exactly as this server
/// writes one.
///
/// Committed, and rewritten by every run of this test: the diff is the review.
/// The viewer drives its stream off these rather than off a hand-written string,
/// so a kind added on this side that nobody carried across shows up as a failing
/// fixture rather than as a page that quietly stops reacting.
#[test]
fn the_viewers_own_tests_are_fed_from_here() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(FIXTURES);
    std::fs::create_dir_all(&dir).unwrap();

    let mut pretty = serde_json::to_string_pretty(&KINDS).unwrap();
    pretty.push('\n');

    std::fs::write(dir.join("nudges.json"), pretty).unwrap();
}

#[tokio::test]
async fn what_goes_down_the_wire_is_what_the_fixture_says() {
    let (_dir, app) = fresh_app().await;
    let mut page = Listening::open(&app).await;

    post_set(&app).await;

    // The fixture is written from the enum and the stream is written from the
    // same enum, and this is the one place the two are put side by side: a Set
    // arriving is the one kind an integration test can reach the whole way, so
    // it is the one that ties the vocabulary to the wire.
    let scoped = serde_json::to_value(page.nudge().await).unwrap();
    let fixture = serde_json::to_value(Nudge::Set {
        conversation: SCOPED_TO,
    })
    .unwrap();

    assert_eq!(scoped["kind"], fixture["kind"]);
    assert_eq!(scoped["conversation"], serde_json::json!(ASKING_FROM));
}
