//! Reading one session's Transcript over the viewer's namespace, and reading it
//! again without reading the whole of it again.
//!
//! What makes this its own file is the second half. A running session's open
//! pane re-reads its record every time the session says anything, which is twice
//! a second while it talks — so the endpoint grows an incremental form, and the
//! properties that form has to hold are the ones asserted here: that what comes
//! back is only what is new, that accumulating it gives the record read whole,
//! and that every way of failing to carry on ends in the whole record rather
//! than in a gap (ADR 0009).
//!
//! Written through the store rather than by running an agent. What a session
//! does to a Transcript is `tests/sessions.rs`'s subject; this is what the
//! endpoint hands a reader of one.

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde::de::DeserializeOwned;
use sqlx::SqlitePool;
use tower::ServiceExt;
use verkstead_render::TranscriptView;
use verkstead_server::{open_database, router, store};

/// A session's log, in the shape a backend writes one: the conversation itself
/// and the backend's own bookkeeping among it — because what an incremental
/// read has to keep in step is both numberings and the lines they were counted
/// over.
const LOG: &[&str] = &[
    r#"{"type":"user","message":{"role":"user","content":"Where should the counter live?"}}"#,
    r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"Reading the **brief**."}]}}"#,
    r#"{"type":"attachment","attachment":{"type":"todos","content":"two things still to do"}}"#,
    r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"tool_use","id":"toolu_01","name":"Bash","input":{"command":"rg -n counter","description":"Find the counter"}}]}}"#,
    r#"{"type":"user","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"toolu_01","is_error":false,"content":"crates/server/src/limit.rs:14"}]}}"#,
    r#"{"type":"attachment","attachment":{"type":"todos","content":"one thing still to do"}}"#,
    r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"In the store, beside the window it counts over."}]}}"#,
];

/// A Conversation with a session's output on its Timeline, and the first
/// `said` lines of that session's log already on the Transcript.
///
/// What comes back is the router, the pool the rest of the log is put in
/// through, and the Event the record hangs off.
async fn talking(said: usize) -> (tempfile::TempDir, SqlitePool, Router, i64, i64) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    let repo = store::register_repo(
        &pool,
        std::path::Path::new("/srv/repos/verkstead"),
        "verkstead",
        "main",
    )
    .await
    .unwrap()
    .unwrap();

    let conversation = store::start_conversation(&pool, repo.id, "rate-limiting")
        .await
        .unwrap()
        .unwrap();
    let event = store::start_capture(&pool, conversation, None, None)
        .await
        .unwrap();

    wrote(&pool, event, ..said).await;

    (dir, pool.clone(), router(pool), conversation, event)
}

/// The session writing some more of its log.
async fn wrote(
    pool: &SqlitePool,
    event: i64,
    lines: impl std::slice::SliceIndex<[&'static str], Output = [&'static str]>,
) {
    let batch: Vec<String> = LOG[lines].iter().map(|line| (*line).to_owned()).collect();

    store::append_transcript(pool, event, &batch).await.unwrap();
}

/// The pane reading the record, whole or from wherever it got to.
async fn read(app: &Router, conversation: i64, event: i64, after: Option<&str>) -> TranscriptView {
    let at = after.map(|at| format!("?after={at}")).unwrap_or_default();

    fetch(
        app,
        &format!("/api/ui/conversations/{conversation}/transcript/{event}{at}"),
    )
    .await
}

async fn fetch<T: DeserializeOwned>(app: &Router, path: &str) -> T {
    let response = app
        .clone()
        .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK, "GET {path}");
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

/// The whole point of the cursor: the pane that has already read an hour of
/// talking asks for the minute since, and that is what crosses the wire.
#[tokio::test]
async fn a_reader_that_says_where_it_got_to_is_sent_only_what_came_after() {
    let (_dir, pool, app, conversation, event) = talking(3).await;

    let first = read(&app, conversation, event, None).await;
    assert!(first.whole, "the first reading is the record itself");
    assert_eq!(first.turns.len(), 2);
    assert_eq!(first.bookkeeping.len(), 1);

    wrote(&pool, event, 3..).await;
    let rest = read(&app, conversation, event, Some(&first.cursor)).await;

    assert!(
        !rest.whole,
        "and the second is a piece of it, which is what says to add rather than replace"
    );
    assert_eq!(
        rest.turns.len(),
        3,
        "only the turns written since: {:?}",
        rest.turns
    );
    assert_eq!(rest.bookkeeping.len(), 1);
    assert!(
        !serde_json::to_string(&rest)
            .unwrap()
            .contains("Reading the"),
        "and nothing of what the reader already had: {rest:?}"
    );
}

/// The property everything else rests on. A record read in pieces and put
/// together is the record read in one go — the numbering that reconcile matches
/// rows by, the bookkeeping that folds into the one group at the end, and the
/// ending itself, which arrives in the last piece.
#[tokio::test]
async fn a_record_accumulated_batch_by_batch_is_the_record_read_whole() {
    let (_dir, pool, app, conversation, event) = talking(0).await;

    // Read after every batch, as an open pane does: a Nudge per batch, and a
    // reading per Nudge.
    let mut accumulated = read(&app, conversation, event, None).await;

    for line in 0..LOG.len() {
        wrote(&pool, event, line..line + 1).await;

        let more = read(&app, conversation, event, Some(&accumulated.cursor)).await;
        accumulated.turns.extend(more.turns);
        accumulated.bookkeeping.extend(more.bookkeeping);
        accumulated.cursor = more.cursor;
    }

    assert_eq!(
        accumulated,
        read(&app, conversation, event, None).await,
        "the accumulated record and the record read whole are the same record"
    );
}

/// Every way of failing to carry on, and the one answer to all of them. A gap
/// in what somebody is reading is the failure this endpoint has to not have, and
/// the whole record is always a correct answer.
#[tokio::test]
async fn a_cursor_that_cannot_be_carried_on_from_reads_the_record_whole() {
    let (_dir, _pool, app, conversation, event) = talking(LOG.len()).await;

    let whole = read(&app, conversation, event, None).await;

    for cursor in [
        // Never written here — a cursor is a URL parameter, which is to say
        // something anybody can type.
        "elsewhere",
        "3",
        // Written in this shape, but naming a place this record has never been:
        // a page held open across a restart of the server it was reading, or
        // pointed at a Transcript that is not the one it read.
        "400.90.20",
    ] {
        let read = read(&app, conversation, event, Some(cursor)).await;

        assert!(read.whole, "{cursor:?} should have been read whole");
        assert_eq!(read, whole, "{cursor:?} should have read the whole record");
    }
}

/// The cursor changes none of what the endpoint is: an Event on somebody else's
/// Conversation is still not a Transcript, whatever is asked of it.
#[tokio::test]
async fn a_transcript_of_another_conversation_is_no_more_readable_with_a_cursor() {
    let (_dir, pool, app, _conversation, event) = talking(LOG.len()).await;

    let elsewhere = store::start_conversation(&pool, 1, "another-branch")
        .await
        .unwrap()
        .unwrap();

    for at in ["", "?after=2.1.1"] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/api/ui/conversations/{elsewhere}/transcript/{event}{at}"
                    ))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND, "asked {at:?}");
    }
}
