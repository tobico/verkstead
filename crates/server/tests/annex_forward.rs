//! The **Annex Forward**: a Keeper's listener of its own, forwarding raw TCP to
//! its Annex's peer listener (ADR-0024, *The Keeper forwards for its Annex*).
//!
//! **Two Verksteads and a real socket each.** What is being asked is that what
//! arrives at the Keeper's forward is the Annex — its certificate presented in
//! the handshake, its identity endpoint answering — and the only way to ask
//! that is to dial the forward the way a third device would: pinned to the
//! Annex's fingerprint, at the Keeper's port. So the Annex stands its peer
//! listener up on the loopback, the Keeper stands its forward up beside it, and
//! the Keeper links the Annex through the same workbench route stage 03 calls
//! at every boot.
//!
//! Ports the machine picks, for the reason every suite here takes one: the real
//! ones would fight whatever is already on 8423 and 8424, and two of these
//! tests at once would fight each other.

use std::net::SocketAddr;
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use sqlx::SqlitePool;
use tokio::io::AsyncReadExt;
use tower::ServiceExt;
use verkstead_server::device::reading::Reading;
use verkstead_server::device::{Device, Devices};
use verkstead_server::nudge::Nudges;
use verkstead_server::open_database;
use verkstead_server::peer::annex_forward::{self, Pointing};
use verkstead_server::peer::dialling::Peers;
use verkstead_server::peer::joining::Joins;
use verkstead_server::peer::{self, Members};
use verkstead_server::platform::Platform;
use verkstead_server::remote::Tailscale;
use verkstead_server::router_answering_devices_telling;
use verkstead_server::settings::Settings;

/// The ids the two devices are stated as.
const KEEPER: &str = "aa00bb11cc22dd33ee44ff5566778899";
const ANNEX: &str = "0011223344556677889900aabbccddee";

/// Where the linking operation is posted.
const ANNEXES: &str = "/api/ui/devices/annexes";

/// The port the workbench is taken to be on, which nothing here asks about.
const PORT: u16 = 8422;

/// How long a connection the forward closes is waited on before the test says
/// it was left open.
const CLOSED_WITHIN: Duration = Duration::from_secs(5);

/// A machine with no Tailscale on it.
fn no_tailscale() -> Tailscale {
    Tailscale::running(vec!["verkstead-no-such-tailscale".to_owned()], PORT)
}

/// One Verkstead: what it is, where it keeps things, and the Data Directory
/// they are in — the Annex's is what the Keeper links it by.
struct Verkstead {
    device: Device,
    pool: SqlitePool,
    members: Members,
    joins: Joins,
    nudges: Nudges,
    reading: Reading,
    dir: tempfile::TempDir,
}

impl Verkstead {
    async fn called(id: &str) -> Verkstead {
        let dir = tempfile::tempdir().unwrap();
        let pool = open_database(&dir.path().join("verkstead.db"))
            .await
            .unwrap();

        Verkstead {
            device: Device::stated(dir.path(), id).unwrap(),
            members: Members::recorded(pool.clone()),
            joins: Joins::recorded(pool.clone()),
            nudges: Nudges::new(),
            reading: Reading::stated(no_tailscale(), Platform::Linux, None, Vec::new()),
            pool,
            dir,
        }
    }

    /// The Annex, told by its own `config.yaml` whose it is, with its peer
    /// listener up; handed back with where that landed.
    async fn annex() -> (Verkstead, SocketAddr) {
        let mut annex = Verkstead::called(ANNEX).await;

        annex.reading = annex
            .reading
            .clone()
            .told_by(Settings::in_data_dir(annex.dir.path()));

        let listener = peer::Listener::bound("127.0.0.1:0".parse().unwrap(), &annex.device)
            .expect("the loopback on a port the machine picked is free");
        let at = listener.address();

        tokio::spawn(listener.serving(peer::router(
            annex.device.clone(),
            annex.reading.clone(),
            annex.members.clone(),
            annex.joins.clone(),
            annex.nudges.clone(),
            Router::new(),
        )));

        (annex, at)
    }

    /// The Keeper, with its Annex Forward up and pointed nowhere; handed back
    /// with the handle the forward reads and where it landed.
    async fn keeper() -> (Verkstead, Pointing, SocketAddr) {
        let keeper = Verkstead::called(KEEPER).await;

        let forward = annex_forward::Listener::bound("127.0.0.1:0".parse().unwrap())
            .expect("the loopback on a port the machine picked is free");
        let at = forward.address();
        let pointing = Pointing::at_nothing();

        tokio::spawn(forward.serving(pointing.clone()));

        (keeper, pointing, at)
    }

