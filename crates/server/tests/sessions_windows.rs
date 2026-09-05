//! A grilling session end to end on Windows: started by pressing the button,
//! run on a pseudoconsole in a profile of the Conversation's own, and read back
//! off the Timeline while it is still going.
//!
//! The Unix suite's questions, asked of the other machine — and asked in a file
//! of its own rather than by making that one portable. `tests/sessions.rs`
//! stands on a mount namespace, a `/bin/sh` probe and a `stty`, and what is
//! being proved here is a different machine: a console rather than a
//! pseudo-terminal, a rendering with no boundary in it, a profile joined
//! together out of junctions and hard links, and a prompt that is not on the
//! command line at all.
//!
//! **Everything here is real except the agent.** The repository is a
//! repository, the worktree is one git made, the console is Verkstead's own
//! pseudoconsole and the profile is really made and really joined in. What
//! stands in for claude is a PowerShell script, for the reason the other suite
//! stands a shell script there: what these ask is whether a session's output
//! reaches the human, and asking it of the real claude would be a test that
//! needed an account, a network and a model's patience.
//!
//! The stand-in is handed exactly what the backend it stands where would be —
//! the model flag, the Profile's model, and then what Verkstead starts a
//! session on, with the session name and the bypass flag after it. So `$args[1]`
//! is the model it was told to run and `$args[2]` is the message it was started
//! on, exactly as `$1` and `$2` are on the other arm.
//!
//! **And on this platform that message is not the Brief.** The prompt is
//! written into the Conversation's handoff directory and what goes on the line
//! is one sentence naming the file, in backticks — so the stand-in reads the
//! name out from between them and opens it, which is what a real agent told to
//! read a file does. See [`PREAMBLE`], which is the whole of that.
//!
//! **What each test asserts, it asserts off a file wherever it can.** A console
//! is a grid, and what comes off a pseudoconsole is a drawing of one — so a
//! long path printed on it is a path with the console's own line break in the
//! middle. What is read off the Capture here is therefore short lines the
//! stand-in printed to say where it got to, and everything with a path in it is
//! written to a directory of the test's own and read back from there.
#![cfg(windows)]

use std::collections::BTreeSet;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, LazyLock};
use std::time::{Duration, Instant};

use avt::Vt;
use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use futures_util::{SinkExt, StreamExt};
use http_body_util::BodyExt;
use serde::de::DeserializeOwned;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use tower::ServiceExt;
use verkstead_render::{
    AgentOutputEvent, BriefSaved, Capture, ConversationStopped, ConversationView, GrillingStarted,
    ProfileSaved, Registered, Shown, Size, Started, TerminalOpened, TerminalsView, TimelineEvent,
    Watching,
};
use verkstead_server::build_cache::BuildCache;
use verkstead_server::handoffs::Handoffs;
use verkstead_server::platform::Platform;
use verkstead_server::sandbox::{Executable, Homes, Reachable, SandboxConfig};
use verkstead_server::settings::Settings;
use verkstead_server::skills::Skills;
use verkstead_server::terminal::COLUMNS;
use verkstead_server::{Agents, Gh, Pace, WatchedPaths, open_database, router_running_sessions};

/// The Brief every Conversation here is started from, and what the stand-in is
/// primed with.
const BRIEF: &str = "# Rate limiting\n\nThe API has none.\n";

/// Where the server these sessions belong to would be listening.
const LISTENING: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8422);

/// Who a session commits as, which every sandbox is configured out of.
const THE_AUTHOR: &str = "git_author:\n  name: Verkstead Test\n  email: test@verkstead.invalid\n";

/// How long to wait for something a session says. Generously long: what is
/// being waited on is a process starting, and a first `powershell.exe` on a
/// cold two-core runner is not quick.
const PATIENCE: Duration = Duration::from_secs(60);

/// What the stand-in is started as: Windows PowerShell, which every machine
/// carries, running a script file.
///
/// **`-File` rather than `-Command`**, which is what makes this a stand-in at
/// all: with `-File` everything after the script is handed to it as arguments,
/// and with `-Command` it would be appended to the script and *run*. So what
/// Verkstead puts on the line arrives as `$args`, which is where a stand-in
/// reads it.
///
/// `-NoProfile` because a machine whose account has a PowerShell profile would
/// otherwise run it first, and nothing here is about that; `-ExecutionPolicy
/// Bypass` because the script is one the test wrote a moment ago, which is not
/// a thing a machine's signing policy has anything useful to say about. Nothing
/// else: a `-NonInteractive` would be a claim about the console this is started
/// on, and one of these tests types into it.
const POWERSHELL: [&str; 4] = ["powershell.exe", "-NoProfile", "-ExecutionPolicy", "Bypass"];

/// What every stand-in in this file begins with: three things a test's own
/// script says with, the whole argument vector written down, and the line the
/// session was started on taken apart.
///
/// `Note` writes a value where the test will read it, by writing beside the
/// name and renaming onto it — a rename is one step, so a test polling for the
/// file never reads half of one. `Say` prints a line on the console, which is
/// what reaches the Capture. `Idle` is a session that has said what it has to
/// say and is not going to exit, which is what the tests about a *running*
/// session need.
///
/// The vector is written down by every stand-in rather than by the one test
/// that reads it: what `$args[1]` and `$args[2]` are turns on Verkstead having
/// built the line in the order it builds it, so a test failing anywhere in this
/// file is a test whose evidence directory says what the line really was.
const PREAMBLE: &str = r#"
$ErrorActionPreference = 'Stop'
$evidence = '{evidence}'

function Note($name, $value) {
    $to = Join-Path $evidence $name
    Set-Content -LiteralPath ($to + '.writing') -Value ([string]$value) -NoNewline
    Move-Item -LiteralPath ($to + '.writing') -Destination $to -Force
}

function Say($said) { [Console]::Out.WriteLine($said) }

function Idle { while ($true) { Start-Sleep -Milliseconds 50 } }

Note 'args' ($args -join "`n")

$model = $args[1]
$line = $args[2]
$named = [regex]::Match($line, '`([^`]+)`').Groups[1].Value
$prompt = if ($named) { Get-Content -Raw -LiteralPath $named } else { '' }
"#;

