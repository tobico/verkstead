//! **Update** on a member's Devices row: the member told over the link, and
//! its own account of the Update drawn on its row here (ADR-0027, *A device
//! updates itself*).
//!
//! **Two Verksteads, a real dial and a real stream.** The pressing device holds
//! the member's Nudge stream and reads its list of Conversations as a running
//! server does, and the member answers on its own Peer Listener behind the
//! Member Gate. What stands in for the member's desktop app is the line it
//! would act on, heard in process (`Asking::heard`), and the reports it would
//! post to its own server's workbench.
//!
//! **The sessions running on the member** are its own Conversations with
//! `working` said of them on the way out — a layer over its listener, because
//! a session running is a process, and this suite is about the card naming
//! them rather than about running one.

use std::net::SocketAddr;
use std::path::Path;
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use http_body_util::BodyExt;
use serde_json::json;
use sqlx::SqlitePool;
use tokio::task::JoinHandle;
use tower::ServiceExt;
use verkstead_render::{DevicesView, InstallKind, LinkedDevice, ThisUpdating};
use verkstead_server::device::reading::{RUNNING, Reading};
use verkstead_server::device::{Device, Devices};
use verkstead_server::nudge::Nudges;
use verkstead_server::onboarding::Machine;
use verkstead_server::peer::joining::Joins;
use verkstead_server::peer::{self, Members};
use verkstead_server::platform::Platform;
use verkstead_server::remote::Tailscale;
use verkstead_server::updating::Asking;
use verkstead_server::{
    Routers, open_database, router_answering_devices_telling, routers_answering_devices_telling,
    store,
};
use verkstead_store::{Linking, record_member};

/// The device the human presses on.
const A: &str = "aa00bb11cc22dd33ee44ff5566778899";

/// And crow, the member it updates.
const CROW: &str = "0011223344556677889900aabbccddee";

/// The branches crow's two sessions are running on.
const RUNNING_THERE: [&str; 2] = ["rate-limiting", "solid-viewer"];

/// How long one address has to answer here.
const PATIENCE: Duration = Duration::from_millis(300);

/// How long a test waits for a row to come to read what it expects. Generous,
/// because it is only ever paid when the assertion is about to fail.
const WAITING: Duration = Duration::from_secs(10);

/// The line crow's desktop app acts on.
const ASKED: &str = "verkstead-desktop: update this device";

/// crow: its own workbench, which its desktop app reports to, and what that
/// app has been asked.
struct Crow {
    device: Device,
    address: SocketAddr,
    pool: SqlitePool,
    workbench: Router,
    asking: Asking,
    _dir: tempfile::TempDir,
}

impl Crow {
    /// crow installed as `install`, with two sessions running on it, its Peer
    /// Listener up.
    async fn installed_as(install: InstallKind) -> Crow {
        let dir = tempfile::tempdir().unwrap();
        let pool = open_database(&dir.path().join("verkstead.db"))
            .await
            .unwrap();

        let device = Device::stated(dir.path(), CROW).unwrap();
        let members = Members::recorded(pool.clone());
        let nudges = Nudges::new();

        let listener = peer::Listener::bound("127.0.0.1:0".parse().unwrap(), &device)
            .expect("the loopback on a port the machine picked is free");
        let address = listener.address();

        let reading = Reading::advertising(
            no_tailscale(),
            Platform::Linux,
            None,
            vec![format!("127.0.0.1:{}", address.port())],
        )
        .calling_itself("crow", "Linux")
        .installed_as(Some(install));

        let asking = Asking::heard();

        let devices = Devices::of(
            device.clone(),
            reading.clone(),
            members.clone(),
            Joins::none(),
        )
        .asking_the_app(asking.clone());

        let Routers {
            workbench,
            over_the_link,
        } = routers_answering_devices_telling(
            pool.clone(),
            devices,
            nudges.clone(),
            Machine::here(),
        );

        let serving = peer::router(
            device.clone(),
            reading,
            members,
            Joins::none(),
            nudges,
            over_the_link,
        )
        .layer(middleware::from_fn(sessions_running));

        tokio::spawn(listener.serving(serving));

        let repo = store::register_repo(&pool, Path::new("/srv/verkstead"), "verkstead", "main")
            .await
            .unwrap()
            .unwrap();

        for branch in RUNNING_THERE {
            store::start_conversation(&pool, repo.id, branch, CROW)
                .await
                .unwrap()
                .unwrap();
        }

        Crow {
            device,
            address,
            pool,
            workbench,
            asking,
            _dir: dir,
        }
    }

    /// crow's desktop app reporting how much of the release is down.
    async fn reports_progress(&self, percent: u8) {
        self.reports("progress", json!({ "percent": percent }))
            .await;
    }

