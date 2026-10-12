//! The notice stream: every notice this device pushes to its phones, told to the
//! desktop app on the same machine as it is pushed.
//!
//! Read off the wire, the way the app reads it, and set beside what a real push
//! service took for a subscribed phone: what has to hold is that the app is told
//! everything a phone is, in the same words, and is told it with no phone
//! subscribed at all — which is the install the stream is for. A member's news
//! and a Conversation reaching Done are asked in `tests/news.rs`, which stands up
//! the cluster they need.

use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::Router;
use axum::body::{Body, Bytes};
use axum::extract::{Path as UrlPath, State};
use axum::http::{Request, StatusCode, header};
use axum::routing::post;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use http_body_util::BodyExt;
use p256::SecretKey;
use p256::elliptic_curve::rand_core::OsRng;
use p256::elliptic_curve::sec1::ToEncodedPoint;
use sqlx::SqlitePool;
use tower::ServiceExt;
use verkstead_schema::SetCreated;
use verkstead_server::{open_database, router, store};
use web_push_native::Auth;

/// The device every Conversation started here is ranked by, named the way a
/// cluster names one (ADR-0020, *Ranks*).
const THIS_DEVICE: &str = "aa00bb11cc22dd33ee44ff5566778899";

/// The Conversation every Set in this file is asked from, made by [`fresh_app`]
/// over a database with nothing in it.
const ASKING_FROM: i64 = 1;

const SET: &str = r#"
title: Rate limiting for the public API
project: verkstead
branch: desktop-notices
questions:
  - label: Q1
    text: Where should the request counter live?
"#;

/// How long a test will wait for something that happens behind the response:
/// generous, because it is only ever paid when the assertion is about to fail.
const PATIENCE: Duration = Duration::from_secs(5);

/// How long to keep listening after everything expected has arrived, to catch a
/// notice that should not have been sent at all.
const SETTLING: Duration = Duration::from_millis(300);

/// One router over a fresh database, shared by every request in a test.
async fn fresh_app() -> (tempfile::TempDir, SqlitePool, Router) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    let repo = store::register_repo(&pool, Path::new("/srv/verkstead"), "verkstead", "main")
        .await
        .unwrap()
        .expect("nothing is registered at that path yet");

    let conversation = store::start_conversation(&pool, repo.id, "notices", THIS_DEVICE)
        .await
        .unwrap()
        .expect("the Repo was just registered");
    assert_eq!(conversation, ASKING_FROM);

    let app = router(pool.clone());
    (dir, pool, app)
}

/// Send a Set the way the CLI does.
async fn post_set(app: &Router) -> SetCreated {
    let http = app
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

    assert_eq!(http.status(), StatusCode::CREATED);
    let body = http.into_body().collect().await.unwrap().to_bytes();
    serde_saphyr::from_str(std::str::from_utf8(&body).unwrap()).unwrap()
}

/// The desktop app, holding the notice stream open.
struct Holding {
    body: Body,

    /// What has been read off the stream and is not a whole frame yet. SSE
    /// frames are not the chunks they arrive in.
    buffered: String,
}

impl Holding {
    /// Open the stream the way the app does. Returns once the response is in
    /// hand, which is after the handler has subscribed — so anything the test
    /// does next is something the app is listening for.
    async fn open(app: &Router) -> Holding {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/ui/push/notices")
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

        Holding {
            body: response.into_body(),
            buffered: String::new(),
        }
    }

