//! The `verkstead ask` round trip: a Set goes out, the CLI blocks, and the
//! human's Response comes back on stdout — across a server restart if need be.
//!
//! The last test here is the quickstart in `docs/development.md`, run against
//! the very files that guide tells the reader to run.

mod support;

use std::future::Future;
use std::io::Write;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

use support::{REPO_NAME, linked_worktree, repo_with_a_commit};
use verkstead_schema::{QuestionSet, Response, SetCreated};
use verkstead_server::store::{self, StoredSet};

/// Two Questions, one with a Sub-question, so a Response has to cover `Q1`,
/// `Q2` and `Q2a`. `project` and `branch` are forged: the CLI derives both
/// from the working directory and must overwrite whatever the agent claimed.
const SET: &str = "
title: How should the CLI wait?
preface: |
  The CLI holds a long-poll until the Response lands.

  It has no expiry: only delivery or a kill ends the wait.
project: forged-by-the-agent
branch: forged-by-the-agent
questions:
  - label: Q1
    text: How long a hold should the CLI ask for?
    options:
      - n: 1
        text: Thirty seconds
        recommended: true
      - n: 2
        text: Five minutes
  - label: Q2
    text: What should a dropped connection do?
    subquestions:
      - letter: a
        text: How long before the first retry?
";

/// A Response covering everything in [`SET`] there is to answer, with a
/// multi-line comment so the block-scalar rendering can be checked on the way
/// out.
///
/// `Q2` is not among them and must not be: it carries a Sub-question and no
/// Options, which makes it a Heading over `Q2a` rather than a question of its
/// own, and the grammar refuses a Response that answers one.
const COMPLETE: &str = "
answers:
  - label: Q1
    selected: 1
  - label: Q2a
    unanswered: true
comment: |
  Thirty seconds, and reconnect without saying anything on stdout.

  The agent parses stdout; keep the noise on stderr.
";

/// The Conversation these Sets are asked from, made by the server fixture over a
/// database with nothing in it — so it is always the first there is.
const ASKING_FROM: i64 = 1;

/// A third level of nesting, which the question grammar forbids. The CLI has
/// to refuse this itself, before anything reaches the server.
const THREE_LEVELS_DEEP: &str = "
title: Where should validation live?
questions:
  - label: Q1
    text: Who owns the retry?
    subquestions:
      - letter: a
        text: And who owns the backoff?
        subquestions:
          - letter: i
            text: A third level, which the grammar forbids.
";

/// The real server, on a runtime of its own, so a blocking test can kill it
/// under the CLI's feet and bring it back on the same port.
struct Server {
    addr: SocketAddr,
    database: PathBuf,
    runtime: tokio::runtime::Runtime,
}

impl Server {
    fn start(database: PathBuf) -> Self {
        Self::bind("127.0.0.1:0".parse().unwrap(), database)
    }

    fn bind(addr: SocketAddr, database: PathBuf) -> Self {
        Self::serve(addr, database)
    }

    fn serve(addr: SocketAddr, database: PathBuf) -> Self {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap();

        let (listener, addr, pool) = runtime.block_on(async {
            let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
            let addr = listener.local_addr().unwrap();
            let pool = verkstead_server::open_database(&database).await.unwrap();

            // Somewhere for the Sets to land. Every Set is asked from a
            // Conversation, and the base URL a session is given is what says
            // which — so a test standing in for a session has to be given the
            // same thing. Made only where there is none: this server is brought
            // up twice over one database, and the second time is a restart.
            if store::conversations(&pool).await.unwrap().is_empty() {
                let repo =
                    store::register_repo(&pool, Path::new("/srv/verkstead"), "verkstead", "main")
                        .await
                        .unwrap()
                        .expect("nothing is registered at that path yet");

                let conversation = store::start_conversation(&pool, repo.id, "api-core-and-cli")
                    .await
                    .unwrap()
                    .expect("the Repo was just registered");
                assert_eq!(conversation, ASKING_FROM);
            }

            (listener, addr, pool)
        });

        runtime.spawn(async move {
            let _ = axum::serve(listener, verkstead_server::router(pool)).await;
        });

        Server {
            addr,
            database,
            runtime,
        }
    }

    /// Where the server is, whole — the viewer's namespace hangs off this, and
    /// it is nobody's Conversation.
    fn base(&self) -> String {
        format!("http://{}", self.addr)
    }

    /// And what a session is given as `VERKSTEAD_SERVER`: the same server,
    /// scoped to the Conversation it is asking from.
    fn url(&self) -> String {
        format!("{}/conversations/{ASKING_FROM}", self.base())
    }

