//! The ground the rest of the server stands on: it opens its database, it
//! answers a health check, and it can be pointed somewhere other than the
//! defaults.

use std::net::{IpAddr, Ipv4Addr};
use std::path::{Path, PathBuf};

use axum::body::Body;
use axum::http::{Request, StatusCode};
use clap::Parser;
use http_body_util::BodyExt;
use tower::ServiceExt;
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
    assert_eq!(
        database(Path::new("/srv/verkstead")).to_str().unwrap(),
        "/srv/verkstead/verkstead.db",
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
    let separated = Config::parse_from([
        "verkstead serve",
        "--watched-path",
        "/srv/repos:/srv/scratch",
    ]);

    assert_eq!(repeated.watched_paths, separated.watched_paths);
    assert_eq!(
        repeated.watched_paths,
        [PathBuf::from("/srv/repos"), PathBuf::from("/srv/scratch")]
    );
}
