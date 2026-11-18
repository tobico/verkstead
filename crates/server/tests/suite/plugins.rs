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
    PluginCapability, PluginConsent, PluginFieldRefused, PluginFieldValue, PluginFix, PluginNeed,
    PluginRow, PluginSettingsSaved, PluginSettingsView, PluginSource, PluginUpdated, PluginsView,
};
use verkstead_server::plugins::built_in::{FIXTURE, VERKSTEAD};
use verkstead_server::settings::Settings;
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
         provides:\n  - category: forge\n  - category: activity\n  - category: forge\n\
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

/// A plugin under `dir/<name>` called `name`, whose one `provides:` entry is
/// the Skill Pack `pack` holding the one skill `reviewing`.
fn packing(dir: &Path, uuid: &str, name: &str, pack: &str) -> String {
    let text = manifest(uuid, name, &[]).replace(
        "provides:\n  - category: forge\n  - category: activity\n  - category: forge\n",
        &format!("provides:\n  - category: skills\n    name: {pack}\n    directory: skills\n"),
    );
    let at = plugin(dir, name, &text);
    let skill = Path::new(&at).join("skills/reviewing");
    std::fs::create_dir_all(&skill).unwrap();
    std::fs::write(
        skill.join("SKILL.md"),
        "---\nname: reviewing\ndescription: Review Rust.\n---\n\nRead /verkstead/skills/rust-review/reviewing/notes.md.\n",
    )
    .unwrap();
    std::fs::write(skill.join("notes.md"), "Be kind.\n").unwrap();
    at
}

