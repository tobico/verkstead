//! The named pipe the server listens on beside its socket, asked the way
//! anything else asks a server: by making a request over it and reading what
//! came back.
//!
//! **What is being proved is that there is nothing to prove.** The pipe is one
//! more listener under the same router, so everything a request can ask for
//! over the socket it can ask for over the pipe — the health check that is
//! nobody's Conversation, and the endpoints under a Conversation's own base
//! that are the whole of what a session asks through. Each question here is
//! asked twice, once down each transport, and what the two answered is compared.
//!
//! **The requests are written out by hand**, because that is what proves the
//! transport rather than a client library's arrangement with it: the same bytes
//! go down the pipe and down the socket, and the answers are compared as they
//! arrive. Everything but the headers, that is — one of them is the date, and
//! two requests a moment apart are not owed the same second.
#![cfg(windows)]

use std::net::{Ipv4Addr, SocketAddr};
use std::path::Path;
use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use sqlx::SqlitePool;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::net::windows::named_pipe::ClientOptions;
use tower::ServiceExt;
use verkstead_server::pipe;
use verkstead_server::{open_database, router, store};

/// The Conversation every Set here is asked from, which is the first one made
/// in a database with nothing in it.
const ASKING_FROM: i64 = 1;

/// How long a request over either transport may take before the test says so.
///
/// Everything here is a loopback connection to a server in this process, so it
/// is generous by a long way: what it is really guarding against is a read that
/// would otherwise hang the suite.
const PATIENCE: Duration = Duration::from_secs(30);

/// One Question Set, answered, so that the endpoint under a Conversation's base
/// has something settled to answer with — a Set nobody has answered would hold
/// the wait open, and what is being compared here is two answers rather than
/// two waits.
const SET: &str = "
title: Which transport does a session ask through?
questions:
  - label: Q1
    text: The socket or the pipe?
    options:
      - n: 1
        text: The socket
      - n: 2
        text: The pipe
        recommended: true
";

const ANSWER: &str = "
answers:
  - label: Q1
    selected: 2
";

/// What a request came back with: the status line, and the body under the
/// headers.
///
/// The headers themselves are left out — see this file's own documentation.
#[derive(Debug, PartialEq, Eq)]
struct Answered {
    status: String,
    body: String,
}

/// A Verkstead standing on both transports: its Data Directory, its database,
/// and where each half of it is reached.
struct Standing {
    _dir: tempfile::TempDir,
    pool: SqlitePool,
    pipe: String,
    socket: SocketAddr,
}

/// One server, listening on a pipe named after a Data Directory of its own and
/// on a socket the machine chose, both serving one router over one database.
async fn standing() -> Standing {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    let app = router(pool.clone());

    let pipe = pipe::Listener::open(dir.path(), None).expect("nothing holds this name yet");
    let named = pipe.name().to_owned();

    let socket = tokio::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
        .await
        .unwrap();
    let address = socket.local_addr().unwrap();

    tokio::spawn({
        let app = app.clone();
        async move { axum::serve(pipe, app).await }
    });
    tokio::spawn(async move { axum::serve(socket, app).await });

    Standing {
        _dir: dir,
        pool,
        pipe: named,
        socket: address,
    }
}

/// Somewhere for a Set to land: every Set is asked from a Conversation, and the
/// endpoints a session asks through are reached under the one that asked.
async fn a_conversation(pool: &SqlitePool) {
    let repo = store::register_repo(pool, Path::new("/srv/verkstead"), "verkstead", "main")
        .await
        .unwrap()
        .expect("nothing is registered at that path yet");

    let conversation = store::start_conversation(pool, repo.id, "named-pipe")
        .await
        .unwrap()
        .expect("the Repo was just registered");

    assert_eq!(conversation, ASKING_FROM);
}

/// A Set asked and answered, through the router rather than over either
/// transport: what the two are compared on is reading it back.
async fn a_settled_set(pool: &SqlitePool) -> i64 {
    let app = router(pool.clone());

    let asked = app
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
    assert_eq!(asked.status(), StatusCode::CREATED);

    let created: verkstead_schema::SetCreated = serde_saphyr::from_str(&text(asked).await).unwrap();

    let answered = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/conversations/{ASKING_FROM}/api/v1/sets/{}/response",
                    created.id
                ))
                .header(header::CONTENT_TYPE, "application/yaml")
                .body(Body::from(ANSWER))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(answered.status(), StatusCode::CREATED);

    created.id
}

async fn text(response: axum::response::Response) -> String {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();

    String::from_utf8(bytes.to_vec()).unwrap()
}

/// A GET of `path`, as a client that is closing afterwards writes one.
///
/// `Connection: close` so that the read below ends where the answer does: this
/// asks one question and wants the whole of one answer, and a connection held
/// open for a second question it is not going to ask would simply never end.
fn get(path: &str) -> String {
    format!("GET {path} HTTP/1.1\r\nHost: verkstead\r\nConnection: close\r\n\r\n")
}

