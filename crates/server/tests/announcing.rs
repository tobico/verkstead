//! The announcement: a third device joining through either of two, and landing
//! on all three lists off one press (ADR-0020, *A cluster is a membership*).
//!
//! **Three servers, which is the stage's own demonstration.** Each Verkstead
//! here stands a peer listener up on the loopback and advertises the address it
//! really landed on, so every call between them is a real dial over a real
//! handshake against a real certificate — the join, the dial back that answers
//! it, and the announcement that follows. What is being asked is what nobody
//! can ask of two: that one confirmation joins the newcomer to *everybody*, and
//! that the telling is carried by the device the human pressed Allow on rather
//! than by the newcomer itself.
//!
//! **And the gate is the whole of what refuses a newcomer that speaks for
//! itself.** An announcement is a member's own call, so it stands behind
//! [`peer::members_only`] with everything that follows it — a device announcing
//! itself to a member it has not joined is a caller that member holds no
//! membership for, and is refused there without a line having been written to
//! refuse it.

use std::net::SocketAddr;
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use sqlx::SqlitePool;
use tower::ServiceExt;
use verkstead_render::{AskingDevice, DeviceIdentity, DevicesView};
use verkstead_server::device::reading::Reading;
use verkstead_server::device::{Device, Devices};
use verkstead_server::nudge::Nudges;
use verkstead_server::open_database;
use verkstead_server::peer::dialling::Peers;
use verkstead_server::peer::joining::Joins;
use verkstead_server::peer::{self, Members};
use verkstead_server::platform::Platform;
use verkstead_server::remote::Tailscale;
use verkstead_server::router_answering_devices_telling;
use verkstead_store::Linking;

/// Where the pane reads the section, and where its presses go.
const DEVICES: &str = "/api/ui/devices";
const ADD: &str = "/api/ui/devices/joins";
const ASKING: &str = "/api/ui/devices/asking";

/// The three devices, named by the ids a cluster names them by.
///
/// A and B are the two that link first; C is the newcomer, which joins through
/// one of them and has to land on the other's list without anybody touching
/// that machine.
const A: &str = "aa00bb11cc22dd33ee44ff5566778899";
const B: &str = "0011223344556677889900aabbccddee";
const C: &str = "ffeeddccbbaa00998877665544332211";

/// And a fourth that is nobody's member, for the questions about the gate.
const STRANGER: &str = "99887766554433221100ffeeddccbbaa";

/// The port the workbench is taken to be on, which nothing here asks about:
/// each reading's Tailscale is built with it and never runs.
const PORT: u16 = 8422;

/// How long a dial in this suite gives one address, rather than the two seconds
/// a running server gives one.
///
/// Spent only where a dial is meant to reach nobody, which here is the member
/// whose machine is not there: what is being asked is what an announcement does
/// when it cannot be made, and a test that waited out the real deadline would
/// be spending its time on the clock rather than on the question.
const PATIENCE: Duration = Duration::from_millis(300);

/// A machine with no Tailscale on it: `verkstead-no-such-tailscale` is a
/// program that is not there, which is what having none *is*.
fn no_tailscale() -> Tailscale {
    Tailscale::running(vec!["verkstead-no-such-tailscale".to_owned()], PORT)
}

/// One Verkstead, answering: what it is, where it keeps things, and the socket
/// another one dials it on.
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

    /// Held for the length of the test: the identity and the database both live
    /// in it.
    _dir: tempfile::TempDir,
}

impl Verkstead {
    /// A Verkstead with a store of its own, an identity in it, and its peer
    /// listener up on a port the machine picked.
    async fn answering(id: &str) -> Verkstead {
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
            None,
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
            _dir: dir,
        };

        tokio::spawn(listener.serving(peer::router(
            standing.device.clone(),
            standing.reading.clone(),
            standing.members.clone(),
            standing.joins.clone(),
            standing.nudges.clone(),
            // And none of the workbench over the link: what these suites ask
            // about is the link itself — see `tests/relayed.rs`, which is the
            // suite about what a member reaches through one.
            Router::new(),
        )));

