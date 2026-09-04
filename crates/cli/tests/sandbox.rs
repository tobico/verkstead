//! The CLI put to work where a session actually runs it: inside a Conversation's
//! sandbox, against the server that equipped it.
//!
//! Every other test here runs the binary on the host, which proves the round
//! trip and says nothing about which binary a session would have found. What
//! this file is for is the other half — that a sandbox hands a session the
//! executable serving it, and that the two halves of an ask, being one build,
//! agree about what a Question Set may carry.
//!
//! The closing move of a grilling is the case that broke: a Set carrying a
//! `proposal`, refused by a running server because the CLI on the machine was a
//! different build with a different idea of the block. So that is the Set this
//! sends, written exactly as the bundled grilling skill tells a session to write
//! one, with nothing built by hand.
//!
//! On the platforms a session runs on: what this puts the CLI inside is a real
//! sandbox, and a Windows Verkstead has none — see the server's `sandbox`.
#![cfg(unix)]

mod support;

use std::io::Write;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use support::repo_with_a_commit;
use verkstead_schema::{Direction, QuestionSet, Response};
use verkstead_server::build_cache::BuildCache;
use verkstead_server::handoffs::Handoffs;
use verkstead_server::platform::Platform;
use verkstead_server::sandbox::{Executable, Homes, Reachable, Sandbox};
use verkstead_server::settings::Settings;
use verkstead_server::skills::Skills;
use verkstead_server::store;

/// The grilling's closing Set, in the shape the bundled skill writes: an
/// ordinary Set with a `proposal` block naming a direction and the reasoning
/// behind it.
const CLOSING_SET: &str = "
title: Ready to build the rate limiter
preface: |
  I think we have this. Here is what we settled.
questions:
  - label: Q1
    text: Anything in the above you want changed before we build it?
proposal:
  direction: task-list
  rationale: |
    Six changes across the limiter, the config and the migration, each
    independently testable.
";

/// And the human accepting it: the direction picked, which is the whole of
/// accepting, and the one Question answered in their own words.
const ACCEPTED: &str = "
direction: task-list
answers:
  - label: Q1
    free_text: Nothing. Go.
";

/// A Conversation part-way through its first grilling, and the server it is
/// asking.
///
/// Everything is real: a repository git made, a worktree git added, a row the
/// store wrote, and a server listening on the loopback the sandbox shares. The
/// one thing standing in for anything is the session itself — this test is the
/// session, running the command a session runs.
struct Grilling {
    /// Kept alive for as long as the fixture is: the directories go when these
    /// drop, and a worktree that vanished mid-ask would fail obscurely.
    watched: tempfile::TempDir,
    state: tempfile::TempDir,
    home: tempfile::TempDir,

    conversation: store::Conversation,
    profile: store::Profile,

    listening: SocketAddr,
    database: PathBuf,

    /// The runtime the server is on, which is not this test's: the CLI is a
    /// blocking child, and the server has to go on answering while it waits.
    runtime: tokio::runtime::Runtime,
}

impl Grilling {
    /// The sandbox this Conversation's session would run in, equipped with the
    /// binary the server would equip it with.
    fn sandbox(&self) -> Sandbox {
        self.sandbox_under(&self.profile)
    }

    /// And one around a Profile of another agent type, which is how the Guide's
    /// tailoring is asked of a real sandbox rather than of an environment a test
    /// set by hand.
    fn sandbox_under(&self, profile: &store::Profile) -> Sandbox {
        let settings = Settings::in_data_dir(self.state.path());

        Sandbox::for_conversation(
            &self.conversation,
            profile,
            &Homes::on(
                Platform::HERE,
                self.home.path().to_owned(),
                self.state.path(),
            ),
            &Reachable::at(self.listening),
            &Skills::installed(self.state.path()).expect("this binary carries skills"),
            // What a real server hands over is its own image. A test harness's
            // own image is the test harness — so this names the binary cargo
            // built from this crate, which is the same thing a `verkstead serve`
            // would be running.
            &Executable::at(
                PathBuf::from(env!("CARGO_BIN_EXE_verkstead")),
                self.state.path(),
            )
            .expect("cargo builds this crate's binary for its own tests"),
            &Handoffs::under(self.state.path()),
            &settings.secrets(),
            &settings.config(),
            // What this asks is whether the bundled CLI reaches its server, so
            // there is nothing here to build and no cache to build it into.
            &BuildCache::none(),
            vec![],
        )
        .expect("a grilling Conversation has a worktree to build a sandbox around")
    }

