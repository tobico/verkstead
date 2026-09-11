//! A directory Verkstead installs into stays on the session `PATH` — asked of
//! the real process rather than of a stated machine.
//!
//! `session_path` in `config.yaml` is the list a session's `PATH` is composed
//! with **ahead of the server's own entries**, so that a harness the wizard
//! installed is the one a session finds rather than whatever older copy the
//! distribution's packages hold. It is read at startup beside the server's own
//! `PATH` and held for the run, and the one thing that moves it afterwards is an
//! install landing in a directory.
//!
//! **One test in a binary of its own, and that is the whole design of this
//! file** — `tests/servers_path.rs`'s design, for its reason. What is asserted
//! is about values the *process* holds: the `PATH` the server was started with,
//! and the list beside it. Standing a machine up whose server has no
//! `~/.local/bin` on its `PATH` means moving the environment, and
//! `std::env::set_var` is `unsafe` under this edition because it races every
//! other thread in the process reading one. So there is one test here, and no
//! other thread to race.
//!
//! Everything else about the composing is a unit test beside the function that
//! does it, where a `PATH`, a home and a configured list are values a test hands
//! over: see `sandbox::composed` and `sandbox::kept_entries`. And what a session
//! is *granted* of such a directory is `tests/per_user_path.rs`, which runs one.
//!
//! Unix only, for `tests/servers_path.rs`'s reason: what is written here is a
//! `HOME` and a colon-separated `PATH`, and the Windows arm reads neither.
#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;
use verkstead_render::{Dependency, DependencyState, OnboardingView};
use verkstead_server::platform::Platform;
use verkstead_server::settings::Settings;
use verkstead_server::{open_database, router_keeping, sandbox};

/// A harness that is there and does nothing, which is the whole of what a row
/// probed by a `PATH` walk asks of one.
const A_PROGRAM: &str = "#!/bin/sh\nexit 0\n";

/// Where the vendor's own installer puts one, under the home of whoever ran it
/// — and where the wizard's own run of it will land one.
const NATIVE_INSTALL: &str = ".local/bin";

/// And where the second install of the run lands, which is what proves the held
/// list can grow: xAI's installer keeps its own directory.
const GROK_INSTALL: &str = ".grok/bin";

/// A directory that is nobody's home and no part of the machine's own toolchain
/// — an `/opt/something/bin`, which is somewhere a session could not reach
/// whatever a file says about it.
const OUT_OF_REACH: &str = "/opt/foo/bin";