/// The server's own log, printed under whichever test was reading when it gave
/// up.
///
/// **A session that never starts says so nowhere else.** Everything that
/// refuses one answers its caller with a bare `None` or a `Refused` and writes
/// the reason to `tracing` — a sandbox that could not be built, a console that
/// could not be opened, a program the description's `PATH` had nothing at — so
/// a suite with no subscriber is a suite where every one of those failures
/// reads *the session printed nothing*, which is the one thing about them that
/// is the same.
///
/// `with_test_writer` so that libtest keeps each line with the test that was
/// running on that thread and prints it only for the ones that failed; work the
/// server does on a blocking thread lands on the run's own output instead,
/// which is the same account a little further from the failure. And the filter
/// narrows it to this crate: at `debug` the whole of what a session's start
/// decided, with everything else left at `warn` so that sqlx's every statement
/// is not what a reader has to scroll through.
static LOGGING: LazyLock<()> = LazyLock::new(|| {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new(SAID))
        .with_test_writer()
        .with_target(false)
        .try_init();
});

/// What of it is worth printing — see [`LOGGING`].
const SAID: &str = "warn,verkstead_server=debug";

/// How many of these fixtures may be standing at once.
///
/// Each one is a real server, a real repository on disk and a real session on a
/// real pseudoconsole, and `cargo test` will happily start every test in this
/// file at once — which on a two-core runner is a dozen benches competing for
/// two cores. The other suite's ceiling, for the other suite's reason: what
/// broke there was having none at all.
static ROOM: LazyLock<Arc<tokio::sync::Semaphore>> = LazyLock::new(|| {
    let cores = std::thread::available_parallelism().map_or(1, |cores| cores.get());

    Arc::new(tokio::sync::Semaphore::new((cores * 2).clamp(4, 16)))
});

/// The pace these sessions are driven at.
///
/// **Every clock that ends a session is longer than any test in this file
/// runs**, and that is the whole shape of it. Nothing here is about *when* a
/// session is ended — the Unix suite is where the grace, the stir and the long
/// stop are asked about, and it keeps a pace with all three inside a test's
/// life to ask them. What these want is the opposite: a session that says its
/// piece and then sits there is a session the tests can go on reading, rather
/// than one prodded and stopped out from under an assertion because a two-core
/// runner took a moment over the poll before it.
///
/// So the two that are short are the two that drive rather than end: the poll
/// the runner works at, and how often a wrap-up would ask about its checks. The
/// sessions that are meant to end here end by exiting, which no clock has
/// anything to say about.
static UNHURRIED: LazyLock<Pace> = LazyLock::new(|| Pace {
    poll: Duration::from_millis(100),
    checks: Duration::from_millis(100),
    reviewing: Duration::ZERO,
    grace: Duration::from_secs(600),
    proposing: Duration::from_secs(600),
    waking: Duration::from_secs(600),
    long_stop: Duration::from_secs(600),
    stalls: Duration::from_secs(600),
    merges: Duration::from_secs(600),
    cleanup: Duration::from_secs(600),
});

/// A Conversation with a session running under a stand-in agent, and everything
/// holding its directories open.
struct Grilling {
    /// Dropped last, and only these keep the directories alive: a worktree that
    /// vanished mid-session would fail obscurely.
    _watched: tempfile::TempDir,
    _scripts: tempfile::TempDir,
    state: tempfile::TempDir,

    /// Where the stand-in writes what this test reads back — see [`PREAMBLE`].
    evidence: tempfile::TempDir,

    app: Router,
    id: i64,

    /// Where the Agent Profile's account is on the host, which is the far end
    /// of what a session's own profile has joined in.
    account: PathBuf,

    /// And where a session reads the bundled skills, which is what the prompt
    /// names.
    skills_inside: PathBuf,

    /// This fixture's place in the suite — see [`ROOM`]. Last, so that it is
    /// handed back only once everything above has been let go of.
    _room: tokio::sync::OwnedSemaphorePermit,
}

impl Grilling {
    /// The Conversation as the workbench reads it.
    async fn view(&self) -> ConversationView {
        get(&self.app, &format!("/api/ui/conversations/{}", self.id)).await
    }

    /// Read it back until the session has got somewhere, or give up.
    async fn until<T>(&self, reached: impl Fn(&ConversationView) -> Option<T>) -> T {
        let deadline = Instant::now() + PATIENCE;

        loop {
            let view = self.view().await;

            if let Some(reached) = reached(&view) {
                return reached;
            }

            if Instant::now() >= deadline {
                panic!(
                    "the session never got there. It printed: {:?}. A Timeline with no \
                     session on it at all is one that was refused, and the \
                     server's log above says which refusal that was",
                    self.said_by_each(&view).await,
                );
            }

            pause(Duration::from_millis(25)).await;
        }
    }

    /// What every session on this Timeline actually put on its console, for the
    /// assertion above — escaped, because half of what a failure here turns on
    /// is what the bytes were.
    async fn said_by_each(&self, view: &ConversationView) -> String {
        let mut said = String::new();

        for output in outputs(view) {
            said.push_str(&format!(
                "\n  #{} printed: {:?}",
                output.id,
                self.capture(output.id).await,
            ));
        }

        said
    }

    /// What a session has printed, whole, as the details pane fetches it.
    async fn capture(&self, event: i64) -> String {
        let capture: Capture = get(
            &self.app,
            &format!("/api/ui/conversations/{}/capture/{event}", self.id),
        )
        .await;

        capture.text
    }

    /// Wait until a session has printed something, or give up.
    async fn printed(&self, event: i64, said: &str) -> String {
        let deadline = Instant::now() + PATIENCE;

        loop {
            let capture = self.capture(event).await;

            if capture.contains(said) {
                return capture;
            }

            assert!(
                Instant::now() < deadline,
                "the session never printed {said:?}. It printed: {capture:?}",
            );

            pause(Duration::from_millis(25)).await;
        }
    }

    /// Wait until there is a session running, and hand back the Event it is
    /// printing into — which is what a Screen is watched by.
    async fn running(&self) -> i64 {
        self.until(|view| output(view).filter(|output| output.running).map(|o| o.id))
            .await
    }

    /// And wait until the one this Conversation ran has stopped, whatever it
    /// said on the way.
    async fn stopped(&self) -> i64 {
        self.until(|view| output(view).filter(|output| !output.running).map(|o| o.id))
            .await
    }

    /// What the stand-in wrote under that name, waited for — see [`PREAMBLE`].
    ///
    /// Written beside the name and renamed onto it, so a file that is there is
    /// a file that is whole — and what a failure says is what the session
    /// printed, because the ordinary reason a stand-in writes nothing is that
    /// PowerShell said why on the console and gave up.
    async fn written(&self, name: &str) -> String {
        let path = self.evidence.path().join(name);
        let deadline = Instant::now() + PATIENCE;

        loop {
            if let Ok(written) = std::fs::read_to_string(&path) {
                return written;
            }

            if Instant::now() >= deadline {
                let view = self.view().await;

                panic!(
                    "the session never wrote {name}. It printed: {}. A Timeline \
                     with no session on it at all is one that was refused, and \
                     the server's log above says which refusal that was",
                    self.said_by_each(&view).await,
                );
            }

            pause(Duration::from_millis(25)).await;
        }
    }

