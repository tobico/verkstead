//! `verkstead memo`: a session leaving what it learned for the sessions after it.
//!
//! What the server makes of a memo from a running session — who may leave one,
//! and what it is stamped with — is the sessions suite's to ask, because only a
//! real session was sent for anything. What is asked here is this end of it:
//! the memo the CLI refuses before sending, the same memo refused by the server
//! when sent by hand, a refusal being a non-zero exit with the server's own
//! words on stderr and nothing on stdout, and a memo the server takes being its
//! id on stdout, sent as the title and the markdown.

mod support;

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::process::{Command, Output, Stdio};

use support::server::{Server, finished, stderr, stdout};

/// A server nothing listens at, so a memo that reached for one would fail
/// saying so rather than with the refusal under test.
const NOWHERE: &str = "http://127.0.0.1:9/conversations/1";

/// What the memos here are called.
const TITLE: &str = "The sessions suite flakes";

/// Run `verkstead memo <TITLE>` against `server` with `markdown` on stdin.
fn memo(server: &str, markdown: &str) -> Output {
    memo_with(&[TITLE], server, markdown)
}

/// The same, with `args` after `memo`.
fn memo_with(args: &[&str], server: &str, markdown: &str) -> Output {
    let tmp = tempfile::tempdir().unwrap();

    let mut child = Command::new(env!("CARGO_BIN_EXE_verkstead"))
        .arg("memo")
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
    match child.stdin.take().unwrap().write_all(markdown.as_bytes()) {
        Err(error) if error.kind() == std::io::ErrorKind::BrokenPipe => {}
        written => written.unwrap(),
    }

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

/// A title that is empty once trimmed, or that holds a newline, is refused
/// before anything is read or sent.
#[test]
fn an_empty_or_multi_line_title_is_refused_without_a_request() {
    for (title, said) in [
        ("", "the memo has no title"),
        ("   ", "the memo has no title"),
        ("The suite\nflakes", "the memo's title has a newline in it"),
        (
            "The suite\r\nflakes",
            "the memo's title has a newline in it",
        ),
    ] {
        refused_saying(
            &memo_with(&[title], NOWHERE, "The sessions suite flakes.\n"),
            said,
        );
    }
}

/// No title at all is a mistake on the command line, caught by the parser
/// before anything is read or sent, and naming what it needs.
#[test]
fn a_memo_with_no_title_is_refused() {
    let output = memo_with(&[], NOWHERE, "The sessions suite flakes.\n");

    assert!(!output.status.success(), "{:?}", output.status);
    assert!(stdout(&output).is_empty(), "{}", stdout(&output));
    assert!(
        stderr(&output).contains("<TITLE>"),
        "naming what it needs: {}",
        stderr(&output)
    );
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

/// And the same refused by the server, sent by hand: the CLI's check is a
/// courtesy, and the server's is the rule. A body with no title at all — the
/// shape a memo was sent in before there were titles — is refused for having
/// none.
#[test]
fn a_malformed_memo_sent_by_hand_is_refused_by_the_server() {
    let tmp = tempfile::tempdir().unwrap();
    let server = Server::start(tmp.path().join("verkstead.db"));

    let sent = |title: &str, markdown: &str| {
        serde_saphyr::to_string(&verkstead_schema::MemoLeaving {
            title: title.to_owned(),
            markdown: markdown.to_owned(),
        })
        .unwrap()
    };

    for (body, said) in [
        (sent(TITLE, ""), "the memo is empty"),
        (sent(TITLE, &too_long()), "over the maximum of 8192"),
        (sent("", "A memo.\n"), "the memo has no title"),
        (
            sent("The suite\nflakes", "A memo.\n"),
            "the memo's title has a newline in it",
        ),
        (
            "markdown: \"A memo.\"\n".to_owned(),
            "the memo has no title",
        ),
    ] {
        let mut reply = ureq::post(format!("{}/api/v1/memos", server.url()))
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

/// A retirement is put to the server, which refuses it here the way it refuses
/// a memo: by name, with a non-zero exit and nothing on stdout.
#[test]
fn a_retirement_with_no_session_running_is_refused_by_name() {
    let tmp = tempfile::tempdir().unwrap();
    let server = Server::start(tmp.path().join("verkstead.db"));

    refused_saying(
        &memo_with(
            &["--retire", "42", "fixed", "in", "8f3c2a1"],
            &server.url(),
            "",
        ),
        "no session running in this Conversation",
    );
}

/// A reason with nothing to retire is a mistake on the command line, caught
/// before anything is read or sent.
#[test]
fn a_reason_without_retire_is_refused() {
    let output = memo_with(&["fixed", "in", "8f3c2a1"], NOWHERE, "A memo.\n");

    assert!(!output.status.success(), "{:?}", output.status);
    assert!(stdout(&output).is_empty(), "{}", stdout(&output));
    assert!(
        stderr(&output).contains("--retire"),
        "naming what it needs: {}",
        stderr(&output)
    );
}

/// A server that takes one request, hands back what was sent in its body, and
/// answers it with `id: 42` — a session's memo being taken, which only the
/// sessions suite can stand up for real.
fn taking_one_memo() -> (String, std::thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/conversations/1", listener.local_addr().unwrap());

    let taken = std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut reader = BufReader::new(stream.try_clone().unwrap());

        let mut length = 0;
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            if line.trim().is_empty() {
                break;
            }
            if let Some((name, value)) = line.split_once(':')
                && name.eq_ignore_ascii_case("content-length")
            {
                length = value.trim().parse().unwrap();
            }
        }

        let mut body = vec![0; length];
        reader.read_exact(&mut body).unwrap();

        let reply = "id: 42\n";
        let mut stream = stream;
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: application/yaml\r\nContent-Length: {}\r\n\
             Connection: close\r\n\r\n{reply}",
            reply.len()
        )
        .unwrap();

        String::from_utf8(body).unwrap()
    });

    (url, taken)
}

/// A memo the server takes is sent as its title, trimmed, and its markdown, and
/// comes back as its id on stdout and nothing else.
#[test]
fn a_titled_memo_is_sent_with_its_title_and_its_id_comes_back() {
    let (url, taken) = taking_one_memo();

    let output = memo_with(
        &["  The sessions suite flakes  "],
        &url,
        "Run it with `--test-threads=4`.\n",
    );

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), "id: 42\n");
    assert!(
        stderr(&output).contains("Verkstead has left memo 42"),
        "{}",
        stderr(&output)
    );

    let sent: verkstead_schema::MemoLeaving =
        serde_saphyr::from_str(&taken.join().unwrap()).unwrap();
    assert_eq!(
        sent,
        verkstead_schema::MemoLeaving {
            title: "The sessions suite flakes".to_owned(),
            markdown: "Run it with `--test-threads=4`.\n".to_owned(),
        }
    );
}
