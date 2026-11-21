//! The `PATH` a session gets is the `PATH` the server itself was started with,
//! read once — asked of the real process rather than of a stated machine.
//!
//! **One test in a binary of its own, and that is the whole design of this
//! file.** What is being asserted is that the read happens once and that both
//! the sandbox builder and the onboarding probes are handed what it read, and
//! the only way to show a value has stopped following the environment is to
//! move the environment and look again. `std::env::set_var` is `unsafe` under
//! this edition because it races every other thread in the process reading one
//! — so there is one test here, and no other thread to race.
//!
//! Everything else about the composing is a unit test beside the function that
//! does it, where a `PATH` and a home are values a test hands over: see
//! `sandbox::composed`, which is what this file is the one process-level
//! reading of.
//!
//! Unix only. What is set here is a `HOME` and a colon-separated `PATH`, and
//! the Windows arm neither composes nor reads either — see the same module's
//! `a_windows_session_is_given_the_machines_own_path_rather_than_a_list`.
#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;
use verkstead_render::{AgentType, Dependency, DependencyState, OnboardingView};
use verkstead_server::platform::Platform;
use verkstead_server::{open_database, router_keeping, sandbox};

/// A `claude` that is there and does nothing, which is the whole of what a row
/// probed by a `PATH` walk asks of one.
const A_PROGRAM: &str = "#!/bin/sh\nexit 0\n";

/// Where the vendor's own installer puts it, under the home of whoever it was
/// run by — which is the install this whole feature is about.
const NATIVE_INSTALL: &str = ".local/bin";

/// A harness installed under the server's own home is what a session finds and
/// what the wizard's row says is there — and both go on saying so once the
/// environment has moved, because the `PATH` was read at startup and is held.
#[tokio::test]
async fn the_servers_path_is_read_once_and_both_the_probes_and_a_session_get_it() {
    let home = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();

    let local = home.path().join(NATIVE_INSTALL);
    std::fs::create_dir_all(&local).unwrap();
    program(&local.join("claude"), A_PROGRAM);

    // Before anything has asked: this is the environment the server is being
    // started in, and the reading below is the one read of it.
    //
    // Safe here for the reason the module says: this is the only test in this
    // binary, so there is no other thread in the process to be reading an
    // environment while it is written.
    unsafe {
        std::env::set_var("HOME", home.path());
        std::env::set_var("PATH", format!("{}:/usr/bin", local.display()));
    }

    let machine_path = sandbox::machine_path(Platform::HERE);
    let entries: Vec<String> = machine_path
        .to_string_lossy()
        .split(':')
        .map(str::to_owned)
        .collect();

    assert_eq!(
        entries.first().map(String::as_str),
        local.to_str(),
        "the human's own install is the first place a session looks: {entries:?}",
    );
    assert!(
        entries.iter().any(|entry| entry == "/usr/bin"),
        "with the machine's own under it: {entries:?}",
    );

    let pool = open_database(&data.path().join("verkstead.db"))
        .await
        .unwrap();
    let app = router_keeping(pool, data.path().to_owned());

    let install = DependencyState::Present {
        at: Some(local.join("claude").to_string_lossy().into_owned()),
        target: None,
    };

    assert_eq!(
        claude(&reading(&app).await),
        install,
        "and the wizard's row is that same list walked, so it says a session \
         would find it — and says it is the human's own install rather than \
         whatever else answers to the name",
    );

    // And now the environment moves out from under both of them — a `PATH`
    // exported in a shell after Verkstead was started, which is the case the
    // wizard's own instructions tell a human to restart the server for.
    unsafe {
        std::env::set_var("HOME", "/nonexistent");
        std::env::set_var("PATH", "/usr/bin");
    }

    assert_eq!(
        sandbox::machine_path(Platform::HERE),
        machine_path,
        "the `PATH` a session gets was read at startup and is held: a server \
         whose environment moved goes on starting the sessions it started",
    );
    assert_eq!(
        claude(&reading(&app).await),
        install,
        "and the probes are the same value, so the row cannot come to disagree \
         with what a session would find",
    );
}

/// What the Claude row of a reading says.
fn claude(reading: &OnboardingView) -> DependencyState {
    reading
        .dependencies
        .iter()
        .find(|row| row.dependency == Dependency::Harness(AgentType::CLAUDE))
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