        standing
    }

    /// Where another device dials it, which is the address it advertises.
    fn at(&self) -> String {
        format!("127.0.0.1:{}", self.address.port())
    }

    /// The workbench its browser would be talking to, which is what the presses
    /// go through.
    fn workbench(&self) -> Router {
        router_answering_devices_telling(self.pool.clone(), self.devices(), self.nudges.clone())
    }

    /// And the same with every dial given [`PATIENCE`], for the press whose
    /// announcement is meant to reach nobody.
    fn workbench_in_a_hurry(&self) -> Router {
        router_answering_devices_telling(
            self.pool.clone(),
            self.devices().waiting(PATIENCE),
            self.nudges.clone(),
        )
    }

    fn devices(&self) -> Devices {
        Devices::of(
            self.device.clone(),
            self.reading.clone(),
            self.members.clone(),
            self.joins.clone(),
        )
    }

    /// How it dials, for the questions that are about a call rather than about
    /// a press.
    fn peers(&self) -> Peers {
        Peers::of(self.device.clone(), self.members.clone()).waiting(PATIENCE)
    }

    /// What it says about itself, which is what an announcement carries.
    fn identity(&self) -> DeviceIdentity {
        DeviceIdentity {
            device: self.device.id().to_owned(),
            fingerprint: self.device.fingerprint().to_owned(),
            name: "somewhere".to_owned(),
            os: "Linux".to_owned(),
            addresses: vec![self.at()],
        }
    }

    /// And every device on its own list, by id and sorted — which is what
    /// *three lists read the same* comes down to.
    async fn cluster(&self) -> Vec<String> {
        let mut listed: Vec<String> = listing(&self.workbench())
            .await
            .members
            .into_iter()
            .map(|member| member.identity.device)
            .collect();

        listed.sort();
        listed
    }
}

/// Press Add against `address`, and hand back what the workbench answered.
async fn add(app: &Router, address: &str) -> (StatusCode, String) {
    let body = serde_json::to_vec(&serde_json::json!({ "address": address })).unwrap();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(ADD)
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();

    (status, String::from_utf8_lossy(&bytes).into_owned())
}

/// Press Allow on a pending question.
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

/// What the section reads, parsed as the type the pane draws.
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

/// And what the modal is drawn from.
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

/// `joining` links itself to `answering`, all the way through: Add pressed on
/// the first, Allow pressed on the second, and every call between them real.
///
/// One press, which is the whole of what a link costs — and the whole of what
/// a link costs is the point of the suite.
async fn linked(joining: &Verkstead, answering: &Verkstead) {
    let (status, said) = add(&joining.workbench(), &answering.at()).await;
    assert_eq!(status, StatusCode::OK, "POST {ADD}: {said}");

    let held = being_asked(&answering.workbench()).await;
    assert_eq!(held.len(), 1, "one question, just asked");

    allow(&answering.workbench(), &held[0].request).await;
}

// ---------------------------------------------------------------------------
// Three servers
// ---------------------------------------------------------------------------

/// A third device joining through the device that was asked lands on all three
/// lists, off the one press.
///
/// **The stage's own demonstration.** B is what A asked and what C asked, so B
/// is the introducer both times: when C joins, B tells A about it over the link
/// B has held since the first join, and A records C without anybody having
/// touched A at all.
#[tokio::test]
async fn a_third_device_joining_through_the_introducer_lands_on_all_three_lists() {
    let a = Verkstead::answering(A).await;
    let b = Verkstead::answering(B).await;
    let c = Verkstead::answering(C).await;

    linked(&a, &b).await;
    linked(&c, &b).await;

    assert_eq!(
        a.cluster().await,
        sorted(&[B, C]),
        "A was never pressed for C, and holds it: the introducer announced it \
         over the link A had already verified",
    );
    assert_eq!(b.cluster().await, sorted(&[A, C]));
    assert_eq!(
        c.cluster().await,
        sorted(&[A, B]),
        "and C was handed the whole roster in the dial back, rather than B alone",
    );
}

/// And a third device joining through the device that *asked* lands on all
/// three lists too — which is *either of them* rather than one of them.
///
/// Here A is the introducer for C although A is the device that pressed Add on
/// B: what makes a device able to introduce another is the link it holds, and
/// the two ends of a link hold the same one.
#[tokio::test]
async fn a_third_device_joining_through_the_device_that_asked_lands_on_all_three_lists() {
    let a = Verkstead::answering(A).await;
    let b = Verkstead::answering(B).await;
    let c = Verkstead::answering(C).await;

    linked(&a, &b).await;
    linked(&c, &a).await;

    assert_eq!(a.cluster().await, sorted(&[B, C]));
    assert_eq!(
        b.cluster().await,
        sorted(&[A, C]),
        "B was never pressed for C either: A announced it over the link the first \
         join made",
    );
    assert_eq!(c.cluster().await, sorted(&[A, B]));
}