    /// And that the Update failed, with why.
    async fn reports_failure(&self, why: &str) {
        self.reports("failed", json!({ "why": why })).await;
    }

    async fn reports(&self, what: &str, said: serde_json::Value) {
        let answered = self
            .workbench
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/ui/devices/this/update/{what}"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(said.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(answered.status(), StatusCode::NO_CONTENT);
    }
}

/// Every Conversation crow answers its list with, said to have a session
/// running on it and to be called by its branch.
async fn sessions_running(asked: Request<Body>, onwards: Next) -> Response {
    let listing = asked.uri().path() == "/api/ui/conversations";
    let answered = onwards.run(asked).await;

    if !listing || !answered.status().is_success() {
        return answered;
    }

    let bytes = answered.into_body().collect().await.unwrap().to_bytes();
    let mut rows: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

    for row in rows.as_array_mut().expect("a list of Conversations") {
        row["working"] = json!(true);
        row["branch_named"] = json!(true);
    }

    axum::Json(rows).into_response()
}

/// The pressing device: its store, its cluster, and the Nudge stream to crow
/// held for as long as the test runs.
struct Pressing {
    pool: SqlitePool,
    devices: Devices,
    nudges: Nudges,
    workbench: Router,
    _holding: Holding,
    _dir: tempfile::TempDir,
}

impl Pressing {
    /// The pressing device, linked to crow at the address crow listens on.
    async fn linked_to(crow: &Crow) -> Pressing {
        Pressing::linked_at(crow, &format!("127.0.0.1:{}", crow.address.port())).await
    }

    /// The same, its row saying crow is at `address`.
    async fn linked_at(crow: &Crow, address: &str) -> Pressing {
        let dir = tempfile::tempdir().unwrap();
        let pool = open_database(&dir.path().join("verkstead.db"))
            .await
            .unwrap();

        let device = Device::stated(dir.path(), A).unwrap();

        let reading = Reading::advertising(
            no_tailscale(),
            Platform::Linux,
            None,
            vec!["127.0.0.1:1".to_owned()],
        )
        .calling_itself("wolf", "Linux");

        // Both ways, a link being both ways: this device dials crow pinned on
        // its fingerprint, and crow admits it on its own.
        record_member(
            &pool,
            &Linking {
                device: CROW.to_owned(),
                name: "crow".to_owned(),
                os: "Linux".to_owned(),
                addresses: vec![address.to_owned()],
                fingerprint: crow.device.fingerprint().to_owned(),
                kept_by: None,
                version: Some(RUNNING.to_owned()),
                install: None,
            },
        )
        .await
        .unwrap();

        record_member(
            &crow.pool,
            &Linking {
                device: A.to_owned(),
                name: "wolf".to_owned(),
                os: "Linux".to_owned(),
                addresses: vec!["127.0.0.1:1".to_owned()],
                fingerprint: device.fingerprint().to_owned(),
                kept_by: None,
                version: Some(RUNNING.to_owned()),
                install: None,
            },
        )
        .await
        .unwrap();

        let devices = Devices::of(
            device,
            reading,
            Members::recorded(pool.clone()),
            Joins::none(),
        )
        .waiting(PATIENCE);

        let nudges = Nudges::new();
        let workbench =
            router_answering_devices_telling(pool.clone(), devices.clone(), nudges.clone());

        let holding = {
            let devices = devices.clone();
            let nudges = nudges.clone();
            Holding(tokio::spawn(
                async move { devices.stay_fresh(nudges).await },
            ))
        };

        Pressing {
            pool,
            devices,
            nudges,
            workbench,
            _holding: holding,
            _dir: dir,
        }
    }

    /// Another workbench of the same device — a phone beside the laptop that
    /// pressed.
    fn second_workbench(&self) -> Router {
        router_answering_devices_telling(
            self.pool.clone(),
            self.devices.clone(),
            self.nudges.clone(),
        )
    }

    /// Update on crow's row, pressed.
    async fn presses_update(&self) -> LinkedDevice {
        let answered = self
            .workbench
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/ui/devices/members/{CROW}/update"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let status = answered.status();
        let bytes = answered.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(
            status,
            StatusCode::OK,
            "{}",
            String::from_utf8_lossy(&bytes)
        );

        crow_in(&serde_json::from_slice(&bytes).unwrap())
    }
}

/// The streams the pressing device holds, let go of when the test drops it.
struct Holding(JoinHandle<()>);

impl Drop for Holding {
    fn drop(&mut self) {
        self.0.abort();
    }
}

fn no_tailscale() -> Tailscale {
    Tailscale::running(vec!["verkstead-no-such-tailscale".to_owned()], 8422)
}

/// crow's row, as a workbench reads it.
async fn crow_on(workbench: &Router) -> LinkedDevice {
    let answered = workbench
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/ui/devices")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(answered.status(), StatusCode::OK);

    let bytes = answered.into_body().collect().await.unwrap().to_bytes();
    crow_in(&serde_json::from_slice(&bytes).unwrap())
}