    /// The workbench a Keeper's start serves, its Devices holding the handle
    /// its forward reads.
    fn workbench(&self, pointing: &Pointing) -> Router {
        let devices = Devices::of(
            self.device.clone(),
            self.reading.clone(),
            self.members.clone(),
            self.joins.clone(),
        )
        .forwarding_for_the_annex(pointing.clone());

        router_answering_devices_telling(self.pool.clone(), devices, self.nudges.clone())
    }

    fn peers(&self) -> Peers {
        Peers::of(self.device.clone(), self.members.clone())
    }
}

/// Link the Annex whose Data Directory is `annex`'s, at `address`.
async fn keep(app: &Router, annex: &Verkstead, address: SocketAddr) {
    let body = serde_json::to_vec(&serde_json::json!({
        "data_dir": annex.dir.path().to_str().unwrap(),
        "address": address.to_string(),
    }))
    .unwrap();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(ANNEXES)
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();

    assert_eq!(
        status,
        StatusCode::OK,
        "POST {ANNEXES}: {}",
        String::from_utf8_lossy(&bytes),
    );
}

/// Dial the Annex the way a third device would, at the Keeper's forward and
/// pinned to the fingerprint the Keeper holds for it: a handshake on any other
/// certificate is refused, so an answer is the Annex's own TLS, end to end.
async fn through_the_forward(
    keeper: &Verkstead,
    forward: SocketAddr,
) -> anyhow::Result<verkstead_render::DeviceIdentity> {
    let row = verkstead_store::members(&keeper.pool)
        .await
        .unwrap()
        .into_iter()
        .find(|member| member.device == ANNEX)
        .expect("the Keeper holds its Annex");

    keeper
        .peers()
        .identity(&verkstead_store::Member {
            addresses: vec![forward.to_string()],
            ..row
        })
        .await
}

/// After the Keeper links its Annex, a dial to the Keeper's forward completes
/// its handshake on the Annex's certificate and the Annex's identity endpoint
/// answers through it.
#[tokio::test]
async fn a_dial_to_the_forward_is_answered_by_the_annex() {
    let (annex, annex_at) = Verkstead::annex().await;
    let (keeper, pointing, forward) = Verkstead::keeper().await;

    keep(&keeper.workbench(&pointing), &annex, annex_at).await;

    let answered = through_the_forward(&keeper, forward)
        .await
        .expect("the forward puts a pinned dial through to the Annex");

    assert_eq!(answered.device, ANNEX);
    assert_eq!(answered.fingerprint, annex.device.fingerprint());
    assert_ne!(
        answered.fingerprint,
        keeper.device.fingerprint(),
        "the TLS is the Annex's, and the Keeper never terminates it",
    );
}

/// Before any Annex is linked, a connection to the forward is closed with
/// nothing answering.
#[tokio::test]
async fn a_forward_with_no_annex_closes_what_it_accepts() {
    let (_keeper, pointing, forward) = Verkstead::keeper().await;

    assert_eq!(pointing.pointed_at(), None);

    let mut caller = tokio::net::TcpStream::connect(forward)
        .await
        .expect("the forward is listening");

    let mut said = [0u8; 64];
    let read = tokio::time::timeout(CLOSED_WITHIN, caller.read(&mut said))
        .await
        .expect("the forward closes the connection rather than holding it open");

    assert!(
        matches!(read, Ok(0) | Err(_)),
        "nothing answers, the connection is only closed: {read:?}",
    );
}

/// Linking twice leaves one forward, pointed at the Annex — and still putting
/// a dial through to it.
#[tokio::test]
async fn linking_twice_leaves_one_forward_pointed_at_the_annex() {
    let (annex, annex_at) = Verkstead::annex().await;
    let (keeper, pointing, forward) = Verkstead::keeper().await;

    for _ in 0..2 {
        keep(&keeper.workbench(&pointing), &annex, annex_at).await;
    }

    assert_eq!(pointing.pointed_at(), Some(annex_at.to_string()));

    let answered = through_the_forward(&keeper, forward)
        .await
        .expect("the one forward still puts a dial through to the Annex");

    assert_eq!(answered.device, ANNEX);
}
