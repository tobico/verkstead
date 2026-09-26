//! The tailnet half of the **Discovered** list: the peers `tailscale status`
//! names, asked on the peer port what they are (ADR-0020, *Discovery*).
//!
//! The other half of `tests/discovering.rs`. That suite browses a real multicast
//! and asks which of what it heard is drawn; this one stands a device on the
//! loopback, tells this one that the device is a node of its tailnet, and asks
//! what the probe made of it. Every probe here is a real dial over a real socket
//! against a real peer listener — what is being asked is what the product does
//! with what a stranger answers, and a stubbed dial would be asking nothing.
//!
//! **A tailnet is a thing this suite states rather than one it stands up.** What
//! `tailscale status --json` prints is another project's JSON, and the runner is
//! on whatever tailnet it happens to be on — most likely none — so `tailscale` is
//! a shell script here, the way it is in `tests/remote.rs`: the server runs the
//! program it is given and reads what it printed, and what it is given prints the
//! peers one of these cases has.
//!
//! **And the peer port is stated too.** A peer list names an address and never a
//! port, so a probe assumes one; the real one is a port on the machine running
//! this and may be another Verkstead's, so each test stands its device on a port
//! the operating system picked and tells the probe to assume that — see
//! [`Probe::of_this_tailnet_on`], which is there for this.
//!
//! Unix only, for `tests/remote.rs`'s reason and no other: what is stated here is
//! a shape of stdout rather than anything about a platform, and a Windows run
//! would be a second machine reading the same JSON. The shapes themselves are
//! pinned where they are read, in `remote.rs`'s own tests, and the merge of the
//! two halves in `discovery.rs`'s.
#![cfg(unix)]

use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::routing::get;
use axum::{Json, Router};
use http_body_util::BodyExt;
use sqlx::SqlitePool;
use tower::ServiceExt;
use verkstead_render::{DeviceIdentity, DiscoveredDevice, FoundOn};
use verkstead_server::device::reading::Reading;
use verkstead_server::device::{Device, Devices};
use verkstead_server::discovery::{AT_ONCE, Browse, Found, MOST_PEERS, Probe};
use verkstead_server::nudge::Nudges;
use verkstead_server::peer::joining::Joins;
use verkstead_server::peer::{self, Members};
use verkstead_server::platform::Platform;
use verkstead_server::remote::Tailscale;
use verkstead_server::{open_database, router_answering_devices_telling};

/// Where the pane reads the list a probe fills.
const DISCOVERED: &str = "/api/ui/devices/discovered";

/// This device, named by the id its own list leaves out.
const THIS_DEVICE: &str = "aa00bb11cc22dd33ee44ff5566778899";

/// The device at the far end of a probe, and a second for the row a test wants
/// drawn while it asks about one that is not.
const OVER_THERE: &str = "0011223344556677889900aabbccddee";
const A_THIRD: &str = "99887766554433221100aabbccddeeff";

/// The port the workbench is taken to be on, which nothing here asks about: it is
/// what a serve would have to be proxying to, and no test here presses one.
const PORT: u16 = 8422;

/// How long a test gives one peer, rather than the three seconds a running server
/// gives one.
///
/// What the tests about a peer that answers nothing are asking is that the list
/// comes back without whatever the tailnet half would have added — so what they
/// must not spend is the real deadline, once per peer, on the clock.
const PATIENCE: Duration = Duration::from_millis(300);

/// A machine with no Tailscale on it: `verkstead-no-such-tailscale` is a program
/// that is not there, which is what having none *is*.
fn no_tailscale() -> Tailscale {
    Tailscale::running(vec!["verkstead-no-such-tailscale".to_owned()], PORT)
}

/// A `tailscale` printing a status whose tailnet holds a node at each of
/// `addresses`, every one of them up.
///
/// The node key each is under is a string this makes up, that being a key nothing
/// reads: what a probe takes out of this list is the addresses and whether the
/// node is online.
fn tailscale_naming(addresses: &[String]) -> Tailscale {
    let peers: Vec<String> = addresses
        .iter()
        .enumerate()
        .map(|(which, address)| {
            format!(
                "\"nodekey:{which}\":{{\"DNSName\":\"peer-{which}.tailnet-name.ts.net.\",\
                 \"TailscaleIPs\":[\"{address}\"],\"Online\":true}}"
            )
        })
        .collect();

    let status = format!(
        "{{\"BackendState\":\"Running\",\
         \"Self\":{{\"DNSName\":\"workbench.tailnet-name.ts.net.\"}},\
         \"Peer\":{{{}}}}}",
        peers.join(","),
    );

    // Single quotes around the JSON, which carries none of its own: what is in it
    // is another project's field names and a handful of addresses.
    script(&format!("printf '%s' '{status}'"))
}

