//! The sockets over the hop: a socket opened against one device, put to one of
//! its members over the Peer Listener, and the two ends held together
//! (ADR-0020, *The opened device relays*).
//!
//! **Two Verksteads and a real upgrade between them.** `tests/relaying.rs` is
//! the same hop asked about calls, and this is the same hop asked about the three
//! endpoints that answer `101` instead of an answer. Nothing here stubs either
//! end: A's workbench is *served*, because an upgrade is a connection rather
//! than a request and a `oneshot` over the Router cannot make one, and what
//! stands at the far end is B's own namespace behind the Member Gate.
//!
//! **A terminal is the socket read for traffic**, because it is the one that
//! carries any: a shell really running in B's own Sandbox, keystrokes really
//! typed into it from a browser on A, and what the shell printed read back all
//! the way. Which is why the far end here is stood up with Sandboxes behind it —
//! see [`sandboxing`]. A session's Screen is the same function on the far side
//! (`crate::screen::follow`) over the same bridge, pointed at an agent instead of
//! a shell, and `tests/sessions.rs` is where one of those is watched as it
//! prints.
//!
//! **And a Code pane's watcher is the socket read for its silence.** It carries
//! nothing in either direction, so *open* is the whole of what it says, and what
//! it costs the other machine is a watch per Worktree. So those claims are made
//! the way `tests/watching.rs` makes them — a file written by something that is
//! not Verkstead, and the `files` Nudge it reaches an open page as — except that
//! here the page is on A, the Worktree is on B, and the Nudge is read off B's own
//! stream through the relay. A watcher left running behind a pane that closed is
//! exactly what would not show up any other way.
//!
//! **And a member that goes away is a member that goes away.** The one thing that
//! cannot be asked of two servers in one process is a machine switching off, so
//! A reaches B down a patch of wire this file owns and can cut — see [`Cable`].
//! What A holds after that is what it would hold if the laptop at the other end
//! had suspended.

use std::net::{Ipv4Addr, SocketAddr};
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
use tokio::sync::watch;
use tokio_tungstenite::tungstenite::Message;
use tower::ServiceExt;
use verkstead_render::{Shown, TerminalOpened, Watching};
use verkstead_schema::Nudge;
use verkstead_server::attachments::Attachments;
use verkstead_server::build_cache::BuildCache;
use verkstead_server::device::reading::Reading;
use verkstead_server::device::{Device, Devices};
use verkstead_server::handoffs::Handoffs;
use verkstead_server::nudge::Nudges;
use verkstead_server::peer::joining::Joins;
use verkstead_server::peer::{self, Members};
use verkstead_server::platform::Platform;
use verkstead_server::remote::Tailscale;
use verkstead_server::sandbox::{Executable, Homes, Reachable, SandboxConfig};
use verkstead_server::settings::Settings;
use verkstead_server::skills::Skills;
use verkstead_server::{
    Agents, open_database, router_answering_devices_telling, router_over_the_link_running_sessions,
    store,
};
use verkstead_store::{Linking, record_member};

/// The device the browser is on, which relays.
const A: &str = "aa00bb11cc22dd33ee44ff5566778899";

/// The device its Conversation lives on, which is reached through the hop.
const B: &str = "0011223344556677889900aabbccddee";

/// The branch B's one Conversation is on.
const BRANCH: &str = "following-over-there";

/// B's Conversation, which is the first one in a store with nothing else in it.
const THERE: i64 = 1;

/// How long a test will wait for something it expects — a Nudge, a frame off a
/// socket, a socket closing. Generous, because it is only ever paid when the
/// assertion is about to fail.
///
/// `tests/sessions.rs`'s half-minute rather than a few seconds, and for the
/// reason that suite takes one: what is on the other end of a terminal is the
/// *machine's own* login shell, and a shell may spend its first seconds asking
/// the terminal questions nothing on this end of a socket answers — fish waits
/// ten of them for an answer about the device it is on before carrying on
/// regardless. So the first thing typed into a terminal here may not be acted on
/// for as long as that shell takes to give up, on a machine whose shell does it
/// at all.
const PATIENCE: Duration = Duration::from_secs(30);

/// And how long it listens on after everything expected has arrived, to catch a
/// Nudge that should not have been sent at all. `tests/watching.rs`'s window,
/// which is past the watcher's own debounce and ceiling both.
const SETTLING: Duration = Duration::from_secs(3);

