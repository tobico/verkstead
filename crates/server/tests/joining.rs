//! A join up to the press that settles it: A asks, B holds the question and
//! raises it in front of its human, and somebody there presses Allow or Deny
//! (ADR-0020, *The join*).
//!
//! **Two Verksteads, and every join here is a real dial over a real socket.**
//! What is being asked is what the product does when somebody types an address
//! and presses Add — which certificate is presented, which is accepted, what the
//! far end writes down, and what is left on the pane afterwards — and none of
//! that is a question a router asked in process can answer. So B stands its peer
//! listener up on the loopback, and A presses Add through the same workbench
//! route the browser presses.
//!
//! **A is not answering here, so the press stops at B.** Allow records A as a
//! member over here and settles the question, and the dial back that would
//! close the link finds nothing at the addresses A advertised — which are two
//! addresses this file chose and nothing is at. That is the point of the
//! arrangement rather than a gap in it: what this file is about is the question
//! and the press, and what a press does when it *can* reach the far end is
//! `tests/exchange.rs`'s, where both devices are listening. So a join in this
//! file ends one of four ways — cancelled, expired, allowed or denied — and in
//! none of them is there a link.
//!
//! Which is also why the presses go through a workbench in a hurry: a dial back
//! to two addresses nobody is at costs the real deadline twice, and what is
//! being asked has nothing to do with how long that takes.
//!
//! **And the Nudge is read off the stream a page really listens on.** A join
//! lands on B's *peer* listener and the modal it raises is drawn on a page B's
//! *workbench* listener served; the two are one process sharing one handle, and
//! the only way to ask whether that holds is to open the page's own connection
//! and wait on it — see [`Listening`].
//!
//! The machines both devices are on are stated rather than read, for the reason
//! `tests/devices.rs` states one: what a WSL reads as is the whole point of the
//! OS word, and the box a suite happens to be running on is the one machine that
//! cannot be asked about it. Which is also what makes the addresses assertable —
//! a join carries every address the asking device has, and here that is a list
//! this file chose.

use std::net::SocketAddr;
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use sqlx::SqlitePool;
use tower::ServiceExt;
use verkstead_render::{AskingDevice, DevicesView};
use verkstead_schema::Nudge;
use verkstead_server::device::reading::Reading;
use verkstead_server::device::{Device, Devices};
use verkstead_server::nudge::Nudges;
use verkstead_server::open_database;
use verkstead_server::peer::dialling::Peers;
use verkstead_server::peer::joining::{HELD, HELD_AT_ONCE, Joins};
use verkstead_server::peer::{self, Members};
use verkstead_server::platform::{self, Platform};
use verkstead_server::remote::Tailscale;
use verkstead_server::router_answering_devices_telling;
use verkstead_store::{
    AskedJoin, HeldJoin, ask_join, asked_joins, held_join, held_joins, hold_join,
};

/// Where the pane reads the section, and where its one press goes.
const DEVICES: &str = "/api/ui/devices";
const ADD: &str = "/api/ui/devices/joins";

/// The ids the two devices are stated as, so that what a test asserts against is
/// a string it chose rather than sixteen random bytes — see [`Device::stated`],
/// which is here for that reason.
const A: &str = "aa00bb11cc22dd33ee44ff5566778899";
const B: &str = "0011223344556677889900aabbccddee";

/// What a WSL kernel calls itself, which is the one thing that says one apart
/// from the Linux it is in every other way — and the case the whole of cluster
/// mode was written for. A is stated as one so that the OS word travelling with
/// a join is a word this file chose.
const WSL_KERNEL: &str = "5.15.167.4-microsoft-standard-WSL2";

/// The addresses A advertises, which is what B has to write down: a join carries
/// every address the asking device has, in the order to try them.
const A_ADDRESSES: [&str; 2] = ["192.168.1.24", "10.0.0.7"];

/// The port the workbench is taken to be on, which nothing here asks about: each
/// reading's Tailscale is built with it and never runs.
const PORT: u16 = 8422;

/// How long a dial in this suite gives one address, rather than the two seconds
/// a running server gives one.
///
/// Spent by the test about an address nobody is at, and by every press: a dial
/// back in this file reaches none of the two addresses A advertises, and what
/// each of those tests is asking has nothing to do with how long that takes.
const PATIENCE: Duration = Duration::from_millis(300);

/// A moment well behind any test run: the far side of *has this run out*.
const LONG_AGO: &str = "2020-01-01T00:00:00Z";

/// A machine with no Tailscale on it: `verkstead-no-such-tailscale` is a program
/// that is not there, which is what having none *is*.
fn no_tailscale() -> Tailscale {
    Tailscale::running(vec!["verkstead-no-such-tailscale".to_owned()], PORT)
}

/// One Verkstead: what it is, where it keeps things, and — where it is answering
/// — the socket another one dials it on.
struct Verkstead {
    device: Device,
    pool: SqlitePool,
    members: Members,
    joins: Joins,

