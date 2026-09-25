//! What the **Devices** section of the Remote access pane reads: this device,
//! and how many others are linked to it (ADR-0020).
//!
//! The workbench's side of `tests/peer.rs`. That suite dials the identity
//! endpoint the way a stranger does, over TLS on the peer port; this one asks
//! the same question over the workbench, which is what the browser drawing the
//! pane can actually reach. What both of them are about is one answer told to
//! two askers, so this suite's job is that the answer really is the same one —
//! and that a server with no identity refuses rather than inventing a row.
//!
//! The machine is stated rather than read, for the reason `tests/peer.rs`
//! states one: what a WSL reads as is the whole point of the OS word, and the
//! box a suite happens to be running on is the one machine that cannot be asked
//! about it. The hostname is the one thing left read off the machine, because
//! there is one reading of it and nothing to compare it against but itself.

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;
use verkstead_render::DevicesView;
use verkstead_server::device::reading::Reading;
use verkstead_server::device::{Device, Devices};
use verkstead_server::peer::Members;
use verkstead_server::platform::{self, Platform};
use verkstead_server::remote::Tailscale;
use verkstead_server::{open_database, router, router_answering_devices};

/// Where the pane reads this device from.
const DEVICES: &str = "/api/ui/devices";

/// The id this device is stated as, so that what a test asserts against is a
/// string it chose rather than sixteen random bytes it has to filter out of a
/// payload — see [`Device::stated`], which is here for that reason.
const THIS_DEVICE: &str = "aa00bb11cc22dd33ee44ff5566778899";

/// What a WSL kernel calls itself, which is the one thing that says one apart
/// from the Linux it is in every other way.
const WSL_KERNEL: &str = "5.15.167.4-microsoft-standard-WSL2";

/// The port the workbench is taken to be on, which nothing here asks about: the
/// reading's Tailscale is built with it and never runs.
const PORT: u16 = 8422;

/// A machine with no Tailscale on it: `verkstead-no-such-tailscale` is a
/// program that is not there, which is what having none *is*.
fn no_tailscale() -> Tailscale {
    Tailscale::running(vec!["verkstead-no-such-tailscale".to_owned()], PORT)
}

/// A router answering for a device on a stated machine, and the directory
/// holding that device's files alive.
async fn app(reading: Reading) -> (tempfile::TempDir, Router) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    let device = Device::stated(dir.path(), THIS_DEVICE).unwrap();
    let devices = Devices::of(device, reading, Members::none());

    (dir, router_answering_devices(pool, devices))
}

/// The machine this device is on where nothing about it is interesting: this
/// platform, no Tailscale, and no interface worth advertising.
fn plainly() -> Reading {
    Reading::stated(no_tailscale(), Platform::HERE, None, Vec::new())
}

/// What the endpoint answered, parsed as the pane's own type — which is what
/// makes this the shape the browser is handed rather than whatever JSON came
/// out.
async fn listing(app: &Router) -> DevicesView {
    let response = app
        .clone()
        .oneshot(Request::builder().uri(DEVICES).body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK, "GET {DEVICES}");

    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap_or_else(|why| {
        panic!(
            "the Devices reading should be the type the pane draws: {why}\n{}",
            String::from_utf8_lossy(&bytes),
        )
    })
}

/// The list is this device, named by the id every record names it by and
/// carrying the fingerprint a link is pinned on.
#[tokio::test]
async fn the_list_is_this_device() {
    let (dir, app) = app(plainly()).await;
    let device = Device::issued(dir.path()).unwrap();

    let listing = listing(&app).await;

    assert_eq!(listing.this.device, THIS_DEVICE);
    assert_eq!(
        listing.this.fingerprint,
        device.fingerprint(),
        "the row the pane draws and the certificate this device presents have to \
         be the same device's, or the section is a list of somebody else",
    );
}

/// And nothing is linked to it, which is the clause the Remote access card
/// carries beside what Tailscale is doing.
///
/// Nought rather than one: this device is the row the list holds, and a device
/// is not linked to itself. A count that read one here would put *1 device
/// linked* on the card of a Verkstead that has never met another.
#[tokio::test]
async fn nothing_is_linked_to_it_yet() {
    let (_dir, app) = app(plainly()).await;

    assert_eq!(
        listing(&app).await.linked,
        0,
        "a member is made by a join, and there is no join to make one with yet",
    );
}

/// The name is the hostname read off the machine, with nothing configurable
/// about it, and the OS is this platform's own word.
///
/// Checked against the one reading there is of either rather than against a
/// string in this file: a hostname read off the box the suite is running on
/// would be a golden fixture nobody could commit.
#[tokio::test]
async fn the_row_names_the_machine_this_device_is_on() {
    let (_dir, app) = app(plainly()).await;

    let listing = listing(&app).await;

    assert_eq!(listing.this.name, platform::hostname());
    assert!(
        !listing.this.name.is_empty(),
        "and a machine that will not say what it is called still reads as something",
    );
    assert_eq!(
        listing.this.os,
        platform::os_word(Platform::HERE, platform::kernel_release().as_deref()),
    );
}

/// The case the whole roadmap was written for, as the pane draws it: a Windows
/// machine and the WSL on it share a hostname, so the OS is the only thing that
/// tells the two rows apart — and it is the OS word that picks the icon beside
/// the name.
#[tokio::test]
async fn a_wsl_reads_linux_wsl() {
    let wsl = Reading::stated(
        no_tailscale(),
        Platform::Linux,
        Some(WSL_KERNEL.to_owned()),
        Vec::new(),
    );

    let (_dir, app) = app(wsl).await;

    assert_eq!(listing(&app).await.this.os, "Linux (WSL)");
}

/// And the addresses are the machine's own, in the order a peer should try
/// them — the same list the identity endpoint answers, because it is the same
/// reading assembled twice.
#[tokio::test]
async fn the_row_carries_the_addresses_this_device_is_reachable_on() {
    let on_the_lan = Reading::stated(
        no_tailscale(),
        Platform::HERE,
        None,
        vec!["192.168.1.24".parse().unwrap(), "10.0.0.7".parse().unwrap()],
    );

    let (_dir, app) = app(on_the_lan).await;

    assert_eq!(
        listing(&app).await.this.addresses,
        vec!["192.168.1.24", "10.0.0.7"],
    );
}

/// A server that was never given an identity refuses rather than answering
/// about a device that does not exist.
///
/// Which is every router but the served one: a device is invented in a Data
/// Directory, and one with nowhere to have invented it has nothing to answer
/// for. The same judgement **Reset key** makes about a router behind no gate.
#[tokio::test]
async fn a_server_with_no_device_refuses() {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    let response = router(pool)
        .oneshot(Request::builder().uri(DEVICES).body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::INTERNAL_SERVER_ERROR,
        "an answer about a device that was never made is the one answer this \
         endpoint must not give",
    );
}