    /// The Worktree the grilling made, waited for.
    async fn worktree(&self) -> PathBuf {
        PathBuf::from(
            self.until(|view| view.worktree.as_ref().map(|worktree| worktree.path.clone()))
                .await,
        )
    }

    /// This Conversation's handoff directory on the host, which is where the
    /// prompt a session is started on is written.
    fn handoffs(&self) -> PathBuf {
        self.state.path().join("handoffs").join(self.id.to_string())
    }

    /// And the profile its sessions run in, which is made fresh as each one
    /// starts.
    fn profile(&self) -> PathBuf {
        self.state.path().join("homes").join(self.id.to_string())
    }

    /// Force stop, which is the press that ends a session where it stands.
    async fn force_stop(&self) -> ConversationStopped {
        post(
            &self.app,
            &format!("/api/ui/conversations/{}/force-stop", self.id),
            &serde_json::json!({}),
        )
        .await
    }

    /// The same workbench, on a socket of its own, and where to find it.
    ///
    /// Everything else here asks the Router directly. The Screen's socket
    /// cannot be asked that way: an upgrade is a connection rather than a
    /// request, so this is the one thing that needs a server actually listening.
    async fn listening(&self) -> SocketAddr {
        let listener = tokio::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .await
            .expect("a port to listen on");

        let at = listener.local_addr().unwrap();
        let app = self.app.clone();

        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });

        at
    }

    /// Open one of the Conversation's own terminals, the way the pane does.
    async fn terminal(&self) -> i64 {
        let opened: TerminalOpened = post(
            &self.app,
            &format!("/api/ui/conversations/{}/terminals", self.id),
            &serde_json::json!({}),
        )
        .await;

        let TerminalOpened::Opened { number } = opened else {
            panic!(
                "expected a terminal to open, and the server said: {opened:?} \
                 — the log above says which refusal that was"
            );
        };

        number
    }
}

/// Stand a workbench up with `script` where claude goes, and press *start
/// grilling*.
///
/// The script is PowerShell, appended to [`PREAMBLE`] — so it has `$model`,
/// `$line`, `$named` and `$prompt` already read, and `Note`, `Say` and `Idle`
/// to say things with.
async fn grilling(script: &str) -> Grilling {
    grilling_caching(script, None).await
}

/// The same, with a shared build cache behind it — which is what the tests
/// about the sccache need and what nothing else here wants: a cache is a
/// `RUSTC_WRAPPER` in every session's environment, and every other test in this
/// file is about a session that builds nothing.
async fn grilling_caching(script: &str, cache: Option<&Path>) -> Grilling {
    // Before the server exists, because what is worth reading is what it says
    // as it starts a session — see [`LOGGING`].
    LazyLock::force(&LOGGING);

    // Before anything is built, so that a bench queued behind the suite's
    // ceiling costs nothing while it waits — see [`ROOM`].
    let room = ROOM
        .clone()
        .acquire_owned()
        .await
        .expect("the suite's room is never closed");

    let watched = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    let scripts = tempfile::tempdir().unwrap();
    let evidence = tempfile::tempdir().unwrap();

    std::fs::write(state.path().join("config.yaml"), THE_AUTHOR).unwrap();

    let database = state.path().join("verkstead.db");
    let pool = open_database(&database).await.unwrap();

    let stand_in = scripts.path().join("agent.ps1");
    std::fs::write(
        &stand_in,
        format!(
            "{}\n{script}\n",
            PREAMBLE.replace(
                "{evidence}",
                &evidence.path().display().to_string().replace('\'', "''"),
            ),
        ),
    )
    .unwrap();

    let agent = POWERSHELL
        .iter()
        .map(|word| (*word).to_owned())
        .chain(["-File".to_owned(), stand_in.display().to_string()])
        .collect();

    let skills =
        Skills::installed(Platform::HERE, state.path()).expect("this binary carries skills");
    let skills_inside = skills.inside().to_owned();

    let build_cache = match cache {
        Some(dir) => BuildCache::resolve(Some(dir), state.path()).expect("a cache to resolve"),
        None => BuildCache::none(),
    };

    let agents = Agents::running(
        agent,
        // The server's own home, which this platform never hands a session:
        // every Conversation here gets one of its own under the Data
        // Directory. See `Homes::for_conversation`.
        Homes::on(Platform::HERE, state.path().join("nobody"), state.path()),
        Reachable::at(LISTENING),
        // No configured binds: a Windows path is not one `--bind` takes, and
        // there is nothing for one to do on a platform whose rendering binds
        // nothing.
        SandboxConfig::default(),
        build_cache,
        skills,
        Executable::of_the_server(state.path()),
        Handoffs::under(state.path()),
        Settings::in_data_dir(state.path()),
    )
    .at_pace(*UNHURRIED);

    let app = router_running_sessions(
        pool,
        WatchedPaths::resolve(&[watched.path().to_owned()]).unwrap(),
        state.path().to_owned(),
        agents,
        // Nothing here reaches a wrap-up, so nothing here asks GitHub anything
        // — and what stands where `gh` goes is a program that answers nobody.
        Gh::running(vec![
            "cmd.exe".to_owned(),
            "/c".to_owned(),
            "exit 1".to_owned(),
        ]),
    );

    let repo = repository(watched.path().join("verkstead"));
    let registered: Registered =
        post(&app, "/api/ui/repos", &serde_json::json!({ "path": repo })).await;
    assert_eq!(registered, Registered::Added);

    let repos: Vec<verkstead_render::RepoEntry> = get(&app, "/api/ui/repos").await;
    let repo_id = repos[0].id;

    let started: Started = post(
        &app,
        "/api/ui/conversations",
        &serde_json::json!({ "repo_id": repo_id }),
    )
    .await;
    let Started::Started { id } = started else {
        panic!("expected the Conversation to start, got {started:?}");
    };

    let account = account(watched.path());

    for role in ["grilling", "implementation", "review"] {
        let profile = profile(&app, &account, role).await;
        let pairing = serde_json::json!({
            "profile_id": profile,
            "model": format!("claude-{role}-5"),
        });

        // Two of the pickers offer a row that is no account at all, so what they
        // send is which of their rows was picked.
        let picked = match role {
            "grilling" | "review" => serde_json::json!({ "pairing": pairing }),
            _ => pairing,
        };

        let chosen: verkstead_render::ProfileChosen = post(
            &app,
            &format!("/api/ui/conversations/{id}/{role}-pairing"),
            &picked,
        )
        .await;
        assert_eq!(chosen, verkstead_render::ProfileChosen::Chosen);
    }

    let saved: BriefSaved = post(
        &app,
        &format!("/api/ui/conversations/{id}/brief"),
        &serde_json::json!({ "markdown": BRIEF }),
    )
    .await;
    assert_eq!(saved, BriefSaved::Saved);

    let grilling: GrillingStarted = post(
        &app,
        &format!("/api/ui/conversations/{id}/grill"),
        &serde_json::json!({}),
    )
    .await;
    assert_eq!(grilling, GrillingStarted::Started);

    Grilling {
        _watched: watched,
        _scripts: scripts,
        state,
        evidence,
        app,
        id,
        account,
        skills_inside,
        _room: room,
    }
}

