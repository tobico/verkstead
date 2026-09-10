//! The CLI put to work where a Windows session actually runs it: inside its
//! Conversation's own rendering, against a server it can reach one way only.
//!
//! `tests/sandbox.rs` is this file's Unix sibling and asks the same thing of the
//! other machine — that a session runs the executable the server equipped it
//! with, and that the ask it makes with it goes through. What is different here
//! is the transport. A Windows session is told the **named pipe** the server
//! opened beside its socket: it was the one way in when that platform's boundary
//! was an AppContainer, which is refused the loopback interface, and it stays
//! because it is landed and is the one transport no firewall on the human's
//! machine has to agree with (ADR-0014). This is where that ends up being true
//! of a real `verkstead ask` rather than of a client a test built by hand.
//!
//! **Nothing is listening on TCP here.** The server in this file is a router
//! served over a pipe and nothing else, and the socket address the sandbox is
//! built around is a port nothing is bound to — so a session that reached for
//! the URL would fail rather than quietly pass. What answers the Set is the
//! human's own route, asked of the same router in this process, which is the
//! one way in that does not put a socket back.
//!
//! **And the loopback is open again, which is what the account changed.** A
//! session runs as a local account of Verkstead's own now, and an ordinary local
//! account reaches `127.0.0.1` like anything else on the machine — so
//! [`the_loopback_answers_a_session`] binds a socket that is genuinely
//! listening and reaches it from behind the boundary. It is asserted rather than
//! assumed because the Compile Server depends on it: an sccache is a client and
//! a server talking over the loopback, and it was refused for want of that.
//!
//! **A session really runs behind that boundary here.** A Windows rendering
//! names the account it is started as, which `std::process::Command` has no way
//! to start — so what runs the CLI here is [`off_a_console`], the same call the
//! server's own [`Sandbox`] work stands on. Which makes the pipe's descriptor
//! load-bearing in this file rather than merely chosen: the pipe is opened
//! granting that account, and a pipe opened granting nobody refuses it.
//!
//! **It needs the session account**, which is an administrator's call to make
//! and so not one this suite can make for itself: a machine where
//! `verkstead session-account create` has never been run fails here with the
//! line that names it, rather than passing quietly.
//!
//! **A probe behind the boundary is written without cmdlets**, which is what the
//! `windows-2025` job answered the first time one ran there. Windows PowerShell
//! starts behind one and parses and runs what it is given, and the commands it
//! would ordinarily import from a module at startup are not there: `New-Object`
//! and `Write-Output` alike came back `CommandNotFoundException`. So the probe
//! below is the language and the framework and nothing else — a constructor
//! rather than `New-Object`, `[Console]::Out` rather than `Write-Output` — which
//! is what a program has behind a boundary whatever the shell managed to load.

//!
//! **Why here rather than in the server crate's Windows sessions suite.** That
//! suite cannot run this command at all: it is a server-crate test, so what a
//! session finds first on its `PATH` is the test binary's own directory and
//! there is no `verkstead.exe` in it. This crate builds one, and the
//! `windows-2025` job runs the whole workspace.
#![cfg(windows)]

mod support;

use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, Instant};

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use support::repo_with_a_commit;
use tower::ServiceExt;
use verkstead_schema::{QuestionSet, Response};
use verkstead_server::attachments::Attachments;
use verkstead_server::build_cache::BuildCache;
use verkstead_server::handoffs::Handoffs;
use verkstead_server::platform::{self, Platform};
use verkstead_server::sandbox::account::Logon;
use verkstead_server::sandbox::account::machine::Account;

use verkstead_server::sandbox::{Executable, Homes, Reachable, Rendering, Sandbox, off_a_console};
use verkstead_server::settings::Settings;
use verkstead_server::skills::Skills;
use verkstead_server::store;

/// Where this server would be listening if it were listening on a socket at
/// all, which it is not.
///
/// Port 1 rather than a port a listener was taken down from: what makes this
/// test's answer worth anything is that there is nothing on TCP for the session
/// to have used, and a port nothing binds is the plainest way to say so.
const NOWHERE: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 1);

/// One Question, because what is under test is the transport a session asks
/// through rather than the grammar — `ask.rs` is where a Set's shape is proved.
const SET: &str = "
title: Which transport did this session ask through?
preface: |
  Asked from inside the rendering a Windows session runs in.
