//! **Plugins installed from a local directory**, on one device: the consent
//! card read off a path, **Install** with the grant it showed, the row on the
//! Plugins page, the trouble a broken manifest is after **Update**, the
//! checkbox, and the uuids that are refused (ADR-0030).
//!
//! Every press is the one the section makes, on a router kept over a Data
//! Directory of its own; the plugins are directories written here. What the
//! record does across two devices is `replicating.rs`'s.

use std::path::Path;

use axum::Router;
use axum::body::Body;
use axum::http::header::CONTENT_TYPE;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;
use verkstead_render::{
    PluginCapability, PluginConsent, PluginRow, PluginSource, PluginUpdated, PluginsView,
};
use verkstead_server::plugins::built_in::FIXTURE;
use verkstead_server::{open_database, router_keeping};

const PLUGINS: &str = "/api/ui/plugins";

const PLUGIN: &str = "0b6f6b7e-5c3a-4d0e-9a51-2f7d8c1e4a90";
const ANOTHER: &str = "4c1d2e3f-8a9b-4c0d-9e1f-2a3b4c5d6e7f";

/// A plugin's `plugin.yaml`, under `uuid` and called `name`, asking for
/// `capabilities` — the YAML list's items, each one line.
fn manifest(uuid: &str, name: &str, capabilities: &[&str]) -> String {
    let capabilities = if capabilities.is_empty() {
        "capabilities: []\n".to_owned()
    } else {
        format!(
            "capabilities:\n{}",
            capabilities
                .iter()
                .map(|capability| format!("  - {capability}\n"))
                .collect::<String>()
        )
    };

    format!(
        "uuid: {uuid}\n\
         name: {name}\n\
         author:\n  name: Ada Lovelace\n  email: ada@example.com\n\
         version: 1.4.0\n\
         interface: 1\n\
         provides:\n  - category: forge\n  - category: skills\n  - category: forge\n\
         {capabilities}"
    )
}

/// A directory under `dir` holding `text` as its `plugin.yaml`, by its path.
fn plugin(dir: &Path, name: &str, text: &str) -> String {
    let at = dir.join(name);
    std::fs::create_dir_all(&at).unwrap();
    std::fs::write(at.join("plugin.yaml"), text).unwrap();
    at.to_str().unwrap().to_owned()
}

async fn pressing(app: &Router, path: &str, body: serde_json::Value) -> (StatusCode, String) {
    let answered = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(path)
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = answered.status();
    let bytes = answered.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8_lossy(&bytes).into_owned())
}