/// And how long a watcher that should have stopped is given to stop: the grace a
/// detach waits out, with room over it.
const STOPPING: Duration = Duration::from_secs(4);

/// One Verkstead: its store, its identity, where it keeps what it makes, and
/// where its Peer Listener landed.
///
/// `tests/relaying.rs`'s, with a served workbench added — see
/// [`Verkstead::served`].
struct Verkstead {
    device: Device,
    pool: SqlitePool,
    members: Members,
    reading: Reading,

    /// Where the other device dials it. The operating system's port rather than
    /// 8423, for the reason every other suite here takes one: two of these
    /// running at once must not fight each other.
    address: SocketAddr,

    /// Held for the length of the test: the identity, the database and the
    /// repository all live in it.
    dir: tempfile::TempDir,

    /// And the two a Sandbox is built out of, where this device is the one
    /// running them: where a session's home is made, and the one directory every
    /// Sandbox gets read-write. Held here for the same reason `dir` is — a
    /// Sandbox whose directories went out from under it fails obscurely.
    _sandboxing: Vec<tempfile::TempDir>,
}

impl Verkstead {
    /// A Verkstead with a store of its own, an identity in it, and its Peer
    /// Listener up on a port the machine picked — serving `over_the_link`
    /// behind the Member Gate.
    ///
    /// The one that serves it serves it with Sandboxes behind it, because a
    /// Conversation terminal is a shell inside one: see
    /// [`router_over_the_link_running_sessions`], and [`sandboxing`], which is
    /// what a device that can open one is configured with.
    async fn answering(id: &str, over_the_link: bool) -> Verkstead {
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

        tokio::spawn(listener.serving(peer::router(
            device.clone(),
            reading.clone(),
            members.clone(),
            Joins::none(),
            Nudges::new(),
            // The far end of the hop, or nothing at all: A is the device the
            // browser opened and serves its members nothing in this file.
            match over_the_link {
                true => {
                    let home = tempfile::tempdir().unwrap();
                    let spill = tempfile::tempdir().unwrap();
                    let agents = sandboxing(dir.path(), home.path(), spill.path());

                    held.push(home);
                    held.push(spill);

                    router_over_the_link_running_sessions(
                        pool.clone(),
                        dir.path().to_owned(),
                        agents,
                    )
                }
                false => Router::new(),
            },
        )));

        Verkstead {
            device,
            pool,
            members,
            reading,
            address,
            dir,
            _sandboxing: held,
        }
    }

    /// The workbench its own browser talks to.
    fn workbench(&self) -> Router {
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
    }

    /// And that workbench served on a port of its own, which is what a socket
    /// takes: an upgrade is a connection rather than a request, so it is the one
    /// thing a `oneshot` over the Router cannot ask for.
    ///
    /// The Router comes back beside the address and is the same one being served,
    /// so a Nudge stream opened through it is a stream over the state the socket
    /// dialled over there started something in.
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

    /// The one address it advertises.
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

/// A machine with no Tailscale on it: `verkstead-no-such-tailscale` is a program
/// that is not there, which is what having none *is*.
fn no_tailscale() -> Tailscale {
    Tailscale::running(vec!["verkstead-no-such-tailscale".to_owned()], 8422)
}

/// What a device that can open a terminal is configured with: the real thing,
/// over directories of this test's own.
///
/// `tests/sessions.rs`'s configuration with nothing standing in for anything —
/// there is no stub agent here, because a terminal has no agent in it: what runs
/// in one is the server's own shell, so what this has to be able to do is build a
/// Sandbox and open a pseudo-terminal in it.
///
/// No build cache, nothing being compiled; and the address a session would dial
/// back on is nowhere in particular, nothing here being a session.
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

/// A patch of wire between two devices, which the test can cut.
///
/// Everything A does over it is what A would do over the LAN — the handshake and
/// the certificate are between the two ends, and this forwards bytes — so a
/// member reached through one is reached the ordinary way. What it adds is the
/// one thing two servers in one process cannot do to each other: stop being
/// there, mid-socket, without a word.
struct Cable {
    /// The address A is linked to B at.
    at: String,