/// The Agent Profile's account on the host: Claude's pair, with a file of its
/// own in the directory half so that a session can be asked whether the account
/// it is running under is the one the Profile named.
///
/// One account for all three roles rather than one apiece. What a session's
/// profile joins in is the account of the Profile it was launched under, and
/// every session in this file is the grilling one — so a second and a third
/// would be directories nothing ever looked at.
fn account(watched: &Path) -> PathBuf {
    let account = watched.join("claude");

    std::fs::create_dir_all(account.join(".claude")).unwrap();
    std::fs::write(account.join(".claude").join("marker.txt"), THE_ACCOUNTS).unwrap();
    std::fs::write(account.join(".claude.json"), "{}\n").unwrap();

    account
}

/// What is in that file, which is a thing only the Profile's own account holds.
const THE_ACCOUNTS: &str = "the account the Profile named";

/// An Agent Profile saved over that account, on models that are worth reading
/// back.
async fn profile(app: &Router, account: &Path, name: &str) -> i64 {
    let saved: ProfileSaved = post(
        app,
        "/api/ui/profiles",
        &serde_json::json!({
            "name": name,
            "account": {
                "agent_type": "Claude",
                "claude_dir": account.join(".claude"),
                "config_file": account.join(".claude.json"),
            },
            "models": [format!("claude-{name}-5"), format!("claude-{name}-4.8")],
        }),
    )
    .await;
    assert_eq!(saved, ProfileSaved::Saved);

    let profiles: Vec<verkstead_render::ProfileEntry> = get(app, "/api/ui/profiles").await;
    profiles
        .into_iter()
        .find(|profile| profile.name == name)
        .expect("the Profile just saved should be on the list")
        .id
}

/// A git repository at `path`, with one commit on `main`.
fn repository(path: PathBuf) -> PathBuf {
    std::fs::create_dir_all(&path).unwrap();
    git(&path, &["init", "--initial-branch", "main"]);
    git(&path, &["config", "user.email", "test@verkstead.invalid"]);
    git(&path, &["config", "user.name", "Verkstead Test"]);
    std::fs::write(path.join("README.md"), "# a repository\n").unwrap();
    git(&path, &["add", "README.md"]);
    git(&path, &["commit", "-m", "first"]);

    path
}

fn git(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .expect("git should be on the PATH for these tests");

    assert!(
        output.status.success(),
        "git {args:?} failed in {}",
        dir.display()
    );

    String::from_utf8(output.stdout).unwrap()
}

/// The one agent-output Event on a Timeline, where there is one yet.
fn output(view: &ConversationView) -> Option<&AgentOutputEvent> {
    outputs(view).into_iter().next()
}

/// And every one of them, oldest first.
fn outputs(view: &ConversationView) -> Vec<&AgentOutputEvent> {
    view.timeline
        .iter()
        .filter_map(|event| match event {
            TimelineEvent::AgentOutput(output) => Some(output),
            _ => None,
        })
        .collect()
}

/// How much a file has had appended to it by now, for the comparison that says
/// something has stopped writing.
fn appended(path: &Path) -> u64 {
    std::fs::metadata(path).map(|it| it.len()).unwrap_or(0)
}

/// Wait until something is at `path`, or give up saying nothing ever was.
///
/// What a shell in a Terminal is asked with: a file it wrote turning up where
/// the test can see it is the whole of *it was standing there*.
async fn until_there(path: &Path) {
    let deadline = Instant::now() + PATIENCE;

    while !path.exists() {
        assert!(
            Instant::now() < deadline,
            "nothing ever turned up at {}",
            path.display(),
        );

        pause(Duration::from_millis(25)).await;
    }
}

/// Wait until it has been written to at least once, so that what is asserted
/// after a session is ended is something that was really happening.
async fn until_appended(path: &Path) -> u64 {
    let deadline = Instant::now() + PATIENCE;

    loop {
        let written = appended(path);

        if written > 0 {
            return written;
        }

        assert!(
            Instant::now() < deadline,
            "the session never got as far as writing to {}, so nothing here \
             would prove anything",
            path.display(),
        );

        pause(Duration::from_millis(25)).await;
    }
}

async fn pause(span: Duration) {
    tokio::time::sleep(span).await;
}

async fn get<T: DeserializeOwned>(app: &Router, path: &str) -> T {
    let (status, body) = fetch(
        app,
        Request::builder().uri(path).body(Body::empty()).unwrap(),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "GET {path} failed: {body}");
    read(&body)
}

async fn post<T: DeserializeOwned>(app: &Router, path: &str, body: &serde_json::Value) -> T {
    let (status, body) = fetch(
        app,
        Request::builder()
            .method("POST")
            .uri(path)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(serde_json::to_vec(body).unwrap()))
            .unwrap(),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "POST {path} failed: {body}");
    read(&body)
}

async fn fetch(app: &Router, request: Request<Body>) -> (StatusCode, String) {
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8(bytes.to_vec()).unwrap())
}

fn read<T: DeserializeOwned>(body: &str) -> T {
    serde_json::from_str(body).unwrap_or_else(|err| panic!("reading {body:?}: {err}"))
}

/// Two paths compared as the filesystem has them rather than as they are
/// spelled.
///
/// Which matters on this platform twice over: a temporary directory is reached
/// through a short name (`RUNNER~1`) and a session's handoff directory is
/// reached through a junction, so two names for one file are the ordinary case
/// rather than the corner.
fn the_same_file(one: &Path, another: &Path) {
    assert_eq!(
        std::fs::canonicalize(one)
            .unwrap_or_else(|error| panic!("resolving {}: {error}", one.display())),
        std::fs::canonicalize(another)
            .unwrap_or_else(|error| panic!("resolving {}: {error}", another.display())),
        "{} and {} should be the one file",
        one.display(),
        another.display(),
    );
}

