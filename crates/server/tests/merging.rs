//! The **Merged List**: one sidebar for the cluster, answered out of the lists
//! the device the browser opened holds of its members (ADR-0020, *The opened
//! device relays*).
//!
//! **Two Verksteads, a real dial and a real stream.** `tests/relaying.rs` is the
//! hop a browser makes through this device and `tests/freshness.rs` is the news
//! coming back over it; this is the list drawn out of both. So nothing here
//! hands A a list of B's by the back door: A holds B's own
//! `/api/ui/conversations` over the Peer Listener, refreshed off the Nudge
//! stream it holds to B, and what every assertion reads is A's own
//! `/api/ui/conversations` — the one an open sidebar asks for.
//!
//! **And both of A's routers stand over one state**, which is what a running
//! server serves: the sidebar the human's browser asks for is merged, and the
//! one a member asks for over the link is A's own rows alone. Two routers over
//! two states would be two devices asked the one question, each holding its own
//! memory of the cluster — see `routers_answering_devices_telling`.
//!
//! **The one thing said by hand is B's own announcement.** A Conversation
//! started on B is a row written to B's store here rather than a branch cut in a
//! worktree, so what stands in for B's `POST /api/ui/conversations` is the store
//! write and the Nudge that handler sends — announced on B's own channel, which
//! is the channel its Nudge stream carries. Everything after it is the
//! mechanism: A's stream hears it, A announces it under B's Device Id, and A's
//! refresher reads B's list over the link.

use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::http::header::CONTENT_TYPE;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use sqlx::SqlitePool;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use tokio::task::JoinHandle;
use tower::ServiceExt;
use verkstead_render::{ConversationEntry, ShowingArchived};
use verkstead_schema::Nudge;
use verkstead_server::device::reading::Reading;
use verkstead_server::device::{Device, Devices};
use verkstead_server::nudge::Nudges;
use verkstead_server::onboarding::Machine;
use verkstead_server::peer::joining::Joins;
use verkstead_server::peer::{self, Members};
use verkstead_server::platform::{self, Platform};
use verkstead_server::remote::Tailscale;
use verkstead_server::{Routers, open_database, routers_answering_devices_telling, store};
use verkstead_store::{Linking, record_member};

/// The device the browser is on, which holds the lists and does the merging.
const A: &str = "aa00bb11cc22dd33ee44ff5566778899";

/// And the device whose work is merged into them.
const B: &str = "0011223344556677889900aabbccddee";

/// And the same device under an id that sorts *after* A's, which is the one
/// thing a Device Id decides: two rows at one key are ordered by it, so this is
/// the member whose row sits under A's rather than over it — see [`pair`].
const LATER: &str = "ff11223344556677889900aabbccddee";

/// What B is written down as on A: the name and the OS word A holds against B's
/// row, which is what a merged row of B's says.
///
/// Stated rather than read off a machine, because they are the membership row's
/// — a member's name and OS are what it last said about itself at an exchange,
/// and nothing in a cluster asks a device for them again. And deliberately not
/// this box's own words: every device in this suite is one machine, so a row
/// that took the hub's hostname for a member's would read right for the wrong
/// reason.
const B_MACHINE: &str = "the-laptop";
const B_OS: &str = "macOS 15.1";

/// The sidebar itself, which is the path every assertion here reads.
const SIDEBAR: &str = "/api/ui/conversations";

/// And where the **Show archived conversations** switch is read and moved: one
/// switch for the whole merged list, held by the device the browser opened.
const ARCHIVES: &str = "/api/ui/conversations/archived";

/// How long one address has to answer here, rather than the two seconds a
/// running server gives one.
const PATIENCE: Duration = Duration::from_millis(300);

/// How long a test will wait for the merged list to say what it is waiting for.
/// Generous, because it is only ever paid in full when the assertion is about to
/// fail — what it waits on is a Nudge crossing a link and a list read back over
/// one, both on the loopback.
const WAITING: Duration = Duration::from_secs(10);

/// And how often it looks while it waits.
const LOOKING: Duration = Duration::from_millis(50);

/// And how long a test waits to be sure a Nudge is *not* coming.
///
/// Far longer than the hop it is watching for: a word that crossed the link is
/// announced by the far end the instant it comes off the stream, which is one
/// loopback hop rather than a read of anything.
const SETTLING: Duration = Duration::from_secs(2);

/// One Verkstead: its store, its identity, the Nudge channel its own pages and
/// its members both read, and both of the routers standing over its one state.
struct Verkstead {
    device: Device,
    pool: SqlitePool,

    /// What this device announces on — the one handle behind both of its
    /// routers, as it is in a running server.
    nudges: Nudges,

    /// Its cluster, kept for the one thing that is not a router: the Nudge
    /// streams it holds to its members.
    cluster: Devices,

    /// The workbench its own browser asks, which is where the merged list is
    /// read.
    workbench: Router,

    /// Where the other device dials it, on a port the machine picked.
    address: SocketAddr,

    /// Held for the length of the test: the identity and the database live in
    /// it.
    _dir: tempfile::TempDir,
}

impl Verkstead {
    /// A Verkstead with a store of its own, an identity in it, and its Peer
    /// Listener up — both routers over the one state, the peer half behind the
    /// Member Gate.
    async fn answering(id: &str) -> Verkstead {
        let dir = tempfile::tempdir().unwrap();
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
        } = routers_answering_devices_telling(
            pool.clone(),
            cluster.clone(),
            nudges.clone(),
            Machine::here(),
        );

