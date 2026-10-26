//! What a terminal is listening on, read for the devices attached to it from
//! elsewhere in the cluster.
//!
//! **Two Verksteads and a real terminal between them**, the way
//! `tests/bridging.rs` stands them up: a shell really running in B's own
//! Sandbox, attached to from a browser on A through A's relay, and a server
//! really started in it. What is asked is what B says of that server — in the
//! reading A takes over the link, and in the `ports` Nudge B announces when it
//! moves — and what B says to the devices that are *not* attached.
//!
//! **And the walk underneath, asked on its own** against a listener inside a
//! nested bwrap that has left its parent's session, which is the arrangement the
//! reading has to see through: a Sandbox's pid namespace, and a server that
//! `setsid` took out of the shell's tree.
//!
//! **Linux alone**: the two Verksteads stand on bwrap. The readers of the other
//! two platforms are proven by `terminals::ports`'s own tests, on the macOS and
//! Windows runners.
#![cfg(target_os = "linux")]

use std::net::{Ipv4Addr, SocketAddr, TcpListener};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use futures_util::{SinkExt, StreamExt};
use http_body_util::BodyExt;
use serde::de::DeserializeOwned;
use sqlx::SqlitePool;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::Instant;
use tokio_tungstenite::tungstenite::Message;
use tower::ServiceExt;
use verkstead_render::{
    ConversationPorts, ForwardSkip, ForwardStanding, ForwardView, ForwardsView, OfferedView,
    PortsView, Shown, TerminalOpened, TerminalPorts, Watching,
};
use verkstead_schema::{Nudge, Nudged};
use verkstead_server::attachments::Attachments;
use verkstead_server::build_cache::BuildCache;
use verkstead_server::device::reading::Reading;
use verkstead_server::device::{Device, Devices};
use verkstead_server::handoffs::Handoffs;
use verkstead_server::nudge::Nudges;
use verkstead_server::onboarding::Machine;
use verkstead_server::peer::joining::Joins;
use verkstead_server::peer::{self, Members};
use verkstead_server::platform::Platform;
use verkstead_server::remote::Tailscale;
use verkstead_server::sandbox::{Executable, Homes, Reachable, SandboxConfig};
use verkstead_server::settings::Settings;
use verkstead_server::skills::Skills;
use verkstead_server::terminals::ports;
use verkstead_server::{
    Agents, Gh, open_database, router_answering_devices_telling,
    routers_running_sessions_answering_devices, store,
};
use verkstead_store::{Linking, record_member};

/// The device the browser is on, which relays.
const A: &str = "aa00bb11cc22dd33ee44ff5566778899";

/// The device the Conversation and its terminal live on.
const B: &str = "0011223344556677889900aabbccddee";

/// The branch B's one Conversation is on.
const BRANCH: &str = "listening-over-there";

/// B's Conversation, which is the first one in a store with nothing else in it.
const THERE: i64 = 1;

/// How long a test waits for something it expects. `tests/bridging.rs`'s, and
/// for its reason: the machine's own login shell may sit on its first seconds
/// asking the terminal questions nothing answers, and fish waits ten.
const PATIENCE: Duration = Duration::from_secs(30);

/// How soon a port opening or closing has to be in the reading: the two seconds
/// the reading promises, with room over it for the relay and for this test's
/// own polling.
const PROMPTLY: Duration = Duration::from_secs(4);

/// How soon a server that printed its address has to be in the reading: well
/// inside the two seconds a turn takes.
const HURRIED: Duration = Duration::from_millis(1200);

/// How long to listen on for a Nudge that should not come at all: past a whole
/// turn of the reading, twice.
const SETTLING: Duration = Duration::from_secs(5);

/// How often a terminal is read that has been attached from elsewhere but has
/// no attach on it now: `terminals::UNWATCHED`.
const UNWATCHED: Duration = Duration::from_secs(10);

/// How long a forwarding device waits before taking a member's Nudge stream up
/// again once it has ended: `relaying::freshness::AGAIN`.
const STREAM_AGAIN: Duration = Duration::from_secs(5);

/// And how long it waits after a try that reached nobody: the wait doubles
/// while nothing answers, so a member found away is tried again this long
/// after.
const AGAIN_AFTER_A_MISS: Duration = Duration::from_secs(10);

/// How long a Forward outlives its port leaving the reading:
/// `forwarding::GRACE`.
const GRACE: Duration = Duration::from_secs(2);

/// One Verkstead: its store, its identity, and where its Peer Listener landed.
struct Verkstead {
    device: Device,
    pool: SqlitePool,
    members: Members,
    reading: Reading,
    address: SocketAddr,

    /// The workbench its own browser talks to, where it runs terminals — `None`
    /// on A, which runs none in this file.
    own: Option<Router>,

    /// Shared with the same Verkstead started again — see
    /// [`Verkstead::restarted`].
    dir: Arc<tempfile::TempDir>,
    _sandboxing: Vec<tempfile::TempDir>,
}

impl Verkstead {
    /// A Verkstead with a store and an identity, its Peer Listener up on a port
    /// the machine picked. One that `runs` terminals serves its namespace over
    /// the link with Sandboxes behind it, over the same state its own browser's
    /// workbench answers out of — which is what says, of an attach, whether it
    /// came over the link and from whom.
    async fn answering(id: &str, runs: bool) -> Verkstead {
        let dir = Arc::new(tempfile::tempdir().unwrap());
        let pool = open_database(&dir.path().join("verkstead.db"))
            .await
            .unwrap();

        let device = Device::stated(dir.path(), id).unwrap();

        Verkstead::answering_as(device, pool, dir, runs).await
    }

    /// The same Verkstead started again: its identity, its store and its Data
    /// Directory, and nothing it held in memory — no terminal, and nothing
    /// offered to anybody. On a Peer Listener of its own, which a member reaches
    /// once its [`Link`] is pointed at it.
    async fn restarted(&self) -> Verkstead {
        Verkstead::answering_as(
            self.device.clone(),
            self.pool.clone(),
            self.dir.clone(),
            self.own.is_some(),
        )
        .await
    }

    async fn answering_as(
        device: Device,
        pool: SqlitePool,
        dir: Arc<tempfile::TempDir>,
        runs: bool,
    ) -> Verkstead {
        let members = Members::recorded(pool.clone());
        let mut held = Vec::new();

        let listener = peer::Listener::bound("127.0.0.1:0".parse().unwrap(), &device)
            .expect("the loopback on a port the machine picked is free");
        let address = listener.address();

        let reading = Reading::advertising(
            no_tailscale(),
            Platform::Linux,
            None,
            vec![format!("127.0.0.1:{}", address.port())],
        );

        let (own, over_the_link) = match runs {
            true => {
                let home = tempfile::tempdir().unwrap();
                let spill = tempfile::tempdir().unwrap();
                let agents = sandboxing(dir.path(), home.path(), spill.path());

                held.push(home);
                held.push(spill);

                let routers = routers_running_sessions_answering_devices(
                    pool.clone(),
                    dir.path().to_owned(),
                    agents,
                    Gh::running(vec!["verkstead-no-such-gh".to_owned()]),
                    Devices::of(
                        device.clone(),
                        reading.clone(),
                        members.clone(),
                        Joins::none(),
                    ),
                    Nudges::new(),
                    Machine::here(),
                );

                (Some(routers.workbench), routers.over_the_link)
            }
            false => (None, Router::new()),
        };

        tokio::spawn(listener.serving(peer::router(
            device.clone(),
            reading.clone(),
            members.clone(),
            Joins::none(),
            Nudges::new(),
            over_the_link,
        )));

        Verkstead {
            device,
            pool,
            members,
            reading,
            address,
            own,
            dir,
            _sandboxing: held,
        }
    }

    /// The workbench a browser on this device talks to: its own, where it has
    /// one, and otherwise one that relays to its members and runs nothing.
    fn workbench(&self) -> Router {
        self.own.clone().unwrap_or_else(|| {
            router_answering_devices_telling(
                self.pool.clone(),
                Devices::of(
                    self.device.clone(),
                    self.reading.clone(),
                    self.members.clone(),
                    Joins::none(),
                ),
                Nudges::new(),
            )
        })
    }

    /// And that workbench served on a port of its own, which is what a socket
    /// takes.
    async fn served(&self) -> (Router, SocketAddr) {
        let app = self.workbench();

        let listener = tokio::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .await
            .unwrap();
        let at = listener.local_addr().unwrap();

        tokio::spawn({
            let app = app.clone();
            async move {
                let _ = axum::serve(listener, app).await;
            }
        });

        (app, at)
    }

    fn at(&self) -> String {
        format!("127.0.0.1:{}", self.address.port())
    }

    /// Write `other` down as one of this device's members, at `addresses`.
    async fn linked_to(&self, other: &Device, addresses: Vec<String>) {
        record_member(
            &self.pool,
            &Linking {
                device: other.id().to_owned(),
                name: "somewhere-else".to_owned(),
                os: "Linux".to_owned(),
                addresses,
                fingerprint: other.fingerprint().to_owned(),
                kept_by: None,
            },
        )
        .await
        .unwrap();
    }
}

fn no_tailscale() -> Tailscale {
    Tailscale::running(vec!["verkstead-no-such-tailscale".to_owned()], 8422)
}

