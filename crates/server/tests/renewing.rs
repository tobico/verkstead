//! The renewal announced: a device that made its certificate again, and a cluster
//! that never notices (ADR-0020, *The certificate is renewed before it runs out*).
//!
//! **Three servers, because the whole of the difficulty is the third one.** Each
//! Verkstead here stands a peer listener up on the loopback and advertises the
//! address it really landed on, so every call between them is a real dial over a
//! real handshake against a real certificate — the join, the announcement of the
//! renewal, and the calls in both directions that have to go on working while it
//! is in flight. With two devices a changeover is over the moment the other one
//! answers; with three, one of them can be switched off, and the question is what
//! the *other* two do in the meantime.
//!
//! **The answer is that both certificates are accepted.** A member records the
//! incoming fingerprint against the same Device Id and keeps the outgoing one
//! beside it, because the renewing device goes on presenting the outgoing one
//! until the last member has acknowledged and tells nobody when that was. A
//! member that let go the moment it acknowledged would be a member refusing the
//! device it had just acknowledged, for as long as somebody's laptop stayed shut.
//!
//! **And a start near the expiry is stood at by a certificate with less life in
//! it**, rather than by a clock this process does not keep — see
//! [`Device::stated_good_for`]. The re-issue that follows is the one a start makes
//! on a machine that has been running for two months.

use std::net::SocketAddr;
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use sqlx::SqlitePool;
use tower::ServiceExt;
use verkstead_render::{AskingDevice, DeviceIdentity, DevicesView, RenewedCertificate};
use verkstead_server::device::reading::Reading;
use verkstead_server::device::{Changeover, Device, Devices, RENEW_WITHIN};
use verkstead_server::nudge::Nudges;
use verkstead_server::open_database;
use verkstead_server::peer::dialling::Peers;
use verkstead_server::peer::joining::Joins;
use verkstead_server::peer::{self, Members};
use verkstead_server::platform::Platform;
use verkstead_server::remote::Tailscale;
use verkstead_server::router_answering_devices_telling;
use verkstead_store::{Linking, Member, Telling};

/// Where the pane reads the section, and where its presses go.
const DEVICES: &str = "/api/ui/devices";
const ADD: &str = "/api/ui/devices/joins";
const ASKING: &str = "/api/ui/devices/asking";

/// The devices, named by the ids a cluster names them by.
///
/// A is the one that renews, and every question here is asked of what B and C
/// make of it. D is the newcomer for the one question about a join arriving in the
/// middle of a changeover.
const A: &str = "aa00bb11cc22dd33ee44ff5566778899";
const B: &str = "0011223344556677889900aabbccddee";
const C: &str = "ffeeddccbbaa00998877665544332211";
const D: &str = "1122334455667788990011aabbccddff";

/// And a device that is nobody's member, for the question about the gate.
const STRANGER: &str = "99887766554433221100ffeeddccbbaa";

/// An address on the loopback that nothing is listening on, which is what a
/// member whose machine is switched off looks like from here.
const NOWHERE: &str = "127.0.0.1:1";

/// The port the workbench is taken to be on, which nothing here asks about.
const PORT: u16 = 8422;

/// How long a dial in this suite gives one address, rather than the two seconds a
/// running server gives one.
///
/// Spent only where a dial is meant to reach nobody, which here is the member
/// whose machine is not there.
const PATIENCE: Duration = Duration::from_millis(300);

/// A day, for standing a start inside the renewal window.
const A_DAY: time::Duration = time::Duration::days(1);

/// A machine with no Tailscale on it: `verkstead-no-such-tailscale` is a program
/// that is not there, which is what having none *is*.
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

    /// The Data Directory: held for the length of the test, and read again by the
    /// re-issue below — a changeover is two files in here.
    dir: tempfile::TempDir,
}