/// A `tailscale` that is `line` of shell.
///
/// `sh -c` gives `$0` the script's own name, so what Verkstead passes lands in
/// `$1` onwards — see `tests/remote.rs`, which stands the same thing up.
fn script(line: &str) -> Tailscale {
    Tailscale::running(
        vec![
            "/bin/sh".to_owned(),
            "-c".to_owned(),
            line.to_owned(),
            "tailscale".to_owned(),
        ],
        PORT,
    )
}

/// What a browse would have heard of a device on the LAN, for the tests that want
/// a row drawn while the tailnet half is the thing being asked about.
///
/// Stated rather than heard, for `tests/discovering.rs`'s reason: whatever is
/// advertising itself on the LAN of the box running this is a fact about that
/// LAN, and a test written off one would be a test of that network.
fn heard(device: &str) -> Found {
    Found {
        device: device.to_owned(),
        name: "laptop".to_owned(),
        os: "macOS".to_owned(),
        addresses: vec![LAN_ADDRESS.to_owned()],
    }
}

/// Where those stated devices were heard, with the port a listener over there
/// landed on.
const LAN_ADDRESS: &str = "192.168.1.31:9423";

/// And what a probe would have got out of one, for the tests that are about which
/// rows an answer leaves out rather than about a dial.
fn answered(device: &str) -> Found {
    Found {
        device: device.to_owned(),
        name: "laptop".to_owned(),
        os: "macOS".to_owned(),
        addresses: vec!["100.64.0.2:8423".to_owned()],
    }
}

/// This device's workbench: the router the pane reads, browsing `browse` and
/// probing `probe`, with the pool its membership is written into.
///
/// The machine is stated for `tests/devices.rs`'s reason — what the box running
/// this happens to be is a fact about that box — and its addresses are empty,
/// nothing here being about what *this* device says it is. Its reading's own
/// Tailscale is the one that is not there: what a probe runs is the handle the
/// probe holds, and a reading that ran the script too would be one fixture
/// answering two questions.
async fn workbench(browse: Browse, probe: Probe) -> (tempfile::TempDir, SqlitePool, Router) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    let device = Device::stated(dir.path(), THIS_DEVICE).unwrap();
    let reading = Reading::stated(no_tailscale(), Platform::Linux, None, Vec::new());

    let devices = Devices::of(
        device,
        reading,
        Members::recorded(pool.clone()),
        Joins::recorded(pool.clone()),
    )
    .browsing(browse)
    .probing(probe);

    let app = router_answering_devices_telling(pool.clone(), devices, Nudges::new());

    (dir, pool, app)
}

/// The Discovered list as the pane reads it, parsed as the type the page draws.
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
    serde_json::from_slice(&bytes).unwrap_or_else(|why| {
        panic!(
            "the Discovered list should be the type the pane draws: {why}\n{}",
            String::from_utf8_lossy(&bytes),
        )
    })
}

/// The rows of that list as a device and where it was found, which is what every
/// test here is asking about.
async fn rows(app: &Router) -> Vec<(String, Vec<FoundOn>)> {
    discovered(app)
        .await
        .into_iter()
        .map(|row| (row.device, row.found))
        .collect()
}

/// A device at the far end of a probe: a peer listener on the loopback, answering
/// the identity endpoint.
///
/// What it answers is stated rather than read off this machine, for
/// `tests/dialling.rs`'s reason: what the box running the suite is called is the
/// one thing a test about a *far end* cannot stand on. The fingerprint is the real
/// one all the same — it is the certificate that listener presents, and a device
/// naming any other is one a probe refuses.
struct FarEnd {
    /// Where it is reached, which is the port the operating system picked.
    address: SocketAddr,

    /// Held for the length of the test, the certificate living in it.
    _dir: tempfile::TempDir,
}

impl FarEnd {
    /// One that answers as a Verkstead does.
    fn answering(id: &str) -> FarEnd {
        let dir = tempfile::tempdir().unwrap();
        let device = Device::stated(dir.path(), id).unwrap();

        let identity = DeviceIdentity {
            device: id.to_owned(),
            fingerprint: device.fingerprint().to_owned(),
            name: "kitchen-mini".to_owned(),
            os: "macOS".to_owned(),
            addresses: vec!["100.64.0.2".to_owned()],
        };

        FarEnd::serving(
            dir,
            &device,
            Router::new().route(
                peer::IDENTITY,
                get(move || {
                    let identity = identity.clone();

                    async move { Json(identity) }
                }),
            ),
        )
    }

    /// And one that answers something which is not a device identity at all,
    /// which is what a machine with anything else on that port does.
    fn answering_something_else() -> FarEnd {
        let dir = tempfile::tempdir().unwrap();
        let device = Device::stated(dir.path(), A_THIRD).unwrap();

        FarEnd::serving(
            dir,
            &device,
            Router::new().route(
                peer::IDENTITY,
                get(|| async { Json(serde_json::json!({ "hello": "there" })) }),
            ),
        )
    }