    /// Told once, and every connection this is holding is dropped.
    cut: watch::Sender<bool>,
}

impl Cable {
    /// A cable onto `onwards`, listening on a port the machine picked.
    async fn onto(onwards: String) -> Cable {
        let listener = tokio::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .await
            .unwrap();
        let at = format!("127.0.0.1:{}", listener.local_addr().unwrap().port());

        // A watch rather than a notify, so that a connection which has not
        // reached its wait yet is cut all the same: what each task reads is the
        // value, and a value already changed is a wait that returns at once.
        let (cut, _) = watch::channel(false);

        tokio::spawn({
            let cut = cut.clone();

            async move {
                loop {
                    let mut told = cut.subscribe();

                    let accepted = tokio::select! {
                        accepted = listener.accept() => accepted,
                        _ = told.changed() => return,
                    };

                    let Ok((mut inbound, _)) = accepted else {
                        return;
                    };

                    let onwards = onwards.clone();
                    let mut told = cut.subscribe();

                    tokio::spawn(async move {
                        let Ok(mut outbound) = tokio::net::TcpStream::connect(&onwards).await
                        else {
                            return;
                        };

                        tokio::select! {
                            _ = tokio::io::copy_bidirectional(&mut inbound, &mut outbound) => {}
                            _ = told.changed() => {}
                        }
                    });
                }
            }
        });

        Cable { at, cut }
    }

    /// Where A is told B is.
    fn at(&self) -> String {
        self.at.clone()
    }

    /// And cut it, which is the machine at the far end going away.
    fn cut(&self) {
        self.cut.send(true).unwrap();
    }
}

/// A and B up, with one Repo on B and one Conversation grilling in a Worktree of
/// its own, and B holding a membership for A — everything but the one thing the
/// two shapes below differ in, which is where A is told B *is*.
///
/// A real checkout because a watcher watches directories: `tests/watching.rs`'s
/// fixture, made on the far side of the hop. The Worktree comes back because what
/// every test here writes into is that directory, from outside this server
/// entirely.
async fn both() -> (Verkstead, Verkstead, PathBuf) {
    let a = Verkstead::answering(A, false).await;
    let b = Verkstead::answering(B, true).await;

    let repo = repository(b.dir.path().join("verkstead"));
    let registered = store::register_repo(&b.pool, &repo, "verkstead", "main")
        .await
        .unwrap()
        .expect("nothing is registered at that path yet");

    let conversation = store::start_conversation(&b.pool, registered.id, BRANCH)
        .await
        .unwrap()
        .expect("the Repo was just registered");
    assert_eq!(conversation, THERE);

    paired(&b, b.dir.path(), conversation).await;

    let worktree = b.dir.path().join("worktrees/verkstead-following");
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

    (a, b, worktree)
}

/// The pair linked both ways, which is what a settled cluster is: A dials B
/// pinned on B's fingerprint, and B admits A because A's certificate is one it
/// holds a membership for.
async fn linked() -> (Verkstead, Verkstead, PathBuf) {
    let (a, b, worktree) = both().await;

    a.linked_to(&b.device, vec![b.at()]).await;

    (a, b, worktree)
}

/// And the same pair with a cable in the middle of it, for the one test about
/// B not being there any more — see [`Cable`]. A is linked to the cable's address
/// rather than to B's, which is the whole of the difference: the handshake and
/// the certificate are B's either way.
async fn linked_by_cable() -> (Verkstead, Verkstead, PathBuf, Cable) {
    let (a, b, worktree) = both().await;
    let cable = Cable::onto(b.at()).await;

    a.linked_to(&b.device, vec![cable.at()]).await;

    (a, b, worktree, cable)
}

/// A git repository with one commit in it. `tests/watching.rs`'s.
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

/// Run git in `dir`, insisting it worked. Scaffolding rather than the code under
/// test, so a failure here is a broken test.
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

/// Where a local call stands when it is for `device` instead: the prefix takes
/// the place of `/api/ui`, and everything under it is untouched.
fn through(device: &str, path: &str) -> String {
    let leaf = path
        .strip_prefix("/api/ui")
        .expect("a relayed call is a call into the viewer's own namespace");

    format!("/api/ui/members/{device}{leaf}")
}

/// And where one of the sockets stands, as a browser on `at` would dial it.
fn socket(at: SocketAddr, device: &str, path: &str) -> String {
    format!("ws://{at}{}", through(device, path))
}

/// A press through the hop, which is every one this file makes: none of them
/// carries a body.
async fn post<T: DeserializeOwned>(app: &Router, path: &str) -> T {
    let answered = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(path)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let status = answered.status();
    let said = answered.into_body().collect().await.unwrap().to_bytes();
    let said = String::from_utf8_lossy(&said).into_owned();

    assert_eq!(status, StatusCode::OK, "POST {path}: {said}");
    serde_json::from_str(&said).unwrap_or_else(|why| panic!("POST {path} answered {said}: {why}"))
}

/// An Agent Profile on `device`, paired with its Conversation.
///
/// Which is what a terminal needs of the far end besides a Worktree: it has no
/// role of its own and runs under the account the work is done under, so a
/// Conversation with no Profile settled has no account to open a shell in. An
/// account of directories this test made, which is what every suite here pairs
/// with: what a Profile names is where a session's `~/.claude` is built from, and
/// nothing in this file logs in anywhere.
///
/// Settled while the Conversation is still a Draft, because that is when a
/// Pairing is settled: what runs the work is fixed when grilling starts.
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

/// A file written at the top of a Worktree, the way a terminal tab writes one:
/// outside this server entirely, and on the machine the Conversation lives on.
fn wrote(worktree: &Path, name: &str) {
    std::fs::write(worktree.join(name), "something\n").unwrap();
}

/// A Code pane's attachment, as the browser holds one: the socket dialled
/// through the device it opened, and closed when the pane goes.
struct Pane(
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
);

impl Pane {
    /// Draw a Code pane on a member's Conversation, which is the attachment
    /// being taken over there.
    async fn drawn(at: SocketAddr, device: &str, conversation: i64) -> Pane {
        let url = socket(
            at,
            device,
            &format!("/api/ui/conversations/{conversation}/files/attach"),
        );

        let (socket, _) = tokio_tungstenite::connect_async(&url)
            .await
            .unwrap_or_else(|error| panic!("{url} to be attachable: {error}"));

        Pane(socket)
    }

