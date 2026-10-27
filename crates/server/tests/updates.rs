//! Learning that a newer Verkstead has been released, and saying so on the
//! viewer's namespace.
//!
//! Against a GitHub stood up in-process rather than against a mock: what is
//! worth proving is that a request goes out at all, that the tag it comes back
//! with is read the way a human would read it, and that the ways GitHub can fail
//! to answer cost a notice rather than the server. A latest-release API only
//! ever tells us a tag or a status code, so a fake that hands over both is the
//! whole of the contract.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{Request, StatusCode};
use axum::routing::get;
use axum::{Json, response::IntoResponse};
use clap::Parser;
use http_body_util::BodyExt;
use tower::ServiceExt;
use verkstead_render::{DevicesView, InstallKind, MemberUpdate, ThisUpdating, UpdateNotice};
use verkstead_server::device::reading::Reading;
use verkstead_server::device::{Device, Devices};
use verkstead_server::nudge::Nudges;
use verkstead_server::onboarding::Machine;
use verkstead_server::peer::joining::Joins;
use verkstead_server::peer::{self, Members};
use verkstead_server::platform::Platform;
use verkstead_server::remote::Tailscale;
use verkstead_server::updating::Asking;
use verkstead_server::{
    Config, Routers, open_database, router_answering_devices_checking_updates,
    router_checking_updates, routers_answering_devices_telling,
};
use verkstead_store::{Linking, record_member};

/// The release this test binary was built as, which is the version the server
/// under test is comparing tags against.
const RUNNING: &str = env!("CARGO_PKG_VERSION");

/// A tag no Verkstead will ever carry, so a test that expects an update is
/// never one release away from expecting nothing.
const FAR_AHEAD: &str = "v99.0.0";

/// How long a test will wait for the startup poll to come back: generous,
/// because it is only ever paid when the assertion is about to fail.
const PATIENCE: Duration = Duration::from_secs(5);

/// How long to keep watching after the poll has been answered, to catch a
/// request that should not have been made at all.
const SETTLING: Duration = Duration::from_millis(300);

/// Every request one of these fake GitHubs has taken.
type Asked = Arc<Mutex<Vec<String>>>;

/// A GitHub on a loopback port, and what it has been asked for.
///
/// The path says what it should answer, which is how one of them stands in for
/// every case: [`GitHub::releasing`] is a repository whose latest release is
/// tagged so, and [`GitHub::nothing_released`] is one that has never released
/// anything — which is what `releases/latest` 404s for.
struct GitHub {
    address: String,
    asked: Asked,
}

impl GitHub {
    /// Where to ask about a repository whose latest release is tagged `tag`.
    fn releasing(&self, tag: &str) -> String {
        format!("{}/releases/{tag}", self.address)
    }

    /// Where to ask about one with no release to be latest.
    fn nothing_released(&self) -> String {
        format!("{}/nothing", self.address)
    }

    fn times_asked(&self) -> usize {
        self.asked.lock().unwrap().len()
    }

    /// Wait until it has been asked at all. Nothing sends that request but the
    /// server starting, so this is what says the poll happens at startup rather
    /// than a day into the run.
    async fn is_asked(&self) {
        let deadline = tokio::time::Instant::now() + PATIENCE;

        while self.times_asked() == 0 {
            assert!(
                tokio::time::Instant::now() < deadline,
                "the server never asked about the latest release",
            );

            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }
}

async fn github() -> GitHub {
    let asked: Asked = Arc::new(Mutex::new(Vec::new()));

    let app = Router::new()
        .route("/releases/{tag}", get(latest_release))
        .route("/nothing", get(no_release))
        .with_state(asked.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    GitHub {
        address: format!("http://{address}"),
        asked,
    }
}

/// The one field of a release that is any of the server's business, in the shape
/// GitHub sends it.
async fn latest_release(State(asked): State<Asked>, Path(tag): Path<String>) -> impl IntoResponse {
    asked.lock().unwrap().push(tag.clone());

    Json(serde_json::json!({ "tag_name": tag }))
}

/// How GitHub answers for a repository with no release yet.
async fn no_release(State(asked): State<Asked>) -> impl IntoResponse {
    asked.lock().unwrap().push("nothing".to_owned());

    (
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({ "message": "Not Found" })),
    )
}

/// An address nothing is listening on: bound to claim a free port, then dropped.
async fn nowhere() -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    drop(listener);
    format!("http://{address}")
}

