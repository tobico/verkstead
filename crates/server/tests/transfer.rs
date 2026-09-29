//! **The move itself**: a Conversation pressed onto another device of the
//! cluster, and what each end holds afterwards (ADR-0020, *Transfer*).
//!
//! **Two Verksteads, a real dial and a real session.** `tests/preflight.rs` is
//! the reading that says whether the press can be made; this is the press, the
//! wait for the turn to end, and the record crossing the link. So nothing here
//! is stood in for but the agent: the repositories are repositories, the
//! session is a real session in a real sandbox, and the Conversation lands on B
//! through the Peer Listener behind the Member Gate.
//!
//! **What stands where claude goes is a shell script that waits at a gate the
//! test opens**, which is `tests/sessions.rs`'s way of holding a turn open: the
//! press has to arrive while a session is genuinely part way through one, and
//! what the press promises is that nothing is cut short.
//!
//! **What crosses in this stage is the Conversation itself** — the Repo the
//! matching settled, the branch, the lifecycle, the Pairings as B's own Profile
//! ids, the Rank verbatim and the birth key. The Timeline, the branch and the
//! Worktree are the tasks after this one, and nothing here asks after them.
//!
//! **On the machine this suite is about**: the sandbox is bwrap and the
//! terminal is a real pseudo-terminal, which is what `tests/sessions.rs` and
//! `tests/accounts.rs` are written for and the reason this file is Unix-only
//! too.
#![cfg(unix)]

use std::net::SocketAddr;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use axum::Router;
use axum::body::Body;
use axum::http::header::CONTENT_TYPE;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use sqlx::SqlitePool;
use tokio::task::JoinHandle;
use tower::ServiceExt;
use verkstead_render::{
    ConversationEntry, ConversationView, Lifecycle, ProfileEntry, Started, TimelineEvent,
};
use verkstead_server::attachments::Attachments;
use verkstead_server::build_cache::BuildCache;
use verkstead_server::device::reading::Reading;
use verkstead_server::device::{Device, Devices};
use verkstead_server::handoffs::Handoffs;
use verkstead_server::nudge::Nudges;
use verkstead_server::onboarding::Machine;
use verkstead_server::peer::joining::Joins;
use verkstead_server::peer::{self, Members};
use verkstead_server::platform::{self, Environment, Platform};
use verkstead_server::remote::Tailscale;
use verkstead_server::sandbox::{Executable, Homes, Reachable, SandboxConfig};
use verkstead_server::settings::Settings;
use verkstead_server::skills::Skills;
use verkstead_server::{
    Agents, Gh, Routers, open_database, routers_answering_devices_telling,
    routers_running_sessions_answering_devices, store,
};
use verkstead_store::{Linking, record_member};

/// The device the work is on, where every press is made.
const A: &str = "aa00bb11cc22dd33ee44ff5566778899";

/// And the device it is moved onto.
const B: &str = "0011223344556677889900aabbccddee";

/// What B is written down as on A: the name the human gave the machine, which is
/// what the Timeline says the work is going to rather than sixteen bytes of hex.
const B_MACHINE: &str = "the-laptop";
const B_OS: &str = "macOS 15.1";

/// The Profiles section, which is where an account is saved and where a mirror
/// of a member's turns up.
const PROFILES: &str = "/api/ui/profiles";

/// The sidebar, which is where a **Rank** and a **birth key** are read off a row.
const SIDEBAR: &str = "/api/ui/conversations";

/// The account the work runs under, and the model it lists.
const ACCOUNT: &str = "work";
const MODEL: &str = "claude-opus-5";

/// What every repository in this suite is called. Neither end has an origin, so
/// the match is by name — which is the ordinary case for two clones nobody has
/// pushed anywhere.
const REPOSITORY: &str = "verkstead";

/// The Brief every Conversation here is started from.
const BRIEF: &str = "# Rate limiting\n\nThe API has none.\n";

/// A `gh` that answers nothing: nothing here reaches GitHub, and the real one
/// would need an account.
const NO_GITHUB: &str = "exit 1";

/// How long one address has to answer here, rather than the two seconds a
/// running server gives one.
const PATIENCE: Duration = Duration::from_millis(300);