questions:
  - label: Q1
    text: Did this reach you?
";

/// And the human's answer to it.
const ANSWERED: &str = "
answers:
  - label: Q1
    free_text: It did, over the pipe.
";

/// How long a connection to a socket on this machine is given from out here,
/// where it is a loopback connection to a listener in this process and takes no
/// time at all. Long only by the standards of what it is measuring.
const OUT_HERE: Duration = Duration::from_secs(5);

/// And from behind the boundary, which is the same connection made by another
/// account of the same machine.
///
/// A wait rather than a timeout, and generous: what a failing run has to be able
/// to say is that nothing arrived rather than that this file was impatient. Well
/// past the round trip out here, which is a fraction of a millisecond on the
/// same machine, and well short of the twenty-odd seconds Windows spends
/// retrying a connection nothing answers.
const INSIDE: Duration = Duration::from_secs(10);

/// The two words the probe behind the boundary prints, one of which it is.
const REACHED: &str = "the loopback answered";
const REFUSED: &str = "the loopback did not answer";

/// A Conversation part-way through its first grilling, and the server it is
/// asking — which it can only ask over a pipe.
///
/// Everything is real: a repository git made, a worktree git added, a row the
/// store wrote, a profile the rendering joins the account into, and a router
/// served over the pipe the server would have opened. The one thing standing in
/// for anything is the session itself — this test is the session, running the
/// command a session runs.
struct Grilling {
    /// Kept alive for as long as the fixture is: the directories go when these
    /// drop, and a worktree that vanished mid-ask would fail obscurely.
    _work: tempfile::TempDir,
    state: tempfile::TempDir,
    home: tempfile::TempDir,

    conversation: store::Conversation,
    profile: store::Profile,

    /// The pipe in the spelling a client is told it in, which is what a session
    /// is handed and the whole of how one reaches this server.
    pipe: String,

    database: PathBuf,

    /// The router the pipe is served with, kept so the human's own route can be
    /// asked of the same state a waiting session is held by — see this file's
    /// own documentation for why it is not asked over a socket.
    app: Router,

    /// The runtime the server is on, which is not this test's: the CLI is a
    /// blocking child, and the server has to go on answering while it waits.
    runtime: tokio::runtime::Runtime,
}

impl Grilling {
    /// The sandbox this Conversation's session would run in, equipped with the
    /// binary the server would equip it with and told where the server is.
    fn sandbox(&self) -> Sandbox {
        let settings = Settings::in_data_dir(self.state.path());

        Sandbox::for_conversation(
            &self.conversation,
            &self.profile,
            // The account said outright, which is the one thing a temporary Data
            // Directory cannot name for itself — see [`the_machines_account`].
            &Homes::on(
                Platform::HERE,
                self.home.path().to_owned(),
                self.state.path(),
            )
            .running_sessions_as(the_machines_account().name()),
            // The pipe beside an address nothing answers on: what a Windows
            // session is given is the pipe, and this is where that is decided.
            &Reachable::at(NOWHERE).piped(&self.pipe),
            &Skills::installed(Platform::HERE, self.state.path())
                .expect("this binary carries skills"),
            // What a real server hands over is its own image. A test harness's
            // own image is the test harness — so this names the binary cargo
            // built from this crate, which is the same thing a `verkstead serve`
            // would be running.
            &Executable::at(
                Platform::HERE,
                PathBuf::from(env!("CARGO_BIN_EXE_verkstead")),
                self.state.path(),
            )
            .expect("cargo builds this crate's binary for its own tests"),
            &Handoffs::under(self.state.path()),
            &Attachments::under(self.state.path()),
            &settings.secrets(),
            &settings.config(),
            // What this asks is whether the bundled CLI reaches its server, so
            // there is nothing here to build and no cache to build it into.
            &BuildCache::none(),
            vec![],
        )
        .expect("a grilling Conversation has a worktree to build a sandbox around")
    }

