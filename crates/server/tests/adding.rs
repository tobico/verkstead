//! The press on a **Discovered** row: an **Add** that links two devices with
//! nothing typed (ADR-0020, *Discovery*).
//!
//! **The other half of `tests/discovering.rs`.** That suite is about which rows a
//! browse and a probe come to; this one is about what a press on one of them
//! does — a real join over a real socket, to the address the row was found at.
//! So B stands its peer listener up on the loopback and A is told it heard B
//! there, and A presses through the same workbench route the browser presses.
//!
//! **What was heard is stated rather than browsed**, for `tests/discovering.rs`'s
//! reason and one more: what is being asked here is what a press *dials*, and a
//! row that came off a multicast would be a test standing on whatever port the
//! daemon resolved a device at. Stated rows name a socket this file chose, which
//! is the socket B is really on.
//!
//! **Both devices answer**, the way they do in `tests/exchange.rs`: the last test
//! here presses Allow on B and reads both lists afterwards, and the dial back
//! that carries the roster is a dial. Which is also why each device advertises
//! the loopback address it really landed on — the real peer port would fight
//! whatever is already on 8423, and two of these tests at once would fight each
//! other.
//!
//! The machines are stated rather than read, for `tests/devices.rs`'s reason:
//! what the box running this happens to be is the one thing a test about two
//! machines cannot stand on.

use std::net::SocketAddr;
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use sqlx::SqlitePool;
use tower::ServiceExt;
use verkstead_render::{AskingDevice, DevicesView, DiscoveredDevice};
use verkstead_server::device::reading::Reading;
use verkstead_server::device::{Device, Devices};
use verkstead_server::discovery::{Browse, Found};
use verkstead_server::nudge::Nudges;
use verkstead_server::open_database;
use verkstead_server::peer::joining::Joins;
use verkstead_server::peer::{self, Members};
use verkstead_server::platform::{self, Platform};
use verkstead_server::remote::Tailscale;
use verkstead_server::router_answering_devices_telling;

/// Where the pane reads the two lists, where a press on a discovered row goes,
/// and where the far end's human answers it.
const DEVICES: &str = "/api/ui/devices";
const DISCOVERED: &str = "/api/ui/devices/discovered";
const ASKING: &str = "/api/ui/devices/asking";

/// The ids the two devices are stated as, so that what a test asserts against is
/// a string it chose rather than sixteen random bytes.
///
/// A is the one that presses, B the one it found. GONE is a third that A was told
/// it heard and that is not there at all, which is what a stale row is.
const A: &str = "aa00bb11cc22dd33ee44ff5566778899";
const B: &str = "0011223344556677889900aabbccddee";
const GONE: &str = "99887766554433221100aabbccddeeff";

/// What a WSL kernel calls itself, which is the one thing that tells one from the
/// Windows it shares a hostname with. A is stated as one so that the OS word
/// crossing the wire is a word this file chose.
const WSL_KERNEL: &str = "5.15.167.4-microsoft-standard-WSL2";

/// The name the rows this file states are drawn under, which is what a refusal
/// about one of them names it by.
const FOUND_AS: &str = "kitchen-mini";

/// The port the workbench is taken to be on, which nothing here asks about: each
/// reading's Tailscale is built with it and never runs.
const PORT: u16 = 8422;

/// How long a dial in this suite gives one address, rather than the two seconds a
/// running server gives one.
///
/// Spent by the tests about an address nobody is at: what those are asking is
/// which address a walk lands on and what it says when none of them answers, and
/// a test that waited out the real deadline would be spending its time on the
/// clock rather than on the question.
const PATIENCE: Duration = Duration::from_millis(300);

/// A machine with no Tailscale on it: `verkstead-no-such-tailscale` is a program
/// that is not there, which is what having none *is*. Never run, the readings here
/// saying their addresses outright.
fn no_tailscale() -> Tailscale {
    Tailscale::running(vec!["verkstead-no-such-tailscale".to_owned()], PORT)
}

/// A port on the loopback that nothing is on at all: one taken and given straight
/// back, which is what an address a device has left looks like.
fn nothing_there() -> String {
    let taken = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = taken.local_addr().unwrap().port();

    drop(taken);

    format!("127.0.0.1:{port}")
}