/// The wizard's Claude row reads Present at a directory only `session_path` put
/// on the list, a session's `PATH` leads with that directory, and an install
/// that lands mid-run is on the next probe with no restart.
#[tokio::test]
async fn what_verkstead_installed_is_on_a_sessions_path_and_grows_within_the_run() {
    let home = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();

    let local = home.path().join(NATIVE_INSTALL);
    std::fs::create_dir_all(&local).unwrap();
    program(&local.join("claude"), A_PROGRAM);

    // The machine this server is coming up on: a `PATH` with the system on it
    // and nothing of the human's, which is what a fresh install has. The
    // directory the harness is in is on no `PATH` at all — only `config.yaml`
    // says it, which is the whole of what is being asked here.
    //
    // Safe for the reason the module says: this is the only test in this binary,
    // so there is no other thread in the process to be reading an environment
    // while it is written.
    unsafe {
        std::env::set_var("HOME", home.path());
        std::env::set_var("PATH", "/usr/bin");
    }

    // And what Verkstead was told, written the way a hand-edit would write it:
    // the directory an install landed in, and one no session could reach.
    let settings = Settings::in_data_dir(data.path());
    std::fs::write(
        settings.config_path(),
        format!(
            "session_path:\n  - {}\n  - {OUT_OF_REACH}\n",
            local.display()
        ),
    )
    .unwrap();

    // Startup: the read the server makes as it comes up, before a router is
    // built or a session is spawned.
    sandbox::hold_session_path(&settings);

    let entries = looks_in(&sandbox::machine_path(Platform::HERE));

    assert_eq!(
        entries.first().map(String::as_str),
        local.to_str(),
        "the directory Verkstead installed into is the first place a session \
         looks, ahead of the system ones: {entries:?}",
    );
    assert!(
        entries.iter().any(|entry| entry == "/usr/bin"),
        "with the machine's own under it: {entries:?}",
    );
    assert!(
        !entries.iter().any(|entry| entry == OUT_OF_REACH),
        "and a configured directory under neither the home nor the machine's \
         own floor is dropped, a session being granted no such place to look \
         in: {entries:?}",
    );

    let pool = open_database(&data.path().join("verkstead.db"))
        .await
        .unwrap();
    let app = router_keeping(pool, data.path().to_owned());

    assert_eq!(
        row(&reading(&app).await, Dependency::Claude),
        DependencyState::Present {
            at: Some(local.join("claude").to_string_lossy().into_owned()),
            target: None,
        },
        "so the wizard's row is that same list walked: a harness on no `PATH` \
         but the configured one is a harness a session would find",
    );
    assert!(
        reading(&app)
            .await
            .path
            .iter()
            .any(|entry| Some(entry.as_str()) == local.to_str()),
        "and the list the wizard draws is where a session looks, the configured \
         entry among them with nothing else to do",
    );

    // And now an install lands inside the run — the wizard's own Next, a task
    // or two from here, which is the one thing that moves the held list.
    let grok = home.path().join(GROK_INSTALL);
    std::fs::create_dir_all(&grok).unwrap();
    program(&grok.join("grok"), A_PROGRAM);

    let installed = DependencyState::Present {
        at: Some(grok.join("grok").to_string_lossy().into_owned()),
        target: None,
    };

    assert_ne!(
        row(&reading(&app).await, Dependency::Grok),
        installed,
        "which nothing has said yet: the directory it landed in is on no list, \
         so this is not the file a session would find — whatever else of the \
         name the machine's own floor may hold",
    );

    sandbox::installed_into(&settings, &grok).unwrap();

    assert_eq!(
        row(&reading(&app).await, Dependency::Grok),
        installed,
        "the next probe is composed with it — the same server, the same router \
         and nothing restarted",
    );

    let entries = looks_in(&sandbox::machine_path(Platform::HERE));

    assert_eq!(
        entries
            .iter()
            .position(|entry| Some(entry.as_str()) == grok.to_str()),
        Some(1),
        "and the next session's `PATH` names it, behind the directory that was \
         configured before it and ahead of the system ones: {entries:?}",
    );

    assert_eq!(
        settings.config().session_path(),
        [
            local.to_string_lossy().into_owned(),
            OUT_OF_REACH.to_owned(),
            grok.to_string_lossy().into_owned(),
        ],
        "and it is in `config.yaml`, so the next start reads it back — the \
         unreachable entry still written down, that being the human's own word \
         to correct rather than Verkstead's to delete",
    );
}

/// The directories a `PATH` names, in the order a session searches them.
fn looks_in(path: &std::ffi::OsStr) -> Vec<String> {
    path.to_string_lossy()
        .split(':')
        .map(str::to_owned)
        .collect()
}

/// What one row of a reading says.
fn row(reading: &OnboardingView, dependency: Dependency) -> DependencyState {
    reading
        .dependencies
        .iter()
        .find(|row| row.dependency == dependency)
        .expect("every dependency is a row")
        .state
        .clone()
}

/// The reading the wizard is drawn from, over the viewer's own namespace.
async fn reading(app: &Router) -> OnboardingView {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/ui/onboarding")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();

    serde_json::from_slice(&body).expect("the reading the wizard is drawn from")
}

/// A file that is there and runnable, which is what a `PATH` walk is looking
/// for.
fn program(path: &Path, contents: &str) {
    std::fs::write(path, contents).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
}
