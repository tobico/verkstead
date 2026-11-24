//! `verkstead pull-request open`: a session asking the Repo's Forge to open the
//! pull request its work goes on.
//!
//! What lands on the Conversation — the Forge asked as the Author, the row and
//! the Timeline Event, the Conversation carried on to wrapping it up — is the
//! sessions suite's to ask, because only a real session was sent for anything.
//! What is asked here is this end of it: the body the verb sends, a refusal
//! being a non-zero exit with the words on stderr and nothing on stdout, and
//! the flags being a session's to leave off.

use std::process::{Command, Output, Stdio};

use super::support::server::{Server, finished, stderr, stdout};

/// A server nothing listens at, for the runs that must not reach one.
const NOWHERE: &str = "http://127.0.0.1:9/conversations/1";

/// Run `verkstead pull-request open` with `args` against `server`.
fn open(args: &[&str], server: &str) -> Output {
    let tmp = tempfile::tempdir().unwrap();

    let child = Command::new(env!("CARGO_BIN_EXE_verkstead"))
        .args(["pull-request", "open"])
        .args(args)
        .env("VERKSTEAD_SERVER", server)
        .current_dir(tmp.path())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the verkstead binary should be built for its own tests");

    finished(child)
}

/// Refused: a non-zero exit, `said` on stderr, and nothing on stdout for a
/// session to mistake for a URL.
fn refused_saying(output: &Output, said: &str) {
    assert!(
        !output.status.success(),
        "a refused open is a non-zero exit, got {:?}",
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

/// Every flag is a session's to leave off, and a bare run reaches the server
/// all the same: it is refused there for want of a session rather than for its
/// shape, the title, the body and the base each being read off the record.
#[test]
fn a_bare_open_is_sent_and_refused_by_the_server() {
    let tmp = tempfile::tempdir().unwrap();
    let server = Server::start(tmp.path().join("verkstead.db"));

    refused_saying(
        &open(&[], &server.url()),
        "no session running in this Conversation",
    );
}

/// And the same with what a session does say about it, which goes to the
/// server as the body rather than to a forge's own tool as a command line.
#[test]
fn a_title_a_body_and_a_base_are_sent_to_the_server() {
    let tmp = tempfile::tempdir().unwrap();
    let server = Server::start(tmp.path().join("verkstead.db"));

    refused_saying(
        &open(
            &[
                "--title",
                "Rate limiting",
                "--body",
                "A counter per key.\n\nIt refuses the excess.",
                "--base",
                "release/2.1",
            ],
            &server.url(),
        ),
        "no session running in this Conversation",
    );
}

/// A server that cannot be reached is said as that, naming where it looked:
/// the one thing a session can do about it is say so, and a URL on stdout
/// would have it believe a pull request was opened.
#[test]
fn a_server_that_cannot_be_reached_is_a_refusal_naming_it() {
    refused_saying(&open(&[], NOWHERE), NOWHERE);
}
