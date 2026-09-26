//! Dialling a peer: what this device does when it is the caller (ADR-0020).
//!
//! The other side of `tests/peer.rs`. That suite stands a listener up and dials
//! it with a client of its own, which is how a handshake is proved at all; this
//! one stands a *device* up and has the product dial it, because what is being
//! asked here is what the product does — which certificate it presents, which
//! certificate it will accept, how far down a list of addresses it gets, and
//! what it writes down about the member afterwards.
//!
//! Every dial here is a real one over a real socket. A member is reached on the
//! loopback because that is a network two devices can both be on inside one
//! test, and the addresses recorded against it carry the port the operating
//! system picked: the peer port is the one a device answers on unless it has
//! been told another, and a suite that took the real one would fight whatever is
//! already there.
//!
//! **A machine that is not answering is a socket that is taken and served by
//! nothing** — see [`FarEnd::quiet`], which is a peer listener bound and not
//! yet serving. That is a laptop with its lid shut as far as a caller can tell:
//! the connection is accepted by the machine and the handshake never finishes,
//! which is what the per-address patience is spent on.

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::extract::ConnectInfo;
use axum::routing::get;
use axum::{Json, Router};
use sqlx::SqlitePool;
use verkstead_render::DeviceIdentity;
use verkstead_server::device::{Device, RENEW_WITHIN};
use verkstead_server::open_database;
use verkstead_server::peer::dialling::Peers;
use verkstead_server::peer::{self, Caller, Members};
use verkstead_store::{Linking, Member, record_member};

/// The id this device is stated as, so that what a test asserts against is a
/// string it chose rather than sixteen random bytes — see [`Device::stated`],
/// which is here for that reason.
const THIS_DEVICE: &str = "aa00bb11cc22dd33ee44ff5566778899";

/// And the one at the far end of every dial: a device written down as a member
/// of this one's cluster.
const THE_MEMBER: &str = "0011223344556677889900aabbccddee";

/// And a third that is nobody's member, for the two questions about a far end
/// that is not the device the row is about. Another Verkstead's identity rather
/// than a certificate minted for the occasion: what a device presents is exactly
/// what a device presents.
const SOMEBODY_ELSE: &str = "ffeeddccbbaa00998877665544332211";

/// How long a test gives one address, rather than the two seconds a running
/// server gives one.
///
/// What the tests about a member that answers nothing are asking is that the
/// patience is spent per address — so what they must not spend is the real
/// thing, four times over, on the clock.
const PATIENCE: Duration = Duration::from_millis(300);

/// A device at the far end of a dial: the certificate it presents, what it says
/// of itself, and the socket it is reached on.
///
/// Its answer is stated rather than read off this machine, for the reason
/// `tests/peer.rs` states one: what the box running the suite is called and what
/// interfaces it has are the two things a test about a *far end* cannot stand
/// on. Which is also what makes the doctored answers below possible — a device
/// that names a certificate other than the one it presented is a thing to state,
/// no real one ever saying it.
struct FarEnd {
    /// Where it is dialled, which is the port the operating system picked.
    address: SocketAddr,

    /// What it presents, and what it is called by.
    device: Device,

    /// What it answers the identity endpoint with.
    identity: DeviceIdentity,

    /// The listener, until it starts answering: a socket taken and served by
    /// nothing, which is [`FarEnd::quiet`] — see the module note.
    listener: Option<peer::Listener>,

    /// And the fingerprint the caller presented, as this end read it off the
    /// handshake. Which is the whole of what says a dial presented this
    /// device's own certificate rather than nothing or something else.
    presented: Arc<Mutex<Option<String>>>,

    /// Held for the length of the test, the certificate living in it.
    _dir: tempfile::TempDir,
}

