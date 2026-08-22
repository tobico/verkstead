//! The Screen a session leaves behind, asked of the endpoint the details pane
//! fetches it from.
//!
//! What comes back is a repaint — the escape sequences that would paint the grid
//! — so nothing here reads it as a string. It is fed to a terminal, and what that
//! terminal is left showing is the assertion, because that is the only claim a
//! repaint makes: a terminal that has seen none of the session ends up showing
//! what the session left.
//!
//! The Captures are written through the store rather than printed by a session.
//! Whether a session's output reaches the store at all is `tests/sessions.rs`'s
//! subject; this file's is what the store's copy leaves on a terminal.

use avt::Vt;
use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde::de::DeserializeOwned;
use sqlx::SqlitePool;
use tower::ServiceExt;
use verkstead_server::{open_database, router, store};

/// A router over a database with nothing in it, plus the pool the fixtures are
/// written through and the directory keeping both alive.
async fn app() -> (tempfile::TempDir, SqlitePool, Router) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    (dir, pool.clone(), router(pool))
}

/// A Conversation with one session's Capture on its Timeline: the ids of the
/// two, in the order the URL wants them.
///
/// The Capture is finished the moment it is written, which is a session that has
/// ended — the shape this file is about. Nothing here starts anything.
async fn session_that_printed(pool: &SqlitePool, printed: &str) -> (i64, i64) {
    let repo = store::register_repo(
        pool,
        std::path::Path::new("/srv/verkstead"),
        "verkstead",
        "main",
    )
    .await
    .unwrap()
    .unwrap();

    let conversation = store::start_conversation(pool, repo.id, "screen")
        .await
        .unwrap()
        .unwrap();

    let event = store::start_capture(pool, conversation, None)
        .await
        .unwrap();

    store::append_capture(
        pool,
        event,
        printed,
        &store::Summary {
            lines: printed.matches('\n').count() as i64,
            latest: String::new(),
        },
    )
    .await
    .unwrap();

    (conversation, event)
}

/// What the details pane is handed for that session.
async fn screen(app: &Router, conversation: i64, event: i64) -> verkstead_render::Screen {
    get(
        app,
        &format!("/api/ui/conversations/{conversation}/screen/{event}"),
    )
    .await
}

/// What a terminal fed `screen`'s repaint is left showing: the visible grid, row
/// by row, with the blank rows at the bottom left off.
fn painted(screen: &verkstead_render::Screen) -> Vec<String> {
    let mut vt = Vt::new(usize::from(screen.columns), usize::from(screen.rows));
    vt.feed_str(&screen.repaint);

    let mut rows: Vec<String> = vt
        .view()
        .map(|line| line.text().trim_end().to_owned())
        .collect();

    while rows.last().is_some_and(|row| row.is_empty()) {
        rows.pop();
    }

    rows
}

/// The Screen of a session that has ended is what its terminal last showed —
/// which is the grid the bytes drew, and not the bytes.
#[tokio::test]
async fn a_session_that_has_ended_shows_the_screen_it_left() {
    let (_dir, pool, app) = app().await;

    // A spinner that redrew its line, and a status line drawn from the bottom
    // up: two of the three things an agent's display does that make a Capture
    // unreadable and a Screen readable.
    let (conversation, event) = session_that_printed(
        &pool,
        "Reading the brief.\r\n\
         Thinking.\rThinking..\rThinking… done\r\n\
         \x1b[1mWhat should happen to a delivery that has failed forty times?\x1b[0m\r\n",
    )
    .await;

    assert_eq!(
        painted(&screen(&app, conversation, event).await),
        vec![
            "Reading the brief.".to_owned(),
            "Thinking… done".to_owned(),
            "What should happen to a delivery that has failed forty times?".to_owned(),
        ],
    );
}

/// A session that ended inside a full-screen display ended on the alternate
/// screen, and the Screen it left is that one.
#[tokio::test]
async fn a_session_that_ended_on_the_alternate_screen_shows_that_one() {
    let (_dir, pool, app) = app().await;

    let (conversation, event) = session_that_printed(
        &pool,
        "$ verkstead ask\r\n\x1b[?1049h\x1b[2J\x1b[H\x1b[1mQuestion 1 of 3\x1b[0m\r\n  Where does the counter live?",
    )
    .await;

    assert_eq!(
        painted(&screen(&app, conversation, event).await),
        vec![
            "Question 1 of 3".to_owned(),
            "  Where does the counter live?".to_owned(),
        ],
        "what the shell printed is behind the display, not in front of it",
    );
}

/// However much a session printed, what is handed over is the grid.
#[tokio::test]
async fn nothing_above_the_top_of_the_grid_is_handed_over() {
    let (_dir, pool, app) = app().await;

    let printed: String = (0..500).map(|line| format!("line {line}\r\n")).collect();
    let (conversation, event) = session_that_printed(&pool, &printed).await;

    let screen = screen(&app, conversation, event).await;
    let shown = painted(&screen);

    assert!(
        shown.len() < usize::from(screen.rows),
        "a Screen is {} rows of terminal, and it handed over {}",
        screen.rows,
        shown.len(),
    );

    assert_eq!(
        shown.last().map(String::as_str),
        Some("line 499"),
        "and what it shows is the end of what was printed",
    );

    assert!(
        !screen.repaint.contains("line 0\r\n"),
        "nothing that scrolled off the top is in the repaint",
    );
}

/// An Event on another Conversation's Timeline names no Screen on this one —
/// the same pairing every other Event payload is reached through.
#[tokio::test]
async fn an_event_on_another_conversation_names_no_screen() {
    let (_dir, pool, app) = app().await;

    let (_, event) = session_that_printed(&pool, "printed\r\n").await;

    let elsewhere = store::start_conversation(&pool, 1, "somewhere-else")
        .await
        .unwrap()
        .unwrap();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/ui/conversations/{elsewhere}/screen/{event}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

/// One GET, decoded.
async fn get<T: DeserializeOwned>(app: &Router, uri: &str) -> T {
    let response = app
        .clone()
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK, "GET {uri}");

    let body = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&body).unwrap()
}