    /// What a session started right now would find in its environment under
    /// `name`.
    fn given(&self, name: &str) -> String {
        let (rendering, _closing) = self
            .sandbox()
            .command(&["verkstead", "guide"])
            .expect("a session's sandbox to be one this machine can make");

        rendering
            .env()
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.to_string_lossy().into_owned())
            .unwrap_or_else(|| panic!("a session is given no {name} at all"))
    }

    /// Block until Question Set `id` has been submitted, and hand back what it
    /// asked.
    ///
    /// The Set itself rather than the row holding it: a stored body this build
    /// cannot read is a broken test rather than a case with anything to say
    /// here.
    fn await_asked_set(&self, id: i64) -> QuestionSet {
        let deadline = Instant::now() + Duration::from_secs(60);

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

    /// Whether the server has been asked nothing at all, which is what a
    /// session that could not open the pipe leaves behind.
    ///
    /// The store rather than the CLI's own complaint: what is being asked is
    /// whether the Set got through, and a session refused the transport is one
    /// whose Set was never written down.
    fn nothing_was_asked(&self) -> bool {
        self.runtime.block_on(async {
            let pool = verkstead_server::open_database(&self.database)
                .await
                .unwrap();
            let stored = store::load_set(&pool, 1).await.unwrap();
            pool.close().await;

            stored.is_none()
        })
    }

    /// Answer a Set the way the human's device does: YAML through the
    /// Conversation the Set was asked from, which is the route their browser
    /// posts to.
    ///
    /// Asked of the router in this process rather than over a socket, there
    /// being no socket — and of the *same* router the pipe is served with, so a
    /// session held on the long poll is woken by this rather than left to find
    /// out when its hold runs out.
    ///
    /// A Set that belongs to another Conversation is nothing to this route, so
    /// the 201 is also what says the Set landed on this Conversation's Timeline.
    fn answer(&self, id: i64, yaml: &str) {
        let asked_from = self.conversation.id;
        let request = Request::builder()
            .method("POST")
            .uri(format!(
                "/conversations/{asked_from}/api/v1/sets/{id}/response"
            ))
            .header(header::CONTENT_TYPE, "application/yaml")
            .body(Body::from(yaml.to_owned()))
            .unwrap();

        let reply = self
            .runtime
            .block_on(async { self.app.clone().oneshot(request).await.unwrap() });

        assert_eq!(
            reply.status(),
            StatusCode::CREATED,
            "the human's own route should take the Response to Set {id} of \
             Conversation {asked_from}"
        );
    }
}

/// Stand one up, its pipe granting the account a session runs as — which is
/// what a real server's does.
fn grilling() -> Grilling {
    standing(true)
}

/// And one whose pipe grants nobody beyond the server itself.
///
/// Which is not a thing a real Verkstead does where there is an account to
/// grant, and is the only way to ask, on a machine where the test and the
/// server are one account, whether the descriptor is what lets a session in.
fn a_pipe_granting_nobody() -> Grilling {
    standing(false)
}