impl FarEnd {
    /// A machine whose peer port is taken and answering nothing.
    ///
    /// The socket is bound so that the address is real and stays this device's
    /// for the length of the test — an address nothing at all is on would be a
    /// port the next test could be given.
    fn quiet(id: &str) -> FarEnd {
        let dir = tempfile::tempdir().unwrap();
        let device = Device::stated(dir.path(), id).unwrap();

        let listener = peer::Listener::bound("127.0.0.1:0".parse().unwrap(), &device)
            .expect("the loopback on a port the machine picked is free");

        let address = listener.address();

        FarEnd {
            identity: DeviceIdentity {
                device: id.to_owned(),
                fingerprint: device.fingerprint().to_owned(),
                name: "laptop".to_owned(),
                os: "macOS".to_owned(),
                addresses: vec![format!("127.0.0.1:{}", address.port())],
            },
            address,
            device,
            listener: Some(listener),
            presented: Arc::default(),
            _dir: dir,
        }
    }

    /// And the same machine answering, which is a device that is there.
    fn answering(id: &str) -> FarEnd {
        let mut far = FarEnd::quiet(id);
        far.answer();
        far
    }

    /// One that answers with `said` of itself rather than with the truth: a
    /// device naming a certificate it did not present, or answering under
    /// somebody else's id.
    fn saying(id: &str, said: impl FnOnce(&mut DeviceIdentity)) -> FarEnd {
        let mut far = FarEnd::quiet(id);

        said(&mut far.identity);
        far.answer();

        far
    }

    /// Start answering, which is the lid coming up on the machine.
    fn answer(&mut self) {
        let listener = self
            .listener
            .take()
            .expect("a far end starts answering once");

        let identity = self.identity.clone();
        let presented = Arc::clone(&self.presented);

        tokio::spawn(listener.serving(Router::new().route(
            peer::IDENTITY,
            get(move |ConnectInfo(caller): ConnectInfo<Caller>| {
                let identity = identity.clone();
                let presented = Arc::clone(&presented);

                async move {
                    *presented.lock().unwrap() = caller.fingerprint();

                    Json(identity)
                }
            }),
        )));
    }

    /// Where a member row records it.
    fn at(&self) -> String {
        format!("127.0.0.1:{}", self.address.port())
    }

    /// The certificate it really presents, which is what a member row is pinned
    /// on.
    fn fingerprint(&self) -> &str {
        self.device.fingerprint()
    }

    /// And what the caller showed it, where anything was dialled at all.
    fn showed(&self) -> Option<String> {
        self.presented.lock().unwrap().clone()
    }
}

/// An address on the loopback that nothing is on at all: a port taken and given
/// straight back, which is a machine that refuses rather than one that says
/// nothing.
///
/// What a member's earlier addresses are in the test about reaching a later
/// one. A refusal rather than a silence there deliberately — the question is
/// whether the walk gets past them, and a silence would be a question about the
/// patience instead.
fn nothing_there() -> String {
    let taken = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = taken.local_addr().unwrap();

    drop(taken);

    format!("127.0.0.1:{}", address.port())
}

/// This device: what it presents, and the membership a dial writes into.
struct ThisDevice {
    device: Device,
    members: Members,
    pool: SqlitePool,
    _dir: tempfile::TempDir,
}

impl ThisDevice {
    /// A Verkstead with a store of its own and an identity in it.
    async fn here() -> ThisDevice {
        let (dir, pool) = a_store().await;
        let device = Device::stated(dir.path(), THIS_DEVICE).unwrap();

        ThisDevice::over(dir, pool, device)
    }

    /// And one that came up in the middle of a changeover: a certificate near
    /// enough its expiry for the start to have made another, and a member that
    /// has yet to acknowledge the one it made.
    ///
    /// The membership the changeover was decided against is stated, the way
    /// `tests/peer.rs` states one — what puts a start inside a changeover is
    /// that somebody is owed an announcement, and that is a number. The
    /// membership the *dial* writes into is the real one below, because a row is
    /// what a dial works from.
    async fn mid_changeover() -> ThisDevice {
        let (dir, pool) = a_store().await;

        Device::stated_good_for(
            dir.path(),
            THIS_DEVICE,
            RENEW_WITHIN - time::Duration::days(1),
        )
        .unwrap();

        let device = Device::issued(dir.path(), &Members::stated(1))
            .await
            .unwrap();

        assert!(
            device.incoming_fingerprint().is_some(),
            "a start inside the renewal window makes a fresh certificate",
        );

        ThisDevice::over(dir, pool, device)
    }

