//! The Workbench Key and the gate in front of it: what a request that has not
//! shown the key is answered, what a link carrying it does, and what stays open
//! whatever anybody holds.
//!
//! The router here is keyed, which no other suite's is: everything else is stood
//! up over a constructor that answers every request, because what those suites
//! are about is what the answer says rather than who was allowed to ask. This is
//! the one that is about who was allowed to ask — see `router_keyed_with_viewer`
//! — and the site behind it is `tests/site`, the same fixture `viewer.rs` uses,
//! so nothing here waits on `pnpm build`.
//!
//! The other end of this is in `tests/sandbox.rs`, where the same 401 is asked
//! for by a `curl` running inside a real Conversation's sandbox. That is the
//! claim the key exists to make; this file is the shape of it.

use std::path::Path;

use axum::Router;
use axum::body::Body;
use axum::http::header::{COOKIE, LOCATION, SET_COOKIE};
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use sqlx::SqlitePool;
use tower::ServiceExt;
use verkstead_server::key::{WorkbenchKey, login_link};
use verkstead_server::settings::Settings;
use verkstead_server::{Embed, open_database, router_keyed_with_viewer, store};

/// A site shaped like the one vite builds, which is what the workbench's own
/// pages are answered out of.
#[derive(Embed)]
#[folder = "$CARGO_MANIFEST_DIR/tests/site"]
struct Site;

/// One Question Set, so that the Conversation-scoped API can be asked for
/// something real rather than merely asked.
const SET: &str = r#"
title: What a session may reach over the loopback
questions:
  - label: Q1
    text: Is the workbench's own namespace among it?
    options:
      - n: 1
        text: No
        recommended: true
"#;

/// A keyed router over a fresh database, with one Conversation on it for the
/// agents' half to be asked about — and the Data Directory the key was made in,
/// kept alive for as long as the test is.
async fn keyed() -> (tempfile::TempDir, SqlitePool, WorkbenchKey, Router, i64) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    let repo = store::register_repo(&pool, Path::new("/srv/verkstead"), "verkstead", "main")
        .await
        .unwrap()
        .expect("nothing is registered at that path yet");

    let conversation = store::start_conversation(&pool, repo.id, "workbench-key")
        .await
        .unwrap()
        .expect("the Repo was just registered");

    let key = WorkbenchKey::issued(dir.path()).unwrap();
    let app = router_keyed_with_viewer::<Site>(pool.clone(), key.clone());

    (dir, pool, key, app, conversation)
}

/// Ask for something as a browser holding nothing would.
async fn get(app: &Router, path: &str) -> axum::http::Response<Body> {
    app.clone()
        .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
        .await
        .unwrap()
}

/// And as one that has been through the handshake.
async fn get_with_cookie(app: &Router, path: &str, cookie: &str) -> axum::http::Response<Body> {
    app.clone()
        .oneshot(
            Request::builder()
                .uri(path)
                .header(COOKIE, cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
}

fn header_of(response: &axum::http::Response<Body>, name: header::HeaderName) -> String {
    response
        .headers()
        .get(&name)
        .unwrap_or_else(|| panic!("the answer should carry a {name}"))
        .to_str()
        .unwrap()
        .to_owned()
}

async fn text(response: axum::http::Response<Body>) -> String {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    String::from_utf8(bytes.to_vec()).unwrap()
}

#[tokio::test]
async fn nothing_without_the_key_reaches_the_namespace_or_a_page() {
    let (_dir, _pool, _key, app, conversation) = keyed().await;

    // The viewer's own namespace, which is the half a session sharing the
    // host's loopback could otherwise ask everything of.
    for path in ["/api/ui/repos", "/api/ui/settings", "/api/ui/conversations"] {
        let response = get(&app, path).await;

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED, "GET {path}");
    }

    // And the pages of the workbench, which the fallback answers with the
    // document: a session that could read one would have the whole viewer.
    for path in [
        "/",
        "/conversations",
        &format!("/conversations/{conversation}"),
    ] {
        let response = get(&app, path).await;

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED, "GET {path}");
    }
}

#[tokio::test]
async fn the_health_check_is_open() {
    let (_dir, _pool, _key, app, _conversation) = keyed().await;

    // Whether the server is up is not a question about anybody's work — and it
    // is what a health check on the machine asks, holding no cookie at all.
    let response = get(&app, "/api/v1/health").await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(text(response).await, "ok");
}

#[tokio::test]
async fn a_sessions_own_conversation_scoped_api_is_open() {
    let (_dir, pool, _key, app, conversation) = keyed().await;

    // Scoped already: the whole of what a session may ask is under the
    // Conversation it was launched for, and its own Timeline is what a Set on
    // it lands on. A key here would be a key inside every sandbox.
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/conversations/{conversation}/api/v1/sets"))
                .header(header::CONTENT_TYPE, "application/yaml")
                .body(Body::from(SET))
                .unwrap(),
        )
        .await
        .unwrap();

    // The endpoint's own answer — a Set was created — rather than the gate's.
    assert_eq!(response.status(), StatusCode::CREATED);

    let timeline = store::timeline(&pool, conversation).await.unwrap();

    assert!(
        timeline
            .iter()
            .any(|event| matches!(event.event, store::Event::QuestionSet(_))),
        "the Set a session put should be on its Conversation's Timeline",
    );
}