/// How long a test waits for something two hops away: a session starting in a
/// real sandbox, a mirror written off a member's list, a record crossing a link.
const WAITING: Duration = Duration::from_secs(30);

/// And how often it looks while it waits.
const LOOKING: Duration = Duration::from_millis(50);

/// Where the server these sessions belong to would be listening. Nothing dials
/// it: a router driven by `oneshot` has no socket, and what this is for is the
/// `VERKSTEAD_SERVER` a session inside is told.
const LISTENING: SocketAddr =
    SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), 8422);

/// One Verkstead: its store, its identity, its Peer Listener up behind the
/// Member Gate, and the router its own browser presses.
struct Verkstead {
    device: Device,
    pool: SqlitePool,
    nudges: Nudges,
    cluster: Devices,
    workbench: Router,
    address: SocketAddr,

    /// The listener, held so that a test can switch this machine off part way
    /// through — which is what a move that fails between the press and the
    /// crossing is made of.
    listening: JoinHandle<anyhow::Result<()>>,

    /// The Data Directory: the identity, the database and every session's own
    /// profile are in it. Held for the length of the test rather than read: what
    /// it is for is that none of it is swept out from under a running session.
    _dir: tempfile::TempDir,

    /// And what `~` is for a session on this device, held for the same reason.
    _home: tempfile::TempDir,

    /// And where this device's repositories and accounts are, away from the Data
    /// Directory so that nothing confuses a repository with a Worktree.
    elsewhere: tempfile::TempDir,
}

impl Verkstead {
    /// A Verkstead that runs no session of its own, which is what B is here: the
    /// work arrives as a row, and starting it again is stage 07's.
    async fn answering(id: &str) -> Verkstead {
        Verkstead::standing(id, None, Path::new("/")).await
    }

    /// And one that runs its sessions on `stub`, with `spill` bound into the
    /// sandbox so a session can see the gate the test opens.
    async fn running(id: &str, stub: &str, spill: &Path) -> Verkstead {
        Verkstead::standing(id, Some(stub), spill).await
    }

    async fn standing(id: &str, stub: Option<&str>, spill: &Path) -> Verkstead {
        let dir = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        let elsewhere = tempfile::tempdir().unwrap();

        // Who a session commits as, which every sandbox is configured out of.
        std::fs::write(
            dir.path().join("config.yaml"),
            "git_author:\n  name: Verkstead Test\n  email: test@verkstead.invalid\n",
        )
        .unwrap();

        let pool = open_database(&dir.path().join("verkstead.db"))
            .await
            .unwrap();

        let device = Device::stated(dir.path(), id).unwrap();
        let members = Members::recorded(pool.clone());
        let nudges = Nudges::new();

        let listener = peer::Listener::bound("127.0.0.1:0".parse().unwrap(), &device)
            .expect("the loopback on a port the machine picked is free");
        let address = listener.address();

        let reading = Reading::advertising(
            no_tailscale(),
            Platform::Linux,
            None,
            vec![format!("127.0.0.1:{}", address.port())],
        );

        let cluster = Devices::of(
            device.clone(),
            reading.clone(),
            members.clone(),
            Joins::none(),
        )
        .waiting(PATIENCE);

        let Routers {
            workbench,
            over_the_link,
        } = match stub {
            None => routers_answering_devices_telling(
                pool.clone(),
                cluster.clone(),
                nudges.clone(),
                probing(dir.path()),
            ),

            Some(stub) => routers_running_sessions_answering_devices(
                pool.clone(),
                dir.path().to_owned(),
                agents(stub, home.path(), dir.path(), spill),
                gh_stub(NO_GITHUB),
                cluster.clone(),
                nudges.clone(),
                probing(dir.path()),
            ),
        };

        let listening = tokio::spawn(listener.serving(peer::router(
            device.clone(),
            reading,
            members,
            Joins::none(),
            nudges.clone(),
            over_the_link,
        )));

        Verkstead {
            device,
            pool,
            nudges,
            cluster,
            workbench,
            address,
            listening,
            _dir: dir,
            _home: home,
            elsewhere,
        }
    }

