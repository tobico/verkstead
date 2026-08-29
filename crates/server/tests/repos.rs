//! Registering a Repo over the viewer's namespace: what gets on the list, what
//! is refused before it can, what one of them says when it is opened, and what
//! taking one off the registry does to the list it was on.
//!
//! Every refusal here is asked of the *server*, through the endpoint, rather
//! than of the boundary type underneath it — which is the point of the Watched
//! Paths being a security boundary. A browser that skipped the form, or a `curl`
//! that never saw one, meets the same answers.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use serde::de::DeserializeOwned;
use sqlx::SqlitePool;
use tower::ServiceExt;
use verkstead_render::{Registered, RepoEntry, RepoRemoved, RepoView};
use verkstead_server::{WatchedPaths, open_database, router_watching, store};

/// A router watching `watched`, plus the directory holding its database alive.
async fn app_watching(watched: &Path) -> (tempfile::TempDir, Router) {
    let (dir, _pool, app) = app_and_pool_watching(watched).await;

    (dir, app)
}

/// The same, with the pool beside it — for the tests that put Conversations on
/// a Repo, which is the one thing they need that this namespace has no endpoint
/// for.
async fn app_and_pool_watching(watched: &Path) -> (tempfile::TempDir, SqlitePool, Router) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();
    let watched = WatchedPaths::resolve(&[watched.to_owned()]).unwrap();

    let data_dir = dir.path().to_owned();

    (dir, pool.clone(), router_watching(pool, watched, data_dir))
}

/// A git repository at `path`, with one commit on `main` so it has a branch to
/// call its default.
fn repository(path: PathBuf) -> PathBuf {
    std::fs::create_dir_all(&path).unwrap();
    git(&path, &["init", "--initial-branch", "main"]);
    git(&path, &["config", "user.email", "test@verkstead.invalid"]);
    git(&path, &["config", "user.name", "Verkstead Test"]);
    std::fs::write(path.join("README.md"), "# a repository\n").unwrap();
    git(&path, &["add", "README.md"]);
    git(&path, &["commit", "-m", "first"]);

    path
}

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("git should be on the PATH for these tests");

    assert!(status.success(), "git {args:?} failed in {}", dir.display());
}

/// Ask to register a path, and read back what the server made of it.
async fn register(app: &Router, path: &Path) -> Registered {
    post(app, "/api/ui/repos", &serde_json::json!({ "path": path })).await
}

/// The same, for a path that is not one the filesystem can hand back — a string
/// typed into the form.
async fn register_text(app: &Router, path: &str) -> Registered {
    post(app, "/api/ui/repos", &serde_json::json!({ "path": path })).await
}

async fn listed(app: &Router) -> Vec<RepoEntry> {
    get(app, "/api/ui/repos").await
}

/// The branches of one registered Repo, which is what the base dropdown offers.
async fn branches(app: &Router, id: i64) -> Vec<String> {
    get(app, &format!("/api/ui/repos/{id}/branches")).await
}

/// Ask for one to be taken off the registry, and read back what the server made
/// of that.
async fn remove(app: &Router, id: i64) -> RepoRemoved {
    post(
        app,
        &format!("/api/ui/repos/{id}/remove"),
        &serde_json::Value::Null,
    )
    .await
}

async fn get<T: DeserializeOwned>(app: &Router, path: &str) -> T {
    let (status, body) = fetch(
        app,
        Request::builder().uri(path).body(Body::empty()).unwrap(),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "GET {path} failed: {body}");
    read(&body)
}

async fn post<T: DeserializeOwned>(app: &Router, path: &str, body: &serde_json::Value) -> T {
    let (status, body) = fetch(
        app,
        Request::builder()
            .method("POST")
            .uri(path)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(serde_json::to_vec(body).unwrap()))
            .unwrap(),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "POST {path} failed: {body}");
    read(&body)
}

async fn fetch(app: &Router, request: Request<Body>) -> (StatusCode, String) {
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8(bytes.to_vec()).unwrap())
}

fn read<T: DeserializeOwned>(body: &str) -> T {
    serde_json::from_str(body).unwrap_or_else(|err| panic!("reading {body:?}: {err}"))
}