/// What a device that can open a terminal is configured with —
/// `tests/bridging.rs`'s.
fn sandboxing(data_dir: &Path, home: &Path, spill: &Path) -> Agents {
    Agents::new(
        Homes::on(Platform::HERE, home.to_owned(), data_dir),
        Reachable::at("127.0.0.1:8422".parse().unwrap()),
        SandboxConfig::resolve(&[spill.display().to_string()]).unwrap(),
        BuildCache::none(),
        Skills::installed(Platform::HERE, data_dir).expect("this binary carries skills"),
        Executable::of_the_server(data_dir),
        Handoffs::under(data_dir),
        Attachments::under(data_dir),
        Settings::in_data_dir(data_dir),
    )
}

/// A and B linked both ways, with one Conversation on B in a Worktree of its
/// own and a Profile to run a terminal under.
async fn linked() -> (Verkstead, Verkstead) {
    let a = Verkstead::answering(A, false).await;
    let b = Verkstead::answering(B, true).await;

    there(&a, &b).await;

    (a, b)
}

/// B's one Conversation, in a Worktree of its own with a Profile to run a
/// terminal under, and A and B linked both ways.
async fn there(a: &Verkstead, b: &Verkstead) {
    let repo = repository(b.dir.path().join("verkstead"));
    let registered = store::register_repo(&b.pool, &repo, "verkstead", "main")
        .await
        .unwrap()
        .expect("nothing is registered at that path yet");

    let conversation = store::start_conversation(&b.pool, registered.id, BRANCH, A, None)
        .await
        .unwrap()
        .expect("the Repo was just registered");
    assert_eq!(conversation, THERE);

    paired(b, b.dir.path(), conversation).await;

    // Beside the Data Directory's own `worktrees/` rather than in it: B is
    // already up, and its startup sweep reclaims a directory there that no
    // Conversation has recorded yet — which this one is, until the grilling
    // below records it.
    let worktree = b.dir.path().join("checkouts/verkstead-listening");
    let base = git(&repo, &["rev-parse", "HEAD"]).trim().to_owned();

    git(
        &repo,
        &[
            "worktree",
            "add",
            "-b",
            BRANCH,
            &worktree.to_string_lossy(),
            &base,
        ],
    );

    store::start_grilling(&b.pool, conversation, &base, &worktree, &[])
        .await
        .unwrap();

    b.linked_to(&a.device, vec![a.at()]).await;
    a.linked_to(&b.device, vec![b.at()]).await;
}

