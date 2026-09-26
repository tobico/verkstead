//! The **Discovered** list: the devices nobody has typed an address for, as the
//! Remote access pane reads them off this one (ADR-0020, *Discovery*).
//!
//! The other half of `tests/advertising.rs`. That suite stands where the
//! Verkstead on the next desk stands and reads what this device *says*; this one
//! stands where the browser stands and reads what this device *heard* — over the
//! workbench, which is what the pane can actually reach.
//!
//! **Two kinds of test, because there are two kinds of question.** Whether a
//! device on the LAN turns into a row is about a real browse of a real
//! multicast, so those tests advertise on a port of their own and wait: both
//! halves of a discovery have to be on one port to hear each other, and 5353 is
//! where every other implementation on the runner is — a test there would be
//! reading whatever real Verksteads are on the LAN it happens to be plugged
//! into. Whether a row is *left out* is about a membership, a join and this
//! device's own id, and none of that is on any wire: those state what was heard
//! and ask which of it is drawn.
//!
//! **And the first read of a cold browse is empty.** The browse starts when this
//! list is first asked for, so nothing has been heard when that answer is given
//! and the rows arrive over the seconds after it. Which is not a shortcoming to
//! work around here but the thing the Nudge exists for, and one of the tests
//! below is exactly that sequence: a page listening, a list read, and the row
//! arriving down the stream.

use std::time::{Duration, Instant};

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use sqlx::SqlitePool;
use tower::ServiceExt;
use verkstead_render::{DevicesView, DiscoveredDevice, FoundOn};
use verkstead_schema::Nudge;
use verkstead_server::device::reading::Reading;
use verkstead_server::device::{Device, Devices};
use verkstead_server::discovery::{Advertisement, Announcement, Browse, Found};
use verkstead_server::nudge::Nudges;
use verkstead_server::peer::Members;
use verkstead_server::peer::joining::Joins;
use verkstead_server::platform::Platform;
use verkstead_server::remote::Tailscale;
use verkstead_server::{open_database, router_answering_devices_telling};

/// Where the pane reads the two lists, and where a page listens for the word
/// that one of them moved.
const DISCOVERED: &str = "/api/ui/devices/discovered";
const DEVICES: &str = "/api/ui/devices";
const NUDGES: &str = "/api/ui/nudges";

/// This device, named by the id its own list leaves out.
const THIS_DEVICE: &str = "aa00bb11cc22dd33ee44ff5566778899";

/// The device on the other side of the multicast, and a second beside it: the
/// one a test is asking about, and the one that says the browse was listening at
/// all.
const OVER_THERE: &str = "0011223344556677889900aabbccddee";
const BESIDE_IT: &str = "ffeeddccbbaa00998877665544332211";

/// And a third, for the device a test wants drawn while it asks about one that
/// is not.
const A_THIRD: &str = "99887766554433221100aabbccddeeff";

/// The peer port the advertised devices say their listeners landed on.
///
/// Not 8423, deliberately: what a row draws is the port the far end really
/// bound, and a suite standing on the default would not tell the two apart.
const LANDED_ON: u16 = 9423;

/// The ports these tests speak mDNS over, one apiece: libtest runs them side by
/// side, and two tests on one multicast would each be reading the other's
/// devices.
const HEARING: u16 = 5471;
const GOODBYE: u16 = 5472;
const ARRIVING: u16 = 5473;
const TOO_MUCH: u16 = 5474;

/// How long a test waits for a row it is expecting.
///
/// Generous, because what is being waited on is a probe, an announcement and a
/// resolution over a multicast rather than a call: the daemon proves an instance
/// name is nobody else's before it says anything, and a runner under load takes
/// as long as it takes. Nothing here spends it when the row arrives.
const PATIENCE: Duration = Duration::from_secs(20);

/// And how often the list is read while waiting for one.
const AGAIN: Duration = Duration::from_millis(100);

