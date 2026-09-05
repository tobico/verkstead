//! `verkstead ask` and `verkstead answers` over the named pipe the server
//! listens on beside its socket.
//!
//! Windows' own, and the whole of what a sandboxed session there will ask
//! through: an AppContainer is refused the loopback interface, so the transport
//! has to be one an identity can be granted rather than one an address is
//! routed to (ADR-0014).
//!
//! What is proved here is that the pipe is the *same* ask. The Set goes, the
//! wait holds, the Response comes back on stdout, a restart mid-wait is ridden
//! out, and a pipe nothing is listening on is a server that could not be
//! reached — each of them what the URL does, checked against the URL doing it
//! wherever a run can be put through both.

#![cfg(windows)]

mod support;

use std::io::Write;
use std::path::Path;
use std::process::{Child, Command, Output, Stdio};
use std::time::Duration;

use support::server::{Server, finished, stderr, stdout};

/// One Question, because what is under test is the transport rather than the
/// grammar — `ask.rs` is where a Set's shape is proved.
const SET: &str = "
title: Which transport should a Windows session ask through?
questions:
  - label: Q1
    text: The pipe, or the socket beside it?
    options:
      - n: 1
        text: The pipe
        recommended: true
      - n: 2
        text: The socket
";

/// And its Response, which is what both runs have to print.
const ANSWERED: &str = "
answers:
  - label: Q1
    selected: 1
comment: |
  The pipe: a container is refused the loopback interface.
";