#[tokio::test]
async fn the_key_on_a_link_sets_the_cookie_and_the_query_goes() {
    let (_dir, _pool, key, app, conversation) = keyed().await;

    let path = format!("/conversations/{conversation}");
    let response = get(&app, &format!("{path}?key={}", key.secret())).await;

    assert_eq!(
        response.status(),
        StatusCode::SEE_OTHER,
        "a link carrying the key is a handshake rather than a page",
    );
    assert_eq!(
        header_of(&response, LOCATION),
        path,
        "the secret is off the URL before the page it asked for is drawn",
    );

    let cookie = header_of(&response, SET_COOKIE);

    assert!(
        cookie.contains(&key.secret()),
        "the cookie is what carries the key from here on: got `{cookie}`",
    );
    assert!(
        cookie.contains("HttpOnly"),
        "nothing in the viewer reads it: got `{cookie}`",
    );

    // And the same browser, holding what it was just given, is let in.
    let landed = get_with_cookie(&app, &path, &cookie).await;

    assert_eq!(landed.status(), StatusCode::OK);
    assert!(text(landed).await.contains(r#"<div id="app">"#));
}

#[tokio::test]
async fn what_else_was_on_the_link_survives_the_handshake() {
    let (_dir, _pool, key, app, _conversation) = keyed().await;

    // The parameter is taken off; the rest of the query is where the page was
    // pointed, and a redirect that dropped it would land somewhere else.
    let response = get(&app, &format!("/settings?pane=remote&key={}", key.secret())).await;

    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(header_of(&response, LOCATION), "/settings?pane=remote");
}

#[tokio::test]
async fn a_wrong_key_is_no_better_than_none() {
    let (_dir, _pool, _key, app, _conversation) = keyed().await;

    for asked in [
        "/api/ui/repos?key=not-the-key",
        "/?key=not-the-key",
        "/?key=",
    ] {
        let response = get(&app, asked).await;

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED, "GET {asked}");
        assert!(
            response.headers().get(SET_COOKIE).is_none(),
            "{asked} was refused, so nothing should have been handed over",
        );
    }

    // And a cookie holding the wrong thing is the same refusal: holding one is
    // not what being logged in is.
    let response = get_with_cookie(&app, "/api/ui/repos", "workbench_key=not-the-key").await;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

/// The link the daemon logs and the desktop app opens: the address with the key
/// on it, which is the whole of how a device is first let in (ADR-0015).
#[tokio::test]
async fn the_login_link_is_what_a_browser_follows_in_on() {
    let (_dir, _pool, key, app, _conversation) = keyed().await;

    let link = login_link("127.0.0.1:8422".parse().unwrap(), &key);

    assert_eq!(
        link,
        format!("http://127.0.0.1:8422/?key={}", key.secret()),
        "the workbench's own root, with the key on it",
    );

    // And what following it does: the handshake, and then the workbench drawn
    // for the browser that came out of it holding the cookie.
    let response = get(&app, &format!("/?key={}", key.secret())).await;

    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(header_of(&response, LOCATION), "/");

    let landed = get_with_cookie(&app, "/", &header_of(&response, SET_COOKIE)).await;

    assert_eq!(landed.status(), StatusCode::OK);
    assert!(text(landed).await.contains(r#"<div id="app">"#));
}

/// A browser on this machine is pointed at the loopback whatever interfaces the
/// server was told to answer on — `http://0.0.0.0:8422/` is not an address a URL
/// bar has anything to do with.
#[test]
fn a_server_on_every_interface_hands_out_a_loopback_link() {
    let dir = tempfile::tempdir().unwrap();
    let key = WorkbenchKey::issued(dir.path()).unwrap();
    let expected = format!("http://127.0.0.1:8422/?key={}", key.secret());

    for bound in ["0.0.0.0:8422", "[::]:8422"] {
        assert_eq!(
            login_link(bound.parse().unwrap(), &key),
            expected,
            "{bound}"
        );
    }
}

/// And a device that reaches Verkstead from somewhere else gets the same key
/// against the address it reaches it at: `tailscale serve` is in front of the
/// same server, and where a request arrives from is not something the server can
/// read off its own socket.
#[test]
fn the_link_carries_the_key_to_whatever_address_it_is_built_against() {
    let dir = tempfile::tempdir().unwrap();
    let key = WorkbenchKey::issued(dir.path()).unwrap();
    let expected = format!("https://desk.tail0000.ts.net/?key={}", key.secret());

    // With the root on it or without, because an address is written both ways
    // and a link with two slashes in it is one somebody will not trust.
    assert_eq!(key.link("https://desk.tail0000.ts.net"), expected);
    assert_eq!(key.link("https://desk.tail0000.ts.net/"), expected);
}

/// A page opened from a push notification is a page of the workbench like any
/// other: the worker navigates to the path the server named, same-origin and
/// with no key on it — see `assets/sw.js` — so what lets it in is the cookie the
/// browser is already holding, sent on a navigation because the cookie is
/// `SameSite=Lax` rather than `Strict`.
///
/// Nothing had to be built for that to be true. This is what says it is.
#[tokio::test]
async fn a_page_a_push_notification_opens_lands_logged_in() {
    let (_dir, _pool, key, app, conversation) = keyed().await;

    let handshake = get(&app, &format!("/?key={}", key.secret())).await;
    let cookie = header_of(&handshake, SET_COOKIE);

    assert!(
        cookie.contains("SameSite=Lax"),
        "a `Strict` cookie is one a tapped notification would not carry: got `{cookie}`",
    );

    // The two shapes the path a push names takes — see `Notice` in
    // `crates/server/src/push.rs`.
    for path in ["/sets/1", &format!("/conversations/{conversation}")] {
        assert_eq!(
            get(&app, path).await.status(),
            StatusCode::UNAUTHORIZED,
            "GET {path} without the cookie",
        );

        let opened = get_with_cookie(&app, path, &cookie).await;

        assert_eq!(opened.status(), StatusCode::OK, "GET {path}");
        assert!(text(opened).await.contains(r#"<div id="app">"#));
    }
}

#[tokio::test]
async fn what_a_phone_installs_the_viewer_from_is_open() {
    let (_dir, _pool, _key, app, _conversation) = keyed().await;

    // A browser fetches a web manifest without credentials, and a service worker
    // behind a gate is a push notification that never arrives. None of the three
    // says anything about anybody's work.
    for path in ["/sw.js", "/manifest.webmanifest", "/icons/icon-192.png"] {
        let response = get(&app, path).await;

        assert_eq!(response.status(), StatusCode::OK, "GET {path}");
    }
}

/// And nothing else under that prefix, which is what makes exempting a
/// directory safe at all.
///
/// The viewer answers a path whose last segment carries no extension with the
/// workbench's own document — that is how every route it draws is served — so a
/// prefix let past on its own would hand `/icons/anything` the very page `/` is
/// refused for, under a name nobody thinks to ask about.
#[tokio::test]
async fn a_route_dressed_as_an_icon_is_still_refused() {
    let (_dir, _pool, _key, app, _conversation) = keyed().await;

    for path in ["/icons/anything", "/icons/", "/icons/deeper/still"] {
        let response = get(&app, path).await;

        assert_eq!(
            response.status(),
            StatusCode::UNAUTHORIZED,
            "GET {path} names no file, so it is the workbench asked for sideways",
        );
    }

    // And a file under it that is genuinely not there is missing rather than
    // refused: it was let past the gate, and the viewer had nothing to hand
    // over.
    assert_eq!(
        get(&app, "/icons/icon-512.png").await.status(),
        StatusCode::NOT_FOUND,
    );
}

#[test]
fn the_key_is_kept_to_the_account_verkstead_runs_as() {
    let dir = tempfile::tempdir().unwrap();
    let key = WorkbenchKey::issued(dir.path()).unwrap();

    assert!(!key.secret().is_empty());

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mode = std::fs::metadata(key.path()).unwrap().permissions().mode();

        assert_eq!(
            mode & 0o777,
            0o600,
            "the key is a password to the whole workbench: {:o}",
            mode & 0o777,
        );
    }
}

#[test]
fn a_second_start_reads_the_key_the_first_made() {
    let dir = tempfile::tempdir().unwrap();

    // A key re-issued at every start would log every device out on every
    // restart, and a phone would have nothing worth remembering.
    let first = WorkbenchKey::issued(dir.path()).unwrap();
    let second = WorkbenchKey::issued(dir.path()).unwrap();

    assert_eq!(first.secret(), second.secret());
}

#[test]
fn clearing_the_github_token_leaves_the_key_where_it_was() {
    let dir = tempfile::tempdir().unwrap();
    let key = WorkbenchKey::issued(dir.path()).unwrap();

    // Which is why it is a file of its own: a settings save writes the whole of
    // `secrets.yaml`, so a key kept in there would be rewritten by somebody
    // tidying up a token they had finished with — and this is the tidying up.
    let settings = Settings::in_data_dir(dir.path());
    settings
        .save_secrets(
            &settings
                .secrets()
                .with_token(Some("ghp_something".to_owned())),
        )
        .unwrap();
    settings
        .save_secrets(&settings.secrets().with_token(None))
        .unwrap();

    assert_eq!(
        WorkbenchKey::issued(dir.path()).unwrap().secret(),
        key.secret(),
        "an ordinary settings save should not have logged every device out",
    );
}
