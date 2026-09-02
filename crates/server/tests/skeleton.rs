//! The ground the rest of the server stands on: it opens its database, it
//! answers a health check, and it can be pointed somewhere other than the
//! defaults.

use std::ffi::OsStr;
use std::net::{IpAddr, Ipv4Addr};
use std::path::{Path, PathBuf};

use axum::body::Body;
use axum::http::{Request, StatusCode};
use clap::Parser;
use http_body_util::BodyExt;
use tower::ServiceExt;
use verkstead_server::platform::{Environment, Platform, default_log_dir, log_dir};
use verkstead_server::{Config, database, open_database, router};

#[tokio::test]
async fn opening_the_database_creates_a_missing_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state/verkstead.db");
    assert!(!path.exists());

    let _pool = open_database(&path).await.unwrap();

    assert!(path.exists(), "expected {} to be created", path.display());
}

#[tokio::test]
async fn opening_the_database_reuses_an_existing_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("verkstead.db");

    let pool = open_database(&path).await.unwrap();
    sqlx::query("CREATE TABLE marker (id INTEGER PRIMARY KEY)")
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;

    let pool = open_database(&path).await.unwrap();
    sqlx::query("SELECT id FROM marker")
        .fetch_optional(&pool)
        .await
        .expect("reopening the database should find the existing table");
}

#[tokio::test]
async fn health_route_answers_ok() {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    let response = router(pool)
        .oneshot(
            Request::builder()
                .uri("/api/v1/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&body[..], b"ok");
}

#[test]
fn config_defaults_to_localhost() {
    let config = Config::parse_from(["verkstead serve", "--watched-path", "/srv/repos"]);

    assert_eq!(config.listen.ip(), IpAddr::V4(Ipv4Addr::LOCALHOST));

    // And nothing at all for the Data Directory, which is the flag holding what
    // was said rather than where that resolves to: the platform's own directory
    // is what a run with nothing said gets, and where that is is resolved at
    // startup — see `verkstead_server::platform`.
    assert_eq!(config.data_dir, None);
}

#[test]
fn config_is_overridable_by_flag() {
    let config = Config::parse_from([
        "verkstead serve",
        "--listen",
        "0.0.0.0:9999",
        "--data-dir",
        "/srv/verkstead",
        "--watched-path",
        "/srv/repos",
    ]);

    assert_eq!(config.listen.to_string(), "0.0.0.0:9999");
    assert_eq!(
        config.data_dir.as_deref(),
        Some(Path::new("/srv/verkstead"))
    );
    // Spelled as a join rather than as a string, because the separator between
    // the directory and the name is the platform's: a literal `/` here asserts
    // that Verkstead is on Unix rather than that the name is inside the
    // directory.
    assert_eq!(
        database(Path::new("/srv/verkstead")),
        Path::new("/srv/verkstead").join("verkstead.db"),
        "the database is that one name inside whichever directory won",
    );
}

/// Configuration with no default and no requirement either: what Verkstead may
/// touch is the machine owner's to say, and a guess at it would be a guess at a
/// security boundary — but a standalone install says it on the settings page
/// rather than in flags, so a server given none of them here parses and comes
/// up watching nothing.
#[test]
fn config_parses_without_a_watched_path_and_watches_nothing() {
    let config = Config::parse_from(["verkstead serve"]);

    assert!(config.watched_paths.is_empty());
}

/// Several of them, as `PATH` is written — which is how they arrive from a
/// service unit, where there is one string and not a repeatable flag.
#[test]
fn watched_paths_are_a_list_however_they_are_given() {
    let repeated = Config::parse_from([
        "verkstead serve",
        "--watched-path",
        "/srv/repos",
        "--watched-path",
        "/srv/scratch",
    ]);
    // Written the way the platform writes `PATH`, which is what the flag is
    // parsed with: `join_paths` puts a `:` between them on Unix and a `;` on
    // Windows, where a literal `:` would be a drive letter's punctuation and
    // would leave the two directories as one string nobody split.
    let together = std::env::join_paths(["/srv/repos", "/srv/scratch"])
        .expect("two plain directories go into one list");
    let separated = Config::parse_from([
        OsStr::new("verkstead serve"),
        OsStr::new("--watched-path"),
        together.as_os_str(),
    ]);

    assert_eq!(repeated.watched_paths, separated.watched_paths);
    assert_eq!(
        repeated.watched_paths,
        [PathBuf::from("/srv/repos"), PathBuf::from("/srv/scratch")]
    );
}

/// The Log Directory, asked for the way stage 02's desktop binary will ask —
/// from outside this crate, where it is the only caller there is ever going to
/// be. Nothing in the server turns on the answer: it goes on logging to stdout,
/// and the directory stands empty and uncreated until there is a binary with a
/// log file to open in it.
#[test]
fn the_log_directory_is_reachable_from_another_crate() {
    let env = Environment {
        home: Some(PathBuf::from("/home/you")),
        ..Environment::default()
    };

    assert_eq!(
        default_log_dir(Platform::Linux, &env),
        Some(PathBuf::from("/home/you/.local/state/verkstead")),
    );

    // And the read of the real environment, which is what that binary calls.
    // Whether this machine answers at all is the machine's business — nowhere
    // to resolve to is an answer of nothing rather than a failure of anything —
    // but an answer is a path the platform named, so it is absolute.
    if let Some(dir) = log_dir() {
        assert!(
            dir.is_absolute(),
            "{} is where a log file would go, so it cannot depend on the \
             directory the app was launched from",
            dir.display(),
        );
    }
}
