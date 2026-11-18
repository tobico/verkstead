//! **Plugins installed from a local directory or a git URL**, on one device:
//! the consent card read off a path or a clone, **Install** with the grant it
//! showed, the row on the Plugins page, the trouble a broken manifest or a
//! failed pull is after **Update**, the checkbox, and the uuids that are
//! refused (ADR-0030).
//!
//! Every press is the one the section makes, on a router kept over a Data
//! Directory of its own; the plugins are directories written here. What the
//! record does across two devices is `replicating.rs`'s.

use std::path::Path;
use std::process::{Command, Stdio};

use axum::Router;
use axum::body::Body;
use axum::http::header::CONTENT_TYPE;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;
use verkstead_render::{
    PluginCapability, PluginConsent, PluginFix, PluginNeed, PluginRow, PluginSource, PluginUpdated,
    PluginsView,
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
        serde_json::json!({ "source": path }),
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
        serde_json::json!({ "source": path, "granted": consent.capabilities }),
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
        serde_json::json!({ "source": twin }),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{said}");
    assert!(said.contains("One is already installed"), "{said}");

    let shipped = plugin(dir.path(), "shipped", &manifest(FIXTURE, "Shipped", &[]));
    let (status, said) = pressing(
        &app,
        PLUGINS,
        serde_json::json!({ "source": shipped, "granted": [] }),
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
        serde_json::json!({ "source": dir.path() }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{said}");
    assert!(said.contains("isn't a plugin"), "{said}");
}

/// One git command in `dir`, answering what it printed.
fn git(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args([
            "-c",
            "user.name=Ada Lovelace",
            "-c",
            "user.email=ada@example.com",
            "-c",
            "init.defaultBranch=main",
        ])
        .args(args)
        // Dated, so a commit of the same tree is the same commit every run and
        // the viewer's fixture names one revision.
        .env("GIT_AUTHOR_DATE", "2026-11-18T12:00:00+11:00")
        .env("GIT_COMMITTER_DATE", "2026-11-18T12:00:00+11:00")
        .current_dir(dir)
        .stdin(Stdio::null())
        .output()
        .expect("git should be on the PATH for these tests");
    assert!(
        output.status.success(),
        "git {args:?} failed in {}: {}",
        dir.display(),
        String::from_utf8_lossy(&output.stderr),
    );
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

/// A git repository under `dir` holding `text` as its `plugin.yaml`, by its
/// `file://` URL and the commit it is at.
fn repository(dir: &Path, name: &str, text: &str) -> (String, String) {
    let at = dir.join(name);
    std::fs::create_dir_all(&at).unwrap();
    git(&at, &["init", "--quiet"]);
    committed(&at, text)
}

/// `text` committed as the repository at `at`'s `plugin.yaml`, answering its
/// URL and the new commit.
fn committed(at: &Path, text: &str) -> (String, String) {
    std::fs::write(at.join("plugin.yaml"), text).unwrap();
    git(at, &["add", "plugin.yaml"]);
    git(at, &["commit", "--quiet", "-m", "the manifest"]);
    (file_url(at), git(at, &["rev-parse", "HEAD"]))
}

/// `at` as a `file://` URL, which git clones like any other.
fn file_url(at: &Path) -> String {
    let path = at.to_string_lossy().replace('\\', "/");
    match path.starts_with('/') {
        true => format!("file://{path}"),
        false => format!("file:///{path}"),
    }
}

/// What the Data Directory's `plugins/` holds, by name.
fn plugins_dir(dir: &Path) -> Vec<String> {
    std::fs::read_dir(dir.join("plugins"))
        .map(|entries| {
            entries
                .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default()
}

/// A URL that isn't a repository is refused before anything is listed, and the
/// clone that was tried leaves nothing behind.
#[tokio::test]
async fn a_url_that_is_not_a_repository_is_refused_before_anything_is_written() {
    let (app, dir) = kept().await;
    let nowhere = file_url(&dir.path().join("nowhere"));

    let (status, said) = pressing(
        &app,
        "/api/ui/plugins/read",
        serde_json::json!({ "source": nowhere }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{said}");
    assert!(said.contains("couldn't be cloned"), "{said}");

    let (status, said) = pressing(
        &app,
        PLUGINS,
        serde_json::json!({ "source": nowhere, "granted": [] }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{said}");

    assert!(page(&app).await.installed.is_empty());
    assert_eq!(plugins_dir(dir.path()), Vec::<String>::new());
}

/// A git plugin's card shows the host it asks for before Install; Install
/// clones it under `plugins/<uuid>/` at the revision the record pins, and the
/// card's own clone is gone.
#[tokio::test]
async fn a_git_plugin_shows_its_card_then_installs_at_a_pinned_revision() {
    let (app, dir) = kept().await;
    let sources = tempfile::tempdir().unwrap();
    let (url, revision) = repository(
        sources.path(),
        "gitlab",
        &manifest(PLUGIN, "GitLab", &["outbound: gitlab.com"]),
    );

    let consent = read(&app, &url).await;
    assert_eq!(
        consent.capabilities,
        [PluginCapability::Outbound {
            host: "gitlab.com".to_owned()
        }],
    );
    assert!(page(&app).await.installed.is_empty());
    assert_eq!(
        plugins_dir(dir.path()),
        Vec::<String>::new(),
        "the card's clone is taken back"
    );

    let view = installs(&app, &url).await;

    let row = row(&view, PLUGIN);
    assert_eq!(
        row.source,
        PluginSource::Git {
            url: url.clone(),
            revision: revision.clone()
        }
    );
    assert_eq!(row.trouble, None);
    assert_eq!(row.elsewhere, None);
    assert_eq!(plugins_dir(dir.path()), [PLUGIN]);
    assert_eq!(
        git(
            &dir.path().join("plugins").join(PLUGIN),
            &["rev-parse", "HEAD"]
        ),
        revision
    );
}

/// A pull that fails leaves the old checkout loaded and says so on the row; a
/// pull that reads lands and re-pins; one asking for more shows the card
/// before the new revision lands.
#[tokio::test]
async fn update_on_a_git_row_pulls_and_re_pins() {
    let (app, dir) = kept().await;
    let sources = tempfile::tempdir().unwrap();
    let origin = sources.path().join("one");
    let (url, first) = repository(sources.path(), "one", &manifest(PLUGIN, "One", &[]));
    installs(&app, &url).await;
    let checkout = dir.path().join("plugins").join(PLUGIN);

    // A manifest that doesn't read: the old checkout stays, with the trouble.
    committed(
        &origin,
        &manifest(PLUGIN, "One", &[]).replace("version: 1.4.0", "version: 1.4"),
    );
    let PluginUpdated::Updated { plugins } = updates(&app, PLUGIN, None).await else {
        panic!("nothing new was asked for");
    };
    let trouble = row(&plugins, PLUGIN).trouble.clone().expect("trouble");
    assert!(trouble.contains("`version`"), "{trouble}");
    assert_eq!(
        row(&plugins, PLUGIN).source,
        PluginSource::Git {
            url: url.clone(),
            revision: first.clone()
        }
    );
    assert_eq!(git(&checkout, &["rev-parse", "HEAD"]), first);

    // Fixed, the pull lands and re-pins.
    let (_, second) = committed(&origin, &manifest(PLUGIN, "One again", &[]));
    let PluginUpdated::Updated { plugins } = updates(&app, PLUGIN, None).await else {
        panic!("nothing new was asked for");
    };
    assert_eq!(row(&plugins, PLUGIN).trouble, None);
    assert_eq!(row(&plugins, PLUGIN).name, "One again");
    assert_eq!(
        row(&plugins, PLUGIN).source,
        PluginSource::Git {
            url: url.clone(),
            revision: second.clone()
        }
    );
    assert_eq!(git(&checkout, &["rev-parse", "HEAD"]), second);

    // Asking for more: the card first, and nothing lands until it is granted.
    let (_, third) = committed(
        &origin,
        &manifest(PLUGIN, "One again", &["outbound: gitlab.com"]),
    );
    let PluginUpdated::Consent { consent } = updates(&app, PLUGIN, None).await else {
        panic!("the card comes back first");
    };
    assert_eq!(
        consent.capabilities,
        [PluginCapability::Outbound {
            host: "gitlab.com".to_owned()
        }],
    );
    assert_eq!(
        row(&page(&app).await, PLUGIN).source,
        PluginSource::Git {
            url: url.clone(),
            revision: second.clone()
        }
    );
    assert_eq!(git(&checkout, &["rev-parse", "HEAD"]), second);

    let PluginUpdated::Updated { plugins } =
        updates(&app, PLUGIN, Some(&consent.capabilities)).await
    else {
        panic!("granted, it lands");
    };
    assert_eq!(
        row(&plugins, PLUGIN).source,
        PluginSource::Git {
            url: url.clone(),
            revision: third.clone()
        }
    );
    assert_eq!(row(&plugins, PLUGIN).trouble, None);

    // The repository gone: the pull fails, and the old checkout stays loaded.
    std::fs::remove_dir_all(&origin).unwrap();
    let PluginUpdated::Updated { plugins } = updates(&app, PLUGIN, None).await else {
        panic!("nothing new was asked for");
    };
    let trouble = row(&plugins, PLUGIN).trouble.clone().expect("trouble");
    assert!(trouble.contains("The pull failed"), "{trouble}");
    assert_eq!(git(&checkout, &["rev-parse", "HEAD"]), third);
}

/// Where the golden fixtures are written, relative to this crate — the
/// directory `ui_content.rs` writes the other endpoints' payloads to.
/// `text`, a manifest, depending on the plugin `uuid` called `name` at
/// `version`, fetched from `repository` where there is one.
fn depending(
    text: &str,
    uuid: &str,
    name: &str,
    version: &str,
    repository: Option<&str>,
) -> String {
    let repository = repository
        .map(|url| format!("    repository: {url}\n"))
        .unwrap_or_default();
    format!(
        "{text}dependencies:\n  - uuid: {uuid}\n    name: {name}\n    author:\n      \
         name: Grace Hopper\n      email: grace@example.com\n{repository}    version: \"{version}\"\n"
    )
}

/// The dependency the tests below install, ANOTHER, called Base.
const BASE: &str = "Base";

/// A plugin depending on one that isn't installed is in trouble naming it, and
/// offers its Install from the repository it names; installed through the
/// card, the dependent loads.
#[tokio::test]
async fn a_missing_dependency_is_trouble_and_installs_from_its_repository() {
    let (app, dir) = kept().await;
    let (url, _) = repository(
        &dir.path().join("sources"),
        "base",
        &manifest(ANOTHER, BASE, &[]),
    );
    let path = plugin(
        dir.path(),
        "one",
        &depending(
            &manifest(PLUGIN, "One", &[]),
            ANOTHER,
            BASE,
            "^1.2",
            Some(&url),
        ),
    );

    let view = installs(&app, &path).await;
    let dependent = row(&view, PLUGIN);
    let trouble = dependent.trouble.clone().expect("trouble");
    assert!(
        trouble.contains("needs Base, which isn't installed"),
        "{trouble}"
    );
    assert_eq!(
        dependent.needs,
        [PluginNeed {
            uuid: ANOTHER.to_owned(),
            name: BASE.to_owned(),
            fix: PluginFix::Install {
                repository: url.clone()
            },
        }],
    );

    let view = installs(&app, &url).await;
    assert_eq!(row(&view, ANOTHER).trouble, None);
    assert_eq!(row(&view, PLUGIN).trouble, None, "the dependent loads");
    assert!(row(&view, PLUGIN).needs.is_empty());
}

/// A dependency installed outside the specifier reads as that, and offers its
/// Update; updated into it, the dependent loads.
#[tokio::test]
async fn a_dependency_outside_its_specifier_is_outdated_and_offers_the_update() {
    let (app, dir) = kept().await;
    let base = plugin(dir.path(), "base", &manifest(ANOTHER, BASE, &[]));
    let path = plugin(
        dir.path(),
        "one",
        &depending(&manifest(PLUGIN, "One", &[]), ANOTHER, BASE, "^2", None),
    );
    installs(&app, &base).await;

    let view = installs(&app, &path).await;
    let dependent = row(&view, PLUGIN);
    let trouble = dependent.trouble.clone().expect("trouble");
    assert!(
        trouble.contains("needs Base ^2, and 1.4.0 is installed"),
        "{trouble}"
    );
    assert_eq!(
        dependent.needs,
        [PluginNeed {
            uuid: ANOTHER.to_owned(),
            name: BASE.to_owned(),
            fix: PluginFix::Update,
        }],
    );

    std::fs::write(
        Path::new(&base).join("plugin.yaml"),
        manifest(ANOTHER, BASE, &[]).replace("version: 1.4.0", "version: 2.0.1"),
    )
    .unwrap();
    let PluginUpdated::Updated { plugins } = updates(&app, ANOTHER, None).await else {
        panic!("nothing new was asked for");
    };
    assert_eq!(row(&plugins, PLUGIN).trouble, None);
}

/// Unticking a dependency puts its dependent in trouble as disabled, and
/// ticking it again clears that. A plugin depending on that one in turn is
/// in trouble too.
#[tokio::test]
async fn unticking_a_dependency_puts_its_dependent_in_trouble_until_ticked() {
    let (app, dir) = kept().await;
    let base = plugin(dir.path(), "base", &manifest(ANOTHER, BASE, &[]));
    let path = plugin(
        dir.path(),
        "one",
        &depending(&manifest(PLUGIN, "One", &[]), ANOTHER, BASE, "^1", None),
    );
    const TOP: &str = "6a5b4c3d-2e1f-4a0b-9c8d-7e6f5a4b3c2d";
    let top = plugin(
        dir.path(),
        "top",
        &depending(&manifest(TOP, "Top", &[]), PLUGIN, "One", ">=1.0", None),
    );
    installs(&app, &base).await;
    installs(&app, &path).await;
    let view = installs(&app, &top).await;
    assert_eq!(row(&view, PLUGIN).trouble, None);
    assert_eq!(row(&view, TOP).trouble, None);

    let ticked = |enabled: bool| {
        let app = app.clone();
        async move {
            let (status, said) = pressing(
                &app,
                &format!("/api/ui/plugins/{ANOTHER}/enabled"),
                serde_json::json!({ "enabled": enabled }),
            )
            .await;
            assert_eq!(status, StatusCode::OK, "{said}");
            serde_json::from_str::<PluginsView>(&said).unwrap()
        }
    };

    let view = ticked(false).await;
    let trouble = row(&view, PLUGIN).trouble.clone().expect("trouble");
    assert!(
        trouble.contains("needs Base, which is disabled"),
        "{trouble}"
    );
    assert!(row(&view, PLUGIN).needs.is_empty(), "ticking it is the fix");
    let trouble = row(&view, TOP).trouble.clone().expect("trouble");
    assert!(
        trouble.contains("needs One, which didn't load"),
        "{trouble}"
    );

    let view = ticked(true).await;
    assert_eq!(row(&view, PLUGIN).trouble, None);
    assert_eq!(row(&view, TOP).trouble, None);
}

const FIXTURES: &str = "../../web/tests/fixtures";

/// Leave the viewer's tests the page and the consent card as the server writes
/// them: a plugin that loaded, one in trouble, one from git, one whose
/// dependency isn't installed, and the card for
/// one asking for a host. The directories are written as `/home/you/src`, so that a fixture
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
    let (url, _) = repository(
        &dir.path().join("sources"),
        "skills",
        &manifest("2e4f6a8b-1c3d-4e5f-8a7b-9c0d1e2f3a4b", "Shared skills", &[]),
    );
    installs(&app, &url).await;
    let dependent = plugin(
        dir.path(),
        "dependent",
        &depending(
            &manifest("3b2a1f0e-9d8c-4b7a-8f6e-5d4c3b2a1f0e", "Dependent", &[]),
            "8e7d6c5b-4a39-4281-9f0e-1d2c3b4a5968",
            BASE,
            "^1",
            Some("https://example.com/base.git"),
        ),
    );
    installs(&app, &dependent).await;

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