    /// The stream its open workbenches hear what moved on, made once and given
    /// to both its listeners: a join lands on the peer one and the modal it
    /// raises is drawn on a page the other serves, so a test asking whether a
    /// page hears a join is asking about one handle.
    nudges: Nudges,

    /// The machine it is on, stated: what it answers for itself, and what a
    /// join it posts carries.
    reading: Reading,

    /// Where its peer listener landed, for the one that is answering. `None` is
    /// a Verkstead that only ever does the asking.
    address: Option<SocketAddr>,

    /// Held for the length of the test: the identity and the database both live
    /// in it.
    _dir: tempfile::TempDir,
}

impl Verkstead {
    /// A Verkstead with a store of its own and an identity in it, on the machine
    /// `reading` describes.
    async fn called(id: &str, reading: Reading) -> Verkstead {
        let dir = tempfile::tempdir().unwrap();
        let pool = open_database(&dir.path().join("verkstead.db"))
            .await
            .unwrap();

        Verkstead {
            device: Device::stated(dir.path(), id).unwrap(),
            members: Members::recorded(pool.clone()),
            joins: Joins::recorded(pool.clone()),
            nudges: Nudges::new(),
            reading,
            pool,
            address: None,
            _dir: dir,
        }
    }

    /// The device that presses Add: a WSL, so that the OS word a join carries is
    /// one this file chose, on the two addresses above.
    async fn asking() -> Verkstead {
        Verkstead::called(
            A,
            Reading::stated(
                no_tailscale(),
                Platform::Linux,
                Some(WSL_KERNEL.to_owned()),
                A_ADDRESSES.iter().map(|at| at.parse().unwrap()).collect(),
            ),
        )
        .await
    }

    /// And the device that is asked, with its peer listener up on a port the
    /// machine picked — the real one would fight whatever is already on 8423,
    /// and two of these tests at once would fight each other.
    async fn answering() -> Verkstead {
        let mut asked = Verkstead::called(
            B,
            Reading::stated(no_tailscale(), Platform::HERE, None, Vec::new()),
        )
        .await;

        asked.answer();
        asked
    }

    /// Stand the peer listener up, serving what a Verkstead serves.
    fn answer(&mut self) {
        let listener = peer::Listener::bound("127.0.0.1:0".parse().unwrap(), &self.device)
            .expect("the loopback on a port the machine picked is free");

        self.address = Some(listener.address());

        tokio::spawn(listener.serving(peer::router(
            self.device.clone(),
            self.reading.clone(),
            self.members.clone(),
            self.joins.clone(),
            self.nudges.clone(),
            // And none of the workbench over the link: what these suites ask
            // about is the link itself — see `tests/relayed.rs`, which is the
            // suite about what a member reaches through one.
            Router::new(),
        )));
    }

    /// Where another device dials it.
    fn at(&self) -> String {
        format!(
            "127.0.0.1:{}",
            self.address.expect("this Verkstead is answering").port(),
        )
    }

    /// The workbench this Verkstead's browser would be talking to, which is what
    /// Add is pressed through.
    ///
    /// Built afresh on each call, and over the same store: a router is a handle
    /// over the rows rather than a copy of them, so one built after a press reads
    /// what the press wrote — which is what makes it a stand-in for a restart.
    fn workbench(&self) -> Router {
        router_answering_devices_telling(self.pool.clone(), self.devices(), self.nudges.clone())
    }

    /// And the same with every dial given [`PATIENCE`], for the one test about
    /// an address nobody is at.
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