async fn page(app: &Router) -> PluginsView {
    let answered = app
        .clone()
        .oneshot(Request::builder().uri(PLUGINS).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(answered.status(), StatusCode::OK);
    let bytes = answered.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

/// The consent card for the plugin at `path`.
async fn read(app: &Router, path: &str) -> PluginConsent {
    let (status, said) = pressing(
        app,
        "/api/ui/plugins/read",
        serde_json::json!({ "path": path }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{said}");
    serde_json::from_str(&said).unwrap()
}

/// **Install** on the card read for `path`, granting what it showed.
async fn installs(app: &Router, path: &str) -> PluginsView {
    let consent = read(app, path).await;
    let (status, said) = pressing(
        app,
        PLUGINS,
        serde_json::json!({ "path": path, "granted": consent.capabilities }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{said}");
    serde_json::from_str(&said).unwrap()
}

async fn updates(app: &Router, uuid: &str, granted: Option<&[PluginCapability]>) -> PluginUpdated {
    let (status, said) = pressing(
        app,
        &format!("/api/ui/plugins/{uuid}/update"),
        serde_json::json!({ "granted": granted }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{said}");
    serde_json::from_str(&said).unwrap()
}

fn row<'a>(view: &'a PluginsView, uuid: &str) -> &'a PluginRow {
    view.installed
        .iter()
        .find(|row| row.uuid == uuid)
        .unwrap_or_else(|| panic!("{uuid} is listed in {view:?}"))
}

async fn kept() -> (Router, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();
    (router_keeping(pool, dir.path().to_owned()), dir)
}

/// A local plugin read through the card and installed is listed with its name,
/// version, author, what it provides and where it came from, and loads.
#[tokio::test]
async fn a_local_plugin_installed_through_the_card_is_listed_on_its_row() {
    let (app, dir) = kept().await;
    let path = plugin(
        dir.path(),
        "gitlab",
        &manifest(PLUGIN, "GitLab", &["outbound: gitlab.com"]),
    );

    let consent = read(&app, &path).await;
    assert_eq!(consent.name, "GitLab");
    assert_eq!(consent.provides, ["forge", "skills"]);
    assert_eq!(
        consent.capabilities,
        [PluginCapability::Outbound {
            host: "gitlab.com".to_owned()
        }],
    );
    assert!(
        page(&app).await.installed.is_empty(),
        "reading the card installs nothing"
    );

    let view = installs(&app, &path).await;

    let row = row(&view, PLUGIN);
    assert_eq!(row.name, "GitLab");
    assert_eq!(row.version, "1.4.0");
    assert_eq!(row.author, "Ada Lovelace");
    assert_eq!(row.provides, ["forge", "skills"]);
    assert_eq!(row.source, PluginSource::Local { path: path.clone() });
    assert!(row.enabled);
    assert_eq!(row.trouble, None);
    assert_eq!(row.elsewhere, None);
    assert!(view.built_in.is_empty(), "the fixture isn't listed");
}

/// Breaking a manifest on disk and pressing Update is trouble on its row, which
/// leaves the other plugin loaded; fixed, it loads at the next Update.
#[tokio::test]
async fn a_manifest_broken_on_disk_is_trouble_after_update_and_loads_again_once_fixed() {
    let (app, dir) = kept().await;
    let path = plugin(dir.path(), "one", &manifest(PLUGIN, "One", &[]));
    let other = plugin(dir.path(), "two", &manifest(ANOTHER, "Two", &[]));
    installs(&app, &path).await;
    installs(&app, &other).await;

    std::fs::write(
        Path::new(&path).join("plugin.yaml"),
        manifest(PLUGIN, "One", &[]).replace("version: 1.4.0", "version: 1.4"),
    )
    .unwrap();

    let PluginUpdated::Updated { plugins } = updates(&app, PLUGIN, None).await else {
        panic!("nothing new was asked for");
    };
    let trouble = row(&plugins, PLUGIN).trouble.clone().expect("trouble");
    assert!(trouble.contains("`version`"), "{trouble}");
    assert_eq!(
        row(&plugins, ANOTHER).trouble,
        None,
        "the other is unaffected"
    );

    std::fs::write(
        Path::new(&path).join("plugin.yaml"),
        manifest(PLUGIN, "One again", &[]),
    )
    .unwrap();

    let PluginUpdated::Updated { plugins } = updates(&app, PLUGIN, None).await else {
        panic!("nothing new was asked for");
    };
    assert_eq!(row(&plugins, PLUGIN).trouble, None);
    assert_eq!(row(&plugins, PLUGIN).name, "One again");
}

/// A manifest edited to ask for more is trouble at load, and Update shows the
/// card before the new grant lands.
#[tokio::test]
async fn a_manifest_asking_for_more_shows_the_card_at_update() {
    let (app, dir) = kept().await;
    let path = plugin(dir.path(), "one", &manifest(PLUGIN, "One", &[]));
    installs(&app, &path).await;

    std::fs::write(
        Path::new(&path).join("plugin.yaml"),
        manifest(PLUGIN, "One", &["outbound: gitlab.com"]),
    )
    .unwrap();

    let PluginUpdated::Consent { consent } = updates(&app, PLUGIN, None).await else {
        panic!("the card comes back first");
    };
    assert_eq!(
        consent.capabilities,
        [PluginCapability::Outbound {
            host: "gitlab.com".to_owned()
        }],
    );

    let PluginUpdated::Updated { plugins } =
        updates(&app, PLUGIN, Some(&consent.capabilities)).await
    else {
        panic!("granted, it lands");
    };
    assert_eq!(row(&plugins, PLUGIN).trouble, None);
}

/// Unticking a row writes the record disabled.
#[tokio::test]
async fn unticking_a_row_disables_it() {
    let (app, dir) = kept().await;
    let path = plugin(dir.path(), "one", &manifest(PLUGIN, "One", &[]));
    installs(&app, &path).await;

    let (status, said) = pressing(
        &app,
        &format!("/api/ui/plugins/{PLUGIN}/enabled"),
        serde_json::json!({ "enabled": false }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{said}");

    assert!(!row(&page(&app).await, PLUGIN).enabled);
}

/// A uuid already installed, and a built-in's, are refused at the card.
#[tokio::test]
async fn a_uuid_installed_or_built_in_is_refused() {
    let (app, dir) = kept().await;
    let path = plugin(dir.path(), "one", &manifest(PLUGIN, "One", &[]));
    installs(&app, &path).await;

    let twin = plugin(dir.path(), "twin", &manifest(PLUGIN, "Twin", &[]));
    let (status, said) = pressing(
        &app,
        "/api/ui/plugins/read",
        serde_json::json!({ "path": twin }),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{said}");
    assert!(said.contains("One is already installed"), "{said}");

    let shipped = plugin(dir.path(), "shipped", &manifest(FIXTURE, "Shipped", &[]));
    let (status, said) = pressing(
        &app,
        PLUGINS,
        serde_json::json!({ "path": shipped, "granted": [] }),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{said}");
    assert!(said.contains("built-in"), "{said}");

    assert_eq!(page(&app).await.installed.len(), 1);
}

/// A directory with no manifest is refused with the reader's own sentence.
#[tokio::test]
async fn a_directory_that_is_not_a_plugin_is_refused() {
    let (app, dir) = kept().await;

    let (status, said) = pressing(
        &app,
        "/api/ui/plugins/read",
        serde_json::json!({ "path": dir.path() }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{said}");
    assert!(said.contains("isn't a plugin"), "{said}");
}

/// Where the golden fixtures are written, relative to this crate — the
/// directory `ui_content.rs` writes the other endpoints' payloads to.
const FIXTURES: &str = "../../web/tests/fixtures";

/// Leave the viewer's tests the page and the consent card as the server writes
/// them: a plugin that loaded, one in trouble, and the card for one asking for
/// a host. The directories are written as `/home/you/src`, so that a fixture
/// is not rewritten by where a test happened to run.
#[tokio::test]
async fn the_page_and_the_card_are_written_for_the_viewer() {
    let (app, dir) = kept().await;
    let path = plugin(
        dir.path(),
        "gitlab",
        &manifest(PLUGIN, "GitLab", &["outbound: gitlab.com"]),
    );
    let other = plugin(dir.path(), "broken", &manifest(ANOTHER, "Broken", &[]));
    installs(&app, &path).await;
    installs(&app, &other).await;
    std::fs::write(
        Path::new(&other).join("plugin.yaml"),
        manifest(ANOTHER, "Broken", &[]).replace("version: 1.4.0", "version: 1.4"),
    )
    .unwrap();
    updates(&app, ANOTHER, None).await;

    let card = plugin(
        dir.path(),
        "card",
        &manifest(
            "9d8c7b6a-5f4e-4d3c-8b2a-1f0e9d8c7b6a",
            "Card",
            &[
                "outbound: gitlab.com",
                "filesystem: ~/.config/glab",
                "environment: GITLAB_TOKEN",
            ],
        ),
    );
    let consent = read(&app, &card).await;

    let at = dir.path().to_str().unwrap();
    let written = |name: &str, payload: String| {
        let payload: serde_json::Value =
            serde_json::from_str(&payload.replace(at, "/home/you/src")).unwrap();
        let mut pretty = serde_json::to_string_pretty(&payload).unwrap();
        pretty.push('\n');

        let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join(FIXTURES);
        std::fs::create_dir_all(&fixtures).unwrap();
        std::fs::write(fixtures.join(name), pretty).unwrap();
    };

    written(
        "plugins.json",
        serde_json::to_string(&page(&app).await).unwrap(),
    );
    written(
        "plugin-consent.json",
        serde_json::to_string(&consent).unwrap(),
    );
}