    /// The next notice off the stream, skipping keep-alive comments, or `None`
    /// where none arrives within `waiting`.
    async fn within(&mut self, waiting: Duration) -> Option<serde_json::Value> {
        tokio::time::timeout(waiting, async {
            loop {
                if let Some(end) = self.buffered.find("\n\n") {
                    let frame: String = self.buffered.drain(..end + 2).collect();

                    if frame.lines().any(|line| line == "event: notice") {
                        let data = frame
                            .lines()
                            .find_map(|line| line.strip_prefix("data: "))
                            .expect("a notice carries data");

                        return serde_json::from_str(data).expect("the notice has to be JSON");
                    }

                    continue;
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
        })
        .await
        .ok()
    }

    async fn notice(&mut self) -> serde_json::Value {
        self.within(PATIENCE)
            .await
            .expect("no notice came down the stream")
    }
}

/// One push, as the push service received it.
#[derive(Debug, Clone)]
struct Received {
    phone: String,
    body: Vec<u8>,
}

/// A phone as its browser would have described it, plus the private half the
/// browser would have kept — which is what lets a test read a push back.
struct Phone {
    name: String,
    endpoint: String,
    secret: SecretKey,
    auth: Vec<u8>,
}

impl Phone {
    fn new(service: &str, name: &str) -> Phone {
        Phone {
            name: name.to_owned(),
            endpoint: format!("{service}/{name}"),
            secret: SecretKey::random(&mut OsRng),
            auth: name.bytes().cycle().take(16).collect(),
        }
    }

    /// Put it on the list, the way the page does.
    async fn subscribe(&self, app: &Router) {
        let args = serde_json::json!({
            "endpoint": self.endpoint,
            "p256dh": URL_SAFE_NO_PAD
                .encode(self.secret.public_key().to_encoded_point(false).as_bytes()),
            "auth": URL_SAFE_NO_PAD.encode(&self.auth),
        });

        let http = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/ui/push/subscribe")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(serde_json::to_vec(&args).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(http.status(), StatusCode::OK, "subscribing {}", self.name);
    }

    /// What this phone was shown, as its service worker would read it.
    fn read(&self, push: &Received) -> serde_json::Value {
        let plain = web_push_native::decrypt(
            push.body.clone(),
            &self.secret,
            &Auth::clone_from_slice(&self.auth),
        )
        .expect("a push for this phone has to decrypt with this phone's keys");

        serde_json::from_slice(&plain).expect("the notice has to be JSON")
    }
}

/// A push service on a loopback port that takes everything.
async fn push_service() -> (String, Arc<Mutex<Vec<Received>>>) {
    let received = Arc::new(Mutex::new(Vec::new()));

    let app = Router::new()
        .route("/{phone}", post(take))
        .with_state(received.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    (format!("http://{address}"), received)
}

async fn take(
    State(received): State<Arc<Mutex<Vec<Received>>>>,
    UrlPath(phone): UrlPath<String>,
    body: Bytes,
) -> StatusCode {
    received.lock().unwrap().push(Received {
        phone,
        body: body.to_vec(),
    });

    StatusCode::CREATED
}

/// Wait until the push service has taken `count` pushes, then hand them over.
async fn pushes(received: &Arc<Mutex<Vec<Received>>>, count: usize) -> Vec<Received> {
    let deadline = tokio::time::Instant::now() + PATIENCE;

    loop {
        {
            let taken = received.lock().unwrap();
            if taken.len() >= count {
                return taken.clone();
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "waited for {count} pushes and {} arrived",
                taken.len(),
            );
        }

        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

/// A Set asked while the app holds the stream is one notice, carrying the
/// Timeline Event the Set is read on, its title and its project — with no phone
/// subscribed anywhere.
#[tokio::test]
async fn a_set_asked_is_one_notice_on_the_stream() {
    let (_dir, pool, app) = fresh_app().await;
    let mut holding = Holding::open(&app).await;

    let created = post_set(&app).await;

    let (conversation, event) = store::opened_at(&pool, created.id)
        .await
        .expect("the store answers")
        .expect("a stored Set is on a Timeline");

    assert_eq!(
        holding.notice().await,
        serde_json::json!({
            "path": format!("/conversations/{conversation}/events/{event}"),
            "title": "Rate limiting for the public API",
            "project": "verkstead",
        }),
    );

    assert_eq!(holding.within(SETTLING).await, None, "one Set, one notice",);
}

/// What a phone is shown is what the stream was told, word for word.
///
/// **The two carriers are one announcement**: the stream is told in the same
/// place the phones are, before they are looked for, so nothing reaches a phone
/// that did not reach the app.
#[tokio::test]
async fn a_phone_is_shown_nothing_the_stream_was_not() {
    let (service, received) = push_service().await;
    let (_dir, _pool, app) = fresh_app().await;

    let phone = Phone::new(&service, "phone");
    phone.subscribe(&app).await;

    let mut holding = Holding::open(&app).await;

    post_set(&app).await;
    post_set(&app).await;

    let taken = pushes(&received, 2).await;
    let streamed = [holding.notice().await, holding.notice().await];

    for push in &taken {
        assert_eq!(push.phone, phone.name);
        assert!(
            streamed.contains(&phone.read(push)),
            "the phone was shown {:?}, which never came down the stream: {streamed:?}",
            phone.read(push),
        );
    }

    assert_eq!(
        holding.within(SETTLING).await,
        None,
        "and the stream was told nothing the phone was not",
    );
}

/// A stream nobody holds costs nothing: a Set asked before any app listens is
/// not queued for the app that opens the stream afterwards.
#[tokio::test]
async fn a_notice_nobody_was_holding_the_stream_for_is_not_kept() {
    let (_dir, _pool, app) = fresh_app().await;

    post_set(&app).await;
    tokio::time::sleep(SETTLING).await;

    let mut holding = Holding::open(&app).await;

    assert_eq!(holding.within(SETTLING).await, None);
}
