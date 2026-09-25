//! The exchange: the dial back that turns a question into a link, and the two
//! Devices lists that read the same afterwards (ADR-0020, *A cluster is a
//! membership*, *The join*).
//!
//! **Both devices answer here, which is the whole difference from
//! `tests/joining.rs`.** That suite is about the question and the press, and A
//! is not listening in it, so a press is where the story stops. Here each
//! Verkstead stands a peer listener up on the loopback and advertises the
//! address it really landed on — so B's dial back is a dial, A's certificate is
//! the one B pinned when the join arrived, and what lands on A is what a device
//! really hands over.
//!
//! **Which is also why the addresses carry a port.** A device advertises places
//! to dial and a peer knocks at each in turn, at the peer port unless the entry
//! says otherwise; the real one would fight whatever is already on 8423 and two
//! of these tests at once would fight each other, so each device here is at the
//! loopback on a port the operating system picked and says so — see
//! [`Reading::advertising`].
//!
//! **And the third un-gated route is proved by being reached.** B is not a
//! member of A when it dials back — it cannot be, the membership being what the
//! call is about to make — so a route behind the member gate could never answer
//! it. What stands in the gate's place is the pending request: the certificate
//! A met when it posted the join, and the ten minutes that request is held for.
//! Both are asked about here, each with a real dial from a device presenting a
//! real certificate.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{WebPkiSupportedAlgorithms, verify_tls12_signature, verify_tls13_signature};
use rustls::{ClientConfig, DigitallySignedStruct, SignatureScheme};
use rustls_pki_types::pem::PemObject;
use rustls_pki_types::{CertificateDer, PrivateKeyDer, ServerName, UnixTime};
use sqlx::SqlitePool;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use tower::ServiceExt;
use verkstead_render::{AskingDevice, DeviceIdentity, DevicesView, JoinSettled};
use verkstead_server::device::reading::Reading;
use verkstead_server::device::{Device, Devices};
use verkstead_server::nudge::Nudges;
use verkstead_server::open_database;
use verkstead_server::peer::dialling::Peers;
use verkstead_server::peer::joining::Joins;
use verkstead_server::peer::{self, Members};
use verkstead_server::platform::{self, Platform};
use verkstead_server::remote::Tailscale;
use verkstead_server::router_answering_devices_telling;
use verkstead_store::{AskedJoin, HeldJoin, Linking};

/// Where the pane reads the section, and where its presses go.
const DEVICES: &str = "/api/ui/devices";
const ADD: &str = "/api/ui/devices/joins";
const ASKING: &str = "/api/ui/devices/asking";

/// The ids the devices are stated as, so that what a test asserts against is a
/// string it chose rather than sixteen random bytes.
///
/// A is the one that presses Add, B the one that is asked, and C a third that
/// neither of them asked anything of — the machine at the wrong end of a dial,
/// and the member B is already holding when a roster is handed over.
const A: &str = "aa00bb11cc22dd33ee44ff5566778899";
const B: &str = "0011223344556677889900aabbccddee";
const C: &str = "ffeeddccbbaa00998877665544332211";

/// What a WSL kernel calls itself, which is the one thing that says one apart
/// from the Linux it is in every other way. A is stated as one so that the OS
/// word crossing the wire is a word this file chose.
const WSL_KERNEL: &str = "5.15.167.4-microsoft-standard-WSL2";

/// The port the workbench is taken to be on, which nothing here asks about:
/// each reading's Tailscale is built with it and never runs.
const PORT: u16 = 8422;

/// A moment well behind any test run: the far side of *has this run out*.
const LONG_AGO: &str = "2020-01-01T00:00:00Z";

/// And one well ahead of it, for a request written straight in and meant to be
/// live.
const LONG_HENCE: &str = "2099-01-01T00:00:00Z";

/// How long a dial in this suite gives one address, rather than the two seconds
/// a running server gives one.
///
/// Spent only where a dial is meant to reach nobody, which here is the machine
/// that answers with the wrong certificate: the handshake fails on the first
/// address and there is no second one, so this is a patience rather than a
/// wait.
const PATIENCE: Duration = Duration::from_millis(300);