    /// The listener bound and serving `routes`, with `device`'s certificate on it.
    fn serving(dir: tempfile::TempDir, device: &Device, routes: Router) -> FarEnd {
        let listener = peer::Listener::bound("127.0.0.1:0".parse().unwrap(), device)
            .expect("the loopback on a port the machine picked is free");

        let address = listener.address();

        tokio::spawn(listener.serving(routes));

        FarEnd { address, _dir: dir }
    }

    /// Where a row drawn from it says it was found.
    fn at(&self) -> String {
        format!("127.0.0.1:{}", self.address.port())
    }
}

/// The one address every status here names its peers at, the probe putting the
/// port it assumes on the end of it.
const LOOPBACK: &str = "127.0.0.1";

/// A port on the loopback that nothing is on at all: one taken and given straight
/// back, which is a machine that refuses rather than one that says nothing.
fn nothing_there() -> u16 {
    let taken = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = taken.local_addr().unwrap().port();

    drop(taken);

    port
}

/// A peer of this machine's tailnet that is running a Verkstead is a row under
/// Discovered, reading *Tailscale*: the name and the mark it answered with, and
/// the address it answered at with the peer port on it.
#[tokio::test]
async fn a_tailnet_peer_running_a_verkstead_is_drawn() {
    let far = FarEnd::answering(OVER_THERE);

    let (_dir, _pool, app) = workbench(
        Browse::heard_nothing(),
        Probe::of_this_tailnet_on(far.address.port(), tailscale_naming(&[LOOPBACK.to_owned()])),
    )
    .await;

    let drawn = discovered(&app).await;

    assert_eq!(
        drawn.len(),
        1,
        "the one peer of that tailnet that answered as a device: {drawn:?}",
    );

    let row = &drawn[0];

    assert_eq!(row.device, OVER_THERE, "keyed by the id it answered with");
    assert_eq!(
        row.name, "kitchen-mini",
        "the name is what it answered rather than what the peer list called it: a \
         peer list is the coordination server's word for a machine, and a row \
         draws the machine's own",
    );
    assert_eq!(row.os, "macOS", "and the OS word the mark comes from");
    assert_eq!(
        row.found,
        vec![FoundOn::Tailscale],
        "found on the tailnet, there being nothing on the LAN half to find it",
    );
    assert_eq!(
        row.addresses,
        vec![far.at()],
        "and where it answered, with the port a probe assumes on it — which is \
         what makes the row something an Add can dial as it is written",
    );
}

/// A peer that is not a Verkstead is no row: it answers nothing, or it answers
/// something that is not a device identity, and both come to the same absent row.
///
/// **The LAN row beside it is what says the list was not simply empty.** A probe
/// that found nothing reads the same as a probe that never ran unless something
/// else is drawn, so each half of this states a device heard on the LAN and asks
/// that the answer is that row and no other.
#[tokio::test]
async fn a_peer_that_is_not_a_verkstead_is_not_drawn() {
    let nonsense = FarEnd::answering_something_else();

    for (what, port) in [
        (
            "a peer answering something that is not a device identity",
            nonsense.address.port(),
        ),
        (
            "a peer with nothing on the peer port at all",
            nothing_there(),
        ),
    ] {
        let (_dir, _pool, app) = workbench(
            Browse::stated(vec![heard(A_THIRD)]),
            Probe::of_this_tailnet_on(port, tailscale_naming(&[LOOPBACK.to_owned()]))
                .waiting(PATIENCE),
        )
        .await;

        assert_eq!(
            rows(&app).await,
            vec![(A_THIRD.to_owned(), vec![FoundOn::Lan])],
            "{what} draws nothing, and the device heard on the LAN is untouched",
        );
    }
}

/// A device found both ways is one row, and its source reads both: it is one
/// machine, and two rows offering to link it would be two presses about one
/// device.
#[tokio::test]
async fn a_device_found_both_ways_is_one_row() {
    let far = FarEnd::answering(OVER_THERE);

    let (_dir, _pool, app) = workbench(
        Browse::stated(vec![heard(OVER_THERE)]),
        Probe::of_this_tailnet_on(far.address.port(), tailscale_naming(&[LOOPBACK.to_owned()])),
    )
    .await;

    let drawn = discovered(&app).await;

    assert_eq!(drawn.len(), 1, "one machine is one row: {drawn:?}");

    assert_eq!(
        drawn[0].found,
        vec![FoundOn::Lan, FoundOn::Tailscale],
        "and the row says every way this device was heard of rather than whichever \
         way was heard of first",
    );

    assert_eq!(
        drawn[0].addresses,
        vec![LAN_ADDRESS.to_owned(), far.at()],
        "every place either half found it, the LAN's first: two machines on one \
         network reach each other without a tailnet in the middle",
    );
}

