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
//! **A restart is the member's link going down and a new server of it coming
//! up behind it**, on whatever release the test says, over the same store and
//! the same certificate: the Nudge stream the pressing device holds ends, and
//! is taken up again on a server that has never heard of the press — which is
//! what a desktop app quitting to install and relaunching is to this device.
//!
//! **The sessions running on the member** are its own Conversations with
//! `working` said of them on the way out — a layer over its listener, because
//! a session running is a process, and this suite is about the card naming
//! them rather than about running one.

use std::net::SocketAddr;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use http_body_util::BodyExt;
use serde_json::json;
use sqlx::SqlitePool;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
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
    Routers, open_database, router_answering_devices_telling,
    router_answering_devices_telling_checking_updates, routers_answering_devices_telling, store,
};
use verkstead_store::{Linking, record_member};

/// The device the human presses on.
const A: &str = "aa00bb11cc22dd33ee44ff5566778899";

/// And crow, the member it updates.
const CROW: &str = "0011223344556677889900aabbccddee";

/// And raven, a second member, for Update all.
const RAVEN: &str = "ffeeddccbbaa99887766554433221100";

/// The branches crow's two sessions are running on.
const RUNNING_THERE: [&str; 2] = ["rate-limiting", "solid-viewer"];

/// How long one address has to answer here.
const PATIENCE: Duration = Duration::from_millis(300);

/// How long a test waits for a row to come to read what it expects. Generous,
/// because it is only ever paid when the assertion is about to fail — and a
/// stream that ended is taken up again five seconds later at the soonest.
const WAITING: Duration = Duration::from_secs(30);

/// The latest stable release, as the pressing device's Update Notice learns
/// it: what crow is told of and is to come back on.
const RELEASE: &str = "0.3.0";

/// And how long crow has to come back where a test isn't about the deadline:
/// longer than any restart here takes.
const COMING_BACK: Duration = Duration::from_secs(120);

/// The line crow's desktop app acts on.
const ASKED: &str = "verkstead-desktop: update this device";

/// crow: its own workbench, which its desktop app reports to, what that app has
/// been asked, and the link the pressing device reaches it over.
struct Crow {
    name: &'static str,
    device: Device,
    link: Link,
    pool: SqlitePool,
    install: InstallKind,
    workbench: Router,
    asking: Asking,
    _dir: tempfile::TempDir,
}

impl Crow {
    /// crow installed as `install`, with two sessions running on it, its Peer
    /// Listener up behind a link.
    async fn installed_as(install: InstallKind) -> Crow {
        Crow::named("crow", CROW, install).await
    }

    /// The same member called `name`, its Device Id `id` — another of them.
    async fn named(name: &'static str, id: &str, install: InstallKind) -> Crow {
        let dir = tempfile::tempdir().unwrap();
        let pool = open_database(&dir.path().join("verkstead.db"))
            .await
            .unwrap();

        let device = Device::stated(dir.path(), id).unwrap();
        let link = Link::opened().await;

        let (workbench, asking) = serving(name, &device, &pool, &link, install, None);

        let repo = store::register_repo(&pool, Path::new("/srv/verkstead"), "verkstead", "main")
            .await
            .unwrap()
            .unwrap();

        for branch in RUNNING_THERE {
            store::start_conversation(&pool, repo.id, branch, id)
                .await
                .unwrap()
                .unwrap();
        }

        Crow {
            name,
            device,
            link,
            pool,
            install,
            workbench,
            asking,
            _dir: dir,
        }
    }

    /// crow's app quitting to install: its link goes down, every connection
    /// over it with it, and nothing answers until it comes back.
    fn goes_quiet(&self) {
        self.link.down();
    }

    /// And crow started again on `version`, a server that has heard of no
    /// press, its link up again in front of it.
    fn comes_back_on(&mut self, version: &str) {
        self.starts_on(version);
        self.link.up();
    }