/// What a browse heard of `device`, at `addresses` — the shape a row is drawn
/// from, stated.
fn found(device: &str, addresses: &[String]) -> Found {
    Found {
        device: device.to_owned(),
        name: FOUND_AS.to_owned(),
        os: "macOS".to_owned(),
        addresses: addresses.to_vec(),
    }
}

/// One Verkstead, answering: what it is, where it keeps things, the socket the
/// other dials it on, and what it has heard of the devices it is not linked to.
struct Verkstead {
    device: Device,
    pool: SqlitePool,
    members: Members,
    joins: Joins,
    nudges: Nudges,
    reading: Reading,

    /// Where its peer listener landed, which is also the one address it
    /// advertises.
    address: SocketAddr,

    /// And what it heard, held here rather than made per workbench: a press that
    /// reached nobody forgets the row it was made on, and a browse built afresh
    /// for each router would be a forget that never outlived the press.
    browse: Browse,

    /// Held for the length of the test: the identity and the database both live in
    /// it.
    _dir: tempfile::TempDir,
}

impl Verkstead {
    /// A Verkstead with a store of its own, an identity in it, its peer listener
    /// up on a port the machine picked, and `browse` for what it has heard.
    async fn answering(id: &str, kernel: Option<&str>, browse: Browse) -> Verkstead {
        let dir = tempfile::tempdir().unwrap();
        let pool = open_database(&dir.path().join("verkstead.db"))
            .await
            .unwrap();

        let device = Device::stated(dir.path(), id).unwrap();

        let listener = peer::Listener::bound("127.0.0.1:0".parse().unwrap(), &device)
            .expect("the loopback on a port the machine picked is free");

        let address = listener.address();

        let reading = Reading::advertising(
            no_tailscale(),
            Platform::Linux,
            kernel.map(str::to_owned),
            vec![format!("127.0.0.1:{}", address.port())],
        );

        let standing = Verkstead {
            device,
            members: Members::recorded(pool.clone()),
            joins: Joins::recorded(pool.clone()),
            nudges: Nudges::new(),
            reading,
            pool,
            address,
            browse,
            _dir: dir,
        };

        tokio::spawn(listener.serving(peer::router(
            standing.device.clone(),
            standing.reading.clone(),
            standing.members.clone(),
            standing.joins.clone(),
            standing.nudges.clone(),
        )));

        standing
    }

    /// The device that presses: a WSL, so that the OS word crossing the wire is
    /// one this file chose, holding `heard` as what a browse of its LAN found.
    async fn asking(heard: Vec<Found>) -> Verkstead {
        Verkstead::answering(A, Some(WSL_KERNEL), Browse::stated(heard)).await
    }

    /// And the device it found, which has heard nothing itself: nothing here is
    /// about what *B* would draw.
    async fn asked() -> Verkstead {
        Verkstead::answering(B, None, Browse::heard_nothing()).await
    }

    /// Where the other device dials it, which is the address it advertises.
    fn at(&self) -> String {
        format!("127.0.0.1:{}", self.address.port())
    }

    /// The workbench its browser would be talking to, with every dial given
    /// [`PATIENCE`]: the presses here are meant to walk past addresses nobody is
    /// at, and the real deadlines have nothing to do with what is being asked.
    ///
    /// Built afresh on each call and over the same store, the way
    /// `tests/exchange.rs` builds one — and over the same browse, which is what
    /// makes a row forgotten by one press gone from the next read.
    fn workbench(&self) -> Router {
        router_answering_devices_telling(
            self.pool.clone(),
            Devices::of(
                self.device.clone(),
                self.reading.clone(),
                self.members.clone(),
                self.joins.clone(),
            )
            .browsing(self.browse.clone())
            .waiting(PATIENCE),
            self.nudges.clone(),
        )
    }
}

/// Press **Add** on the discovered row for `device`, and hand back what the
/// workbench answered.
async fn add(app: &Router, device: &str) -> (StatusCode, String) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("{DISCOVERED}/{device}/add"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();

    (status, String::from_utf8_lossy(&bytes).into_owned())
}

/// Press **Allow** on the modal over there.
async fn allow(app: &Router, request: &str) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("{ASKING}/{request}/allow"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();

    assert_eq!(
        status,
        StatusCode::OK,
        "POST {ASKING}/{request}/allow: {}",
        String::from_utf8_lossy(&bytes),
    );
}