    /// How it dials, for the questions that are about a call rather than about a
    /// press.
    fn peers(&self) -> Peers {
        Peers::of(self.device.clone(), self.members.clone())
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

/// And press Cancel — or Dismiss, which is the same press — on a pending row.
async fn cancel(app: &Router, request: &str) -> StatusCode {
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("{ADD}/{request}/cancel"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
        .status()
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

/// Add against B's address leaves a pending row on A and a recorded request on
/// B, and the request holds the whole of what A said about itself.
///
/// The one test that is the criterion whole, because the two halves are one
/// dial: what B wrote down is only there because A's press got that far, and
/// what A drew is only there because B answered.
#[tokio::test]
async fn add_leaves_a_pending_row_here_and_a_request_there() {
    let asked = Verkstead::answering().await;
    let asking = Verkstead::asking().await;

    let app = asking.workbench();
    let (status, said) = add(&app, &asked.at()).await;

    assert_eq!(status, StatusCode::OK, "POST {ADD}: {said}");

    // A's side: one pending row, waiting on the device that answered, with the
    // address that was typed.
    let listing = listing(&app).await;

    assert_eq!(listing.pending.len(), 1);
    assert!(
        listing.members.is_empty(),
        "nothing has been agreed — a member is made by a press on the other machine",
    );

    let pending = &listing.pending[0];

    assert_eq!(pending.address, asked.at());
    assert_eq!(
        pending.name,
        platform::hostname(),
        "the row says which device is being waited on, which is what that device \
         answered it is shown under",
    );
    assert!(!pending.expired, "it has just been asked");

    // And the fingerprint the row draws is this device's own, which is the whole
    // point of it: the modal on the other machine draws the same string, and the
    // two are there to be compared by eye.
    assert_eq!(listing.this.fingerprint, asking.device.fingerprint());

    // B's side: the request, holding what A said and the certificate A presented.
    let held = held_join(&asked.pool, &pending.request)
        .await
        .unwrap()
        .expect("the device that was asked should be holding the request");

    assert_eq!(held.device, A);
    assert_eq!(held.name, platform::hostname());
    assert_eq!(
        held.os, "Linux (WSL)",
        "the OS word travels with the join, which is what tells a WSL from the \
         Windows it shares a hostname with",
    );
    assert_eq!(held.addresses, A_ADDRESSES, "every address, in order");
    assert_eq!(
        held.fingerprint,
        asking.device.fingerprint(),
        "the certificate the handshake took is what is pinned into the request, \
         and it is what the dial back will be matched against",
    );
}

/// The join post reaches a device that holds no membership for the caller, and
/// posting one does not make it one.
///
/// **Which is the whole arrangement.** A join comes from a non-member by
/// definition — that is what a join is — so the post stands outside the member
/// gate; and it is still no membership afterwards, because the gate reads the
/// members table and the join writes a pending request. What that same caller
/// meets on every gated route is `tests/peer.rs`'s own question, asked there
/// with a dial that shows an unrecorded certificate: refused, in words that say
/// it is a membership rather than a missing path.
#[tokio::test]
async fn a_join_reaches_a_device_that_holds_no_membership_for_the_caller() {
    let asked = Verkstead::answering().await;
    let asking = Verkstead::asking().await;

    assert!(
        !verkstead_store::member_holding(&asked.pool, asking.device.fingerprint())
            .await
            .unwrap(),
        "the caller is a stranger before it posts",
    );

    let (status, said) = add(&asking.workbench(), &asked.at()).await;
    assert_eq!(status, StatusCode::OK, "POST {ADD}: {said}");

    assert!(
        !verkstead_store::member_holding(&asked.pool, asking.device.fingerprint())
            .await
            .unwrap(),
        "and it is still a stranger after — a join asks for a membership rather \
         than taking one, which is what the press on the other machine is for",
    );
}

/// Cancel takes the request off the device that was asked as well as off this
/// one, and a second press is not a second thing happening.
#[tokio::test]
async fn cancel_takes_the_request_off_both_devices() {
    let asked = Verkstead::answering().await;
    let asking = Verkstead::asking().await;

    let app = asking.workbench();
    add(&app, &asked.at()).await;

    let request = listing(&app).await.pending[0].request.clone();

    assert_eq!(cancel(&app, &request).await, StatusCode::OK);

    assert!(
        listing(&app).await.pending.is_empty(),
        "the row is off the device that pressed it",
    );
    assert_eq!(
        held_join(&asked.pool, &request).await.unwrap(),
        None,
        "and the question is off the device that was holding it",
    );

    assert_eq!(
        cancel(&app, &request).await,
        StatusCode::OK,
        "a second Cancel is not a second thing happening: the row it names is \
         already gone, and saying so twice is one thing said twice",
    );
}

/// A request older than ten minutes reads expired on the row that asked for it,
/// and is dismissed from it.
#[tokio::test]
async fn an_expired_request_reads_so_and_is_dismissed() {
    let asking = Verkstead::asking().await;

    // Written straight in, because what is being asked about is a request that
    // was made ten minutes ago and there is no ten minutes to spend on it.
    ask_join(
        &asking.pool,
        &AskedJoin {
            request: "1122334455667788".to_owned(),
            address: "192.168.1.31".to_owned(),
            device: B.to_owned(),
            name: "workbench".to_owned(),
            fingerprint: "AA:BB:CC".to_owned(),
            asked_at: LONG_AGO.to_owned(),
            expires_at: LONG_AGO.to_owned(),
            refused: false,
        },
    )
    .await
    .unwrap();

    let app = asking.workbench();
    let waiting = listing(&app).await;

    assert_eq!(waiting.pending.len(), 1);
    assert!(
        waiting.pending[0].expired,
        "the ten minutes are up, and the row says so rather than going on reading \
         waiting for ever",
    );

    assert_eq!(cancel(&app, "1122334455667788").await, StatusCode::OK);

    assert!(
        listing(&app).await.pending.is_empty(),
        "and it is dismissed by the same press that cancels a live one — nothing \
         is dialled for it, the other device having let go of it already",
    );
}

/// And the device that was asked refuses anything naming a request whose ten
/// minutes have run out.
#[tokio::test]
async fn a_request_that_ran_out_is_refused_by_the_device_holding_it() {
    let asked = Verkstead::answering().await;
    let asking = Verkstead::asking().await;

    // A request that arrived ten minutes ago, pinned on the certificate the
    // caller below really presents: what refuses this call has to be the expiry
    // rather than the certificate.
    hold_join(
        &asked.pool,
        &HeldJoin {
            request: "1122334455667788".to_owned(),
            device: A.to_owned(),
            name: "laptop".to_owned(),
            os: "Linux (WSL)".to_owned(),
            addresses: A_ADDRESSES.iter().map(|at| (*at).to_owned()).collect(),
            fingerprint: asking.device.fingerprint().to_owned(),
            asked_at: LONG_AGO.to_owned(),
            expires_at: LONG_AGO.to_owned(),
        },
    )
    .await
    .unwrap();

    let why = asking
        .peers()
        .cancel(&asked.at(), "1122334455667788", asked.device.fingerprint())
        .await
        .expect_err("a request whose ten minutes are up is one there is nothing to do about");

    assert!(
        format!("{why:#}").contains("404"),
        "and it is refused in the same words as a request that was never there, \
         so that what a caller is told cannot depend on whether the housekeeping \
         has been round yet, got: {why:#}",
    );
}

/// Both records survive a restart of the server holding them, with the ten
/// minutes counted from when the request was made.
///
/// The restart is a fresh set of handles over the same database, which is what a
/// start really is: nothing here is held in memory, so what the second one reads
/// is what the first one wrote.
#[tokio::test]
async fn both_records_survive_a_restart() {
    let asked = Verkstead::answering().await;
    let asking = Verkstead::asking().await;

    add(&asking.workbench(), &asked.at()).await;

    let before = asked_joins(&asking.pool).await.unwrap();
    assert_eq!(before.len(), 1);

    let request = before[0].request.clone();
    let expires_at = before[0].expires_at.clone();

    // The device that asked, come up again.
    let again = listing(&asking.workbench()).await;

    assert_eq!(again.pending.len(), 1);
    assert_eq!(again.pending[0].request, request);
    assert!(!again.pending[0].expired);
    assert_eq!(
        asked_joins(&asking.pool).await.unwrap()[0].expires_at,
        expires_at,
        "the moment it runs out at is where it was, rather than ten fresh minutes \
         counted from the start",
    );

    // And the device that was asked, likewise: the same request, still pinned on
    // the certificate the handshake took, read through a handle of its own.
    let held = held_join(&asked.pool, &request)
        .await
        .unwrap()
        .expect("the request is still held over there");

    assert_eq!(held.fingerprint, asking.device.fingerprint());
    assert!(
        asked_joins(&asked.pool).await.unwrap().is_empty(),
        "and it is holding a question rather than waiting on one — the two sides \
         of a join are two records",
    );
}

/// An address that says which port is dialled at that port.
///
/// Which is this suite's whole way of reaching anything: every device here
/// answers on a port the operating system picked, so every address in every
/// other test carries one. A bare address goes to the peer port instead — see
/// `dialling`'s own unit tests, which is where that is asked, there being no way
/// for a suite to take 8423 without fighting whatever is on it.
#[tokio::test]
async fn an_address_that_names_a_port_is_asked_at_it() {
    let asked = Verkstead::answering().await;
    let asking = Verkstead::asking().await;

    let at = asked.at();
    assert!(at.contains(':'), "the address carries the port: {at}");

    let app = asking.workbench();
    let (status, said) = add(&app, &at).await;

    assert_eq!(status, StatusCode::OK, "POST {ADD}: {said}");
    assert_eq!(listing(&app).await.pending[0].address, at);
}

/// An address with nothing at it leaves no row and says what happened, in the
/// words the dial put it in.
///
/// Because what the human can do about each way of not getting through is
/// different — a machine that is off, a Verkstead too old to have the route, one
/// that refused — and a press that said only *it did not work* would leave them
/// nothing to act on.
#[tokio::test]
async fn an_address_nobody_is_at_leaves_no_row() {
    let asking = Verkstead::asking().await;

    // A port taken and given straight back, which is a machine that refuses
    // rather than one that says nothing.
    let taken = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let nowhere = taken.local_addr().unwrap();
    drop(taken);

    let app = asking.workbench_in_a_hurry();
    let (status, said) = add(&app, &format!("127.0.0.1:{}", nowhere.port())).await;

    assert_eq!(status, StatusCode::BAD_GATEWAY, "{said}");
    assert!(
        said.contains("127.0.0.1"),
        "the refusal names the address that answered nothing, got: {said}",
    );
    assert!(
        listing(&app).await.pending.is_empty(),
        "a request that was never made leaves nothing to cancel",
    );
}

/// And an empty address is refused before anything is dialled.
#[tokio::test]
async fn an_empty_address_is_refused() {
    let asking = Verkstead::asking().await;

    let (status, said) = add(&asking.workbench(), "   ").await;

    assert_eq!(status, StatusCode::BAD_REQUEST, "{said}");
    assert!(
        asked_joins(&asking.pool).await.unwrap().is_empty(),
        "and nothing is written down about it",
    );
}

/// A device asking itself is refused, and the question it asked is taken back
/// off itself.
///
/// The pane draws this machine's own addresses a few lines above the box, so
/// typing one in is an easy mistake — and the state it would otherwise leave is
/// this workbench raising a modal asking whether to link to this workbench.
#[tokio::test]
async fn a_device_cannot_ask_itself() {
    let mut itself = Verkstead::asking().await;
    itself.answer();

    let app = itself.workbench();
    let (status, said) = add(&app, &itself.at()).await;

    assert_eq!(status, StatusCode::BAD_GATEWAY, "{said}");
    assert!(
        listing(&app).await.pending.is_empty(),
        "no row is left on the device that asked",
    );
    assert!(
        asked_joins(&itself.pool).await.unwrap().is_empty(),
        "and none on the device that was asked, which is the same one",
    );
}

/// A device already holding as many questions as it will refuses the next one,
/// and nothing of it is written down.
///
/// **The one route a stranger reaches that writes**, so how much of this machine
/// it is worth is this machine's to decide rather than the caller's — see
/// `HELD_AT_ONCE`. Every held request is a row, a push to this human's phones, a
/// Nudge to every open workbench and a task that wakes ten minutes later to dial
/// the addresses the post named; without the ceiling, all of that is spent as
/// often as anybody who can reach the port cares to ask.
#[tokio::test]
async fn a_device_holding_all_the_questions_it_will_refuses_the_next() {
    let asked = Verkstead::answering().await;
    let asking = Verkstead::asking().await;

    fill(&asked, &far_ahead()).await;

    let (status, said) = add(&asking.workbench(), &asked.at()).await;

    assert_eq!(
        status,
        StatusCode::BAD_GATEWAY,
        "the press says what the far end said: {said}",
    );
    assert!(
        said.contains("as many join requests as it will"),
        "in the far end's own words, so the human can tell it from a machine that \
         is off: {said}",
    );

    assert_eq!(
        held_joins(&asked.pool).await.unwrap().len(),
        HELD_AT_ONCE,
        "nothing was written, so nothing was pushed and no timer was set",
    );

    assert!(
        listing(&asking.workbench()).await.pending.is_empty(),
        "and nothing is drawn over here either: there is no request to cancel",
    );
}

/// And questions that ran out do not keep a device from being asked, because the
/// sweep runs before the counting.
///
/// A device turned away for questions nobody answered last week is a device that
/// stopped taking them a week ago — which is the ceiling refusing the thing it
/// exists to protect.
#[tokio::test]
async fn questions_that_ran_out_leave_room_for_another() {
    let asked = Verkstead::answering().await;
    let asking = Verkstead::asking().await;

    fill(&asked, LONG_AGO).await;

    let (status, said) = add(&asking.workbench(), &asked.at()).await;
    assert_eq!(status, StatusCode::OK, "POST {ADD}: {said}");

    assert_eq!(
        held_joins(&asked.pool).await.unwrap().len(),
        1,
        "the ones that had run out went with the sweep, and the new one is held",
    );
}

/// As many questions as a device will hold, written straight into its store:
/// what the ceiling is asked about is the count, and posting sixteen real joins
/// would be asking the same question sixteen times over a socket.
async fn fill(asked: &Verkstead, expires_at: &str) {
    for held in 0..HELD_AT_ONCE {
        hold_join(
            &asked.pool,
            &HeldJoin {
                request: format!("{held:032x}"),
                device: format!("{held:032x}"),
                name: "laptop".to_owned(),
                os: "Linux".to_owned(),
                addresses: vec!["192.168.1.99".to_owned()],
                fingerprint: format!("AA:BB:{held:02X}"),
                asked_at: LONG_AGO.to_owned(),
                expires_at: expires_at.to_owned(),
            },
        )
        .await
        .unwrap();
    }
}

/// A moment well ahead of any test run: the near side of *has this run out*.
fn far_ahead() -> String {
    (time::OffsetDateTime::now_utc() + Duration::from_secs(600))
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap()
}

/// The ten minutes on the row are the ten minutes the ADR says, taken from when
/// the request was made.
#[tokio::test]
async fn the_request_runs_out_ten_minutes_after_it_was_made() {
    use time::OffsetDateTime;
    use time::format_description::well_known::Rfc3339;

    let asked = Verkstead::answering().await;
    let asking = Verkstead::asking().await;

    let pressed = OffsetDateTime::now_utc();
    add(&asking.workbench(), &asked.at()).await;

    let rows = asked_joins(&asking.pool).await.unwrap();
    let expires = OffsetDateTime::parse(&rows[0].expires_at, &Rfc3339).unwrap();

    let held_for = expires - pressed;

    // Measured from just before the press, so the ten minutes are counted from a
    // moment a hair later than this one — the slack is for that and for nothing
    // else.
    assert!(
        held_for >= HELD && held_for <= HELD + Duration::from_secs(30),
        "a request is held ten minutes from the moment it was made, and this one \
         is held for {held_for}",
    );
}

// ---------------------------------------------------------------------------
// The second half: the human on B is asked, and presses.
// ---------------------------------------------------------------------------

/// Where the workbench of the device that was asked reads the question, and
/// where the two presses that settle it go.
const ASKING: &str = "/api/ui/devices/asking";

/// Where an open page listens for the word that something moved.
const NUDGES: &str = "/api/ui/nudges";

/// How long a test will wait for a Nudge it expects: generous, because it is
/// only ever paid when the assertion is about to fail.
const HEARING: Duration = Duration::from_secs(5);

/// One of that device's open workbenches, listening on the Nudge stream.
///
/// Read off the wire rather than off the channel behind it, the way
/// `tests/nudges.rs` reads one and for a reason of this suite's own: what has to
/// hold here is that a join landing on the *peer* listener reaches a page served
/// by the *workbench* one, and the only thing that can be asked about is what
/// comes down the page's own connection.
struct Listening {
    body: Body,

    /// What has been read off the stream and is not a whole frame yet. SSE
    /// frames are not the chunks they arrive in.
    buffered: String,
}

impl Listening {
    /// Open the stream the way a page does. Returns once the response is in
    /// hand, which is after the handler has subscribed — so anything the test
    /// does next is something this page is listening for.
    async fn open(app: &Router) -> Listening {
        let response = app
            .clone()
            .oneshot(Request::builder().uri(NUDGES).body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK, "GET {NUDGES}");

        Listening {
            body: response.into_body(),
            buffered: String::new(),
        }
    }

    /// The next Nudge, past the keep-alives that are the stream's other traffic.
    async fn nudge(&mut self) -> Nudge {
        let waited_for = tokio::time::timeout(HEARING, async {
            loop {
                let frame = self.frame().await;

                if let Some(data) = frame.lines().find_map(|line| line.strip_prefix("data: ")) {
                    return serde_json::from_str(data).unwrap();
                }
            }
        });

        waited_for.await.expect("waited for a Nudge in vain")
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
}

/// What the modal is drawn from, parsed as the type it draws.
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

/// Press Allow — or Deny, which goes to the route beside it — and hand back what
/// the workbench answered with.
async fn press(app: &Router, request: &str, button: &str) -> (StatusCode, Vec<AskingDevice>) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("{ASKING}/{request}/{button}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();

    let left = if status == StatusCode::OK {
        serde_json::from_slice(&bytes).unwrap()
    } else {
        panic!(
            "POST {ASKING}/{request}/{button}: {}",
            String::from_utf8_lossy(&bytes),
        )
    };

    (status, left)
}

/// A join asked of B, with B's workbench and the request's own name in hand.
///
/// Every test below this line starts here: what each of them is about is the
/// press, and the press needs a question to be pressed on.
async fn a_join_asked_of(asked: &Verkstead, asking: &Verkstead) -> String {
    let (status, said) = add(&asking.workbench(), &asked.at()).await;
    assert_eq!(status, StatusCode::OK, "POST {ADD}: {said}");

    let held = held_joins(&asked.pool).await.unwrap();
    assert_eq!(held.len(), 1, "one question, just asked");

    held[0].request.clone()
}

/// The modal reads the device asking by the four things it names it with: its
/// name, the word for its OS, the address it advertised and the fingerprint of
/// the certificate it presented.
///
/// And that last is the string the *asking* device's own pending row is drawing,
/// which is the whole reason both ends show one: two people, one at each screen,
/// comparing one certificate by eye.
#[tokio::test]
async fn the_modal_reads_the_device_asking_by_name_os_address_and_fingerprint() {
    let asked = Verkstead::answering().await;
    let asking = Verkstead::asking().await;

    let request = a_join_asked_of(&asked, &asking).await;

    let being = being_asked(&asked.workbench()).await;
    assert_eq!(being.len(), 1);

    let card = &being[0];

    assert_eq!(card.request, request);
    assert_eq!(card.identity.device, A);
    assert_eq!(card.identity.name, platform::hostname());
    assert_eq!(
        card.identity.os, "Linux (WSL)",
        "the OS word is what tells a WSL from the Windows it shares a hostname \
         with, which is the case this whole card is drawn for",
    );
    assert_eq!(
        card.identity.addresses, A_ADDRESSES,
        "every address it advertised, in the order it advertised them — the modal \
         draws the first",
    );

    // The one string the two screens are compared on.
    assert_eq!(card.identity.fingerprint, asking.device.fingerprint());
    assert_eq!(
        card.identity.fingerprint,
        listing(&asking.workbench()).await.this.fingerprint,
        "the modal over here and the pending row over there draw one certificate",
    );
}

/// Allow records the device that asked as a member of this one, and settles the
/// request.
///
/// **And a dial back that reaches nobody does not undo the press.** A is not
/// answering in this file, so what B tells it goes nowhere at all — the human
/// pressed Allow and A is a member of B for it, which is the whole of what the
/// press means. What is left over there is a row that runs out, and an Add to
/// press again once that machine is up.
#[tokio::test]
async fn allow_records_the_device_asking_and_settles_the_request() {
    let asked = Verkstead::answering().await;
    let asking = Verkstead::asking().await;

    let request = a_join_asked_of(&asked, &asking).await;

    let over_here = asked.workbench_in_a_hurry();
    let (_, left) = press(&over_here, &request, "allow").await;

    assert!(
        left.is_empty(),
        "the question is answered, so the modal goes"
    );

    // The membership's first real row, out of what the asker said about itself.
    let members = listing(&over_here).await.members;
    assert_eq!(members.len(), 1);

    let member = &members[0];

    assert_eq!(member.identity.device, A);
    assert_eq!(member.identity.name, platform::hostname());
    assert_eq!(member.identity.os, "Linux (WSL)");
    assert_eq!(member.identity.addresses, A_ADDRESSES);
    assert_eq!(
        member.identity.fingerprint,
        asking.device.fingerprint(),
        "a member *is* a fingerprint — the certificate the join was posted under",
    );
    assert!(member.reachable, "it was answering a moment ago");

    assert!(
        held_joins(&asked.pool).await.unwrap().is_empty(),
        "and the question is let go of rather than left lying about",
    );

    // And the other machine has not been told a thing.
    let there = listing(&asking.workbench()).await;

    assert_eq!(there.pending.len(), 1, "still waiting");
    assert!(
        !there.pending[0].expired,
        "waiting rather than run out: nothing has happened to it at all",
    );
    assert!(
        there.members.is_empty(),
        "the dial back that would have made this a link reached none of the \
         addresses A advertised, so nothing of B is recorded over there",
    );
}

/// Deny settles the request and records nothing.
#[tokio::test]
async fn deny_settles_the_request_and_records_nothing() {
    let asked = Verkstead::answering().await;
    let asking = Verkstead::asking().await;

    let request = a_join_asked_of(&asked, &asking).await;

    let over_here = asked.workbench_in_a_hurry();
    let (_, left) = press(&over_here, &request, "deny").await;

    assert!(
        left.is_empty(),
        "the question is answered, so the modal goes"
    );
    assert!(
        listing(&over_here).await.members.is_empty(),
        "a device turned away is a device this one is not linked to",
    );
    assert!(held_joins(&asked.pool).await.unwrap().is_empty());
}

/// Neither press can be made twice to any effect, which is what two workbenches
/// showing one modal need: the first settles the request, and the second finds
/// nothing held.
///
/// Both orders, because they are the same shrug from two directions — a second
/// Allow must not be a second member, and a Deny after an Allow must not take
/// the member away.
#[tokio::test]
async fn neither_press_can_be_made_twice_to_any_effect() {
    let asked = Verkstead::answering().await;
    let asking = Verkstead::asking().await;

    let request = a_join_asked_of(&asked, &asking).await;
    let over_here = asked.workbench_in_a_hurry();

    press(&over_here, &request, "allow").await;

    // The second workbench, which is a router built afresh over the same store —
    // and pressing on it is what the human who did not see the first press does.
    let over_there = asked.workbench_in_a_hurry();

    let (_, left) = press(&over_there, &request, "allow").await;
    assert!(left.is_empty(), "there is nothing left to be asked about");

    press(&over_there, &request, "deny").await;

    assert_eq!(
        listing(&over_here).await.members.len(),
        1,
        "one press, one member — and a Deny that lost the race takes nothing away",
    );
}

/// A press on a request this device never held is nothing happening, which is
/// the same answer a request that has been settled gets.
#[tokio::test]
async fn a_press_on_a_request_that_was_never_there_does_nothing() {
    let asked = Verkstead::answering().await;
    let over_here = asked.workbench();

    let (_, left) = press(&over_here, "nosuchrequest0000", "allow").await;

    assert!(left.is_empty());
    assert!(listing(&over_here).await.members.is_empty());
}

/// A question whose ten minutes have run out is not one the modal is drawn
/// over, and pressing Allow on it records nobody.
///
/// **Which is how the modal goes by itself.** The page reads this list again at
/// the moment the ten minutes are up — the Nudge the request's own arrival
/// scheduled is what makes it — and the question it was holding open is not in
/// what comes back.
#[tokio::test]
async fn a_question_that_has_run_out_is_not_asked_about_and_cannot_be_allowed() {
    let asked = Verkstead::answering().await;

    hold_join(
        &asked.pool,
        &HeldJoin {
            request: "0011223344556677".to_owned(),
            device: A.to_owned(),
            name: "laptop".to_owned(),
            os: "macOS".to_owned(),
            addresses: vec!["100.64.0.2".to_owned()],
            fingerprint: "AA:BB:CC:DD".to_owned(),
            asked_at: LONG_AGO.to_owned(),
            expires_at: LONG_AGO.to_owned(),
        },
    )
    .await
    .unwrap();

    let over_here = asked.workbench();

    assert!(
        being_asked(&over_here).await.is_empty(),
        "there is nothing left to answer, so there is nothing to draw",
    );

    press(&over_here, "0011223344556677", "allow").await;

    assert!(
        listing(&over_here).await.members.is_empty(),
        "and a press on a modal somebody was still looking at records nobody",
    );
}

/// A join arriving on the peer listener nudges every open workbench of the
/// device it was asked of, which is what raises the modal wherever the human
/// happens to be looking.
///
/// Read off the stream a page really listens on rather than off the channel
/// behind it: what has to hold is that a join landing on *that* listener reaches
/// a page served by *this* one, and the two are one process sharing one handle.
#[tokio::test]
async fn a_join_arriving_nudges_every_open_workbench_of_the_device_asked() {
    let asked = Verkstead::answering().await;
    let asking = Verkstead::asking().await;

    let over_here = asked.workbench();
    let mut page = Listening::open(&over_here).await;

    a_join_asked_of(&asked, &asking).await;

    assert_eq!(page.nudge().await, Nudge::Joins);
}

/// And a press settles it for every workbench: the one that pressed is answered
/// with the list read again, and the one that did not is told the joins moved.
#[tokio::test]
async fn a_press_tells_the_workbench_that_did_not_press() {
    let asked = Verkstead::answering().await;
    let asking = Verkstead::asking().await;

    let request = a_join_asked_of(&asked, &asking).await;

    // The second workbench, listening. Opened after the join so that the Nudge
    // it hears is the press rather than the arrival.
    let over_there = asked.workbench();
    let mut page = Listening::open(&over_there).await;

    press(&asked.workbench_in_a_hurry(), &request, "allow").await;

    assert_eq!(page.nudge().await, Nudge::Joins);
    assert!(
        being_asked(&over_there).await.is_empty(),
        "and what it reads back is a question that is no longer being asked",
    );
}

/// And the presses on the pane itself tell this device's other workbenches, so
/// a pending row appears and goes on a page that pressed nothing.
///
/// **The same arrangement every other way this section moves already makes.** A
/// member naming a newcomer, a member saying a device is out and a press on the
/// modal each say so as they land; Add and Cancel are the same list moving, on
/// the machine the press was made on. Re-reads in the viewer are the Nudge and
/// nothing else — nothing polls — so a second workbench that heard neither
/// would go on drawing the section as it was before the press.
#[tokio::test]
async fn the_presses_on_the_pane_tell_the_workbench_that_did_not_press() {
    let asked = Verkstead::answering().await;
    let asking = Verkstead::asking().await;

    // The second workbench of the device doing the asking, opened before any
    // press so that what it hears is the press.
    let over_here = asking.workbench();
    let mut page = Listening::open(&over_here).await;

    let (status, said) = add(&asking.workbench(), &asked.at()).await;
    assert_eq!(status, StatusCode::OK, "POST {ADD}: {said}");

    assert_eq!(page.nudge().await, Nudge::Devices);

    let pending = listing(&over_here).await.pending;
    assert_eq!(
        pending.len(),
        1,
        "and what it reads back is the row the press left behind",
    );

    assert_eq!(
        cancel(&asking.workbench(), &pending[0].request).await,
        StatusCode::OK,
    );

    assert_eq!(page.nudge().await, Nudge::Devices);
    assert!(
        listing(&over_here).await.pending.is_empty(),
        "and the row it was drawing is gone",
    );
}

/// And a cancel from the device that asked settles it the same way, so a modal
/// standing over a question that has been taken back goes too.
#[tokio::test]
async fn a_cancel_from_the_asking_device_tells_the_workbenches_too() {
    let asked = Verkstead::answering().await;
    let asking = Verkstead::asking().await;

    let request = a_join_asked_of(&asked, &asking).await;

    let over_here = asked.workbench();
    let mut page = Listening::open(&over_here).await;

    assert_eq!(cancel(&asking.workbench(), &request).await, StatusCode::OK);

    assert_eq!(page.nudge().await, Nudge::Joins);
    assert!(being_asked(&over_here).await.is_empty());
}

/// A push that cannot be sent costs the notification and nothing else: the
/// question is written down, the modal is up, and the press works.
///
/// The subscription is a device at an endpoint nothing answers at, which is what
/// a push service being unreachable looks like from here — and the point is that
/// none of it is on the path the join takes.
#[tokio::test]
async fn a_push_that_cannot_be_sent_costs_the_notification_and_not_the_request() {
    let asked = Verkstead::answering().await;
    let asking = Verkstead::asking().await;

    verkstead_store::store_subscription(
        &asked.pool,
        &verkstead_store::PushSubscription {
            endpoint: "http://127.0.0.1:1/nowhere".to_owned(),
            p256dh: "not a key at all".to_owned(),
            auth: "nor is this".to_owned(),
        },
    )
    .await
    .unwrap();

    let request = a_join_asked_of(&asked, &asking).await;

    let over_here = asked.workbench();
    assert_eq!(being_asked(&over_here).await.len(), 1);

    press(&over_here, &request, "allow").await;

    assert_eq!(listing(&over_here).await.members.len(), 1);
}