/// `verkstead ask` against `server`, running in `dir`, with its streams on
/// pipes the test can drive.
fn ask(server: &str, dir: &Path, set: &str) -> Child {
    let mut child = Command::new(env!("CARGO_BIN_EXE_verkstead"))
        .arg("ask")
        .env("VERKSTEAD_SERVER", server)
        .current_dir(dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the verkstead binary should be built for its own tests");

    // Dropping the handle closes the pipe, which is the CLI's end of input.
    let mut stdin = child.stdin.take().unwrap();
    stdin.write_all(set.as_bytes()).unwrap();
    drop(stdin);

    child
}

/// `verkstead answers <id>` against `server`, run to its end.
fn answers(server: &str, dir: &Path, id: i64) -> Output {
    let child = Command::new(env!("CARGO_BIN_EXE_verkstead"))
        .args(["answers", &id.to_string()])
        .env("VERKSTEAD_SERVER", server)
        .current_dir(dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the verkstead binary should be built for its own tests");

    finished(child)
}

/// A Set asked over the pipe and answered, and the same over the URL: the two
/// runs print the same Response, because they are the same ask.
#[test]
fn an_ask_over_the_pipe_prints_what_the_same_ask_over_the_url_prints() {
    let tmp = tempfile::tempdir().unwrap();
    let server = Server::start(tmp.path().join("verkstead.db"));

    let over_the_pipe = ask(&server.pipe_url(), tmp.path(), SET);
    server.await_asked_set(1);
    server.answer(1, ANSWERED);
    let over_the_pipe = finished(over_the_pipe);

    let over_the_url = ask(&server.url(), tmp.path(), SET);
    server.await_asked_set(2);
    server.answer(2, ANSWERED);
    let over_the_url = finished(over_the_url);

    assert!(
        over_the_pipe.status.success(),
        "an ask over the pipe should exit cleanly, got {:?}\n{}",
        over_the_pipe.status,
        stderr(&over_the_pipe),
    );
    assert_eq!(
        stdout(&over_the_pipe),
        stdout(&over_the_url),
        "the pipe is the same ask, so what it prints is what the URL prints",
    );
    assert!(
        stderr(&over_the_pipe).is_empty(),
        "a wait that goes to plan says nothing at all, got:\n{}",
        stderr(&over_the_pipe),
    );
}

/// And a Response fetched by id comes back over the pipe the same way.
#[test]
fn answers_fetches_a_response_over_the_pipe() {
    let tmp = tempfile::tempdir().unwrap();
    let server = Server::start(tmp.path().join("verkstead.db"));

    let stored = server.store_and_nudge(SET);
    server.answer(stored, ANSWERED);

    let over_the_pipe = answers(&server.pipe_url(), tmp.path(), stored);
    let over_the_url = answers(&server.url(), tmp.path(), stored);

    assert!(
        over_the_pipe.status.success(),
        "fetching over the pipe should exit cleanly, got {:?}\n{}",
        over_the_pipe.status,
        stderr(&over_the_pipe),
    );
    assert_eq!(
        stdout(&over_the_pipe),
        stdout(&over_the_url),
        "one Response shape reaches the agent however it came by it",
    );
}

/// A pipe nothing is listening on is a server that could not be reached, said
/// the way a refused TCP connection is said and exiting the way one does.
#[test]
fn a_pipe_nothing_is_listening_on_is_a_server_that_cannot_be_reached() {
    let tmp = tempfile::tempdir().unwrap();

    // A name nothing has created and a port nothing is bound to: the two
    // shapes of the same thing, which is a Verkstead that is not running.
    let no_pipe = "pipe://verkstead-nothing-is-listening-here/conversations/1";
    let no_socket = "http://127.0.0.1:1/conversations/1";

    let over_the_pipe = finished(ask(no_pipe, tmp.path(), SET));
    let over_the_socket = finished(ask(no_socket, tmp.path(), SET));

    assert!(
        !over_the_pipe.status.success(),
        "a Set that was never accepted is a non-zero exit, got {:?}",
        over_the_pipe.status,
    );
    assert!(
        stdout(&over_the_pipe).is_empty(),
        "nothing but a Response is ever written to stdout, got:\n{}",
        stdout(&over_the_pipe),
    );

    // The same sentence about the same failure, each naming the server it was
    // given — which for a pipe is the spelling that was typed rather than the
    // placeholder URL it is dialled at.
    assert!(
        stderr(&over_the_pipe).starts_with(&format!(
            "verkstead: submitting the Question Set to {no_pipe}:"
        )),
        "the failure should name the pipe it could not reach, got:\n{}",
        stderr(&over_the_pipe),
    );
    assert!(
        stderr(&over_the_socket).starts_with(&format!(
            "verkstead: submitting the Question Set to {no_socket}:"
        )),
        "and the socket the same way, got:\n{}",
        stderr(&over_the_socket),
    );
}

/// The wait rides out a restart over the pipe as it does over the socket: the
/// hold is cut short, the retry is reported on stderr as a YAML comment, and
/// the Response still arrives.
#[test]
fn the_wait_reconnects_over_the_pipe_when_the_server_restarts() {
    let tmp = tempfile::tempdir().unwrap();
    let server = Server::start(tmp.path().join("verkstead.db"));

    let waiting = ask(&server.pipe_url(), tmp.path(), SET);
    server.await_asked_set(1);

    // The same Data Directory, so the server comes back on the same pipe — as
    // it comes back on the same port.
    let (addr, database) = server.kill();
    std::thread::sleep(Duration::from_millis(250));
    let server = Server::bind(addr, database);

    server.answer(1, ANSWERED);

    let output = finished(waiting);
    assert!(
        output.status.success(),
        "the CLI should have ridden out the restart, got {:?}\n{}",
        output.status,
        stderr(&output),
    );
    assert!(
        stderr(&output).contains("retrying"),
        "the reconnection should be reported on stderr, got:\n{}",
        stderr(&output),
    );

    // What a harness capturing both streams into one file is handed: still one
    // Response, because everything the CLI said on the way is a YAML comment.
    let merged = format!("{}{}", stderr(&output), stdout(&output));
    assert!(
        verkstead_schema::Response::from_yaml(&merged).is_ok(),
        "the two streams merged should still parse as the Response, got:\n{merged}"
    );
}