/// A server over a fresh database, checking for updates wherever it is pointed —
/// or not checking at all, where it is pointed nowhere.
async fn fresh_app(releases: Option<&str>) -> (tempfile::TempDir, Router) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    (dir, router_checking_updates(pool, releases))
}

/// What the viewer would be told, asked for the way the viewer asks.
async fn notice(app: &Router) -> UpdateNotice {
    let http = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/ui/update")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let status = http.status();
    let body = http.into_body().collect().await.unwrap().to_bytes();
    let body = String::from_utf8(body.to_vec()).unwrap();

    assert_eq!(status, StatusCode::OK, "GET /api/ui/update failed: {body}");

    serde_json::from_str(&body).unwrap_or_else(|err| panic!("reading {body:?}: {err}"))
}

/// The same, once the poll it depends on has been answered and anything behind
/// that answer has had time to land.
async fn settled_notice(github: &GitHub, app: &Router) -> UpdateNotice {
    github.is_asked().await;
    settle().await;

    notice(app).await
}

/// Give anything still on its way time to arrive, for an assertion about a
/// request that should never be sent.
async fn settle() {
    tokio::time::sleep(SETTLING).await;
}

/// Whether the server is still answering at all, for the cases where the update
/// check has just failed.
async fn healthy(app: &Router) -> bool {
    let http = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    http.status() == StatusCode::OK
}

#[tokio::test]
async fn a_newer_release_is_named_to_the_viewer() {
    let github = github().await;
    let (_dir, app) = fresh_app(Some(&github.releasing(FAR_AHEAD))).await;

    assert_eq!(
        settled_notice(&github, &app).await,
        UpdateNotice::Available {
            version: "99.0.0".to_owned(),
        },
        "the banner has to be able to name the version that is waiting",
    );
}

#[tokio::test]
async fn the_release_this_is_and_an_older_one_are_nothing_to_update_to() {
    for tag in [format!("v{RUNNING}"), "v0.0.1".to_owned()] {
        let github = github().await;
        let (_dir, app) = fresh_app(Some(&github.releasing(&tag))).await;

        assert_eq!(
            settled_notice(&github, &app).await,
            UpdateNotice::Current,
            "the latest release being {tag} is nothing to tell the human about",
        );
    }
}

#[tokio::test]
async fn a_latest_release_that_cannot_be_read_is_no_news() {
    // No release at all, which is what `releases/latest` 404s for — and a tag
    // that is not a version, which is as much use as no tag. One GitHub each,
    // so each case is genuinely waited on rather than riding the last one's
    // request.
    for release in ["nothing", "the-first-one"] {
        let github = github().await;
        let releases = match release {
            "nothing" => github.nothing_released(),
            tag => github.releasing(tag),
        };

        let (_dir, app) = fresh_app(Some(&releases)).await;

        assert_eq!(settled_notice(&github, &app).await, UpdateNotice::Current);
        assert!(healthy(&app).await, "and the server is still serving");
    }
}

#[tokio::test]
async fn a_github_that_cannot_be_reached_is_no_news_either() {
    let unreachable = nowhere().await;
    let (_dir, app) = fresh_app(Some(&unreachable)).await;

    settle().await;

    assert_eq!(notice(&app).await, UpdateNotice::Current);
    assert!(
        healthy(&app).await,
        "a GitHub that is not there costs a notice, not the server",
    );
}

#[tokio::test]
async fn the_poll_happens_at_startup_rather_than_a_day_later() {
    let github = github().await;
    let (_dir, _app) = fresh_app(Some(&github.releasing(FAR_AHEAD))).await;

    // Nothing but starting has happened, and the daily cycle is a day away.
    github.is_asked().await;
}

#[tokio::test]
async fn a_server_told_not_to_check_asks_nobody() {
    let github = github().await;
    let releases = github.releasing(FAR_AHEAD);

    // The control: pointed at this GitHub, a server asks it and comes back with
    // the verdict — so the count below moving is a real thing to watch.
    let (_checking_dir, checking) = fresh_app(Some(&releases)).await;
    assert!(matches!(
        settled_notice(&github, &checking).await,
        UpdateNotice::Available { .. },
    ));
    assert_eq!(github.times_asked(), 1);

    // Turned off, there is nowhere to ask — with a GitHub standing right there.
    let off = Config::parse_from(["verkstead serve", "--no-update-check"]);
    assert_eq!(off.releases(), None, "turned off leaves nowhere to ask");

    let (_quiet_dir, quiet) = fresh_app(off.releases()).await;
    settle().await;

    assert_eq!(
        github.times_asked(),
        1,
        "a server told not to check makes no request at all",
    );
    assert_eq!(
        notice(&quiet).await,
        UpdateNotice::Current,
        "and the endpoint still answers, saying there is nothing to update to",
    );
}