/// One Conversation part-way through a grilling, its pipe granting the session
/// account where `granting` says so.
fn standing(granting: bool) -> Grilling {
    let work = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    // And the password of the account this session runs as, written into this
    // Data Directory's own secrets — the other half of running as the machine's
    // account, whose name goes on the `Homes`. See [`the_machines_account`].
    let running_as = the_machines_account();
    let settings = Settings::in_data_dir(state.path());

    settings
        .save_secrets(
            &settings
                .secrets()
                .with_session_account_password(Some(running_as.password().to_owned())),
        )
        .expect("a secrets file to be writable under a temporary Data Directory");

    let repo = repo_with_a_commit(work.path());
    let database = state.path().join("verkstead.db");

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .unwrap();

    // The account a session runs under: a Profile is the pair of files, and both
    // are joined into the session's profile, so both have to be there.
    let claude_dir = work.path().join("account/.claude");
    let config_file = work.path().join("account/.claude.json");
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

    let (pipe, app, conversation, profile) = runtime.block_on(async {
        let pool = verkstead_server::open_database(&database).await.unwrap();

        let repo_row = store::register_repo(&pool, &repo, support::REPO_NAME, "main")
            .await
            .unwrap()
            .expect("nothing is registered at that path yet");

        let profile = store::create_profile(
            &pool,
            &store::ProfileFacts {
                name: Some("work".to_owned()),
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

        // The pipe the server would have opened, named after the Data Directory
        // the database is in — and opened on this runtime, because tokio's
        // pipes register with its reactor.
        // The identity the pipe is opened granting: the account every session of
        // this installation runs as, resolved the way the server resolves it at
        // startup.
        let granted = granting.then(|| {
            Account::resolving(&running_as)
                .expect("the account resolved a moment ago")
                .sid()
                .text()
                .to_owned()
        });

        let listener = verkstead_server::pipe::Listener::open(state.path(), granted.as_deref())
            .expect("nothing else holds this Data Directory's pipe");

        let pipe = listener.asked_through().to_owned();

        let app = verkstead_server::router(pool);
        let served = app.clone();
        tokio::spawn(async move {
            let _ = axum::serve(listener, served).await;
        });

        (pipe, app, conversation, profile)
    });

    Grilling {
        _work: work,
        state,
        home,
        conversation,
        profile,
        pipe,
        database,
        app,
        runtime,
    }
}

/// The local account this machine's Verkstead runs its sessions as, name and
/// password both.
///
/// **A suite cannot make one and says so rather than passing** — the rule
/// `crates/server/tests/account_windows.rs` follows, and for its reason:
/// creating a local account is an administrator's call, so a machine where the
/// elevated verb has never been run fails here with the line that names it.
///
/// **Which is why it is the *machine's* Data Directory rather than this
/// fixture's.** An account's name is a fingerprint of the Data Directory it
/// belongs to (see the server's `sandbox::account`), and every fixture here runs
/// against a temporary one — so the account it would be named after is one no
/// elevated verb was ever run for. What a fixture runs as instead is the account
/// the human really has, said outright on the `Homes` and with its password
/// written into the fixture's own secrets.
fn the_machines_account() -> Logon {
    let data_dir =
        platform::data_dir(None).expect("this machine has somewhere for a Data Directory");
    let settings = Settings::in_data_dir(&data_dir);

    match Account::on_this_machine(&data_dir, &settings.secrets()) {
        Ok(account) => Logon::of(account.name(), account.password()),
        Err(missing) => panic!(
            "this suite runs sessions as the session account and there is not one: {missing}\n\
             \n\
             The Data Directory it asked about is {}.",
            data_dir.display(),
        ),
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

/// A Windows session is told the pipe, scoped to the Conversation it is asking
/// from — and what it is told is still a *base*, so the paths the CLI composes
/// onto it compose the way they do onto a URL.
#[test]
fn a_windows_session_is_told_the_pipe_its_server_opened() {
    let fixture = grilling();
    let given = fixture.given("VERKSTEAD_SERVER");

    assert_eq!(
        given,
        format!("{}/conversations/{}", fixture.pipe, fixture.conversation.id),
        "a session on this platform asks through the pipe, from its own Conversation"
    );
    assert!(
        given.starts_with("pipe://"),
        "in the spelling a paste survives rather than Win32's own, got {given}"
    );
    assert!(
        !given.contains(&NOWHERE.to_string()),
        "and nothing of the socket, which nothing here is listening on, got {given}"
    );
}

/// The whole round trip, made by the binary a session is equipped with, from
/// behind the boundary a session runs in and over the transport it is told
/// about: the Set goes through the pipe onto this Conversation's Timeline, the
/// human answers it, and the Response comes back on the session's stdout.
#[test]
fn a_session_asks_through_the_pipe_and_the_response_comes_back() {
    let fixture = grilling();
    let (rendering, _closing) = fixture
        .sandbox()
        .command(&["verkstead", "ask"])
        .expect("a session's sandbox to be one this machine can make");

    assert!(
        rendering.account().is_some(),
        "a Windows session runs as a local account of Verkstead's own, and this one names none"
    );

    // On a thread, because the ask does not end until the human has answered it
    // and the human is this thread — see [`off_a_console`], which waits.
    let asking = std::thread::spawn(move || off_a_console(&rendering, SET.as_bytes()));

    let asked = fixture.await_asked_set(1);

    assert_eq!(
        asked.title, "Which transport did this session ask through?",
        "the Set the session sent over the pipe is the Set the server took"
    );

    fixture.answer(1, ANSWERED);

    let output = finished(asking);
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
        response.answers.len(),
        1,
        "and it is the human's own answer that came back, got {response:?}"
    );
}

/// And a pipe opened granting nobody cannot be opened from behind the boundary,
/// so the same ask goes nowhere.
///
/// Which is what makes the test above about the descriptor rather than about
/// this machine being permissive: the one thing separating a session that gets
/// in from one that does not is the entry written for the account it runs as.
#[test]
fn a_pipe_that_does_not_grant_the_session_account_cannot_be_opened_by_one() {
    let fixture = a_pipe_granting_nobody();

    let (rendering, _closing) = fixture
        .sandbox()
        .command(&["verkstead", "ask"])
        .expect("a session's sandbox to be one this machine can make");

    let asking = std::thread::spawn(move || off_a_console(&rendering, SET.as_bytes()));
    let output = finished(asking);

    assert!(
        !output.status.success(),
        "the ask should have failed for want of a pipe it may open, and it exited {:?} \
         having printed:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout)
    );

    assert!(
        fixture.nothing_was_asked(),
        "and nothing of it should have reached the server"
    );
}

/// The loopback, from behind the boundary a session runs in: reached, against a
/// socket that is genuinely listening and that answers this process at once.
///
/// **Which is what the account changed, and what the Compile Server needs.** An
/// AppContainer was refused the local machine — a connection from inside one
/// timed out rather than being refused outright — and an sccache is a client and
/// a server talking over the loopback, so it was off on this platform for want
/// of that. An ordinary local account reaches it, and this is where that is
/// attempted rather than assumed (ADR-0014, *Amended: the Sandbox is an
/// account*).
///
/// **The deadline is still the probe's own**, and for the reason it was: a
/// connect that is being dropped rather than refused comes back only when
/// Windows has finished retrying it, so a test with no deadline of its own would
/// hang instead of failing.
#[test]
fn the_loopback_answers_a_session() {
    let fixture = grilling();
    let (rendering, _closing) = fixture
        .sandbox()
        .command(&["verkstead", "guide"])
        .expect("a session's sandbox to be one this machine can make");

    let logon = rendering
        .account()
        .expect("a Windows session runs as a local account of Verkstead's own")
        .clone();

    // A socket nothing serves anything on: what is being asked is whether a
    // connection can be made at all, and a listener with a backlog answers that
    // without anything having to accept.
    let listening = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("a port of this machine's");
    let address = listening.local_addr().unwrap();

    std::net::TcpStream::connect_timeout(&address, OUT_HERE)
        .expect("this process, outside every container, to reach a socket on its own machine");

    let mut probe = Rendering::running("powershell.exe");
    probe
        .arg("-NoProfile")
        .arg("-NonInteractive")
        .arg("-Command")
        .arg(dialling(address))
        .as_account(logon);

    // The session's own environment rather than a list of names taken off this
    // process, which is a thing the session account is refused: a `USERPROFILE`
    // read out here is the human's own profile, and the whole point of the
    // boundary the probe is standing behind is that it cannot reach one. What a
    // session gets instead points at the profile the description grants it, so
    // this is both what a session really has and a set of paths the account can
    // open.
    for (name, value) in rendering.env() {
        probe.set(name, value);
    }

    // And where it starts, which a rendering of this test's own has to say: a
    // process started with nowhere to be is one the machine refuses before it
    // has run, and the Worktree is the one directory this description grants
    // that the account can be in.
    if let Some(chdir) = rendering.chdir() {
        probe.starting_in(chdir);
    }

    let output = off_a_console(&probe, b"").expect("a probe started as the session account");
    let said = String::from_utf8_lossy(&output.stdout).into_owned();

    assert!(
        said.contains(REACHED),
        "the session account should reach {address}, and the probe said: {said:?} \
         having complained: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// What the probe above runs: a connection to `address`, waited out, and one
/// word about how it went.
///
/// `ConnectAsync` rather than the blocking connect, because the wait is the
/// whole point: a connect that is being dropped rather than refused comes back
/// only when Windows has finished retrying it, which is far longer than a test
/// should sit on.
///
/// **Not one cmdlet in it**, which is the whole of why this reads the way it
/// does — see this file's own documentation for what the machine said when it
/// had two.
fn dialling(address: SocketAddr) -> String {
    format!(
        "$client = [System.Net.Sockets.TcpClient]::new(); \
         try {{ $reached = $client.ConnectAsync('{}', {}).Wait({}) }} \
         catch {{ $reached = $false }}; \
         if ($reached) {{ [Console]::Out.WriteLine('{REACHED}') }} \
         else {{ [Console]::Out.WriteLine('{REFUSED}') }}",
        address.ip(),
        address.port(),
        INSIDE.as_millis(),
    )
}

/// What a thread running a rendering off a console came back with.
fn finished(asking: std::thread::JoinHandle<std::io::Result<Output>>) -> Output {
    asking
        .join()
        .expect("the thread running the session to have finished rather than panicked")
        .expect("a rendering this machine can start")
}