/// How long a test reads before calling a row absent — see [`never_drawn`].
///
/// Shorter than [`PATIENCE`], and spent in full every time: there is no moment at
/// which a multicast has finished saying things, so proving a row is not coming is
/// reading for a while and not seeing it. Five seconds, the length
/// `tests/advertising.rs` listens for the same reason.
const QUIET: Duration = Duration::from_secs(5);

/// The port the workbench is taken to be on, which nothing here asks about: the
/// reading's Tailscale is built with it and never runs.
const PORT: u16 = 8422;

/// A machine with no Tailscale on it: `verkstead-no-such-tailscale` is a program
/// that is not there, which is what having none *is*.
fn no_tailscale() -> Tailscale {
    Tailscale::running(vec!["verkstead-no-such-tailscale".to_owned()], PORT)
}

/// What a device on the LAN would be saying about itself.
///
/// The name and the OS are stated rather than read off the runner, for the
/// reason `tests/advertising.rs` states them: what a Linux runner would answer
/// about itself is the one answer a test of this must not stand on.
fn announcement(device: &str) -> Announcement {
    Announcement {
        device: device.to_owned(),
        name: "kitchen-mini".to_owned(),
        os: "macOS".to_owned(),
        peer: LANDED_ON,
    }
}

/// And what a browse would have heard of one, for the tests that are about which
/// rows an answer leaves out rather than about a multicast.
fn heard(device: &str) -> Found {
    Found {
        device: device.to_owned(),
        name: "laptop".to_owned(),
        os: "macOS".to_owned(),
        addresses: vec![format!("192.168.1.31:{LANDED_ON}")],
    }
}

/// This device's workbench, answering out of `browse`: the router the pane
/// reads, the pool its membership and its joins are written into, and the
/// channel an open page hears about a browse on.
///
/// The machine is stated for `tests/devices.rs`'s reason — what the box running
/// this happens to be is a fact about that box — and its addresses are empty,
/// nothing here being about what *this* device says it is.
async fn workbench(browse: Browse) -> (tempfile::TempDir, SqlitePool, Router) {
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
    .browsing(browse);

    let app = router_answering_devices_telling(pool.clone(), devices, Nudges::new());

    (dir, pool, app)
}

/// The same, with the [`Nudges`] the browse announces on handed back: what an
/// open page listens to.
async fn workbench_telling(browse: Browse, nudges: Nudges) -> (tempfile::TempDir, Router) {
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
    .browsing(browse);

    let app = router_answering_devices_telling(pool, devices, nudges);

    (dir, app)
}

/// The Discovered list as the pane reads it, parsed as the type the page draws
/// — which is what makes this the shape the browser is handed rather than
/// whatever JSON came out.
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

/// And the section above it, for the tests that ask what a browse did *not* do.
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

/// Read the list until `device` is on it, and hand back its row.
///
/// Polled, which is what a *test* does rather than what the viewer does: the
/// page is told by a Nudge and reads once, and the test after these two is the
/// one that asks about that. Here the reading is also what holds the browse open,
/// so reading again is the thing being waited on as much as the way of waiting.
async fn drawn(app: &Router, device: &str) -> DiscoveredDevice {
    let deadline = Instant::now() + PATIENCE;

    loop {
        if let Some(row) = discovered(app)
            .await
            .into_iter()
            .find(|row| row.device == device)
        {
            return row;
        }

        assert!(
            Instant::now() < deadline,
            "device {device} never turned up on the Discovered list",
        );

        tokio::time::sleep(AGAIN).await;
    }
}

/// And that `devices` never turn up on it at all, which is a different kind of
/// waiting: proving a row is not going to be drawn is reading for a while and not
/// seeing it.
///
/// [`QUIET`] is spent in full every time, because there is no moment at which a
/// multicast has finished saying things. Long enough to cover the resolution of
/// the device advertised beside these, which is what says the browse was hearing
/// the wire at all.
async fn never_drawn(app: &Router, devices: &[&str]) {
    let deadline = Instant::now() + QUIET;

    while Instant::now() < deadline {
        let drawn = discovered(app).await;

        for device in devices {
            assert!(
                !drawn.iter().any(|row| &row.device == device),
                "device {device} was drawn a row: {drawn:?}",
            );
        }

        tokio::time::sleep(AGAIN).await;
    }
}

