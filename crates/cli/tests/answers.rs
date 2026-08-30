//! `verkstead answers`: coming back for the Response to a Set stored earlier.
//!
//! The far end of a store-and-nudge ask. The Set was stored and `verkstead ask`
//! came back at once; the session ended its turn; the human answered in their
//! own time; and this is the session coming back for what they said, in the
//! shape a blocking ask would have printed.
//!
//! **And the one stored ask this is not for.** A Deferred Ask's Answers go into
//! a later session's prompt by the agent's own word, so fetching one is refused
//! rather than served — see
//! [`a_deferred_ask_is_refused_because_its_answers_are_the_next_prompts`].

mod support;

use std::io::Write;
use std::path::Path;
use std::process::{Child, Command, Stdio};

use support::server::{ASKING_FROM, Server, finished, stderr, stdout};
use verkstead_schema::{Response, SetCreated};

/// One Question with Options, which is the whole of what a Response needs to
/// cover. The asking end is `ask.rs`'s subject; this file is about the fetch.
const SET: &str = "
title: Which channel does this backend ask on?
questions:
  - label: Q1
    text: Store and nudge, or block?
    options:
      - n: 1
        text: Store and nudge
        recommended: true
      - n: 2
        text: Block
";

/// A comment over more than one line, so the fetched YAML can be checked for
/// the block scalar the blocking ask renders.
const COMPLETE: &str = "
answers:
  - label: Q1
    selected: 1
comment: |
  Store and nudge.

  End the turn and come back for these.
";

/// A Set stored with a session idling on it, which is what every fetch below
/// comes back for.
///
/// Written through the store rather than asked through the CLI — see
/// [`Server::store_and_nudge`], which says why the channel cannot be reached
/// from this suite.
fn stored(server: &Server, set: &str) -> i64 {
    server.store_and_nudge(set)
}

/// And one asked with `--deferred`, which is the other stored kind and the one
/// nobody comes back for.
fn deferred(server: &Server, dir: &Path, set: &str) -> i64 {
    let mut child = Command::new(env!("CARGO_BIN_EXE_verkstead"))
        .arg("ask")
        .arg("--deferred")
        .env("VERKSTEAD_SERVER", server.url())
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

    let output = finished(child);
    assert!(output.status.success(), "the Set should have been stored");

    let created: SetCreated = serde_saphyr::from_str(&stdout(&output)).unwrap();
    created.id
}

/// `verkstead answers <id>`, pointed at the test server.
fn answers(server: &Server, dir: &Path, id: i64) -> Child {
    Command::new(env!("CARGO_BIN_EXE_verkstead"))
        .arg("answers")
        .arg(id.to_string())
        .env("VERKSTEAD_SERVER", server.url())
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the verkstead binary should be built for its own tests")
}

/// What this Conversation still owes a later session's prompt.
fn unfolded(server: &Server) -> Vec<i64> {
    server.block_on(async {
        let pool = verkstead_server::open_database(server.database())
            .await
            .unwrap();
        let unfolded = verkstead_server::store::unfolded(&pool, ASKING_FROM)
            .await
            .unwrap();
        pool.close().await;
        unfolded
            .into_iter()
            .map(|answered| answered.set_id)
            .collect()
    })
}

#[test]
fn an_answered_set_is_fetched_as_the_response_yaml_on_stdout() {
    let tmp = tempfile::tempdir().unwrap();
    let server = Server::start(tmp.path().join("verkstead.db"));

    let id = stored(&server, SET);
    server.answer(id, COMPLETE);

    let output = finished(answers(&server, tmp.path(), id));
    assert!(
        output.status.success(),
        "a fetched Response is a clean exit, got {:?}",
        output.status
    );

    let printed = stdout(&output);
    let response = Response::from_yaml(&printed)
        .unwrap_or_else(|error| panic!("stdout should be a Response: {error}\n{printed}"));
    assert_eq!(response.answers.len(), 1);
    assert_eq!(response.answers[0].selected, Some(1));

    assert!(
        printed.contains("comment: |"),
        "and it is rendered as the blocking ask renders one, got:\n{printed}"
    );
    assert!(
        stderr(&output).is_empty(),
        "a fetch that goes to plan says nothing at all, got:\n{}",
        stderr(&output)
    );
}

