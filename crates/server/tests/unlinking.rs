//! **Unlink**: a device taken out of the cluster for everybody, off one press
//! (ADR-0020, *A cluster is a membership*).
//!
//! **Three servers again, and for the reason the announcement needed three.**
//! A membership is not a set of pairs, so what has to be asked is what nobody
//! can ask of two: that the press on one machine takes the device off *every*
//! list, that the device itself is told to let go of the cluster, and that the
//! member which was not reachable at the moment of the press is told when it
//! next answers. Each Verkstead here stands a real peer listener up and every
//! call between them is a real dial over a real handshake.
//!
//! **And the gate is what makes the unlink mean something.** A device dropped
//! from a membership is a certificate that membership no longer holds, so its
//! very next call is refused at every member — which is what is asked here
//! rather than assumed, the gate reading the rows afresh at every call
//! precisely so that an unlink takes effect on the next one.

use std::net::SocketAddr;
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use sqlx::SqlitePool;
use tower::ServiceExt;
use verkstead_render::{AskingDevice, DeviceIdentity, DevicesView};
use verkstead_schema::Nudge;
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
use verkstead_store::{Linking, Telling};

/// Where the pane reads the section, and where its presses go.
const DEVICES: &str = "/api/ui/devices";
const ADD: &str = "/api/ui/devices/joins";
const ASKING: &str = "/api/ui/devices/asking";

/// And where an open page listens for the word that something moved.
const NUDGES: &str = "/api/ui/nudges";

/// How long a test will wait for a Nudge it expects: generous, because it is
/// only ever paid when the assertion is about to fail.
const HEARING: Duration = Duration::from_secs(5);

/// And the press this suite is about.
fn unlink_at(device: &str) -> String {
    format!("/api/ui/devices/members/{device}/unlink")
}

/// The three devices that make the cluster, named by the ids a cluster names
/// them by.
const A: &str = "aa00bb11cc22dd33ee44ff5566778899";
const B: &str = "0011223344556677889900aabbccddee";
const C: &str = "ffeeddccbbaa00998877665544332211";

/// And a fourth, for the join that is the next thing the cluster does after a
/// member comes back.
const D: &str = "99887766554433221100ffeeddccbbaa";

/// The port the workbench is taken to be on, which nothing here asks about.
const PORT: u16 = 8422;

/// How long a dial in this suite gives one address, rather than the two seconds
/// a running server gives one.
///
/// Spent where a dial is meant to reach nobody, which here is the member that
/// is not there when the human presses Unlink: what is being asked is that the
/// press stands whole anyway, and a test that waited out the real deadline
/// would be spending its time on the clock rather than on the question.
const PATIENCE: Duration = Duration::from_millis(300);

/// A machine with no Tailscale on it.
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
    /// broadcast is meant to reach somebody who is not there.
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

    /// And every device on its own list, by id and sorted.
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

/// One of this device's other open workbenches, listening on the Nudge stream.
///
/// Read off the wire rather than off the channel behind it, the way
/// `tests/joining.rs` reads one: the press is answered to the workbench that
/// made it, and what has to hold is that a page which made no press hears about
/// it down its own connection.
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

/// Press Unlink on a member's row, and hand back the section as it answered.
///
/// The asking is the browser's — one card over the page, naming the device, as
/// Remove on a Repo is — so what reaches the server is the press already
/// confirmed.
async fn unlink(app: &Router, device: &str) -> DevicesView {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(unlink_at(device))
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
        "POST {}: {}",
        unlink_at(device),
        String::from_utf8_lossy(&bytes),
    );

    serde_json::from_slice(&bytes).unwrap()
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

/// `joining` links itself to `answering`, all the way through.
async fn linked(joining: &Verkstead, answering: &Verkstead) {
    let (status, said) = add(&joining.workbench(), &answering.at()).await;
    assert_eq!(status, StatusCode::OK, "POST {ADD}: {said}");

    let held = being_asked(&answering.workbench()).await;
    assert_eq!(held.len(), 1, "one question, just asked");

    allow(&answering.workbench(), &held[0].request).await;
}

// ---------------------------------------------------------------------------
// One press, every list
// ---------------------------------------------------------------------------

/// Unlink on a member's row drops it here, tells every other member to drop it,
/// and tells the leaver to forget everyone.
///
/// **The stage's own demonstration, the other way round.** A join joins a
/// device to everybody off one press; an unlink takes it off everybody's list
/// off one press, and the leaver ends up holding nothing. A cluster is a
/// membership rather than a set of pairs, so there is no state here in which
/// two of the three lists agree and the third does not.
#[tokio::test]
async fn one_press_takes_a_device_off_every_list_including_its_own() {
    let a = Verkstead::answering(A).await;
    let b = Verkstead::answering(B).await;
    let c = Verkstead::answering(C).await;

    linked(&a, &b).await;
    linked(&c, &b).await;

    assert_eq!(a.cluster().await, sorted(&[B, C]), "three, linked");

    // Pressed on A, about C — which is a device A was never asked about: B
    // announced it. What A is pressing is a membership rather than its own half
    // of a link.
    let after = unlink(&a.workbench(), C).await;

    assert_eq!(
        after
            .members
            .into_iter()
            .map(|member| member.identity.device)
            .collect::<Vec<_>>(),
        vec![B.to_owned()],
        "the press answers with the section read again, already without it",
    );

    assert_eq!(a.cluster().await, sorted(&[B]));
    assert_eq!(
        b.cluster().await,
        sorted(&[A]),
        "B was never pressed and holds no C: the broadcast is what a membership \
         means",
    );
    assert!(
        c.cluster().await.is_empty(),
        "and the leaver was told to let go of the cluster, rather than left \
         holding two machines that no longer hold it",
    );
}