#[tokio::test]
async fn a_repo_inside_a_watched_path_registers_and_appears_on_the_list() {
    let watched = tempfile::tempdir().unwrap();
    let (_dir, app) = app_watching(watched.path()).await;
    let repo = repository(watched.path().join("verkstead"));

    assert_eq!(register(&app, &repo).await, Registered::Added);

    let repos = listed(&app).await;
    assert_eq!(repos.len(), 1);
    assert_eq!(repos[0].name, "verkstead");
    assert_eq!(
        repos[0].path,
        repo.canonicalize().unwrap().to_str().unwrap()
    );
    assert_eq!(repos[0].default_branch, "main");
}

#[tokio::test]
async fn nothing_is_registered_to_begin_with() {
    let watched = tempfile::tempdir().unwrap();
    let (_dir, app) = app_watching(watched.path()).await;

    assert!(listed(&app).await.is_empty());
}

/// The boundary itself: a perfectly good repository, refused for being
/// somewhere Verkstead was never given.
#[tokio::test]
async fn a_repo_outside_the_watched_paths_is_refused_by_the_server() {
    let root = tempfile::tempdir().unwrap();
    let watched = root.path().join("watched");
    std::fs::create_dir(&watched).unwrap();
    let (_dir, app) = app_watching(&watched).await;

    let elsewhere = repository(root.path().join("elsewhere"));

    assert_eq!(
        register(&app, &elsewhere).await,
        Registered::OutsideWatchedPaths
    );
    assert!(listed(&app).await.is_empty());
}

/// A path that reads as inside a Watched Path and is not: the symlink is
/// followed before the boundary is consulted.
#[tokio::test]
async fn a_repo_reached_through_a_symlink_out_of_a_watched_path_is_refused() {
    let root = tempfile::tempdir().unwrap();
    let watched = root.path().join("watched");
    std::fs::create_dir(&watched).unwrap();
    let (_dir, app) = app_watching(&watched).await;

    let elsewhere = repository(root.path().join("elsewhere"));
    let inside = watched.join("looks-inside");
    std::os::unix::fs::symlink(&elsewhere, &inside).unwrap();

    assert_eq!(
        register(&app, &inside).await,
        Registered::OutsideWatchedPaths
    );
    assert!(listed(&app).await.is_empty());
}

/// The other way of reading as inside one: `..` climbs back out.
#[tokio::test]
async fn a_repo_reached_by_climbing_out_with_dot_dot_is_refused() {
    let root = tempfile::tempdir().unwrap();
    let watched = root.path().join("watched");
    std::fs::create_dir(&watched).unwrap();
    let (_dir, app) = app_watching(&watched).await;

    repository(root.path().join("elsewhere"));
    let climbed = watched.join("..").join("elsewhere");

    assert_eq!(
        register(&app, &climbed).await,
        Registered::OutsideWatchedPaths
    );
    assert!(listed(&app).await.is_empty());
}

#[tokio::test]
async fn a_directory_that_is_not_a_git_repository_is_refused() {
    let watched = tempfile::tempdir().unwrap();
    let (_dir, app) = app_watching(watched.path()).await;

    let plain = watched.path().join("notes");
    std::fs::create_dir(&plain).unwrap();

    assert_eq!(register(&app, &plain).await, Registered::NotARepository);
}

/// A directory *in* a repository is not the repository: everything Verkstead
/// later builds hangs off the root, so a subdirectory would put a Conversation's
/// worktree somewhere nobody meant.
#[tokio::test]
async fn a_subdirectory_of_a_repository_is_not_the_repository() {
    let watched = tempfile::tempdir().unwrap();
    let (_dir, app) = app_watching(watched.path()).await;
    let repo = repository(watched.path().join("verkstead"));

    let inside = repo.join("crates");
    std::fs::create_dir(&inside).unwrap();

    assert_eq!(register(&app, &inside).await, Registered::NotARepository);
}

#[tokio::test]
async fn a_path_with_nothing_at_it_is_refused_as_missing() {
    let watched = tempfile::tempdir().unwrap();
    let (_dir, app) = app_watching(watched.path()).await;

    assert_eq!(
        register(&app, &watched.path().join("never-made")).await,
        Registered::Missing
    );
}