fn crow_in(view: &DevicesView) -> LinkedDevice {
    view.members
        .iter()
        .find(|member| member.identity.device == CROW)
        .expect("crow is drawn")
        .clone()
}

/// crow's row, once it reads as `wanted` says — or the test fails saying what
/// it read last.
async fn crow_comes_to(workbench: &Router, wanted: impl Fn(&LinkedDevice) -> bool) -> LinkedDevice {
    let deadline = tokio::time::Instant::now() + WAITING;

    loop {
        let row = crow_on(workbench).await;

        if wanted(&row) {
            return row;
        }

        assert!(
            tokio::time::Instant::now() < deadline,
            "crow's row never came to read as wanted: {row:?}"
        );

        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

/// Pressed for crow with two sessions running there, its row names both, and
/// crow is told: its app is asked, and the row reads *updating…* and then the
/// percent crow reports.
#[tokio::test]
async fn a_press_tells_the_member_and_its_row_follows_the_update() {
    let crow = Crow::installed_as(InstallKind::AppImage).await;
    let pressing = Pressing::linked_to(&crow).await;

    let row = crow_comes_to(&pressing.workbench, |row| row.running.len() == 2).await;
    // In the sidebar's order, which is crow's ranks: newest first.
    let mut running = row.running.clone();
    running.sort();
    assert_eq!(running, RUNNING_THERE.map(str::to_owned).to_vec());
    assert_eq!(row.running_drafts, 0);
    assert_eq!(row.updating, None);

    let pressed = pressing.presses_update().await;
    assert_eq!(pressed.updating, Some(ThisUpdating { percent: None }));
    assert_eq!(pressed.trouble, None);

    assert_eq!(crow.asking.lines(), vec![ASKED.to_owned()]);

    crow.reports_progress(40).await;

    crow_comes_to(&pressing.workbench, |row| {
        row.updating == Some(ThisUpdating { percent: Some(40) })
    })
    .await;

    // A second press while it goes reads the one going, and crow's app is
    // asked nothing more.
    let again = pressing.presses_update().await;
    assert_eq!(again.updating, Some(ThisUpdating { percent: Some(40) }));
    assert_eq!(crow.asking.lines().len(), 1);
}

/// A download that fails on crow is crow's trouble line on the pressing
/// device's row, read on a second workbench of it too, and the next press
/// takes it away.
#[tokio::test]
async fn a_failure_on_the_member_is_its_rows_trouble_line() {
    let crow = Crow::installed_as(InstallKind::AppImage).await;
    let pressing = Pressing::linked_to(&crow).await;

    pressing.presses_update().await;
    crow.reports_failure("sha512 checksum mismatch, expected abc, got def\n    at stack")
        .await;

    const LINE: &str = "crow couldn't be updated: sha512 checksum mismatch, expected abc, got def.";

    let row = crow_comes_to(&pressing.workbench, |row| row.trouble.is_some()).await;
    assert_eq!(row.trouble.as_deref(), Some(LINE));
    assert_eq!(row.updating, None);

    let phone = crow_on(&pressing.second_workbench()).await;
    assert_eq!(phone.trouble.as_deref(), Some(LINE));
    assert_eq!(phone.updating, None);

    let pressed = pressing.presses_update().await;
    assert_eq!(pressed.trouble, None);
    assert_eq!(pressed.updating, Some(ThisUpdating { percent: None }));
}

/// A member unreachable at the press is not told, and its row says it was
/// unreachable and so wasn't updated.
#[tokio::test]
async fn a_member_unreachable_at_the_press_is_not_told() {
    let crow = Crow::installed_as(InstallKind::AppImage).await;

    let nowhere = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = nowhere.local_addr().unwrap().to_string();
    drop(nowhere);

    let pressing = Pressing::linked_at(&crow, &address).await;

    let pressed = pressing.presses_update().await;

    assert_eq!(pressed.updating, None);
    assert_eq!(
        pressed.trouble.as_deref(),
        Some("crow couldn't be reached, so it wasn't updated.")
    );
    assert!(crow.asking.lines().is_empty());
}

/// A member whose install can't update itself refuses by name, and its
/// sentence is the row's line.
#[tokio::test]
async fn a_member_refusing_by_name_is_its_rows_line() {
    let crow = Crow::installed_as(InstallKind::Nix).await;
    let pressing = Pressing::linked_to(&crow).await;

    let pressed = pressing.presses_update().await;

    assert_eq!(pressed.updating, None);
    assert_eq!(
        pressed.trouble.as_deref(),
        Some("crow was installed with nix, so it's updated by hand.")
    );
    assert!(crow.asking.lines().is_empty());
}