/// And the leaver's own list is empty but for itself, which is what its card's
/// count reads off.
#[tokio::test]
async fn the_leavers_list_is_its_own_row_alone() {
    let a = Verkstead::answering(A).await;
    let b = Verkstead::answering(B).await;

    linked(&a, &b).await;

    unlink(&a.workbench(), B).await;

    let listed = listing(&b.workbench()).await;

    assert_eq!(
        listed.this.device, B,
        "the row read off this machine is still there: a Verkstead linked to \
         nothing still has an identity",
    );
    assert!(
        listed.members.is_empty(),
        "and nothing beside it, which is what the card's count is taken from",
    );
    assert!(listed.pending.is_empty(), "and nothing pending either");
}

/// This device's own row is not one an Unlink names, and pressing one against
/// its id changes nothing.
///
/// The pane draws no press on that row — there being nothing to unlink this
/// machine from itself — and the server is the same shrug a second press on a
/// member's row gets: a device that is not a member is one this machine has
/// already unlinked.
#[tokio::test]
async fn this_devices_own_id_names_nothing_to_unlink() {
    let a = Verkstead::answering(A).await;
    let b = Verkstead::answering(B).await;

    linked(&a, &b).await;

    let after = unlink(&a.workbench(), A).await;

    assert_eq!(
        after
            .members
            .into_iter()
            .map(|member| member.identity.device)
            .collect::<Vec<_>>(),
        vec![B.to_owned()],
        "this device is not on its own membership, so there is nothing there to \
         take away",
    );
    assert_eq!(
        b.cluster().await,
        sorted(&[A]),
        "and nothing was broadcast about it either",
    );
}

/// A second press is not a second thing happening.
#[tokio::test]
async fn unlinking_a_device_twice_is_unlinking_it_once() {
    let a = Verkstead::answering(A).await;
    let b = Verkstead::answering(B).await;

    linked(&a, &b).await;

    unlink(&a.workbench(), B).await;

    let after = unlink(&a.workbench(), B).await;

    assert!(
        after.members.is_empty(),
        "a device that is not a member is one this machine has already unlinked",
    );
}

// ---------------------------------------------------------------------------
// The gate
// ---------------------------------------------------------------------------

/// A device that has been unlinked is refused at the member gate on its next
/// call, and by every remaining member too.
///
/// **Which is the whole of what an unlink has to mean.** The membership is read
/// afresh at every call precisely so that this is true on the next one rather
/// than at the next restart — a cached membership would be a device that went
/// on being admitted after the human took it out.
#[tokio::test]
async fn an_unlinked_device_is_refused_at_every_members_gate() {
    let a = Verkstead::answering(A).await;
    let b = Verkstead::answering(B).await;
    let c = Verkstead::answering(C).await;

    linked(&a, &b).await;
    linked(&c, &b).await;

    // While it is a member, C's own call to A gets through — which is what the
    // announcement bought it.
    c.peers()
        .announce(&member_at(&a, C), &b.identity())
        .await
        .expect("a member's own call to another member");

    unlink(&b.workbench(), C).await;

    for (standing, device) in [(&a, A), (&b, B)] {
        let refused = c
            .peers()
            .announce(&member_at(standing, C), &b.identity())
            .await
            .expect_err("a device that is not a member is refused at the gate");

        assert!(
            format!("{refused:#}").contains("403"),
            "refused at {device}'s gate rather than anywhere else: {refused:#}",
        );
    }
}

// ---------------------------------------------------------------------------
// A member that is not there
// ---------------------------------------------------------------------------