    /// The new server alone, not yet reachable — for what its app says to it
    /// before anybody asks.
    fn starts_on(&mut self, version: &str) {
        let (workbench, asking) = serving(
            self.name,
            &self.device,
            &self.pool,
            &self.link,
            self.install,
            Some(version),
        );

        self.workbench = workbench;
        self.asking = asking;
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

/// A crow server over `device` and `pool`, installed as `install` and saying
/// `version` (this build's where `None`), its Peer Listener up and `link`
/// pointed at it: its workbench, and what its app is asked.
fn serving(
    name: &str,
    device: &Device,
    pool: &SqlitePool,
    link: &Link,
    install: InstallKind,
    version: Option<&str>,
) -> (Router, Asking) {
    let members = Members::recorded(pool.clone());
    let nudges = Nudges::new();

    let listener = peer::Listener::bound("127.0.0.1:0".parse().unwrap(), device)
        .expect("the loopback on a port the machine picked is free");

    // What crow says of where it is, which a take-up writes onto its row on
    // the pressing device, is the link: a restart on another port behind it is
    // still crow at the address it was linked at.
    let reading = Reading::advertising(no_tailscale(), Platform::Linux, None, vec![link.at()])
        .calling_itself(name, "Linux")
        .installed_as(Some(install));

    if let Some(version) = version {
        reading.now_running(Some(version));
    }

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
    } = routers_answering_devices_telling(pool.clone(), devices, nudges.clone(), Machine::here());

    let serving = peer::router(
        device.clone(),
        reading,
        members,
        Joins::none(),
        nudges,
        over_the_link,
    )
    .layer(middleware::from_fn(sessions_running));

    link.through_to(listener.address());
    tokio::spawn(listener.serving(serving));

    (workbench, asking)
}

/// A link to crow as something that can **go down**: a socket on the loopback
/// carrying bytes to whichever server of crow is up and back, which lets go of
/// every connection it holds and turns new ones away while it is down.
///
/// **Which is what a restart is from the far side of it** — the freshness
/// suite's cut link, kept down for as long as a restart takes. The bytes are
/// crow's own TLS, untouched, so the fingerprint the pressing device pins is
/// still crow's.
struct Link {
    address: SocketAddr,
    far: Arc<Mutex<Far>>,
    cutting: broadcast::Sender<()>,
}

/// Where the link goes, and whether it is up.
#[derive(Clone, Copy)]
struct Far {
    to: Option<SocketAddr>,
    up: bool,
}

impl Link {
    /// A link going nowhere yet, up.
    async fn opened() -> Link {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let far = Arc::new(Mutex::new(Far { to: None, up: true }));
        let (cutting, _) = broadcast::channel(1);

        let going = far.clone();
        let cut = cutting.clone();

        tokio::spawn(async move {
            while let Ok((mut near, _)) = listener.accept().await {
                let Far {
                    to: Some(to),
                    up: true,
                } = *going.lock().unwrap()
                else {
                    continue;
                };

                let mut cut = cut.subscribe();

                tokio::spawn(async move {
                    let Ok(mut across) = TcpStream::connect(to).await else {
                        return;
                    };

                    tokio::select! {
                        _ = tokio::io::copy_bidirectional(&mut near, &mut across) => {}
                        _ = cut.recv() => {}
                    }
                });
            }
        });

        Link {
            address,
            far,
            cutting,
        }
    }

    fn at(&self) -> String {
        format!("127.0.0.1:{}", self.address.port())
    }

    /// Carry what arrives from here on to `to`.
    fn through_to(&self, to: SocketAddr) {
        self.far.lock().unwrap().to = Some(to);
    }

    fn down(&self) {
        self.far.lock().unwrap().up = false;
        let _ = self.cutting.send(());
    }

    fn up(&self) {
        self.far.lock().unwrap().up = true;
    }
}

/// A GitHub whose latest release is [`RELEASE`], on a loopback port: where the
/// pressing device's Update Notice asks.
async fn github() -> String {
    let app = Router::new().route(
        "/latest",
        get(|| async { axum::Json(json!({ "tag_name": format!("v{RELEASE}") })) }),
    );

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    format!("http://{address}/latest")
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
    /// What its own desktop app is asked, which only Update all asks here.
    asking: Asking,
    nudges: Nudges,
    workbench: Router,
    _holding: Holding,
    _dir: tempfile::TempDir,
}

impl Pressing {
    /// The pressing device, linked to crow at the address crow listens on.
    async fn linked_to(crow: &Crow) -> Pressing {
        Pressing::linked_at(crow, &crow.link.at(), COMING_BACK).await
    }