    fn over(dir: tempfile::TempDir, pool: SqlitePool, device: Device) -> ThisDevice {
        ThisDevice {
            device,
            members: Members::recorded(pool.clone()),
            pool,
            _dir: dir,
        }
    }

    /// How it dials, with every address given [`PATIENCE`].
    fn peers(&self) -> Peers {
        Peers::of(self.device.clone(), self.members.clone()).waiting(PATIENCE)
    }

    /// Write a device down as a member of this one's cluster: the certificate it
    /// is pinned on, and the addresses to work down.
    ///
    /// A fixture, the join being a later task's — what this task is about is
    /// what a dial does with a row that is already there.
    async fn linked(&self, device: &str, fingerprint: &str, addresses: &[String]) {
        record_member(
            &self.pool,
            &Linking {
                device: device.to_owned(),
                name: "the name it was recorded under".to_owned(),
                os: "Linux".to_owned(),
                addresses: addresses.to_vec(),
                fingerprint: fingerprint.to_owned(),
            },
        )
        .await
        .unwrap();
    }

    /// And the row as it stands now, which is what a dial's findings are read
    /// out of.
    async fn member(&self, device: &str) -> Member {
        verkstead_store::members(&self.pool)
            .await
            .unwrap()
            .into_iter()
            .find(|member| member.device == device)
            .unwrap_or_else(|| panic!("device {device} should still be a member"))
    }
}

/// A database in a directory of its own, opened the way a start opens one.
async fn a_store() -> (tempfile::TempDir, SqlitePool) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    (dir, pool)
}

/// A member whose first addresses are gone is still reached on a later one — and
/// what this device dialled with is its own certificate.
///
/// Both in one test because they are one dial: what the far end read off the
/// handshake is only readable because the dial got that far.
#[tokio::test]
async fn a_member_is_reached_on_an_address_past_the_ones_that_are_gone() {
    let this = ThisDevice::here().await;
    let far = FarEnd::answering(THE_MEMBER);

    this.linked(
        THE_MEMBER,
        far.fingerprint(),
        &[nothing_there(), nothing_there(), far.at()],
    )
    .await;

    let member = this.member(THE_MEMBER).await;
    let identity = this
        .peers()
        .identity(&member)
        .await
        .expect("the third address is the machine, and a dial works down the list");

    assert_eq!(identity.device, THE_MEMBER);
    assert_eq!(
        far.showed().as_deref(),
        Some(this.device.fingerprint()),
        "a dial presents this device's own certificate — what a member holds is what \
         it has to be answered with",
    );
}

/// A member that answers at none of its addresses is left on the list, dimmed,
/// with nothing about it forgotten.
#[tokio::test]
async fn a_member_that_answers_nowhere_is_dimmed_and_nothing_else() {
    let this = ThisDevice::here().await;
    let quiet = FarEnd::quiet(THE_MEMBER);

    let addresses = vec![nothing_there(), quiet.at()];

    this.linked(THE_MEMBER, quiet.fingerprint(), &addresses)
        .await;

    let before = this.member(THE_MEMBER).await;

    this.peers()
        .identity(&before)
        .await
        .expect_err("a machine that says nothing is a machine that is not there");

    let after = this.member(THE_MEMBER).await;

    assert!(
        !after.reachable,
        "a dial that reached none of a member's addresses dims the row on that first \
         failure, which is what says what a press on it would do",
    );

    assert_eq!(
        after.addresses, addresses,
        "the addresses are what the dial will work down next time, so they are the one \
         thing a failure must not touch",
    );
    assert_eq!(after.fingerprint, before.fingerprint);
    assert_eq!(after.name, before.name);
    assert_eq!(after.os, before.os);
    assert_eq!(
        after.last_seen, before.last_seen,
        "when it was last heard from is when it really was, and a dial that heard \
         nothing heard nothing",
    );
}