/// A file a session writes somewhere other than its evidence directory, waited
/// for the way [`Grilling::written`] waits for one there.
///
/// Waited for rather than looked at, for the reason everything else in this
/// file is: a stand-in that has written the line before it is a stand-in the
/// test has caught up with, not one that has finished. The evidence directory
/// is polled a name at a time, so the last name read says nothing about the
/// line after it — and on a runner with two cores and a dozen sessions on them,
/// the gap between one line and the next is as long as the scheduler makes it.
async fn landed(path: &Path, why: &str) {
    let deadline = Instant::now() + PATIENCE;

    loop {
        if path.is_file() {
            return;
        }

        assert!(
            Instant::now() < deadline,
            "{why} — {} is not there",
            path.display()
        );

        pause(Duration::from_millis(25)).await;
    }
}

/// The whole of what pressing the button does here: the Profile's agent, on the
/// Profile's model, running in the Conversation's worktree, sent into the
/// bundled grilling skill and primed with the Brief.
#[tokio::test]
async fn a_session_runs_the_grilling_profiles_agent_on_the_brief_in_the_worktree() {
    let fixture = grilling(
        r#"
        Note 'model' $model
        Note 'where' (Get-Location).Path
        Note 'prompt' $prompt
        Say 'read the brief'
        "#,
    )
    .await;

    // The whole line, which is the other half of *the Profile's agent ran*: the
    // model flag first, the model after it, and Verkstead's own bypass at the
    // end, so that a session runs unattended rather than stopping to ask
    // approval of nobody.
    let argv: Vec<String> = fixture
        .written("args")
        .await
        .lines()
        .map(str::to_owned)
        .collect();

    assert_eq!(
        argv.first().map(String::as_str),
        Some("--model"),
        "the model is the first thing after the program: {argv:?}"
    );
    assert_eq!(
        argv.get(1).map(String::as_str),
        Some("claude-grilling-5"),
        "the grilling Profile's model is what the session runs on, not the \
         implementation one's: {argv:?}"
    );
    assert!(
        argv.iter()
            .any(|word| word == "--dangerously-skip-permissions"),
        "and the flag that stops it asking approval of nobody is on the line: \
         {argv:?}"
    );

    assert_eq!(
        fixture.written("model").await,
        "claude-grilling-5",
        "which is what a stand-in reads off `$args[1]`, and what every other \
         test in this file is standing on",
    );

    let prompt = fixture.written("prompt").await;

    assert!(
        prompt.contains(BRIEF),
        "the Brief is what the grilling starts from: {prompt:?}"
    );
    assert!(
        prompt.contains(&format!(
            "{}/grilling/SKILL.md",
            fixture.skills_inside.display()
        )),
        "and the session is sent into the bundled grilling skill, which on this \
         platform is where it was really written: {prompt:?}"
    );

    the_same_file(
        Path::new(&fixture.written("where").await),
        &fixture.worktree().await,
    );

    // And the Event it printed into says what it ran under, stamped as the
    // Capture was opened.
    let stamped = output(&fixture.view().await)
        .expect("the session printed into an Event")
        .clone();

    assert_eq!(
        (stamped.profile.as_deref(), stamped.model.as_deref()),
        (Some("grilling"), Some("claude-grilling-5")),
        "the grilling Profile and the model it was launched on"
    );
}

/// And what it is started *on*, which on this platform is a file rather than an
/// argument.
///
/// Windows caps a command line at 32,767 characters and an implementing
/// session's prompt carries a whole handoff document, so the prompt is written
/// into the Conversation's handoff directory and one line naming the file goes
/// where it was. Three things have to be true together: the line names the
/// file and does not carry the Brief, the file the session opened is the one in
/// the Conversation's handoff directory, and what is in it is the prompt.
#[tokio::test]
async fn the_prompt_a_session_is_started_on_is_a_file_it_reads_the_brief_out_of() {
    let fixture = grilling(
        r#"
        Note 'line' $line
        Note 'named' $named
        Note 'prompt' $prompt
        "#,
    )
    .await;

    let line = fixture.written("line").await;

    assert!(
        !line.contains(BRIEF),
        "the Brief is not on the command line at all here: {line:?}"
    );
    assert_eq!(
        line.lines().count(),
        1,
        "and what is there instead is one line, because it is one argument: {line:?}"
    );

    // The path the session read it at, which is the handoff directory as it is
    // reached from inside the profile — the far side of the junction.
    let named = PathBuf::from(fixture.written("named").await);

    assert!(
        named.starts_with(fixture.profile()),
        "the file is named by the path the session opens it at, which is inside \
         the profile it was given: {}",
        named.display(),
    );

    let written = fixture.handoffs().join("prompt.md");

    the_same_file(&named, &written);

    assert!(
        std::fs::read_to_string(&written).unwrap().contains(BRIEF),
        "and the Conversation's handoff directory is where it really is",
    );
    assert!(
        fixture.written("prompt").await.contains(BRIEF),
        "and the session read the Brief out of it, which is what the line told \
         it to do",
    );
}

/// What a session prints reaches the human while it is still printing.
///
/// The Capture is the record of what an agent did, and this is the whole of the
/// path from a console write to the details pane: the relay reads the
/// pseudoconsole, the store keeps what it read, and the pane fetches it.
#[tokio::test]
async fn what_a_session_prints_reaches_the_capture_while_it_is_still_running() {
    let fixture = grilling(
        r#"
        Say 'reading the brief'
        Idle
        "#,
    )
    .await;

    let event = fixture.running().await;
    let said = fixture.printed(event, "reading the brief").await;

    assert!(
        said.contains("reading the brief"),
        "what the session printed should be on the Capture: {said:?}"
    );
    assert!(
        output(&fixture.view().await).is_some_and(|output| output.running),
        "and it should still be running, this being a Capture read while the \
         session goes on",
    );
}

/// The console a session is on, asked from inside it.
///
/// Verkstead opens a pseudoconsole and starts the session on it, so what the
/// session sees is a console of Verkstead's own at the size Verkstead opened
/// one at — rather than whatever console the tests were started from, or none
/// at all. The width is what is asked about for the reason
/// `tests/terminal_windows.rs` gives: how tall a pseudoconsole's buffer is, is
/// the console host's own business.
#[tokio::test]
async fn a_session_runs_on_a_console_verkstead_opened_for_it() {
    let fixture = grilling(
        r#"
        Say ('width=' + [Console]::WindowWidth)
        Idle
        "#,
    )
    .await;

    let event = fixture.running().await;
    let width = format!("width={COLUMNS}");
    let said = fixture.printed(event, &width).await;

    assert!(
        said.contains(&width),
        "a session should be on a console of its own, {COLUMNS} columns across, \
         and it said: {said:?}"
    );
}