/// And until it is off it again, which is what a goodbye leaves behind.
async fn left(app: &Router, device: &str) {
    let deadline = Instant::now() + PATIENCE;

    loop {
        let drawn = discovered(app).await;

        if !drawn.iter().any(|row| row.device == device) {
            return;
        }

        assert!(
            Instant::now() < deadline,
            "device {device} stayed on the Discovered list: {drawn:?}",
        );

        tokio::time::sleep(AGAIN).await;
    }
}

/// A device advertising itself on the LAN is a row under Discovered: its name,
/// the word its mark is drawn from, where it was found, and that the LAN is
/// where.
#[tokio::test]
async fn a_device_on_the_lan_is_drawn_under_discovered() {
    let _advertising = Advertisement::of_this_device_on(HEARING, true, &announcement(OVER_THERE));

    let (_dir, _pool, app) = workbench(Browse::of_this_device_on(HEARING, Nudges::new())).await;

    let row = drawn(&app, OVER_THERE).await;

    assert_eq!(
        row.name, "kitchen-mini",
        "the name is what the row is drawn under, and it is the far end's rather \
         than this machine's",
    );
    assert_eq!(
        row.os, "macOS",
        "and the OS word is what the mark beside it comes from — the one thing \
         that tells a Windows machine from the WSL on it",
    );
    assert_eq!(
        row.found,
        vec![FoundOn::Lan],
        "found on the LAN, which is the one source there is so far",
    );

    assert!(
        !row.addresses.is_empty(),
        "and where it was found, which is what an Add will dial: {row:?}",
    );
    assert!(
        row.addresses
            .iter()
            .all(|address| address.ends_with(&format!(":{LANDED_ON}"))),
        "every one of them with the port that device's listener really landed on, \
         which is the only thing that tells two Verksteads on one machine apart: {row:?}",
    );
}

/// And a device that says goodbye leaves the list at once, rather than being
/// drawn until its records run out.
///
/// The withdrawal an ordered stop sends, which is what makes a restart tidy — see
/// `tests/withdrawing.rs`, which watches the same goodbye go out. A device that
/// was *killed* leaves the list the other way, its records expiring on their own
/// TTL, and both arrive here as the same event: what a browse hears is that the
/// instance is gone.
#[tokio::test]
async fn a_device_that_says_goodbye_leaves_the_list() {
    let advertising = Advertisement::of_this_device_on(GOODBYE, true, &announcement(OVER_THERE));
    let _beside_it = Advertisement::of_this_device_on(GOODBYE, true, &announcement(BESIDE_IT));

    let (_dir, _pool, app) = workbench(Browse::of_this_device_on(GOODBYE, Nudges::new())).await;

    drawn(&app, OVER_THERE).await;
    drawn(&app, BESIDE_IT).await;

    advertising.withdrawn().await;

    left(&app, OVER_THERE).await;

    assert!(
        discovered(&app)
            .await
            .iter()
            .any(|row| row.device == BESIDE_IT),
        "and the device beside it is untouched, which is what says the list lost \
         one row rather than the browse having stopped",
    );
}