/// The id this device is stated as, and the ones its members are.
const THIS_DEVICE: &str = "aa00bb11cc22dd33ee44ff5566778899";
const BEHIND: &str = "0011223344556677889900aabbccddee";
const ON_THE_RELEASE: &str = "ffeeddccbbaa00998877665544332211";
const SAYING_NONE: &str = "99887766554433221100aabbccddeeff";

/// A device on 0.2.2 linked to one member a release behind it, one on the
/// latest release, and one from before a member said its version — asking
/// about newer releases at `releases`.
async fn cluster_app(releases: &str) -> (tempfile::TempDir, Router) {
    cluster_app_installed_as(releases, InstallKind::Binary).await
}

/// The same cluster, this device installed as `install`.
async fn cluster_app_installed_as(
    releases: &str,
    install: InstallKind,
) -> (tempfile::TempDir, Router) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    for (device, version) in [
        (BEHIND, Some("0.2.0")),
        (ON_THE_RELEASE, Some("0.3.0")),
        (SAYING_NONE, None),
    ] {
        record_member(
            &pool,
            &Linking {
                device: device.to_owned(),
                name: device[..4].to_owned(),
                os: "Linux".to_owned(),
                addresses: Vec::new(),
                fingerprint: format!("AA:BB:{device}"),
                kept_by: None,
                version: version.map(str::to_owned),
                install: None,
            },
        )
        .await
        .unwrap();
    }

    let reading = Reading::stated(
        Tailscale::running(vec!["verkstead-no-such-tailscale".to_owned()], 8422),
        Platform::HERE,
        None,
        Vec::new(),
    )
    .running(Some("0.2.2"));

    let devices = Devices::of(
        Device::stated(dir.path(), THIS_DEVICE).unwrap(),
        reading,
        Members::recorded(pool.clone()),
        Joins::recorded(pool.clone()),
    )
    .installed_as(install);

    (
        dir,
        router_answering_devices_checking_updates(pool, devices, releases),
    )
}

/// The Devices section as the pane reads it.
async fn devices(app: &Router) -> DevicesView {
    let http = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/ui/devices")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(http.status(), StatusCode::OK);

    let body = http.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&body).unwrap()
}

/// Which rows read *update available*, this device's as `this`.
fn tagged(view: &DevicesView) -> Vec<String> {
    let mut tagged: Vec<String> = view
        .members
        .iter()
        .filter(|member| member.update_available)
        .map(|member| member.identity.device.clone())
        .collect();

    if view.this_update_available.is_some() {
        tagged.push("this".to_owned());
    }

    tagged.sort();
    tagged
}

#[tokio::test]
async fn every_row_behind_the_latest_release_is_tagged() {
    let github = github().await;
    let (_dir, app) = cluster_app(&github.releasing("v0.3.0")).await;

    github.is_asked().await;
    settle().await;

    assert_eq!(
        tagged(&devices(&app).await),
        vec![BEHIND.to_owned(), "this".to_owned()],
        "a row on the release, or saying none, is not tagged",
    );
}

/// A NixOS module install behind the release is tagged like any other row, and
/// says it is nix's, which is what draws the by-hand line and no Update.
#[tokio::test]
async fn a_nix_install_behind_the_release_says_it_is_nixs() {
    let github = github().await;
    let (_dir, app) = cluster_app_installed_as(&github.releasing("v0.3.0"), InstallKind::Nix).await;

    github.is_asked().await;
    settle().await;

    let view = devices(&app).await;

    assert_eq!(
        view.this_update_available.as_deref(),
        Some("0.3.0"),
        "this device is behind the release, and the row names it"
    );
    assert_eq!(view.this_install, InstallKind::Nix);
    assert!(!view.this_install.updates_itself());
}

/// A press on this device's own Update, answered with the section read again.
async fn press_update(app: &Router) -> (StatusCode, serde_json::Value) {
    let http = app
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

    let status = http.status();
    let body = http.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&body).unwrap())
}

