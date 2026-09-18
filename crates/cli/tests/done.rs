//! `verkstead done`: the session saying its work is finished.
//!
//! What the server makes of a signal from a running session is the sessions
//! suite's to ask, because only a real session is one to end. What is asked
//! here is this end of it: that a refusal is a non-zero exit with the server's
//! own words on stderr, and nothing on stdout for an agent to mistake for an
//! acceptance.

mod support;

use std::process::{Command, Stdio};

use support::server::{Server, finished, stderr, stdout};

/// A Conversation with no session running is the one refusal a server with no
/// agents can give, and it has to say so rather than fail obscurely.
#[test]
fn a_signal_with_no_session_running_is_refused_saying_there_is_nothing_to_end() {
    let tmp = tempfile::tempdir().unwrap();
    let server = Server::start(tmp.path().join("verkstead.db"));

    let child = Command::new(env!("CARGO_BIN_EXE_verkstead"))
        .arg("done")
        .env("VERKSTEAD_SERVER", server.url())
        .current_dir(tmp.path())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the verkstead binary should be built for its own tests");

    let output = finished(child);

    assert!(
        !output.status.success(),
        "a refused signal is a non-zero exit, got {:?}",
        output.status
    );
    assert!(
        stderr(&output).contains("no session running in this Conversation"),
        "and says why on stderr, got:\n{}",
        stderr(&output)
    );
    assert!(
        stdout(&output).is_empty(),
        "with nothing on stdout, got:\n{}",
        stdout(&output)
    );
}