        tokio::spawn(listener.serving(peer::router(
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
            _dir: dir,
        }
    }

    /// Hold a Nudge stream to every member of this device's cluster, which is
    /// what a running server spawns at the start.
    ///
    /// The handle is kept by the caller so the streams are let go of when the
    /// test ends rather than left dialling a listener that has gone.
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

    /// Write `other` down as one of this device's members, at `addresses`, under
    /// the name and OS word it is to be shown as.
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

    /// One Repo in its store, for the Conversations below to be against.
    async fn holding_a_repo(&self) -> i64 {
        store::register_repo(&self.pool, Path::new("/srv/verkstead"), "verkstead", "main")
            .await
            .unwrap()
            .expect("nothing is registered at that path yet")
            .id
    }

    /// A Conversation started on it, and the Nudge its own handler sends about
    /// one — which is what its Nudge stream carries to whoever holds one.
    async fn starts(&self, repo: i64, branch: &str) -> i64 {
        let conversation = store::start_conversation(&self.pool, repo, branch, self.device.id())
            .await
            .unwrap()
            .expect("the Repo is registered");

        self.nudges.announce(Nudge::Conversations);

        conversation
    }

    /// One of its Conversations closed and put away on it, and the same Nudge
    /// its own handlers send about either.
    ///
    /// Closed first because that is what archiving is offered on, and by the
    /// store rather than through the endpoint for [`starts`]'s reason: a close
    /// through the endpoint would be ending a session and a worktree that were
    /// never made.
    async fn puts_away(&self, conversation: i64) {
        store::close_conversation(&self.pool, conversation)
            .await
            .unwrap();

        store::archive_conversation(&self.pool, conversation)
            .await
            .unwrap();

        self.nudges.announce(Nudge::Conversations);
    }

    /// And taken back out again.
    async fn takes_back(&self, conversation: i64) {
        store::unarchive_conversation(&self.pool, conversation)
            .await
            .unwrap();

        self.nudges.announce(Nudge::Conversations);
    }

    /// Where its **Show archived conversations** switch is put, which is the
    /// press a browser makes on the switch under the sidebar.
    async fn switches_archives(&self, showing: bool) {
        press(
            &self.workbench,
            ARCHIVES,
            Some(&format!(r#"{{"showing":{showing}}}"#)),
        )
        .await;
    }

    /// And what that switch reads back: where it stands, and whether there is
    /// anything anywhere in the cluster behind it.
    async fn archives(&self) -> ShowingArchived {
        reading(&self.workbench, ARCHIVES).await
    }

    /// The same once it says what the test is waiting for, for
    /// [`sidebar_saying`]'s reason: a member's archives are read back over the
    /// link the way its list is.
    async fn archives_saying(&self, what: impl Fn(&ShowingArchived) -> bool) -> ShowingArchived {
        let mut last = ShowingArchived {
            showing: false,
            any: false,
        };

        let waited = tokio::time::timeout(WAITING, async {
            loop {
                last = self.archives().await;

                if what(&last) {
                    return last;
                }

                tokio::time::sleep(LOOKING).await;
            }
        })
        .await;

        waited.unwrap_or_else(|_| panic!("the switch never said it: {last:#?}"))
    }

    /// A Conversation of somebody else's copied onto this device the way a
    /// transfer copies one: this device's own row, numbered here, carrying the
    /// **birth key** the work was drafted under and the rank it already had.
    ///
    /// Written to the store rather than through an endpoint, for the reason
    /// [`starts`] is: what puts a copy here is a transfer, and a transfer is the
    /// stages after this one. What is being asked here is what the merged list
    /// makes of the rows it leaves behind.
    async fn takes_a_copy(&self, repo: i64, branch: &str, born: &store::Birth, rank: &str) -> i64 {
        let copy = store::start_conversation(&self.pool, repo, branch, self.device.id())
            .await
            .unwrap()
            .expect("the Repo is registered");

        store::record_birth(&self.pool, copy, born).await.unwrap();
        store::rank_conversation(&self.pool, copy, rank)
            .await
            .unwrap();

        self.nudges.announce(Nudge::Conversations);

        copy
    }

    /// And one of its own Conversations marked as handed on: the copy stays,
    /// saying which device holds the record and the id it goes by there.
    async fn hands_on(&self, conversation: i64, to: &str, there: i64) {
        store::transfer_away(
            &self.pool,
            conversation,
            &store::Transferred {
                device: to.to_owned(),
                id: there,
            },
        )
        .await
        .unwrap();

        self.nudges.announce(Nudge::Conversations);
    }

    /// The key one of its Conversations was born under.
    async fn born(&self, conversation: i64) -> store::Birth {
        store::birth(&self.pool, conversation)
            .await
            .unwrap()
            .expect("every Conversation is stamped as it is started")
    }

    /// The sidebar as this device's own browser reads it.
    async fn sidebar(&self) -> Vec<ConversationEntry> {
        rows(&self.workbench, SIDEBAR).await
    }

    /// And what a member asked of it gets, which is the same endpoint over the
    /// other listener — reached through `member`'s own relay, so the call
    /// crosses a real handshake and lands behind the Member Gate.
    async fn sidebar_over_the_link(&self, member: &Verkstead) -> Vec<ConversationEntry> {
        rows(&member.workbench, &through(self.device.id(), SIDEBAR)).await
    }

    /// Where the human has just dropped one row of the merged list, which is the
    /// press a browser makes on letting go of a card: the row that moved and the
    /// row it landed under, each named by device and id — `None` being this
    /// device's own, and no row at all being the top of the list.
    ///
    /// Answered with whatever the endpoint said, refusals included: a drag that
    /// could not be saved is a sentence the sidebar draws under the list.
    async fn drags(
        &self,
        row: (Option<&str>, i64),
        below: Option<(Option<&str>, i64)>,
    ) -> (StatusCode, String) {
        let saying = serde_json::json!({
            "row": named(row),
            "below": below.map(named),
        });

        let answered = self
            .workbench
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("{SIDEBAR}/rank"))
                    .header(CONTENT_TYPE, "application/json")
                    .body(Body::from(serde_json::to_vec(&saying).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let status = answered.status();
        let bytes = answered.into_body().collect().await.unwrap().to_bytes();

        (status, String::from_utf8_lossy(&bytes).into_owned())
    }

    /// The same, asserted to have been taken.
    async fn drops(&self, row: (Option<&str>, i64), below: Option<(Option<&str>, i64)>) {
        let (status, said) = self.drags(row, below).await;

        assert_eq!(
            status,
            StatusCode::NO_CONTENT,
            "the drag was refused: {said}"
        );
    }

    /// The rank one row of the merged list carries, by the branch it is on.
    async fn rank_on(&self, branch: &str) -> String {
        self.sidebar()
            .await
            .into_iter()
            .find(|row| row.branch == branch)
            .unwrap_or_else(|| panic!("{branch} is on the merged list"))
            .rank
    }

    /// The merged list once it says what the test is waiting for, or a panic
    /// saying what it did say instead.
    ///
    /// A poll rather than a wait on a signal, because what is being waited for is
    /// two hops away: B's store settles, B's stream says so, A announces it under
    /// B's id, and A's refresher reads B's list over the link. What is asked
    /// throughout is the question the browser asks.
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

        waited.unwrap_or_else(|_| panic!("the merged list never said it: {last:#?}"))
    }
}

/// The streams one device is holding, given back when the test drops it.
struct Holding(JoinHandle<()>);

impl Drop for Holding {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// A machine with no Tailscale on it: `verkstead-no-such-tailscale` is a program
/// that is not there, which is what having none *is*.
fn no_tailscale() -> Tailscale {
    Tailscale::running(vec!["verkstead-no-such-tailscale".to_owned()], 8422)
}

/// What a device says about the machine it is on, which is what its *own* rows
/// carry: the hostname and the OS word, read at the moment the list is drawn.
///
/// Read here the way the server reads it rather than stated, because that is the
/// claim — A's own rows say A's own machine, and nothing about them comes off a
/// membership row.
fn this_machine() -> (String, String) {
    (
        platform::hostname(),
        platform::os_word(Platform::Linux, None),
    )
}

/// A and B, linked both ways, with A's row for B at whatever `reaching` says
/// instead of B's own address.
///
/// Both ways because a link is both ways: A dials B pinned on B's fingerprint,
/// and B admits A because A's certificate is one it holds a membership for.
async fn linked(reaching: impl Fn(&Verkstead) -> String) -> (Verkstead, Verkstead) {
    pair(B, reaching).await
}

/// The same, with the member under a Device Id of the caller's choosing.
///
/// Which matters for exactly one question: **which of two rows at one key is the
/// lower**. A rank is the key and then the device, so the pair's order is the
/// two ids' order — and [`B`] sorts above [`A`], so the lower of the pair at the
/// top of this suite's lists is always A's own row. [`LATER`] is a member the
/// other way round, which is what puts a member's row under A's and so puts the
/// gap-opening write over the link.
async fn pair(member: &str, reaching: impl Fn(&Verkstead) -> String) -> (Verkstead, Verkstead) {
    let a = Verkstead::answering(A).await;
    let b = Verkstead::answering(member).await;

    a.linked_to(&b.device, B_MACHINE, B_OS, vec![reaching(&b)])
        .await;

    let (machine, os) = this_machine();
    b.linked_to(&a.device, &machine, &os, vec![a.at()]).await;

    (a, b)
}

/// The same, with A reaching B at B's own address, which is every test here but
/// the one about a member that stops answering.
async fn linked_up() -> (Verkstead, Verkstead) {
    linked(Verkstead::at).await
}

/// The address A reaches B at, as something that can be **switched off**: a
/// socket on the loopback carrying bytes to the far end and back, which lets go
/// of every connection it is holding when it is told to and answers nothing at
/// all after that.
///
/// The same stand-in `tests/freshness.rs` takes, and for the same reason: the
/// bytes crossing it are B's own TLS, so the certificate A pins is still B's —
/// what this tests is a machine going away rather than a fingerprint changing.
/// Where that file's link is *cut* and comes back, this one stays down: what is
/// being asked here is what A's list holds of a laptop whose lid is shut.
struct Link {
    address: SocketAddr,
    cutting: broadcast::Sender<()>,

    /// Whether it is down, read by the accept loop so that a connection made
    /// after the switch-off is refused rather than carried.
    off: Arc<AtomicBool>,
}

impl Link {
    /// A link to `far`, open until it is switched off.
    async fn to(far: SocketAddr) -> Link {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let (cutting, _) = broadcast::channel(1);
        let cut = cutting.clone();
        let off = Arc::new(AtomicBool::new(false));
        let down = Arc::clone(&off);

        tokio::spawn(async move {
            while let Ok((mut near, _)) = listener.accept().await {
                // Accepted and dropped, which is what a dial to a machine that
                // has gone finds: something answered the port and then nothing.
                if down.load(Ordering::SeqCst) {
                    continue;
                }

                let mut cut = cut.subscribe();

                tokio::spawn(async move {
                    let Ok(mut across) = TcpStream::connect(far).await else {
                        return;
                    };

                    tokio::select! {
                        _ = tokio::io::copy_bidirectional(&mut near, &mut across) => {}
                        _ = cut.recv() => {}
                    }
                });
            }
        });

        Link {
            address,
            cutting,
            off,
        }
    }

    fn at(&self) -> String {
        format!("127.0.0.1:{}", self.address.port())
    }

    /// B is switched off: the connections it is holding go, and nothing it is
    /// dialled on afterwards is carried anywhere.
    fn off(&self) {
        self.off.store(true, Ordering::SeqCst);

        self.cutting
            .send(())
            .expect("the link holds its own receiver");
    }
}

/// Where a local call stands when it is for `device` instead: the prefix takes
/// the place of `/api/ui`, and everything under it is untouched.
fn through(device: &str, path: &str) -> String {
    let leaf = path
        .strip_prefix("/api/ui")
        .expect("a relayed call is a call into the viewer's own namespace");

    format!("/api/ui/members/{device}{leaf}")
}

/// A read of a list of rows, made the way the browser makes one.
async fn rows(app: &Router, path: &str) -> Vec<ConversationEntry> {
    reading(app, path).await
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

/// And a press on `path`, with a body where the endpoint takes one: what it
/// answered, once it has answered something that is not a refusal.
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

/// An open page, listening on this device's own Nudge stream — which is what a
/// browser is, and the one reader that cannot poll.
///
/// Every other assertion here reads the sidebar until it says what it is waiting
/// for, which is the right shape for a list that arrives over a link. It is also
/// exactly what a browser does not do: a page reads the list back when it is
/// told to and at no other time. So one test reads the stream instead — see
/// [`the_sidebar_is_told_again_once_a_members_list_has_landed`].
struct Listening {
    body: Body,
    buffered: String,
}

impl Listening {
    async fn open(app: &Router) -> Listening {
        let answered = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/ui/nudges")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(answered.status(), StatusCode::OK);

        Listening {
            body: answered.into_body(),
            buffered: String::new(),
        }
    }

    /// The next whole frame off the stream, whatever kind it is.
    async fn frame(&mut self) -> String {
        loop {
            if let Some(end) = self.buffered.find("\n\n") {
                return self.buffered.drain(..end + 2).collect();
            }

            let chunk = self
                .body
                .frame()
                .await
                .expect("the stream ended")
                .unwrap()
                .into_data()
                .expect("the stream carries data frames");

            self.buffered.push_str(std::str::from_utf8(&chunk).unwrap());
        }
    }

    /// Insist that no Nudge saying `device`'s Conversations moved ever arrives.
    ///
    /// What it is watching for is the word this device tells its own pages having
    /// crossed the link — see
    /// [`the_word_that_a_members_list_has_landed_stops_at_this_device`]. A device
    /// that heard it would announce it under the device it came from, which is
    /// this frame, and nothing else in the test that asks makes one.
    async fn no_word_of(&mut self, device: &str) {
        let looking = serde_json::json!({ "device": device, "kind": "conversations" });

        let arrived = tokio::time::timeout(SETTLING, async {
            loop {
                let frame = self.frame().await;

                if frame.starts_with("event: nudge") && said(&frame) == looking {
                    return frame;
                }
            }
        })
        .await;

        assert!(
            arrived.is_err(),
            "the word that a member's list had landed crossed the link: {arrived:?}",
        );
    }

    /// The next Nudge this device announced about its **own** world: one naming a
    /// device is a member's news said again, and a page hearing that has nothing
    /// new of this device's to read yet.
    async fn own_nudge(&mut self) -> serde_json::Value {
        let waited_for = tokio::time::timeout(WAITING, async {
            loop {
                let frame = self.frame().await;

                if !frame.starts_with("event: nudge") {
                    continue;
                }

                let said = said(&frame);

                if said.get("device").is_none_or(serde_json::Value::is_null) {
                    return said;
                }
            }
        });

        waited_for.await.expect("waited for a Nudge in vain")
    }
}

/// What one frame of that stream said: the JSON of its `data` line, read as the
/// page reads it.
fn said(frame: &str) -> serde_json::Value {
    let data = frame
        .lines()
        .find_map(|line| line.strip_prefix("data: "))
        .unwrap_or_else(|| panic!("a Nudge frame carries a data line, not {frame:?}"));

    serde_json::from_str(data)
        .unwrap_or_else(|why| panic!("a Nudge should be readable as one: {data:?} — {why}"))
}

/// One row of the merged list as a drag names one: the device and the id.
fn named(row: (Option<&str>, i64)) -> serde_json::Value {
    serde_json::json!({ "device": row.0, "id": row.1 })
}

/// Which row is which, for an assertion to name one by: the branch it is on.
fn branches(rows: &[ConversationEntry]) -> Vec<&str> {
    rows.iter().map(|row| row.branch.as_str()).collect()
}

/// And whose each row is, by the Device Id it names — `None` being this device's
/// own.
fn whose(rows: &[ConversationEntry]) -> Vec<Option<&str>> {
    rows.iter()
        .map(|row| {
            row.device
                .as_ref()
                .expect("every row of a merged list says whose it is")
                .id
                .as_deref()
        })
        .collect()
}

/// A Conversation created on B is on A's sidebar within a Nudge, in rank order
/// among A's own rows — and the row says it is B's, by B's own name and OS word.
#[tokio::test]
async fn a_conversation_started_on_a_member_arrives_on_the_merged_list() {
    let (a, b) = linked_up().await;
    let _holding = a.holding();

    let here = a.holding_a_repo().await;
    let there = b.holding_a_repo().await;

    a.starts(here, "the-merged-list").await;

    // Nothing of B's yet, so A's list is A's own work — and every row of it says
    // whose it is all the same, there being somebody else in the cluster to be
    // distinguished from.
    let alone = a
        .sidebar_saying(|rows| branches(rows) == ["the-merged-list"])
        .await;

    let (machine, os) = this_machine();
    let mine = alone[0].device.as_ref().expect("A is in a cluster");

    assert_eq!(mine.id, None, "A's own rows name no device");
    assert_eq!(mine.name, machine, "and say A's own machine");
    assert_eq!(mine.os, os);
    assert!(
        mine.reachable,
        "a device answering its own browser is there"
    );

    // And then one on B, which arrives on A's list without A being asked to read
    // anything: B announces, A's stream carries it, A reads B's list back.
    b.starts(there, "the-cross-device-drag").await;

    let merged = a.sidebar_saying(|rows| rows.len() == 2).await;

    // In rank order across the lot: a start ranks above everything, so B's newer
    // Conversation is above A's — which is where B's own sidebar has it and where
    // A's has A's.
    assert_eq!(
        branches(&merged),
        ["the-cross-device-drag", "the-merged-list"],
    );

    let theirs = merged[0].device.as_ref().expect("A is in a cluster");

    assert_eq!(
        theirs.id.as_deref(),
        Some(B),
        "a member's row names its device",
    );
    assert_eq!(theirs.name, B_MACHINE, "and the member's own machine");
    assert_eq!(theirs.os, B_OS, "and the member's own OS word");
    assert!(theirs.reachable);
}

/// And the page is told again once B's list has actually landed, which is the
/// only moment there is anything new for it to read.
///
/// **The test above polls and a browser does not.** What sets A's read of B's
/// list going is B's own Nudge, said again on A's channel under B's Device Id —
/// which every open sidebar hears at the same instant and re-reads the merge on.
/// That read is a local call and A's read of B's list is a dial across the room,
/// so the page draws the merge as it stood a moment before. What puts it right is
/// A announcing on its own account once it has kept what B said: the one Nudge
/// here with no device on it, and the one a page can read the new row back on.
#[tokio::test]
async fn the_sidebar_is_told_again_once_a_members_list_has_landed() {
    let (a, b) = linked_up().await;
    let _holding = a.holding();

    let here = a.holding_a_repo().await;
    let there = b.holding_a_repo().await;

    a.starts(here, "the-merged-list").await;
    a.sidebar_saying(|rows| rows.len() == 1).await;

    // An open page on A, listening from before anything happens on B.
    let mut page = Listening::open(&a.workbench).await;

    b.starts(there, "the-cross-device-drag").await;

    // And the page reads once each time it is told, which is the whole of what a
    // browser does about a Nudge — no polling of its own, and no second look
    // between them.
    //
    // **Which of A's own Nudges carries it is not the claim.** A holds B's list
    // again for each thing that asks it to — the stream being taken up, and then
    // B's own news — and it says so on its own account every time it has kept
    // one, so a page that opened between two of those reads is told twice and
    // finds the row on the later one. What is being tested is that it is told at
    // all: without that word there is no Nudge a page could ever read the row
    // back on, whatever it did.
    let read_back = tokio::time::timeout(WAITING, async {
        loop {
            let told = page.own_nudge().await;

            assert_eq!(
                told.get("kind").and_then(serde_json::Value::as_str),
                Some("conversations"),
                "the sidebar's own list is what moved: {told}",
            );

            let rows = a.sidebar().await;

            if branches(&rows) == ["the-cross-device-drag", "the-merged-list"] {
                return;
            }
        }
    })
    .await;

    assert!(
        read_back.is_ok(),
        "no Nudge of A's own left the page a list with the member's row on it",
    );
}

/// And that word stops at this device: it is told to the pages in front of it and
/// to no member of its cluster.
///
/// **Because sent, it would go round for ever.** It names no device, so on the
/// stream a member holds it would read as *this device's Conversations moved* —
/// that member would announce it under this device, re-read this device's list,
/// and tell its own pages, which is the word back over here, a relayed read
/// apiece every time. And a member has no use for it either way: its own list is
/// its own, and this is only this device's account of it catching up.
///
/// Watched from B, which is where it would land: B holds a stream to A, so a word
/// of A's that crossed is one B announces under A. Nothing else here makes that
/// frame — A starts nothing and is pressed for nothing, and all it does is read
/// the list B has just moved.
#[tokio::test]
async fn the_word_that_a_members_list_has_landed_stops_at_this_device() {
    let (a, b) = linked_up().await;
    let _here = a.holding();
    let _there = b.holding();

    let there = b.holding_a_repo().await;

    let mut page = Listening::open(&b.workbench).await;

    b.starts(there, "the-merged-list").await;

    // A hears that, reads B's list, and tells its own pages — which is the word
    // under test, and by the time the row is on A's sidebar it has been said.
    a.sidebar_saying(|rows| rows.len() == 1).await;

    page.no_word_of(A).await;
}

/// Two Conversations the two devices each numbered 1 are two rows, and what
/// tells them apart is the device each names.
#[tokio::test]
async fn one_number_on_two_devices_is_two_rows() {
    let (a, b) = linked_up().await;
    let _holding = a.holding();

    let here = a.holding_a_repo().await;
    let there = b.holding_a_repo().await;

    assert_eq!(a.starts(here, "here-first").await, 1);
    assert_eq!(b.starts(there, "there-first").await, 1);

    let merged = a.sidebar_saying(|rows| rows.len() == 2).await;

    assert_eq!(
        merged.iter().map(|row| row.id).collect::<Vec<_>>(),
        [1, 1],
        "both devices numbered their first Conversation 1",
    );
    assert_eq!(whose(&merged), [Some(B), None]);

    // And the two ranks are distinct, which is what makes the order above an
    // order rather than a tie: the devices minted the same key for their first
    // row, and the suffix is the whole of what parts them (ADR-0020, *Ranks*).
    assert_ne!(merged[0].rank, merged[1].rank);
    assert!(merged[0].rank.ends_with(&format!("-{B}")));
    assert!(merged[1].rank.ends_with(&format!("-{A}")));
}

/// A member that stops answering keeps its rows, from the last list held, and
/// they carry the flag that says the device is not there.
#[tokio::test]
async fn a_member_that_stops_answering_keeps_its_rows_dimmed() {
    let b = Verkstead::answering(B).await;
    let link = Link::to(b.address).await;

    let a = Verkstead::answering(A).await;
    a.linked_to(&b.device, B_MACHINE, B_OS, vec![link.at()])
        .await;

    let (machine, os) = this_machine();
    b.linked_to(&a.device, &machine, &os, vec![a.at()]).await;

    let _holding = a.holding();

    let there = b.holding_a_repo().await;
    b.starts(there, "the-push-relay").await;

    let merged = a
        .sidebar_saying(|rows| branches(rows) == ["the-push-relay"])
        .await;

    assert!(merged[0].device.as_ref().unwrap().reachable);

    // B goes away. What marks the row unreachable is a dial that answered
    // nowhere, which is what the browser's next call through the hop is.
    link.off();

    let refused = a
        .workbench
        .clone()
        .oneshot(
            Request::builder()
                .uri(through(B, SIDEBAR))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        refused.status(),
        StatusCode::BAD_GATEWAY,
        "a member that is not there is a hop that cannot be made",
    );

    let dimmed = a
        .sidebar_saying(|rows| {
            rows.first()
                .and_then(|row| row.device.as_ref())
                .is_some_and(|whose| !whose.reachable)
        })
        .await;

    assert_eq!(
        branches(&dimmed),
        ["the-push-relay"],
        "the rows stay, from the last list held",
    );
    assert_eq!(whose(&dimmed), [Some(B)]);
}

/// A member never reached contributes no rows at all, there being nothing held
/// of it to draw — while it is a member all the same, which is what puts a device
/// on this one's own rows.
#[tokio::test]
async fn a_member_never_reached_contributes_nothing() {
    let a = Verkstead::answering(A).await;
    let b = Verkstead::answering(B).await;

    // A's row for B names an address nothing is listening on, so no dial of A's
    // ever gets through.
    a.linked_to(&b.device, B_MACHINE, B_OS, vec!["127.0.0.1:1".to_owned()])
        .await;

    let _holding = a.holding();

    let here = a.holding_a_repo().await;
    a.starts(here, "the-merged-list").await;

    let merged = a.sidebar_saying(|rows| !rows.is_empty()).await;

    assert_eq!(branches(&merged), ["the-merged-list"]);
    assert_eq!(
        whose(&merged),
        [None],
        "A's own row, and nothing of a member A has never reached",
    );
}

/// And A asked for the sidebar over the Peer Listener answers A's own rows
/// alone, with no device on them — whatever it is holding of its members.
#[tokio::test]
async fn a_member_reading_the_sidebar_gets_this_devices_own_rows_alone() {
    let (a, b) = linked_up().await;
    let _holding = a.holding();

    let here = a.holding_a_repo().await;
    let there = b.holding_a_repo().await;

    a.starts(here, "the-merged-list").await;
    b.starts(there, "the-cross-device-drag").await;

    // A's own browser sees both, so the lists really are held: what the link
    // answers below is a narrowing rather than an emptiness.
    let merged = a.sidebar_saying(|rows| rows.len() == 2).await;
    assert_eq!(
        branches(&merged),
        ["the-cross-device-drag", "the-merged-list"],
    );

    let over_the_link = a.sidebar_over_the_link(&b).await;

    assert_eq!(
        branches(&over_the_link),
        ["the-merged-list"],
        "a member is answered this device's own rows alone",
    );
    assert_eq!(
        over_the_link[0].device, None,
        "and no device on them: whose they are is the reader's to say",
    );
    assert!(
        over_the_link[0].rank.ends_with(&format!("-{A}")),
        "while the rank rides out, which is what the reader merges by",
    );
}

/// The hub's switch governs the whole merged list: turned on, a member's
/// archived Conversations are on it in their ordinary places, and turned off
/// they are not — with the member's own switch untouched throughout.
#[tokio::test]
async fn the_hubs_switch_governs_a_members_archived_rows() {
    let (a, b) = linked_up().await;
    let _holding = a.holding();

    let there = b.holding_a_repo().await;

    // Three of B's, in the order a start ranks them: the one put away lands
    // between the two that are not, which is where *its ordinary place* is.
    b.starts(there, "under").await;
    let away = b.starts(there, "away").await;
    b.starts(there, "over").await;

    b.puts_away(away).await;

    // A's switch is off, which is where every switch starts.
    let hidden = a
        .sidebar_saying(|rows| branches(rows) == ["over", "under"])
        .await;

    assert_eq!(whose(&hidden), [Some(B), Some(B)]);

    a.switches_archives(true).await;

    let shown = a.sidebar_saying(|rows| rows.len() == 3).await;

    assert_eq!(
        branches(&shown),
        ["over", "away", "under"],
        "a member's archived row is on the merged list in its ordinary place",
    );

    // And the member's own switch has not moved: that row is B's standing choice
    // for the browser in front of B, and A asking for its rows is not A writing
    // it.
    assert!(
        !store::showing_archived(&b.pool).await.unwrap(),
        "the hub asks with its position rather than putting the member's switch there",
    );
    assert_eq!(
        branches(&b.sidebar().await),
        ["over", "under"],
        "so B's own sidebar is still drawn at B's own switch",
    );

    a.switches_archives(false).await;

    let hidden = a.sidebar_saying(|rows| rows.len() == 2).await;

    assert_eq!(branches(&hidden), ["over", "under"]);
    assert!(!store::showing_archived(&b.pool).await.unwrap());
}

/// Whether there is anything archived at all folds across the cluster: a device
/// with nothing of its own draws the switch while a member has something behind
/// it, and draws none when nothing anywhere does.
#[tokio::test]
async fn a_device_with_nothing_archived_draws_the_switch_for_a_member() {
    let (a, b) = linked_up().await;
    let _holding = a.holding();

    let here = a.holding_a_repo().await;
    let there = b.holding_a_repo().await;

    a.starts(here, "the-archived-switch-governs").await;
    let away = b.starts(there, "finished-with").await;

    // Both devices' rows are on the list, and nothing anywhere has been put
    // away: there is no switch worth drawing.
    a.sidebar_saying(|rows| rows.len() == 2).await;

    assert!(
        !a.archives().await.any,
        "nothing is archived on either device",
    );

    b.puts_away(away).await;

    a.archives_saying(|archives| archives.any).await;

    // A's own record still says nothing of A's is archived, which is the half
    // the member's answer is folded into rather than replacing.
    assert!(!store::any_archived(&a.pool).await.unwrap());

    // And a member asking the same question over the link is answered A's own
    // half alone: a member that folded its own members in would be folding the
    // hub that asked, and the two would read each other round for ever.
    let over_the_link: ShowingArchived =
        reading(&b.workbench, &through(a.device.id(), ARCHIVES)).await;

    assert!(
        !over_the_link.any,
        "over the link a device answers for itself",
    );

    // Taken back out on B, and the switch has nothing behind it again.
    b.takes_back(away).await;

    a.archives_saying(|archives| !archives.any).await;
}

/// And archiving a member's Conversation from the merged list takes its row off
/// the list, while unarchiving it puts the row back.
#[tokio::test]
async fn archiving_a_members_conversation_takes_its_row_off_the_merged_list() {
    let (a, b) = linked_up().await;
    let _holding = a.holding();

    let there = b.holding_a_repo().await;
    let away = b.starts(there, "finished-with").await;
    b.starts(there, "still-going").await;

    // Closed on B, that being what archiving is offered on.
    store::close_conversation(&b.pool, away).await.unwrap();

    a.sidebar_saying(|rows| rows.len() == 2).await;

    // The press the card's menu makes on a member's row: through A, to the device
    // that owns it.
    let said = press(
        &a.workbench,
        &through(B, &format!("{SIDEBAR}/{away}/archive")),
        None,
    )
    .await;

    assert_eq!(said, "\"Archived\"", "the owning device did the archiving");

    let left = a.sidebar_saying(|rows| rows.len() == 1).await;

    assert_eq!(branches(&left), ["still-going"]);

    let said = press(
        &a.workbench,
        &through(B, &format!("{SIDEBAR}/{away}/unarchive")),
        None,
    )
    .await;

    assert_eq!(said, "\"Unarchived\"");

    let back = a.sidebar_saying(|rows| rows.len() == 2).await;

    assert_eq!(branches(&back), ["still-going", "finished-with"]);
}

/// A member's row dragged between two of this device's holds, and it holds on
/// both devices: the rank is minted here off the merged list and written to the
/// device that owns the row, which reads its own list back afterwards saying the
/// same thing.
#[tokio::test]
async fn a_members_row_dragged_between_two_of_this_devices_holds_on_both() {
    let (a, b) = linked_up().await;
    let _here = a.holding();
    let _there = b.holding();

    let here = a.holding_a_repo().await;
    let there = b.holding_a_repo().await;

    a.starts(here, "a-one").await;
    a.starts(here, "a-two").await;
    a.starts(here, "a-three").await;
    let moved = b.starts(there, "b-one").await;

    let before = a.sidebar_saying(|rows| rows.len() == 4).await;

    assert_eq!(branches(&before), ["a-three", "a-two", "b-one", "a-one"]);

    let untouched: Vec<String> = ["a-one", "a-two", "a-three"]
        .into_iter()
        .map(|branch| {
            before
                .iter()
                .find(|row| row.branch == branch)
                .unwrap()
                .rank
                .clone()
        })
        .collect();

    // B's row, dropped under the top row of A's — which is between two of A's
    // own, and so a neighbour on either side that B has never heard of.
    a.drops((Some(B), moved), Some((None, id_on(&before, "a-three"))))
        .await;

    let after = a
        .sidebar_saying(|rows| branches(rows) == ["a-three", "b-one", "a-two", "a-one"])
        .await;

    // The rank that moved is B's own, whoever its neighbours belong to — which
    // is what makes it B's to keep.
    let minted = after
        .iter()
        .find(|row| row.branch == "b-one")
        .unwrap()
        .rank
        .clone();

    assert!(
        minted.ends_with(&format!("-{B}")),
        "the row that moved carries its own device: {minted}",
    );

    // And nothing of A's was written: the only device told anything is the one
    // that owns the row.
    for (branch, rank) in ["a-one", "a-two", "a-three"].into_iter().zip(untouched) {
        assert_eq!(
            after.iter().find(|row| row.branch == branch).unwrap().rank,
            rank,
            "{branch} was not rewritten",
        );
    }

    // And B says the same order, which is the whole claim: B's own store holds
    // the rank now, and B merges A's rows around it for itself.
    let over_there = b
        .sidebar_saying(|rows| branches(rows) == ["a-three", "b-one", "a-two", "a-one"])
        .await;

    assert_eq!(
        over_there
            .iter()
            .find(|row| row.branch == "b-one")
            .unwrap()
            .rank,
        minted,
        "the rank B is holding is the one A minted for it",
    );
}

/// And one of this device's dragged between two of a member's, which is the same
/// sentence the other way round: the neighbours are B's and the write is A's own
/// store.
#[tokio::test]
async fn one_of_this_devices_rows_dragged_between_two_of_a_members_holds_on_both() {
    let (a, b) = linked_up().await;
    let _here = a.holding();
    let _there = b.holding();

    let here = a.holding_a_repo().await;
    let there = b.holding_a_repo().await;

    b.starts(there, "b-one").await;
    let under = b.starts(there, "b-two").await;
    let moved = a.starts(here, "a-one").await;

    let before = a.sidebar_saying(|rows| rows.len() == 3).await;

    assert_eq!(branches(&before), ["b-two", "b-one", "a-one"]);

    let theirs: Vec<String> = before
        .iter()
        .filter(|row| row.branch.starts_with("b-"))
        .map(|row| row.rank.clone())
        .collect();

    a.drops((None, moved), Some((Some(B), under))).await;

    let after = a
        .sidebar_saying(|rows| branches(rows) == ["b-two", "a-one", "b-one"])
        .await;

    assert!(
        after
            .iter()
            .find(|row| row.branch == "a-one")
            .unwrap()
            .rank
            .ends_with(&format!("-{A}")),
        "A's row carries A's own device",
    );
    assert_eq!(
        after
            .iter()
            .filter(|row| row.branch.starts_with("b-"))
            .map(|row| row.rank.clone())
            .collect::<Vec<_>>(),
        theirs,
        "and nothing of B's was written",
    );

    let over_there = b
        .sidebar_saying(|rows| branches(rows) == ["b-two", "a-one", "b-one"])
        .await;

    assert_eq!(whose(&over_there), [None, Some(A), None]);
}

/// The two rows the devices minted at one key — their first Conversations — with
/// a card dropped between them: no rank sits there, so the lower of the pair is
/// re-ranked first and the card lands where it was dropped. And a second drop
/// into that same gap needs no gap opened, the pair no longer sharing a key.
#[tokio::test]
async fn a_row_dropped_between_two_rows_at_one_key_lands_there() {
    let (a, b) = linked_up().await;
    let _holding = a.holding();

    let here = a.holding_a_repo().await;
    let there = b.holding_a_repo().await;

    a.starts(here, "a-one").await;
    let moved = a.starts(here, "a-two").await;
    let top = b.starts(there, "b-one").await;

    let before = a.sidebar_saying(|rows| rows.len() == 3).await;

    assert_eq!(branches(&before), ["a-two", "b-one", "a-one"]);

    // The two first rows are the one key under two devices, which is what the
    // whole exception is about.
    let pair = (a.rank_on("b-one").await, a.rank_on("a-one").await);

    assert_eq!(
        pair.0.split_once('-').unwrap().0,
        pair.1.split_once('-').unwrap().0,
        "two devices ranking above their own top minted the same key: {pair:?}",
    );

    a.drops((None, moved), Some((Some(B), top))).await;

    let after = a
        .sidebar_saying(|rows| branches(rows) == ["b-one", "a-two", "a-one"])
        .await;

    assert_eq!(
        branches(&after),
        ["b-one", "a-two", "a-one"],
        "the card is where it was dropped",
    );

    // The lower of the pair moved down to make the room, and it is A's own row,
    // so A wrote twice and B not at all.
    let opened = a.rank_on("a-one").await;

    assert_ne!(opened, pair.1, "the lower of the pair was re-ranked");
    assert_eq!(a.rank_on("b-one").await, pair.0, "and the upper was not");
    assert!(
        opened.ends_with(&format!("-{A}")),
        "re-ranked through its own device: {opened}",
    );

    // And the second drop into that same gap: nothing is re-ranked, because the
    // pair is a pair no longer.
    let again = a.starts(here, "a-three").await;

    a.sidebar_saying(|rows| rows.len() == 4).await;
    a.drops((None, again), Some((Some(B), top))).await;

    let settled = a
        .sidebar_saying(|rows| branches(rows) == ["b-one", "a-three", "a-two", "a-one"])
        .await;

    assert_eq!(branches(&settled), ["b-one", "a-three", "a-two", "a-one"]);
    assert_eq!(
        a.rank_on("a-one").await,
        opened,
        "the second drop opened no gap: nothing under it moved",
    );
    assert_eq!(a.rank_on("b-one").await, pair.0);
}

/// And where the lower of the pair is a member's row and that member is not
/// answering, the gap cannot be opened — so the drag is refused, the sidebar is
/// told which device it was, and nothing at all is written.
#[tokio::test]
async fn a_gap_that_needs_a_member_that_is_not_there_is_refused_by_name() {
    let b = Verkstead::answering(LATER).await;
    let link = Link::to(b.address).await;

    let a = Verkstead::answering(A).await;
    a.linked_to(&b.device, B_MACHINE, B_OS, vec![link.at()])
        .await;

    let (machine, os) = this_machine();
    b.linked_to(&a.device, &machine, &os, vec![a.at()]).await;

    let _holding = a.holding();

    let here = a.holding_a_repo().await;
    let there = b.holding_a_repo().await;

    let first = a.starts(here, "a-one").await;
    let moved = a.starts(here, "a-two").await;
    b.starts(there, "b-one").await;

    let before = a.sidebar_saying(|rows| rows.len() == 3).await;

    // This member's id sorts after A's, so of the two rows at one key its is the
    // lower — which is the row that would have to be re-ranked.
    assert_eq!(branches(&before), ["a-two", "a-one", "b-one"]);

    let held = (a.rank_on("a-one").await, a.rank_on("a-two").await);

    link.off();

    let (status, said) = a.drags((None, moved), Some((None, first))).await;

    assert_eq!(
        status,
        StatusCode::BAD_GATEWAY,
        "a gap that cannot be opened is a drag that cannot be saved: {said}",
    );
    assert!(
        said.contains(B_MACHINE),
        "and the sidebar is told which device it was: {said}",
    );

    assert_eq!(
        (a.rank_on("a-one").await, a.rank_on("a-two").await),
        held,
        "and nothing was written",
    );
    assert_eq!(branches(&a.sidebar().await), ["a-two", "a-one", "b-one"]);
}

/// A neighbour that has gone since the list was drawn is not a refusal: there is
/// nothing left to rank against, so the order stays as the rest of the list says
/// and the press is taken.
#[tokio::test]
async fn a_neighbour_that_has_gone_is_still_taken() {
    let (a, b) = linked_up().await;
    let _holding = a.holding();

    let here = a.holding_a_repo().await;
    let there = b.holding_a_repo().await;

    let moved = a.starts(here, "a-one").await;
    b.starts(there, "b-one").await;

    let before = a.sidebar_saying(|rows| rows.len() == 2).await;

    assert_eq!(branches(&before), ["b-one", "a-one"]);

    a.drops((None, moved), Some((Some(B), 9_999))).await;

    assert_eq!(branches(&a.sidebar().await), ["b-one", "a-one"]);
}

/// Which row on the merged list is which, by the branch it is on.
fn id_on(rows: &[ConversationEntry], branch: &str) -> i64 {
    rows.iter()
        .find(|row| row.branch == branch)
        .unwrap_or_else(|| panic!("{branch} is on the merged list"))
        .id
}

/// A Conversation that has been transferred is one row of the merged list, drawn
/// wherever the live record is — and it is one row throughout, the copies sharing
/// a **birth key** being what says they are one piece of work (ADR-0020,
/// *Transfer*).
///
/// Read from both ends, because both are merges and each has the other's rows the
/// other way round: A holds the tombstone and draws B's copy, and B holds the
/// tombstone as a member's row and draws its own.
#[tokio::test]
async fn a_conversation_transferred_to_a_member_is_one_row_of_the_merged_list() {
    let (a, b) = linked_up().await;
    let _holding_here = a.holding();
    let _holding_there = b.holding();

    let here = a.holding_a_repo().await;
    let there = b.holding_a_repo().await;

    // A's own, and what a transfer of it would carry: the key it was born under
    // and the rank it sits at.
    let mine = a.starts(here, "the-transfer").await;
    a.sidebar_saying(|rows| branches(rows) == ["the-transfer"])
        .await;

    let born = a.born(mine).await;
    let rank = a.rank_on("the-transfer").await;

    // The copy the transfer writes on B, and a Conversation of B's own after it.
    // The second one is the sync point: A re-reads the whole of B's list on a
    // Nudge, so a list with the marker in it is a list with the copy in it — and
    // without one there is no telling a copy that has been merged away from a
    // copy that has not arrived yet.
    let theirs = b.takes_a_copy(there, "the-transfer", &born, &rank).await;
    b.starts(there, "b-own").await;

    // Both copies say they are live, which is the moment between B's write and
    // A's mark landing. One row all the same.
    let between = a
        .sidebar_saying(|rows| branches(rows).contains(&"b-own"))
        .await;

    assert_eq!(
        branches(&between),
        ["b-own", "the-transfer"],
        "one row for the work and one for B's own, rather than two copies drawn \
         beside each other",
    );
    assert_eq!(
        whose(&between),
        [Some(B), None],
        "and while nothing says otherwise it is A's own copy that stands",
    );

    // And then the mark lands: A's copy is not the record, whatever else is true
    // of it.
    a.hands_on(mine, B, theirs).await;

    let after = a
        .sidebar_saying(|rows| whose(rows) == [Some(B), Some(B)])
        .await;

    assert_eq!(
        branches(&after),
        ["b-own", "the-transfer"],
        "the tombstone is off the list and the live copy is on it, at the rank \
         the work has always sat at",
    );
    assert_eq!(
        after[1].id, theirs,
        "and the row is B's copy, by the id B numbered it",
    );
    assert_eq!(
        after[1].born,
        born.key(),
        "which is the row A's copy was born under: the ids are two and the work \
         is one",
    );

    // And the same list from B, where the tombstone is a member's row rather than
    // its own and the live copy is its own rather than a member's.
    let theirs = b
        .sidebar_saying(|rows| whose(rows) == [None, None] && rows.len() == 2)
        .await;

    assert_eq!(
        branches(&theirs),
        ["b-own", "the-transfer"],
        "B draws the work it is doing once, and nothing of the copy A kept",
    );
}