#[tokio::test]
async fn a_relative_path_is_refused_rather_than_resolved() {
    let watched = tempfile::tempdir().unwrap();
    let (_dir, app) = app_watching(watched.path()).await;
    repository(watched.path().join("verkstead"));

    assert_eq!(
        register_text(&app, "verkstead").await,
        Registered::NotAbsolute
    );
}

#[tokio::test]
async fn a_repo_already_registered_is_refused_however_its_path_is_spelled() {
    let watched = tempfile::tempdir().unwrap();
    let (_dir, app) = app_watching(watched.path()).await;
    let repo = repository(watched.path().join("verkstead"));

    assert_eq!(register(&app, &repo).await, Registered::Added);

    // The same directory, spelled its way out and back again.
    let roundabout = repo.join("..").join("verkstead");
    assert_eq!(
        register(&app, &roundabout).await,
        Registered::AlreadyRegistered
    );

    assert_eq!(listed(&app).await.len(), 1);
}

/// The closed state the server refuses to start in, asked of the router
/// directly: with no Watched Path there is nowhere a Repo could be registered
/// from, and every path is outside.
#[tokio::test]
async fn a_server_watching_nothing_registers_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();
    let app = router_watching(pool, WatchedPaths::none(), dir.path().to_owned());

    let repo = repository(dir.path().join("verkstead"));

    assert_eq!(register(&app, &repo).await, Registered::OutsideWatchedPaths);
}

/// What a Conversation branches from: the remote's idea of the default branch
/// wins over whatever happens to be checked out, because that is what everyone
/// working on the repository means by it.
#[tokio::test]
async fn the_default_branch_is_what_the_remote_calls_it() {
    let watched = tempfile::tempdir().unwrap();
    let (_dir, app) = app_watching(watched.path()).await;
    let repo = repository(watched.path().join("verkstead"));

    // A remote of its own, pointing back at itself: enough for `origin/HEAD` to
    // exist and name a branch, without a network anywhere.
    git(&repo, &["remote", "add", "origin", repo.to_str().unwrap()]);
    git(&repo, &["fetch", "--quiet", "origin"]);
    git(
        &repo,
        &[
            "symbolic-ref",
            "refs/remotes/origin/HEAD",
            "refs/remotes/origin/main",
        ],
    );
    git(&repo, &["checkout", "--quiet", "-b", "some-feature"]);

    assert_eq!(register(&app, &repo).await, Registered::Added);
    assert_eq!(listed(&app).await[0].default_branch, "main");
}

/// A repository with nothing checked out has no branch to work from, and
/// inventing one would put work on a branch nobody chose.
#[tokio::test]
async fn a_repository_with_no_branch_to_call_its_default_is_refused() {
    let watched = tempfile::tempdir().unwrap();
    let (_dir, app) = app_watching(watched.path()).await;
    let repo = repository(watched.path().join("verkstead"));

    git(&repo, &["checkout", "--quiet", "--detach", "HEAD"]);

    assert_eq!(register(&app, &repo).await, Registered::NoDefaultBranch);
}

/// The list a drafting Conversation picks what it comes off out of: every
/// branch the repository has, local and remote-tracking both.
///
/// `origin/HEAD` is left out of it. It is a symbolic ref — another name for a
/// branch that is already on the list — and offering it twice would be offering
/// a choice that is not one.
#[tokio::test]
async fn a_repos_branches_are_the_local_and_remote_tracking_ones() {
    let watched = tempfile::tempdir().unwrap();
    let (_dir, app) = app_watching(watched.path()).await;
    let repo = repository(watched.path().join("verkstead"));

    git(&repo, &["branch", "release"]);
    git(&repo, &["remote", "add", "origin", repo.to_str().unwrap()]);
    git(&repo, &["fetch", "--quiet", "origin"]);
    git(
        &repo,
        &[
            "symbolic-ref",
            "refs/remotes/origin/HEAD",
            "refs/remotes/origin/main",
        ],
    );

    assert_eq!(register(&app, &repo).await, Registered::Added);
    let id = listed(&app).await[0].id;

    assert_eq!(
        branches(&app, id).await,
        vec![
            "main".to_owned(),
            "release".to_owned(),
            "origin/main".to_owned(),
            "origin/release".to_owned(),
        ],
        "the locals first, then what the remote is carrying",
    );
}

