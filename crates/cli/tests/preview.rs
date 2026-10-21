//! `verkstead preview`: a session suggesting how to try its work by hand.
//!
//! Who may suggest one, and what lands on the Conversation, is the sessions
//! suite's to ask, because only a real session was sent for anything. What is
//! asked here is this end of it: the suggestion the CLI refuses before
//! sending, the same suggestion refused by the server when sent by hand, and a
//! refusal being a non-zero exit with the words on stderr and nothing on
//! stdout.

mod support;

use std::io::Write;
use std::process::{Command, Output, Stdio};

use support::server::{Server, finished, stderr, stdout};

/// A server nothing listens at, so a preview that reached for one would fail
/// saying so rather than with the refusal under test.
const NOWHERE: &str = "http://127.0.0.1:9/conversations/1";

/// Run `verkstead preview` with `args` against `server`, `command` on stdin.
fn preview(args: &[&str], server: &str, command: &str) -> Output {
    let tmp = tempfile::tempdir().unwrap();

    let mut child = Command::new(env!("CARGO_BIN_EXE_verkstead"))
        .arg("preview")
        .args(args)
        .env("VERKSTEAD_SERVER", server)
        .current_dir(tmp.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the verkstead binary should be built for its own tests");

    // A command refused on its arguments exits before it reads anything, so
    // the pipe may be closed under the write: that is the refusal, which the
    // output says, rather than a failure of the test's.
    match child.stdin.take().unwrap().write_all(command.as_bytes()) {
        Err(error) if error.kind() == std::io::ErrorKind::BrokenPipe => {}
        written => written.unwrap(),
    }

    finished(child)
}

/// Refused: a non-zero exit, `said` on stderr, and nothing on stdout for an
/// agent to mistake for a title.
fn refused_saying(output: &Output, said: &str) {
    assert!(
        !output.status.success(),
        "a refused preview is a non-zero exit, got {:?}",
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
    "x".repeat(4 * 1024 + 1)
}

/// A command with a description over its maximum by one byte under it.
fn too_long_a_description() -> String {
    format!("cargo test\n\n{}\n", "x".repeat(2 * 1024 + 1))
}

/// Every shape refused, with what each refusal says.
fn malformed() -> Vec<(&'static str, String, &'static str)> {
    vec![
        ("", "cargo test\n".to_owned(), "the preview has no title"),
        ("  ", "cargo test\n".to_owned(), "the preview has no title"),
        (
            "Build and test",
            String::new(),
            "the preview's command is empty",
        ),
        (
            "Build and test",
            "\n\n".to_owned(),
            "the preview's command is empty",
        ),
        (
            "Build and test",
            "cargo build\ncargo test\n".to_owned(),
            "runs onto a second line",
        ),
        (
            "Build and test",
            "cargo build\ncargo test\n\nBuilds it.\n".to_owned(),
            "runs onto a second line",
        ),
        ("Build and test", too_long(), "over the maximum of 4096"),
        (
            "Build and test",
            too_long_a_description(),
            "the preview's description is 2049 bytes, over the maximum of 2048",
        ),
    ]
}

#[test]
fn a_malformed_preview_is_refused_without_a_request() {
    for (title, command, said) in malformed() {
        refused_saying(&preview(&[title], NOWHERE, &command), said);
    }
}

/// The heredoc's own trailing newline is not a newline in the command, a
/// command of exactly the maximum is not over it, and nor is a description of
/// exactly its own under a blank line: each reaches the server, which refuses
/// it here for want of a session rather than for its shape.
#[test]
fn a_trailing_newline_a_description_and_the_maximums_themselves_are_sent() {
    let tmp = tempfile::tempdir().unwrap();
    let server = Server::start(tmp.path().join("verkstead.db"));

    for command in [
        "cargo test\n".to_owned(),
        "x".repeat(4 * 1024) + "\n",
        "cargo test\n\nRuns **every** suite.\n".to_owned(),
        format!("cargo test\n\n\n{}\n\n", "x".repeat(2 * 1024)),
    ] {
        refused_saying(
            &preview(&["Build and test"], &server.url(), &command),
            "no session running in this Conversation",
        );
    }
}

/// And the same shapes refused by the server, sent by hand: the CLI's check is
/// a courtesy, and the server's is the rule.
#[test]
fn a_malformed_preview_sent_by_hand_is_refused_by_the_server() {
    let tmp = tempfile::tempdir().unwrap();
    let server = Server::start(tmp.path().join("verkstead.db"));

    for (title, command, said) in malformed() {
        // As the fields the CLI would have sent, before its own check: a
        // second line that is not blank arrives as a command holding a newline.
        let body =
            serde_saphyr::to_string(&verkstead_schema::PreviewSuggestion::read(title, &command))
                .unwrap();

        let mut reply = ureq::post(format!("{}/api/v1/previews", server.url()))
            .config()
            .http_status_as_error(false)
            .build()
            .send(&body)
            .unwrap();

        assert_eq!(reply.status().as_u16(), 409, "{said}");
        let text = reply.body_mut().read_to_string().unwrap();
        let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(
            text.contains(said),
            "the server says {said:?}, got:\n{text}"
        );
    }
}

/// A retirement is put to the server, which refuses it here by name, with a
/// non-zero exit and nothing on stdout.
#[test]
fn a_retirement_with_no_session_running_is_refused_by_name() {
    let tmp = tempfile::tempdir().unwrap();
    let server = Server::start(tmp.path().join("verkstead.db"));

    refused_saying(
        &preview(&["--retire", "Build and test"], &server.url(), ""),
        "no session running in this Conversation",
    );
}

/// Neither a title nor a retire is a mistake on the command line, caught before
/// anything is read or sent.
#[test]
fn a_preview_with_no_title_is_refused() {
    let output = preview(&[], NOWHERE, "cargo test\n");

    assert!(!output.status.success(), "{:?}", output.status);
    assert!(stdout(&output).is_empty(), "{}", stdout(&output));
    assert!(
        stderr(&output).contains("<TITLE>"),
        "naming what it needs: {}",
        stderr(&output)
    );
}