    /// Hold a Nudge stream to every member, which is what a running server
    /// spawns at the start: it is what refreshes this device's mirrors of a
    /// member's Profiles and what keeps its merged sidebar fresh.
    fn holding(&self) -> Holding {
        let cluster = self.cluster.clone();
        let nudges = self.nudges.clone();

        Holding(tokio::spawn(async move {
            cluster.stay_fresh(nudges).await;
        }))
    }

    /// The one address it advertises.
    fn at(&self) -> String {
        format!("127.0.0.1:{}", self.address.port())
    }

    /// The machine switched off: the listener stops answering, and every dial at
    /// it from here on is a device that is not there.
    fn stops_answering(&self) {
        self.listening.abort();
    }

    /// Write `other` down as one of this device's members.
    async fn linked_to(&self, other: &Device, name: &str, os: &str, addresses: Vec<String>) {
        record_member(
            &self.pool,
            &Linking {
                device: other.id().to_owned(),
                name: name.to_owned(),
                os: os.to_owned(),
                addresses,
                fingerprint: other.fingerprint().to_owned(),
            },
        )
        .await
        .unwrap();
    }

    /// A repository of this device's own, really made and really registered.
    ///
    /// Two of these, one per device, are what the match settles between: neither
    /// has an origin, so what says they are one repository is the directory's own
    /// name.
    async fn repo(&self) -> PathBuf {
        let path = repository(self.elsewhere.path().join(REPOSITORY));

        let registered = press(
            &self.workbench,
            "/api/ui/repos",
            Some(&serde_json::json!({ "path": path }).to_string()),
        )
        .await;

        assert!(registered.contains("Added"), "registering the repository");

        path
    }

    /// An account of this device's own, really on disk, with a Profile saved over
    /// it the way the form saves one.
    async fn account(&self) -> i64 {
        let under = self.elsewhere.path().join(ACCOUNT);
        let claude_dir = under.join(".claude");
        let config_file = under.join(".claude.json");

        std::fs::create_dir_all(&claude_dir).unwrap();
        std::fs::write(&config_file, "{}\n").unwrap();
        std::fs::write(
            claude_dir.join(".credentials.json"),
            "{\"claudeAiOauth\":{\"accessToken\":\"sk-ant-oat01-the-desk\"}}",
        )
        .unwrap();

        let said = press(
            &self.workbench,
            PROFILES,
            Some(
                &serde_json::json!({
                    "name": ACCOUNT,
                    "account": {
                        "agent_type": "Claude",
                        "claude_dir": claude_dir,
                        "config_file": config_file,
                    },
                    "models": [MODEL],
                    "memory": true,
                })
                .to_string(),
            ),
        )
        .await;

        assert_eq!(said, "\"Saved\"", "saving the account");

        self.profile_called(ACCOUNT).await.id
    }

    /// The row of this device's Profiles with that name, once there is one —
    /// which on B is the **mirror** its refresher writes off A's list.
    async fn profile_called(&self, name: &str) -> ProfileEntry {
        let mut last = Vec::new();

        let waited = tokio::time::timeout(WAITING, async {
            loop {
                last = reading(&self.workbench, PROFILES).await;

                if let Some(row) = last
                    .iter()
                    .find(|row: &&ProfileEntry| row.name.as_deref() == Some(name))
                {
                    return row.clone();
                }

                tokio::time::sleep(LOOKING).await;
            }
        })
        .await;

        waited.unwrap_or_else(|_| panic!("no Profile called {name} was ever listed: {last:#?}"))
    }

