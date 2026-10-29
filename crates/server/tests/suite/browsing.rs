//! Browsing the filesystem over the viewer's namespace: what one directory
//! hands back, and where a field standing empty opens.
//!
//! Asked of the *server*, through the endpoint, for the reason registering a
//! Repo is asked that way in `tests/suite/repos.rs`: what a path field may reach is
//! the endpoint's own answer, and a browse arriving with a path nobody typed
//! into a dropdown gets the same one.
//!
//! Every refusal here is a 200 with a named outcome. A field is typed into a
//! character at a time, so a path that is relative or missing is the ordinary
//! state of one halfway through a word — something the dropdown draws a line
//! about, rather than an error to report.

use std::path::Path;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde::de::DeserializeOwned;
use tower::ServiceExt;
use verkstead_render::{DirectoryEntry, DirectoryListing, EntryKind};
use verkstead_server::{open_database, router};

/// A router, plus the directory holding its database alive.
///
/// Nothing is configured on it: a browse consults no boundary and no setting,
/// so the plainest router there is answers everything asked here.
async fn app() -> (tempfile::TempDir, Router) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    (dir, router(pool))
}

/// What the dropdown would be filled from: one directory, or the one a field
/// standing empty opens on.
async fn browse(app: &Router, path: Option<&Path>) -> DirectoryListing {
    let query = match path {
        Some(path) => format!("?path={}", encoded(path)),
        None => String::new(),
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
async fn a_directory_lists_with_directories_first() {
    let (_dir, app) = app().await;
    let looking = tempfile::tempdir().unwrap();

    std::fs::create_dir(looking.path().join("src")).unwrap();
    std::fs::create_dir(looking.path().join("assets")).unwrap();
    std::fs::write(looking.path().join("README.md"), "# a directory\n").unwrap();

    let listing = browse(&app, Some(looking.path())).await;

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

/// Where a browse begins when the field is empty: the server's own home, listed
/// like any other directory rather than as a boundary with no way out of it.
///
/// Asked on the platform whose home this suite can be sure of — the runner sets
/// `HOME`, and what a Windows machine reads instead is `platform`'s own subject.
#[cfg(unix)]
#[tokio::test]
async fn an_empty_field_opens_on_the_servers_home() {
    let (_dir, app) = app().await;

    let home = std::path::PathBuf::from(std::env::var("HOME").expect("the runner has a HOME"))
        .canonicalize()
        .expect("and it is there");

    let DirectoryListing::Listed { path, .. } = browse(&app, None).await else {
        panic!("a home that is there lists");
    };

    assert_eq!(path.as_deref(), home.to_str());
}

/// And nothing is out of reach from there: the directory above the home lists
/// too, which is what says the opening is a starting point rather than a
/// ceiling.
#[cfg(unix)]
#[tokio::test]
async fn the_directory_above_the_home_lists_as_well() {
    let (_dir, app) = app().await;

    let home = std::path::PathBuf::from(std::env::var("HOME").expect("the runner has a HOME"))
        .canonicalize()
        .expect("and it is there");
    let above = home.parent().expect("a home is under something");

    let DirectoryListing::Listed { path, entries } = browse(&app, Some(above)).await else {
        panic!("the directory above a home lists");
    };

    assert_eq!(path.as_deref(), above.to_str());
    assert!(
        entries.iter().any(|row| Path::new(&row.path) == home),
        "the home is one of the rows above it"
    );
}

/// The one entry the Repos' form is looking for, marked so it can draw it as
/// what it is.
#[tokio::test]
async fn a_directory_holding_a_git_comes_back_as_a_repository() {
    let (_dir, app) = app().await;
    let looking = tempfile::tempdir().unwrap();

    repository(&looking.path().join("verkstead"));
    std::fs::create_dir(looking.path().join("notes")).unwrap();

    assert_eq!(
        rows(browse(&app, Some(looking.path())).await)
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
    let (_dir, app) = app().await;
    let looking = tempfile::tempdir().unwrap();

    std::fs::create_dir(looking.path().join(".claude")).unwrap();
    std::fs::write(looking.path().join(".claude.json"), "{}\n").unwrap();

    assert_eq!(
        names(browse(&app, Some(looking.path())).await),
        [".claude", ".claude.json"]
    );
}

/// A directory that was there a moment ago and is not now — which is the
/// ordinary way a browse meets one it cannot read, and the same answer a field
/// halfway through a word gets.
#[tokio::test]
async fn a_directory_that_went_between_two_asks_answers_a_refusal_rather_than_a_failure() {
    let (_dir, app) = app().await;
    let looking = tempfile::tempdir().unwrap();

    let going = looking.path().join("going");
    std::fs::create_dir(&going).unwrap();

    assert_eq!(names(browse(&app, Some(&going)).await), [] as [&str; 0]);

    std::fs::remove_dir(&going).unwrap();

    assert_eq!(browse(&app, Some(&going)).await, DirectoryListing::Missing);
}

/// A path naming a file is a browse that has gone as deep as it goes.
#[tokio::test]
async fn a_file_is_not_a_directory() {
    let (_dir, app) = app().await;
    let looking = tempfile::tempdir().unwrap();

    let file = looking.path().join("notes.md");
    std::fs::write(&file, "# notes\n").unwrap();

    assert_eq!(
        browse(&app, Some(&file)).await,
        DirectoryListing::NotADirectory
    );
}

/// Nothing here resolves a relative path: the directory the server happens to be
/// running in is not something a path should mean.
#[tokio::test]
async fn a_relative_path_is_refused() {
    let (_dir, app) = app().await;

    assert_eq!(
        browse(&app, Some(Path::new("src"))).await,
        DirectoryListing::NotAbsolute
    );
}

/// A cleared input sends the key with nothing after it, and that names the same
/// nothing as not sending it at all — which is the home either way.
#[cfg(unix)]
#[tokio::test]
async fn an_empty_path_is_no_path() {
    let (_dir, app) = app().await;

    let cleared: DirectoryListing = get(&app, "/api/ui/directories?path=").await;

    assert_eq!(cleared, browse(&app, None).await);
}

/// A symlink is followed rather than read off, here as everywhere else: what a
/// browse lists is where the path really goes.
///
/// Made where a link can be made without asking anybody's permission, which is
/// both Unixes and not Windows.
#[cfg(unix)]
#[tokio::test]
async fn a_symlink_lists_what_it_points_at() {
    let root = tempfile::tempdir().unwrap();
    let elsewhere = root.path().join("elsewhere");
    std::fs::create_dir(&elsewhere).unwrap();
    std::fs::create_dir(elsewhere.join("src")).unwrap();

    let (_dir, app) = app().await;

    let link = root.path().join("link");
    std::os::unix::fs::symlink(&elsewhere, &link).unwrap();

    let DirectoryListing::Listed { path, entries } = browse(&app, Some(&link)).await else {
        panic!("a link to a directory lists that directory");
    };

    assert_eq!(path.as_deref(), elsewhere.canonicalize().unwrap().to_str());
    assert_eq!(
        entries.into_iter().map(|row| row.name).collect::<Vec<_>>(),
        ["src"]
    );
}