/// A path nothing on the peer listener answers, which is what makes it the one
/// worth dialling to ask about the gate.
///
/// Everything that is not a route falls to the gate — see `peer::router` — so
/// a caller this device holds no membership for is refused here, and one it
/// does is let through to a router with nothing in it and misses. Refused and
/// missed are the two answers, and which of them comes back is the whole of
/// what a membership does.
const NOTHING_ANSWERS: &str = "/api/peer/v1/members-only";

/// A machine with no Tailscale on it: `verkstead-no-such-tailscale` is a
/// program that is not there, which is what having none *is*. Never run, the
/// readings here saying their addresses outright.
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
    ///
    /// The listener is bound before the reading is built, because the reading
    /// is what says where this device can be reached and until the socket is
    /// taken there is no answer to that.
    async fn answering(id: &str, kernel: Option<&str>) -> Verkstead {
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

    /// The device that presses Add: a WSL, so that the OS word crossing the
    /// wire is one this file chose.
    async fn asking() -> Verkstead {
        Verkstead::answering(A, Some(WSL_KERNEL)).await
    }

    /// And the device that is asked.
    async fn asked() -> Verkstead {
        Verkstead::answering(B, None).await
    }

    /// Where another device dials it, which is the address it advertises.
    fn at(&self) -> String {
        format!("127.0.0.1:{}", self.address.port())
    }

    /// The workbench its browser would be talking to, which is what the presses
    /// go through.
    ///
    /// Built afresh on each call and over the same store, the way
    /// `tests/joining.rs` builds one: a router is a handle over the rows rather
    /// than a copy of them.
    fn workbench(&self) -> Router {
        router_answering_devices_telling(self.pool.clone(), self.devices(), self.nudges.clone())
    }

    /// And the same with every dial given [`PATIENCE`], for the press whose
    /// dial back is meant to fail.
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

    /// What it says about itself, which is what a roster carries.
    async fn identity(&self) -> DeviceIdentity {
        DeviceIdentity {
            device: self.device.id().to_owned(),
            fingerprint: self.device.fingerprint().to_owned(),
            name: platform::hostname(),
            os: "Linux".to_owned(),
            addresses: vec![self.at()],
        }
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

/// Press Cancel — or Dismiss, which is the same press — on a pending row.
async fn dismiss(app: &Router, request: &str) -> StatusCode {
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

/// Press Allow — or Deny, which goes to the route beside it.
async fn press(app: &Router, request: &str, button: &str) {
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

    assert_eq!(
        status,
        StatusCode::OK,
        "POST {ASKING}/{request}/{button}: {}",
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

/// A join asked of B by A, with the request's own name in hand.
///
/// Every test below starts here: what each is about is what comes back, and
/// nothing comes back until somebody has asked.
async fn a_join_asked_of(asked: &Verkstead, asking: &Verkstead) -> String {
    let (status, said) = add(&asking.workbench(), &asked.at()).await;
    assert_eq!(status, StatusCode::OK, "POST {ADD}: {said}");

    let held = being_asked(&asked.workbench()).await;
    assert_eq!(held.len(), 1, "one question, just asked");

    held[0].request.clone()
}

// ---------------------------------------------------------------------------
// The link
// ---------------------------------------------------------------------------

/// Allow leaves both devices recording each other: the same two devices on
/// either list, and no pending row left on the one that asked.
///
/// **The criterion whole, because the two halves are one call.** What A holds
/// is only there because B handed it over, and what B holds is only there
/// because A asked — and the dial that carried the first of those is the dial
/// that proves the second.
#[tokio::test]
async fn allow_leaves_both_devices_recording_each_other() {
    let asked = Verkstead::asked().await;
    let asking = Verkstead::asking().await;

    let request = a_join_asked_of(&asked, &asking).await;

    press(&asked.workbench(), &request, "allow").await;

    // B's side: A, as A said it was in the join post.
    let over_there = listing(&asked.workbench()).await;
    assert_eq!(over_there.members.len(), 1);

    let a = &over_there.members[0];

    assert_eq!(a.identity.device, A);
    assert_eq!(a.identity.fingerprint, asking.device.fingerprint());
    assert_eq!(
        a.identity.os, "Linux (WSL)",
        "the OS word travels with the join, which is what tells a WSL from the \
         Windows it shares a hostname with",
    );
    assert_eq!(a.identity.addresses, vec![asking.at()]);
    assert!(a.reachable);

    // A's side: B, out of the roster the dial back carried — and nothing left
    // waiting, the waiting having been answered.
    let over_here = listing(&asking.workbench()).await;

    assert!(
        over_here.pending.is_empty(),
        "the row is not waiting on anything any more: there is a member where it \
         was",
    );
    assert_eq!(over_here.members.len(), 1);

    let b = &over_here.members[0];

    assert_eq!(b.identity.device, B);
    assert_eq!(
        b.identity.fingerprint,
        asked.device.fingerprint(),
        "a member *is* a fingerprint, and this one is the certificate A met when \
         it posted the join",
    );
    assert_eq!(b.identity.name, platform::hostname());
    assert_eq!(b.identity.addresses, vec![asked.at()]);
    assert!(b.reachable, "it was answering a moment ago");

    assert!(
        being_asked(&asked.workbench()).await.is_empty(),
        "and the question is let go of rather than left lying about",
    );
}

/// The handover carries every member the device holds rather than that device
/// alone, so a newcomer joining a cluster of two lands holding both.
///
/// **Which is what makes the announcement nothing more than *and now tell the
/// others*.** The roster is one shape whatever the cluster's size — empty
/// beside the introducer in a cluster of two — and a newcomer that had to be
/// told about the rest in some second call would be half linked until that call
/// got through.
#[tokio::test]
async fn the_handover_carries_every_member_the_device_holds() {
    let asked = Verkstead::asked().await;
    let asking = Verkstead::asking().await;

    // A device B is already linked to. Written straight in, because what is
    // being asked is what B *hands over* rather than how it came to hold it.
    verkstead_store::record_member(
        &asked.pool,
        &Linking {
            device: C.to_owned(),
            name: "studio".to_owned(),
            os: "macOS".to_owned(),
            addresses: vec![
                "studio.tailnet-name.ts.net".to_owned(),
                "100.64.0.9".to_owned(),
            ],
            fingerprint: "AA:BB:CC:DD:EE:FF".to_owned(),
        },
    )
    .await
    .unwrap();

    let request = a_join_asked_of(&asked, &asking).await;
    press(&asked.workbench(), &request, "allow").await;

    let mut landed: Vec<String> = listing(&asking.workbench())
        .await
        .members
        .into_iter()
        .map(|member| member.identity.device)
        .collect();

    landed.sort();

    let mut expected = vec![B.to_owned(), C.to_owned()];
    expected.sort();

    assert_eq!(
        landed, expected,
        "the introducer and every member it holds, in the one call",
    );

    // And the member that came along is the whole of what B knew about it: a
    // row recorded off a roster is dialled like any other, so what it carries
    // has to be enough to dial.
    let studio = listing(&asking.workbench())
        .await
        .members
        .into_iter()
        .find(|member| member.identity.device == C)
        .expect("the member handed over");

    assert_eq!(studio.identity.name, "studio");
    assert_eq!(studio.identity.os, "macOS");
    assert_eq!(studio.identity.fingerprint, "AA:BB:CC:DD:EE:FF");
    assert_eq!(
        studio.identity.addresses,
        vec!["studio.tailnet-name.ts.net", "100.64.0.9"],
        "in the order they were advertised, which is the order a dial works down",
    );
}

/// A call A makes to B afterwards is an ordinary member's call, gated on the
/// certificate B now holds for it.
///
/// **Dialled at a path nothing answers**, because what is being asked is the
/// gate rather than any route: everything that is not a route falls to it, so a
/// caller B holds no membership for is refused there and one it does is let
/// through to miss. The two answers are the question.
#[tokio::test]
async fn a_call_afterwards_is_an_ordinary_members_call() {
    let asked = Verkstead::asked().await;
    let asking = Verkstead::asking().await;

    assert_eq!(
        status(&dialled(&asked, &asking.device, NOTHING_ANSWERS).await),
        403,
        "before the exchange A is a device B holds no membership for, whatever \
         certificate it presents",
    );

    let request = a_join_asked_of(&asked, &asking).await;
    press(&asked.workbench(), &request, "allow").await;

    assert_eq!(
        status(&dialled(&asked, &asking.device, NOTHING_ANSWERS).await),
        404,
        "and afterwards it is through the gate — missing a path rather than being \
         refused a membership, which is the gate letting it by",
    );
}

// ---------------------------------------------------------------------------
// What the dial back is matched against
// ---------------------------------------------------------------------------

/// The dial back is answered although the device making it is not a member,
/// and refused once the request's ten minutes are up.
///
/// **Which is why it stands outside the member gate at all.** B cannot be a
/// member of A when it dials: the membership is the thing the call is about to
/// make. What stands in the gate's place is the request — its certificate and
/// its ten minutes — and the second of those is asked about here.
#[tokio::test]
async fn the_dial_back_is_answered_by_a_stranger_and_refused_after_ten_minutes() {
    let asked = Verkstead::asked().await;
    let asking = Verkstead::asking().await;

    // A request A made ten minutes ago, pinned on the certificate B really
    // presents: what refuses this call has to be the expiry rather than the
    // certificate.
    verkstead_store::ask_join(
        &asking.pool,
        &AskedJoin {
            request: "1122334455667788".to_owned(),
            address: asked.at(),
            device: B.to_owned(),
            name: "workbench".to_owned(),
            fingerprint: asked.device.fingerprint().to_owned(),
            asked_at: LONG_AGO.to_owned(),
            expires_at: LONG_AGO.to_owned(),
            refused: false,
        },
    )
    .await
    .unwrap();

    assert!(
        !verkstead_store::member_holding(&asking.pool, asked.device.fingerprint())
            .await
            .unwrap(),
        "B is a stranger to A, which is what a dial back arrives as",
    );

    let refused = asked
        .peers()
        .settle(
            &toward(&asking, "1122334455667788"),
            &JoinSettled::Joined {
                introducer: asked.identity().await,
                members: Vec::new(),
            },
        )
        .await
        .expect_err("a request whose ten minutes are up is answered by nobody");

    assert!(
        format!("{refused:#}").contains("403"),
        "refused rather than missed, and refused by the expiry: {refused:#}",
    );

    assert!(
        listing(&asking.workbench()).await.members.is_empty(),
        "and nothing is recorded off a call that was refused",
    );

    // And the same call inside the ten minutes lands, which is what says the
    // refusal above was the clock rather than the stranger.
    let request = a_join_asked_of(&asked, &asking).await;

    asked
        .peers()
        .settle(
            &toward(&asking, &request),
            &JoinSettled::Joined {
                introducer: asked.identity().await,
                members: Vec::new(),
            },
        )
        .await
        .expect("a live request is answered although the caller is no member");

    assert_eq!(
        listing(&asking.workbench()).await.members.len(),
        1,
        "a stranger's dial back is what makes the membership, so it cannot have \
         needed one",
    );
}

/// B refuses the exchange where the certificate it meets at the asking device's
/// address is not the one the pending request holds.
///
/// A machine answering on that address with some other certificate is not the
/// device that asked, whatever it says about itself — and the handshake is
/// where that is settled, so nothing of this cluster is said to it at all.
#[tokio::test]
async fn a_dial_back_stops_at_a_certificate_the_request_does_not_hold() {
    let asked = Verkstead::asked().await;
    let asking = Verkstead::asking().await;
    let somebody_else = Verkstead::answering(C, None).await;

    let met = asked
        .peers()
        .settle(
            // A's address, pinned on a certificate that is not A's: the request
            // B is holding names the device that really asked.
            &HeldJoin {
                request: "1122334455667788".to_owned(),
                device: A.to_owned(),
                name: "laptop".to_owned(),
                os: "Linux (WSL)".to_owned(),
                addresses: vec![asking.at()],
                fingerprint: somebody_else.device.fingerprint().to_owned(),
                asked_at: LONG_AGO.to_owned(),
                expires_at: LONG_HENCE.to_owned(),
            },
            &JoinSettled::Joined {
                introducer: asked.identity().await,
                members: Vec::new(),
            },
        )
        .await
        .expect_err("the machine at that address is not the device that asked");

    assert!(
        format!("{met:#}").contains(asking.device.fingerprint()),
        "and the refusal says which certificate turned up: {met:#}",
    );

    assert!(
        listing(&asking.workbench()).await.members.is_empty(),
        "nothing of B's cluster reached the machine that answered",
    );
}

/// And A refuses a dial back whose certificate is not the one it met when it
/// posted the join.
///
/// The other half of the pinning, and the one that matters most: a stranger
/// able to reach this port could otherwise answer somebody else's question and
/// be recorded for it.
#[tokio::test]
async fn a_dial_back_under_the_wrong_certificate_is_refused() {
    let asked = Verkstead::asked().await;
    let asking = Verkstead::asking().await;
    let somebody_else = Verkstead::answering(C, None).await;

    let request = a_join_asked_of(&asked, &asking).await;

    let refused = somebody_else
        .peers()
        .settle(
            &toward(&asking, &request),
            &JoinSettled::Joined {
                introducer: somebody_else.identity().await,
                members: Vec::new(),
            },
        )
        .await
        .expect_err("that request was not answered under this certificate");

    assert!(
        format!("{refused:#}").contains("403"),
        "refused by name rather than missed: {refused:#}",
    );

    let still = listing(&asking.workbench()).await;

    assert!(still.members.is_empty(), "and nobody is recorded for it");
    assert_eq!(
        still.pending.len(),
        1,
        "the row is still waiting on the device it really asked",
    );
    assert!(!still.pending[0].refused);
}

/// And an answer that names a certificate other than the one it was dialled
/// under is refused too, whoever dialled it.
///
/// The same judgement the join post makes of an identity: a device that names
/// one certificate and presents another is not the device it says it is.
#[tokio::test]
async fn an_answer_naming_another_certificate_is_refused() {
    let asked = Verkstead::asked().await;
    let asking = Verkstead::asking().await;

    let request = a_join_asked_of(&asked, &asking).await;

    let mut lying = asked.identity().await;
    lying.fingerprint = "AA:BB:CC:DD:EE:FF".to_owned();

    let refused = asked
        .peers()
        .settle(
            &toward(&asking, &request),
            &JoinSettled::Joined {
                introducer: lying,
                members: Vec::new(),
            },
        )
        .await
        .expect_err("a device that names another certificate is not the one it says");

    assert!(
        format!("{refused:#}").contains("403"),
        "refused by name: {refused:#}",
    );

    assert!(listing(&asking.workbench()).await.members.is_empty());
}

/// A roster naming the device reading it leaves no row for that device: a
/// cluster's membership is everybody *else*.
///
/// Nothing sends one — B's members are the devices B is linked to and A is not
/// among them when the roster is read — and a row for this machine on its own
/// list would be a row nothing ever took away.
#[tokio::test]
async fn a_roster_naming_this_device_records_no_row_for_it() {
    let asked = Verkstead::asked().await;
    let asking = Verkstead::asking().await;

    let request = a_join_asked_of(&asked, &asking).await;

    asked
        .peers()
        .settle(
            &toward(&asking, &request),
            &JoinSettled::Joined {
                introducer: asked.identity().await,
                members: vec![asking.identity().await],
            },
        )
        .await
        .unwrap();

    let members = listing(&asking.workbench()).await.members;

    assert_eq!(members.len(), 1, "the introducer, and nothing else");
    assert_eq!(members[0].identity.device, B);
}

// ---------------------------------------------------------------------------
// The answers that are not a link
// ---------------------------------------------------------------------------

/// A Deny reaches the pending row, which reads it and is dismissed — with
/// nothing recorded on either side.
///
/// **Told rather than left to run out**, which is the human's own decision: the
/// ADR spelled the dial back out on an Allow, and without this one the device
/// that asked reads *waiting* for ten minutes over a question that has been
/// answered.
#[tokio::test]
async fn a_deny_reaches_the_pending_row_and_is_dismissed() {
    let asked = Verkstead::asked().await;
    let asking = Verkstead::asking().await;

    let request = a_join_asked_of(&asked, &asking).await;

    press(&asked.workbench(), &request, "deny").await;

    let over_here = listing(&asking.workbench()).await;

    assert_eq!(over_here.pending.len(), 1, "the row stays to be read");
    assert!(
        over_here.pending[0].refused,
        "and it reads that somebody said no rather than going on reading waiting",
    );
    assert!(
        !over_here.pending[0].expired,
        "which is a different thing from nobody having been there",
    );

    assert!(
        over_here.members.is_empty(),
        "nothing is recorded on the device that asked",
    );
    assert!(
        listing(&asked.workbench()).await.members.is_empty(),
        "and nothing on the device that refused",
    );

    // And the same press that cancels a live row clears this one.
    assert_eq!(dismiss(&asking.workbench(), &request).await, StatusCode::OK);
    assert!(listing(&asking.workbench()).await.pending.is_empty());
}

/// An expiry told to the device that asked records nothing: the moment it is
/// about is already on that device's own row.
///
/// What the call is worth is the row redrawing as it happens rather than
/// whenever the page next asks — and a call that never arrives costs nothing,
/// the clock reaching the same answer alone.
#[tokio::test]
async fn an_expiry_told_to_the_asking_device_records_nothing() {
    let asked = Verkstead::asked().await;
    let asking = Verkstead::asking().await;

    let request = a_join_asked_of(&asked, &asking).await;

    asked
        .peers()
        .settle(&toward(&asking, &request), &JoinSettled::Expired)
        .await
        .expect("the request is live, so the device that asked answers");

    let over_here = listing(&asking.workbench()).await;

    assert_eq!(over_here.pending.len(), 1, "the row is still the row");
    assert!(
        !over_here.pending[0].refused,
        "nobody refused it — it ran out, which this device reads off its own row",
    );
    assert!(over_here.members.is_empty());
}

/// And a dial back naming a request this device never asked for is refused,
/// with nothing written.
#[tokio::test]
async fn a_dial_back_naming_no_request_is_refused() {
    let asked = Verkstead::asked().await;
    let asking = Verkstead::asking().await;

    let refused = asked
        .peers()
        .settle(
            &toward(&asking, "nosuchrequest0000"),
            &JoinSettled::Joined {
                introducer: asked.identity().await,
                members: Vec::new(),
            },
        )
        .await
        .expect_err("this device is waiting on no such thing");

    assert!(
        format!("{refused:#}").contains("404"),
        "missed rather than refused: there is nothing here to be refused \
         against: {refused:#}",
    );

    assert!(listing(&asking.workbench()).await.members.is_empty());
}

/// And a press whose dial back reaches nobody still lets the device in: the
/// human pressed Allow, and a machine that was shut is that machine's problem.
///
/// What is left is the state the announcement in the next stage's task is built
/// to mend — a member here that has not been told — and the row over there runs
/// out on its own.
#[tokio::test]
async fn a_dial_back_that_reaches_nobody_still_lets_the_device_in() {
    let asked = Verkstead::asked().await;
    let asking = Verkstead::asking().await;

    // A question naming an address nothing is at, written straight in: what is
    // being asked is what the press does when the dial back fails, and the
    // shape of the failure is not the question.
    verkstead_store::hold_join(
        &asked.pool,
        &HeldJoin {
            request: "1122334455667788".to_owned(),
            device: A.to_owned(),
            name: "laptop".to_owned(),
            os: "Linux (WSL)".to_owned(),
            addresses: vec!["192.168.1.24".to_owned()],
            fingerprint: asking.device.fingerprint().to_owned(),
            asked_at: LONG_AGO.to_owned(),
            expires_at: LONG_HENCE.to_owned(),
        },
    )
    .await
    .unwrap();

    press(&asked.workbench_in_a_hurry(), "1122334455667788", "allow").await;

    let over_there = listing(&asked.workbench()).await;

    assert_eq!(
        over_there.members.len(),
        1,
        "the press stands: the human said yes and the device is a member",
    );
    assert_eq!(over_there.members[0].identity.device, A);

    assert!(
        being_asked(&asked.workbench()).await.is_empty(),
        "and the question is settled rather than left to be pressed again",
    );
}

// ---------------------------------------------------------------------------
// The plumbing
// ---------------------------------------------------------------------------

/// A request as the device that was asked holds one, pointed at `asking`.
///
/// What a dial back is made of is the addresses to try and the certificate to
/// insist on, and both are the asking device's — so for the tests that dial
/// rather than press, this is the request said from the outside.
fn toward(asking: &Verkstead, request: &str) -> HeldJoin {
    HeldJoin {
        request: request.to_owned(),
        device: A.to_owned(),
        name: "laptop".to_owned(),
        os: "Linux (WSL)".to_owned(),
        addresses: vec![asking.at()],
        fingerprint: asking.device.fingerprint().to_owned(),
        asked_at: LONG_AGO.to_owned(),
        expires_at: LONG_HENCE.to_owned(),
    }
}

/// Dial `listening` presenting `showing`'s certificate, ask for `path`, and
/// hand back the response as it arrived.
///
/// A dial of this suite's own rather than the product's, because what is being
/// asked is what the *gate* answers a member: the product dials the routes it
/// knows, and the path that proves a gate is the one no route answers.
///
/// `Connection: close`, so that reading to the end of the stream is reading to
/// the end of the response and this suite needs no HTTP client of its own.
async fn dialled(listening: &Verkstead, showing: &Device, path: &str) -> String {
    let name = ServerName::try_from(listening.device.id().to_owned()).unwrap();

    let connection = TcpStream::connect(listening.address).await.unwrap();
    let mut secured = TlsConnector::from(Arc::new(presenting(showing)))
        .connect(name, connection)
        .await
        .expect("the handshake should complete: this listener takes what arrives");

    secured
        .write_all(
            format!(
                "GET {path} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
                listening.device.id(),
            )
            .as_bytes(),
        )
        .await
        .unwrap();

    let mut answered = Vec::new();
    secured.read_to_end(&mut answered).await.unwrap();

    String::from_utf8(answered).unwrap()
}

/// The client that dial is made with: `device`'s certificate presented, and
/// whatever the server shows accepted — what the far end presents is not what
/// this question is about.
fn presenting(device: &Device) -> ClientConfig {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let algorithms = provider.signature_verification_algorithms;
    let pem = device.certificate().as_bytes();

    ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .unwrap()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(WhateverTheServerShows { algorithms }))
        .with_client_auth_cert(
            vec![CertificateDer::from_pem_slice(pem).unwrap()],
            PrivateKeyDer::from_pem_slice(pem).unwrap(),
        )
        .expect("a device's own certificate and the key that signed it")
}

/// The status of a response read that way, which is what says a refusal apart
/// from a miss.
fn status(answered: &str) -> u16 {
    answered
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap_or_else(|| panic!("a response starts with a status line, got:\n{answered}"))
}

/// The client half of *whatever arrives is taken*, for the one dial here that
/// is not the product's.
#[derive(Debug)]
struct WhateverTheServerShows {
    algorithms: WebPkiSupportedAlgorithms,
}

impl ServerCertVerifier for WhateverTheServerShows {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls12_signature(message, cert, dss, &self.algorithms)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls13_signature(message, cert, dss, &self.algorithms)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.algorithms.supported_schemes()
    }
}
