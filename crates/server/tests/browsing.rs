//! Browsing the filesystem over the viewer's namespace: what one directory
//! hands back, and what each of the two scopes will and will not look at.
//!
//! Asked of the *server*, through the endpoint, for the reason registering a
//! Repo is asked that way in `tests/repos.rs`: the Watched Paths are a security
//! boundary, and the scope that is bounded by them has to refuse a path a
//! browser never went near a dropdown to ask about.
//!
//! Every refusal here is a 200 with a named outcome. A field is typed into a
//! character at a time, so a path that is relative, missing or outside the
//! boundary is the ordinary state of one halfway through a word — something the
//! dropdown draws a line about, rather than an error to report.

use std::path::Path;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde::de::DeserializeOwned;
use tower::ServiceExt;
use verkstead_render::{DirectoryEntry, DirectoryListing, EntryKind};
use verkstead_server::{WatchedPaths, open_database, router_watching};

/// A router watching `watched`, plus the directory holding its database and its
/// `config.yaml` alive.
async fn app_watching(watched: &[&Path]) -> (tempfile::TempDir, Router) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    let watched = WatchedPaths::resolve(
        &watched
            .iter()
            .map(|path| (*path).to_owned())
            .collect::<Vec<_>>(),
    )
    .unwrap();

    let data_dir = dir.path().to_owned();

    (dir, router_watching(pool, watched, data_dir))
}

/// Write `config.yaml` in `data_dir` saying `paths` are the Watched Paths — the
/// half of the boundary the human says on the settings page, which the endpoint
/// reads at every ask.
fn watch_in_the_settings(data_dir: &Path, paths: &[&Path]) {
    let mut yaml = String::from("watched_paths:\n");
    for path in paths {
        yaml.push_str(&format!("  - {}\n", path.display()));
    }

    std::fs::write(data_dir.join("config.yaml"), yaml).unwrap();
}

/// What the dropdown would be filled from: one directory, asked in one scope.
async fn browse(app: &Router, scope: &str, path: Option<&Path>) -> DirectoryListing {
    let query = match path {
        Some(path) => format!("?scope={scope}&path={}", encoded(path)),
        None => format!("?scope={scope}"),
    };

    get(app, &format!("/api/ui/directories{query}")).await
}

/// A path as a query value. Enough of an encoding for what these tests name:
/// temporary directories and the words written under them.
fn encoded(path: &Path) -> String {
    path.to_str()
        .unwrap()
        .replace('%', "%25")
        .replace('&', "%26")
        .replace('#', "%23")
        .replace('+', "%2B")
        .replace(' ', "%20")
}

/// The rows of a listing, or a panic saying what came back instead.
fn rows(listing: DirectoryListing) -> Vec<DirectoryEntry> {
    match listing {
        DirectoryListing::Listed { entries, .. } => entries,
        other => panic!("expected a listing, got {other:?}"),
    }
}

/// And the rows' names, which is what the dropdown draws.
fn names(listing: DirectoryListing) -> Vec<String> {
    rows(listing).into_iter().map(|row| row.name).collect()
}

/// A directory holding a `.git`, which is a repository from outside it.
fn repository(at: &Path) {
    std::fs::create_dir_all(at.join(".git")).unwrap();
}

async fn get<T: DeserializeOwned>(app: &Router, path: &str) -> T {
    let response = app
        .clone()
        .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
        .await
        .unwrap();

    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body = String::from_utf8(bytes.to_vec()).unwrap();

    assert_eq!(status, StatusCode::OK, "GET {path} failed: {body}");
    serde_json::from_str(&body).unwrap_or_else(|error| panic!("reading {body:?}: {error}"))
}

