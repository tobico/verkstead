//! **Advertise as told**: an Annex advertises its Keeper's addresses at the
//! Annex Forward's port rather than its own interfaces, asked of the Keeper
//! over a member's route at each reading (ADR-0024; CONTEXT.md, **Annex
//! Forward**).
//!
//! **Three servers and every call real.** What is being asked is that a third
//! device records an Annex at an address it can reach, and goes on recording
//! it there — every identity dial overwriting a member's addresses with what
//! that member advertises. So each Verkstead here stands its peer listener up
//! on the loopback, the Keeper stands its forward up beside it, and the Annex
//! asks the Keeper what to advertise over a pinned dial of its own.
//!
//! Ports the machine picks, for the reason every suite here takes one: the real
//! ones would fight whatever is already on 8423 and 8424.

use std::net::SocketAddr;
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use sqlx::SqlitePool;
use tokio::task::JoinHandle;
use tower::ServiceExt;
use verkstead_render::{AskingDevice, DeviceIdentity};
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

/// The Keeper, its Annex, and a third device linked to the Keeper.
const A: &str = "aa00bb11cc22dd33ee44ff5566778899";
const B: &str = "0011223344556677889900aabbccddee";
const C: &str = "ffeeddccbbaa00998877665544332211";

/// Where the pane's presses go.
const ADD: &str = "/api/ui/devices/joins";
const ASKING: &str = "/api/ui/devices/asking";
const ANNEXES: &str = "/api/ui/devices/annexes";

/// The port the workbench is taken to be on, which nothing here asks about.
const PORT: u16 = 8422;

/// How long a dial in this suite gives an address nothing answers at.
const PATIENCE: Duration = Duration::from_millis(300);

fn no_tailscale() -> Tailscale {
    Tailscale::running(vec!["verkstead-no-such-tailscale".to_owned()], PORT)
}

/// What a Verkstead is in this suite.
#[derive(Clone, Copy, PartialEq)]
enum Role {
    /// An ordinary device.
    Member,

    /// A Keeper, with its Annex Forward up beside its peer listener.
    Keeper,

    /// An Annex, told by its own `config.yaml` whose it is and asking its
    /// Keeper what to advertise at every reading — holding no answer between
    /// two, so that every reading in a test is an ask.
    Annex,
}

struct Verkstead {
    device: Device,
    pool: SqlitePool,
    members: Members,
    joins: Joins,
    nudges: Nudges,
    reading: Reading,

    /// Where its peer listener landed, which is the one address it advertises
    /// of its own.
    address: SocketAddr,

    /// The listener, served — held so that a test can stop it.
    serving: Option<JoinHandle<anyhow::Result<()>>>,

    /// And a Keeper's forward: the handle it reads and where it landed.
    forward: Option<(Pointing, SocketAddr)>,

    dir: tempfile::TempDir,
}

impl Verkstead {
    async fn standing(id: &str, role: Role) -> Verkstead {
        let dir = tempfile::tempdir().unwrap();
        let pool = open_database(&dir.path().join("verkstead.db"))
            .await
            .unwrap();

        let device = Device::stated(dir.path(), id).unwrap();
        let members = Members::recorded(pool.clone());

        let listener = peer::Listener::bound("127.0.0.1:0".parse().unwrap(), &device)
            .expect("the loopback on a port the machine picked is free");
        let address = listener.address();

        let mut reading = Reading::advertising(
            no_tailscale(),
            Platform::Linux,
            None,
            vec![format!("127.0.0.1:{}", address.port())],
        );

        let mut forward = None;

        match role {
            Role::Member => {}

            Role::Keeper => {
                let bound = annex_forward::Listener::bound("127.0.0.1:0".parse().unwrap())
                    .expect("the loopback on a port the machine picked is free");
                let at = bound.address();
                let pointing = Pointing::at_nothing();

                tokio::spawn(bound.serving(pointing.clone()));

                reading = reading.forwarding_at(at.port());
                forward = Some((pointing, at));
            }

            Role::Annex => {
                reading = reading
                    .told_by(Settings::in_data_dir(dir.path()))
                    .asking_its_keeper(Peers::of(device.clone(), members.clone()).waiting(PATIENCE))
                    .holding_its_keepers_answer_for(Duration::ZERO);
            }
        }

        let joins = Joins::recorded(pool.clone());
        let nudges = Nudges::new();

        let serving = tokio::spawn(listener.serving(peer::router(
            device.clone(),
            reading.clone(),
            members.clone(),
            joins.clone(),
            nudges.clone(),
            Router::new(),
        )));

        Verkstead {
            device,
            pool,
            members,
            joins,
            nudges,
            reading,
            address,
            serving: Some(serving),
            forward,
            dir,
        }
    }

    /// Where another device dials its own peer listener.
    fn at(&self) -> String {
        format!("127.0.0.1:{}", self.address.port())
    }

    /// Where a Keeper's forward is, as an address a dial reads.
    fn forward(&self) -> String {
        let (_, at) = self.forward.as_ref().expect("a Keeper forwards");
        at.to_string()
    }

    /// Stop its peer listener, and wait until nothing is listening there.
    async fn stop(&mut self) {
        if let Some(serving) = self.serving.take() {
            serving.abort();
            let _ = serving.await;
        }
    }