/// A newcomer's own first call to a member it was announced to is an ordinary
/// member's call and gets through — which is what says the announcement was
/// worth making.
///
/// **The same call, made twice, either side of the announcement.** C dials A
/// with a member's own call: refused before, because A holds no membership for
/// C, and answered after, because A recorded C off B's word without anybody
/// having touched A. What is carried is a device A already holds, so the two
/// dials differ in the gate's answer and in nothing else.
#[tokio::test]
async fn a_newcomers_own_first_call_to_a_member_it_was_announced_to_gets_through() {
    let a = Verkstead::answering(A).await;
    let b = Verkstead::answering(B).await;
    let c = Verkstead::answering(C).await;

    linked(&a, &b).await;

    let refused = c
        .peers()
        .announce(&member_at(&a, A), &b.identity())
        .await
        .expect_err("before the announcement C is a device A holds no membership for");

    assert!(
        format!("{refused:#}").contains("403"),
        "refused at A's gate rather than anywhere else: {refused:#}",
    );

    linked(&c, &b).await;

    c.peers()
        .announce(&member_at(&a, A), &b.identity())
        .await
        .expect("and afterwards C is through A's gate, A's human having pressed nothing");

    assert_eq!(
        a.cluster().await,
        sorted(&[B, C]),
        "and the call said what A already held, so the list is what it was",
    );
}

// ---------------------------------------------------------------------------
// Who may name a newcomer
// ---------------------------------------------------------------------------

/// A device announcing itself to a member is refused, and a member announcing a
/// newcomer to another member is not.
///
/// **Which is the whole of why the introducer makes the announcement.** The
/// announcement stands inside the member gate, so what refuses a stranger
/// naming itself is the gate — and a member has no way to tell a newcomer's
/// word for itself from anybody else's who can reach this port. Announced by a
/// device the receiver has confirmed, the claim comes down a verified link.
#[tokio::test]
async fn a_device_announcing_itself_is_refused_and_a_member_announcing_one_is_not() {
    let a = Verkstead::answering(A).await;
    let b = Verkstead::answering(B).await;
    let stranger = Verkstead::answering(STRANGER).await;

    linked(&a, &b).await;

    // The stranger, saying exactly what an introducer would say about it — and
    // refused for being the one device that cannot vouch for it.
    let refused = stranger
        .peers()
        .announce(&member_at(&a, STRANGER), &stranger.identity())
        .await
        .expect_err("a device A holds no membership for is refused at A's gate");

    assert!(
        format!("{refused:#}").contains("403"),
        "refused by the gate rather than missed: {refused:#}",
    );

    assert_eq!(
        a.cluster().await,
        sorted(&[B]),
        "and nothing a stranger said about itself is on A's list",
    );

    // And the same words from B, which A has confirmed, land.
    b.peers()
        .announce(&member_at(&a, B), &stranger.identity())
        .await
        .expect("a member's own peer may name a newcomer to it");

    assert_eq!(
        a.cluster().await,
        sorted(&[B, STRANGER]),
        "the claim is worth recording because of the link it came down, rather \
         than because of what it said",
    );
}

// ---------------------------------------------------------------------------
// What the record is worth
// ---------------------------------------------------------------------------

/// An announcement about a device already recorded updates the row rather than
/// making a second one.
///
/// **Which is what keeps an announcement safe to make again**, and it has to
/// be: a member that was off when a newcomer joined is told when it next
/// answers, and a member that had already heard is told twice for nothing.
/// Keyed on the Device Id, which outlives the name, the addresses and the
/// certificate all three.
#[tokio::test]
async fn an_announcement_about_a_recorded_device_updates_the_row() {
    let a = Verkstead::answering(A).await;
    let b = Verkstead::answering(B).await;
    let c = Verkstead::answering(C).await;

    linked(&a, &b).await;
    linked(&c, &b).await;

    assert_eq!(a.cluster().await, sorted(&[B, C]));

    // The same device said again, having moved and been renamed since.
    let mut moved = c.identity();
    moved.name = "studio".to_owned();
    moved.addresses = vec!["studio.tailnet-name.ts.net".to_owned(), c.at()];

    b.peers().announce(&member_at(&a, B), &moved).await.unwrap();

    assert_eq!(
        a.cluster().await,
        sorted(&[B, C]),
        "one row for C rather than two, the id being what a member is keyed by",
    );

    let recorded = listing(&a.workbench())
        .await
        .members
        .into_iter()
        .find(|member| member.identity.device == C)
        .expect("the device that was announced twice");

    assert_eq!(recorded.identity.name, "studio", "and it is the newer word");
    assert_eq!(
        recorded.identity.addresses,
        vec!["studio.tailnet-name.ts.net".to_owned(), c.at()],
        "in the order they were advertised, which is the order a dial works down",
    );
}