    /// And take it down, which is the socket closing under both servers.
    async fn closed(mut self) {
        self.0.close(None).await.unwrap();
    }
}

/// A Conversation terminal, as the pane holds one: the socket dialled through the
/// device the browser opened, and what has been printed down it so far.
///
/// The grid is not rebuilt here — `tests/sessions.rs` is where a Screen is fed to
/// a terminal and read off the cells. What this file is about is the bytes making
/// the crossing, so what it keeps is the bytes.
struct Terminal {
    socket: tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
    printed: String,
}

impl Terminal {
    /// Attach to one of a member's terminals, and take the repaint it opens with.
    async fn attached(at: SocketAddr, device: &str, conversation: i64, number: i64) -> Terminal {
        let url = socket(
            at,
            device,
            &format!("/api/ui/conversations/{conversation}/terminals/{number}/attach"),
        );

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

    /// The next thing the member says down it, or a panic if it says nothing for
    /// long enough.
    async fn shown(&mut self) -> Shown {
        let said = tokio::time::timeout(PATIENCE, self.socket.next())
            .await
            .expect("the member should have said something")
            .expect("the socket should still be open")
            .expect("the socket should be readable");

        match said {
            Message::Text(said) => serde_json::from_str(&said).unwrap(),
            said => panic!("a Screen's socket speaks JSON, and it said: {said:?}"),
        }
    }

    /// Type something into it, which is what goes back up the socket.
    async fn typed(&mut self, keystrokes: &str) {
        let putting = serde_json::to_string(&Watching::PutIn(keystrokes.to_owned())).unwrap();

        self.socket
            .send(Message::Text(putting.into()))
            .await
            .unwrap();
    }

    /// And follow it until `wanted` shows up in what has been printed, or give up.
    async fn until(&mut self, wanted: &str) {
        let deadline = tokio::time::Instant::now() + PATIENCE;

        while !self.printed.contains(wanted) {
            assert!(
                tokio::time::Instant::now() < deadline,
                "the terminal never printed {wanted:?}. It has printed: {:?}",
                self.printed,
            );

            if let Shown::Printed(printed) = self.shown().await {
                self.printed.push_str(&printed);
            }
        }
    }
}

/// An open page, listening on a member's Nudge stream through the relay.
/// `tests/watching.rs`'s reading, over the hop.
struct Listening {
    body: Body,
    buffered: String,
}

impl Listening {
    /// Open that stream the way a page does. Returns once the response is in
    /// hand, which is after the far end's handler has subscribed — so anything
    /// the test does next is something this page is listening for.
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

    /// The next whole frame off the stream, whatever kind it is.
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
        let until = tokio::time::Instant::now() + over;

