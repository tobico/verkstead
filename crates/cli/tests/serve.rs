//! `verkstead serve`: the verb that runs the server out of the same binary the
//! agent asks with. What it is judged on is that the process an operator starts
//! answers the agent API, hands over the viewer, and is pointed at its socket
//! and its database exactly as the server binary this verb replaced was.

mod support;

use std::io::Write;
use std::net::TcpListener;
use std::path::Path;
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

use support::repo_with_a_commit;
use verkstead_schema::Response;

/// A Set small enough to be answered by the Response below, and legal without a
/// repository: `verkstead ask` derives `project` and `branch` from the working
/// directory and finds neither in a tempdir.
const SET: &str = "
title: Does the serve verb serve?
questions:
  - label: Q1
    text: Did the Response come back?
    options:
      - n: 1
        text: It did
        recommended: true
";

const ANSWER: &str = "
answers:
  - label: Q1
    selected: 1
";

/// How long a test waits on the server it just started before calling it dead.
const PATIENCE: Duration = Duration::from_secs(30);

/// A port nothing is listening on. Bound and released, so the server that takes
/// it is the next thing to claim it — the alternative is a test that fights
/// whatever is already on the default port.
fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

/// `verkstead serve` as an operator runs it: a child process with its own
/// database, killed when the test is done with it.
struct Serve {
    child: Option<Child>,
    url: String,
}

impl Serve {
    /// Start the verb with `args`, in `dir`, and block until it answers.
    ///
    /// `RUST_LOG` is dropped from the environment so the default filter is what
    /// the log assertions below are reading.
    fn start(dir: &Path, port: u16, args: &[&str], env: &[(&str, &str)]) -> Self {
        let mut command = Command::new(env!("CARGO_BIN_EXE_verkstead"));
        command
            .arg("serve")
            .args(args)
            .current_dir(dir)
            .env_remove("RUST_LOG")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        for (name, value) in env {
            command.env(name, value);
        }

        let child = command
            .spawn()
            .expect("the verkstead binary should be built for its own tests");
        let serving = Serve {
            child: Some(child),
            url: format!("http://127.0.0.1:{port}"),
        };
        serving.await_health();
        serving
    }

    /// The plain form: the flags every test but the environment one uses.
    ///
    /// `dir` is the Watched Path as well as the working directory. The server
    /// refuses to start without one, and a test that is not about the boundary
    /// still has to give it something real to watch.
    fn with_flags(dir: &Path, port: u16, database: &Path) -> Self {
        Self::start(
            dir,
            port,
            &[
                "--listen",
                &format!("127.0.0.1:{port}"),
                "--database",
                database.to_str().unwrap(),
                "--watched-path",
                dir.to_str().unwrap(),
            ],
            &[],
        )
    }