/// And it un-dims on the next dial that gets through, which is the machine's lid
/// coming up.
#[tokio::test]
async fn a_dimmed_member_comes_back_on_the_next_dial_that_answers() {
    let this = ThisDevice::here().await;
    let mut far = FarEnd::quiet(THE_MEMBER);

    this.linked(THE_MEMBER, far.fingerprint(), &[far.at()])
        .await;

    let member = this.member(THE_MEMBER).await;
    this.peers()
        .identity(&member)
        .await
        .expect_err("nothing answers yet");

    assert!(!this.member(THE_MEMBER).await.reachable);

    far.answer();

    let member = this.member(THE_MEMBER).await;
    this.peers()
        .identity(&member)
        .await
        .expect("the same address, with the machine on it now");

    assert!(
        this.member(THE_MEMBER).await.reachable,
        "a dial that got through is the whole of what says a member is answering again",
    );
}

/// A far end whose certificate is not the one recorded for that member is
/// refused — and the row is left exactly as it was, because nothing has been
/// learned about the device it is about.
#[tokio::test]
async fn a_far_end_presenting_another_certificate_is_refused() {
    let this = ThisDevice::here().await;
    let far = FarEnd::answering(THE_MEMBER);
    let somebody_else = FarEnd::quiet(SOMEBODY_ELSE);

    // The row is pinned on a certificate the machine at that address does not
    // hold, which is what a device standing in for a member looks like.
    this.linked(THE_MEMBER, somebody_else.fingerprint(), &[far.at()])
        .await;

    let member = this.member(THE_MEMBER).await;
    let why = this
        .peers()
        .identity(&member)
        .await
        .expect_err("the pinned fingerprint is the whole of what proves a far end");

    assert!(
        format!("{why:#}").contains(far.fingerprint()),
        "the refusal says which certificate turned up, that being the thing to go and \
         look at, got: {why:#}",
    );

    let after = this.member(THE_MEMBER).await;

    assert_eq!(
        after.fingerprint,
        somebody_else.fingerprint(),
        "a dial does not re-pin a member on whatever answered for it",
    );
    assert!(
        after.reachable,
        "somebody else answering is not the member being unreachable — one is a row to \
         leave alone and the other is a row to dim",
    );
    assert_eq!(after.name, member.name, "and nothing was recorded off it");
}

/// And so is one that presents the certificate recorded for it and then names
/// another: the fingerprint travels with an identity so the two can be held up
/// beside each other.
#[tokio::test]
async fn a_far_end_that_names_a_certificate_it_did_not_present_is_refused() {
    let this = ThisDevice::here().await;
    let somebody_else = FarEnd::quiet(SOMEBODY_ELSE);
    let named = somebody_else.fingerprint().to_owned();

    let far = FarEnd::saying(THE_MEMBER, |identity| identity.fingerprint = named);

    this.linked(THE_MEMBER, far.fingerprint(), &[far.at()])
        .await;

    let member = this.member(THE_MEMBER).await;
    let why = this
        .peers()
        .identity(&member)
        .await
        .expect_err("a device that names one certificate and presents another is not itself");

    assert!(
        format!("{why:#}").contains(somebody_else.fingerprint()),
        "the refusal says what was named as well as what was held, got: {why:#}",
    );

    let after = this.member(THE_MEMBER).await;

    assert_eq!(
        after.name, member.name,
        "nothing a device that is not itself says is written down",
    );
    assert!(after.reachable, "and the row is not dimmed over it either");
}