    /// A Conversation grilling under `profile`, started the way the composer and
    /// the setup card start one.
    async fn grilling_under(&self, profile: i64) -> i64 {
        let repos: Vec<verkstead_render::RepoEntry> =
            reading(&self.workbench, "/api/ui/repos").await;

        let started = press(
            &self.workbench,
            SIDEBAR,
            Some(&serde_json::json!({ "repo_id": repos[0].id }).to_string()),
        )
        .await;

        let Started::Started {
            id: conversation, ..
        } = serde_json::from_str(&started).unwrap()
        else {
            panic!("the Conversation was not started: {started}")
        };

        let pairing = serde_json::json!({ "profile_id": profile, "model": MODEL });
        let role = serde_json::json!({ "pairing": pairing });

        for (path, saying) in [
            ("grilling-pairing", &pairing),
            ("implementation-pairing", &pairing),
            ("review-pairing", &role),
        ] {
            let said = press(
                &self.workbench,
                &format!("/api/ui/conversations/{conversation}/{path}"),
                Some(&saying.to_string()),
            )
            .await;

            assert_eq!(said, "\"Chosen\"", "picking {path}");
        }

        let saved = press(
            &self.workbench,
            &format!("/api/ui/conversations/{conversation}/brief"),
            Some(&serde_json::json!({ "markdown": BRIEF }).to_string()),
        )
        .await;

        assert_eq!(saved, "\"Saved\"", "writing the Brief");

        let grilling = press(
            &self.workbench,
            &format!("/api/ui/conversations/{conversation}/grill"),
            None,
        )
        .await;

        assert_eq!(grilling, "\"Started\"", "starting the grilling");

        conversation
    }

    /// Press *Transfer to…* for `device`, and answer with what it said.
    async fn transfers(&self, conversation: i64, device: &str) -> String {
        press(
            &self.workbench,
            &format!("/api/ui/conversations/{conversation}/transfer/{device}"),
            None,
        )
        .await
    }

    /// One of its Conversations as its own browser reads it.
    async fn view(&self, conversation: i64) -> ConversationView {
        reading(
            &self.workbench,
            &format!("/api/ui/conversations/{conversation}"),
        )
        .await
    }

    /// The same, polled until it says what the test is waiting for — or a panic
    /// saying what it says instead.
    async fn view_saying(
        &self,
        conversation: i64,
        what: impl Fn(&ConversationView) -> bool,
    ) -> ConversationView {
        let deadline = Instant::now() + WAITING;

        loop {
            let drawn = self.view(conversation).await;

            if what(&drawn) {
                return drawn;
            }

            assert!(
                Instant::now() < deadline,
                "Conversation {conversation} never said it. It says: \
                 state {:?}, working {}, transferring {:?}, transferred {:?}, stopped {:?}",
                drawn.state,
                drawn.working,
                drawn.transferring,
                drawn.transferred,
                drawn.blocked_on,
            );

            tokio::time::sleep(LOOKING).await;
        }
    }

    /// The Conversations this device holds rows for of its own, which is a
    /// different question from its sidebar: that one is the cluster's merged
    /// list, and what is asked here is whether anything was written *here*.
    async fn own_rows(&self) -> Vec<verkstead_store::ConversationRow> {
        store::conversations(&self.pool, true).await.unwrap()
    }

    /// The sidebar as this device's own browser reads it, which is where a Rank
    /// and a birth key ride out.
    async fn sidebar(&self) -> Vec<ConversationEntry> {
        reading(&self.workbench, SIDEBAR).await
    }

    /// The same, polled until it says it — a merged list being two hops from
    /// whatever changed on the far end.
    async fn sidebar_saying(
        &self,
        what: impl Fn(&[ConversationEntry]) -> bool,
    ) -> Vec<ConversationEntry> {
        let mut last = Vec::new();

        let waited = tokio::time::timeout(WAITING, async {
            loop {
                last = self.sidebar().await;

                if what(&last) {
                    return last.clone();
                }

                tokio::time::sleep(LOOKING).await;
            }
        })
        .await;

        waited.unwrap_or_else(|_| panic!("the sidebar never said it: {last:#?}"))
    }

    /// Wait until a session is running on that Conversation and has printed
    /// `saying`, which is what says a turn is genuinely in flight.
    async fn printed(&self, conversation: i64, saying: &str) {
        let deadline = Instant::now() + WAITING;
        let mut said = String::new();

        loop {
            if let Some(text) = self.capture(conversation).await {
                said = text;
            }

            if said.contains(saying) {
                return;
            }

            assert!(
                Instant::now() < deadline,
                "the session never printed {saying:?}. It printed: {said:?}",
            );

            tokio::time::sleep(LOOKING).await;
        }
    }