    fn block_on<F: Future>(&self, future: F) -> F::Output {
        self.runtime.block_on(future)
    }

    /// Stop serving without a graceful shutdown, so a held long-poll is
    /// dropped exactly as it would be if the process were killed. Hands back
    /// what [`Server::bind`] needs to bring the same server up again.
    fn kill(self) -> (SocketAddr, PathBuf) {
        let Server {
            addr,
            database,
            runtime,
        } = self;
        runtime.shutdown_timeout(Duration::from_millis(100));
        (addr, database)
    }

    /// The Set the CLI submitted, read back through a second pool on the same
    /// file — the store is where the enriched Set can actually be seen.
    fn stored_set(&self, id: i64) -> Option<StoredSet> {
        self.block_on(async {
            let pool = verkstead_server::open_database(&self.database)
                .await
                .unwrap();
            let stored = store::load_set(&pool, id).await.unwrap();
            pool.close().await;
            stored
        })
    }

    /// Block until the CLI has submitted Set `id`, and hand back what it asked.
    ///
    /// The Set itself rather than the row holding it: a stored body this build
    /// cannot read is a broken test rather than a case with anything to say
    /// here.
    fn await_asked_set(&self, id: i64) -> QuestionSet {
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            if let Some(stored) = self.stored_set(id) {
                return stored
                    .set
                    .set()
                    .expect("the Set the CLI just sent reads back")
                    .clone();
            }
            assert!(
                Instant::now() < deadline,
                "the CLI never submitted Question Set {id}"
            );
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    /// Answer a Set the way the human's device does: YAML over HTTP.
    fn answer(&self, id: i64, yaml: &str) {
        let reply = ureq::post(format!("{}/api/v1/sets/{id}/response", self.url()))
            .header("Content-Type", "application/yaml")
            .send(yaml)
            .unwrap();
        assert_eq!(reply.status().as_u16(), 201);
    }

    /// Lock a Set unanswered the way the human's browser does. It lives in the
    /// viewer's namespace and nowhere else: the agent API has no route for it,
    /// because only a human may close a Set nobody is going to answer.
    fn lock(&self, id: i64) {
        let reply = ureq::post(format!("{}/api/ui/sets/{id}/lock", self.base()))
            .header("Content-Type", "application/json")
            .send("{}")
            .unwrap();
        assert_eq!(reply.status().as_u16(), 200);
    }
}

/// `verkstead ask`, pointed at the test server and running in `dir`, with its
/// three streams on pipes the test can drive and read back.
fn command(server: &Server, dir: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_verkstead"));
    command
        .arg("ask")
        .env("VERKSTEAD_SERVER", server.url())
        .current_dir(dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command
}

/// Start `verkstead ask`, feeding it `set` on stdin, running in `dir`.
fn ask(server: &Server, dir: &Path, set: &str) -> Child {
    let mut child = command(server, dir)
        .spawn()
        .expect("the verkstead binary should be built for its own tests");

    // Dropping the handle closes the pipe, which is the CLI's end of input.
    let mut stdin = child.stdin.take().unwrap();
    stdin.write_all(set.as_bytes()).unwrap();
    drop(stdin);

    child
}

/// The same, deferred: the Set goes and the command comes back without waiting.
fn ask_deferred(server: &Server, dir: &Path, set: &str) -> Child {
    let mut child = command(server, dir)
        .arg("--deferred")
        .spawn()
        .expect("the verkstead binary should be built for its own tests");

    let mut stdin = child.stdin.take().unwrap();
    stdin.write_all(set.as_bytes()).unwrap();
    drop(stdin);

    child
}

/// Start `verkstead ask <file>`, the form the quickstart uses.
fn ask_file(server: &Server, dir: &Path, file: &Path) -> Child {
    command(server, dir)
        .arg(file)
        .stdin(Stdio::null())
        .spawn()
        .expect("the verkstead binary should be built for its own tests")
}

/// Cut a still-waiting CLI short — the wait has no other end — without leaving
/// a zombie behind.
fn kill(mut child: Child) {
    child.kill().unwrap();
    child.wait().unwrap();
}

/// What the CLI wrote and how it exited, insisting it exited at all.
fn finished(child: Child) -> Output {
    let output = child.wait_with_output().unwrap();
    eprintln!(
        "verkstead stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).unwrap()
}

#[test]
fn a_schema_violating_set_is_refused_locally_naming_the_question() {
    let tmp = tempfile::tempdir().unwrap();
    let server = Server::start(tmp.path().join("verkstead.db"));

    let output = finished(ask(&server, tmp.path(), THREE_LEVELS_DEEP));

    assert!(
        !output.status.success(),
        "an illegal Set should not be accepted"
    );
    assert!(
        stderr(&output).contains("Q1a"),
        "the refusal should name the offending Sub-question, got:\n{}",
        stderr(&output)
    );
    assert!(
        stdout(&output).is_empty(),
        "a refusal has nothing to say on stdout"
    );
    assert!(
        server.stored_set(1).is_none(),
        "an illegal Set must fail before anything is sent"
    );
}

#[test]
fn the_response_is_delivered_on_stdout_as_yaml() {
    let tmp = tempfile::tempdir().unwrap();
    let server = Server::start(tmp.path().join("verkstead.db"));

    let waiting = ask(&server, tmp.path(), SET);
    server.await_asked_set(1);
    server.answer(1, COMPLETE);

    let output = finished(waiting);
    assert!(
        output.status.success(),
        "delivery is a clean exit, got {:?}",
        output.status
    );

    let printed = stdout(&output);
    let response = Response::from_yaml(&printed)
        .unwrap_or_else(|error| panic!("stdout should be a Response: {error}\n{printed}"));
    assert_eq!(response.answers.len(), 2);
    assert_eq!(response.answers[0].selected, Some(1));
    assert!(response.answers[1].unanswered);

    assert!(
        printed.contains("comment: |"),
        "a multi-line comment comes out as a block scalar, got:\n{printed}"
    );
    assert!(
        !printed.contains("verkstead:"),
        "the CLI's own chatter belongs on stderr, got:\n{printed}"
    );
    assert!(
        stderr(&output).is_empty(),
        "a wait that goes to plan says nothing at all: a harness that merges the \
         two streams into one file is handed the Response alone, got:\n{}",
        stderr(&output)
    );
}

#[test]
fn the_cli_reconnects_when_the_server_restarts_mid_wait() {
    let tmp = tempfile::tempdir().unwrap();
    let server = Server::start(tmp.path().join("verkstead.db"));

    let waiting = ask(&server, tmp.path(), SET);
    server.await_asked_set(1);

    let (addr, database) = server.kill();
    std::thread::sleep(Duration::from_millis(250));
    let server = Server::bind(addr, database);

    server.answer(1, COMPLETE);

    let output = finished(waiting);
    assert!(
        output.status.success(),
        "the CLI should have ridden out the restart, got {:?}",
        output.status
    );
    assert!(
        Response::from_yaml(&stdout(&output)).is_ok(),
        "the Response should still arrive on stdout, got:\n{}",
        stdout(&output)
    );
    assert!(
        stderr(&output).contains("retrying"),
        "the reconnection should be reported on stderr, got:\n{}",
        stderr(&output)
    );

    // A harness running the CLI in the background captures both streams into
    // one file, so anything the CLI said on the way is read along with the
    // Response. Saying it as a YAML comment is what keeps that file parseable.
    let merged = format!("{}{}", stderr(&output), stdout(&output));
    assert!(
        Response::from_yaml(&merged).is_ok(),
        "the two streams merged should still parse as the Response, got:\n{merged}"
    );
}

#[test]
fn a_set_locked_unanswered_ends_the_wait_for_good() {
    let tmp = tempfile::tempdir().unwrap();
    let server = Server::start(tmp.path().join("verkstead.db"));

    let waiting = ask(&server, tmp.path(), SET);
    server.await_asked_set(1);

    // The human closes the Set instead of answering it — the CLI is holding a
    // wait at this moment, and that is the wait that has to end.
    server.lock(1);

    let output = finished(waiting);
    assert!(
        !output.status.success(),
        "no Response arrived, so this is not a clean exit, got {:?}",
        output.status
    );
    assert!(
        stderr(&output).contains("locked unanswered"),
        "the agent has to be told why it is not getting an answer, got:\n{}",
        stderr(&output)
    );
    assert!(
        !stderr(&output).contains("retrying"),
        "a locked Set is not a transient failure to reconnect through, got:\n{}",
        stderr(&output)
    );
    assert!(
        stdout(&output).is_empty(),
        "there is no Response to print, got:\n{}",
        stdout(&output)
    );
}

/// The whole of what a Deferred Ask is on this end: the Set is stored, the
/// command is over, and the session it was run from goes on working.
///
/// Nothing answers it here, deliberately. The point is that the CLI exits
/// without one — a test that answered first could not tell an ask that returned
/// from an ask that was answered very quickly.
#[test]
fn a_deferred_ask_returns_as_soon_as_the_set_is_stored() {
    let tmp = tempfile::tempdir().unwrap();
    let server = Server::start(tmp.path().join("verkstead.db"));

    let output = finished(ask_deferred(&server, tmp.path(), SET));
    assert!(
        output.status.success(),
        "a stored Set is a clean exit, got {:?}",
        output.status
    );

    // Which Set it is, and nothing else: there is no Response coming, so what
    // the agent is owed on stdout is the id the human's Answers will be filed
    // under.
    let printed = stdout(&output);
    let created: SetCreated = serde_saphyr::from_str(&printed)
        .unwrap_or_else(|error| panic!("stdout should be the stored Set: {error}\n{printed}"));

    assert_eq!(created.id, 1);
    assert!(
        Response::from_yaml(&printed).is_err(),
        "and it is not a Response — no Answers exist yet, got:\n{printed}"
    );
    assert!(
        stderr(&output).is_empty(),
        "nothing was waited on, so there was nothing to say about waiting, got:\n{}",
        stderr(&output)
    );

    // And it is a Set on the record like any other — the human answers it from
    // the same page, and nothing about the Set itself says how it was sent.
    let stored = server.stored_set(created.id).expect("the Set was stored");
    let asked = stored.set.set().expect("it reads back").clone();

    assert_eq!(asked.title, "How should the CLI wait?");
    assert!(
        stored.deferred,
        "and the record beside it says which kind of ask it was"
    );
}

#[test]
fn the_project_and_the_branch_are_derived_from_the_working_directory() {
    let tmp = tempfile::tempdir().unwrap();
    let server = Server::start(tmp.path().join("verkstead.db"));

    let root = repo_with_a_commit(tmp.path());
    let linked = linked_worktree(&root, "feature");

    let asking = ask(&server, &linked, SET);
    let stored = server.await_asked_set(1);
    kill(asking);

    assert_eq!(
        stored.project.as_deref(),
        Some(REPO_NAME),
        "from a linked worktree the project is the root repo's name"
    );
    assert_eq!(stored.branch.as_deref(), Some("feature"));
}

#[test]
fn the_diff_is_not_the_clis_to_send_however_dirty_the_working_directory_is() {
    let tmp = tempfile::tempdir().unwrap();
    let server = Server::start(tmp.path().join("verkstead.db"));

    let root = repo_with_a_commit(tmp.path());
    let linked = linked_worktree(&root, "feature");

    // Dirty enough that the CLI would once have carried it: an untracked file
    // with contents in it.
    std::fs::write(
        linked.join("open-questions.md"),
        "a line only in the working tree\n",
    )
    .unwrap();

    let asking = ask(&server, &linked, SET);
    let stored = server.await_asked_set(1);
    kill(asking);

    // The Conversation these Sets are asked from has no Worktree of its own, so
    // the server — which is what reads one now — has nothing to attach. That
    // this directory is dirty no longer says anything about the Set.
    assert_eq!(
        stored.diff, None,
        "the Diff comes from the Conversation's Worktree, not the CLI's, got {:?}",
        stored.diff
    );
}

/// One of the workspace's `examples/`, by the name the guide uses.
fn example(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(name)
}

#[test]
fn the_quickstart_delivers_the_example_response() {
    let tmp = tempfile::tempdir().unwrap();
    let server = Server::start(tmp.path().join("verkstead.db"));

    // `verkstead ask examples/questions.yaml`, and the Set is the first one
    // this server has seen, so the id the quickstart curls is 1.
    let waiting = ask_file(&server, tmp.path(), &example("questions.yaml"));
    let stored = server.await_asked_set(1);
    assert_eq!(stored.title, "Rate limiting for the public API");

    // The curl step. `answer` insists on a 201, so this is also where an
    // example Response that failed to resolve the example Set would be caught.
    let submitted = std::fs::read_to_string(example("response.yaml")).unwrap();
    server.answer(1, &submitted);

    let output = finished(waiting);
    assert!(
        output.status.success(),
        "the quickstart ends with the CLI exiting 0, got {:?}",
        output.status
    );

    let printed = stdout(&output);
    let response = Response::from_yaml(&printed)
        .unwrap_or_else(|error| panic!("stdout should be a Response: {error}\n{printed}"));
    assert_eq!(
        response,
        Response::from_yaml(&submitted).unwrap(),
        "the agent should get back exactly what the human submitted"
    );
}
