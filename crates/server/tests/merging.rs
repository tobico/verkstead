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
        } = routers_answering_devices_telling(pool.clone(), cluster.clone(), nudges.clone());

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
    let a = Verkstead::answering(A).await;
    let b = Verkstead::answering(B).await;

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