    /// The same, crow having `within` of a press to come back.
    async fn linked_within(crow: &Crow, within: Duration) -> Pressing {
        Pressing::linked_at(crow, &crow.link.at(), within).await
    }

    /// The same, its row saying crow is at `address`.
    async fn linked_at(crow: &Crow, address: &str, within: Duration) -> Pressing {
        Pressing::linked(&[(crow, address)], within).await
    }

    /// The pressing device linked to each member at the address beside it,
    /// each having `within` of a press to come back. Installed as an AppImage,
    /// so that Update all updates it last.
    async fn linked(members: &[(&Crow, &str)], within: Duration) -> Pressing {
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

        for (crow, address) in members {
            // Both ways, a link being both ways: this device dials crow pinned
            // on its fingerprint, and crow admits it on its own. Recorded as
            // crow says of itself, so a take-up doesn't read the row as moved.
            record_member(
                &pool,
                &Linking {
                    device: crow.device.id().to_owned(),
                    name: crow.name.to_owned(),
                    os: "Linux".to_owned(),
                    addresses: vec![(*address).to_owned()],
                    fingerprint: crow.device.fingerprint().to_owned(),
                    kept_by: None,
                    version: Some(RUNNING.to_owned()),
                    install: serde_json::to_value(crow.install)
                        .unwrap()
                        .as_str()
                        .map(str::to_owned),
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
        }

        let asking = Asking::heard();

        let devices = Devices::of(
            device,
            reading,
            Members::recorded(pool.clone()),
            Joins::none(),
        )
        .waiting(PATIENCE)
        .coming_back_within(within)
        .installed_as(InstallKind::AppImage)
        .asking_the_app(asking.clone());

        let nudges = Nudges::new();
        let workbench = router_answering_devices_telling_checking_updates(
            pool.clone(),
            devices.clone(),
            nudges.clone(),
            &github().await,
        );

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
            asking,
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

    /// Update all on the Update Notice's banner, pressed: the Devices section
    /// it answers with.
    async fn presses_update_all(&self) -> DevicesView {
        let answered = self
            .workbench
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/ui/devices/all/update")
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

        serde_json::from_slice(&bytes).unwrap()
    }
}

/// A device's notice stream, held open the way its desktop app holds it: every
/// notification it pushes to its phones, read here instead.
struct Notices {
    body: Body,
    buffered: String,
}

/// How long to go on listening for a notice that should not come.
const SETTLING: Duration = Duration::from_millis(500);

impl Notices {
    /// Open on `workbench`; listening by the time it returns.
    async fn held(workbench: &Router) -> Notices {
        let answered = workbench
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/ui/push/notices")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(answered.status(), StatusCode::OK);

        Notices {
            body: answered.into_body(),
            buffered: String::new(),
        }
    }

    /// The next notice, or `None` where none comes within `waiting`.
    async fn within(&mut self, waiting: Duration) -> Option<serde_json::Value> {
        tokio::time::timeout(waiting, async {
            loop {
                if let Some(end) = self.buffered.find("\n\n") {
                    let frame: String = self.buffered.drain(..end + 2).collect();

                    if frame.lines().any(|line| line == "event: notice") {
                        let data = frame
                            .lines()
                            .find_map(|line| line.strip_prefix("data: "))
                            .expect("a notice carries data");

                        return serde_json::from_str(data).unwrap();
                    }

                    continue;
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
        })
        .await
        .ok()
    }

    /// The one notice there is to be, as its title, landing on the Remote
    /// access pane — and nothing after it.
    async fn one(&mut self) -> String {
        let notice = self.within(WAITING).await.expect("no notification came");

        assert_eq!(notice["path"], "/settings/remote");
        self.none().await;

        notice["title"].as_str().unwrap().to_owned()
    }

    /// And none at all.
    async fn none(&mut self) {
        if let Some(notice) = self.within(SETTLING).await {
            panic!("a notification that should not have been raised: {notice}");
        }
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
    crow_in(&devices_on(workbench).await)
}

/// The Devices section, as a workbench reads it.
async fn devices_on(workbench: &Router) -> DevicesView {
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
    serde_json::from_slice(&bytes).unwrap()
}

fn crow_in(view: &DevicesView) -> LinkedDevice {
    view.members
        .iter()
        .find(|member| member.identity.device == CROW)
        .expect("crow is drawn")
        .clone()
}

/// The Devices section, once it reads as `wanted` says — or the test fails
/// saying what it read last.
async fn devices_come_to(workbench: &Router, wanted: impl Fn(&DevicesView) -> bool) -> DevicesView {
    let deadline = tokio::time::Instant::now() + WAITING;

    loop {
        let view = devices_on(workbench).await;

        if wanted(&view) {
            return view;
        }

        assert!(
            tokio::time::Instant::now() < deadline,
            "the devices never came to read as wanted: {view:?}"
        );

        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

/// `id`'s row in `view`.
fn member_in(view: &DevicesView, id: &str) -> LinkedDevice {
    view.members
        .iter()
        .find(|member| member.identity.device == id)
        .expect("the member is drawn")
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
    let mut notices = Notices::held(&pressing.workbench).await;

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

    // Raised once, in the row's own words, however often the row is read.
    assert_eq!(notices.one().await, LINE);

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

    let pressing = Pressing::linked_at(&crow, &address, COMING_BACK).await;
    let mut notices = Notices::held(&pressing.workbench).await;

    let pressed = pressing.presses_update().await;

    assert_eq!(pressed.updating, None);
    assert_eq!(
        pressed.trouble.as_deref(),
        Some("crow couldn't be reached, so it wasn't updated.")
    );
    assert!(crow.asking.lines().is_empty());

    // Nothing was tried on it, and its row says so: no notification.
    notices.none().await;
}

/// A member whose install can't update itself refuses by name, and its
/// sentence is the row's line.
#[tokio::test]
async fn a_member_refusing_by_name_is_its_rows_line() {
    let crow = Crow::installed_as(InstallKind::Nix).await;
    let pressing = Pressing::linked_to(&crow).await;
    let mut notices = Notices::held(&pressing.workbench).await;

    let pressed = pressing.presses_update().await;

    assert_eq!(pressed.updating, None);
    assert_eq!(
        pressed.trouble.as_deref(),
        Some("crow was installed with nix, so it's updated by hand.")
    );
    assert!(crow.asking.lines().is_empty());

    assert_eq!(
        notices.one().await,
        "crow was installed with nix, so it's updated by hand."
    );
}

/// crow, told and going, with the pressing device holding its stream and
/// knowing of [`RELEASE`]: what every restart below starts from.
async fn told_and_going(crow: &Crow, pressing: &Pressing) {
    // Its sessions read means its stream is held, and the release learned is
    // what it is told of.
    crow_comes_to(&pressing.workbench, |row| {
        row.running.len() == 2 && row.update_release.as_deref() == Some(RELEASE)
    })
    .await;

    let pressed = pressing.presses_update().await;
    assert_eq!(pressed.updating, Some(ThisUpdating { percent: None }));
    assert_eq!(crow.asking.lines(), vec![ASKED.to_owned()]);

    crow.reports_progress(100).await;

    crow_comes_to(&pressing.workbench, |row| {
        row.updating == Some(ThisUpdating { percent: Some(100) })
    })
    .await;
}

/// With crow's stream dropped after the press its row reads *restarting…*,
/// and taken up again on the new release the row shows it, with no state word
/// and no line.
#[tokio::test]
async fn a_member_back_on_the_new_release_is_drawn_on_it() {
    let mut crow = Crow::installed_as(InstallKind::AppImage).await;
    let pressing = Pressing::linked_to(&crow).await;
    let mut notices = Notices::held(&pressing.workbench).await;

    told_and_going(&crow, &pressing).await;

    crow.goes_quiet();

    let quiet = crow_comes_to(&pressing.workbench, |row| row.restarting).await;
    assert!(
        quiet.updating.is_some(),
        "restarting is still its Update going"
    );
    assert_eq!(quiet.trouble, None);

    // Read on a second workbench too: the pressing device's server says it.
    assert!(crow_on(&pressing.second_workbench()).await.restarting);

    crow.comes_back_on(RELEASE);

    let back = crow_comes_to(&pressing.workbench, |row| row.updating.is_none()).await;
    assert_eq!(back.identity.version.as_deref(), Some(RELEASE));
    assert!(!back.restarting);
    assert_eq!(back.trouble, None);

    tokio::time::timeout(WAITING, pressing.devices.member_update_settled(CROW))
        .await
        .expect("the Update is over");

    // Success is the row's to say.
    notices.none().await;
}

/// Taken up again on the release it ran, crow's row carries one line naming
/// both releases.
#[tokio::test]
async fn a_member_back_on_the_release_it_ran_is_a_line_naming_both() {
    let mut crow = Crow::installed_as(InstallKind::AppImage).await;
    let pressing = Pressing::linked_to(&crow).await;
    let mut notices = Notices::held(&pressing.workbench).await;

    told_and_going(&crow, &pressing).await;

    crow.goes_quiet();
    crow_comes_to(&pressing.workbench, |row| row.restarting).await;

    crow.comes_back_on(RUNNING);

    let back = crow_comes_to(&pressing.workbench, |row| row.updating.is_none()).await;
    assert!(!back.restarting);

    let line = format!("crow couldn't be updated: it came back on {RUNNING}, not {RELEASE}.");
    assert_eq!(back.trouble.as_deref(), Some(line.as_str()));
    assert_eq!(notices.one().await, line);
}

/// And where crow's own app said so on relaunching, that is the one line,
/// rather than it and this device's beside it.
#[tokio::test]
async fn a_members_own_relaunch_line_is_the_one_line() {
    let mut crow = Crow::installed_as(InstallKind::AppImage).await;
    let pressing = Pressing::linked_to(&crow).await;

    told_and_going(&crow, &pressing).await;

    crow.goes_quiet();
    crow_comes_to(&pressing.workbench, |row| row.restarting).await;

    crow.starts_on(RUNNING);
    crow.reports_failure(&format!(
        "Verkstead {RELEASE} was installed, but this device started on {RUNNING}."
    ))
    .await;
    crow.link.up();

    let back = crow_comes_to(&pressing.workbench, |row| row.updating.is_none()).await;
    assert_eq!(
        back.trouble.as_deref(),
        Some(
            format!(
                "crow couldn't be updated: Verkstead {RELEASE} was installed, but this device \
                 started on {RUNNING}."
            )
            .as_str()
        )
    );
}

/// A member that never comes back gets a line at the deadline — and turning
/// up on the new release after all takes it away.
#[tokio::test]
async fn a_member_that_never_comes_back_is_a_line_at_the_deadline() {
    let mut crow = Crow::installed_as(InstallKind::AppImage).await;
    let pressing = Pressing::linked_within(&crow, Duration::from_secs(3)).await;
    let mut notices = Notices::held(&pressing.workbench).await;

    told_and_going(&crow, &pressing).await;

    crow.goes_quiet();

    tokio::time::timeout(WAITING, pressing.devices.member_update_settled(CROW))
        .await
        .expect("the deadline ends the Update");

    let late = crow_on(&pressing.workbench).await;
    assert_eq!(late.updating, None);
    assert!(!late.restarting);
    assert_eq!(
        late.trouble.as_deref(),
        Some("crow didn't come back after its update.")
    );
    assert_eq!(
        notices.one().await,
        "crow didn't come back after its update."
    );

    crow.comes_back_on(RELEASE);

    let back = crow_comes_to(&pressing.workbench, |row| row.trouble.is_none()).await;
    assert_eq!(back.identity.version.as_deref(), Some(RELEASE));
}

/// Update all with raven asleep: raven's row says it couldn't be reached, crow
/// is told and goes through its restart, and this device is told to update
/// only once crow is back on the new release.
#[tokio::test]
async fn update_all_updates_the_members_then_this_device_and_skips_a_sleeper() {
    let mut crow = Crow::installed_as(InstallKind::AppImage).await;
    let raven = Crow::named("raven", RAVEN, InstallKind::AppImage).await;

    let nowhere = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let asleep = nowhere.local_addr().unwrap().to_string();
    drop(nowhere);

    let pressing = Pressing::linked(
        &[(&crow, crow.link.at().as_str()), (&raven, asleep.as_str())],
        COMING_BACK,
    )
    .await;

    // crow's stream held, and the release learned.
    crow_comes_to(&pressing.workbench, |row| {
        row.running.len() == 2 && row.update_release.as_deref() == Some(RELEASE)
    })
    .await;

    let mut notices = Notices::held(&pressing.workbench).await;

    let pressed = pressing.presses_update_all().await;
    assert!(pressed.updating_all);

    let view = devices_come_to(&pressing.workbench, |view| {
        member_in(view, RAVEN).trouble.is_some() && member_in(view, CROW).updating.is_some()
    })
    .await;
    assert_eq!(
        member_in(&view, RAVEN).trouble.as_deref(),
        Some("raven couldn't be reached, so it wasn't updated.")
    );
    assert!(raven.asking.lines().is_empty());
    assert_eq!(crow.asking.lines(), vec![ASKED.to_owned()]);

    // A second press reads the one going: nobody is told twice.
    assert!(pressing.presses_update_all().await.updating_all);
    assert_eq!(crow.asking.lines().len(), 1);

    crow.reports_progress(100).await;
    crow.goes_quiet();
    crow_comes_to(&pressing.workbench, |row| row.restarting).await;

    assert!(
        pressing.asking.lines().is_empty(),
        "this device waits on crow"
    );
    assert_eq!(devices_on(&pressing.workbench).await.this_updating, None);

    crow.comes_back_on(RELEASE);

    let done = devices_come_to(&pressing.workbench, |view| !view.updating_all).await;
    assert_eq!(pressing.asking.lines(), vec![ASKED.to_owned()]);
    assert_eq!(done.this_updating, Some(ThisUpdating { percent: None }));

    let crow = member_in(&done, CROW);
    assert_eq!(crow.identity.version.as_deref(), Some(RELEASE));
    assert_eq!(crow.trouble, None);

    // The sleeper's row says it; nothing was tried on it, and crow updated.
    notices.none().await;
}

/// This device's own Update failing raises the same sentence, naming it, from
/// its own server — and the app saying it again raises nothing more.
#[tokio::test]
async fn this_devices_own_failed_update_is_one_notification_naming_it() {
    let crow = Crow::installed_as(InstallKind::AppImage).await;
    let pressing = Pressing::linked_to(&crow).await;
    let mut notices = Notices::held(&pressing.workbench).await;

    let pressed = pressing
        .workbench
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/ui/devices/this/update")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(pressed.status(), StatusCode::OK);
    assert_eq!(pressing.asking.lines(), vec![ASKED.to_owned()]);

    for _ in 0..2 {
        let reported = pressing
            .workbench
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/ui/devices/this/update/failed")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        json!({ "why": "net::ERR_INTERNET_DISCONNECTED" }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(reported.status(), StatusCode::NO_CONTENT);
    }

    assert_eq!(
        notices.one().await,
        "wolf couldn't be updated: net::ERR_INTERNET_DISCONNECTED."
    );
}

/// Update all passes over a member updated by hand and an Annex in silence,
/// and updates this device.
#[tokio::test]
async fn update_all_passes_over_nix_members_and_annexes() {
    let crow = Crow::installed_as(InstallKind::Nix).await;
    let pressing = Pressing::linked_to(&crow).await;

    // An Annex of crow's, which follows crow's release.
    record_member(
        &pressing.pool,
        &Linking {
            device: RAVEN.to_owned(),
            name: "crow-annex".to_owned(),
            os: "Linux".to_owned(),
            addresses: vec!["127.0.0.1:1".to_owned()],
            fingerprint: "00".repeat(32),
            kept_by: Some(CROW.to_owned()),
            version: Some(RUNNING.to_owned()),
            install: Some("AppImage".to_owned()),
        },
    )
    .await
    .unwrap();

    crow_comes_to(&pressing.workbench, |row| {
        row.update_release.as_deref() == Some(RELEASE)
    })
    .await;

    pressing.presses_update_all().await;

    let done = devices_come_to(&pressing.workbench, |view| !view.updating_all).await;
    assert_eq!(pressing.asking.lines(), vec![ASKED.to_owned()]);
    assert!(crow.asking.lines().is_empty());

    for member in &done.members {
        assert_eq!(
            member.updating, None,
            "{} is passed over",
            member.identity.name
        );
        assert_eq!(
            member.trouble, None,
            "{} is passed over",
            member.identity.name
        );
    }
}
