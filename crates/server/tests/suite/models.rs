//! What an account can run, asked of its harness over the viewer's namespace:
//! `POST /api/ui/models`.
//!
//! OpenCode is the harness with a listing, so what stands on the `PATH` here is
//! an `opencode` of the test's own — a script printing what opencode 2.4.2
//! prints, and saying back which account it was given — rather than whatever
//! the runner happens to have installed.

#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use axum::Router;
use axum::body::Body;
use axum::http::{Request, header};
use http_body_util::BodyExt;
use tower::ServiceExt;
use verkstead_render::ModelsListed;
use verkstead_server::onboarding::Machine;
use verkstead_server::platform::{Environment, Platform};
use verkstead_server::{open_database, router_onboarding};

/// The app over a machine whose `PATH` is `bin` alone, with the database and
/// everything else under `dir`.
async fn workbench(dir: &Path, bin: &Path) -> Router {
    let pool = open_database(&dir.join("verkstead.db")).await.unwrap();

    let machine = Machine::stated(
        Platform::Linux,
        bin.as_os_str().to_owned(),
        bin.as_os_str().to_owned(),
        None,
        None,
        &Environment {
            home: Some(dir.to_owned()),
            ..Environment::default()
        },
    );

    router_onboarding(pool, dir.join("data"), machine)
}

/// A directory holding an `opencode` that runs `script`.
fn standing(dir: &Path, script: &str) -> PathBuf {
    let bin = dir.join("bin");
    std::fs::create_dir_all(&bin).unwrap();

    let at = bin.join("opencode");
    std::fs::write(&at, format!("#!/bin/sh\n{script}\n")).unwrap();
    std::fs::set_permissions(&at, std::fs::Permissions::from_mode(0o755)).unwrap();

    bin
}

/// An OpenCode home with the two directories opencode keeps an account in.
fn opencode_home(dir: &Path) -> PathBuf {
    let home = dir.join("accounts").join("free");
    std::fs::create_dir_all(home.join(".config/opencode")).unwrap();
    std::fs::create_dir_all(home.join(".local/share/opencode")).unwrap();
    home
}

/// What the endpoint says of `account`.
async fn listed(app: &Router, account: serde_json::Value) -> ModelsListed {
    let response = app
        .clone()
        .oneshot(
            Request::post("/api/ui/models")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(account.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    let body = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&body).unwrap()
}

/// An OpenCode account's models are what `opencode models --standalone` prints
/// for it, given that account's own XDG directories, in its order.
#[tokio::test]
async fn an_opencode_accounts_models_are_what_opencode_lists_for_it() {
    let dir = tempfile::tempdir().unwrap();
    let home = opencode_home(dir.path());

    // Lists only when asked for a private server and given this account's
    // configuration, so that a listing of anybody else's reads as a refusal.
    let bin = standing(
        dir.path(),
        &format!(
            "[ \"$1 $2\" = 'models --standalone' ] || exit 2\n\
             [ \"$XDG_CONFIG_HOME\" = '{config}' ] || exit 3\n\
             [ \"$XDG_DATA_HOME\" = '{data}' ] || exit 4\n\
             echo ollama/qwen3-coder:30b\n\
             echo opencode/big-pickle\n\
             echo opencode/beam-free",
            config = home.join(".config").display(),
            data = home.join(".local/share").display(),
        ),
    );

    let app = workbench(dir.path(), &bin).await;

    assert_eq!(
        listed(
            &app,
            serde_json::json!({ "agent_type": "OpenCode", "home": home })
        )
        .await,
        ModelsListed::Listed(vec![
            "ollama/qwen3-coder:30b".to_owned(),
            "opencode/big-pickle".to_owned(),
            "opencode/beam-free".to_owned(),
        ]),
    );
}

/// An opencode with no `--standalone` — every 1.x release — is asked again
/// without it.
#[tokio::test]
async fn an_opencode_without_standalone_is_asked_again_without_it() {
    let dir = tempfile::tempdir().unwrap();
    let home = opencode_home(dir.path());

    let bin = standing(
        dir.path(),
        "[ \"$2\" = '--standalone' ] && { echo 'Unknown argument: standalone' >&2; exit 1; }\n\
         echo opencode/gpt-5.1-codex",
    );

    let app = workbench(dir.path(), &bin).await;

    assert_eq!(
        listed(
            &app,
            serde_json::json!({ "agent_type": "OpenCode", "home": home })
        )
        .await,
        ModelsListed::Listed(vec!["opencode/gpt-5.1-codex".to_owned()]),
    );
}

/// A listing that fails both ways is the last thing opencode said, as a
/// sentence.
#[tokio::test]
async fn an_opencode_that_cannot_list_says_why() {
    let dir = tempfile::tempdir().unwrap();
    let home = opencode_home(dir.path());

    let bin = standing(
        dir.path(),
        "echo 'Error: config is not valid JSON.' >&2\nexit 1",
    );

    let app = workbench(dir.path(), &bin).await;

    assert_eq!(
        listed(
            &app,
            serde_json::json!({ "agent_type": "OpenCode", "home": home })
        )
        .await,
        ModelsListed::Refused(
            "OpenCode couldn't list its models: Error: config is not valid JSON.".to_owned()
        ),
    );
}

/// Nothing is asked where there is nothing to ask: a harness with no listing,
/// an account that would not save, or no opencode on this machine.
#[tokio::test]
async fn nothing_is_asked_where_there_is_no_listing_to_ask() {
    let dir = tempfile::tempdir().unwrap();
    let home = opencode_home(dir.path());
    let bin = standing(dir.path(), "echo opencode/big-pickle");

    let app = workbench(dir.path(), &bin).await;

    let codex = dir.path().join("accounts").join(".codex");
    std::fs::create_dir_all(&codex).unwrap();

    assert_eq!(
        listed(
            &app,
            serde_json::json!({ "agent_type": "Codex", "home": codex })
        )
        .await,
        ModelsListed::Unasked,
        "Codex has no listing",
    );

    assert_eq!(
        listed(
            &app,
            serde_json::json!({ "agent_type": "OpenCode", "home": dir.path().join("nowhere") }),
        )
        .await,
        ModelsListed::Unasked,
        "a home that would not save",
    );

    let empty = dir.path().join("empty");
    std::fs::create_dir_all(&empty).unwrap();
    let bare = workbench(&dir.path().join("bare"), &empty).await;

    assert_eq!(
        listed(
            &bare,
            serde_json::json!({ "agent_type": "OpenCode", "home": home })
        )
        .await,
        ModelsListed::Unasked,
        "no opencode on this machine",
    );
}
