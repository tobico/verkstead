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
    let config = Config::parse_from(["verkstead serve"]);

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
    ]);

    assert_eq!(config.listen.to_string(), "0.0.0.0:9999");
    assert_eq!(
        config.data_dir.as_deref(),
        Some(Path::new("/srv/verkstead"))
    );
    // Said as the two halves rather than as one spelling, because a separator
    // is the platform's: the same join reads `/srv/verkstead/verkstead.db` on
    // Unix and `/srv/verkstead\verkstead.db` on Windows, and neither of those
    // is what is being asserted. That one name inside whichever directory won
    // is, and a literal here would be this test asserting which platform it is
    // running on instead.
    let database = database(Path::new("/srv/verkstead"));
    assert_eq!(
        database.file_name().unwrap(),
        "verkstead.db",
        "the database is that one name",
    );
    assert_eq!(
        database.parent().unwrap(),
        Path::new("/srv/verkstead"),
        "inside whichever directory won",
    );
}

/// A bare `verkstead serve` is the whole of what a standalone install is
/// started with: nothing says where Verkstead may work, because nothing bounds
/// it any more, and a sandbox is given nothing beyond what it always had.
#[test]
fn config_parses_with_nothing_said_at_all() {
    let config = Config::parse_from(["verkstead serve"]);

    assert!(config.sandbox_binds.is_empty());
}

/// Several binds, as `PATH` is written — which is how they arrive from a
/// service unit, where there is one string and not a repeatable flag.
///
/// The one string is built rather than written out, because how `PATH` is
/// written is the platform's own — a `:` on Unix, a `;` on Windows, which is
/// what the flag is parsed with; see `PATH_LIST_SEPARATOR` in `lib.rs`. A
/// literal `:` here would be asserting that Windows cuts a drive letter off
/// the path it belongs to.
#[test]
fn sandbox_binds_are_a_list_however_they_are_given() {
    let repeated = Config::parse_from([
        "verkstead serve",
        "--sandbox-bind",
        "/var/cache/node",
        "--sandbox-bind",
        "/var/cache/cargo",
    ]);
    let one_string = std::env::join_paths(["/var/cache/node", "/var/cache/cargo"])
        .unwrap()
        .into_string()
        .unwrap();
    let separated = Config::parse_from(["verkstead serve", "--sandbox-bind", &one_string]);

    assert_eq!(repeated.sandbox_binds, separated.sandbox_binds);
    assert_eq!(
        repeated.sandbox_binds,
        ["/var/cache/node".to_owned(), "/var/cache/cargo".to_owned()]
    );
}

/// The Log Directory, asked from outside this crate the way anything asking it
/// would. Nothing in the server turns on the answer: it goes on logging to
/// stdout, and the directory stands empty and uncreated until something with a
/// log file opens one in it — which is the desktop app, and it resolves the
/// same three arms in TypeScript rather than calling this. So what is held
/// here is the answer itself: the arms are what they say, and the read of a
/// real machine is a path that machine named.
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

    // And the read of the real environment, which is the answer this machine
    // gives.
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