/// And a watcher's window is that console's size, so the session's interface
/// redraws to fit.
///
/// Read back off the session rather than off the Screen, because that is the
/// claim: a grid made wider on the server and nowhere else would be a Screen
/// the session never heard about. The probe asks again and again rather than
/// once, there being no signal on this platform to wait for.
#[tokio::test]
async fn resizing_a_watchers_window_resizes_the_session() {
    let fixture = grilling(
        r#"
        while ($true) {
            Say ('width=' + [Console]::WindowWidth)
            Start-Sleep -Milliseconds 300
        }
        "#,
    )
    .await;

    let event = fixture.running().await;
    let at = fixture.listening().await;

    let mut watcher = Watcher::attach(at, fixture.id, event).await;

    // The size it started on, which is the console Verkstead opened for it.
    watcher
        .until(|grid| grid.contains(&format!("width={COLUMNS}")))
        .await;

    watcher.resize(132, 43).await;

    let showing = watcher.until(|grid| grid.contains("width=132")).await;

    assert!(
        showing.iter().any(|row| row.contains("width=132")),
        "the session should have been told its window is now 132 across, and \
         the Screen is showing: {showing:?}",
    );
}

/// And what a watcher types reaches the session's own console input.
///
/// The Hold in one keystroke: a browser sends what a terminal would have made
/// of the keys, and what is on the far end of the socket is the console the
/// session is reading its own standard input from.
#[tokio::test]
async fn what_a_watcher_types_reaches_the_session() {
    let fixture = grilling(
        r#"
        Say 'ready'
        $typed = [Console]::In.ReadLine()
        Say ('typed=' + $typed)
        Idle
        "#,
    )
    .await;

    let event = fixture.running().await;
    let at = fixture.listening().await;

    let mut watcher = Watcher::attach(at, fixture.id, event).await;

    watcher.until(|grid| grid.contains("ready")).await;

    // A carriage return, which is what a press of Enter is on a terminal.
    watcher.types("hello\r").await;

    let showing = watcher.until(|grid| grid.contains("typed=hello")).await;

    assert!(
        showing.iter().any(|row| row.contains("typed=hello")),
        "the session should have read what was typed into it, and the Screen is \
         showing: {showing:?}",
    );
}

/// And Force stop ends it where it stands, which on this platform is the Job
/// Object's doing.
///
/// Asserted on a thing the session was doing and has stopped doing, rather than
/// on the record alone: what holds a Windows session's whole process tree is a
/// Job with kill-on-close, and a session that is off the Timeline but still
/// writing is exactly what that is there to prevent.
#[tokio::test]
async fn force_stop_ends_a_session_where_it_stands() {
    let fixture = grilling(
        r#"
        $ticks = Join-Path $evidence 'ticks'
        Say 'working'
        while ($true) {
            Add-Content -LiteralPath $ticks -Value 'tick' -NoNewline
            Start-Sleep -Milliseconds 50
        }
        "#,
    )
    .await;

    let event = fixture.running().await;
    fixture.printed(event, "working").await;

    let ticks = fixture.evidence.path().join("ticks");
    until_appended(&ticks).await;

    assert_eq!(fixture.force_stop().await, ConversationStopped::Stopped);

    fixture.stopped().await;

    // A window in which it would have written more, had anything of it been
    // left running.
    let when_it_stopped = appended(&ticks);
    pause(Duration::from_secs(1)).await;

    assert_eq!(
        appended(&ticks),
        when_it_stopped,
        "a session that has been stopped should have stopped writing",
    );
}

/// A session runs in a profile of the Conversation's own, with the Profile's
/// account joined into it.
///
/// The whole of what *fresh profile* means in code, asked from inside: the five
/// names a Windows program reads for the account's own directories all point
/// inside one directory under the Data Directory; the two halves they name are
/// really there; what a session throws away lands in it; and the account the
/// Profile named is really there, joined in by the junction and the hard link
/// the open rendering makes.
#[tokio::test]
async fn a_session_runs_in_a_profile_of_the_conversations_own() {
    let fixture = grilling(
        r#"
        Note 'userprofile' $env:USERPROFILE
        Note 'home' $env:HOME
        Note 'appdata' $env:APPDATA
        Note 'localappdata' $env:LOCALAPPDATA
        Note 'temp' $env:TEMP
        Note 'tmp' $env:TMP

        Set-Content -LiteralPath (Join-Path $env:TEMP 'thrown-away.txt') -Value 'gone with it'

        Note 'marker' (Get-Content -Raw -LiteralPath (Join-Path $env:USERPROFILE '.claude\marker.txt'))
        Note 'config' (Get-Content -Raw -LiteralPath (Join-Path $env:USERPROFILE '.claude.json'))

        Say 'read the account'
        Idle
        "#,
    )
    .await;

    let profile = fixture.profile();
    let roaming = profile.join("AppData").join("Roaming");
    let local = profile.join("AppData").join("Local");
    let temporary = local.join("Temp");

    // Compared as they are spelled rather than as the filesystem has them,
    // which is the stricter of the two here: every one of these is built out of
    // the Data Directory this fixture handed in, so a name that differs at all
    // is a name Verkstead composed differently.
    for (name, expected) in [
        ("userprofile", profile.clone()),
        ("home", profile.clone()),
        ("appdata", roaming.clone()),
        ("localappdata", local.clone()),
        ("temp", temporary.clone()),
        ("tmp", temporary.clone()),
    ] {
        assert_eq!(
            fixture.written(name).await,
            expected.display().to_string(),
            "a Windows session reads {name} for one of the directories its \
             account keeps things in, and every one of them is inside the \
             profile this Conversation was given",
        );
    }

    landed(
        &temporary.join("thrown-away.txt"),
        "what a session writes to its temporary directory lands under the \
         profile it was given, which is what makes it thrown away with it",
    )
    .await;

    // And both halves are really there, which is a claim about the profile
    // rather than about anything the session did in it. A Windows program does
    // not read `APPDATA` to find where its settings go — it asks the shell,
    // which resolves the halves against `USERPROFILE` and then will not answer
    // at all for one that is not really a directory. A half left to be made by
    // whoever writes there first is a half nothing can ask about.
    for half in [&roaming, &local] {
        assert!(
            half.is_dir(),
            "{} is a half of the profile a session was told it has, so it is \
             one the profile really has",
            half.display(),
        );
    }

    assert_eq!(
        fixture.written("marker").await,
        THE_ACCOUNTS,
        "and the account inside is the one the Profile named, joined in by the \
         junction the rendering makes",
    );
    assert_eq!(
        fixture.written("config").await,
        "{}\n",
        "and so is the file half of it, joined in by a hard link",
    );

    // Read from the host rather than from inside, which is the other half of
    // the same claim: the account's own directory is where the Profile said,
    // and the profile is somewhere else entirely.
    assert!(
        fixture.account.join(".claude").join("marker.txt").is_file(),
        "the account itself is untouched",
    );
}