/// The desktop app's report of how much is down.
async fn report_progress(app: &Router, percent: u8) -> StatusCode {
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/ui/devices/this/update/progress")
                .header("content-type", "application/json")
                .body(Body::from(format!(r#"{{"percent":{percent}}}"#)))
                .unwrap(),
        )
        .await
        .unwrap()
        .status()
}

/// An AppImage's Update reads *updating…* from the press, with the app's
/// reports of how much is down on it — and a second press reads the first.
#[tokio::test]
async fn an_appimage_update_reads_updating_from_the_press() {
    let github = github().await;
    let (_dir, app) =
        cluster_app_installed_as(&github.releasing("v0.3.0"), InstallKind::AppImage).await;

    assert_eq!(devices(&app).await.this_updating, None);

    let (status, pressed) = press_update(&app).await;
    assert_eq!(status, StatusCode::OK);
    let pressed: DevicesView = serde_json::from_value(pressed).unwrap();
    assert_eq!(pressed.this_updating, Some(ThisUpdating { percent: None }));

    assert_eq!(report_progress(&app, 42).await, StatusCode::NO_CONTENT);
    assert_eq!(
        devices(&app).await.this_updating,
        Some(ThisUpdating { percent: Some(42) })
    );

    let (status, again) = press_update(&app).await;
    assert_eq!(status, StatusCode::OK);
    let again: DevicesView = serde_json::from_value(again).unwrap();
    assert_eq!(
        again.this_updating,
        Some(ThisUpdating { percent: Some(42) }),
        "a second press reads the Update already going"
    );
}

/// A server the desktop app didn't start, nix's and a checkout's are each
/// refused by what they are, and nothing is marked updating.
#[tokio::test]
async fn an_install_that_cannot_update_itself_is_refused_by_name() {
    let github = github().await;

    for (install, named) in [
        (InstallKind::Binary, "wasn't started by the desktop app"),
        (InstallKind::Nix, "installed with nix"),
        (InstallKind::Checkout, "built from a checkout"),
    ] {
        let (_dir, app) = cluster_app_installed_as(&github.releasing("v0.3.0"), install).await;

        let (status, refusal) = press_update(&app).await;
        assert_eq!(status, StatusCode::CONFLICT, "{install:?}");
        assert!(
            refusal["error"].as_str().unwrap().contains(named),
            "{install:?}: {refusal}"
        );
        assert_eq!(devices(&app).await.this_updating, None, "{install:?}");
    }
}

/// The desktop app's report that the Update failed.
async fn report_failure(app: &Router, why: &str) -> StatusCode {
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/ui/devices/this/update/failed")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::json!({ "why": why }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap()
        .status()
}

/// A failed download leaves *updating…* for the row's trouble line, carrying
/// the updater's reason, and the row is offered Update again; the next press
/// takes the line away.
#[tokio::test]
async fn a_failed_update_is_the_rows_trouble_line() {
    let github = github().await;
    let (_dir, app) =
        cluster_app_installed_as(&github.releasing("v0.3.0"), InstallKind::AppImage).await;

    press_update(&app).await;
    report_progress(&app, 60).await;
    assert_eq!(
        report_failure(&app, "sha512 checksum mismatch, expected abc, got def").await,
        StatusCode::NO_CONTENT
    );

    let view = devices(&app).await;
    assert_eq!(view.this_updating, None);
    assert_eq!(
        view.this_update_trouble.as_deref(),
        Some("This device couldn't be updated: sha512 checksum mismatch, expected abc, got def.")
    );

    let (_, pressed) = press_update(&app).await;
    let pressed: DevicesView = serde_json::from_value(pressed).unwrap();
    assert_eq!(pressed.this_update_trouble, None);
}

/// A relaunch on another version than the one handed over to reports to a
/// server with no Update going, which draws the line all the same.
#[tokio::test]
async fn a_relaunch_on_the_wrong_version_is_the_rows_trouble_line() {
    let github = github().await;
    let (_dir, app) =
        cluster_app_installed_as(&github.releasing("v0.3.0"), InstallKind::AppImage).await;

    report_failure(
        &app,
        "Verkstead 0.3.0 was installed, but this device started on 0.2.0.",
    )
    .await;

    let view = devices(&app).await;
    assert_eq!(view.this_updating, None);
    assert_eq!(
        view.this_update_trouble.as_deref(),
        Some(
            "This device couldn't be updated: Verkstead 0.3.0 was installed, but this device \
             started on 0.2.0."
        )
    );
}

/// A report with no Update going is dropped rather than drawn.
#[tokio::test]
async fn progress_with_no_update_going_is_dropped() {
    let github = github().await;
    let (_dir, app) =
        cluster_app_installed_as(&github.releasing("v0.3.0"), InstallKind::AppImage).await;

    assert_eq!(report_progress(&app, 10).await, StatusCode::NO_CONTENT);
    assert_eq!(devices(&app).await.this_updating, None);
}

/// A member can be behind a device that is itself up to date, which is what
/// keeping the release rather than the verdict is for.
#[tokio::test]
async fn a_member_behind_an_up_to_date_device_is_tagged() {
    let github = github().await;
    let (_dir, app) = cluster_app(&github.releasing("v0.2.2")).await;

    github.is_asked().await;
    settle().await;

    assert_eq!(tagged(&devices(&app).await), vec![BEHIND.to_owned()]);
}

#[tokio::test]
async fn with_no_answer_no_row_is_tagged() {
    let unreachable = nowhere().await;
    let (_dir, app) = cluster_app(&unreachable).await;

    settle().await;

    assert!(tagged(&devices(&app).await).is_empty());
}

/// A member called crow, installed as `install`, as the device pressing its
/// row reaches it: the router it serves over the link, and what its desktop
/// app hears.
async fn member_installed_as(install: InstallKind) -> (tempfile::TempDir, Router, Asking) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    let reading = Reading::stated(
        Tailscale::running(vec!["verkstead-no-such-tailscale".to_owned()], 8422),
        Platform::HERE,
        None,
        Vec::new(),
    )
    .calling_itself("crow", "Linux")
    .running(Some("0.2.2"));

    let asking = Asking::heard();

    let devices = Devices::of(
        Device::stated(dir.path(), BEHIND).unwrap(),
        reading,
        Members::recorded(pool.clone()),
        Joins::recorded(pool.clone()),
    )
    .installed_as(install)
    .asking_the_app(asking.clone());

    let Routers { over_the_link, .. } =
        routers_answering_devices_telling(pool, devices, Nudges::new(), Machine::here());

    (dir, over_the_link, asking)
}

/// Tell a member to update itself, the way a press on its row does.
async fn tell_to_update(member: &Router) -> (StatusCode, String) {
    let http = member
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(peer::updating::UPDATE)
                .header("content-type", "application/json")
                .body(Body::from(r#"{"release":"0.3.0"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = http.status();
    let body = http.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8(body.to_vec()).unwrap())
}

/// Ask a member how its Update stands.
async fn member_update(member: &Router) -> MemberUpdate {
    let http = member
        .clone()
        .oneshot(
            Request::builder()
                .uri(peer::updating::UPDATE)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(http.status(), StatusCode::OK);

    let body = http.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&body).unwrap()
}

/// A desktop member told to update itself says the line its app acts on, once
/// however often it is told, and reads *updating…* when asked.
#[tokio::test]
async fn a_desktop_member_told_to_update_asks_its_app() {
    let (_dir, member, asking) = member_installed_as(InstallKind::AppImage).await;

    assert_eq!(member_update(&member).await, MemberUpdate::default());

    let (status, answered) = tell_to_update(&member).await;
    assert_eq!(status, StatusCode::OK, "{answered}");
    let answered: MemberUpdate = serde_json::from_str(&answered).unwrap();
    assert_eq!(answered.updating, Some(ThisUpdating { percent: None }));

    assert_eq!(
        asking.lines(),
        vec!["verkstead-desktop: update this device".to_owned()]
    );

    let (status, _) = tell_to_update(&member).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(asking.lines().len(), 1, "a second telling reads the first");

    assert_eq!(
        member_update(&member).await,
        MemberUpdate {
            updating: Some(ThisUpdating { percent: None }),
            trouble: None,
        }
    );
}

/// A member whose install can't update itself refuses by name, and its app
/// is asked nothing.
#[tokio::test]
async fn a_member_that_cannot_update_itself_refuses_by_name() {
    for (install, said) in [
        (
            InstallKind::Nix,
            "crow was installed with nix, so it's updated by hand.\n",
        ),
        (
            InstallKind::Checkout,
            "crow was built from a checkout, so it's updated by hand.\n",
        ),
        (
            InstallKind::Binary,
            "crow wasn't started by the desktop app, so it can't update itself.\n",
        ),
    ] {
        let (_dir, member, asking) = member_installed_as(install).await;

        assert_eq!(
            tell_to_update(&member).await,
            (StatusCode::CONFLICT, said.to_owned()),
            "{install:?}"
        );
        assert!(asking.lines().is_empty(), "{install:?}");
        assert_eq!(
            member_update(&member).await,
            MemberUpdate::default(),
            "{install:?}"
        );
    }
}