#[tokio::test]
async fn a_directory_inside_a_watched_root_lists_with_directories_first() {
    let watched = tempfile::tempdir().unwrap();
    let (_dir, app) = app_watching(&[watched.path()]).await;

    std::fs::create_dir(watched.path().join("src")).unwrap();
    std::fs::create_dir(watched.path().join("assets")).unwrap();
    std::fs::write(watched.path().join("README.md"), "# a directory\n").unwrap();

    let listing = browse(&app, "watched", Some(watched.path())).await;

    assert_eq!(
        rows(listing)
            .into_iter()
            .map(|row| (row.name, row.kind))
            .collect::<Vec<_>>(),
        vec![
            ("assets".to_owned(), EntryKind::Directory),
            ("src".to_owned(), EntryKind::Directory),
            ("README.md".to_owned(), EntryKind::File),
        ]
    );
}

/// Where a browse bounded by the Watched Paths begins: the roots themselves,
/// which are a listing with no directory above them.
#[tokio::test]
async fn the_watched_scope_with_no_path_answers_the_roots() {
    let installed = tempfile::tempdir().unwrap();
    let said = tempfile::tempdir().unwrap();
    let (dir, app) = app_watching(&[installed.path()]).await;

    // Both halves of the boundary, because both of them are directories a Repo
    // may be registered from — and the settings' half is read at the ask rather
    // than when the server came up.
    watch_in_the_settings(dir.path(), &[said.path()]);

    let listing = browse(&app, "watched", None).await;

    let DirectoryListing::Listed { path, entries } = listing else {
        panic!("the roots are a listing");
    };

    assert_eq!(path, None, "the roots have no one directory above them");

    let mut offered: Vec<String> = entries.into_iter().map(|row| row.path).collect();
    offered.sort();

    let mut expected = vec![
        installed.path().canonicalize().unwrap(),
        said.path().canonicalize().unwrap(),
    ]
    .into_iter()
    .map(|path| path.to_str().unwrap().to_owned())
    .collect::<Vec<_>>();
    expected.sort();

    assert_eq!(offered, expected);
}

/// The boundary doing its job, and the other scope being what it is for: one
/// path, two answers.
#[tokio::test]
async fn a_path_outside_every_watched_root_is_refused_and_lists_anywhere() {
    let watched = tempfile::tempdir().unwrap();
    let elsewhere = tempfile::tempdir().unwrap();
    let (_dir, app) = app_watching(&[watched.path()]).await;

    std::fs::create_dir(elsewhere.path().join("src")).unwrap();

    assert_eq!(
        browse(&app, "watched", Some(elsewhere.path())).await,
        DirectoryListing::OutsideWatchedPaths
    );

    assert_eq!(
        names(browse(&app, "anywhere", Some(elsewhere.path())).await),
        ["src"]
    );
}

/// The one entry the Repos' form is looking for, marked so it can draw it as
/// what it is.
#[tokio::test]
async fn a_directory_holding_a_git_comes_back_as_a_repository() {
    let watched = tempfile::tempdir().unwrap();
    let (_dir, app) = app_watching(&[watched.path()]).await;

    repository(&watched.path().join("verkstead"));
    std::fs::create_dir(watched.path().join("notes")).unwrap();

    assert_eq!(
        rows(browse(&app, "watched", Some(watched.path())).await)
            .into_iter()
            .map(|row| (row.name, row.kind))
            .collect::<Vec<_>>(),
        vec![
            ("notes".to_owned(), EntryKind::Directory),
            ("verkstead".to_owned(), EntryKind::Repository),
        ]
    );
}

/// Always listed, whatever the field asking will draw: which of them a human
/// sees is a decision about a field, and a listing that had already dropped them
/// could not serve the fields that exist to point at one.
#[tokio::test]
async fn dotfiles_are_listed() {
    let watched = tempfile::tempdir().unwrap();
    let (_dir, app) = app_watching(&[watched.path()]).await;

    std::fs::create_dir(watched.path().join(".claude")).unwrap();
    std::fs::write(watched.path().join(".claude.json"), "{}\n").unwrap();

    assert_eq!(
        names(browse(&app, "watched", Some(watched.path())).await),
        [".claude", ".claude.json"]
    );
}