/// A Repo that is not registered has no branches to read, and saying so is a
/// refusal rather than an empty list: an empty list is a repository with
/// nothing on it, which is a different thing to be told.
#[tokio::test]
async fn the_branches_of_a_repo_that_is_not_there_are_refused() {
    let watched = tempfile::tempdir().unwrap();
    let (_dir, app) = app_watching(watched.path()).await;

    for asked in ["404", "not-a-number"] {
        let (status, _) = fetch(
            &app,
            Request::builder()
                .uri(format!("/api/ui/repos/{asked}/branches"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;

        assert_eq!(status, StatusCode::NOT_FOUND, "asking about {asked}");
    }
}

/// One Repo opened, which is what its card in the settings leads to: the row's
/// own three facts, plus everything the card had no room for.
///
/// The roadmaps are the same reading the notice under the new-conversation box
/// makes — `ui_content.rs` is where what that finds is pinned — so what is
/// asserted here is that a repository holding none says so with an empty list
/// rather than by leaving the field out.
#[tokio::test]
async fn a_repo_opened_carries_its_branches_its_work_and_its_roadmaps() {
    let watched = tempfile::tempdir().unwrap();
    let (_dir, pool, app) = app_and_pool_watching(watched.path()).await;
    let repo = repository(watched.path().join("verkstead"));
    git(&repo, &["branch", "release"]);

    assert_eq!(register(&app, &repo).await, Registered::Added);
    let id = listed(&app).await[0].id;

    // Three Conversations on it: one still going, and two that are over each
    // way there is to be over.
    for (branch, state) in [
        ("rate-limiting", None),
        ("pane-paths", Some(store::Lifecycle::Done)),
        ("dropped", Some(store::Lifecycle::Closed)),
    ] {
        let started = store::start_conversation(&pool, id, branch)
            .await
            .unwrap()
            .unwrap();

        if let Some(state) = state {
            store::set_state(&pool, started, state).await.unwrap();
        }
    }

    let opened: RepoView = get(&app, &format!("/api/ui/repos/{id}")).await;

    assert_eq!(opened.id, id);
    assert_eq!(opened.name, "verkstead");
    assert_eq!(opened.path, repo.canonicalize().unwrap().to_str().unwrap());
    assert_eq!(opened.default_branch, "main");
    assert_eq!(
        opened.branches,
        vec!["main".to_owned(), "release".to_owned()],
        "the same list the base dropdown is filled from",
    );
    assert_eq!(opened.live, 1);
    assert_eq!(opened.finished, 2, "Done and Closed counted together");
    assert!(
        opened.roadmaps.is_empty(),
        "a repository with no roadmaps has none waiting: {:?}",
        opened.roadmaps,
    );
}

/// A Repo that is not registered has nothing to open, and saying so is a
/// refusal: the pane reads it as the repo being gone — a link followed after
/// somebody took it away — rather than as a Repo with nothing on it.
#[tokio::test]
async fn a_repo_that_is_not_there_cannot_be_opened() {
    let watched = tempfile::tempdir().unwrap();
    let (_dir, app) = app_watching(watched.path()).await;

    for asked in ["404", "not-a-number"] {
        let (status, _) = fetch(
            &app,
            Request::builder()
                .uri(format!("/api/ui/repos/{asked}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;

        assert_eq!(status, StatusCode::NOT_FOUND, "opening {asked}");
    }
}

/// And a Repo that was taken off the registry has nothing to open either. It is
/// still in the table — every Conversation ever worked in it names it — but
/// nothing is registered under that id any more, and the pane reads that as the
/// repo being gone rather than drawing one with a Remove button on it.
#[tokio::test]
async fn a_repo_that_was_removed_cannot_be_opened() {
    let watched = tempfile::tempdir().unwrap();
    let (_dir, app) = app_watching(watched.path()).await;
    let repo = repository(watched.path().join("verkstead"));

    assert_eq!(register(&app, &repo).await, Registered::Added);
    let id = listed(&app).await[0].id;

    assert_eq!(remove(&app, id).await, RepoRemoved::Removed);

    let (status, _) = fetch(
        &app,
        Request::builder()
            .uri(format!("/api/ui/repos/{id}"))
            .body(Body::empty())
            .unwrap(),
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

/// A Repo taken off the registry is off every list that offers Repos for new
/// work — this one, the New conversation menu behind it, and the roadmap notice,
/// all of which are the same read.
#[tokio::test]
async fn a_removed_repo_is_off_the_list() {
    let watched = tempfile::tempdir().unwrap();
    let (_dir, app) = app_watching(watched.path()).await;
    let repo = repository(watched.path().join("verkstead"));

    assert_eq!(register(&app, &repo).await, Registered::Added);
    let id = listed(&app).await[0].id;

    assert_eq!(remove(&app, id).await, RepoRemoved::Removed);
    assert!(listed(&app).await.is_empty());

    // And the roadmap notice, which is a read of its own over the same list.
    let waiting: Vec<serde_json::Value> = get(&app, "/api/ui/abandoned-roadmaps").await;
    assert!(waiting.is_empty(), "an unregistered Repo offers nothing");
}

/// Work still going on in a repository is the reason to keep it registered, so
/// the removal is refused with the reason the pane says out loud — and the Repo
/// is where it was.
#[tokio::test]
async fn a_repo_with_live_work_on_it_is_refused() {
    let watched = tempfile::tempdir().unwrap();
    let (_dir, pool, app) = app_and_pool_watching(watched.path()).await;
    let repo = repository(watched.path().join("verkstead"));

    assert_eq!(register(&app, &repo).await, Registered::Added);
    let id = listed(&app).await[0].id;

    let going = store::start_conversation(&pool, id, "rate-limiting")
        .await
        .unwrap()
        .unwrap();

    assert_eq!(remove(&app, id).await, RepoRemoved::InUse);
    assert_eq!(listed(&app).await.len(), 1, "nothing was taken away");

    // Closed is over, and what is over is no reason to hold the registration.
    store::set_state(&pool, going, store::Lifecycle::Closed)
        .await
        .unwrap();

    assert_eq!(remove(&app, id).await, RepoRemoved::Removed);
}

/// An id nothing is registered under is a named outcome rather than a status:
/// one already taken away, one that never was, and one that is not a number at
/// all are the same sentence.
#[tokio::test]
async fn there_is_nothing_to_remove_twice() {
    let watched = tempfile::tempdir().unwrap();
    let (_dir, app) = app_watching(watched.path()).await;
    let repo = repository(watched.path().join("verkstead"));

    assert_eq!(register(&app, &repo).await, Registered::Added);
    let id = listed(&app).await[0].id;

    assert_eq!(remove(&app, id).await, RepoRemoved::Removed);
    assert_eq!(remove(&app, id).await, RepoRemoved::NoSuchRepo);
    assert_eq!(remove(&app, 404).await, RepoRemoved::NoSuchRepo);

    let refused: RepoRemoved = post(
        &app,
        "/api/ui/repos/not-a-number/remove",
        &serde_json::Value::Null,
    )
    .await;
    assert_eq!(refused, RepoRemoved::NoSuchRepo);
}

/// And registering the same repository again brings it back rather than being
/// refused as registered already — which is what makes a removal something the
/// human can undo.
#[tokio::test]
async fn registering_a_removed_repo_again_brings_it_back() {
    let watched = tempfile::tempdir().unwrap();
    let (_dir, app) = app_watching(watched.path()).await;
    let repo = repository(watched.path().join("verkstead"));

    assert_eq!(register(&app, &repo).await, Registered::Added);
    let id = listed(&app).await[0].id;
    assert_eq!(remove(&app, id).await, RepoRemoved::Removed);

    assert_eq!(register(&app, &repo).await, Registered::Added);

    let back = listed(&app).await;
    assert_eq!(back.len(), 1);
    assert_eq!(back[0].id, id, "the same Repo, under the id it always had");
}