/// And the three exclusions apply to what the probe found exactly as they apply to
/// what the browse heard, the merge happening in front of all of them: a member
/// found over the tailnet is a member.
#[tokio::test]
async fn a_member_the_probe_found_is_not_drawn() {
    let (_dir, pool, app) = workbench(
        Browse::heard_nothing(),
        Probe::stated(vec![answered(OVER_THERE), answered(A_THIRD)]),
    )
    .await;

    assert_eq!(
        rows(&app).await,
        vec![
            (OVER_THERE.to_owned(), vec![FoundOn::Tailscale]),
            (A_THIRD.to_owned(), vec![FoundOn::Tailscale]),
        ],
        "both are drawn while neither is a member",
    );

    verkstead_store::record_member(
        &pool,
        &verkstead_store::Linking {
            device: OVER_THERE.to_owned(),
            name: "laptop".to_owned(),
            os: "macOS".to_owned(),
            addresses: vec!["100.64.0.2".to_owned()],
            fingerprint: format!("AA:BB:CC:DD:{OVER_THERE}"),
        },
    )
    .await
    .unwrap();

    assert_eq!(
        rows(&app).await,
        vec![(A_THIRD.to_owned(), vec![FoundOn::Tailscale])],
        "and the one that is now a member is left out, a row offering to link it \
         being a press with nothing behind it",
    );
}

/// Having no Tailscale costs the list nothing but the tailnet half, and so does a
/// daemon that is down and one that never answers.
///
/// **Which is the stance every other reading of this daemon takes.** There is
/// nothing here for a human to fix: a machine with no Tailscale is found on its
/// LAN, and the typed address was always the answer to a machine this cannot find
/// at all.
#[tokio::test]
async fn a_tailscale_that_says_nothing_costs_only_the_tailnet_half() {
    for (what, tailscale) in [
        ("a machine with no Tailscale on it", no_tailscale()),
        (
            "a machine whose daemon is not running",
            script(
                "echo \"failed to connect to local tailscaled; it doesn't appear to be \
                 running\" >&2; exit 1",
            ),
        ),
        (
            "and one whose daemon never answers at all",
            script("exec sleep 300"),
        ),
    ] {
        let (_dir, _pool, app) = workbench(
            Browse::stated(vec![heard(OVER_THERE)]),
            Probe::of_this_tailnet_on(nothing_there(), tailscale),
        )
        .await;

        assert_eq!(
            rows(&app).await,
            vec![(OVER_THERE.to_owned(), vec![FoundOn::Lan])],
            "{what} leaves the LAN half of the list standing",
        );
    }
}

/// And a status naming far more peers than the ceiling asks no more than the
/// ceiling, each with a deadline of its own.
///
/// **Because the number of nodes on a tailnet is not this machine's to choose.** A
/// pane opened on a machine in a company's tailnet must not be a burst of
/// handshakes at every node in the estate. What is counted is the connections that
/// arrived: every peer this status names is at one address, which is what makes
/// them countable at all, and the socket they arrive at holds where it is — so
/// what each probe spends there is its own deadline and nothing else.
#[tokio::test]
async fn no_more_peers_are_asked_than_the_ceiling() {
    let named = MOST_PEERS * 4;

    let taken = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = taken.local_addr().unwrap().port();
    let arrived = Arc::new(AtomicUsize::new(0));

    // Accepted and held rather than answered, which is a machine whose handshake
    // never finishes. Held, so that no probe ends early on a connection this end
    // dropped.
    std::thread::spawn({
        let arrived = Arc::clone(&arrived);

        move || {
            let mut held = Vec::new();

            while let Ok((stream, _)) = taken.accept() {
                arrived.fetch_add(1, Ordering::SeqCst);
                held.push(stream);
            }
        }
    });

    let (_dir, _pool, app) = workbench(
        Browse::heard_nothing(),
        Probe::of_this_tailnet_on(port, tailscale_naming(&vec![LOOPBACK.to_owned(); named]))
            .waiting(PATIENCE),
    )
    .await;

    assert!(
        discovered(&app).await.is_empty(),
        "nothing answered as a device, every one of them being a socket that holds \
         where it is",
    );

    // The read is back, so every probe it made has finished — but the accepting is
    // another thread's, so the count is given a moment to stop moving before it is
    // read.
    tokio::time::sleep(PATIENCE).await;

    let asked = arrived.load(Ordering::SeqCst);

    assert!(
        asked <= MOST_PEERS,
        "a status naming {named} peers asks no more than the ceiling of \
         {MOST_PEERS}, and this asked {asked}",
    );
    assert!(
        asked >= AT_ONCE,
        "and it really asked: at least one round of {AT_ONCE} went out, where \
         {asked} arrived",
    );
}