fn repository(path: PathBuf) -> PathBuf {
    std::fs::create_dir_all(&path).unwrap();
    git(&path, &["init", "--initial-branch", "main"]);
    git(&path, &["config", "user.email", "tests@verkstead.invalid"]);
    git(&path, &["config", "user.name", "Verkstead Tests"]);
    git(&path, &["config", "commit.gpgsign", "false"]);
    std::fs::write(path.join("README.md"), "# a repository\n").unwrap();
    git(&path, &["add", "-A"]);
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

/// An Agent Profile on `on`, paired with its Conversation — what a terminal is
/// run under. `tests/bridging.rs`'s.
async fn paired(on: &Verkstead, root: &Path, conversation: i64) {
    let claude_dir = root.join("account/.claude");
    let config_file = root.join("account/.claude.json");

    std::fs::create_dir_all(&claude_dir).unwrap();
    std::fs::write(&config_file, "{}\n").unwrap();

    let profile = store::create_profile(
        &on.pool,
        &store::ProfileFacts {
            name: Some("tests".to_owned()),
            account: store::Account::Claude {
                claude_dir,
                config_file,
            },
            models: vec!["claude-tests-5".to_owned()],
            memory: false,
        },
    )
    .await
    .unwrap()
    .expect("nothing else is saved under that name");

    assert_eq!(
        store::set_grilling_pairing(&on.pool, conversation, profile.id, None)
            .await
            .unwrap(),
        store::Chosen::Chosen,
    );
}

/// Where a call stands when it is for `device` instead.
fn through(device: &str, path: &str) -> String {
    let leaf = path
        .strip_prefix("/api/ui")
        .expect("a relayed call is a call into the viewer's own namespace");

    format!("/api/ui/members/{device}{leaf}")
}

/// The reading of B's ports, which is the same path whoever asks it.
fn ports_path() -> String {
    format!("/api/ui/conversations/{THERE}/ports")
}

/// A port nothing on this machine is listening on, this moment.
fn free_port() -> u16 {
    TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

/// What a server started in a terminal is typed as: node, listening on `port`
/// on the loopback and saying so once it is.
fn serving(port: u16) -> String {
    format!(
        "node -e \"require('net').createServer().listen({port}, '127.0.0.1', \
         () => console.log('listening-' + {port}))\"\n"
    )
}

/// And the same server saying where it is, the way a dev server does: its
/// address on this machine, with the port in it.
///
/// Put together as it is printed, so that the line typed — which the terminal
/// echoes — holds no address of its own.
fn announcing(port: u16) -> String {
    format!(
        "node -e \"require('net').createServer().listen({port}, '127.0.0.1', \
         () => console.log('listening-' + {port} + ' at http://localhost:' + {port} + '/'))\"\n"
    )
}

/// A call that carries no body, answered `200` with JSON.
async fn call<T: DeserializeOwned>(app: &Router, method: &str, path: &str) -> T {
    let (status, said) = asked(app, method, path).await;

    assert_eq!(status, StatusCode::OK, "{method} {path}: {said}");
    serde_json::from_str(&said)
        .unwrap_or_else(|why| panic!("{method} {path} answered {said}: {why}"))
}

/// And the same, whatever the status.
async fn asked(app: &Router, method: &str, path: &str) -> (StatusCode, String) {
    let answered = app
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let status = answered.status();
    let said = answered.into_body().collect().await.unwrap().to_bytes();

    (status, String::from_utf8_lossy(&said).into_owned())
}

/// Open a terminal on B's Conversation through `app`, and hand back its number.
async fn opened(app: &Router, path: &str) -> i64 {
    match call(app, "POST", path).await {
        TerminalOpened::Opened { number } => number,
        opened => panic!("a terminal should have opened, and it said: {opened:?}"),
    }
}

/// Read `path` until what it says satisfies `enough`, and say how long that
/// took — or give up, saying what it last said.
async fn until(app: &Router, path: &str, enough: impl Fn(&[TerminalPorts]) -> bool) -> Duration {
    let started = Instant::now();

    loop {
        let read: PortsView = call(app, "GET", path).await;

        if enough(&read.terminals) {
            return started.elapsed();
        }

        assert!(
            started.elapsed() < PATIENCE,
            "the reading never came round, and it last said: {:?}",
            read.terminals,
        );

        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

/// Whether `port` is in a reading.
fn holds(read: &[TerminalPorts], port: u16) -> bool {
    read.iter().any(|terminal| terminal.ports.contains(&port))
}

/// A terminal as a pane holds one: the socket, and what has been printed down it.
struct Terminal {
    socket: tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
    printed: String,
}

impl Terminal {
    /// Attach at `url`, and take the repaint it opens with.
    async fn attached(url: String) -> Terminal {
        let (socket, _) = tokio_tungstenite::connect_async(&url)
            .await
            .unwrap_or_else(|error| panic!("{url} to be attachable: {error}"));

        let mut terminal = Terminal {
            socket,
            printed: String::new(),
        };

        match terminal.shown().await {
            Shown::Painted(_) => terminal,
            shown => panic!("a socket should open with a repaint, and it said: {shown:?}"),
        }
    }

    async fn shown(&mut self) -> Shown {
        let said = tokio::time::timeout(PATIENCE, self.socket.next())
            .await
            .expect("the terminal should have said something")
            .expect("the socket should still be open")
            .expect("the socket should be readable");

        match said {
            Message::Text(said) => serde_json::from_str(&said).unwrap(),
            said => panic!("a Screen's socket speaks JSON, and it said: {said:?}"),
        }
    }

    async fn typed(&mut self, keystrokes: &str) {
        let putting = serde_json::to_string(&Watching::PutIn(keystrokes.to_owned())).unwrap();

        self.socket
            .send(Message::Text(putting.into()))
            .await
            .unwrap();
    }

    /// Follow it until `wanted` has been printed — after whatever had already
    /// been printed when this was asked.
    async fn until(&mut self, wanted: &str) {
        let deadline = Instant::now() + PATIENCE;
        let from = self.printed.len();

        while !self.printed[from..].contains(wanted) {
            assert!(
                Instant::now() < deadline,
                "the terminal never printed {wanted:?}. It has printed: {:?}",
                self.printed,
            );

            if let Shown::Printed(printed) = self.shown().await {
                self.printed.push_str(&printed);
            }
        }
    }

    async fn closed(mut self) {
        let _ = self.socket.close(None).await;
    }
}

/// An open page's Nudge stream — `tests/bridging.rs`'s reading of one.
struct Listening {
    body: Body,
    buffered: String,
}

impl Listening {
    async fn open(app: &Router, path: &str) -> Listening {
        let response = app
            .clone()
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get(header::CONTENT_TYPE)
                .and_then(|kind| kind.to_str().ok()),
            Some("text/event-stream"),
        );

        Listening {
            body: response.into_body(),
            buffered: String::new(),
        }
    }

    async fn frame(&mut self) -> String {
        loop {
            if let Some(end) = self.buffered.find("\n\n") {
                let frame = self.buffered[..end].to_owned();
                self.buffered = self.buffered[end + 2..].to_owned();
                return frame;
            }

            let chunk = self
                .body
                .frame()
                .await
                .expect("the stream should still be open")
                .unwrap();

            self.buffered
                .push_str(std::str::from_utf8(&chunk.into_data().unwrap()).unwrap());
        }
    }

    /// Every Nudge that lands in `over`, keep-alives left out.
    async fn heard(&mut self, over: Duration) -> Vec<Nudge> {
        let mut heard = Vec::new();
        let until = Instant::now() + over;

        loop {
            let left = until.saturating_duration_since(Instant::now());

            let Ok(frame) = tokio::time::timeout(left, self.frame()).await else {
                return heard;
            };

            if let Some(data) = frame.lines().find_map(|line| line.strip_prefix("data: ")) {
                heard.push(serde_json::from_str(data).unwrap());
            }
        }
    }
}

const PORTS: Nudge = Nudge::Ports {
    conversation: THERE,
};

/// A server started in a terminal attached from another device is in that
/// device's reading within a turn of it listening, and out of it within a turn
/// of it stopping — with a `ports` Nudge on B's stream each time.
#[tokio::test]
async fn a_server_in_a_remote_terminal_is_in_the_attaching_devices_reading_until_it_ends() {
    let (a, _b) = linked().await;
    let (app, at) = a.served().await;

    let mut page = Listening::open(&app, &through(B, "/api/ui/nudges")).await;

    let number = opened(
        &app,
        &through(B, &format!("/api/ui/conversations/{THERE}/terminals")),
    )
    .await;

    let mut terminal = Terminal::attached(format!(
        "ws://{at}{}",
        through(
            B,
            &format!("/api/ui/conversations/{THERE}/terminals/{number}/attach")
        )
    ))
    .await;

    // Attached and listening on nothing yet: the terminal is A's, with no ports.
    until(&app, &through(B, &ports_path()), |read| {
        read == [TerminalPorts {
            number,
            ports: Vec::new(),
        }]
    })
    .await;

    assert!(
        page.heard(Duration::from_millis(500))
            .await
            .contains(&PORTS),
        "A attaching should have been a ports Nudge on B's stream",
    );

    let port = free_port();
    terminal.typed(&serving(port)).await;
    terminal.until(&format!("listening-{port}")).await;

    let took = until(&app, &through(B, &ports_path()), |read| holds(read, port)).await;
    assert!(
        took < PROMPTLY,
        "port {port} took {took:?} to reach the reading"
    );

    assert!(
        page.heard(Duration::from_millis(500))
            .await
            .contains(&PORTS),
        "a port opening should have been a ports Nudge on B's stream",
    );

    // ^C, which is the server stopping and the shell coming back.
    terminal.typed("\u{3}").await;

    let took = until(&app, &through(B, &ports_path()), |read| !holds(read, port)).await;
    assert!(
        took < PROMPTLY,
        "port {port} took {took:?} to leave the reading"
    );

    assert!(
        page.heard(Duration::from_millis(500))
            .await
            .contains(&PORTS),
        "a port closing should have been a ports Nudge on B's stream",
    );

    // And A letting go keeps the terminal in A's reading: it is offered to A
    // until it ends, not for as long as a tab is open on it.
    terminal.closed().await;

    assert!(
        page.heard(Duration::from_millis(500))
            .await
            .contains(&PORTS),
        "A letting go should have been a ports Nudge on B's stream",
    );

    let read: PortsView = call(&app, "GET", &through(B, &ports_path())).await;
    assert_eq!(
        read.terminals,
        vec![TerminalPorts {
            number,
            ports: Vec::new(),
        }],
    );
}

/// A terminal attached only from B's own browser is not read: a server in it is
/// in nobody's reading, no `ports` Nudge is announced for it, and the reading is
/// not served to that browser at all.
#[tokio::test]
async fn a_terminal_attached_only_from_its_own_device_is_not_read() {
    let (a, b) = linked().await;
    let (relaying, _) = a.served().await;
    let (own, at) = b.served().await;

    let number = opened(&own, &format!("/api/ui/conversations/{THERE}/terminals")).await;

    let mut terminal = Terminal::attached(format!(
        "ws://{at}/api/ui/conversations/{THERE}/terminals/{number}/attach"
    ))
    .await;

    let mut page = Listening::open(&own, "/api/ui/nudges").await;

    // Saying where it is, which is no hurry to a read that is not running.
    let port = free_port();
    terminal.typed(&announcing(port)).await;
    terminal.until(&format!("listening-{port}")).await;

    assert!(
        !page.heard(SETTLING).await.contains(&PORTS),
        "a terminal no member is attached to should not have been read",
    );

    // A holds no attach on it, so A reads nothing of it.
    let read: PortsView = call(&relaying, "GET", &through(B, &ports_path())).await;
    assert_eq!(read.terminals, Vec::new());

    // Nor anything in the reading of everything B offers A.
    let offered: OfferedView = call(&relaying, "GET", &through(B, "/api/ui/ports")).await;
    assert_eq!(offered.conversations, Vec::new());

    // And B's own browser is not served either reading at all, and is told why.
    let (status, _) = asked(&own, "GET", &ports_path()).await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let (status, said) = asked(&own, "GET", "/api/ui/ports").await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{said}");
    assert!(said.contains("not by a browser on this machine"), "{said}");

    terminal.closed().await;
}

/// Attach to B's terminal from A, and wait until A's reading has it — the read
/// running, with nothing in it yet.
async fn attached_from_a(app: &Router, at: SocketAddr) -> (i64, Terminal) {
    let number = opened(
        app,
        &through(B, &format!("/api/ui/conversations/{THERE}/terminals")),
    )
    .await;

    let terminal = Terminal::attached(format!(
        "ws://{at}{}",
        through(
            B,
            &format!("/api/ui/conversations/{THERE}/terminals/{number}/attach")
        )
    ))
    .await;

    until(app, &through(B, &ports_path()), |read| {
        read == [TerminalPorts {
            number,
            ports: Vec::new(),
        }]
    })
    .await;

    (number, terminal)
}

/// A server that prints its address on starting is in the reading well before
/// the turn after the one that last read the terminal.
///
/// Timed from a turn: one node holds a first port, which is in the reading the
/// moment a turn has read it, and then — on a line typed to it — opens a second
/// and prints that one's address. The next turn is the better part of
/// two seconds away when that line is typed, so the second port in the reading
/// well inside it is the print having hurried it.
#[tokio::test]
async fn a_server_printing_its_address_is_read_before_the_next_turn() {
    let (a, _b) = linked().await;
    let (app, at) = a.served().await;

    let (_, mut terminal) = attached_from_a(&app, at).await;

    let (first, second) = (free_port(), free_port());
    terminal
        .typed(&format!(
            "node -e \"const net = require('net'); \
             net.createServer().listen({first}, '127.0.0.1', () => console.log('holding-' + {first})); \
             process.stdin.once('data', () => net.createServer().listen({second}, '127.0.0.1', \
             () => console.log('up at http://localhost:' + {second} + '/')))\"\n"
        ))
        .await;
    terminal.until(&format!("holding-{first}")).await;

    // A turn has just read the first port.
    until(&app, &through(B, &ports_path()), |read| holds(read, first)).await;

    terminal.typed("go\n").await;

    let took = until(&app, &through(B, &ports_path()), |read| holds(read, second)).await;
    assert!(
        took < HURRIED,
        "port {second} took {took:?} to reach the reading after a turn, which is a turn \
         of its own rather than the print hurrying it",
    );

    terminal.typed("\u{3}").await;
    terminal.closed().await;
}

/// A port printed that nothing in the terminal is listening on is not in the
/// reading, however it was printed: the read it hurried finds nothing, so
/// nothing moves and nothing is said.
#[tokio::test]
async fn a_printed_port_nothing_listens_on_is_not_read() {
    let (a, _b) = linked().await;
    let (app, at) = a.served().await;

    let mut page = Listening::open(&app, &through(B, "/api/ui/nudges")).await;
    let (_, mut terminal) = attached_from_a(&app, at).await;

    // A's attach, said and done with.
    page.heard(Duration::from_millis(500)).await;

    let port = free_port();
    terminal
        .typed(&format!("echo http://localhost:{port}/ printed-{port}\n"))
        .await;
    terminal.until(&format!("printed-{port}")).await;

    assert!(
        !page.heard(SETTLING).await.contains(&PORTS),
        "a port nothing listens on should not have moved the reading",
    );

    let read: PortsView = call(&app, "GET", &through(B, &ports_path())).await;
    assert!(
        !holds(&read.terminals, port),
        "port {port} was only printed, and it is in the reading: {:?}",
        read.terminals,
    );

    terminal.closed().await;
}

/// The walk itself, against a listener inside a nested bwrap that has taken a
/// session of its own: a pid namespace the reader has to find from the wrapper
/// it started as, and a server outside the shell's own tree inside it.
#[tokio::test]
async fn a_listener_inside_a_nested_bwrap_is_read_from_outside_it() {
    let port = free_port();

    let mut sandbox = Command::new("bwrap")
        .args([
            "--die-with-parent",
            "--unshare-all",
            "--share-net",
            "--dev-bind",
            "/",
            "/",
            "--proc",
            "/proc",
            "sh",
            "-c",
        ])
        .arg(format!(
            "setsid node -e \"require('net').createServer().listen({port}, '127.0.0.1')\" & wait"
        ))
        .stdin(Stdio::null())
        .spawn()
        .expect("bwrap should be on the PATH for these tests");

    let leader = sandbox.id();
    let deadline = Instant::now() + PATIENCE;

    while !ports::listening(leader).contains(&port) {
        assert!(
            Instant::now() < deadline,
            "port {port} inside the sandbox was never read from outside it",
        );

        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    let _ = sandbox.kill();
    let _ = sandbox.wait();
}

/// Where one of a terminal's ports is connected to.
fn port_path(number: i64, port: u16) -> String {
    format!("/api/ui/conversations/{THERE}/terminals/{number}/ports/{port}")
}

/// What a server started in a terminal is typed as when it is to be talked to:
/// node on `host`, echoing what it is sent behind `echo:`, ending the
/// connection itself — with `ended` — on anything starting `end`, and exiting
/// on anything starting `quit`, which is the server stopping with no tab open
/// to stop it from.
fn echoing(host: &str, port: u16) -> String {
    format!(
        "node -e \"require('net').createServer(s => s.on('data', d => \
         String(d).startsWith('quit') ? process.exit() : \
         String(d).startsWith('end') ? s.end('ended') : s.write('echo:' + d)))\
         .listen({port}, '{host}', () => console.log('listening-' + {port}))\"\n"
    )
}

/// Upgrade a connection to `at` for `path`, the way a forwarding device does:
/// the socket where it switched protocols, or the status and what was said
/// where it did not.
async fn upgraded(at: SocketAddr, path: &str) -> Result<tokio::net::TcpStream, (u16, String)> {
    let mut stream = tokio::net::TcpStream::connect(at).await.unwrap();

    stream
        .write_all(
            format!(
                "GET {path} HTTP/1.1\r\nHost: {at}\r\nConnection: Upgrade\r\n\
                 Upgrade: verkstead-forward\r\n\r\n"
            )
            .as_bytes(),
        )
        .await
        .unwrap();

    // Read a byte at a time up to the end of the head, so that nothing past it —
    // the first bytes of what crosses — is taken off the socket here.
    let mut head = Vec::new();

    while !head.ends_with(b"\r\n\r\n") {
        let mut byte = [0u8; 1];
        let read = tokio::time::timeout(PATIENCE, stream.read(&mut byte))
            .await
            .expect("the upgrade should have been answered")
            .unwrap();

        assert_eq!(read, 1, "the connection ended mid-answer: {head:?}");
        head.push(byte[0]);
    }

    let head = String::from_utf8(head).unwrap();
    let status: u16 = head
        .split_whitespace()
        .nth(1)
        .and_then(|status| status.parse().ok())
        .unwrap_or_else(|| panic!("an answer starts with its status, and it said {head}"));

    if status == 101 {
        return Ok(stream);
    }

    let length: usize = head
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse().ok())?
        })
        .unwrap_or(0);

    let mut said = vec![0u8; length];
    stream.read_exact(&mut said).await.unwrap();

    Err((status, String::from_utf8_lossy(&said).into_owned()))
}

/// Send `sending` and read until `wanted` has come back — or the connection
/// ended, which fails.
async fn exchanged(stream: &mut tokio::net::TcpStream, sending: &str, wanted: &str) {
    stream.write_all(sending.as_bytes()).await.unwrap();

    let mut heard = Vec::new();

    while !String::from_utf8_lossy(&heard).contains(wanted) {
        let mut chunk = [0u8; 256];
        let read = tokio::time::timeout(PATIENCE, stream.read(&mut chunk))
            .await
            .unwrap_or_else(|_| panic!("{wanted:?} never came back, only {heard:?}"))
            .unwrap();

        assert!(read > 0, "the connection ended before {wanted:?} came back");
        heard.extend_from_slice(&chunk[..read]);
    }
}

/// Whether the connection reaches its end — a read of nothing — within the
/// patience, whatever was still to read before it.
async fn ends(stream: &mut tokio::net::TcpStream) -> bool {
    let mut rest = Vec::new();

    matches!(
        tokio::time::timeout(PROMPTLY, stream.read_to_end(&mut rest)).await,
        Ok(Ok(_)) | Ok(Err(_)),
    )
}

/// A connection to a terminal's port, upgraded over the link from the device
/// attached to it, reaches the server listening in the terminal and brings its
/// answers back; the server ending the connection ends it at the caller, and
/// the caller ending it ends it at the server.
#[tokio::test]
async fn a_connection_upgraded_over_the_link_reaches_the_server_in_the_terminal() {
    let (a, _b) = linked().await;
    let (app, at) = a.served().await;

    let number = opened(
        &app,
        &through(B, &format!("/api/ui/conversations/{THERE}/terminals")),
    )
    .await;

    let mut terminal = Terminal::attached(format!(
        "ws://{at}{}",
        through(
            B,
            &format!("/api/ui/conversations/{THERE}/terminals/{number}/attach")
        )
    ))
    .await;

    let port = free_port();
    terminal.typed(&echoing("127.0.0.1", port)).await;
    terminal.until(&format!("listening-{port}")).await;

    until(&app, &through(B, &ports_path()), |read| holds(read, port)).await;

    // The server ending it.
    let mut stream = upgraded(at, &through(B, &port_path(number, port)))
        .await
        .unwrap_or_else(|refused| panic!("the port should have been joined: {refused:?}"));

    exchanged(&mut stream, "hello", "echo:hello").await;
    exchanged(&mut stream, "end", "ended").await;

    assert!(
        ends(&mut stream).await,
        "the server ending its connection should have ended the caller's",
    );

    // And the caller ending it: the server, which ends its side when the other
    // does, is what ends what is left.
    let mut stream = upgraded(at, &through(B, &port_path(number, port)))
        .await
        .unwrap_or_else(|refused| panic!("the port should have been joined again: {refused:?}"));

    exchanged(&mut stream, "again", "echo:again").await;
    stream.shutdown().await.unwrap();

    assert!(
        ends(&mut stream).await,
        "the caller ending its connection should have reached the server",
    );

    terminal.closed().await;
}

/// Nothing is joined that the caller may not have: a port not in the reading, a
/// terminal it never attached, and the device's own browser are each refused
/// by name — and a port in the reading that the loopback does not answer is
/// refused rather than answered with a socket carrying nothing.
#[tokio::test]
async fn a_connection_is_refused_unless_the_reading_offers_it_to_the_caller() {
    let (a, b) = linked().await;
    let (app, at) = a.served().await;
    let (_, own) = b.served().await;

    let terminals = through(B, &format!("/api/ui/conversations/{THERE}/terminals"));
    let number = opened(&app, &terminals).await;
    let unattached = opened(&app, &terminals).await;

    let mut terminal = Terminal::attached(format!(
        "ws://{at}{}",
        through(
            B,
            &format!("/api/ui/conversations/{THERE}/terminals/{number}/attach")
        )
    ))
    .await;

    // On 127.0.0.2, which is in the reading — a listener on any address is — and
    // which neither loopback the member dials answers on: the same answer a
    // server gone between the read and the dial gets, without racing the read.
    let elsewhere = free_port();
    terminal.typed(&echoing("127.0.0.2", elsewhere)).await;
    terminal.until(&format!("listening-{elsewhere}")).await;

    until(&app, &through(B, &ports_path()), |read| {
        holds(read, elsewhere)
    })
    .await;

    let (status, said) = upgraded(at, &through(B, &port_path(number, elsewhere)))
        .await
        .expect_err("a port nothing answers on should have been refused");
    assert_eq!(status, 502, "{said}");
    assert!(said.contains("Nothing answered"), "{said}");

    // A port the terminal never opened.
    let never = free_port();
    let (status, said) = upgraded(at, &through(B, &port_path(number, never)))
        .await
        .expect_err("a port not in the reading should have been refused");
    assert_eq!(status, 404, "{said}");
    assert!(said.contains("isn't listening"), "{said}");

    // A terminal A never attached, for a port that is listening in another —
    // which is not in A's reading either.
    let read: PortsView = call(&app, "GET", &through(B, &ports_path())).await;
    assert!(
        read.terminals
            .iter()
            .all(|terminal| terminal.number != unattached),
        "a terminal A never attached should not be in A's reading: {read:?}",
    );

    let (status, said) = upgraded(at, &through(B, &port_path(unattached, elsewhere)))
        .await
        .expect_err("a terminal the caller never attached should have been refused");
    assert_eq!(status, 403, "{said}");
    assert!(said.contains("has attached it"), "{said}");

    // And B's own browser, for the very port A may have.
    let (status, said) = upgraded(own, &port_path(number, elsewhere))
        .await
        .expect_err("the device's own browser should have been refused");
    assert_eq!(status, 403, "{said}");
    assert!(said.contains("over the link"), "{said}");

    terminal.closed().await;
}

/// Where A forwards in the tests that follow: an IPv4 loopback address B's own
/// server is not on, and IPv6's one.
///
/// **Not `127.0.0.1`**, because A and B are one machine here and a Sandbox
/// shares its network: the server in B's terminal is on `127.0.0.1`, so that
/// port is busy at A's `127.0.0.1` by construction — which is a Forward
/// skipped, rather than the one these tests are about. Every other address in
/// `127.0.0.0/8` is the loopback too.
fn forwarding_on() -> Vec<std::net::IpAddr> {
    vec![
        std::net::IpAddr::V4(Ipv4Addr::new(127, 0, 0, 2)),
        std::net::IpAddr::V6(std::net::Ipv6Addr::LOCALHOST),
    ]
}

/// The address A's IPv4 Forward is reached at.
fn forwarded_v4(port: u16) -> SocketAddr {
    SocketAddr::new(forwarding_on()[0], port)
}

/// And its IPv6 one.
fn forwarded_v6(port: u16) -> SocketAddr {
    SocketAddr::new(forwarding_on()[1], port)
}

/// What a forwarding A holds its members' Nudge streams with, handed back to
/// be started once the members are written down.
struct Staying {
    cluster: Devices,
    nudges: Nudges,
}

/// The tasks a forwarding A holds, aborted as the test lets go of it.
struct Holding(Vec<tokio::task::JoinHandle<()>>);

impl Drop for Holding {
    fn drop(&mut self) {
        for held in &self.0 {
            held.abort();
        }
    }
}

impl Verkstead {
    /// A Verkstead that **forwards**: its own workbench and the namespace it
    /// serves over the link over one state, the way a served one has them, with
    /// its Forwards held on `on` — and holding a Nudge stream to each of its
    /// members, which is what a running server spawns at the start and what sets
    /// a Forward going.
    async fn forwarding(id: &str, on: Vec<std::net::IpAddr>) -> (Verkstead, Holding, Staying) {
        let dir = Arc::new(tempfile::tempdir().unwrap());
        let pool = open_database(&dir.path().join("verkstead.db"))
            .await
            .unwrap();

        let device = Device::stated(dir.path(), id).unwrap();

        Verkstead::forwarding_as(device, pool, dir, on).await
    }

    /// The same forwarding Verkstead served afresh, holding its Forwards on
    /// `on`: its identity and its store, and nothing it held in memory — no
    /// Forward, and no Nudge stream until the one handed back is started.
    async fn forwarding_again(&self, on: Vec<std::net::IpAddr>) -> (Verkstead, Holding, Staying) {
        Verkstead::forwarding_as(self.device.clone(), self.pool.clone(), self.dir.clone(), on).await
    }

    async fn forwarding_as(
        device: Device,
        pool: SqlitePool,
        dir: Arc<tempfile::TempDir>,
        on: Vec<std::net::IpAddr>,
    ) -> (Verkstead, Holding, Staying) {
        let members = Members::recorded(pool.clone());

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
        .forwarding_on(on);

        let nudges = Nudges::new();

        let routers = verkstead_server::routers_answering_devices_telling(
            pool.clone(),
            cluster.clone(),
            nudges.clone(),
            Machine::here(),
        );

        let peer = peer::router(
            device.clone(),
            reading.clone(),
            members.clone(),
            Joins::none(),
            Nudges::new(),
            routers.over_the_link,
        );

        let serving = tokio::spawn(async move {
            let _ = listener.serving(peer).await;
        });

        (
            Verkstead {
                device,
                pool,
                members,
                reading,
                address,
                own: Some(routers.workbench),
                dir,
                _sandboxing: Vec::new(),
            },
            Holding(vec![serving]),
            Staying { cluster, nudges },
        )
    }
}

/// A and B linked both ways as [`linked`] makes them, with A forwarding on
/// [`forwarding_on`].
async fn linked_forwarding() -> (Verkstead, Verkstead, Holding) {
    let (a, mut holding, staying) = Verkstead::forwarding(A, forwarding_on()).await;
    let b = Verkstead::answering(B, true).await;

    there(&a, &b).await;

    // After the linking rather than before it: the streams are taken up against
    // the membership as it is first read, and it is read again only every ten
    // seconds.
    holding.0.push(staying.started());

    (a, b, holding)
}

/// And the same with A reaching B over a [`Link`] the test can cut.
async fn linked_forwarding_over_a_link() -> (Verkstead, Verkstead, Holding, Link) {
    let (a, mut holding, staying) = Verkstead::forwarding(A, forwarding_on()).await;
    let b = Verkstead::answering(B, true).await;

    there(&a, &b).await;

    let link = Link::to(b.address).await;
    a.linked_to(&b.device, vec![link.at.to_string()]).await;

    holding.0.push(staying.started());

    (a, b, holding, link)
}

impl Staying {
    /// The members' Nudge streams taken up, for as long as what comes back is
    /// held.
    fn started(self) -> tokio::task::JoinHandle<()> {
        let Staying { cluster, nudges } = self;

        tokio::spawn(async move { cluster.stay_fresh(nudges).await })
    }
}

/// What A reaches B's Peer Listener over: a listener of the test's own that
/// joins every connection made to it to B, byte for byte — so that the test
/// can **cut** it, every connection crossing it dropped at once the way a
/// link dropping drops them, and point it at B started again.
struct Link {
    at: SocketAddr,
    to: Arc<Mutex<SocketAddr>>,
    joined: Arc<Mutex<Vec<tokio::task::AbortHandle>>>,
    _accepting: Holding,
}

impl Link {
    async fn to(b: SocketAddr) -> Link {
        let listener = tokio::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .await
            .unwrap();
        let at = listener.local_addr().unwrap();

        let to = Arc::new(Mutex::new(b));
        let joined = Arc::new(Mutex::new(Vec::new()));

        let accepting = tokio::spawn({
            let to = to.clone();
            let joined = joined.clone();

            async move {
                while let Ok((mut near, _)) = listener.accept().await {
                    let to = *to.lock().unwrap();

                    let join = tokio::spawn(async move {
                        if let Ok(mut far) = tokio::net::TcpStream::connect(to).await {
                            let _ = tokio::io::copy_bidirectional(&mut near, &mut far).await;
                        }
                    });

                    joined.lock().unwrap().push(join.abort_handle());
                }
            }
        });

        Link {
            at,
            to,
            joined,
            _accepting: Holding(vec![accepting]),
        }
    }

    /// Every connection crossing it dropped. The next one made is joined as
    /// before.
    fn cut(&self) {
        for join in self.joined.lock().unwrap().drain(..) {
            join.abort();
        }
    }

    /// And the connections made from now on joined to `b` instead.
    fn pointed_at(&self, b: SocketAddr) {
        *self.to.lock().unwrap() = b;
    }
}

/// What A is forwarding, read until it satisfies `enough` — or given up on,
/// saying what it last read.
async fn forwarded_until(app: &Router, enough: impl Fn(&[ForwardView]) -> bool) -> Duration {
    let started = Instant::now();

    loop {
        let read: ForwardsView = call(app, "GET", "/api/ui/forwards").await;

        if enough(&read.forwards) {
            return started.elapsed();
        }

        assert!(
            started.elapsed() < PATIENCE,
            "what is forwarded never came round, and it last said: {:?}",
            read.forwards,
        );

        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

/// How `port` stands in what A is forwarding, where it is there at all.
fn standing(read: &[ForwardView], port: u16) -> Option<ForwardStanding> {
    read.iter()
        .find(|forward| forward.port == port)
        .map(|forward| forward.standing)
}

/// Whether a connection to `at` reaches the echoing server in B's terminal and
/// brings its answer back.
async fn echoes(at: SocketAddr, saying: &str) {
    let mut stream = tokio::time::timeout(PATIENCE, tokio::net::TcpStream::connect(at))
        .await
        .unwrap_or_else(|_| panic!("{at} never answered"))
        .unwrap_or_else(|why| panic!("{at} should have been forwarded: {why}"));

    exchanged(&mut stream, saying, &format!("echo:{saying}")).await;
}

/// The Nudges a page on this device hears, with whose they are: a member's news
/// comes down the same stream under that member's Device Id.
async fn heard_nudged(page: &mut Listening, over: Duration) -> Vec<Nudged> {
    let mut heard = Vec::new();
    let until = Instant::now() + over;

    loop {
        let left = until.saturating_duration_since(Instant::now());

        let Ok(frame) = tokio::time::timeout(left, page.frame()).await else {
            return heard;
        };

        if let Some(data) = frame.lines().find_map(|line| line.strip_prefix("data: ")) {
            heard.push(serde_json::from_str(data).unwrap());
        }
    }
}

/// A server started in a terminal on B, attached from A's browser, is reachable
/// at A's own loopback on the same port — IPv4 and IPv6 both — within a few
/// seconds; a pane swap keeps it, and so does closing the tab, for as long as
/// the server listens. The server stopping ends it, within the grace and a
/// turn.
#[tokio::test]
async fn a_server_in_a_remote_terminal_is_forwarded_to_the_attaching_devices_localhost() {
    let (a, _b, _holding) = linked_forwarding().await;
    let (app, at) = a.served().await;

    let mut page = Listening::open(&app, "/api/ui/nudges").await;

    let number = opened(
        &app,
        &through(B, &format!("/api/ui/conversations/{THERE}/terminals")),
    )
    .await;

    let attach = format!(
        "ws://{at}{}",
        through(
            B,
            &format!("/api/ui/conversations/{THERE}/terminals/{number}/attach")
        )
    );

    let mut terminal = Terminal::attached(attach.clone()).await;

    let port = free_port();
    terminal.typed(&echoing("127.0.0.1", port)).await;
    terminal.until(&format!("listening-{port}")).await;

    let took = forwarded_until(&app, |read| {
        standing(read, port) == Some(ForwardStanding::Forwarding)
    })
    .await;
    assert!(took < PROMPTLY, "port {port} took {took:?} to be forwarded");

    echoes(forwarded_v4(port), "over-ipv4").await;
    echoes(forwarded_v6(port), "over-ipv6").await;

    // The reading says what it is: the port, the device and its OS, the
    // Conversation and the terminal.
    let read: ForwardsView = call(&app, "GET", "/api/ui/forwards").await;
    assert_eq!(
        read.forwards,
        vec![ForwardView {
            port,
            device: B.to_owned(),
            name: "somewhere-else".to_owned(),
            os: "Linux".to_owned(),
            conversation: THERE,
            title: read.forwards[0].title.clone(),
            terminal: number,
            standing: ForwardStanding::Forwarding,
        }],
    );

    assert!(
        heard_nudged(&mut page, Duration::from_millis(500))
            .await
            .contains(&Nudged::here(Nudge::Forwards)),
        "a Forward taken up should have been a forwards Nudge on A's own stream",
    );

    // A pane swap: the attach let go of and taken again at once. The Forward
    // stands across it and past the grace.
    terminal.closed().await;
    let terminal = Terminal::attached(attach).await;

    tokio::time::sleep(SETTLING).await;

    let read: ForwardsView = call(&app, "GET", "/api/ui/forwards").await;
    assert_eq!(
        standing(&read.forwards, port),
        Some(ForwardStanding::Forwarding),
        "a pane swap should have kept the Forward",
    );
    echoes(forwarded_v4(port), "after-the-swap").await;

    // And the tab closed: the Forward is held, not attached, so it stands past
    // the grace with nothing attached at all.
    terminal.closed().await;

    tokio::time::sleep(SETTLING).await;

    let read: ForwardsView = call(&app, "GET", "/api/ui/forwards").await;
    assert_eq!(
        standing(&read.forwards, port),
        Some(ForwardStanding::Forwarding),
        "closing the tab should have kept the Forward",
    );
    echoes(forwarded_v4(port), "after-the-tab-closed").await;
    echoes(forwarded_v6(port), "still-after-the-tab-closed").await;

    // The server stopping, with no tab open: out of the reading on the next
    // unwatched turn, and the Forward gone the grace after that.
    let mut quitting = tokio::net::TcpStream::connect(forwarded_v4(port))
        .await
        .unwrap();
    let stopped = Instant::now();
    quitting.write_all(b"quit").await.unwrap();

    forwarded_until(&app, <[ForwardView]>::is_empty).await;
    let took = stopped.elapsed();
    assert!(
        took < UNWATCHED + Duration::from_secs(2) + PROMPTLY,
        "the Forward ended {took:?} after its server stopped",
    );

    assert!(
        tokio::net::TcpStream::connect(forwarded_v4(port))
            .await
            .is_err(),
        "nothing should be listening on port {port} here once the Forward has ended",
    );
}

/// Closing a terminal no tab is open on ends its Forward within the grace and
/// a little — not a whole unwatched turn later — and nothing listens at A's
/// loopback on that port afterwards.
#[tokio::test]
async fn closing_the_terminal_ends_its_forward() {
    let (a, _b, _holding) = linked_forwarding().await;
    let (app, at) = a.served().await;

    let number = opened(
        &app,
        &through(B, &format!("/api/ui/conversations/{THERE}/terminals")),
    )
    .await;

    let mut terminal = Terminal::attached(format!(
        "ws://{at}{}",
        through(
            B,
            &format!("/api/ui/conversations/{THERE}/terminals/{number}/attach")
        )
    ))
    .await;

    let port = free_port();
    terminal.typed(&echoing("127.0.0.1", port)).await;
    terminal.until(&format!("listening-{port}")).await;

    forwarded_until(&app, |read| {
        standing(read, port) == Some(ForwardStanding::Forwarding)
    })
    .await;

    terminal.closed().await;
    tokio::time::sleep(SETTLING).await;
    echoes(forwarded_v4(port), "with-no-tab-open").await;

    // The ×, asked and said yes to: a server is running in it. Answered once
    // the shell has gone, which is when the terminal has ended.
    let (status, said) = asked(
        &app,
        "DELETE",
        &through(
            B,
            &format!("/api/ui/conversations/{THERE}/terminals/{number}?asked=true"),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{said}");
    let closed = Instant::now();

    forwarded_until(&app, <[ForwardView]>::is_empty).await;
    let took = closed.elapsed();
    assert!(
        took < Duration::from_secs(2) + PROMPTLY,
        "the Forward ended {took:?} after its terminal closed",
    );

    assert!(
        tokio::net::TcpStream::connect(forwarded_v4(port))
            .await
            .is_err(),
        "nothing should be listening on port {port} here once the Forward has ended",
    );
}

/// With no tab attached a terminal is read every ten seconds rather than every
/// two, and a printed address still hurries the read.
///
/// One node, started while the tab is open and left running once it closes:
/// three seconds on it opens a port and prints its address, and two seconds
/// after that opens a second and says nothing. The first is in the reading
/// well inside a turn, being printed; the second waits for the unwatched turn
/// after the read the print hurried, which a two-second turn would have been
/// long past.
#[tokio::test]
async fn a_terminal_no_tab_is_attached_to_is_read_every_ten_seconds() {
    let (a, _b) = linked().await;
    let (app, at) = a.served().await;

    let (number, mut terminal) = attached_from_a(&app, at).await;

    let printed = free_port();
    let quiet = free_port();
    terminal
        .typed(&format!(
            "node -e \"const n = require('net'); console.log('started'); \
             setTimeout(() => n.createServer().listen({printed}, '127.0.0.1', \
             () => console.log('up at http://local' + 'host:' + {printed} + '/')), 3000); \
             setTimeout(() => n.createServer().listen({quiet}, '127.0.0.1'), 5000)\"\n"
        ))
        .await;
    terminal.until("started").await;
    let started = Instant::now();
    terminal.closed().await;

    let ports = through(B, &ports_path());

    until(&app, &ports, |read| holds(read, printed)).await;
    let took = started.elapsed();
    assert!(
        took < Duration::from_secs(3) + HURRIED,
        "the printed port reached the reading {took:?} after the server started",
    );

    // Two seconds a turn would have read the quiet one by seven.
    tokio::time::sleep_until(started + Duration::from_secs(8)).await;
    let read: PortsView = call(&app, "GET", &ports).await;
    assert!(
        !holds(&read.terminals, quiet),
        "with no tab attached the quiet port should not have been read yet: {read:?}",
    );

    until(&app, &ports, |read| holds(read, quiet)).await;
    let took = started.elapsed();
    assert!(
        took < Duration::from_secs(3) + UNWATCHED + PROMPTLY,
        "the quiet port reached the reading {took:?} after the server started",
    );

    assert!(
        call::<PortsView>(&app, "GET", &ports)
            .await
            .terminals
            .iter()
            .any(|terminal| terminal.number == number),
    );
}

/// A member that stops answering keeps its Forward: it stands as unreachable,
/// its port still bound here and a connection to it failing, and A's own pages
/// are told — and once the member answers, it is forwarding again, nobody
/// having opened a tab and nothing about its ports having moved to say so.
#[tokio::test]
async fn a_member_not_answering_keeps_its_forward_until_it_answers_again() {
    let (a, b, _holding, link) = linked_forwarding_over_a_link().await;
    let (app, at) = a.served().await;

    let (_, port) = forwarded_and_closed(&app, at, |port| echoing("127.0.0.1", port)).await;

    let mut own = Listening::open(&app, "/api/ui/nudges").await;

    // B gone from the network: every dial to it now reaches nothing, which is
    // what the stream's next try writes down.
    link.pointed_at(nowhere());
    link.cut();

    let took = forwarded_until(&app, |read| {
        standing(read, port) == Some(ForwardStanding::Unreachable)
    })
    .await;
    assert!(
        took < STREAM_AGAIN + PROMPTLY,
        "the Forward took {took:?} to stand as unreachable",
    );

    assert!(
        heard_nudged(&mut own, Duration::from_millis(500))
            .await
            .contains(&Nudged::here(Nudge::Forwards)),
        "A's own pages should have been told the Forward's standing moved",
    );

    // Still A's to hold: nobody else takes the port while the member is away.
    let taken = TcpListener::bind(forwarded_v4(port))
        .expect_err("the port should still be held by the Forward");
    assert_eq!(taken.kind(), std::io::ErrorKind::AddrInUse);

    fails(forwarded_v4(port)).await;

    // And B back on the network.
    link.pointed_at(b.address);

    let took = forwarded_until(&app, |read| {
        standing(read, port) == Some(ForwardStanding::Forwarding)
    })
    .await;
    assert!(
        took < AGAIN_AFTER_A_MISS + PROMPTLY,
        "port {port} took {took:?} to be forwarded again once B answered",
    );

    echoes(forwarded_v4(port), "after-the-blip").await;
}

/// A member that answers again with the port gone — restarted while it was
/// away, its terminals ended with it — has its Forward end within the grace and
/// a turn of answering.
#[tokio::test]
async fn a_member_answering_again_without_the_port_ends_its_forward() {
    let (a, b, _holding, link) = linked_forwarding_over_a_link().await;
    let (app, at) = a.served().await;

    let (_, port) = forwarded_and_closed(&app, at, |port| echoing("127.0.0.1", port)).await;

    // Restarted before it goes away rather than while it is, so that it is
    // there to answer by the stream's next try.
    let again = b.restarted().await;

    link.pointed_at(nowhere());
    link.cut();

    forwarded_until(&app, |read| {
        standing(read, port) == Some(ForwardStanding::Unreachable)
    })
    .await;

    link.pointed_at(again.address);
    let back = Instant::now();

    forwarded_until(&app, <[ForwardView]>::is_empty).await;
    let took = back.elapsed();
    assert!(
        took < AGAIN_AFTER_A_MISS + GRACE + PROMPTLY,
        "the Forward ended {took:?} after its member answered without the port",
    );
}

/// A device unlinked is no member to wait for: its Forwards end at once.
#[tokio::test]
async fn an_unlinked_member_has_its_forwards_end_at_once() {
    let (a, _b, _holding) = linked_forwarding().await;
    let (app, at) = a.served().await;

    let (_, port) = forwarded_and_closed(&app, at, |port| echoing("127.0.0.1", port)).await;

    verkstead_store::forget_member(&a.pool, B).await.unwrap();
    let unlinked = Instant::now();

    forwarded_until(&app, <[ForwardView]>::is_empty).await;
    let took = unlinked.elapsed();
    assert!(
        took < PROMPTLY,
        "the Forward ended {took:?} after its member was unlinked",
    );

    TcpListener::bind(forwarded_v4(port)).expect("the port to have been let go");
}

/// An address nothing listens on: what a member gone from the network is
/// dialled at.
fn nowhere() -> SocketAddr {
    SocketAddr::new(Ipv4Addr::LOCALHOST.into(), free_port())
}

/// Whether a connection to `at` is ended without an answer: what a Forward
/// whose member cannot be dialled does with one.
async fn fails(at: SocketAddr) {
    let mut stream = tokio::time::timeout(PATIENCE, tokio::net::TcpStream::connect(at))
        .await
        .expect("the Forward to accept")
        .expect("the Forward to accept");

    stream.write_all(b"nobody-there").await.unwrap();

    let mut answer = Vec::new();
    tokio::time::timeout(PATIENCE, stream.read_to_end(&mut answer))
        .await
        .expect("the connection to be ended")
        .ok();
    assert!(answer.is_empty(), "{}", String::from_utf8_lossy(&answer));
}

/// Where A served afresh forwards: another IPv4 loopback address. The first A
/// still holds its Forwards on [`forwarding_on`] — one process cannot stop the
/// other's here — and a port already held at an address is a Forward skipped.
fn forwarding_afresh_on() -> Vec<std::net::IpAddr> {
    vec![std::net::IpAddr::V4(Ipv4Addr::new(127, 0, 0, 3))]
}

/// Attach to B's terminal from A's browser, start the echoing server in it on
/// a port of its own, wait until A forwards it, and close the tab: what every
/// test of a Forward with no tab open starts from.
async fn forwarded_and_closed(
    app: &Router,
    at: SocketAddr,
    typing: impl Fn(u16) -> String,
) -> (i64, u16) {
    let number = opened(
        app,
        &through(B, &format!("/api/ui/conversations/{THERE}/terminals")),
    )
    .await;

    let mut terminal = Terminal::attached(format!(
        "ws://{at}{}",
        through(
            B,
            &format!("/api/ui/conversations/{THERE}/terminals/{number}/attach")
        )
    ))
    .await;

    let port = free_port();
    terminal.typed(&typing(port)).await;
    terminal.until(&format!("listening-{port}")).await;

    forwarded_until(app, |read| {
        standing(read, port) == Some(ForwardStanding::Forwarding)
    })
    .await;

    terminal.closed().await;

    (number, port)
}

/// A hub served afresh while a member still offers a port holds the Forward
/// again within a few seconds of its start, nobody having opened a tab: it
/// reads everything the member offers it as its stream is taken up.
#[tokio::test]
async fn a_hub_served_afresh_holds_what_its_members_still_offer() {
    let (a, _b, _holding) = linked_forwarding().await;
    let (app, at) = a.served().await;

    let (number, port) = forwarded_and_closed(&app, at, |port| echoing("127.0.0.1", port)).await;

    // What B offers A, every Conversation at once.
    let offered: OfferedView = call(&app, "GET", &through(B, "/api/ui/ports")).await;
    assert_eq!(
        offered.conversations,
        vec![ConversationPorts {
            conversation: THERE,
            terminals: vec![TerminalPorts {
                number,
                ports: vec![port],
            }],
        }],
    );

    let started = Instant::now();
    let (again, mut holding, staying) = a.forwarding_again(forwarding_afresh_on()).await;
    holding.0.push(staying.started());

    forwarded_until(&again.workbench(), |read| {
        standing(read, port) == Some(ForwardStanding::Forwarding)
    })
    .await;
    let took = started.elapsed();
    assert!(
        took < PROMPTLY,
        "port {port} took {took:?} to be forwarded by A served afresh",
    );

    echoes(
        SocketAddr::new(forwarding_afresh_on()[0], port),
        "after-the-restart",
    )
    .await;
}

/// What a server is typed as that opens a second port when asked: node,
/// echoing on `first` as [`echoing`] does, and — on anything starting `open` —
/// listening on `second` too, printing its address and answering `opened`.
fn opening(first: u16, second: u16) -> String {
    format!(
        "node -e \"const n = require('net'); \
         const echo = s => s.on('data', d => String(d).startsWith('open') ? \
         n.createServer(echo).listen({second}, '127.0.0.1', () => \
         (console.log('up at http://local' + 'host:' + {second} + '/'), s.write('opened'))) : \
         s.write('echo:' + d)); \
         n.createServer(echo).listen({first}, '127.0.0.1', () => console.log('listening-' + {first}))\"\n"
    )
}

/// A member's Nudge stream dropping and coming back with no tab open leaves
/// its Forward standing, and a port opened while it was down — its `ports`
/// Nudge said to nobody — is forwarded once the stream is taken up again.
#[tokio::test]
async fn a_link_dropping_with_no_tab_open_keeps_what_is_forwarded() {
    let (a, _b, _holding, link) = linked_forwarding_over_a_link().await;
    let (app, at) = a.served().await;

    let second = free_port();
    let (_, first) = forwarded_and_closed(&app, at, |first| opening(first, second)).await;

    link.cut();
    let cut = Instant::now();

    // The second port opened while the stream is down, straight at B's
    // loopback: B reads it at once, being printed, and says so to nobody.
    let mut asking = tokio::net::TcpStream::connect((Ipv4Addr::LOCALHOST, first))
        .await
        .unwrap();
    exchanged(&mut asking, "open", "opened").await;

    until(&app, &through(B, &ports_path()), |read| holds(read, second)).await;

    let read: ForwardsView = call(&app, "GET", "/api/ui/forwards").await;
    assert!(
        cut.elapsed() < STREAM_AGAIN,
        "the stream should still have been down, {:?} after the cut",
        cut.elapsed(),
    );
    assert_eq!(
        standing(&read.forwards, second),
        None,
        "nothing should have told A of the second port while its stream was down",
    );

    forwarded_until(&app, |read| {
        standing(read, first) == Some(ForwardStanding::Forwarding)
            && standing(read, second) == Some(ForwardStanding::Forwarding)
    })
    .await;
    let took = cut.elapsed();
    assert!(
        took < STREAM_AGAIN + PROMPTLY,
        "the second port was forwarded {took:?} after the link dropped",
    );

    echoes(forwarded_v4(first), "after-the-link-dropped").await;
    echoes(forwarded_v4(second), "taken-up-while-it-was-down").await;
}

/// A member restarted has ended its terminals, so it answers the reading of
/// everything it offers with nothing — and the Forwards it had end within the
/// grace and a turn of the stream to it being taken up again.
#[tokio::test]
async fn a_member_restarted_offers_nothing_and_its_forwards_end() {
    let (a, b, _holding, link) = linked_forwarding_over_a_link().await;
    let (app, at) = a.served().await;

    forwarded_and_closed(&app, at, |port| echoing("127.0.0.1", port)).await;

    let again = b.restarted().await;
    link.pointed_at(again.address);
    link.cut();
    let cut = Instant::now();

    forwarded_until(&app, <[ForwardView]>::is_empty).await;
    let took = cut.elapsed();
    assert!(
        took < STREAM_AGAIN + GRACE + PROMPTLY,
        "the Forward ended {took:?} after its member restarted",
    );

    let offered: OfferedView = call(&app, "GET", &through(B, "/api/ui/ports")).await;
    assert_eq!(offered.conversations, Vec::new());
}

/// A port A already holds is skipped, *port busy here*, while the others are
/// forwarded as before — and is taken once it frees.
#[tokio::test]
async fn a_port_already_held_here_is_skipped_and_taken_once_it_frees() {
    let (a, _b, _holding) = linked_forwarding().await;
    let (app, at) = a.served().await;

    let number = opened(
        &app,
        &through(B, &format!("/api/ui/conversations/{THERE}/terminals")),
    )
    .await;

    let mut terminal = Terminal::attached(format!(
        "ws://{at}{}",
        through(
            B,
            &format!("/api/ui/conversations/{THERE}/terminals/{number}/attach")
        )
    ))
    .await;

    // This device's own server, on the very number the terminal is about to
    // listen on.
    let busy = free_port();
    let own = TcpListener::bind(forwarded_v4(busy)).expect("the port to be free here");

    let free = free_port();
    // In the background, so that the shell takes the second one too.
    terminal
        .typed(&echoing("127.0.0.1", busy).replace('\n', " &\n"))
        .await;
    terminal.until(&format!("listening-{busy}")).await;
    terminal.typed(&echoing("127.0.0.1", free)).await;
    terminal.until(&format!("listening-{free}")).await;

    forwarded_until(&app, |read| {
        standing(read, busy)
            == Some(ForwardStanding::Skipped {
                reason: ForwardSkip::PortBusy,
            })
            && standing(read, free) == Some(ForwardStanding::Forwarding)
    })
    .await;

    echoes(forwarded_v4(free), "beside-the-busy-one").await;

    // And this device's own server stopping frees it.
    drop(own);

    let took = forwarded_until(&app, |read| {
        standing(read, busy) == Some(ForwardStanding::Forwarding)
    })
    .await;
    assert!(took < PROMPTLY, "the freed port took {took:?} to be taken");

    echoes(forwarded_v4(busy), "once-it-freed").await;

    terminal.closed().await;
}

/// What A forwards is A's own: its reading is refused to a member over the
/// link, and its `forwards` Nudge goes down A's own stream and not the one a
/// member holds.
#[tokio::test]
async fn what_a_device_forwards_is_not_served_over_the_link() {
    let (a, b, _holding) = linked_forwarding().await;
    let (app, at) = a.served().await;
    let (relaying, _) = b.served().await;

    let mut own = Listening::open(&app, "/api/ui/nudges").await;
    let mut linked = Listening::open(&relaying, &through(A, "/api/ui/nudges")).await;

    let number = opened(
        &app,
        &through(B, &format!("/api/ui/conversations/{THERE}/terminals")),
    )
    .await;

    let mut terminal = Terminal::attached(format!(
        "ws://{at}{}",
        through(
            B,
            &format!("/api/ui/conversations/{THERE}/terminals/{number}/attach")
        )
    ))
    .await;

    let port = free_port();
    terminal.typed(&echoing("127.0.0.1", port)).await;
    terminal.until(&format!("listening-{port}")).await;

    forwarded_until(&app, |read| {
        standing(read, port) == Some(ForwardStanding::Forwarding)
    })
    .await;

    assert!(
        heard_nudged(&mut own, Duration::from_millis(500))
            .await
            .contains(&Nudged::here(Nudge::Forwards)),
        "A's own pages should have been told",
    );
    assert!(
        !heard_nudged(&mut linked, Duration::from_millis(500))
            .await
            .iter()
            .any(|nudged| nudged.moved == Nudge::Forwards),
        "a member should never be told what A forwards",
    );

    let (status, said) = asked(&relaying, "GET", &through(A, "/api/ui/forwards")).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{said}");
    assert!(said.contains("keeps to itself"), "{said}");

    // Nor is a member's × on one of them.
    let (status, said) = asked(&relaying, "DELETE", &through(A, &dismissing(number, port))).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{said}");
    forwarded_until(&app, |read| {
        standing(read, port) == Some(ForwardStanding::Forwarding)
    })
    .await;

    terminal.closed().await;
}

/// Where the × on the row of B's terminal `number`'s Forward of `port` presses.
fn dismissing(number: i64, port: u16) -> String {
    format!("/api/ui/forwards/{B}/{THERE}/{number}/{port}")
}

/// Press the × on that row, which answers with nothing whether or not A held
/// the Forward.
async fn dismissed(app: &Router, number: i64, port: u16) {
    let (status, said) = asked(app, "DELETE", &dismissing(number, port)).await;
    assert_eq!(status, StatusCode::NO_CONTENT, "{said}");
}

/// A press on the × ends the Forward at once — the port free at A's loopback
/// and the row gone — and B's next `ports` Nudge, with the port still offered,
/// does not take it again. Attaching the terminal again does.
#[tokio::test]
async fn a_cross_ends_a_forward_until_the_terminal_is_attached_again() {
    let (a, _b, _holding) = linked_forwarding().await;
    let (app, at) = a.served().await;

    let second = free_port();
    let (number, first) = forwarded_and_closed(&app, at, |first| opening(first, second)).await;

    dismissed(&app, number, first).await;

    let read: ForwardsView = call(&app, "GET", "/api/ui/forwards").await;
    assert_eq!(standing(&read.forwards, first), None, "{:?}", read.forwards);
    assert!(
        tokio::net::TcpStream::connect(forwarded_v4(first))
            .await
            .is_err(),
        "nothing should be listening on port {first} here once the Forward is dismissed",
    );

    // Pressed again, on a Forward A no longer holds: nothing to do.
    dismissed(&app, number, first).await;

    // A second port, opened and printed: a `ports` Nudge from B, whose reading
    // still offers the first.
    let mut asking = tokio::net::TcpStream::connect((Ipv4Addr::LOCALHOST, first))
        .await
        .unwrap();
    exchanged(&mut asking, "open", "opened").await;

    forwarded_until(&app, |read| {
        standing(read, second) == Some(ForwardStanding::Forwarding)
    })
    .await;

    let offered: PortsView = call(&app, "GET", &through(B, &ports_path())).await;
    assert!(holds(&offered.terminals, first), "{:?}", offered.terminals);

    let read: ForwardsView = call(&app, "GET", "/api/ui/forwards").await;
    assert_eq!(
        standing(&read.forwards, first),
        None,
        "a dismissed Forward should not be taken again while its port is still offered: {:?}",
        read.forwards,
    );

    // The tab opened again, which is the way back.
    let terminal = Terminal::attached(format!(
        "ws://{at}{}",
        through(
            B,
            &format!("/api/ui/conversations/{THERE}/terminals/{number}/attach")
        )
    ))
    .await;
    let attached = Instant::now();

    forwarded_until(&app, |read| {
        standing(read, first) == Some(ForwardStanding::Forwarding)
    })
    .await;
    let took = attached.elapsed();
    assert!(
        took < PROMPTLY,
        "port {first} was taken again {took:?} after the terminal was attached again",
    );

    echoes(forwarded_v4(first), "after-the-tab-opened-again").await;

    terminal.closed().await;
}

/// What a server is typed as that stops and listens again when asked: node,
/// echoing on `port` as [`echoing`] does, and — on anything starting `restart`
/// — closing, printing an address so the port's going is read at once, and
/// listening again four seconds on, printing its address again.
fn restarting(port: u16) -> String {
    format!(
        "node -e \"const n = require('net'); let server; \
         const echo = s => s.on('data', d => String(d).startsWith('restart') ? restart() : \
         s.write('echo:' + d)); \
         const up = said => {{ server = n.createServer(echo).listen({port}, '127.0.0.1', \
         () => console.log(said)); }}; \
         const restart = () => {{ server.close(); \
         setTimeout(() => console.log('down at http://local' + 'host:' + {port} + '/'), 300); \
         setTimeout(() => up('up again at http://local' + 'host:' + {port} + '/'), 4000); }}; \
         up('listening-' + {port})\"\n"
    )
}

/// A dismissal is forgotten once the port leaves the member's reading, so a
/// server that stops and listens again is forwarded afresh, nobody having
/// opened a tab.
#[tokio::test]
async fn a_dismissed_server_listening_again_is_forwarded_again() {
    let (a, _b, _holding) = linked_forwarding().await;
    let (app, at) = a.served().await;

    let (number, port) = forwarded_and_closed(&app, at, restarting).await;

    dismissed(&app, number, port).await;

    let mut asking = tokio::net::TcpStream::connect((Ipv4Addr::LOCALHOST, port))
        .await
        .unwrap();
    asking.write_all(b"restart").await.unwrap();
    drop(asking);

    // Gone from B's reading, and back in it.
    until(&app, &through(B, &ports_path()), |read| !holds(read, port)).await;
    until(&app, &through(B, &ports_path()), |read| holds(read, port)).await;

    forwarded_until(&app, |read| {
        standing(read, port) == Some(ForwardStanding::Forwarding)
    })
    .await;

    echoes(forwarded_v4(port), "after-it-listened-again").await;
}

/// A dismissal is memory: a hub served afresh reads everything its member
/// offers and takes a dismissed Forward again.
#[tokio::test]
async fn a_hub_served_afresh_takes_a_dismissed_forward_again() {
    let (a, _b, _holding) = linked_forwarding().await;
    let (app, at) = a.served().await;

    let (number, port) = forwarded_and_closed(&app, at, |port| echoing("127.0.0.1", port)).await;

    dismissed(&app, number, port).await;
    forwarded_until(&app, <[ForwardView]>::is_empty).await;

    let (again, mut holding, staying) = a.forwarding_again(forwarding_afresh_on()).await;
    holding.0.push(staying.started());

    forwarded_until(&again.workbench(), |read| {
        standing(read, port) == Some(ForwardStanding::Forwarding)
    })
    .await;

    echoes(
        SocketAddr::new(forwarding_afresh_on()[0], port),
        "after-the-restart",
    )
    .await;
}