/// A directory that was there a moment ago and is not now — which is the
/// ordinary way a browse meets one it cannot read, and the same answer a field
/// halfway through a word gets.
#[tokio::test]
async fn a_directory_that_went_between_two_asks_answers_a_refusal_rather_than_a_failure() {
    let watched = tempfile::tempdir().unwrap();
    let (_dir, app) = app_watching(&[watched.path()]).await;

    let going = watched.path().join("going");
    std::fs::create_dir(&going).unwrap();

    assert_eq!(
        names(browse(&app, "watched", Some(&going)).await),
        [] as [&str; 0]
    );

    std::fs::remove_dir(&going).unwrap();

    assert_eq!(
        browse(&app, "watched", Some(&going)).await,
        DirectoryListing::Missing
    );
}

/// A path naming a file is a browse that has gone as deep as it goes.
#[tokio::test]
async fn a_file_is_not_a_directory() {
    let watched = tempfile::tempdir().unwrap();
    let (_dir, app) = app_watching(&[watched.path()]).await;

    let file = watched.path().join("notes.md");
    std::fs::write(&file, "# notes\n").unwrap();

    assert_eq!(
        browse(&app, "watched", Some(&file)).await,
        DirectoryListing::NotADirectory
    );
}

/// Nothing here resolves a relative path, in either scope: the directory the
/// server happens to be running in is not something a path should mean.
#[tokio::test]
async fn a_relative_path_is_refused_in_either_scope() {
    let watched = tempfile::tempdir().unwrap();
    let (_dir, app) = app_watching(&[watched.path()]).await;

    for scope in ["watched", "anywhere"] {
        assert_eq!(
            browse(&app, scope, Some(Path::new("src"))).await,
            DirectoryListing::NotAbsolute
        );
    }
}

/// The anywhere scope with nothing typed is `/`, which is where a browse
/// bounded by nothing begins.
#[tokio::test]
async fn the_anywhere_scope_with_no_path_reads_the_filesystem_root() {
    let watched = tempfile::tempdir().unwrap();
    let (_dir, app) = app_watching(&[watched.path()]).await;

    let DirectoryListing::Listed { path, entries } = browse(&app, "anywhere", None).await else {
        panic!("the root lists");
    };

    assert_eq!(path.as_deref(), Some("/"));
    assert!(!entries.is_empty(), "there is something in /");
}

/// A cleared input sends the key with nothing after it, and that names the same
/// nothing as not sending it at all.
#[tokio::test]
async fn an_empty_path_is_no_path() {
    let watched = tempfile::tempdir().unwrap();
    let (_dir, app) = app_watching(&[watched.path()]).await;

    let listing: DirectoryListing = get(&app, "/api/ui/directories?scope=watched&path=").await;

    assert_eq!(
        rows(listing)
            .into_iter()
            .map(|row| row.path)
            .collect::<Vec<_>>(),
        [watched.path().canonicalize().unwrap().to_str().unwrap()]
    );
}

/// A path that merely reads as inside a Watched Path is not inside it: the
/// boundary is consulted on the resolved path, here as everywhere else.
#[tokio::test]
async fn a_symlink_out_of_a_watched_root_is_refused() {
    let root = tempfile::tempdir().unwrap();
    let watched = root.path().join("watched");
    let elsewhere = root.path().join("elsewhere");
    std::fs::create_dir(&watched).unwrap();
    std::fs::create_dir(&elsewhere).unwrap();

    let (_dir, app) = app_watching(&[&watched]).await;

    let escape = watched.join("escape");
    std::os::unix::fs::symlink(&elsewhere, &escape).unwrap();

    assert_eq!(
        browse(&app, "watched", Some(&escape)).await,
        DirectoryListing::OutsideWatchedPaths
    );
}