/// And the Conversation says, in the one value three places on the workbench
/// read, that none of this is sandboxed.
///
/// The trade this whole stage is: a Windows session runs, and it runs with the
/// reach of the account running the server until the sandbox stage lands. What
/// is asserted here is the server's half — that the value the composer, the
/// session pane and the terminal pane all draw from is true on the platform it
/// is about, and that a session ran under it all the same.
#[tokio::test]
async fn a_windows_conversation_says_its_sessions_are_not_sandboxed() {
    let fixture = grilling(
        r#"
        Say 'reading the brief'
        Idle
        "#,
    )
    .await;

    let event = fixture.running().await;
    fixture.printed(event, "reading the brief").await;

    assert!(
        fixture.view().await.unsandboxed,
        "a Conversation on this platform is one whose sessions run unsandboxed, \
         and the view is what says so",
    );
}

/// A Conversation's own terminal comes up on PowerShell, in the Conversation's
/// Worktree.
///
/// The Screen's own machinery pointed at a shell rather than at an agent
/// (ADR 0013). There is no passwd database here to read a login shell out of,
/// so what a terminal opens on is `pwsh` where somebody installed PowerShell 7
/// and Windows PowerShell where nobody did — and which of those this machine is
/// is asked of the machine, with `where.exe`, rather than written down here.
#[tokio::test]
async fn a_terminal_runs_powershell_in_the_conversations_worktree() {
    let fixture = grilling(
        r#"
        Say 'reading the brief'
        Idle
        "#,
    )
    .await;

    let worktree = fixture.worktree().await;
    let at = fixture.listening().await;
    let number = fixture.terminal().await;

    // And it is on the list of live ones, which is what the pane asks for when
    // it loads and what a reload comes back to.
    let live: TerminalsView = get(
        &fixture.app,
        &format!("/api/ui/conversations/{}/terminals", fixture.id),
    )
    .await;

    assert_eq!(live.live, vec![number]);

    let mut watcher = Watcher::terminal(at, fixture.id, number).await;

    // Wide enough that the Worktree's own path lands on one row rather than
    // wrapping over the edge of the grid, a wrapped line being one no
    // `contains` finds.
    watcher.resize(200, 40).await;

    // Which PowerShell it is, is the shell's own answer: `Desktop` is Windows
    // PowerShell and `Core` is PowerShell 7, and a terminal that came up on
    // neither would print nothing at all.
    watcher.types("$PSVersionTable.PSEdition\r").await;

    let showing = watcher
        .until(|grid| grid.contains("Desktop") || grid.contains("Core"))
        .await;

    assert!(
        showing
            .iter()
            .any(|row| row.contains("Desktop") || row.contains("Core")),
        "a terminal here comes up on PowerShell, and it is showing: {showing:?}",
    );

    // And it is the one the machine has: `pwsh` where `where.exe` finds one,
    // and Windows PowerShell where it finds none.
    //
    // Compared as files rather than as text where there is a path to compare.
    // Both lookups walk the same `PATH`, and they part company over the
    // extension alone: `where.exe` prints the name as the directory holds it,
    // and the rendering's own appends whichever spelling `PATHEXT` carries —
    // which on a Windows machine is `.EXE`. Two names for the one file, which
    // is exactly the thing `the_same_file` is for.
    let chosen = verkstead_server::terminals::shell::of_the_server();

    match on_the_path("pwsh") {
        Some(pwsh) => the_same_file(Path::new(&chosen), Path::new(&pwsh)),
        None => assert_eq!(
            chosen, "powershell.exe",
            "a machine with no PowerShell 7 on it opens its terminals on the \
             PowerShell it does have, named rather than looked up",
        ),
    }

    // And it is standing in the Conversation's Worktree, which is where a
    // Terminal is (ADR 0013).
    //
    // Asked by having it write a file rather than by reading a path off the
    // grid: what a Windows path is spelled like is not one answer — a short
    // name, a long one, and whichever case each end of it chose — so a file
    // that turns up in the Worktree is the claim without the spelling.
    watcher
        .types("Set-Content -LiteralPath 'stood-here.txt' -Value 'here'\r")
        .await;

    until_there(&worktree.join("stood-here.txt")).await;
}

/// The shared compile server comes up on this platform too, as a plain process.
///
/// The one thing the open rendering runs that is not a session: an sccache
/// server, in a sandbox of its own, that every session's `rustc` goes through.
/// Asked of the machine rather than of Verkstead — a process id `tasklist` can
/// see is a process that is really running — because what is being proved is
/// that the rendering starts something outside this process.
///
/// **The runner needs an sccache**, which is what the workflow installs: this
/// is the case that only exists where one is on the server's `PATH`, and a
/// machine without one has nothing here to prove.
#[tokio::test]
async fn the_shared_compile_server_comes_up_as_a_plain_process() {
    // The one test here that builds no fixture, and the compile server has as
    // much to say for itself as a session does — see [`LOGGING`].
    LazyLock::force(&LOGGING);

    let state = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();

    let build_cache =
        BuildCache::resolve(Some(cache.path()), state.path()).expect("a cache to resolve");

    assert!(
        build_cache.caches_compiles(),
        "this machine has no sccache on the server's PATH, so there is no \
         compile server to come up: install one, as the Windows job does",
    );

    let already = servers();
    let settings = Settings::in_data_dir(state.path()).config();

    build_cache.compiling(settings.rust_build_cache());

    let deadline = Instant::now() + PATIENCE;

    loop {
        let started: BTreeSet<u32> = servers().difference(&already).copied().collect();

        if !started.is_empty() {
            break;
        }

        assert!(
            Instant::now() < deadline,
            "the compile server never came up: {already:?} were running before, \
             and {:?} are now",
            servers(),
        );

        pause(Duration::from_millis(200)).await;
    }
}