/// The whole of how a device *arrives*: a page listening, a cold list read, and
/// the row coming down the stream.
///
/// **Which is the arrangement, rather than a way of testing it.** A browse has
/// heard nothing when it starts, so the first read of this list is empty and
/// there is nothing for a poll to shorten — what draws the row is
/// [`Nudge::Discovered`], and an open pane redraws without a reload because of
/// it (ADR-0009).
#[tokio::test]
async fn an_open_page_is_told_when_a_device_turns_up() {
    let nudges = Nudges::new();
    let (_dir, app) =
        workbench_telling(Browse::of_this_device_on(ARRIVING, nudges.clone()), nudges).await;

    // Listening before anything is on the wire, so that what it hears is the
    // device arriving rather than a backlog.
    let mut page = Listening::open(&app).await;

    assert!(
        discovered(&app).await.is_empty(),
        "the first read starts the browse, which has heard nothing yet",
    );

    let _advertising = Advertisement::of_this_device_on(ARRIVING, true, &announcement(OVER_THERE));

    assert_eq!(
        page.nudge().await,
        Nudge::Discovered,
        "the found list moving is its own kind: the cluster has not changed, and \
         the rows the pane drew of it are not to be read again for this",
    );

    assert_eq!(
        discovered(&app)
            .await
            .into_iter()
            .map(|row| row.device)
            .collect::<Vec<String>>(),
        vec![OVER_THERE],
        "and the read the word asked for is the row",
    );
}

/// And something advertising this service that says too little or too much about
/// itself is no row, whatever else is on the wire beside it.
///
/// **Which is the same judgement the tailnet half makes of what a peer answered.**
/// A row is a stranger's words drawn on somebody's page either way, and where
/// those words came from is no reason to read a different length of them — so
/// `discovery::row` holds an advertisement to the bounds a join is refused over.
///
/// **A blank as well as a missing key**, because they arrive here as one thing:
/// `mdns-sd` reads a TXT value that is empty — and one whose bytes are not UTF-8 —
/// as the empty string, so a broken advertisement looks like a present one. A row
/// with no name says nothing to the person reading it, and one with no id cannot
/// be keyed, excluded or pressed at all.
///
/// **Waited out rather than read once.** There is no moment at which a multicast
/// has finished saying things, so what says these two are not coming is reading
/// for [`QUIET`] and not seeing them — while the proper device advertised beside
/// them is what says the browse was hearing the wire at all.
#[tokio::test]
async fn an_advertisement_that_says_too_little_or_too_much_is_no_row() {
    let nameless = Announcement {
        name: String::new(),
        ..announcement(BESIDE_IT)
    };

    // Past the bound an OS word is held to, and inside the 255 bytes one TXT
    // string carries — which is what makes it something a device can really say.
    let shouting = Announcement {
        os: "macOS".repeat(40),
        ..announcement(A_THIRD)
    };

    let _nameless = Advertisement::of_this_device_on(TOO_MUCH, true, &nameless);
    let _shouting = Advertisement::of_this_device_on(TOO_MUCH, true, &shouting);
    let _proper = Advertisement::of_this_device_on(TOO_MUCH, true, &announcement(OVER_THERE));

    let (_dir, _pool, app) = workbench(Browse::of_this_device_on(TOO_MUCH, Nudges::new())).await;

    drawn(&app, OVER_THERE).await;

    never_drawn(&app, &[BESIDE_IT, A_THIRD]).await;

    assert_eq!(
        discovered(&app)
            .await
            .into_iter()
            .map(|row| row.device)
            .collect::<Vec<String>>(),
        vec![OVER_THERE],
        "so the one that said what a device says is the whole of the list",
    );
}

/// A **Member** is not drawn: it is in the cluster already, and a row offering to
/// link it would be a press with nothing behind it.
#[tokio::test]
async fn a_member_is_not_drawn() {
    let (_dir, pool, app) =
        workbench(Browse::stated(vec![heard(OVER_THERE), heard(A_THIRD)])).await;

    verkstead_store::record_member(
        &pool,
        &verkstead_store::Linking {
            device: OVER_THERE.to_owned(),
            name: "laptop".to_owned(),
            os: "macOS".to_owned(),
            addresses: vec!["192.168.1.31".to_owned()],
            fingerprint: format!("AA:BB:CC:DD:{OVER_THERE}"),
        },
    )
    .await
    .unwrap();

    assert_eq!(
        discovered(&app)
            .await
            .into_iter()
            .map(|row| row.device)
            .collect::<Vec<String>>(),
        vec![A_THIRD],
        "the device that is not a member is still drawn, which is what says the \
         member was left out rather than the list being empty",
    );

    assert_eq!(
        listing(&app).await.members.len(),
        1,
        "and everything about the member is unchanged: it is a row up there, which \
         is exactly why it is not a row down here",
    );
}