/// The Devices section as the pane reads it.
async fn listing(app: &Router) -> DevicesView {
    let response = app
        .clone()
        .oneshot(Request::builder().uri(DEVICES).body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK, "GET {DEVICES}");

    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

/// And the Discovered list under it.
async fn discovered(app: &Router) -> Vec<DiscoveredDevice> {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(DISCOVERED)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK, "GET {DISCOVERED}");

    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

/// What the modal over there is drawn from.
async fn being_asked(app: &Router) -> Vec<AskingDevice> {
    let response = app
        .clone()
        .oneshot(Request::builder().uri(ASKING).body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK, "GET {ASKING}");

    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

/// A press on a discovered row leaves the pending row a typed press leaves, and
/// the row it was made on is gone from the list.
///
/// **Which is the whole of what the press is**, and both halves of it matter: the
/// join is posted to the address the row was found at, with nothing typed
/// anywhere, and the row moves from one list to the other rather than sitting
/// under a pending row about the same device.
#[tokio::test]
async fn a_press_leaves_the_pending_row_and_takes_the_discovered_row_away() {
    let asked = Verkstead::asked().await;
    let asking = Verkstead::asking(vec![found(B, &[asked.at()])]).await;

    let (status, said) = add(&asking.workbench(), B).await;
    assert_eq!(status, StatusCode::OK, "POST {DISCOVERED}/{B}/add: {said}");

    // The far end is holding the question, which is what a join *is*: nothing has
    // been agreed, and somebody over there has something to press.
    let held = being_asked(&asked.workbench()).await;

    assert_eq!(held.len(), 1, "one question, just asked: {held:?}");
    assert_eq!(
        held[0].identity.device, A,
        "asked by the device that pressed"
    );
    assert_eq!(
        held[0].identity.os, "Linux (WSL)",
        "the OS word travels with a join whichever press made it",
    );

    // And this end is holding the row that says so — out of the press's own
    // answer, rather than out of a second read.
    let answered: DevicesView = serde_json::from_str(&said).expect("the section read again");

    assert_eq!(answered.pending.len(), 1, "one row, waiting: {answered:?}");
    assert_eq!(
        answered.pending[0].name,
        platform::hostname(),
        "reading what the device that answered says it is called",
    );
    assert_eq!(
        answered.pending[0].address,
        asked.at(),
        "naming the address the row was found at, which is the one that answered",
    );
    assert!(
        !answered.pending[0].expired && !answered.pending[0].refused,
        "nobody has answered it yet, and nothing has run out",
    );
    assert!(
        answered.members.is_empty(),
        "and a join is not a membership: nothing is agreed until the press over there",
    );

    assert_eq!(
        discovered(&asking.workbench()).await,
        Vec::new(),
        "while the row it was pressed on is gone: a device a join is pending for \
         is one this list leaves out, so one press is one row about one device",
    );
}

/// A device whose first address no longer answers is joined at the next one the
/// discovery found for it.
///
/// **Because a row holds a list.** mDNS resolves every address a device
/// advertised and a probe adds the tailnet's, so a press works down them in the
/// order they were found — the LAN's first, that being the shorter road — exactly
/// as a dial to a member works down that member's.
#[tokio::test]
async fn a_device_is_joined_at_the_next_address_where_the_first_answers_nothing() {
    let asked = Verkstead::asked().await;

    let asking = Verkstead::asking(vec![found(
        B,
        &[nothing_there(), nothing_there(), asked.at()],
    )])
    .await;

    let (status, said) = add(&asking.workbench(), B).await;
    assert_eq!(status, StatusCode::OK, "POST {DISCOVERED}/{B}/add: {said}");

    assert_eq!(
        being_asked(&asked.workbench()).await.len(),
        1,
        "the third address is where it was reached, and two dead ones before it \
         cost the press nothing but the walk",
    );

    let answered: DevicesView = serde_json::from_str(&said).unwrap();

    assert_eq!(
        answered.pending[0].address,
        asked.at(),
        "and the row names the address that answered rather than the first one \
         tried: what a Cancel dials is where the question really went",
    );
}

/// A press on a row whose device has gone is refused in words naming the device,
/// and the next read of the list is without it.
///
/// **Two ways a row goes stale and both are the same press.** A device may have
/// gone off the LAN between the browse hearing it and somebody pressing Add — the
/// row is still held and nothing answers at any of its addresses — or the row may
/// be gone from the list already, a goodbye having arrived first. Neither is a
/// bare failure: the refusal says which device and where it was tried, which is
/// what a dial that reached nobody says anywhere else.
///
/// **And the row goes with the press**, because the press is where this device
/// learned the row was wrong. A device that is really there advertises again and
/// is a row again; one that has gone stays gone without anybody waiting out a TTL.
#[tokio::test]
async fn a_press_on_a_row_whose_device_has_gone_is_refused_and_the_row_goes() {
    let nowhere = nothing_there();
    let asking = Verkstead::asking(vec![found(GONE, std::slice::from_ref(&nowhere))]).await;

    let (status, said) = add(&asking.workbench(), GONE).await;

    assert_eq!(
        status,
        StatusCode::BAD_GATEWAY,
        "the far end is what went wrong rather than this server: {said}",
    );
    assert!(
        said.contains(FOUND_AS),
        "naming the device the row drew rather than failing bare: {said}",
    );
    assert!(
        said.contains(&nowhere),
        "and where it was tried, that being what somebody looking at it can act \
         on: {said}",
    );

    assert_eq!(
        discovered(&asking.workbench()).await,
        Vec::new(),
        "and the row is forgotten, so the list this refusal is drawn beside is one \
         without it",
    );
    assert!(
        listing(&asking.workbench()).await.pending.is_empty(),
        "nothing was asked of anybody, so there is no pending row either",
    );

    // And the same press again, the row having gone: refused naming the device,
    // there being nowhere left to look for it.
    let (status, said) = add(&asking.workbench(), GONE).await;

    assert_eq!(status, StatusCode::BAD_GATEWAY, "{said}");
    assert!(
        said.contains(GONE),
        "a device this one has not heard of is named by the one thing the press \
         carried, which is its id: {said}",
    );
}

/// And allowing the join on the far end makes the device a **Member** on both,
/// drawn as one rather than as discovered.
///
/// **The criterion whole: the press, the answer, and the two lists afterwards.**
/// What makes the row disappear is not the press but the membership it ends in —
/// a member is the first of the three kinds of device this list leaves out — so
/// the device that was pressed is a row above rather than a row below.
#[tokio::test]
async fn allowing_it_makes_the_device_a_member_on_both_and_no_longer_discovered() {
    let asked = Verkstead::asked().await;
    let asking = Verkstead::asking(vec![found(B, &[asked.at()])]).await;

    let (status, said) = add(&asking.workbench(), B).await;
    assert_eq!(status, StatusCode::OK, "POST {DISCOVERED}/{B}/add: {said}");

    let held = being_asked(&asked.workbench()).await;
    assert_eq!(held.len(), 1, "one question to press: {held:?}");

    allow(&asked.workbench(), &held[0].request).await;

    // B's side: A, as A said it was in the join post it made off a row nobody
    // typed an address into.
    let over_there = listing(&asked.workbench()).await;

    assert_eq!(over_there.members.len(), 1, "{over_there:?}");
    assert_eq!(over_there.members[0].identity.device, A);
    assert_eq!(
        over_there.members[0].identity.fingerprint,
        asking.device.fingerprint(),
        "a member *is* a fingerprint, and this one is the certificate B met when \
         the join arrived",
    );

    // A's side: B, out of the roster the dial back carried — and nothing left
    // waiting, the waiting having been answered.
    let over_here = listing(&asking.workbench()).await;

    assert!(over_here.pending.is_empty(), "{over_here:?}");
    assert_eq!(over_here.members.len(), 1);
    assert_eq!(over_here.members[0].identity.device, B);
    assert_eq!(
        over_here.members[0].identity.fingerprint,
        asked.device.fingerprint(),
    );

    assert_eq!(
        discovered(&asking.workbench()).await,
        Vec::new(),
        "and the device is drawn as one thing rather than as two: a member is in \
         the cluster already, so a row offering to link it would be a press with \
         nothing behind it",
    );
}