/// A dial that gets through records when it did, and the name, the OS and the
/// addresses the far end now says it has.
#[tokio::test]
async fn a_dial_that_answered_refreshes_the_member_it_reached() {
    let this = ThisDevice::here().await;

    let far = FarEnd::saying(THE_MEMBER, |identity| {
        identity.name = "the name the machine answers to now".to_owned();
        identity.os = "Linux (WSL)".to_owned();
        identity.addresses = vec![
            "laptop.tailnet-name.ts.net".to_owned(),
            "192.168.1.31".to_owned(),
        ];
    });

    this.linked(THE_MEMBER, far.fingerprint(), &[far.at()])
        .await;

    let before = this.member(THE_MEMBER).await;
    this.peers()
        .identity(&before)
        .await
        .expect("the machine is there");

    let after = this.member(THE_MEMBER).await;

    assert_eq!(after.name, "the name the machine answers to now");
    assert_eq!(
        after.os, "Linux (WSL)",
        "the OS word is the far end's, this device having no way to read another \
         machine's kernel",
    );
    assert_eq!(
        after.addresses,
        vec!["laptop.tailnet-name.ts.net", "192.168.1.31"],
        "in the order the far end advertised them, which is the order the next dial \
         works down",
    );
    assert!(
        after.last_seen > before.last_seen,
        "a dial that got through writes the moment it did, and {} is not later than {}",
        after.last_seen,
        before.last_seen,
    );
}

/// A list of addresses that answer nothing costs the patience apiece, and no
/// more than that.
///
/// Both halves are the question. More than one patience is what says it is spent
/// per address rather than over the dial as a whole — which is what keeps a
/// member that moved reachable at all; and their number times it is what says a
/// dial is bounded, rather than the operating system's own minutes.
#[tokio::test]
async fn a_list_of_dead_addresses_costs_the_patience_apiece() {
    let this = ThisDevice::here().await;

    // Three machines that take a connection and say nothing, which is what a
    // patience is spent on — an address nothing is on at all refuses at once
    // and would say nothing about a deadline.
    let quiet: Vec<FarEnd> = (0..3).map(|_| FarEnd::quiet(THE_MEMBER)).collect();
    let addresses: Vec<String> = quiet.iter().map(FarEnd::at).collect();

    this.linked(THE_MEMBER, quiet[0].fingerprint(), &addresses)
        .await;

    let member = this.member(THE_MEMBER).await;
    let began = Instant::now();

    this.peers()
        .identity(&member)
        .await
        .expect_err("nothing on that list answers");

    let spent = began.elapsed();

    assert!(
        spent >= PATIENCE * 2,
        "three addresses that say nothing are three patiences, not one: a deadline over \
         the dial as a whole would never reach the third, and it took {spent:?}",
    );
    assert!(
        spent < PATIENCE * addresses.len() as u32 + Duration::from_secs(2),
        "and the walk is bounded by their number times it, not by the machine's own \
         patience with a connection: it took {spent:?}",
    );
}

/// Over a changeover the dial presents the outgoing certificate, which is the
/// one the far end holds.
///
/// Which is the whole of why a changeover exists: the new certificate is the one
/// this device will present once every member has acknowledged it, and a dial
/// that went out under it beforehand would be refused at the gate of every
/// member that has not.
#[tokio::test]
async fn over_a_changeover_a_dial_presents_the_certificate_the_member_holds() {
    let this = ThisDevice::mid_changeover().await;
    let far = FarEnd::answering(THE_MEMBER);

    this.linked(THE_MEMBER, far.fingerprint(), &[far.at()])
        .await;

    let member = this.member(THE_MEMBER).await;
    this.peers()
        .identity(&member)
        .await
        .expect("the machine is there");

    assert_eq!(
        far.showed().as_deref(),
        Some(this.device.fingerprint()),
        "the outgoing certificate is what goes out, being what the member holds",
    );
    assert_ne!(
        far.showed().as_deref(),
        this.device.incoming_fingerprint(),
        "and the one waiting to replace it stays where it is until it has been \
         announced",
    );
}
