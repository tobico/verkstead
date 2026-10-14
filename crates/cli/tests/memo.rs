//! `verkstead memo`: a session leaving what it learned for the sessions after it.
//!
//! What the server makes of a memo from a running session — who may leave one,
//! and what it is stamped with — is the sessions suite's to ask, because only a
//! real session was sent for anything. What is asked here is this end of it:
//! the memo the CLI refuses before sending, the same memo refused by the server
//! when sent by hand, and a refusal being a non-zero exit with the server's own
//! words on stderr and nothing on stdout.

mod support;

use std::io::Write;
use std::process::{Command, Output, Stdio};

use support::server::{Server, finished, stderr, stdout};

/// A server nothing listens at, so a memo that reached for one would fail
/// saying so rather than with the refusal under test.
const NOWHERE: &str = "http://127.0.0.1:9/conversations/1";

/// Run `verkstead memo` against `server` with `markdown` on stdin.
fn memo(server: &str, markdown: &str) -> Output {
    let tmp = tempfile::tempdir().unwrap();

    let mut child = Command::new(env!("CARGO_BIN_EXE_verkstead"))
        .arg("memo")
        .env("VERKSTEAD_SERVER", server)
        .current_dir(tmp.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the verkstead binary should be built for its own tests");

    child
        .stdin
        .take()
        .unwrap()
        .write_all(markdown.as_bytes())
        .unwrap();

    finished(child)
}

/// Refused: a non-zero exit, `said` on stderr, and nothing on stdout for an
/// agent to mistake for a memo's id.
fn refused_saying(output: &Output, said: &str) {
    assert!(
        !output.status.success(),
        "a refused memo is a non-zero exit, got {:?}",
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

/// Over the maximum by one byte.
fn too_long() -> String {
    "x".repeat(8 * 1024 + 1)
}

#[test]
fn an_empty_memo_is_refused_without_a_request() {
    refused_saying(&memo(NOWHERE, ""), "the memo is empty");
    refused_saying(&memo(NOWHERE, "  \n\n"), "the memo is empty");
}

#[test]
fn a_memo_over_eight_kib_is_refused_without_a_request() {
    refused_saying(&memo(NOWHERE, &too_long()), "over the maximum of 8192");
}

/// And the same two refused by the server, sent by hand: the CLI's check is a
/// courtesy, and the server's is the rule.
#[test]
fn an_empty_or_oversized_memo_sent_by_hand_is_refused_by_the_server() {
    let tmp = tempfile::tempdir().unwrap();
    let server = Server::start(tmp.path().join("verkstead.db"));

    for (body, said) in [
        (String::new(), "the memo is empty"),
        (too_long(), "over the maximum of 8192"),
    ] {
        let mut reply = ureq::post(format!("{}/api/v1/memos", server.url()))
            .config()
            .http_status_as_error(false)
            .build()
            .send(&body)
            .unwrap();

        assert_eq!(reply.status().as_u16(), 409, "{said}");
        let text = reply.body_mut().read_to_string().unwrap();
        assert!(
            text.contains(said),
            "the server says {said:?}, got:\n{text}"
        );
    }
}

/// A server with no agents has no session running, which is the one refusal a
/// well-formed memo can get here.
#[test]
fn a_memo_with_no_session_running_is_refused_by_name() {
    let tmp = tempfile::tempdir().unwrap();
    let server = Server::start(tmp.path().join("verkstead.db"));

    refused_saying(
        &memo(
            &server.url(),
            "The sessions suite flakes under parallelism.\n",
        ),
        "no session running in this Conversation",
    );
}