/// The skills Verkstead ships are the Built-in group's first row: providing
/// skills, always on with no tick, and with no Remove — and the fixture is not
/// listed.
#[tokio::test]
async fn the_verkstead_pack_is_a_built_in_row() {
    let (app, _dir) = kept().await;

    let view = page(&app).await;

    let Some(row) = view.built_in.first() else {
        panic!("a built-in is listed: {view:?}");
    };
    assert_eq!(row.uuid, VERKSTEAD);
    assert_eq!(row.name, "Verkstead");
    assert_eq!(row.provides, ["skills"]);
    assert_eq!(row.source, PluginSource::BuiltIn);
    assert!(row.enabled);
    assert_eq!(row.trouble, None);
    assert!(view.installed.iter().all(|row| row.uuid != FIXTURE));

    let (status, said) = pressing(
        &app,
        &format!("/api/ui/plugins/{VERKSTEAD}/remove"),
        serde_json::json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "a built-in has no Remove");
    assert!(said.contains("built in"), "{said}");
    assert!(!said.contains("Untick"), "it has no tick: {said}");

    let (status, said) = pressing(
        &app,
        &format!("/api/ui/plugins/{VERKSTEAD}/enabled"),
        serde_json::json!({ "enabled": false }),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{said}");
    assert!(said.contains("always on"), "{said}");
    assert!(page(&app).await.built_in[0].enabled);
}

/// An enabled plugin's pack is written out under its name, and taken away
/// again when the plugin is unticked.
#[tokio::test]
async fn an_installed_plugins_pack_lands_under_its_name_until_it_is_disabled() {
    let (app, dir) = kept().await;
    let path = packing(dir.path(), PLUGIN, "Rust review", "rust-review");

    let view = installs(&app, &path).await;
    assert_eq!(row(&view, PLUGIN).trouble, None);

    let pack = dir.path().join("skills/rust-review");
    let skill = std::fs::read_to_string(pack.join("reviewing/SKILL.md")).unwrap();
    assert!(skill.contains("name: reviewing"), "{skill}");
    assert_eq!(
        std::fs::read_to_string(pack.join("reviewing/notes.md")).unwrap(),
        "Be kind.\n",
        "every file of the pack lands, not only its SKILL.md files",
    );

    let (status, said) = pressing(
        &app,
        &format!("/api/ui/plugins/{PLUGIN}/enabled"),
        serde_json::json!({ "enabled": false }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{said}");
    assert!(!pack.exists(), "a disabled plugin's pack is not there");

    let (status, said) = pressing(
        &app,
        &format!("/api/ui/plugins/{PLUGIN}/enabled"),
        serde_json::json!({ "enabled": true }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{said}");
    assert!(
        pack.join("reviewing/SKILL.md").is_file(),
        "ticked, it is back"
    );

    let (status, said) = pressing(
        &app,
        &format!("/api/ui/plugins/{PLUGIN}/remove"),
        serde_json::json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{said}");
    assert!(!pack.exists(), "a removed plugin's pack is not there");
}

/// A load rewrites a pack in place: its directory is the one a running
/// session is bound, so it is the same directory afterwards, and only what the
/// pack no longer has is taken out of it.
#[tokio::test]
async fn a_reload_rewrites_a_pack_in_its_own_directory() {
    let (app, dir) = kept().await;
    let path = packing(dir.path(), PLUGIN, "Rust review", "rust-review");
    installs(&app, &path).await;

    let pack = dir.path().join("skills/rust-review");
    #[cfg(unix)]
    let before = std::os::unix::fs::MetadataExt::ino(&std::fs::metadata(&pack).unwrap());

    std::fs::remove_file(Path::new(&path).join("skills/reviewing/notes.md")).unwrap();
    let linting = Path::new(&path).join("skills/linting");
    std::fs::create_dir_all(&linting).unwrap();
    std::fs::write(
        linting.join("SKILL.md"),
        "---\nname: linting\ndescription: Lint Rust.\n---\n",
    )
    .unwrap();

    // Any load writes every pack again, so another plugin's install is one.
    installs(
        &app,
        &plugin(
            dir.path(),
            "gitlab",
            &manifest(ANOTHER, "GitLab", &["outbound: gitlab.com"]),
        ),
    )
    .await;

    #[cfg(unix)]
    assert_eq!(
        std::os::unix::fs::MetadataExt::ino(&std::fs::metadata(&pack).unwrap()),
        before,
        "a session bound the pack's directory is still bound the pack",
    );
    assert!(pack.join("reviewing/SKILL.md").is_file());
    assert!(
        pack.join("linting/SKILL.md").is_file(),
        "what it gained lands"
    );
    assert!(
        !pack.join("reviewing/notes.md").exists(),
        "and what it no longer has is gone"
    );
}

/// A pack name is one plugin's: the second plugin claiming it loads in
/// trouble naming the first, and its pack is not written over the first's.
#[tokio::test]
async fn a_second_plugin_claiming_a_pack_name_loads_in_trouble() {
    let (app, dir) = kept().await;
    installs(
        &app,
        &packing(dir.path(), PLUGIN, "Rust review", "rust-review"),
    )
    .await;

    let view = installs(
        &app,
        &packing(dir.path(), ANOTHER, "Other review", "rust-review"),
    )
    .await;

    assert_eq!(row(&view, PLUGIN).trouble, None);
    assert_eq!(
        row(&view, ANOTHER).trouble.as_deref(),
        Some(
            "Its skill pack is named `rust-review`, and so is Rust review's. Give it a name of \
             its own."
        ),
    );
    assert!(
        dir.path()
            .join("skills/rust-review/reviewing/SKILL.md")
            .is_file()
    );
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
    assert_eq!(consent.provides, ["forge", "activity"]);
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
    assert_eq!(row.provides, ["forge", "activity"]);
    assert_eq!(row.source, PluginSource::Local { path: path.clone() });
    assert!(row.enabled);
    assert_eq!(row.trouble, None);
    assert_eq!(row.elsewhere, None);
    assert!(
        view.built_in.iter().all(|row| row.uuid != FIXTURE),
        "the fixture isn't listed"
    );
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

const GO_PLUGIN: &str = "01b5328a-3524-457d-ab88-1af1c65079ea";
const RUST_CPP: &str = "795515ae-96cd-4d7c-8071-e6753b3f4624";

fn built_in<'a>(view: &'a PluginsView, name: &str) -> &'a PluginRow {
    view.built_in
        .iter()
        .find(|row| row.name == name)
        .unwrap_or_else(|| panic!("{name} is listed under Built-in in {view:?}"))
}

/// The six Language plugins are listed under Built-in, each saying the
/// languages it provides, on with nothing written; the fixture isn't listed,
/// and a built-in has no Remove or Update.
#[tokio::test]
async fn the_language_plugins_are_listed_under_built_in() {
    let (app, _dir) = kept().await;
    let view = page(&app).await;

    assert!(view.installed.is_empty());
    let names: Vec<&str> = view.built_in.iter().map(|row| row.name.as_str()).collect();
    assert_eq!(names.len(), 7, "the pack and six languages: {names:?}");
    assert!(!view.built_in.iter().any(|row| row.uuid == FIXTURE));

    let rust = built_in(&view, "Rust & C/C++");
    assert_eq!(rust.languages, ["Rust", "C/C++"]);
    assert_eq!(rust.provides, ["language"]);
    assert_eq!(rust.source, PluginSource::BuiltIn);
    assert!(rust.enabled);
    for (name, language) in [
        ("Go", "Go"),
        ("Node", "Node"),
        ("Python", "Python"),
        (".NET", ".NET"),
        ("JVM", "JVM"),
    ] {
        let row = built_in(&view, name);
        assert_eq!(row.languages, [language], "{name}");
        assert!(row.enabled, "{name}");
    }

    let go = &built_in(&view, "Go").uuid;
    for press in ["remove", "update"] {
        let (status, said) = pressing(
            &app,
            &format!("/api/ui/plugins/{go}/{press}"),
            serde_json::json!({ "granted": null }),
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT, "{press}: {said}");
        assert!(said.contains("built in"), "{said}");
    }
}

/// Unticking Go is what the next session reads: its languages have Go off,
/// and none of Go's variables are given — and ticking it again gives them
/// back. Everything else stays on.
#[tokio::test]
async fn the_tick_on_a_built_in_is_whether_sessions_get_its_language() {
    use verkstead_server::languages::{self, GO, Machine, RUST};

    let (app, dir) = kept().await;
    let settings = Settings::in_data_dir(dir.path());
    let machine = Machine::of(&dir.path().join("cache"), dir.path(), None);
    let given = |settings: &Settings| {
        languages::configured(&settings.config())
            .given(&machine)
            .env()
            .iter()
            .map(|(name, _)| name.clone())
            .collect::<Vec<_>>()
    };
    assert!(given(&settings).contains(&"GOMODCACHE".to_owned()));

    let enabled = format!("/api/ui/plugins/{GO_PLUGIN}/enabled");
    let press =
        |enabled_: bool| pressing(&app, &enabled, serde_json::json!({ "enabled": enabled_ }));

    let (status, said) = press(false).await;
    assert_eq!(status, StatusCode::OK, "{said}");
    let view: PluginsView = serde_json::from_str(&said).unwrap();
    assert!(!built_in(&view, "Go").enabled);
    assert!(built_in(&view, "Rust & C/C++").enabled);

    let languages = languages::configured(&settings.config());
    assert!(!languages.get(GO).unwrap().enabled());
    assert!(languages.get(RUST).unwrap().enabled());
    assert!(!given(&settings).contains(&"GOMODCACHE".to_owned()));
    assert!(given(&settings).contains(&"CARGO_HOME".to_owned()));

    let (status, said) = press(true).await;
    assert_eq!(status, StatusCode::OK, "{said}");
    assert!(built_in(&page(&app).await, "Go").enabled);
    assert!(given(&settings).contains(&"GOMODCACHE".to_owned()));
}

/// A Language plugin's size is a field on its pane, drawn before anything was
/// written to it with its default as the placeholder. A size that is not one
/// is refused and nothing is written; one saved is what the next session's
/// `SCCACHE_CACHE_SIZE` reads and what Rust's stores are held to — and Go's
/// own, saved on its own pane, what Go's are.
#[tokio::test]
async fn a_size_saved_on_a_language_plugins_pane_is_what_sessions_read() {
    use verkstead_server::languages::{self, CPP, GO, Machine, RUST, SCCACHE};

    let (app, dir) = kept().await;
    let settings = Settings::in_data_dir(dir.path());
    let machine = Machine::of(
        &dir.path().join("cache"),
        dir.path(),
        Some(dir.path().join("sccache")),
    );
    let sccache_size = |settings: &Settings| {
        languages::configured(&settings.config())
            .given(&machine)
            .env()
            .iter()
            .find(|(name, _)| name == "SCCACHE_CACHE_SIZE")
            .map(|(_, value)| value.clone())
    };
    assert_eq!(sccache_size(&settings).as_deref(), Some("30G"));

    let at = |uuid: &str| format!("{PLUGINS}/{uuid}/settings");
    let read = |uuid: &str| {
        let app = app.clone();
        let uri = at(uuid);
        async move {
            let answered = app
                .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(answered.status(), StatusCode::OK);
            let bytes = answered.into_body().collect().await.unwrap().to_bytes();
            serde_json::from_slice::<PluginSettingsView>(&bytes).unwrap()
        }
    };
    let sized = |size: &str| {
        serde_json::json!({ "fields": [
        { "key": "size", "value": { "kind": "Size", "value": size } },
    ] })
    };

    // Drawn before anything is written to it, empty with its default under.
    let view = read(RUST_CPP).await;
    let field = &view.fields[0];
    assert_eq!(field.key, "size");
    assert_eq!(field.placeholder.as_deref(), Some("30G"));
    assert_eq!(
        field.value,
        PluginFieldValue::Size {
            value: String::new()
        }
    );
    assert_eq!(
        read(GO_PLUGIN).await.fields[0].placeholder.as_deref(),
        Some("10G")
    );

    // Not a size: refused with the reason, and nothing written.
    let (status, said) = pressing(&app, &at(RUST_CPP), sized("1.5g")).await;
    assert_eq!(status, StatusCode::OK, "{said}");
    let saved: PluginSettingsSaved = serde_json::from_str(&said).unwrap();
    assert_eq!(
        saved.refused,
        [PluginFieldRefused {
            key: "size".to_owned(),
            why: "1.5g isn't a size. Write a whole number with K, M, G or T after it, such as \
                  30G."
                .to_owned(),
        }],
    );
    assert_eq!(
        read(RUST_CPP).await.fields[0].value,
        PluginFieldValue::Size {
            value: String::new()
        }
    );
    assert_eq!(sccache_size(&settings).as_deref(), Some("30G"));

    // A size: what the next session reads, for both of its languages, and
    // the plugin still on.
    let (status, said) = pressing(&app, &at(RUST_CPP), sized("12G")).await;
    assert_eq!(status, StatusCode::OK, "{said}");
    assert_eq!(
        read(RUST_CPP).await.fields[0].value,
        PluginFieldValue::Size {
            value: "12G".to_owned()
        }
    );
    assert_eq!(sccache_size(&settings).as_deref(), Some("12G"));
    let configured = languages::configured(&settings.config());
    assert_eq!(configured.get(RUST).unwrap().size(RUST), "12G");
    assert_eq!(configured.get(CPP).unwrap().size(CPP), "12G");
    assert_eq!(configured.wanting(SCCACHE), Some("12G"));
    assert!(built_in(&page(&app).await, "Rust & C/C++").enabled);
    assert_eq!(configured.get(GO).unwrap().size(GO), "10G");

    // Go's own, on Go's pane.
    let (status, said) = pressing(&app, &at(GO_PLUGIN), sized("3G")).await;
    assert_eq!(status, StatusCode::OK, "{said}");
    let configured = languages::configured(&settings.config());
    assert_eq!(configured.get(GO).unwrap().size(GO), "3G");
    assert_eq!(configured.get(RUST).unwrap().size(RUST), "12G");

    // And cleared, the default again.
    let (status, said) = pressing(&app, &at(RUST_CPP), sized("")).await;
    assert_eq!(status, StatusCode::OK, "{said}");
    assert_eq!(sccache_size(&settings).as_deref(), Some("30G"));
}

/// What one session on `dir`'s Data Directory is given: each variable by
/// name, with its value.
fn given_to_a_session(settings: &Settings, dir: &Path) -> Vec<(String, String)> {
    let machine = verkstead_server::languages::Machine::of(&dir.join("cache"), dir, None);
    verkstead_server::languages::configured(&settings.config())
        .given(&machine)
        .env()
        .to_vec()
}

/// A `languages:` entry naming a language no plugin provides is a row of its
/// own under Installed, from `config.yaml`: a session gets its variable, and
/// unticking the row drops it from the next. It has no Remove or Update; its
/// pane has the size field, saved and read like a built-in's.
#[tokio::test]
async fn an_installers_own_language_is_a_row_of_its_own() {
    let (app, dir) = kept().await;
    let settings = Settings::in_data_dir(dir.path());
    std::fs::write(
        settings.config_path(),
        "languages:\n  zig:\n    env:\n      ZIG_GLOBAL_CACHE_DIR: \"{cache}/zig\"\n",
    )
    .unwrap();
    let named = |settings: &Settings| {
        given_to_a_session(settings, dir.path())
            .iter()
            .any(|(name, _)| name == "ZIG_GLOBAL_CACHE_DIR")
    };
    assert!(named(&settings));

    let view = page(&app).await;
    let zig = view
        .installed
        .iter()
        .find(|row| row.source == PluginSource::Config)
        .unwrap_or_else(|| panic!("an eighth row, under Installed: {view:#?}"))
        .clone();
    assert_eq!(zig.name, "zig", "named by its entry, there being no label");
    assert_eq!(zig.languages, ["zig"]);
    assert_eq!(zig.provides, ["language"]);
    assert!(zig.enabled);
    assert_eq!(zig.trouble, None);
    assert_eq!(zig.settings, 1, "its store is under the Build Cache");
    assert!(view.built_in.iter().all(|row| row.uuid != zig.uuid));

    let enabled = format!("{PLUGINS}/{}/enabled", zig.uuid);
    let (status, said) = pressing(&app, &enabled, serde_json::json!({ "enabled": false })).await;
    assert_eq!(status, StatusCode::OK, "{said}");
    let view: PluginsView = serde_json::from_str(&said).unwrap();
    assert!(!row(&view, &zig.uuid).enabled);
    assert!(!named(&settings), "the next session gets no zig");

    let (status, said) = pressing(&app, &enabled, serde_json::json!({ "enabled": true })).await;
    assert_eq!(status, StatusCode::OK, "{said}");
    assert!(named(&settings));

    for press in ["remove", "update"] {
        let (status, said) = pressing(
            &app,
            &format!("{PLUGINS}/{}/{press}", zig.uuid),
            serde_json::json!({ "granted": null }),
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT, "{press}: {said}");
        assert!(said.contains("config.yaml"), "{said}");
    }

    // Its pane: the size, its default under it, and its cache.
    let at = format!("{PLUGINS}/{}/settings", zig.uuid);
    let answered = app
        .clone()
        .oneshot(Request::builder().uri(&at).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(answered.status(), StatusCode::OK);
    let bytes = answered.into_body().collect().await.unwrap().to_bytes();
    let pane: PluginSettingsView = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(pane.fields[0].key, "size");
    assert_eq!(pane.fields[0].placeholder.as_deref(), Some("10G"));
    assert_eq!(pane.languages.len(), 1);
    assert_eq!(pane.languages[0].name, "zig");

    let (status, said) = pressing(
        &app,
        &at,
        serde_json::json!({ "fields": [
            { "key": "size", "value": { "kind": "Size", "value": "4G" } },
        ] }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{said}");
    let configured = verkstead_server::languages::configured(&settings.config());
    assert_eq!(configured.get("zig").unwrap().size("zig"), "4G");
}

/// A plugin under `dir/<name>`, called `name`, providing the one language
/// `language` with the variable `ZIG_GLOBAL_CACHE_DIR` and a store, and a
/// `size` setting.
fn languaging(dir: &Path, uuid: &str, name: &str, language: &str) -> String {
    let text = manifest(uuid, name, &[]).replace(
        "provides:\n  - category: forge\n  - category: activity\n  - category: forge\n",
        &format!(
            "settings:\n  - key: size\n    label: Store size limit\n    kind: size\n\
             provides:\n  - category: language\n    language: {language}\n    label: Zig\n    \
             env:\n      ZIG_GLOBAL_CACHE_DIR: \"{{cache}}/zig\"\n    \
             stores:\n      zig:\n        dir: \"{{cache}}/zig\"\n"
        ),
    );
    plugin(dir, name, &text)
}

/// A Language plugin installed from a directory is a language every session
/// gets: its row says which, its tick is the switch, and its pane's size is
/// what `{size}` reads. One providing a language another plugin already
/// provides loads in trouble, and leaves that language as it was.
#[tokio::test]
async fn an_installed_language_plugin_reaches_sessions() {
    let (app, dir) = kept().await;
    let settings = Settings::in_data_dir(dir.path());
    let named = |settings: &Settings| {
        given_to_a_session(settings, dir.path())
            .iter()
            .any(|(name, _)| name == "ZIG_GLOBAL_CACHE_DIR")
    };
    assert!(!named(&settings));

    let view = installs(&app, &languaging(dir.path(), PLUGIN, "Zig support", "zig")).await;
    assert_eq!(row(&view, PLUGIN).trouble, None);
    assert_eq!(row(&view, PLUGIN).languages, ["Zig"]);
    assert!(
        view.installed
            .iter()
            .all(|row| row.source != PluginSource::Config),
        "no row of its own from config.yaml: {view:#?}",
    );
    assert!(named(&settings), "the next session gets zig");

    // Its pane: the size, the default under it, and its cache.
    let at = format!("{PLUGINS}/{PLUGIN}/settings");
    let answered = app
        .clone()
        .oneshot(Request::builder().uri(&at).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let bytes = answered.into_body().collect().await.unwrap().to_bytes();
    let pane: PluginSettingsView = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(pane.fields[0].placeholder.as_deref(), Some("10G"));
    assert_eq!(pane.languages.len(), 1);
    assert_eq!(pane.languages[0].name, "zig");

    let (status, said) = pressing(
        &app,
        &at,
        serde_json::json!({ "fields": [
            { "key": "size", "value": { "kind": "Size", "value": "4G" } },
        ] }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{said}");
    let configured = verkstead_server::languages::configured(&settings.config());
    assert_eq!(configured.get("zig").unwrap().size("zig"), "4G");

    // An override in config.yaml merges over it, as over a built-in.
    std::fs::write(
        settings.config_path(),
        "languages:\n  zig:\n    env:\n      ZIG_LOCAL_CACHE_DIR: /elsewhere\n",
    )
    .unwrap();
    assert!(
        given_to_a_session(&settings, dir.path())
            .iter()
            .any(|(name, value)| name == "ZIG_LOCAL_CACHE_DIR" && value == "/elsewhere")
    );
    assert!(named(&settings));

    let (status, said) = pressing(
        &app,
        &format!("{PLUGINS}/{PLUGIN}/enabled"),
        serde_json::json!({ "enabled": false }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{said}");
    assert!(!named(&settings), "unticked, the next session gets no zig");

    // One providing Go, which a built-in already does.
    let view = installs(&app, &languaging(dir.path(), ANOTHER, "Other Go", "go")).await;
    assert_eq!(
        row(&view, ANOTHER).trouble.as_deref(),
        Some("It provides the language `go`, and so does Go. Give it a name of its own."),
    );
    assert!(
        !given_to_a_session(&settings, dir.path())
            .iter()
            .any(|(name, _)| name == "ZIG_GLOBAL_CACHE_DIR"),
        "and Go is Go's",
    );
}

/// An override of a built-in that won't load — a mistyped `detct` in Go's —
/// is trouble on Go's row, and Go's sessions get the variables Verkstead
/// ships; an override that loads, beside it, still wins.
#[tokio::test]
async fn an_override_that_will_not_load_is_trouble_on_its_plugins_row() {
    let (app, dir) = kept().await;
    let settings = Settings::in_data_dir(dir.path());
    std::fs::write(
        settings.config_path(),
        "languages:\n  go:\n    detct:\n      - go.mod\n    env:\n      \
         GOMODCACHE: \"{cache}/mine\"\n  rust:\n    env:\n      \
         CARGO_HOME: \"{cache}/cargo-mine\"\n",
    )
    .unwrap();

    let view = page(&app).await;
    let trouble = built_in(&view, "Go")
        .trouble
        .clone()
        .unwrap_or_else(|| panic!("Go's row says why: {view:#?}"));
    assert!(
        trouble.starts_with("The go entry in config.yaml"),
        "{trouble}"
    );
    assert!(trouble.contains("detct"), "{trouble}");
    assert!(trouble.ends_with("Go runs on what Verkstead ships until the entry is fixed."));
    assert_eq!(built_in(&view, "Rust & C/C++").trouble, None);
    assert!(view.installed.is_empty(), "no row of its own: {view:#?}");

    let given = given_to_a_session(&settings, dir.path());
    let value = |name: &str| {
        given
            .iter()
            .find(|(named, _)| named == name)
            .map(|(_, value)| value.clone())
            .unwrap_or_else(|| panic!("{name} is given: {given:?}"))
    };
    assert!(
        Path::new(&value("GOMODCACHE")).ends_with("go/mod"),
        "{}",
        value("GOMODCACHE")
    );
    assert!(
        value("CARGO_HOME").ends_with("cargo-mine"),
        "{}",
        value("CARGO_HOME")
    );
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

/// Remove on a git plugin nothing depends on takes its row off the page and
/// deletes its clone; a local plugin's own directory stays where it is.
#[tokio::test]
async fn remove_takes_the_row_off_and_deletes_a_git_checkout() {
    let (app, dir) = kept().await;
    let sources = tempfile::tempdir().unwrap();
    let (url, _) = repository(sources.path(), "gitlab", &manifest(PLUGIN, "GitLab", &[]));
    let local = plugin(dir.path(), "base", &manifest(ANOTHER, BASE, &[]));
    installs(&app, &url).await;
    installs(&app, &local).await;
    assert_eq!(plugins_dir(dir.path()), [PLUGIN]);

    let (status, said) = pressing(
        &app,
        &format!("/api/ui/plugins/{PLUGIN}/remove"),
        serde_json::json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{said}");
    let view: PluginsView = serde_json::from_str(&said).unwrap();
    assert!(view.installed.iter().all(|row| row.uuid != PLUGIN));
    assert_eq!(plugins_dir(dir.path()), Vec::<String>::new());

    let (status, said) = pressing(
        &app,
        &format!("/api/ui/plugins/{ANOTHER}/remove"),
        serde_json::json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{said}");
    assert!(page(&app).await.installed.is_empty());
    assert!(
        Path::new(&local).join("plugin.yaml").is_file(),
        "a local plugin's directory is the human's own",
    );
}

/// A plugin removed is installed again under its own uuid, which is its
/// author's: the record is made again, and a git one is cloned afresh.
#[tokio::test]
async fn a_removed_plugin_can_be_installed_again() {
    let (app, dir) = kept().await;
    let sources = tempfile::tempdir().unwrap();
    let (url, _) = repository(sources.path(), "gitlab", &manifest(PLUGIN, "GitLab", &[]));
    let local = plugin(dir.path(), "base", &manifest(ANOTHER, BASE, &[]));

    for source in [&url, &local] {
        let uuid = installs(&app, source)
            .await
            .installed
            .last()
            .unwrap()
            .uuid
            .clone();
        let (status, said) = pressing(
            &app,
            &format!("/api/ui/plugins/{uuid}/remove"),
            serde_json::json!({}),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{said}");
    }
    assert!(page(&app).await.installed.is_empty());

    installs(&app, &url).await;
    let view = installs(&app, &local).await;

    assert_eq!(row(&view, PLUGIN).name, "GitLab");
    assert_eq!(row(&view, PLUGIN).trouble, None);
    assert_eq!(row(&view, ANOTHER).name, BASE);
    assert_eq!(plugins_dir(dir.path()), [PLUGIN]);
}

/// Remove on a plugin installed plugins depend on is refused, counting and
/// naming them, ticked or not, and nothing is removed. A built-in has no
/// record to remove.
#[tokio::test]
async fn remove_is_refused_while_installed_plugins_depend_on_it() {
    let (app, dir) = kept().await;
    let base = plugin(dir.path(), "base", &manifest(ANOTHER, BASE, &[]));
    let one = plugin(
        dir.path(),
        "one",
        &depending(&manifest(PLUGIN, "One", &[]), ANOTHER, BASE, "^1", None),
    );
    const TWO: &str = "6a5b4c3d-2e1f-4a0b-9c8d-7e6f5a4b3c2d";
    let two = plugin(
        dir.path(),
        "two",
        &depending(&manifest(TWO, "Two", &[]), ANOTHER, BASE, "^1", None),
    );
    installs(&app, &base).await;
    installs(&app, &one).await;
    installs(&app, &two).await;
    let (status, said) = pressing(
        &app,
        &format!("/api/ui/plugins/{TWO}/enabled"),
        serde_json::json!({ "enabled": false }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{said}");

    let (status, said) = pressing(
        &app,
        &format!("/api/ui/plugins/{ANOTHER}/remove"),
        serde_json::json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{said}");
    assert!(
        said.contains("Base can't be removed while 2 plugins depend on it: One and Two."),
        "{said}"
    );
    assert_eq!(page(&app).await.installed.len(), 3);

    let (status, said) = pressing(
        &app,
        &format!("/api/ui/plugins/{FIXTURE}/remove"),
        serde_json::json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{said}");
}

/// A row names the ticked plugins that go with it when it is unticked: those
/// depending on it and on those in turn, and not one already unticked.
#[tokio::test]
async fn a_row_names_what_goes_with_it_when_unticked() {
    let (app, dir) = kept().await;
    let base = plugin(dir.path(), "base", &manifest(ANOTHER, BASE, &[]));
    let one = plugin(
        dir.path(),
        "one",
        &depending(&manifest(PLUGIN, "One", &[]), ANOTHER, BASE, "^1", None),
    );
    const TOP: &str = "6a5b4c3d-2e1f-4a0b-9c8d-7e6f5a4b3c2d";
    let top = plugin(
        dir.path(),
        "top",
        &depending(&manifest(TOP, "Top", &[]), PLUGIN, "One", "^1", None),
    );
    installs(&app, &base).await;
    installs(&app, &one).await;
    let view = installs(&app, &top).await;

    assert_eq!(row(&view, ANOTHER).dependents, ["One", "Top"]);
    assert_eq!(row(&view, PLUGIN).dependents, ["Top"]);
    assert!(row(&view, TOP).dependents.is_empty());

    let (status, said) = pressing(
        &app,
        &format!("/api/ui/plugins/{TOP}/enabled"),
        serde_json::json!({ "enabled": false }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{said}");
    let view: PluginsView = serde_json::from_str(&said).unwrap();
    assert_eq!(row(&view, ANOTHER).dependents, ["One"]);
}

const FIXTURES: &str = "../../web/tests/fixtures";

/// The settings a plugin under test declares: one field of each kind.
const SETTINGS: &str = "settings:\n\
                        \x20 - key: host\n    label: Host\n    kind: text\n    \
                        description: The GitLab instance, where it isn't gitlab.com.\n\
                        \x20 - key: cache\n    label: Cache size\n    kind: size\n\
                        \x20 - key: groups\n    label: Groups\n    kind: list\n\
                        \x20 - key: token\n    label: Token\n    kind: secret\n";

/// A plugin declaring [`SETTINGS`], installed.
async fn settled(app: &Router, dir: &Path) {
    let path = plugin(
        dir,
        "gitlab",
        &format!("{}{SETTINGS}", manifest(PLUGIN, "GitLab", &[])),
    );
    let view = installs(app, &path).await;
    assert_eq!(row(&view, PLUGIN).settings, 4);
}

async fn pane(app: &Router) -> (PluginSettingsView, String) {
    let answered = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("{PLUGINS}/{PLUGIN}/settings"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(answered.status(), StatusCode::OK);
    let bytes = answered.into_body().collect().await.unwrap().to_bytes();
    let said = String::from_utf8_lossy(&bytes).into_owned();
    (serde_json::from_str(&said).unwrap(), said)
}

async fn saves(app: &Router, fields: serde_json::Value) -> (PluginSettingsSaved, String) {
    let (status, said) = pressing(
        app,
        &format!("{PLUGINS}/{PLUGIN}/settings"),
        serde_json::json!({ "fields": fields }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{said}");
    (serde_json::from_str(&said).unwrap(), said)
}

fn value<'a>(view: &'a PluginSettingsView, key: &str) -> &'a PluginFieldValue {
    &view
        .fields
        .iter()
        .find(|field| field.key == key)
        .unwrap_or_else(|| panic!("{key} is drawn in {view:?}"))
        .value
}

/// A plugin's declared fields are drawn by kind, and a text, a size and a
/// list saved through the pane read back from it; saved again empty, each is
/// cleared.
#[tokio::test]
async fn a_text_field_round_trips_through_the_pane() {
    let (app, dir) = kept().await;
    settled(&app, dir.path()).await;

    let (view, _) = pane(&app).await;
    assert_eq!(view.name, "GitLab");
    assert_eq!(
        view.fields
            .iter()
            .map(|field| (field.key.as_str(), field.label.as_str()))
            .collect::<Vec<_>>(),
        [
            ("host", "Host"),
            ("cache", "Cache size"),
            ("groups", "Groups"),
            ("token", "Token")
        ],
    );
    assert_eq!(
        view.fields[0].description.as_deref(),
        Some("The GitLab instance, where it isn't gitlab.com.")
    );
    assert_eq!(
        value(&view, "host"),
        &PluginFieldValue::Text {
            value: String::new()
        }
    );
    assert_eq!(
        value(&view, "token"),
        &PluginFieldValue::Secret { set: false }
    );

    let (saved, _) = saves(
        &app,
        serde_json::json!([
            { "key": "host", "value": { "kind": "Text", "value": " gitlab.example.com " } },
            { "key": "cache", "value": { "kind": "Size", "value": "2G" } },
            { "key": "groups", "value": { "kind": "List", "values": ["infra", "", "web"] } },
        ]),
    )
    .await;
    assert!(saved.refused.is_empty(), "{saved:?}");

    let (view, _) = pane(&app).await;
    assert_eq!(view, saved.settings);
    assert_eq!(
        value(&view, "host"),
        &PluginFieldValue::Text {
            value: "gitlab.example.com".to_owned()
        }
    );
    assert_eq!(
        value(&view, "cache"),
        &PluginFieldValue::Size {
            value: "2G".to_owned()
        }
    );
    assert_eq!(
        value(&view, "groups"),
        &PluginFieldValue::List {
            values: vec!["infra".to_owned(), "web".to_owned()]
        }
    );

    saves(
        &app,
        serde_json::json!([{ "key": "host", "value": { "kind": "Text", "value": "" } }]),
    )
    .await;
    let (view, _) = pane(&app).await;
    assert_eq!(
        value(&view, "host"),
        &PluginFieldValue::Text {
            value: String::new()
        }
    );
    assert_eq!(
        value(&view, "cache"),
        &PluginFieldValue::Size {
            value: "2G".to_owned()
        },
        "a field the save left out is left as it is"
    );
}

/// A secret saved through the pane is written to `secrets.yaml` under the
/// plugin's uuid and never comes back: the pane says only that one is set, and
/// neither it nor the page carries the value. An Update keeps it.
#[tokio::test]
async fn a_secret_goes_to_secrets_yaml_and_never_back_to_the_viewer() {
    let (app, dir) = kept().await;
    settled(&app, dir.path()).await;

    let (saved, said) = saves(
        &app,
        serde_json::json!([
            { "key": "token", "value": { "kind": "Secret", "value": "glpat-hush" } },
        ]),
    )
    .await;
    assert!(saved.refused.is_empty(), "{saved:?}");
    assert!(!said.contains("glpat-hush"), "{said}");
    assert_eq!(
        value(&saved.settings, "token"),
        &PluginFieldValue::Secret { set: true }
    );

    let written = std::fs::read_to_string(dir.path().join("secrets.yaml")).unwrap();
    assert!(written.contains(PLUGIN), "{written}");
    assert_eq!(
        Settings::in_data_dir(dir.path())
            .secrets()
            .plugin_secrets_of(PLUGIN)
            .and_then(|secrets| secrets.get("token"))
            .map(String::as_str),
        Some("glpat-hush"),
    );

    let (_, said) = pane(&app).await;
    assert!(!said.contains("glpat-hush"), "{said}");
    let page = serde_json::to_string(&page(&app).await).unwrap();
    assert!(!page.contains("glpat-hush"), "{page}");

    updates(&app, PLUGIN, None).await;
    let (view, _) = pane(&app).await;
    assert_eq!(
        value(&view, "token"),
        &PluginFieldValue::Secret { set: true },
        "an Update keeps it"
    );

    saves(
        &app,
        serde_json::json!([{ "key": "token", "value": { "kind": "SecretCleared" } }]),
    )
    .await;
    let (view, _) = pane(&app).await;
    assert_eq!(
        value(&view, "token"),
        &PluginFieldValue::Secret { set: false }
    );
    assert_eq!(
        Settings::in_data_dir(dir.path())
            .secrets()
            .plugin_secrets_of(PLUGIN),
        None
    );
}

/// A size that is not a size is refused at its field, with the reason, and
/// nothing the save sent beside it is written.
#[tokio::test]
async fn a_size_that_is_not_a_size_is_refused_with_the_reason() {
    let (app, dir) = kept().await;
    settled(&app, dir.path()).await;

    let (saved, _) = saves(
        &app,
        serde_json::json!([
            { "key": "host", "value": { "kind": "Text", "value": "gitlab.example.com" } },
            { "key": "cache", "value": { "kind": "Size", "value": "2 gigs" } },
        ]),
    )
    .await;
    assert_eq!(
        saved.refused,
        [PluginFieldRefused {
            key: "cache".to_owned(),
            why: "2 gigs isn't a size. Write a whole number with K, M, G or T after it, such \
                  as 30G."
                .to_owned(),
        }],
    );

    let (view, _) = pane(&app).await;
    assert_eq!(
        value(&view, "host"),
        &PluginFieldValue::Text {
            value: String::new()
        },
        "a refused save writes nothing"
    );
}

/// The fields of a plugin in trouble are still drawn, and saved.
#[tokio::test]
async fn a_plugin_in_trouble_still_draws_its_fields() {
    let (app, dir) = kept().await;
    settled(&app, dir.path()).await;
    std::fs::write(
        dir.path().join("gitlab").join("plugin.yaml"),
        "uuid: broken\n",
    )
    .unwrap();
    updates(&app, PLUGIN, None).await;

    let (view, _) = pane(&app).await;
    assert!(view.trouble.is_some(), "{view:?}");
    assert_eq!(view.fields.len(), 4);

    let (saved, _) = saves(
        &app,
        serde_json::json!([{ "key": "host", "value": { "kind": "Text", "value": "h" } }]),
    )
    .await;
    assert!(saved.refused.is_empty());
}

/// Leave the viewer's tests the page and the consent card as the server writes
/// them: a plugin that loaded, one in trouble, one from git, one whose
/// dependency isn't installed, an installer's own language, the card for one asking for a host, and the
/// first one's settings pane with a text, a list and a secret set. The
/// directories are written as `/home/you/src`, so that a fixture is not
/// rewritten by where a test happened to run.
#[tokio::test]
async fn the_page_and_the_card_are_written_for_the_viewer() {
    let (app, dir) = kept().await;
    let path = plugin(
        dir.path(),
        "gitlab",
        &format!(
            "{}{SETTINGS}",
            manifest(PLUGIN, "GitLab", &["outbound: gitlab.com"])
        ),
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

    // And an installer's own language, which is a row under Installed from
    // this device's `config.yaml`.
    std::fs::write(
        Settings::in_data_dir(dir.path()).config_path(),
        "languages:\n  zig:\n    label: Zig\n    env:\n      \
         ZIG_GLOBAL_CACHE_DIR: \"{cache}/zig\"\n",
    )
    .unwrap();

    written(
        "plugins.json",
        serde_json::to_string(&page(&app).await).unwrap(),
    );
    written(
        "plugin-consent.json",
        serde_json::to_string(&consent).unwrap(),
    );

    saves(
        &app,
        serde_json::json!([
            { "key": "host", "value": { "kind": "Text", "value": "gitlab.example.com" } },
            { "key": "groups", "value": { "kind": "List", "values": ["infra", "web"] } },
            { "key": "token", "value": { "kind": "Secret", "value": "glpat-hush" } },
        ]),
    )
    .await;
    let (_, settings) = pane(&app).await;
    written("plugin-settings.json", settings);
}

/// **A Skill Pack attached to a Conversation** while its round drafts, the way
/// an MCP server is (ADR-0030, *Skill packs*): offered on the Attach menu once
/// an installed plugin's pack has loaded, kept as a chip read back from the
/// Conversation, and frozen once the round starts.
mod packs {
    use super::*;
    use sqlx::SqlitePool;
    use verkstead_render::{
        Added, AdditionMade, AdditionState, ConversationView, OfferedPack, PackAttached,
        PackRemoved, Registered, RepoEntry, SetReading, Started, Submitted,
    };
    use verkstead_server::store;

    /// A router over a Data Directory of its own, with the plugin providing
    /// `rust-review` installed and a Conversation drafting in a Repo.
    async fn drafting() -> (Router, tempfile::TempDir, SqlitePool, i64) {
        let dir = tempfile::tempdir().unwrap();
        let pool = open_database(&dir.path().join("verkstead.db"))
            .await
            .unwrap();
        let app = router_keeping(pool.clone(), dir.path().to_owned());

        let path = packing(dir.path(), PLUGIN, "Rust review", "rust-review");
        assert_eq!(row(&installs(&app, &path).await, PLUGIN).trouble, None);

        let repo = dir.path().join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        for args in [
            &["init", "--initial-branch", "main"][..],
            &["config", "user.email", "test@verkstead.invalid"],
            &["config", "user.name", "Verkstead Test"],
            &["commit", "--allow-empty", "-m", "first"],
        ] {
            let status = Command::new("git")
                .args(args)
                .current_dir(&repo)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .unwrap();
            assert!(status.success(), "git {args:?}");
        }

        let registered: Registered =
            answer(&app, "/api/ui/repos", serde_json::json!({ "path": repo })).await;
        assert!(matches!(registered, Registered::Added(_)), "{registered:?}");
        let repos: Vec<RepoEntry> = reading(&app, "/api/ui/repos").await;
        let started: Started = answer(
            &app,
            "/api/ui/conversations",
            serde_json::json!({ "repo_id": repos[0].id }),
        )
        .await;
        let Started::Started { id } = started else {
            panic!("the Conversation starts: {started:?}");
        };

        (app, dir, pool, id)
    }

    async fn answer<T: serde::de::DeserializeOwned>(
        app: &Router,
        path: &str,
        body: serde_json::Value,
    ) -> T {
        let (status, said) = pressing(app, path, body).await;
        assert_eq!(status, StatusCode::OK, "{said}");
        serde_json::from_str(&said).unwrap_or_else(|error| panic!("{error}: {said}"))
    }

    async fn reading<T: serde::de::DeserializeOwned>(app: &Router, path: &str) -> T {
        let answered = app
            .clone()
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(answered.status(), StatusCode::OK);
        let bytes = answered.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    async fn attach(app: &Router, id: i64, name: &str) -> PackAttached {
        answer(
            app,
            &format!("/api/ui/conversations/{id}/skill-packs/{name}"),
            serde_json::json!({}),
        )
        .await
    }

    async fn detach(app: &Router, id: i64, name: &str) -> PackRemoved {
        answer(
            app,
            &format!("/api/ui/conversations/{id}/skill-packs/{name}/remove"),
            serde_json::json!({}),
        )
        .await
    }

    /// The chips, as the composer draws them: each name, and whether it is
    /// still installed here.
    async fn held(app: &Router, id: i64) -> Vec<(String, bool)> {
        let view: ConversationView = reading(app, &format!("/api/ui/conversations/{id}")).await;
        view.skill_packs
            .into_iter()
            .map(|pack| (pack.name, pack.installed))
            .collect()
    }

    async fn offered(app: &Router) -> Vec<String> {
        let offered: Vec<OfferedPack> = reading(app, "/api/ui/skill-packs").await;
        offered.into_iter().map(|pack| pack.name).collect()
    }

    async fn enabled(app: &Router, enabled: bool) {
        let (status, said) = pressing(
            app,
            &format!("/api/ui/plugins/{PLUGIN}/enabled"),
            serde_json::json!({ "enabled": enabled }),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{said}");
    }

    #[tokio::test]
    async fn an_installed_pack_is_offered_attached_and_taken_off() {
        let (app, _dir, _pool, id) = drafting().await;

        let menu: Vec<OfferedPack> = reading(&app, "/api/ui/skill-packs").await;
        assert_eq!(
            menu,
            [OfferedPack {
                name: "rust-review".to_owned(),
                plugin: "Rust review".to_owned(),
                skills: vec!["reviewing".to_owned()],
            }],
            "the installed plugin's pack, and never the built-in verkstead"
        );
        assert!(held(&app, id).await.is_empty());

        assert_eq!(
            attach(&app, id, "rust-review").await,
            PackAttached::Attached
        );
        assert_eq!(
            attach(&app, id, "rust-review").await,
            PackAttached::Attached,
            "a second press is the state there is"
        );
        assert_eq!(held(&app, id).await, [("rust-review".to_owned(), true)]);

        assert_eq!(detach(&app, id, "rust-review").await, PackRemoved::Removed);
        assert!(held(&app, id).await.is_empty());
    }

    /// Nothing is attached that no loaded plugin provides — the built-in pack
    /// included, which every session has already.
    #[tokio::test]
    async fn a_pack_nothing_provides_is_refused() {
        let (app, _dir, _pool, id) = drafting().await;

        assert_eq!(
            attach(&app, id, "verkstead").await,
            PackAttached::NoSuchPack
        );
        assert_eq!(
            attach(&app, id, "go-review").await,
            PackAttached::NoSuchPack
        );
        assert!(held(&app, id).await.is_empty());
    }

    /// A disabled pack is not on the menu and cannot be attached; one disabled
    /// after it was attached keeps its chip, marked gone, until it is ticked
    /// again.
    #[tokio::test]
    async fn a_disabled_pack_is_off_the_menu_and_its_chip_is_gone() {
        let (app, _dir, _pool, id) = drafting().await;
        assert_eq!(
            attach(&app, id, "rust-review").await,
            PackAttached::Attached
        );

        enabled(&app, false).await;

        assert!(offered(&app).await.is_empty());
        assert_eq!(held(&app, id).await, [("rust-review".to_owned(), false)]);
        assert_eq!(
            attach(&app, id, "rust-review").await,
            PackAttached::NoSuchPack
        );

        enabled(&app, true).await;

        assert_eq!(offered(&app).await, ["rust-review"]);
        assert_eq!(held(&app, id).await, [("rust-review".to_owned(), true)]);
    }

    /// Fixed once the round starts, both ways.
    #[tokio::test]
    async fn the_packs_freeze_with_the_brief() {
        let (app, _dir, pool, id) = drafting().await;
        assert_eq!(
            attach(&app, id, "rust-review").await,
            PackAttached::Attached
        );

        store::set_state(&pool, id, store::Lifecycle::Implementing)
            .await
            .unwrap();

        assert_eq!(
            detach(&app, id, "rust-review").await,
            PackRemoved::NotDrafting
        );
        assert_eq!(held(&app, id).await, [("rust-review".to_owned(), true)]);
        assert_eq!(
            attach(&app, 9_999, "rust-review").await,
            PackAttached::NoSuchConversation
        );
    }

    /// Ask a two-Question Set from the Conversation, the ordinary way.
    async fn asked(pool: &SqlitePool, id: i64) -> i64 {
        let set = verkstead_schema::QuestionSet::from_yaml(
            "title: the rate limiter\n\
             questions:\n\
             \x20 - label: Q1\n\
             \x20   text: where does the counter live?\n",
        )
        .unwrap();

        store::ask(pool, id, &set, store::Ask::Blocking)
            .await
            .unwrap()
            .unwrap()
            .id
    }

    /// Pick pack `name` on the Set at Q1's menu.
    async fn pick(app: &Router, set: i64, name: &str) -> AdditionMade {
        answer(
            app,
            &format!("/api/ui/sets/{set}/additions"),
            serde_json::json!({ "label": "Q1", "pick": { "Pack": { "name": name } } }),
        )
        .await
    }

    async fn submit(app: &Router, set: i64) -> Submitted {
        answer(
            app,
            &format!("/api/ui/sets/{set}/response"),
            serde_json::json!({ "answers": [{ "label": "Q1", "free_text": "in Redis" }] }),
        )
        .await
    }

    /// ADR-0028's second moment for a pack: picked on an Answer after the
    /// round started, recorded pending, and put on the Conversation by the
    /// Submit — the Answer the agent reads naming it and where it is mounted,
    /// and the Timeline line saying it came with the Set.
    #[tokio::test]
    async fn a_pack_picked_on_an_answer_is_attached_by_the_submit() {
        let (app, _dir, pool, id) = drafting().await;
        store::set_state(&pool, id, store::Lifecycle::Implementing)
            .await
            .unwrap();
        let set = asked(&pool, id).await;

        let AdditionMade::Added { addition } = pick(&app, set, "rust-review").await else {
            panic!("the pick is recorded");
        };
        assert_eq!(
            addition.added,
            Added::Pack {
                name: "rust-review".to_owned(),
                installed: true,
            }
        );
        assert_eq!(addition.state, AdditionState::Pending);
        assert!(
            store::skill_packs(&pool, id).await.unwrap().is_empty(),
            "nothing is on the Conversation before the Submit"
        );

        assert_eq!(submit(&app, set).await, Submitted::Accepted);

        assert_eq!(
            store::skill_packs(&pool, id).await.unwrap(),
            ["rust-review"]
        );
        assert_eq!(held(&app, id).await, [("rust-review".to_owned(), true)]);

        let reading: SetReading = reading(&app, &format!("/api/ui/sets/{set}")).await;
        let SetReading::Set(view) = reading else {
            panic!("the Set reads back");
        };
        assert_eq!(view.additions[0].state, AdditionState::Applied);
        assert_eq!(
            view.packs,
            ["rust-review"],
            "and the menus leave it out now"
        );

        let response: String = {
            let answered = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri(format!(
                            "/conversations/{id}/api/v1/sets/{set}/response?hold=0"
                        ))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(answered.status(), StatusCode::OK);
            let bytes = answered.into_body().collect().await.unwrap().to_bytes();
            String::from_utf8(bytes.to_vec()).unwrap()
        };
        let response = verkstead_schema::Response::from_yaml(&response).unwrap();
        let added = &response.answers[0].added;
        assert_eq!(added.len(), 1, "{response:?}");
        assert!(
            added[0].contains("`rust-review`") && added[0].contains("/rust-review/`"),
            "the Answer names the pack and where it is mounted: {added:?}"
        );
    }

    /// A pack nothing provides is not picked, and one on the Conversation
    /// already is on already.
    #[tokio::test]
    async fn a_pack_nothing_provides_or_already_on_is_refused_on_a_set() {
        let (app, _dir, pool, id) = drafting().await;
        assert_eq!(
            attach(&app, id, "rust-review").await,
            PackAttached::Attached
        );
        let set = asked(&pool, id).await;

        assert_eq!(pick(&app, set, "go-review").await, AdditionMade::NoSuchPack);
        assert_eq!(pick(&app, set, "verkstead").await, AdditionMade::NoSuchPack);
        assert_eq!(
            pick(&app, set, "rust-review").await,
            AdditionMade::AlreadyOn
        );
    }

    /// A pack disabled between the pick and the Submit refuses the Submit by
    /// name, and nothing is stored: the Set still waits, and the chip says it
    /// is gone. Ticked again, the same Submit goes through.
    #[tokio::test]
    async fn a_pack_no_longer_enabled_refuses_the_submit() {
        let (app, _dir, pool, id) = drafting().await;
        let set = asked(&pool, id).await;

        assert!(matches!(
            pick(&app, set, "rust-review").await,
            AdditionMade::Added { .. }
        ));
        enabled(&app, false).await;

        assert_eq!(
            submit(&app, set).await,
            Submitted::PackGone("rust-review".to_owned())
        );
        assert!(store::settlement(&pool, set).await.unwrap().is_none());
        assert!(store::skill_packs(&pool, id).await.unwrap().is_empty());

        let reading: SetReading = reading(&app, &format!("/api/ui/sets/{set}")).await;
        let SetReading::Set(view) = reading else {
            panic!("the Set reads back");
        };
        assert_eq!(
            view.additions[0].added,
            Added::Pack {
                name: "rust-review".to_owned(),
                installed: false,
            }
        );

        enabled(&app, true).await;
        assert_eq!(submit(&app, set).await, Submitted::Accepted);
        assert_eq!(
            store::skill_packs(&pool, id).await.unwrap(),
            ["rust-review"]
        );
    }
}

/// An Agent plugin installed from a directory puts its harness on the list the
/// Profile form and the pickers draw — under its plugin's uuid, its own name
/// and the mark it ships, with a field apiece for the paths it declares — and
/// a Profile of it saved through that form is bound here and runnable.
#[tokio::test]
async fn an_agent_plugin_installed_from_a_directory_is_a_harness_a_profile_is_saved_under() {
    use crate::stub_harness::{self, StubHarness};
    use verkstead_render::{HarnessEntry, ProfileEntry, ProfileSaved};

    let (app, dir) = kept().await;
    let harness = StubHarness::Silent;

    harness
        .installed(&app, dir.path(), ["stub-large", "stub-grilling"])
        .await;

    let listed: Vec<HarnessEntry> = got(&app, "/api/ui/harnesses").await;
    let entry = listed
        .iter()
        .find(|entry| entry.id.as_str() == harness.uuid())
        .unwrap_or_else(|| panic!("the stub harness is on the list: {listed:?}"));

    assert_eq!(entry.name, "Stub Harness");
    assert_eq!(
        entry.mark,
        stub_harness::MARK,
        "drawn with the mark it ships"
    );
    assert_eq!(
        entry
            .paths
            .iter()
            .map(|path| (path.key.as_str(), path.label.as_str(), path.file))
            .collect::<Vec<_>>(),
        [
            ("dir", "Stub directory", false),
            ("config", "Stub config file", true)
        ],
        "a field apiece for the paths it declares, labelled as it labels them"
    );
    assert_eq!(
        listed.first().map(|entry| entry.id.as_str()),
        Some("Claude"),
        "after the built-ins, which keep their places"
    );

    let (status, said) = pressing(
        &app,
        "/api/ui/profiles",
        serde_json::json!({
            "name": "Stubbed",
            "account": {
                "agent_type": harness.uuid(),
                "paths": stub_harness::account(&dir.path().join("account")),
            },
            "models": ["stub-large"],
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{said}");
    assert_eq!(
        serde_json::from_str::<ProfileSaved>(&said).unwrap(),
        ProfileSaved::Saved
    );

    let profiles: Vec<ProfileEntry> = got(&app, "/api/ui/profiles").await;
    let profile = profiles
        .iter()
        .find(|profile| profile.name.as_deref() == Some("Stubbed"))
        .expect("the Profile is on the list");

    assert_eq!(profile.account.agent_type.as_str(), harness.uuid());
    assert_eq!(
        profile.account.paths.keys().collect::<Vec<_>>(),
        ["config", "dir"],
        "both declared paths are kept"
    );
    assert!(profile.bound, "bound on this device, which saved it");
    assert_eq!(profile.broken, None, "and nothing in the way of running it");
}

async fn got<T: serde::de::DeserializeOwned>(app: &Router, path: &str) -> T {
    let answered = app
        .clone()
        .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(answered.status(), StatusCode::OK);
    let bytes = answered.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}
