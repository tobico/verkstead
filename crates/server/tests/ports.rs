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
//! **Linux alone**, which is the one platform that reads anything yet: the other
//! two read nothing and say so in `terminals::ports`.
#![cfg(target_os = "linux")]

use std::net::{Ipv4Addr, SocketAddr, TcpListener};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
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
use verkstead_render::{PortsView, Shown, TerminalOpened, TerminalPorts, Watching};
use verkstead_schema::Nudge;
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

/// How long to listen on for a Nudge that should not come at all: past a whole
/// turn of the reading, twice.
const SETTLING: Duration = Duration::from_secs(5);

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

    dir: tempfile::TempDir,
    _sandboxing: Vec<tempfile::TempDir>,
}

impl Verkstead {
    /// A Verkstead with a store and an identity, its Peer Listener up on a port
    /// the machine picked. One that `runs` terminals serves its namespace over
    /// the link with Sandboxes behind it, over the same state its own browser's
    /// workbench answers out of — which is what says, of an attach, whether it
    /// came over the link and from whom.
    async fn answering(id: &str, runs: bool) -> Verkstead {
        let dir = tempfile::tempdir().unwrap();
        let pool = open_database(&dir.path().join("verkstead.db"))
            .await
            .unwrap();

        let device = Device::stated(dir.path(), id).unwrap();
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

    let repo = repository(b.dir.path().join("verkstead"));
    let registered = store::register_repo(&b.pool, &repo, "verkstead", "main")
        .await
        .unwrap()
        .expect("nothing is registered at that path yet");

    let conversation = store::start_conversation(&b.pool, registered.id, BRANCH, A)
        .await
        .unwrap()
        .expect("the Repo was just registered");
    assert_eq!(conversation, THERE);

    paired(&b, b.dir.path(), conversation).await;

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

    (a, b)
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

    // And A letting go is A's terminals going from the reading altogether.
    terminal.closed().await;

    until(
        &app,
        &through(B, &ports_path()),
        <[TerminalPorts]>::is_empty,
    )
    .await;
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

    let port = free_port();
    terminal.typed(&serving(port)).await;
    terminal.until(&format!("listening-{port}")).await;

    assert!(
        !page.heard(SETTLING).await.contains(&PORTS),
        "a terminal no member is attached to should not have been read",
    );

    // A holds no attach on it, so A reads nothing of it.
    let read: PortsView = call(&relaying, "GET", &through(B, &ports_path())).await;
    assert_eq!(read.terminals, Vec::new());

    // And B's own browser is not served the reading at all.
    let (status, _) = asked(&own, "GET", &ports_path()).await;
    assert_eq!(status, StatusCode::FORBIDDEN);

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
/// node on `host`, echoing what it is sent behind `echo:` and ending the
/// connection itself — with `ended` — on anything starting `end`.
fn echoing(host: &str, port: u16) -> String {
    format!(
        "node -e \"require('net').createServer(s => s.on('data', d => \
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
/// terminal it holds no attach on, and the device's own browser are each refused
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
    assert!(said.contains("nothing answered"), "{said}");

    // A port the terminal never opened.
    let never = free_port();
    let (status, said) = upgraded(at, &through(B, &port_path(number, never)))
        .await
        .expect_err("a port not in the reading should have been refused");
    assert_eq!(status, 404, "{said}");
    assert!(said.contains("not listening"), "{said}");

    // A terminal A holds no attach on, for a port that is listening in another.
    let (status, said) = upgraded(at, &through(B, &port_path(unattached, elsewhere)))
        .await
        .expect_err("a terminal the caller is not attached to should have been refused");
    assert_eq!(status, 403, "{said}");
    assert!(said.contains("holding an attach"), "{said}");

    // And B's own browser, for the very port A may have.
    let (status, said) = upgraded(own, &port_path(number, elsewhere))
        .await
        .expect_err("the device's own browser should have been refused");
    assert_eq!(status, 403, "{said}");
    assert!(said.contains("over the link"), "{said}");

    terminal.closed().await;
}