/// A member that answers nothing is dimmed, is written down as still owed the
/// telling, and does not fail the join it was part of.
///
/// **Because the press already happened.** A human pressed Allow, and the
/// newcomer is a member of the introducer whatever some third machine made of
/// it — a join that failed because somebody's laptop was shut would be a link
/// nobody could make while a member was away.
#[tokio::test]
async fn a_member_that_answers_nothing_is_dimmed_and_owed_the_telling() {
    let b = Verkstead::answering(B).await;
    let c = Verkstead::answering(C).await;

    // A member of B that is not there: an address nothing is at, written
    // straight in, because what is being asked is what the announcement does
    // when it cannot be made rather than how the member came to be recorded.
    verkstead_store::record_member(
        &b.pool,
        &Linking {
            device: A.to_owned(),
            name: "laptop".to_owned(),
            os: "Linux (WSL)".to_owned(),
            addresses: vec!["127.0.0.1:1".to_owned()],
            fingerprint: "AA:BB:CC:DD:EE:FF".to_owned(),
        },
    )
    .await
    .unwrap();

    let (status, said) = add(&c.workbench(), &b.at()).await;
    assert_eq!(status, StatusCode::OK, "POST {ADD}: {said}");

    let held = being_asked(&b.workbench()).await;
    allow(&b.workbench_in_a_hurry(), &held[0].request).await;

    assert_eq!(
        b.cluster().await,
        sorted(&[A, C]),
        "the press stands whole: the newcomer is a member and the machine that \
         was off is still one",
    );
    assert_eq!(
        c.cluster().await,
        sorted(&[A, B]),
        "and the newcomer was handed the roster, unreachable member and all",
    );

    let laptop = listing(&b.workbench())
        .await
        .members
        .into_iter()
        .find(|member| member.identity.device == A)
        .expect("the member that was not there");

    assert!(
        !laptop.reachable,
        "the dial reached none of its addresses, which is what dims a row",
    );

    assert_eq!(
        verkstead_store::announcements_owed(&b.pool, C)
            .await
            .unwrap(),
        vec![A.to_owned()],
        "and the telling that did not get through is written down as owed, which \
         is what the task after this one pays",
    );
}

/// And a telling that got through is owed by nobody.
#[tokio::test]
async fn a_telling_that_got_through_is_owed_by_nobody() {
    let a = Verkstead::answering(A).await;
    let b = Verkstead::answering(B).await;
    let c = Verkstead::answering(C).await;

    linked(&a, &b).await;
    linked(&c, &b).await;

    assert!(
        verkstead_store::announcements_owed(&b.pool, C)
            .await
            .unwrap()
            .is_empty(),
        "A was there and was told, so there is no debt to pay",
    );
}

// ---------------------------------------------------------------------------
// The plumbing
// ---------------------------------------------------------------------------

/// `device`'s ids, sorted — which is the shape a list is compared in.
fn sorted(devices: &[&str]) -> Vec<String> {
    let mut sorted: Vec<String> = devices.iter().map(|&device| device.to_owned()).collect();
    sorted.sort();
    sorted
}

/// A member row pointing at `listening`, as the device named `device` would
/// hold one of it.
///
/// For the tests that announce rather than press: what a dial is made of is the
/// addresses to work down and the certificate to insist on, and both are the
/// listening device's own — so a row need not have been recorded anywhere to
/// say where a call goes.
fn member_at(listening: &Verkstead, device: &str) -> verkstead_store::Member {
    verkstead_store::Member {
        device: device.to_owned(),
        name: "somewhere".to_owned(),
        os: "Linux".to_owned(),
        addresses: vec![listening.at()],
        fingerprint: listening.device.fingerprint().to_owned(),
        last_seen: "2099-01-01T00:00:00Z".to_owned(),
        reachable: true,
        renewing_from: None,
        acknowledged: None,
    }
}