    fn workbench(&self) -> Router {
        let devices = Devices::of(
            self.device.clone(),
            self.reading.clone(),
            self.members.clone(),
            self.joins.clone(),
        )
        .waiting(PATIENCE);

        let devices = match &self.forward {
            Some((pointing, _)) => devices.forwarding_for_the_annex(pointing.clone()),
            None => devices,
        };

        router_answering_devices_telling(self.pool.clone(), devices, self.nudges.clone())
    }

    fn peers(&self) -> Peers {
        Peers::of(self.device.clone(), self.members.clone()).waiting(PATIENCE)
    }

    /// The row this device holds for `device`.
    async fn row_for(&self, device: &str) -> verkstead_store::Member {
        verkstead_store::members(&self.pool)
            .await
            .unwrap()
            .into_iter()
            .find(|member| member.device == device)
            .unwrap_or_else(|| panic!("{device} is a member"))
    }

    /// What its identity endpoint answers a device nobody has heard of.
    async fn identity(&self) -> DeviceIdentity {
        let asking = Verkstead::standing("1234567890abcdef1234567890abcdef", Role::Member).await;

        asking
            .peers()
            .stranger(&self.at())
            .await
            .expect("the identity endpoint answers anybody")
    }
}

async fn posted(app: &Router, uri: &str, body: serde_json::Value) -> (StatusCode, String) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();

    (status, String::from_utf8_lossy(&bytes).into_owned())
}

/// `joining` linked to `answering` the ordinary way: Add pressed on one, Allow
/// on the other.
async fn linked(joining: &Verkstead, answering: &Verkstead) {
    let (status, said) = posted(
        &joining.workbench(),
        ADD,
        serde_json::json!({ "address": answering.at() }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "POST {ADD}: {said}");

    let response = answering
        .workbench()
        .oneshot(Request::builder().uri(ASKING).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let held: Vec<AskingDevice> = serde_json::from_slice(&bytes).unwrap();

    let (status, said) = posted(
        &answering.workbench(),
        &format!("{ASKING}/{}/allow", held[0].request),
        serde_json::json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "allow: {said}");
}

/// `keeper` links `annex` by the Keeper's Join, at the Annex's own peer
/// listener.
async fn kept(keeper: &Verkstead, annex: &Verkstead) {
    let (status, said) = posted(
        &keeper.workbench(),
        ANNEXES,
        serde_json::json!({
            "data_dir": annex.dir.path().to_str().unwrap(),
            "address": annex.at(),
        }),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "POST {ANNEXES}: {said}");
}

/// After the Keeper's Join, the Annex's identity endpoint lists the Keeper's
/// addresses at the forward's port and none of its own.
#[tokio::test]
async fn an_annex_advertises_its_keepers_addresses_at_the_forward() {
    let a = Verkstead::standing(A, Role::Keeper).await;
    let b = Verkstead::standing(B, Role::Annex).await;

    kept(&a, &b).await;

    let identity = b.identity().await;

    assert_eq!(
        identity.addresses,
        vec![a.forward()],
        "the Keeper's address, at the forward rather than its peer port",
    );
    assert!(
        !identity.addresses.contains(&b.at()),
        "and none of the Annex's own",
    );
    assert_eq!(
        a.row_for(B).await.addresses,
        vec![a.forward()],
        "the Keeper holds its Annex at what the Annex advertises",
    );
}

/// A third device, told about the Annex by its Keeper, records it at the
/// Keeper's forward, reaches it there on the Annex's own certificate, and
/// still holds it there once that dial has refreshed the row.
#[tokio::test]
async fn a_third_device_reaches_the_annex_through_the_forward() {
    let a = Verkstead::standing(A, Role::Keeper).await;
    let b = Verkstead::standing(B, Role::Annex).await;
    let c = Verkstead::standing(C, Role::Member).await;

    linked(&c, &a).await;
    kept(&a, &b).await;

    let row = c.row_for(B).await;
    assert_eq!(
        row.addresses,
        vec![a.forward()],
        "C was told the Annex is at the Keeper's forward",
    );

    // Pinned to the fingerprint C holds for B, so an answer at all is a
    // handshake on B's certificate.
    let answered = c
        .peers()
        .identity(&row)
        .await
        .expect("C reaches the Annex through the forward");

    assert_eq!(answered.device, B);
    assert_eq!(answered.fingerprint, b.device.fingerprint());

    assert_eq!(
        c.row_for(B).await.addresses,
        vec![a.forward()],
        "the refreshed row is still the Keeper's forward",
    );
}

/// With the Keeper gone after one answer, the Annex goes on advertising that
/// answer.
#[tokio::test]
async fn an_annex_whose_keeper_has_gone_advertises_the_last_answer() {
    let mut a = Verkstead::standing(A, Role::Keeper).await;
    let b = Verkstead::standing(B, Role::Annex).await;

    kept(&a, &b).await;
    assert_eq!(b.identity().await.addresses, vec![a.forward()]);

    a.stop().await;

    assert_eq!(
        b.identity().await.addresses,
        vec![a.forward()],
        "the last answer stands while the Keeper cannot be asked",
    );
    assert!(
        !b.row_for(A).await.reachable,
        "the Annex did ask, and found its Keeper gone",
    );
}

/// And an Annex that has never reached its Keeper advertises nothing at all —
/// not its own addresses.
#[tokio::test]
async fn an_annex_that_never_reached_its_keeper_advertises_nothing() {
    let mut a = Verkstead::standing(A, Role::Keeper).await;
    let b = Verkstead::standing(B, Role::Annex).await;

    a.stop().await;
    kept(&a, &b).await;

    assert_eq!(b.identity().await.addresses, Vec::<String>::new());
}