/// Unlink works on a member that is dimmed *unreachable*, and the cluster the
/// human is standing in is left correct although the leaver could not be told.
///
/// **Which is most of why Unlink exists at all.** The machine somebody reaches
/// for this on is the one that is never coming back — so nothing waits on
/// telling it, and the row it leaves behind is gone from this device and from
/// every member that answers.
#[tokio::test]
async fn a_member_that_is_not_there_is_unlinked_all_the_same() {
    let a = Verkstead::answering(A).await;
    let b = Verkstead::answering(B).await;

    linked(&a, &b).await;

    // A third member of A's that is not there: an address nothing is at,
    // written straight in, because what is being asked is what the press does
    // when the leaver cannot be told rather than how it came to be recorded.
    verkstead_store::record_member(
        &a.pool,
        &Linking {
            device: C.to_owned(),
            name: "laptop".to_owned(),
            os: "Linux (WSL)".to_owned(),
            addresses: vec!["127.0.0.1:1".to_owned()],
            fingerprint: "AA:BB:CC:DD:EE:FF".to_owned(),
        },
    )
    .await
    .unwrap();

    let after = unlink(&a.workbench_in_a_hurry(), C).await;

    assert_eq!(
        after
            .members
            .into_iter()
            .map(|member| member.identity.device)
            .collect::<Vec<_>>(),
        vec![B.to_owned()],
        "the press stands whole: the machine that was off is gone from the \
         cluster the human is standing in",
    );
    assert_eq!(
        b.cluster().await,
        sorted(&[A]),
        "and the member that *is* there was told, although the leaver was not",
    );
}

/// And the press tells this device's *other* open workbenches, which is the one
/// way a page here finds out about a row that has gone.
///
/// **Every other way this section moves already says so.** A member naming a
/// newcomer, a member saying a device is out and a member's renewed certificate
/// each announce it as they land; the press made over here is the same list
/// moving. Re-reads in the viewer are the Nudge and nothing else — nothing polls
/// — so without this the one press that takes a row away would be the one change
/// a second workbench of the pressing device went on drawing the old answer for,
/// while every other device in the cluster had it right.
#[tokio::test]
async fn the_press_tells_this_devices_other_workbenches_too() {
    let a = Verkstead::answering(A).await;
    let b = Verkstead::answering(B).await;

    linked(&b, &a).await;

    // The second workbench, opened after the join so that what it hears is the
    // press rather than the linking.
    let over_here = a.workbench();
    let mut page = Listening::open(&over_here).await;

    unlink(&a.workbench(), B).await;

    assert_eq!(page.nudge().await, Nudge::Devices);
    assert!(
        listing(&over_here).await.members.is_empty(),
        "and what it reads back is the cluster without the device that has gone",
    );
}

/// A member that could not be reached is recorded as still owed the removal,
/// and is told when it next answers.
///
/// **Nothing retries in a loop.** The debt is the record, and what pays it is
/// the next dial that finds that member answering — which here is the next
/// thing the cluster does.
#[tokio::test]
async fn a_member_that_was_away_is_told_the_removal_when_it_answers() {
    let a = Verkstead::answering(A).await;
    let b = Verkstead::answering(B).await;
    let c = Verkstead::answering(C).await;

    linked(&a, &b).await;
    linked(&c, &b).await;

    // B goes away: its row on A's list keeps its certificate and is pointed at
    // an address nothing is at, which is what a laptop with its lid shut looks
    // like from here.
    verkstead_store::record_member(
        &a.pool,
        &Linking {
            device: B.to_owned(),
            name: "workbench".to_owned(),
            os: "Linux".to_owned(),
            addresses: vec!["127.0.0.1:1".to_owned()],
            fingerprint: b.device.fingerprint().to_owned(),
        },
    )
    .await
    .unwrap();

    unlink(&a.workbench_in_a_hurry(), C).await;

    assert_eq!(
        a.cluster().await,
        sorted(&[B]),
        "the press stands whole on the machine it was made on",
    );
    assert_eq!(
        b.cluster().await,
        sorted(&[A, C]),
        "and B, which was not there for it, is still holding the device that left",
    );

    assert_eq!(
        verkstead_store::announcements_owed_to(&a.pool, B)
            .await
            .unwrap(),
        vec![(C.to_owned(), Telling::Removed)],
        "so the telling that did not get through is written down as owed, and \
         as the removal rather than as anything else",
    );

    let dimmed = listing(&a.workbench())
        .await
        .members
        .into_iter()
        .find(|member| member.identity.device == B)
        .expect("the member that was not there");

    assert!(
        !dimmed.reachable,
        "and the row is dimmed, the dial having reached none of its addresses",
    );

    // B comes back: its row is pointed at the machine again.
    verkstead_store::record_member(
        &a.pool,
        &Linking {
            device: B.to_owned(),
            name: "workbench".to_owned(),
            os: "Linux".to_owned(),
            addresses: vec![b.at()],
            fingerprint: b.device.fingerprint().to_owned(),
        },
    )
    .await
    .unwrap();

    // And the next thing the cluster does is what finds it answering: a fourth
    // device joins through A, so A dials B to announce it. Nothing here is a
    // retry — something dials a member whenever the cluster does anything, and
    // that dial is the trigger.
    let d = Verkstead::answering(D).await;
    linked(&d, &a).await;

    assert_eq!(
        b.cluster().await,
        sorted(&[A, D]),
        "B was told what it missed on the first call that got through to it: \
         the newcomer it was dialled about, and the removal it was owed",
    );
    assert!(
        verkstead_store::announcements_owed_to(&a.pool, B)
            .await
            .unwrap()
            .is_empty(),
        "and the debt is paid rather than said again",
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