    /// A Profile whose whole account is one home, saved into the same store the
    /// fixture's own was — the second agent type, which asks by store-and-nudge.
    fn codex_profile(&self) -> store::Profile {
        let home = self.watched.path().join("codex-account/.codex");
        std::fs::create_dir_all(&home).unwrap();

        self.runtime.block_on(async {
            let pool = verkstead_server::open_database(&self.database)
                .await
                .unwrap();
            let profile = store::create_profile(
                &pool,
                &store::ProfileFacts {
                    name: "codex".to_owned(),
                    account: store::Account::Codex { home },
                    models: vec!["gpt-5-codex".to_owned()],
                },
            )
            .await
            .unwrap()
            .expect("nothing is called that yet");
            pool.close().await;
            profile
        })
    }

    /// Run `argv` inside the sandbox and hand back what it printed, insisting it
    /// worked.
    fn inside(&self, argv: &[&str]) -> String {
        self.inside_sandbox(&self.sandbox(), argv)
    }

    /// The same inside a sandbox of the caller's choosing.
    fn inside_sandbox(&self, sandbox: &Sandbox, argv: &[&str]) -> String {
        let output = Command::from(&sandbox.command(argv))
            .stdin(Stdio::null())
            .output()
            .expect("bwrap should be on the PATH: the dev shell declares bubblewrap");

        assert!(
            output.status.success(),
            "{argv:?} failed inside the sandbox: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        String::from_utf8_lossy(&output.stdout).trim().to_owned()
    }

    /// Block until Question Set `id` has been submitted, and hand back what it
    /// asked.
    ///
    /// The Set itself rather than the row holding it: a stored body this build
    /// cannot read is a broken test rather than a case with anything to say
    /// here.
    fn await_asked_set(&self, id: i64) -> QuestionSet {
        let deadline = Instant::now() + Duration::from_secs(30);

        loop {
            let stored = self.runtime.block_on(async {
                let pool = verkstead_server::open_database(&self.database)
                    .await
                    .unwrap();
                let stored = store::load_set(&pool, id).await.unwrap();
                pool.close().await;
                stored
            });

            if let Some(stored) = stored {
                return stored
                    .set
                    .set()
                    .expect("the Set the session just sent reads back")
                    .clone();
            }

            assert!(
                Instant::now() < deadline,
                "the session never submitted Question Set {id}"
            );
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    /// Answer a Set the way the human's device does: YAML over HTTP, to the
    /// Conversation the Set was asked from.
    fn answer(&self, id: i64, yaml: &str) {
        let reply = ureq::post(format!(
            "http://{}/conversations/{}/api/v1/sets/{id}/response",
            self.listening, self.conversation.id
        ))
        .header("Content-Type", "application/yaml")
        .send(yaml)
        .unwrap();

        assert_eq!(reply.status().as_u16(), 201);
    }
}

/// Stand one up.
fn grilling() -> Grilling {
    let watched = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    let repo = repo_with_a_commit(watched.path());
    let database = state.path().join("verkstead.db");

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .unwrap();

    // The account a session runs under: a Profile is the pair of files, and both
    // are bound into the sandbox, so both have to be there.
    let claude_dir = watched.path().join("account/.claude");
    let config_file = watched.path().join("account/.claude.json");
    std::fs::create_dir_all(&claude_dir).unwrap();
    std::fs::write(&config_file, "{}\n").unwrap();

    let worktree = state.path().join("worktrees/rate-limiting");
    std::fs::create_dir_all(worktree.parent().unwrap()).unwrap();
    let commit = head(&repo);
    support::git(
        &repo,
        &[
            "worktree",
            "add",
            "--quiet",
            "-b",
            "rate-limiting",
            worktree.to_str().unwrap(),
            &commit,
        ],
    );

    let (listening, conversation, profile) = runtime.block_on(async {
        let pool = verkstead_server::open_database(&database).await.unwrap();

        let repo_row = store::register_repo(&pool, &repo, support::REPO_NAME, "main")
            .await
            .unwrap()
            .expect("nothing is registered at that path yet");

        let profile = store::create_profile(
            &pool,
            &store::ProfileFacts {
                name: "work".to_owned(),
                account: store::Account::Claude {
                    claude_dir,
                    config_file,
                },
                models: vec!["claude-opus-5".to_owned()],
            },
        )
        .await
        .unwrap()
        .expect("the Profile saves");

        let id = store::start_conversation(&pool, repo_row.id, "rate-limiting")
            .await
            .unwrap()
            .expect("the Repo was just registered");

        store::set_grilling_pairing(&pool, id, profile.id, None)
            .await
            .unwrap();
        store::set_implementation_pairing(&pool, id, profile.id, None)
            .await
            .unwrap();
        store::start_grilling(&pool, id, &commit, &worktree, &[])
            .await
            .unwrap();

        let conversation = store::load_conversation(&pool, id)
            .await
            .unwrap()
            .expect("the Conversation is there");

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let listening = listener.local_addr().unwrap();

        tokio::spawn(async move {
            let _ = axum::serve(listener, verkstead_server::router(pool)).await;
        });

        (listening, conversation, profile)
    });

    Grilling {
        watched,
        state,
        home,
        conversation,
        profile,
        listening,
        database,
        runtime,
    }
}

/// The commit a worktree is added at.
fn head(repo: &Path) -> String {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo)
        .output()
        .unwrap();

    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

/// A bare `verkstead` inside a sandbox is the build serving it, so the Guide a
/// session reads documents the schema the server will take.
///
/// Asked of the binary itself rather than of the mount table: what settles which
/// build a session found is the build saying which it is.
#[test]
fn the_binary_a_session_finds_is_the_one_that_equipped_it() {
    let fixture = grilling();

    assert_eq!(
        fixture.inside(&["verkstead", "--version"]),
        format!("verkstead {}", env!("CARGO_PKG_VERSION")),
        "a session runs the executable the server handed it, not the machine's install"
    );

    let guide = fixture.inside(&["verkstead", "guide"]);

    assert!(
        guide.contains("Question Set"),
        "and the Guide it prints comes out of that same build, got:\n{guide}"
    );
}

/// And the Guide it finds is the one for the backend it is running.
///
/// Asked inside a real sandbox rather than of a variable a test set, because
/// what has to hold is that the sandbox says which backend a session is — a
/// Guide tailored off an environment nothing sets is a Guide every session
/// reads as blocking.
#[test]
fn a_session_reads_the_guide_for_the_backend_it_is_running() {
    let fixture = grilling();

    let blocking = fixture.inside(&["verkstead", "guide"]);

    assert!(
        blocking.contains("run_in_background"),
        "a Claude session reads the blocking ask's own mechanics, got:\n{blocking}"
    );
    // What it is never sent to do, rather than a command it never names: the CLI
    // contract every channel quotes says `verkstead answers` refuses a Deferred
    // Ask, which is worth a blocking session's knowing too.
    assert!(
        !blocking.contains("Fetch the Answers"),
        "and is never sent to fetch a Response it is already holding, got:\n{blocking}"
    );

    let codex = fixture.codex_profile();
    let store_and_nudge =
        fixture.inside_sandbox(&fixture.sandbox_under(&codex), &["verkstead", "guide"]);

    assert!(
        store_and_nudge.contains("Fetch the Answers"),
        "a session that cannot hold an ask open reads that it ends its turn and \
         fetches the Answers when the nudge lands, got:\n{store_and_nudge}"
    );
    assert!(
        !store_and_nudge.contains("run_in_background"),
        "and none of the hold-the-ask advice, which is false of it"
    );
}

/// The closing move of a grilling, put through by the binary a session is
/// equipped with: a Set carrying a `proposal`, accepted by the server that
/// equipped it, answered with the direction that accepts it.
///
/// This is the round trip that could not be made at all while a session asked
/// with the machine's install — that binary validated a `proposal` against a
/// field this server refuses as unknown, so the Set died at one end or the
/// other whichever way round it was sent.
#[test]
fn a_set_carrying_a_proposal_goes_through_from_inside_a_sandbox() {
    let fixture = grilling();

    let mut asking = Command::from(&fixture.sandbox().command(&["verkstead", "ask"]))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("bwrap should be on the PATH: the dev shell declares bubblewrap");

    // Dropping the handle closes the pipe, which is the CLI's end of input.
    let mut stdin = asking.stdin.take().unwrap();
    stdin.write_all(CLOSING_SET.as_bytes()).unwrap();
    drop(stdin);

    let stored = fixture.await_asked_set(1);

    assert_eq!(
        stored.proposal.as_ref().map(|proposal| proposal.direction),
        Some(Direction::TaskList),
        "the server took the proposal as written"
    );
    assert_eq!(
        stored.title, "Ready to build the rate limiter",
        "and the Set it belongs to landed on this Conversation's Timeline"
    );

    fixture.answer(1, ACCEPTED);

    let output = asking.wait_with_output().unwrap();
    let printed = String::from_utf8(output.stdout).unwrap();

    assert!(
        output.status.success(),
        "delivery is a clean exit, got {:?} with stderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );

    let response = Response::from_yaml(&printed)
        .unwrap_or_else(|error| panic!("the Response should parse: {error}\n{printed}"));

    assert_eq!(
        response.direction,
        Some(Direction::TaskList),
        "and the session is told which direction was picked, which is the proposal accepted"
    );
}