    fn await_health(&self) {
        let deadline = Instant::now() + PATIENCE;
        while ureq::get(format!("{}/api/v1/health", self.url))
            .call()
            .is_err()
        {
            assert!(
                Instant::now() < deadline,
                "`verkstead serve` never answered on {}",
                self.url
            );
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    /// A Conversation to ask from, made the way the workbench makes one: a Repo
    /// registered from inside the Watched Path, and a Conversation against it.
    ///
    /// Every Set is asked from one, and the base URL a session is given is what
    /// says which — so a test standing in for a session has to have one to be
    /// given.
    fn asking_from(&self, repo: &Path) -> i64 {
        let registered: serde_json::Value = self.through_the_viewer(
            "/api/ui/repos",
            &serde_json::json!({ "path": repo.to_str().unwrap() }),
        );
        assert_eq!(registered, serde_json::json!("Added"));

        let listed: serde_json::Value = serde_json::from_str(&self.read("/api/ui/repos")).unwrap();
        let repo_id = listed[0]["id"]
            .as_i64()
            .expect("the Repo was just registered");

        let started: serde_json::Value = self.through_the_viewer(
            "/api/ui/conversations",
            &serde_json::json!({ "repo_id": repo_id }),
        );

        started["Started"]["id"]
            .as_i64()
            .unwrap_or_else(|| panic!("the Conversation should have started: {started}"))
    }

    /// Tell the viewer's namespace something, in the JSON a browser would send.
    fn through_the_viewer(&self, path: &str, body: &serde_json::Value) -> serde_json::Value {
        let mut reply = ureq::post(format!("{}{path}", self.url))
            .header("Content-Type", "application/json")
            .send(body.to_string())
            .unwrap_or_else(|error| panic!("POST {path}: {error}"));

        assert_eq!(reply.status().as_u16(), 200);

        serde_json::from_str(&reply.body_mut().read_to_string().unwrap()).unwrap()
    }

    fn read(&self, path: &str) -> String {
        ureq::get(format!("{}{path}", self.url))
            .call()
            .unwrap_or_else(|error| panic!("GET {path}: {error}"))
            .body_mut()
            .read_to_string()
            .unwrap()
    }

    /// What a session on `conversation` is given as `VERKSTEAD_SERVER`.
    fn asking_url(&self, conversation: i64) -> String {
        format!("{}/conversations/{conversation}", self.url)
    }

    /// Answer Set `id` the way the human's device does, retrying until the Set
    /// the CLI is submitting has landed.
    fn await_answer(&self, conversation: i64, id: i64, yaml: &str) {
        let deadline = Instant::now() + PATIENCE;
        loop {
            let submitted = ureq::post(format!(
                "{}/api/v1/sets/{id}/response",
                self.asking_url(conversation)
            ))
            .header("Content-Type", "application/yaml")
            .send(yaml)
            .is_ok_and(|reply| reply.status().as_u16() == 201);
            if submitted {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "the CLI never submitted Question Set {id}"
            );
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    /// Stop serving and hand back what the server said on its way up.
    ///
    /// That is stdout: `tracing_subscriber::fmt` writes there by default, and
    /// where the log goes is what the retired server binary settled — the
    /// stdout an agent parses is `ask`'s, and no agent runs this verb.
    fn stop(&mut self) -> String {
        let mut child = self.child.take().expect("stopped once");
        child.kill().unwrap();
        let output = child.wait_with_output().unwrap();
        String::from_utf8_lossy(&output.stdout).into_owned()
    }
}

impl Drop for Serve {
    fn drop(&mut self) {
        if let Some(child) = self.child.as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

/// `verkstead ask`, pointed at the serving process as a session is — at the
/// Conversation it is asking from — and fed `SET` on stdin.
fn ask(serving: &Serve, conversation: i64, dir: &Path) -> Child {
    let mut child = Command::new(env!("CARGO_BIN_EXE_verkstead"))
        .arg("ask")
        .env("VERKSTEAD_SERVER", serving.asking_url(conversation))
        .current_dir(dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the verkstead binary should be built for its own tests");

    let mut stdin = child.stdin.take().unwrap();
    stdin.write_all(SET.as_bytes()).unwrap();
    drop(stdin);

    child
}

/// Run the binary to completion with `args` and hand back what it wrote.
fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_verkstead"))
        .args(args)
        .output()
        .expect("the verkstead binary should be built for its own tests")
}

/// Start the server with `args` and expect it to give up rather than serve.
///
/// The environment is cleared of what the flags stand in for, so a machine that
/// happens to have Verkstead configured for real is not what the test reads.
fn refused_to_start(args: &[&str]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_verkstead"))
        .arg("serve")
        .args(args)
        .env_remove("RUST_LOG")
        .env_remove("VERKSTEAD_WATCHED_PATHS")
        .env_remove("VERKSTEAD_DATABASE")
        .env_remove("VERKSTEAD_LISTEN")
        .output()
        .expect("the verkstead binary should be built for its own tests");

    assert!(
        !output.status.success(),
        "the server should not have started, got {:?}\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
    );

    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// The one piece of configuration with no default and no guess behind it: a
/// server that has not been told what it may touch has no business coming up,
/// because the alternative to a stated boundary is an assumed one.
#[test]
fn serving_without_a_watched_path_refuses_to_start() {
    let refusal = refused_to_start(&[]);

    assert!(
        refusal.contains("--watched-path"),
        "the refusal should name the flag that is missing, got:\n{refusal}"
    );
}

/// A Watched Path that is not there covers nothing, so every repo inside it
/// would be refused with no hint as to why. Said at startup instead, where it
/// can be fixed.
#[test]
fn serving_with_a_watched_path_that_is_not_there_refuses_to_start() {
    let tmp = tempfile::tempdir().unwrap();
    let missing = tmp.path().join("never-made");

    let refusal = refused_to_start(&[
        "--listen",
        &format!("127.0.0.1:{}", free_port()),
        "--database",
        tmp.path().join("unused.db").to_str().unwrap(),
        "--watched-path",
        missing.to_str().unwrap(),
    ]);

    assert!(
        refusal.contains(missing.to_str().unwrap()),
        "the refusal should name the path it could not resolve, got:\n{refusal}"
    );
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

#[test]
fn the_served_api_round_trips_an_ask() {
    let tmp = tempfile::tempdir().unwrap();
    let database = tmp.path().join("state/verkstead.db");
    let port = free_port();
    let mut serving = Serve::with_flags(tmp.path(), port, &database);

    let conversation = serving.asking_from(&repo_with_a_commit(tmp.path()));

    let waiting = ask(&serving, conversation, tmp.path());
    serving.await_answer(conversation, 1, ANSWER);

    let output = waiting.wait_with_output().unwrap();
    eprintln!(
        "verkstead ask stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "the ask should have been answered, got {:?}",
        output.status
    );

    let printed = String::from_utf8(output.stdout).unwrap();
    let response = Response::from_yaml(&printed)
        .unwrap_or_else(|error| panic!("stdout should be a Response: {error}\n{printed}"));
    assert_eq!(response.answers[0].selected, Some(1));

    assert!(
        database.exists(),
        "--database is where the store should have been created, and {} is not there",
        database.display()
    );

    serving.stop();
}

/// The viewer is the other half of what the verb serves: everything the API has
/// not claimed is the app's, so `/` is answered by the viewer rather than 404ed
/// by the router. In a checkout that has never run `pnpm build` there is nothing
/// to hand over, and saying so is still the viewer answering.
#[test]
fn the_viewer_is_served_beside_the_api() {
    let tmp = tempfile::tempdir().unwrap();
    let port = free_port();
    let mut serving = Serve::with_flags(tmp.path(), port, &tmp.path().join("verkstead.db"));

    match ureq::get(format!("{}/", serving.url)).call() {
        Ok(mut document) => {
            let body = document.body_mut().read_to_string().unwrap();
            assert!(
                body.contains("<html") || body.contains("<!DOCTYPE"),
                "`/` should be answered with the viewer's document, got:\n{body}"
            );
        }
        // Only reachable where `pnpm build` has never run: the viewer is
        // embedded from `web/dist`, and an absent one is served as an
        // explanation rather than as a missing route.
        Err(ureq::Error::StatusCode(503)) => {
            eprintln!("no viewer was built into the binary; `/` was answered 503")
        }
        Err(other) => panic!("`/` should be the viewer's to answer, got {other}"),
    }

    serving.stop();
}

#[test]
fn the_listen_address_and_database_come_from_the_environment_too() {
    let tmp = tempfile::tempdir().unwrap();
    let database = tmp.path().join("from-the-environment.db");
    let port = free_port();

    let mut serving = Serve::start(
        tmp.path(),
        port,
        &[],
        &[
            ("VERKSTEAD_LISTEN", &format!("127.0.0.1:{port}")),
            ("VERKSTEAD_DATABASE", database.to_str().unwrap()),
            ("VERKSTEAD_WATCHED_PATHS", tmp.path().to_str().unwrap()),
        ],
    );

    // Reaching the health endpoint at all is `VERKSTEAD_LISTEN` having been
    // honoured: `Serve::start` blocked until this port answered.
    assert!(
        database.exists(),
        "VERKSTEAD_DATABASE should have been created at {}",
        database.display()
    );

    serving.stop();
}

#[test]
fn the_help_describes_the_flags_and_their_defaults() {
    let help = stdout(&run(&["serve", "--help"]));

    for phrase in [
        "--listen",
        "--database",
        "--watched-path",
        "VERKSTEAD_LISTEN",
        "VERKSTEAD_DATABASE",
        "VERKSTEAD_WATCHED_PATHS",
        "127.0.0.1:8422",
        "verkstead.db",
    ] {
        assert!(
            help.contains(phrase),
            "`verkstead serve --help` should mention {phrase:?} — it is the whole \
             of what an operator reads before starting the server, got:\n{help}"
        );
    }
}

/// The verb is one subcommand among the agent-facing ones, and adding it must
/// not have cost the agent the thing bare `verkstead` does.
#[test]
fn serve_is_listed_without_displacing_the_guide() {
    let help = stdout(&run(&["--help"]));
    assert!(
        help.contains("serve"),
        "`verkstead --help` should list the verb, got:\n{help}"
    );

    let bare = run(&[]);
    assert!(
        bare.status.success() && stdout(&bare).contains("## The CLI contract"),
        "bare `verkstead` should still print the Guide, got:\n{}",
        stdout(&bare)
    );
}

#[test]
fn startup_logs_the_listen_address_and_the_database_path() {
    let tmp = tempfile::tempdir().unwrap();
    let database = tmp.path().join("logged.db");
    let port = free_port();
    let mut serving = Serve::with_flags(tmp.path(), port, &database);

    let logged = serving.stop();

    assert!(
        logged.contains(&format!("127.0.0.1:{port}")) && logged.contains("logged.db"),
        "the operator's one confirmation that the server came up where they \
         asked is this line, got:\n{logged}"
    );
}

#[test]
fn rust_log_overrides_the_default_filter() {
    let tmp = tempfile::tempdir().unwrap();
    let port = free_port();
    let mut serving = Serve::start(
        tmp.path(),
        port,
        &[
            "--listen",
            &format!("127.0.0.1:{port}"),
            "--database",
            tmp.path().join("quiet.db").to_str().unwrap(),
            "--watched-path",
            tmp.path().to_str().unwrap(),
        ],
        &[("RUST_LOG", "error")],
    );

    let logged = serving.stop();

    assert!(
        !logged.contains("listening"),
        "`RUST_LOG=error` should have silenced the startup line, got:\n{logged}"
    );
}