/// This device is not drawn, because it hears its own advertisement — and a
/// Verkstead is not linked to itself.
#[tokio::test]
async fn this_device_is_not_drawn() {
    let (_dir, _pool, app) =
        workbench(Browse::stated(vec![heard(THIS_DEVICE), heard(A_THIRD)])).await;

    assert_eq!(
        discovered(&app)
            .await
            .into_iter()
            .map(|row| row.device)
            .collect::<Vec<String>>(),
        vec![A_THIRD],
    );
}

/// And a device this one holds a **Join** for is not drawn: the pending row under
/// the list is already the answer to the press somebody made.
///
/// **A row that was refused counts too.** It is still drawn up there until
/// somebody dismisses it, and a discovered row for the same device beside it
/// would be two things to press about one machine.
#[tokio::test]
async fn a_device_a_join_is_pending_for_is_not_drawn() {
    let (_dir, pool, app) = workbench(Browse::stated(vec![
        heard(OVER_THERE),
        heard(BESIDE_IT),
        heard(A_THIRD),
    ]))
    .await;

    for (request, device, refused) in [
        ("1122334455667788", OVER_THERE, false),
        ("8877665544332211", BESIDE_IT, true),
    ] {
        verkstead_store::ask_join(
            &pool,
            &verkstead_store::AskedJoin {
                request: request.to_owned(),
                address: format!("192.168.1.31:{LANDED_ON}"),
                device: device.to_owned(),
                name: "laptop".to_owned(),
                fingerprint: format!("AA:BB:CC:DD:{device}"),
                asked_at: "2026-09-25T10:00:00Z".to_owned(),
                expires_at: "2099-01-01T00:00:00Z".to_owned(),
                refused,
            },
        )
        .await
        .unwrap();
    }

    assert_eq!(
        discovered(&app)
            .await
            .into_iter()
            .map(|row| row.device)
            .collect::<Vec<String>>(),
        vec![A_THIRD],
        "the one nothing has been asked of is drawn; the one being waited on and \
         the one that said no are both already rows under the list",
    );
}

/// And the two lists are two readings: what a browse heard is nowhere in the
/// section above it.
///
/// **Which is the point of the second endpoint.** A browse hears something every
/// few seconds, and a list that arrived on the same answer as the membership
/// would be the cluster's own rows at the mercy of whatever the LAN said — so
/// drawing the Discovered list again re-reads nothing of the membership, and a
/// browse that found nothing leaves those rows exactly where they were.
#[tokio::test]
async fn what_was_heard_is_nowhere_in_the_devices_reading() {
    let (_dir, _pool, app) = workbench(Browse::stated(vec![heard(OVER_THERE)])).await;

    let listing = listing(&app).await;

    assert_eq!(listing.this.device, THIS_DEVICE, "this device is the row");
    assert!(
        listing.members.is_empty(),
        "a device heard of is not a member: nothing has been agreed with it, and \
         nobody has pressed anything",
    );
    assert!(
        listing.pending.is_empty(),
        "and no join has been asked for either",
    );

    assert_eq!(
        discovered(&app).await.len(),
        1,
        "while the list of its own draws it",
    );
}

/// One of this device's open workbenches, listening on the Nudge stream.
///
/// Read off the wire rather than off the channel behind it, the way
/// `tests/unlinking.rs` reads one: what has to hold is that a page which pressed
/// nothing hears about the list moving down its own connection.
struct Listening {
    body: Body,

    /// What has been read off the stream and is not a whole frame yet. SSE frames
    /// are not the chunks they arrive in.
    buffered: String,
}

impl Listening {
    /// Open the stream the way a page does. Returns once the response is in hand,
    /// which is after the handler has subscribed — so anything the test does next
    /// is something this page is listening for.
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
        let waited_for = tokio::time::timeout(PATIENCE, async {
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