impl Verkstead {
    /// A Verkstead with a store of its own, an identity in it good for its whole
    /// ninety days, and its peer listener up on a port the machine picked.
    async fn answering(id: &str) -> Verkstead {
        Verkstead::answering_with(id, verkstead_server::device::VALIDITY).await
    }

    /// And one whose certificate is inside the renewal window, so that the next
    /// start makes it again.
    async fn nearly_expired(id: &str) -> Verkstead {
        Verkstead::answering_with(id, RENEW_WITHIN - A_DAY).await
    }

    async fn answering_with(id: &str, good_for: time::Duration) -> Verkstead {
        let dir = tempfile::tempdir().unwrap();
        let pool = open_database(&dir.path().join("verkstead.db"))
            .await
            .unwrap();

        let device = Device::stated_good_for(dir.path(), id, good_for).unwrap();

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
            dir,
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

    /// The start that makes the certificate again: a fresh one beside the one still
    /// going out, and what that start found owed.
    ///
    /// **The listener is left exactly as it is, and that is faithful rather than a
    /// shortcut.** What a device presents over a changeover *is* the outgoing
    /// certificate — the one this listener was stood up with — so a process that
    /// re-issued at its start and one that re-issued after it present the same
    /// bytes to the same members. What the fresh handle changes is what this device
    /// *dials* with and what it knows it owes, which is the whole of what a renewal
    /// is.
    async fn re_issued(&mut self) -> Changeover {
        let device = Device::issued(self.dir.path(), &self.members)
            .await
            .unwrap();
        let changeover = device.changeover();

        self.device = device;
        changeover
    }

    /// And the start after all this, which is what reads the files a finished
    /// changeover left behind.
    async fn started_again(&self) -> Device {
        Device::issued(self.dir.path(), &self.members)
            .await
            .unwrap()
    }

    /// Where another device dials it, which is the address it advertises.
    fn at(&self) -> String {
        format!("127.0.0.1:{}", self.address.port())
    }

    /// The workbench its browser would be talking to, which is what the presses go
    /// through.
    fn workbench(&self) -> Router {
        router_answering_devices_telling(self.pool.clone(), self.devices(), self.nudges.clone())
    }

    /// And the same with every dial given [`PATIENCE`], for the presses whose
    /// calls are meant to reach nobody.
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

    /// And what it says when it has made its certificate again: itself as it
    /// answers anybody, with the fingerprint coming in beside it.
    fn renewing(&self) -> RenewedCertificate {
        RenewedCertificate {
            identity: self.identity(),
            incoming: self
                .device
                .incoming_fingerprint()
                .expect("a device in the middle of a changeover")
                .to_owned(),
        }
    }

    /// The row this device holds for `device`, as the table holds it: which is
    /// where the two halves of somebody else's changeover are.
    async fn holds(&self, device: &str) -> Member {
        verkstead_store::members(&self.pool)
            .await
            .unwrap()
            .into_iter()
            .find(|member| member.device == device)
            .unwrap_or_else(|| panic!("device {device} should be a member here"))
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

// ---------------------------------------------------------------------------
// The changeover, from every side
// ---------------------------------------------------------------------------

/// A device that re-issues its certificate tells every member, each of them
/// records the new fingerprint against the same id, and the changeover is over.
///
/// **Which is the whole task in one test.** Every call in this is a real dial: the
/// two joins that make the cluster, the two announcements of the renewal, and the
/// calls in both directions afterwards that say the cluster still works.
#[tokio::test]
async fn a_device_that_re_issues_tells_every_member_and_stays_reachable() {
    let mut a = Verkstead::nearly_expired(A).await;
    let b = Verkstead::answering(B).await;
    let c = Verkstead::answering(C).await;

    linked(&b, &a).await;
    linked(&c, &a).await;

    let outgoing = a.device.fingerprint().to_owned();

    assert_eq!(
        a.re_issued().await,
        Changeover::YetToTell(2),
        "two members, neither of which has acknowledged anything: a changeover in \
         flight rather than one that completed at the start that began it",
    );

    let incoming = a
        .device
        .incoming_fingerprint()
        .expect("a fresh certificate was made")
        .to_owned();

    assert_ne!(incoming, outgoing);
    assert_eq!(
        a.device.fingerprint(),
        outgoing,
        "and the old one is still what it presents, which is what every member holds",
    );

    a.devices().announce_renewal().await;

    for member in [&b, &c] {
        let held = member.holds(A).await;

        assert_eq!(
            held.fingerprint, incoming,
            "the new fingerprint is recorded against the same device id — nothing \
             here is a device leaving and another arriving",
        );
        assert_eq!(
            held.renewing_from.as_deref(),
            Some(outgoing.as_str()),
            "and the one A is still presenting is kept beside it, because A goes on \
             presenting it until the last of them has acknowledged",
        );
        assert_eq!(
            member.cluster().await,
            sorted(&[A, other(member.device.id())]),
            "one row for A rather than two",
        );
    }

    // Both of them acknowledged, so the changeover is over: the certificate that
    // was waiting is the one this Data Directory holds, and the start after this
    // presents it.
    assert!(
        !a.device.incoming_path().exists(),
        "the file the new one was waiting in goes with the changeover",
    );

    let after = a.started_again().await;

    assert_eq!(
        after.fingerprint(),
        incoming,
        "the next start presents the certificate its members have acknowledged",
    );
    assert_eq!(after.incoming_fingerprint(), None);
    assert_eq!(after.changeover(), Changeover::NotDue);

    // And every call in every direction goes on working, which is the point of all
    // of it. A dials out under the certificate it is still presenting; B and C dial
    // in and are answered with the same one, against a row that now holds the
    // other.
    for member in [&b, &c] {
        a.peers()
            .identity(&a.holds(member.device.id()).await)
            .await
            .expect("a device that renewed can still call its members");

        member
            .peers()
            .identity(&member.holds(A).await)
            .await
            .expect("and its members can still call it");
    }
}

/// Three servers, one of them re-issuing: every call in every direction works
/// while the changeover is in flight, and every one of them works afterwards.
///
/// **The stage's own demonstration, and the case two servers cannot make.** C is
/// switched off, so A goes on presenting the certificate it was presenting and the
/// changeover cannot finish — and B, which has already acknowledged the new
/// fingerprint and holds it against A's id, has to go on talking to A the whole
/// time. A member that had let go of the outgoing certificate the moment it
/// acknowledged would be exactly the member that stopped working here.
#[tokio::test]
async fn every_call_works_while_the_changeover_is_in_flight_and_after_it() {
    let mut a = Verkstead::nearly_expired(A).await;
    let b = Verkstead::answering(B).await;
    let c = Verkstead::answering(C).await;

    linked(&b, &a).await;
    linked(&c, &a).await;

    // C, switched off between the join and the re-issue.
    switched_off(&a, C).await;

    let outgoing = a.device.fingerprint().to_owned();

    a.re_issued().await;
    a.devices().waiting(PATIENCE).announce_renewal().await;

    let incoming = a.device.incoming_fingerprint().unwrap().to_owned();

    assert!(
        a.device.incoming_path().exists(),
        "C never answered, so the changeover is in flight — which is the state \
         every call below is made in",
    );
    assert_eq!(
        b.holds(A).await.fingerprint,
        incoming,
        "and B has acknowledged, so B holds a fingerprint A is not presenting",
    );

    // Out from the device that is changing over, and in to it, against a row that
    // holds one certificate and meets the other.
    every_direction(&a, &b).await;

    // And B's own calls to the third device, which the changeover has nothing to do
    // with and must not have broken.
    every_direction(&b, &c).await;

    // Now C answers, which finishes the changeover: the certificate that was
    // waiting is the one this Data Directory holds.
    back_again(&a, &c).await;

    a.devices().announce_renewal().await;

    assert!(!a.device.incoming_path().exists());
    assert_eq!(c.holds(A).await.fingerprint, incoming);

    // And every one of them again, the changeover being over. A still presents the
    // outgoing certificate until its next start, which is exactly why B and C kept
    // it beside the one they acknowledged.
    every_direction(&a, &b).await;
    every_direction(&a, &c).await;
    every_direction(&b, &c).await;

    assert_eq!(
        a.started_again().await.fingerprint(),
        incoming,
        "and the start after this presents the one every member holds",
    );
    assert_ne!(incoming, outgoing);
}

/// And the announcement is made presenting the outgoing certificate, which is not
/// a choice: it is the only one any member holds.
///
/// **Read off the receiving end**, because that is the only place it is a fact: the
/// certificate written down as the one still going out is the one B's own handshake
/// handed over, rather than anything the payload claimed.
#[tokio::test]
async fn the_announcement_is_made_presenting_the_outgoing_certificate() {
    let mut a = Verkstead::nearly_expired(A).await;
    let b = Verkstead::answering(B).await;

    linked(&b, &a).await;

    let outgoing = a.device.fingerprint().to_owned();

    a.re_issued().await;
    a.devices().announce_renewal().await;

    assert_eq!(
        b.holds(A).await.renewing_from.as_deref(),
        Some(outgoing.as_str()),
        "what B wrote down as still going out is the certificate its own handshake \
         met, so the call was made under the one B held rather than the one it did \
         not",
    );
}

/// A renewal naming a certificate the caller is not presenting is refused.
///
/// **Because the payload is the one half of this that anybody can write.** The
/// fingerprint in the identity is checked against what the connection actually
/// carried — the same check a dial makes of an identity answer — and without it a
/// member could name any certificate as the one it was presenting and leave the
/// receiver accepting something it had never met.
#[tokio::test]
async fn a_renewal_naming_a_certificate_it_is_not_presenting_is_refused() {
    let mut a = Verkstead::nearly_expired(A).await;
    let b = Verkstead::answering(B).await;

    linked(&b, &a).await;

    a.re_issued().await;

    let mut lying = a.renewing();
    lying.identity.fingerprint = "AA:BB:CC:DD:EE:FF".to_owned();

    let refused = a
        .peers()
        .announce_renewal(&a.holds(B).await, &lying)
        .await
        .expect_err("a device that names a certificate it is not presenting is refused");

    assert!(
        format!("{refused:#}").contains("400"),
        "refused for what the payload said rather than for who said it: {refused:#}",
    );

    let held = b.holds(A).await;

    assert_eq!(held.renewing_from, None, "and nothing about B's row moved");
}

/// And a renewal from a device that is not a member is refused at the gate.
///
/// **Which is the whole of what stands between a cluster and a stranger rewriting
/// its fingerprints.** The announcement is a member's own call: a device B holds no
/// membership for cannot be the device whose certificate is changing, and it is
/// refused without a line in the route having been written to refuse it.
#[tokio::test]
async fn a_renewal_from_a_stranger_is_refused_at_the_gate() {
    let mut stranger = Verkstead::nearly_expired(STRANGER).await;
    let b = Verkstead::answering(B).await;
    let a = Verkstead::answering(A).await;

    linked(&a, &b).await;

    // A member of the stranger's own, so that its changeover has somebody to wait
    // on and there is an incoming certificate for it to announce at all. Nobody
    // dials it: what is being asked is what B makes of the call.
    switched_off(&stranger, C).await;

    stranger.re_issued().await;

    let refused = stranger
        .peers()
        .announce_renewal(&member_at(&b, B), &stranger.renewing())
        .await
        .expect_err("a device B holds no membership for is refused at B's gate");

    assert!(
        format!("{refused:#}").contains("403"),
        "refused at the gate rather than missed at a path: {refused:#}",
    );

    assert_eq!(
        b.cluster().await,
        sorted(&[A]),
        "and nothing a stranger said is on B's list",
    );
}

/// A member cannot announce a renewal of another device's certificate.
///
/// **Through the gate is not the same as speaking for everybody.** A cluster is a
/// membership, so every member can reach this route — and a member able to say
/// which fingerprint stands against *another* member's id could cut that device out
/// of this one's cluster with one call.
#[tokio::test]
async fn a_member_cannot_announce_another_devices_renewal() {
    let mut a = Verkstead::nearly_expired(A).await;
    let b = Verkstead::answering(B).await;
    let c = Verkstead::answering(C).await;

    linked(&b, &a).await;
    linked(&c, &a).await;

    a.re_issued().await;

    // A, saying the words about C rather than about itself.
    let mut about_c = a.renewing();
    about_c.identity.device = C.to_owned();

    let refused = a
        .peers()
        .announce_renewal(&a.holds(B).await, &about_c)
        .await
        .expect_err("a device may announce its own certificate and no other");

    assert!(
        format!("{refused:#}").contains("403"),
        "refused for naming somebody else: {refused:#}",
    );

    assert_eq!(
        b.holds(C).await.fingerprint,
        c.device.fingerprint(),
        "and C is still the certificate C presents, on B's own list",
    );
}

// ---------------------------------------------------------------------------
// A member that was not there
// ---------------------------------------------------------------------------

/// A member that was switched off when the certificate was made again leaves the
/// changeover in flight, is dimmed, and is owed the telling.
///
/// **Nothing here fails and nothing here retries.** The certificate was made again
/// whatever some third machine made of it, so the old one goes on going out, the
/// startup line names both fingerprints, and the debt waits for the next call that
/// gets through — which is what a cluster does about a laptop with its lid shut.
#[tokio::test]
async fn a_member_that_was_not_there_leaves_the_changeover_in_flight() {
    let mut a = Verkstead::nearly_expired(A).await;
    let b = Verkstead::answering(B).await;

    linked(&b, &a).await;

    // And a second member that is not there: an address nothing is at, written
    // straight in, because what is being asked is what the announcement does when
    // it cannot be made rather than how the member came to be recorded.
    switched_off(&a, C).await;

    let outgoing = a.device.fingerprint().to_owned();

    assert_eq!(a.re_issued().await, Changeover::YetToTell(2));

    let incoming = a.device.incoming_fingerprint().unwrap().to_owned();

    a.devices().waiting(PATIENCE).announce_renewal().await;

    assert_eq!(
        b.holds(A).await.fingerprint,
        incoming,
        "the member that was there holds the new one",
    );

    assert!(
        !a.holds(C).await.reachable,
        "and the one that was not is dimmed, which is what a dial reaching none \
         of its addresses does",
    );

    assert_eq!(
        verkstead_store::announcements_owed_to(&a.pool, C)
            .await
            .unwrap(),
        vec![(A.to_owned(), Telling::Renewed)],
        "owed the telling, on the record a join and an unlink are owed on",
    );

    // And the changeover stays in flight rather than failing anything: the old
    // certificate keeps going out, and both fingerprints are there to be named on
    // the startup line.
    assert!(a.device.incoming_path().exists());

    let again = a.started_again().await;

    assert_eq!(again.changeover(), Changeover::YetToTell(1));
    assert_eq!(again.fingerprint(), outgoing);
    assert_eq!(
        again.incoming_fingerprint(),
        Some(incoming.as_str()),
        "a restart in the middle holds the same pair, and the line says both",
    );

    // And the member that has acknowledged is still answered with the certificate
    // it was answered with before, which is the whole reason it kept it.
    b.peers()
        .identity(&b.holds(A).await)
        .await
        .expect("a member that acknowledged goes on reaching the device it acknowledged");
}

/// And that member is told when it next answers, at which point the changeover
/// finishes.
///
/// **The trigger is a dial that got through, rather than a timer.** Something dials
/// a member whenever the cluster does anything at all — here it is a fourth device
/// joining, which A announces to every member it holds — and the dial that finds
/// that member answering is what pays everything it was owed, the renewal included.
#[tokio::test]
async fn a_member_that_was_not_there_is_told_when_it_answers_and_the_changeover_finishes() {
    let mut a = Verkstead::nearly_expired(A).await;
    let b = Verkstead::answering(B).await;
    let c = Verkstead::answering(C).await;

    linked(&b, &a).await;
    linked(&c, &a).await;

    // C, switched off between the join and the re-issue: the row is the one the
    // join wrote, with its addresses pointing nowhere.
    switched_off(&a, C).await;

    assert_eq!(a.re_issued().await, Changeover::YetToTell(2));

    let incoming = a.device.incoming_fingerprint().unwrap().to_owned();

    a.devices().waiting(PATIENCE).announce_renewal().await;

    assert!(
        a.device.incoming_path().exists(),
        "C never answered, so the changeover is in flight",
    );

    // And now C is back, at the address it always advertised — and something the
    // cluster does reaches it: a fourth device joins through A.
    back_again(&a, &c).await;

    let d = Verkstead::answering(D).await;

    linked(&d, &a).await;

    assert_eq!(
        c.holds(A).await.fingerprint,
        incoming,
        "the dial that told C about the newcomer paid what C was owed about A, \
         which is the renewal it slept through",
    );
    assert!(
        verkstead_store::announcements_owed_to(&a.pool, C)
            .await
            .unwrap()
            .is_empty(),
        "and the debt is paid rather than paid and written down again",
    );

    assert!(
        !a.device.incoming_path().exists(),
        "and the last acknowledgement finished the changeover, which is what it \
         was waiting on",
    );

    assert_eq!(
        a.started_again().await.fingerprint(),
        incoming,
        "so the start after this presents the certificate every member holds",
    );

    // Including the device that joined in the middle of it: it met the certificate
    // going out, which is the only one it could have met, and was told about the
    // other before the changeover was allowed to finish.
    assert_eq!(
        d.holds(A).await.fingerprint,
        incoming,
        "a newcomer is not a member the changeover quietly finished behind",
    );
    assert_eq!(a.cluster().await, sorted(&[B, C, D]));
}

/// And a member that never answers at all is one the human unlinks, which is the
/// press that lets the changeover finish.
///
/// **The one thing a changeover has no other answer to.** Nothing waits for ever on
/// a machine that is gone: the old certificate keeps going out, both fingerprints
/// keep being named, and the list goes on drawing that device dimmed with an Unlink
/// that works on it.
#[tokio::test]
async fn a_member_that_never_answers_is_one_the_human_unlinks() {
    let mut a = Verkstead::nearly_expired(A).await;
    let b = Verkstead::answering(B).await;

    linked(&b, &a).await;
    switched_off(&a, C).await;

    a.re_issued().await;

    let incoming = a.device.incoming_fingerprint().unwrap().to_owned();

    a.devices().waiting(PATIENCE).announce_renewal().await;

    assert!(a.device.incoming_path().exists());

    let dimmed = listing(&a.workbench())
        .await
        .members
        .into_iter()
        .find(|member| member.identity.device == C)
        .expect("the member that was never there is still on the list");

    assert!(!dimmed.reachable, "drawn dimmed, reading unreachable");

    unlink(&a.workbench_in_a_hurry(), C).await;

    assert_eq!(a.cluster().await, sorted(&[B]));
    assert!(
        !a.device.incoming_path().exists(),
        "and the device the changeover was waiting on is gone, so the changeover \
         is over",
    );
    assert_eq!(a.started_again().await.fingerprint(), incoming);
}

// ---------------------------------------------------------------------------
// The plumbing
// ---------------------------------------------------------------------------

/// `joining` links itself to `answering`, all the way through: Add pressed on the
/// first, Allow pressed on the second, and every call between them real.
async fn linked(joining: &Verkstead, answering: &Verkstead) {
    let (status, said) = add(&joining.workbench(), &answering.at()).await;
    assert_eq!(status, StatusCode::OK, "POST {ADD}: {said}");

    let held = being_asked(&answering.workbench()).await;
    assert_eq!(held.len(), 1, "one question, just asked");

    allow(&answering.workbench(), &held[0].request).await;
}

/// A call from each of two devices to the other, which is what *every call in
/// every direction* comes down to between a pair.
///
/// The identity read, because it is the one call that is made against a member row
/// and checks what came back against it: the handshake has to complete against a
/// certificate this end accepts, and the answer has to name one of them.
async fn every_direction(one: &Verkstead, other: &Verkstead) {
    one.peers()
        .identity(&one.holds(other.device.id()).await)
        .await
        .unwrap_or_else(|why| {
            panic!(
                "device {} should be able to call device {}: {why:#}",
                one.device.id(),
                other.device.id(),
            )
        });

    other
        .peers()
        .identity(&other.holds(one.device.id()).await)
        .await
        .unwrap_or_else(|why| {
            panic!(
                "device {} should be able to call device {}: {why:#}",
                other.device.id(),
                one.device.id(),
            )
        });
}

/// Write `device` into `holding`'s membership as a machine that is not there: the
/// row a join would have left, with its addresses pointing at nothing.
///
/// For the questions about a member that was switched off. What is being asked is
/// what a renewal does when it cannot be announced, rather than how that member
/// came to be recorded — and a certificate nothing presents is exactly what a
/// machine that is gone leaves behind.
async fn switched_off(holding: &Verkstead, device: &str) {
    verkstead_store::record_member(
        &holding.pool,
        &Linking {
            device: device.to_owned(),
            name: "laptop".to_owned(),
            os: "Linux (WSL)".to_owned(),
            addresses: vec![NOWHERE.to_owned()],
            fingerprint: "AA:BB:CC:DD:EE:FF".to_owned(),
        },
    )
    .await
    .unwrap();
}

/// And the same machine back on the address it really answers at, under the
/// certificate it really presents.
///
/// Which is what a laptop being opened again looks like from over here: the row is
/// the row, and the next dial that works down its addresses gets through.
async fn back_again(holding: &Verkstead, device: &Verkstead) {
    verkstead_store::record_member(
        &holding.pool,
        &Linking {
            device: device.device.id().to_owned(),
            name: "laptop".to_owned(),
            os: "Linux (WSL)".to_owned(),
            addresses: vec![device.at()],
            fingerprint: device.device.fingerprint().to_owned(),
        },
    )
    .await
    .unwrap();
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

/// And press Unlink on a row, which the browser has already had confirmed.
async fn unlink(app: &Router, device: &str) {
    let at = format!("{DEVICES}/members/{device}/unlink");

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&at)
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
        "POST {at}: {}",
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

/// `devices`' ids, sorted — which is the shape a list is compared in.
fn sorted(devices: &[&str]) -> Vec<String> {
    let mut sorted: Vec<String> = devices.iter().map(|&device| device.to_owned()).collect();
    sorted.sort();
    sorted
}

/// The other of B and C, which is what each of them holds beside A in the cluster
/// of three the first test makes.
fn other(device: &str) -> &'static str {
    match device {
        B => C,
        _ => B,
    }
}

/// A member row pointing at `listening`, as the device named `device` would hold
/// one of it.
///
/// For the tests that dial rather than press: what a dial is made of is the
/// addresses to work down and the certificate to insist on, and both are the
/// listening device's own — so a row need not have been recorded anywhere to say
/// where a call goes.
fn member_at(listening: &Verkstead, device: &str) -> Member {
    Member {
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