        loop {
            let left = until.saturating_duration_since(tokio::time::Instant::now());

            let Ok(frame) = tokio::time::timeout(left, self.frame()).await else {
                return heard;
            };

            if let Some(data) = frame.lines().find_map(|line| line.strip_prefix("data: ")) {
                heard.push(serde_json::from_str(data).unwrap());
            }
        }
    }

    /// And the one Nudge expected, waited for rather than counted: the timeout
    /// is the assertion failing rather than the assertion.
    async fn nudge(&mut self) -> Nudge {
        let frame = tokio::time::timeout(PATIENCE, async {
            loop {
                let frame = self.frame().await;

                if let Some(data) = frame.lines().find_map(|line| line.strip_prefix("data: ")) {
                    return data.to_owned();
                }
            }
        })
        .await
        .expect("a Nudge should have arrived");

        serde_json::from_str(&frame).unwrap()
    }
}

/// A remote Conversation's terminal echoes what is typed into it: a shell running
/// in the member's own Sandbox, driven from a browser on the other device.
///
/// **The one socket that carries traffic both ways**, and it is carrying it over
/// two connections and a TLS handshake: what is typed goes up the browser's
/// socket, across the hop, into a pseudo-terminal on the member, and what the
/// shell printed comes all the way back. A session's Screen is the same function
/// over the same bridge pointed at an agent instead — `tests/sessions.rs` is where
/// one of those is watched as it prints.
///
/// Read off what the terminal *printed* rather than off any one thing in it,
/// because what a shell does with a line is the shell's own: the prompt repeats
/// the keystrokes as they arrive and the command prints its own line after them,
/// and either of those reaching this end is the bytes having made the round trip.
/// A marker nothing else would print is what is looked for, and how the shell
/// chose to lay it out is nobody's claim here — the grid is read off the cells in
/// `tests/sessions.rs`, which is the file about what a Screen *shows*.
#[tokio::test]
async fn a_remote_terminal_echoes_what_is_typed_into_it() {
    let (a, _b, _worktree) = linked().await;
    let (app, at) = a.served().await;

    let opened: TerminalOpened = post(
        &app,
        &through(B, &format!("/api/ui/conversations/{THERE}/terminals")),
    )
    .await;

    let TerminalOpened::Opened { number } = opened else {
        panic!("the member should have opened a terminal, and it said: {opened:?}");
    };

    let mut terminal = Terminal::attached(at, B, THERE, number).await;

    terminal.typed("echo the-relay-is-open\n").await;
    terminal.until("the-relay-is-open").await;
}

/// A Code pane drawn on a member's Conversation starts the watcher on the member,
/// and closing the pane stops it.
///
/// The two halves in one test because they are one claim about one socket, and
/// the first is what makes the second mean anything: a file heard about proves
/// the watcher was running over there, and a test that only asserted the silence
/// would pass against a hop that never opened a socket at all.
#[tokio::test]
async fn a_remote_pane_starts_the_watcher_on_the_member_and_closing_it_stops_it() {
    let (a, _b, worktree) = linked().await;
    let (app, at) = a.served().await;

    let pane = Pane::drawn(at, B, THERE).await;
    let mut page = Listening::open(&app, &through(B, "/api/ui/nudges")).await;

    wrote(&worktree, "before");

    assert_eq!(
        page.nudge().await,
        Nudge::Files {
            conversation: THERE
        },
        "a file written in the member's Worktree should reach the page on this device",
    );

    pane.closed().await;
    tokio::time::sleep(STOPPING).await;

    // Anything the closing itself was still announcing, drained before the write
    // this is about.
    let _ = page.heard(Duration::from_millis(100)).await;

    wrote(&worktree, "after");

    assert_eq!(
        page.heard(SETTLING).await,
        Vec::new(),
        "a Worktree on the member should not be watched for a pane that has closed",
    );
}

/// And nothing at all where no pane was ever drawn, which is what makes the
/// watcher the pane's rather than the Conversation's.
///
/// The same reading `tests/watching.rs` makes locally, asked here because the
/// socket is what starts one and this is the file about that socket crossing a
/// hop: a relay that opened the far socket for something other than a pane would
/// be watching a member's disk for nobody.
#[tokio::test]
async fn nothing_is_watched_on_the_member_for_a_conversation_with_no_pane_on_it() {
    let (a, _b, worktree) = linked().await;
    let (app, _at) = a.served().await;

    let mut page = Listening::open(&app, &through(B, "/api/ui/nudges")).await;

    wrote(&worktree, "notes.md");

    assert_eq!(page.heard(SETTLING).await, Vec::new());
}

/// A relayed socket carries a frame the other way even where the endpoint at the
/// far end of it says nothing, because the answer is the far end's own WebSocket
/// rather than its handler.
///
/// A ping, which is the one thing every WebSocket answers whatever it is for: the
/// watcher's socket is read and nothing is expected up it, so a pong coming back
/// down is the protocol underneath. And the payload is asserted rather than the
/// kind — what crosses this hop is bytes, and a pong carrying somebody else's
/// bytes would be a frame this device had rewritten.
#[tokio::test]
async fn a_relayed_socket_carries_a_frame_back_the_other_way() {
    let (a, _b, _worktree) = linked().await;
    let (_app, at) = a.served().await;

    let mut pane = Pane::drawn(at, B, THERE).await;

    pane.0
        .send(Message::Ping("still there?".into()))
        .await
        .unwrap();

    let answered = tokio::time::timeout(PATIENCE, pane.0.next())
        .await
        .expect("the member should have answered the ping")
        .expect("the socket should still be open")
        .expect("the socket should be readable");

    assert_eq!(answered, Message::Pong("still there?".into()));
}

/// A member that goes away closes the browser's socket, rather than leaving the
/// pane attached to nothing with no way to find out.
#[tokio::test]
async fn a_member_that_goes_away_closes_the_browsers_socket() {
    let (a, _b, _worktree, cable) = linked_by_cable().await;
    let (_app, at) = a.served().await;
    let mut pane = Pane::drawn(at, B, THERE).await;

    cable.cut();

    let ended = tokio::time::timeout(PATIENCE, async {
        // Read until the socket says it is over, whichever way it says it: a
        // member that stopped without a word is a connection that ended rather
        // than a close anybody sent.
        loop {
            match pane.0.next().await {
                None => return,
                Some(Err(_)) => return,
                Some(Ok(Message::Close(_))) => return,
                Some(Ok(_)) => continue,
            }
        }
    })
    .await;

    assert!(
        ended.is_ok(),
        "the browser's socket should have closed behind the member",
    );
}

/// And a refusal at the far end is still a refusal: a Conversation that device
/// has no record of is answered as it is locally, rather than turning into a
/// socket that opens and says nothing.
#[tokio::test]
async fn a_conversation_the_member_has_no_record_of_is_refused_rather_than_opened() {
    let (a, _b, _worktree) = linked().await;
    let (_app, at) = a.served().await;

    let refused =
        tokio_tungstenite::connect_async(socket(at, B, "/api/ui/conversations/404/files/attach"))
            .await;

    let Err(tokio_tungstenite::tungstenite::Error::Http(answered)) = refused else {
        panic!("a Conversation the member has no record of should have no attachment to take");
    };

    assert_eq!(
        answered.status(),
        StatusCode::NOT_FOUND,
        "the member's own refusal should be what the browser is handed",
    );
}

/// And so is a terminal number that is not live, which is the same refusal from
/// the other of the two sockets that take one.
#[tokio::test]
async fn a_terminal_that_is_not_live_on_the_member_is_refused() {
    let (a, _b, _worktree) = linked().await;
    let (_app, at) = a.served().await;

    let refused = tokio_tungstenite::connect_async(socket(
        at,
        B,
        &format!("/api/ui/conversations/{THERE}/terminals/7/attach"),
    ))
    .await;

    let Err(tokio_tungstenite::tungstenite::Error::Http(answered)) = refused else {
        panic!("a terminal number that is not live should have nothing to attach to");
    };

    assert_eq!(answered.status(), StatusCode::NOT_FOUND);
}

/// And a device this one is linked to at all has no socket to offer either — the
/// refusal the hop makes itself, rather than one it carried.
#[tokio::test]
async fn a_device_that_is_no_member_has_no_socket_to_relay() {
    let (a, _b, _worktree) = linked().await;
    let (_app, at) = a.served().await;

    let refused = tokio_tungstenite::connect_async(socket(
        at,
        "ffeeddccbbaa00998877665544332211",
        &format!("/api/ui/conversations/{THERE}/files/attach"),
    ))
    .await;

    let Err(tokio_tungstenite::tungstenite::Error::Http(answered)) = refused else {
        panic!("a device that is no member of this one should have no socket to relay");
    };

    assert_eq!(answered.status(), StatusCode::NOT_FOUND);
}