/// The promise the Guide rests on: an agent parses the two the same way,
/// because they are the same bytes.
#[test]
fn a_fetched_response_is_byte_for_byte_what_the_blocking_ask_prints() {
    let tmp = tempfile::tempdir().unwrap();
    let server = Server::start(tmp.path().join("verkstead.db"));

    // The blocking half, on a Set of its own: ask, answer, and keep what the
    // CLI printed.
    let mut waiting = Command::new(env!("CARGO_BIN_EXE_verkstead"))
        .arg("ask")
        .env("VERKSTEAD_SERVER", server.url())
        .current_dir(tmp.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = waiting.stdin.take().unwrap();
    stdin.write_all(SET.as_bytes()).unwrap();
    drop(stdin);
    server.await_asked_set(1);
    server.answer(1, COMPLETE);
    let blocked = stdout(&finished(waiting));

    // And the stored half, answered identically and fetched.
    let id = stored(&server, SET);
    server.answer(id, COMPLETE);
    let fetched = stdout(&finished(answers(&server, tmp.path(), id)));

    assert_eq!(
        fetched, blocked,
        "the fetch prints what the wait prints, so there is one Response shape \
         for the Guide to describe"
    );
}

#[test]
fn an_unanswered_set_is_refused_rather_than_waited_on() {
    let tmp = tempfile::tempdir().unwrap();
    let server = Server::start(tmp.path().join("verkstead.db"));

    let id = stored(&server, SET);

    // Nothing answers it: the command has to come back on its own, which is the
    // whole difference between this and the wait.
    let output = finished(answers(&server, tmp.path(), id));
    assert!(
        !output.status.success(),
        "there was no Response to print, got {:?}",
        output.status
    );
    assert!(
        stderr(&output).contains("has not been answered yet"),
        "the agent has to be told the Set is simply not answered yet, got:\n{}",
        stderr(&output)
    );
    assert!(
        stdout(&output).is_empty(),
        "a refusal has nothing to say on stdout, got:\n{}",
        stdout(&output)
    );
}

#[test]
fn a_set_locked_unanswered_is_refused_saying_no_response_is_coming() {
    let tmp = tempfile::tempdir().unwrap();
    let server = Server::start(tmp.path().join("verkstead.db"));

    let id = stored(&server, SET);
    server.lock(id);

    let output = finished(answers(&server, tmp.path(), id));
    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("locked unanswered")
            && stderr(&output).contains("no Response is coming"),
        "a locked Set is a different refusal from an unanswered one, got:\n{}",
        stderr(&output)
    );
}

#[test]
fn an_id_belonging_to_no_set_of_this_conversation_is_refused_by_name() {
    let tmp = tempfile::tempdir().unwrap();
    let server = Server::start(tmp.path().join("verkstead.db"));

    let output = finished(answers(&server, tmp.path(), 404));
    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("no Question Set 404"),
        "the refusal names the id that found nothing, got:\n{}",
        stderr(&output)
    );
}

/// A fetch is a delivery: the Answers reach the session that asked or a later
/// session's prompt, never both.
#[test]
fn a_successful_fetch_records_the_set_as_folded() {
    let tmp = tempfile::tempdir().unwrap();
    let server = Server::start(tmp.path().join("verkstead.db"));

    let id = stored(&server, SET);
    server.answer(id, COMPLETE);

    assert_eq!(
        unfolded(&server),
        vec![id],
        "answered and unread, these Answers are owed to the next session's prompt"
    );

    let output = finished(answers(&server, tmp.path(), id));
    assert!(output.status.success());

    assert!(
        unfolded(&server).is_empty(),
        "the session read them itself, so the prompt is not owed them as well"
    );
}

#[test]
fn a_refused_fetch_leaves_the_folding_record_alone() {
    let tmp = tempfile::tempdir().unwrap();
    let server = Server::start(tmp.path().join("verkstead.db"));

    let id = stored(&server, SET);

    // Refused: nothing has answered it yet, so nothing was delivered.
    assert!(!finished(answers(&server, tmp.path(), id)).status.success());

    server.answer(id, COMPLETE);
    assert_eq!(
        unfolded(&server),
        vec![id],
        "a fetch that handed nothing over cannot have spent the folding"
    );
}

/// And a Deferred Ask is refused whatever the human has said to it, because its
/// Answers are not this session's to take.
///
/// The one promise `--deferred` makes. The agent that sends one says it will
/// carry straight on, and what it is told in return is that the Answers go into
/// the prompt of a later session — so nothing *this* session does will ever see
/// them. Served here, that would be untrue twice over: the session that asked
/// would have them, and the folding spent on handing them over would mean the
/// later session never did.
#[test]
fn a_deferred_ask_is_refused_because_its_answers_are_the_next_prompts() {
    let tmp = tempfile::tempdir().unwrap();
    let server = Server::start(tmp.path().join("verkstead.db"));

    let id = deferred(&server, tmp.path(), SET);
    server.answer(id, COMPLETE);

    let output = finished(answers(&server, tmp.path(), id));
    assert!(
        !output.status.success(),
        "answered though it is, its Answers are not fetched, got {:?}",
        output.status
    );
    assert!(
        stderr(&output).contains("--deferred"),
        "and the refusal says which kind of ask it was, so the agent knows it \
         asked for this rather than that something is broken, got:\n{}",
        stderr(&output)
    );
    assert!(
        stdout(&output).is_empty(),
        "a refusal has nothing to say on stdout, got:\n{}",
        stdout(&output)
    );

    assert_eq!(
        unfolded(&server),
        vec![id],
        "and the folding is where it was: these Answers are still owed to the \
         next session's prompt, which is the whole of what was promised"
    );
}