/// And a session whose server has one is told where it is.
///
/// The far end of the same fact: a session compiles through the server above,
/// and what points it there is `RUSTC_WRAPPER`. On this platform that names the
/// sccache where it really is — nothing is joined into a Windows sandbox, and
/// a name with the extension taken off it would be one nothing there can start.
#[tokio::test]
async fn a_session_is_told_where_the_sccache_it_compiles_through_is() {
    let cache = tempfile::tempdir().unwrap();

    let fixture = grilling_caching(
        r#"
        Note 'wrapper' $env:RUSTC_WRAPPER
        Note 'cargo-home' $env:CARGO_HOME
        "#,
        Some(cache.path()),
    )
    .await;

    let sccache = on_the_path("sccache").unwrap_or_else(|| {
        panic!(
            "this machine has no sccache on the server's PATH, so a session has \
             nothing to be told about: install one, as the Windows job does"
        )
    });

    the_same_file(
        Path::new(&fixture.written("wrapper").await),
        Path::new(&sccache),
    );

    // Spelled rather than resolved, which is the one comparison here that has
    // to be: `CARGO_HOME` is a directory the first `cargo` to run under it
    // makes, and nothing in this test runs one — so what is being asked is that
    // the server composed the name out of the cache directory it was handed,
    // and a name is all there is to compare. The line above is the other way
    // round: an sccache found on the `PATH` is a real file spelled however
    // `where.exe` spells it.
    assert_eq!(
        fixture.written("cargo-home").await,
        cache.path().join("cargo").display().to_string(),
        "a session's cargo downloads go under the shared cache the server was \
         given, which is what makes them shared",
    );
}

/// Where a program is on this machine, asked the way this machine answers —
/// which is `where.exe`, and is deliberately not the server's own lookup.
///
/// The tests above are about what Verkstead resolved, so what they check it
/// against has to be something else: `crate::sandbox::open::found` compared with
/// itself would pass whatever it answered.
fn on_the_path(program: &str) -> Option<String> {
    let found = Command::new("where.exe")
        .arg(program)
        .stdin(Stdio::null())
        .output()
        .expect("where.exe is part of Windows");

    if !found.status.success() {
        return None;
    }

    String::from_utf8_lossy(&found.stdout)
        .lines()
        .next()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
}

/// Every sccache server running on this machine right now, by process id.
///
/// `tasklist` because it is what Windows itself says about which processes
/// there are — see `tests/terminal_windows.rs`, which asks it the other way
/// round.
fn servers() -> BTreeSet<u32> {
    let listed = Command::new("tasklist.exe")
        .args(["/fi", "IMAGENAME eq sccache.exe", "/fo", "csv", "/nh"])
        .stdin(Stdio::null())
        .output()
        .expect("tasklist is part of Windows");

    String::from_utf8_lossy(&listed.stdout)
        .lines()
        .filter_map(|row| row.split(',').nth(1))
        .filter_map(|field| field.trim_matches('"').parse().ok())
        .collect()
}

/// A browser watching one live session's Screen: the socket it is attached
/// over, and the terminal it is painting on.
///
/// The other suite's, in the shape it has there: a terminal of its own rather
/// than a list of the messages that arrived, because that is what a browser
/// *is* here.
struct Watcher {
    socket: WebSocketStream<MaybeTlsStream<TcpStream>>,
    vt: Vt,
}

impl Watcher {
    /// Attach to a session's Screen the way the workbench does.
    async fn attach(at: SocketAddr, conversation: i64, event: i64) -> Watcher {
        Watcher::watching(format!(
            "ws://{at}/api/ui/conversations/{conversation}/screen/{event}/attach"
        ))
        .await
    }

    /// And to one of the Conversation's own terminals, which is the same socket
    /// pointed at a shell rather than at a session (ADR 0013).
    async fn terminal(at: SocketAddr, conversation: i64, number: i64) -> Watcher {
        Watcher::watching(format!(
            "ws://{at}/api/ui/conversations/{conversation}/terminals/{number}/attach"
        ))
        .await
    }

    /// Either of them: dial the socket and take the repaint it opens with.
    async fn watching(url: String) -> Watcher {
        let (socket, _) = tokio_tungstenite::connect_async(&url)
            .await
            .unwrap_or_else(|error| panic!("{url} to be attachable: {error}"));

        let mut watcher = Watcher {
            socket,
            // Replaced by the repaint's own size below.
            vt: Vt::new(1, 1),
        };

        match watcher.shown().await {
            Shown::Painted(painted) => watcher.paint(&painted),
            shown => panic!("a socket should open with a repaint, and it said: {shown:?}"),
        }

        watcher
    }

    /// The next thing the server says, or a panic if it says nothing for long
    /// enough.
    async fn shown(&mut self) -> Shown {
        let said = tokio::time::timeout(PATIENCE, self.socket.next())
            .await
            .expect("the server to say something")
            .expect("the socket to still be open")
            .expect("the socket to be readable");

        match said {
            Message::Text(said) => read(&said),
            said => panic!("a Screen's socket speaks JSON, and it said: {said:?}"),
        }
    }

    /// Start again on a grid the repaint's size, and paint it.
    fn paint(&mut self, painted: &verkstead_render::Screen) {
        self.vt = Vt::new(usize::from(painted.columns), usize::from(painted.rows));
        self.vt.feed_str(&painted.repaint);
    }

    /// Follow the socket until the grid says something, and hand back what it is
    /// showing.
    async fn until(&mut self, enough: impl Fn(&str) -> bool) -> Vec<String> {
        let deadline = Instant::now() + PATIENCE;

        loop {
            let showing = self.showing();

            if enough(&showing.join("\n")) {
                return showing;
            }

            assert!(
                Instant::now() < deadline,
                "the Screen never showed it. It is showing: {showing:?}"
            );

            match self.shown().await {
                Shown::Painted(painted) => self.paint(&painted),
                Shown::Printed(printed) => {
                    self.vt.feed_str(&printed);
                }
            }
        }
    }

    /// Type into it: the bytes a terminal would have made of the keys, because
    /// that is what the browser sends.
    async fn types(&mut self, keys: &str) {
        let said = serde_json::to_string(&Watching::PutIn(keys.to_owned())).unwrap();

        self.socket
            .send(Message::Text(said.into()))
            .await
            .expect("the socket to take a keystroke");
    }

    /// This watcher's window is now that big — which is the Screen's size from
    /// here on, for everybody watching it.
    async fn resize(&mut self, columns: u16, rows: u16) {
        let said = serde_json::to_string(&Watching::Resized(Size { columns, rows })).unwrap();

        self.socket
            .send(Message::Text(said.into()))
            .await
            .expect("the socket to take a resize");
    }

    /// The grid it is showing, row by row, with the blank rows at the bottom
    /// left off.
    fn showing(&self) -> Vec<String> {
        let mut rows: Vec<String> = self
            .vt
            .view()
            .map(|line| line.text().trim_end().to_owned())
            .collect();

        while rows.last().is_some_and(|row| row.is_empty()) {
            rows.pop();
        }

        rows
    }
}