    /// The whole of what the session on that Conversation has said, as the
    /// details pane fetches it.
    async fn capture(&self, conversation: i64) -> Option<String> {
        let drawn = self.view(conversation).await;

        let event = drawn.timeline.iter().find_map(|event| match event {
            TimelineEvent::AgentOutput(output) => Some(output.id),
            _ => None,
        })?;

        let capture: verkstead_render::Capture = reading(
            &self.workbench,
            &format!("/api/ui/conversations/{conversation}/capture/{event}"),
        )
        .await;

        Some(capture.text)
    }

    /// And every Notice on its Timeline, which is where Verkstead says what it
    /// did on its own account — a move that failed among them.
    async fn notices(&self, conversation: i64) -> Vec<String> {
        self.view(conversation)
            .await
            .timeline
            .iter()
            .filter_map(|event| match event {
                TimelineEvent::Notice(notice) => Some(notice.html.clone()),
                _ => None,
            })
            .collect()
    }
}

/// The streams one device is holding, given back when the test drops it.
struct Holding(JoinHandle<()>);

impl Drop for Holding {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// What stands where claude goes: a session that says it is grilling, waits at
/// the gate the test opens, and stops.
///
/// The wait is the whole of what this suite needs of an agent. A press that
/// arrived after the session had gone would prove nothing about the promise the
/// press makes, which is that a turn part way through runs to its own end.
fn waits_at(gate: &Path) -> String {
    format!(
        r#"
printf 'grilling\r\n'
while [ ! -f {gate} ]; do sleep 0.05; done
printf 'the turn is over\r\n'
"#,
        gate = quoted(gate),
    )
}

/// A path as a shell script can carry it.
fn quoted(path: &Path) -> String {
    format!("'{}'", path.display().to_string().replace('\'', r"'\''"))
}

/// The agent a session on this device runs, and everything else a launch is
/// equipped out of.
fn agents(stub: &str, home: &Path, data_dir: &Path, spill: &Path) -> Agents {
    Agents::running(
        vec!["/bin/sh".to_owned(), "-c".to_owned(), stub.to_owned()],
        Homes::on(Platform::HERE, home.to_owned(), data_dir),
        Reachable::at(LISTENING),
        // The gate the stub waits at is outside the Worktree, so the sandbox has
        // to be able to see it.
        SandboxConfig::resolve(&[spill.display().to_string()]).unwrap(),
        // Nothing here builds anything: what runs where claude goes is a shell
        // script.
        BuildCache::none(),
        Skills::installed(Platform::HERE, data_dir).expect("this binary carries skills"),
        Executable::of_the_server(data_dir),
        Handoffs::under(data_dir),
        Attachments::under(data_dir),
        Settings::in_data_dir(data_dir),
    )
}

/// A `gh` that is a shell script, so nothing here reaches GitHub.
fn gh_stub(script: &str) -> Gh {
    Gh::running(vec![
        "/bin/sh".to_owned(),
        "-c".to_owned(),
        script.to_owned(),
        "gh".to_owned(),
    ])
}

/// A machine with no Tailscale on it: `verkstead-no-such-tailscale` is a program
/// that is not there, which is what having none *is*.
fn no_tailscale() -> Tailscale {
    Tailscale::running(vec!["verkstead-no-such-tailscale".to_owned()], 8422)
}

/// The machine a device here is judged on: one directory on the `PATH` a session
/// would search, holding a program for every harness.
///
/// **Stated rather than read off the runner**, as the suites beside this one
/// state theirs: whether a harness is on the far end is what `tests/preflight.rs`
/// is about, and here it must not be what decides whether a press can be made.
fn probing(dir: &Path) -> Machine {
    let bin = dir.join("probed");
    std::fs::create_dir_all(&bin).unwrap();

    for harness in ["claude", "codex", "grok", "opencode"] {
        let at = bin.join(harness);

        std::fs::write(&at, "#!/bin/sh\nexit 0\n").unwrap();
        std::fs::set_permissions(&at, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    Machine::stated(
        Platform::Linux,
        bin.as_os_str().to_owned(),
        bin.as_os_str().to_owned(),
        None,
        None,
        &Environment {
            home: Some(dir.to_owned()),
            ..Environment::default()
        },
    )
}

/// A and B, linked both ways, each with the repository registered — and B
/// holding the streams that write its mirror of A's account, which is what a
/// Pairing arriving there resolves against.
///
/// The Conversation is grilling on A under that account, with its session
/// waiting at `gate`.
async fn ready_to_move(gate: &Path, spill: &Path) -> (Verkstead, Verkstead, Holding, i64) {
    let a = Verkstead::running(A, &waits_at(gate), spill).await;
    let b = Verkstead::answering(B).await;

    a.linked_to(&b.device, B_MACHINE, B_OS, vec![b.at()]).await;

    let (machine, os) = this_machine();
    b.linked_to(&a.device, &machine, &os, vec![a.at()]).await;

    a.repo().await;
    b.repo().await;

    let account = a.account().await;

    // B's mirror of that account, which is stage 08's machinery and what the
    // Pairings arriving there are resolved against: a Profile is named across a
    // cluster by the device it is at home on and its id there.
    let holding = b.holding();
    b.profile_called(ACCOUNT).await;

    let conversation = a.grilling_under(account).await;

    (a, b, holding, conversation)
}

/// What a device says about the machine it is on, read the way the server reads
/// it.
fn this_machine() -> (String, String) {
    (
        platform::hostname(),
        platform::os_word(Platform::Linux, None),
    )
}

/// A git repository at `path`, with one commit on `main` so there is a branch to
/// cut a worktree off.
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

/// Run git in `dir`, and fail the test where it does not.
fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("git should be on the PATH for these tests");

    assert!(status.success(), "git {args:?} failed in {}", dir.display());
}

/// Whether a row of a merged list is this device's own copy of Conversation
/// `id`.
///
/// Every row of a merged list says whose it is, and this device's own say so by
/// naming no device inside — see `RowDevice`, whose `null` id is the reading
/// device itself.
fn ours(row: &ConversationEntry, id: i64) -> bool {
    row.id == id && row.device.as_ref().is_none_or(|whose| whose.id.is_none())
}

/// A read of whatever `path` answers, made the way the browser makes one.
async fn reading<T: serde::de::DeserializeOwned>(app: &Router, path: &str) -> T {
    let answered = app
        .clone()
        .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
        .await
        .unwrap();

    let status = answered.status();
    let bytes = answered.into_body().collect().await.unwrap().to_bytes();
    let said = String::from_utf8_lossy(&bytes).into_owned();

    assert_eq!(status, StatusCode::OK, "GET {path}: {said}");

    serde_json::from_str(&said).unwrap_or_else(|why| panic!("GET {path} answered {said}: {why}"))
}

/// And a press on `path`, with a body where the endpoint takes one.
async fn press(app: &Router, path: &str, saying: Option<&str>) -> String {
    let asking = Request::builder().method("POST").uri(path);

    let asking = match saying {
        Some(body) => asking
            .header(CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_owned())),
        None => asking.body(Body::empty()),
    };

    let answered = app.clone().oneshot(asking.unwrap()).await.unwrap();

    let status = answered.status();
    let bytes = answered.into_body().collect().await.unwrap().to_bytes();
    let said = String::from_utf8_lossy(&bytes).into_owned();

    assert!(status.is_success(), "POST {path}: {status} {said}");

    said
}

/// **The press waits for the turn to end**, which is the whole of what it
/// promises: the session running when it is made runs to its own end, nothing is
/// started after it, and the work goes once there is nothing left running.
///
/// And from the press until it lands the Conversation says where it is going,
/// which is what the head of its Timeline reads.
#[tokio::test]
async fn a_press_over_a_running_session_waits_for_the_turn_to_end() {
    let spill = tempfile::tempdir().unwrap();
    let gate = spill.path().join("go");
    let (a, b, _holding, conversation) = ready_to_move(&gate, spill.path()).await;

    // A turn genuinely in flight: the session is up, in a real sandbox, part way
    // through what it was launched for.
    a.printed(conversation, "grilling").await;

    assert_eq!(
        a.transfers(conversation, B).await,
        "\"Transferring\"",
        "the press is taken",
    );

    // What the head of the Timeline reads from here: the machine the work is
    // going to, by the name the human gave it.
    let going = a.view(conversation).await;
    assert_eq!(going.transferring.as_deref(), Some(B_MACHINE));
    assert!(
        going.transferred.is_none(),
        "and nothing has moved: this copy is still the record",
    );

    // Nothing is cut short. The session is still running and still printing,
    // because what the press asked for is the turn's end rather than the
    // session's.
    assert!(
        going.working,
        "the session the press arrived over is running"
    );
    assert!(
        b.own_rows().await.is_empty(),
        "and nothing has reached the far end while it is",
    );

    // The turn ends.
    std::fs::write(&gate, "go").unwrap();

    // And the move runs: B holds the work, and A's copy says where it went.
    let handed = a
        .view_saying(conversation, |drawn| drawn.transferred.is_some())
        .await;

    let there = handed.transferred.expect("the mark the move wrote");
    assert_eq!(there.device, B, "the work is on B");

    assert_eq!(
        handed.transferring, None,
        "and the Conversation is no longer going anywhere: it has gone",
    );

    let arrived = b.view(there.id).await;
    assert_eq!(
        arrived.branch, handed.branch,
        "the branch the work is on crossed with it",
    );
}

/// **What arrives is the Conversation**: the state it was in, the Repo the
/// matching settled, its Pairings as B's own ids, and the two things that are
/// the cluster's rather than either machine's — the Rank and the birth key.
#[tokio::test]
async fn the_conversation_arrives_in_the_state_it_was_in() {
    let spill = tempfile::tempdir().unwrap();
    let gate = spill.path().join("go");
    let (a, b, _holding, conversation) = ready_to_move(&gate, spill.path()).await;

    // The turn is over before the press is made, which is the other way one
    // arrives: there is nothing to see out, and the move runs where it stands.
    a.printed(conversation, "grilling").await;
    std::fs::write(&gate, "go").unwrap();
    a.view_saying(conversation, |drawn| !drawn.working).await;

    // What the work was, before it went: the row's Rank and the key it was born
    // under, read off this device's own store.
    let rank = store::conversation_rank(&a.pool, conversation)
        .await
        .unwrap()
        .expect("every Conversation is ranked as it is started");
    let born = store::birth(&a.pool, conversation)
        .await
        .unwrap()
        .expect("every Conversation is stamped as it is started");

    let branch = a.view(conversation).await.branch;

    assert_eq!(a.transfers(conversation, B).await, "\"Transferring\"");

    let there = a
        .view_saying(conversation, |drawn| drawn.transferred.is_some())
        .await
        .transferred
        .expect("the mark the move wrote")
        .id;

    let arrived = b.view(there).await;

    assert_eq!(
        arrived.state,
        Lifecycle::Grilling,
        "it arrives in the state it was in",
    );
    assert_eq!(arrived.branch, branch, "on the branch the work is on");
    assert_eq!(
        arrived.repo.path,
        b.elsewhere.path().join(REPOSITORY).display().to_string(),
        "under B's own Repo, which is the one the matching settled — and not a \
         path of A's",
    );

    // The Pairings are ids of B's own: the account is at home on A, so what B
    // holds is a mirror of it, and that is what the row names.
    let mirror = b.profile_called(ACCOUNT).await;
    let pairing = arrived
        .grilling_pairing
        .as_ref()
        .expect("the grilling Pairing crossed with it");

    assert_eq!(pairing.profile.id, mirror.id, "B's own id for that account");
    assert_eq!(
        pairing.model.as_deref(),
        Some(MODEL),
        "and the model with it"
    );
    assert_eq!(
        arrived
            .implementation_pairing
            .as_ref()
            .map(|pairing| pairing.profile.id),
        Some(mirror.id),
        "and the implementation's, which is the same account here",
    );

    // And the two the cluster names it by, verbatim: the Rank carries the device
    // that issued it and the birth key says where the work was drafted, so
    // neither is B's to invent.
    let row = b
        .sidebar_saying(|rows| rows.iter().any(|row| ours(row, there)))
        .await
        .into_iter()
        .find(|row| ours(row, there))
        .expect("B's own row for the work that arrived");

    assert_eq!(row.rank, rank, "it keeps its place in the merged order");
    assert_eq!(
        row.born,
        born.key(),
        "and the key it was born under, which is what says the two copies are \
         one piece of work",
    );
}

/// **The source is marked only once the far end has confirmed**, and what it
/// keeps is a tombstone: the merged list draws the work once, and the copy's own
/// URL leads to wherever the record is now.
#[tokio::test]
async fn the_source_keeps_a_copy_that_leads_to_the_live_record() {
    let spill = tempfile::tempdir().unwrap();
    let gate = spill.path().join("go");
    let (a, b, _holding_there, conversation) = ready_to_move(&gate, spill.path()).await;

    // A holds its own streams too, so that its sidebar is the cluster's rather
    // than its own rows alone — which is what a merged list is.
    let _holding_here = a.holding();

    a.printed(conversation, "grilling").await;
    std::fs::write(&gate, "go").unwrap();
    a.view_saying(conversation, |drawn| !drawn.working).await;

    assert_eq!(
        a.sidebar().await.len(),
        1,
        "one row before the move, which is A's own",
    );

    assert_eq!(a.transfers(conversation, B).await, "\"Transferring\"");

    let handed = a
        .view_saying(conversation, |drawn| drawn.transferred.is_some())
        .await;

    let there = handed.transferred.expect("the mark the move wrote");

    assert_eq!(there.device, B);
    assert_eq!(
        b.view(there.id).await.transferred,
        None,
        "and B's copy is the record, wearing no mark of its own",
    );

    // One row, and it is B's: the tombstone comes off the merged list and the
    // live copy stands in its place, at the rank the work has always sat at.
    let drawn = a
        .sidebar_saying(|rows| {
            rows.iter().any(|row| {
                row.id == there.id
                    && row.device.as_ref().and_then(|whose| whose.id.as_deref()) == Some(B)
            })
        })
        .await;

    assert_eq!(drawn.len(), 1, "the work is drawn once: {drawn:#?}");
    assert_eq!(drawn[0].id, there.id, "and the row drawn is B's copy");
}

/// **A move that fails leaves the source live and stopped**: nothing is marked,
/// the Worktree is where it was, and the Notice names the machine and what went
/// wrong.
///
/// The machine goes off between the press and the crossing, which is the
/// ordinary way for this to happen: the preflight is a reading of a moment, and
/// a laptop's lid shuts.
#[tokio::test]
async fn a_move_that_fails_leaves_the_source_live_and_stopped() {
    let spill = tempfile::tempdir().unwrap();
    let gate = spill.path().join("go");
    let (a, b, _holding, conversation) = ready_to_move(&gate, spill.path()).await;

    a.printed(conversation, "grilling").await;

    assert_eq!(a.transfers(conversation, B).await, "\"Transferring\"");

    // The lid shuts while the turn is still running, so the press was made
    // against a machine that answered and the move meets one that does not.
    b.stops_answering();

    let worktree = a
        .view(conversation)
        .await
        .worktree
        .expect("the grilling cut a Worktree")
        .path;

    std::fs::write(&gate, "go").unwrap();

    let stopped = a
        .view_saying(conversation, |drawn| drawn.blocked_on.is_some())
        .await;

    assert_eq!(
        stopped.transferred, None,
        "nothing was marked: this copy is still the record",
    );
    assert_eq!(
        stopped.transferring, None,
        "and it is not on its way anywhere any more",
    );
    assert!(
        stopped.ready_to_resume,
        "it is stopped, which is a press away from being driven again",
    );

    let notices = a.notices(conversation).await;
    let said = notices
        .last()
        .expect("a stop writes a Notice saying what happened");

    assert!(
        said.contains(B_MACHINE),
        "the Notice names the machine: {said}",
    );

    assert!(
        std::path::Path::new(&worktree).is_dir(),
        "and the Worktree is where it was: {worktree}",
    );

    assert!(
        b.own_rows().await.is_empty(),
        "and nothing was left on the far end",
    );
}
