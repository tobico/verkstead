//! `verkstead waiting`: the session saying it is waiting on work of its own.
//!
//! What a standing wait does to a running session is the sessions suite's to
//! ask, because only a real session is one to keep at work. What is asked here is
//! this end of it: that a refusal is a non-zero exit with the server's own words
//! on stderr, naming the maximum where the length is the trouble.

mod support;

use std::process::{Command, Output, Stdio};

use support::server::{Server, finished, stderr, stdout};

/// Run `verkstead waiting` with `args` against `server`.
fn waiting(server: &Server, args: &[&str]) -> Output {
    let tmp = tempfile::tempdir().unwrap();

    let child = Command::new(env!("CARGO_BIN_EXE_verkstead"))
        .arg("waiting")
        .args(args)
        .env("VERKSTEAD_SERVER", server.url())
        .current_dir(tmp.path())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the verkstead binary should be built for its own tests");

    finished(child)
}

/// Refused: a non-zero exit, `said` on stderr, and nothing on stdout for an
/// agent to mistake for a wait standing.
fn refused_saying(output: &Output, said: &str) {
    assert!(
        !output.status.success(),
        "a refused wait is a non-zero exit, got {:?}",
        output.status
    );
    assert!(
        stderr(output).contains(said),
        "and says {said:?} on stderr, got:\n{}",
        stderr(output)
    );
    assert!(
        stdout(output).is_empty(),
        "with nothing on stdout, got:\n{}",
        stdout(output)
    );
}

#[test]
fn a_wait_longer_than_an_hour_is_refused_naming_the_maximum() {
    let tmp = tempfile::tempdir().unwrap();
    let server = Server::start(tmp.path().join("verkstead.db"));

    refused_saying(&waiting(&server, &["2h"]), "maximum of 1h");
}

#[test]
fn a_length_that_does_not_parse_is_refused_naming_the_maximum() {
    let tmp = tempfile::tempdir().unwrap();
    let server = Server::start(tmp.path().join("verkstead.db"));

    refused_saying(&waiting(&server, &["a while"]), "at most 1h");
}

/// A server with no agents has no session running, which is the one refusal a
/// well-formed length can get here — and a bare wait is well formed.
#[test]
fn a_wait_with_no_session_running_is_refused_saying_there_is_nothing_waiting() {
    let tmp = tempfile::tempdir().unwrap();
    let server = Server::start(tmp.path().join("verkstead.db"));

    refused_saying(
        &waiting(&server, &[]),
        "no session running in this Conversation",
    );
    refused_saying(
        &waiting(&server, &["45m"]),
        "no session running in this Conversation",
    );
}