/// `request` asked over the pipe called `named`.
async fn over_the_pipe(named: &str, request: &str) -> Answered {
    let mut pipe = ClientOptions::new()
        .open(named)
        .unwrap_or_else(|what| panic!("opening {named}: {what}"));

    pipe.write_all(request.as_bytes()).await.unwrap();

    let mut said = Vec::new();
    tokio::time::timeout(PATIENCE, pipe.read_to_end(&mut said))
        .await
        .expect("the pipe should answer")
        .unwrap();

    answered(&said)
}

/// And over the socket, which is the same request down the transport this is
/// being compared against.
async fn over_the_socket(address: SocketAddr, request: &str) -> Answered {
    let mut socket = TcpStream::connect(address).await.unwrap();

    socket.write_all(request.as_bytes()).await.unwrap();

    let mut said = Vec::new();
    tokio::time::timeout(PATIENCE, socket.read_to_end(&mut said))
        .await
        .expect("the socket should answer")
        .unwrap();

    answered(&said)
}

/// What came back, cut into the two halves this file compares.
fn answered(said: &[u8]) -> Answered {
    let said = String::from_utf8_lossy(said);
    let (head, body) = said
        .split_once("\r\n\r\n")
        .unwrap_or_else(|| panic!("an HTTP answer, got {said:?}"));

    Answered {
        status: head.lines().next().unwrap().to_owned(),
        body: body.to_owned(),
    }
}

/// The route that is nobody's Conversation, asked both ways.
#[tokio::test]
async fn the_health_check_answers_the_same_over_either() {
    let standing = standing().await;

    let over_a_pipe = over_the_pipe(&standing.pipe, &get("/api/v1/health")).await;
    let over_a_socket = over_the_socket(standing.socket, &get("/api/v1/health")).await;

    assert!(
        over_a_pipe.status.contains("200"),
        "the pipe should have answered, got {over_a_pipe:?}"
    );
    assert_eq!(over_a_pipe.body, "ok");
    assert_eq!(over_a_pipe, over_a_socket);
}

/// And the endpoints under a Conversation's own base, which are the whole of
/// what a session asks through.
#[tokio::test]
async fn a_conversations_own_base_answers_the_same_over_either() {
    let standing = standing().await;
    a_conversation(&standing.pool).await;
    let set = a_settled_set(&standing.pool).await;

    let asking = get(&format!(
        "/conversations/{ASKING_FROM}/api/v1/sets/{set}/response?hold=0"
    ));

    let over_a_pipe = over_the_pipe(&standing.pipe, &asking).await;
    let over_a_socket = over_the_socket(standing.socket, &asking).await;

    assert!(
        over_a_pipe.status.contains("200"),
        "the Response was settled before it was asked for, got {over_a_pipe:?}"
    );
    assert!(
        over_a_pipe.body.contains("selected: 2"),
        "the Response the human gave, got {over_a_pipe:?}"
    );
    assert_eq!(over_a_pipe, over_a_socket);
}

/// Two Verksteads on two Data Directories are two pipes, and neither answers
/// for the other.
///
/// What each is asked is a question only its own database can answer: the
/// Conversation and the Set are one server's, so the other one answering the
/// same request the same way would be the two sharing a name.
#[tokio::test]
async fn two_data_directories_are_two_pipes_and_each_answers_only_its_own() {
    let one = standing().await;
    a_conversation(&one.pool).await;
    let set = a_settled_set(&one.pool).await;

    let other = standing().await;

    assert_ne!(one.pipe, other.pipe, "two Data Directories, two names");

    let asking = get(&format!(
        "/conversations/{ASKING_FROM}/api/v1/sets/{set}/response?hold=0"
    ));

    let its_own = over_the_pipe(&one.pipe, &asking).await;
    let the_others = over_the_pipe(&other.pipe, &asking).await;

    assert!(
        its_own.status.contains("200"),
        "the server that was asked the Set has it, got {its_own:?}"
    );
    assert!(
        the_others.status.contains("404"),
        "and the one beside it has never heard of it, got {the_others:?}"
    );
}

/// And a second client after the first, on a connection of its own.
///
/// A pipe instance *is* a connection — it is created, waited on, and spoken
/// through — so the listener makes the next one as it hands the last one over.
/// A server that did not would answer once and then leave the name standing
/// with nothing behind it.
#[tokio::test]
async fn a_client_after_the_first_is_answered_too() {
    let standing = standing().await;

    let first = over_the_pipe(&standing.pipe, &get("/api/v1/health")).await;
    let second = over_the_pipe(&standing.pipe, &get("/api/v1/health")).await;

    assert!(
        second.status.contains("200"),
        "the second client should have been answered, got {second:?}"
    );
    assert_eq!(first, second);
}
