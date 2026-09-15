//! Registering a Repo over the viewer's namespace: what gets on the list, what
//! is refused before it can, what one of them says when it is opened, and what
//! taking one off the registry does to the list it was on.
//!
//! And making one, which is the other way a Repo arrives: a directory, a
//! repository on `main` with a first commit by the configured author, and the
//! same registration at the end of it — with five refusals of its own, every one
//! of them leaving nothing on the registry.
//!
//! A create may reach GitHub as well, and that half is asked of a script
//! standing where `gh` goes, for the reason the rest of Verkstead's GitHub is:
//! what there is to prove is that a process was run with the right arguments, in
//! the right directory and as the right token, and asking the real GitHub would
//! be a test that needed a network and somebody's account. It is the one thing
//! here that can fail without the create failing.
//!
//! Every refusal here is asked of the *server*, through the endpoint, rather
//! than of the checks underneath it: a browser that skipped the form, or a
//! `curl` that never saw one, meets the same answers.
//!
//! Where a repository *is* is not one of the refusals. Anywhere the server can
//! read is somewhere a Repo can be registered from, whatever the installation
//! was started with.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use serde::de::DeserializeOwned;
use sqlx::SqlitePool;
use tower::ServiceExt;
use verkstead_render::{Created, Registered, RepoEntry, RepoRemoved, RepoView};
// Only the shell stand-in below authenticates as a saved token, and that is
// off Windows with the shell it needs.
#[cfg(unix)]
use verkstead_server::settings::Settings;
use verkstead_server::{Gh, open_database, router_asking_github, router_keeping, store};

/// A router, plus the Data Directory holding its database alive.
async fn workbench() -> (tempfile::TempDir, Router) {
    let (dir, _pool, app) = workbench_and_pool().await;

    (dir, app)
}

/// The same, with the pool beside it — for the tests that put Conversations on
/// a Repo, which is the one thing they need that this namespace has no endpoint
/// for.
async fn workbench_and_pool() -> (tempfile::TempDir, SqlitePool, Router) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    let data_dir = dir.path().to_owned();

    (dir, pool.clone(), router_keeping(pool, data_dir))
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

/// The Repo a registration that landed hands back — and the assertion that it
/// landed, which is the same thing: an outcome that carries a Repo is one that
/// left a Repo registered.
#[track_caller]
fn added(outcome: Registered) -> RepoEntry {
    match outcome {
        Registered::Added(repo) => repo,
        refused => panic!("the registration was refused: {refused:?}"),
    }
}

/// And the Repo a path already registered hands back, which is the same Repo the
/// first registration made.
#[track_caller]
fn already(outcome: Registered) -> RepoEntry {
    match outcome {
        Registered::AlreadyRegistered(repo) => repo,
        other => panic!("the path was not already registered: {other:?}"),
    }
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
async fn a_repository_registers_and_appears_on_the_list() {
    let src = tempfile::tempdir().unwrap();
    let (_dir, app) = workbench().await;
    let repo = repository(src.path().join("verkstead"));

    let registered = added(register(&app, &repo).await);

    let repos = listed(&app).await;
    assert_eq!(repos.len(), 1);
    assert_eq!(repos[0].name, "verkstead");
    assert_eq!(
        repos[0].path,
        repo.canonicalize().unwrap().to_str().unwrap()
    );
    assert_eq!(repos[0].default_branch, "main");

    // And the registration hands back that same row rather than an outcome the
    // caller has to go looking for the Repo behind — the path it recorded is
    // the resolved one, which is not the one that was typed.
    assert_eq!(registered, repos[0]);
}

#[tokio::test]
async fn nothing_is_registered_to_begin_with() {
    let (_dir, app) = workbench().await;

    assert!(listed(&app).await.is_empty());
}

/// The boundary is gone: a repository nowhere near anything the server was
/// started with — its own Data Directory included — registers like any other,
/// and what is stored is where it really is.
#[tokio::test]
async fn a_repository_outside_everything_the_server_was_started_with_registers() {
    let root = tempfile::tempdir().unwrap();
    let (_dir, app) = workbench().await;

    let elsewhere = repository(root.path().join("elsewhere"));

    added(register(&app, &elsewhere).await);

    let repos = listed(&app).await;
    assert_eq!(repos.len(), 1);
    assert_eq!(
        repos[0].path,
        elsewhere.canonicalize().unwrap().to_str().unwrap()
    );
}

/// What is registered is the resolved path rather than the one that was typed:
/// a symlink is followed first, so the row names the directory a session will
/// actually stand in.
///
/// Made where a link can be made without asking anybody's permission, which is
/// both Unixes and not Windows. The resolving is the same everywhere — it is
/// `canonicalize` — so what is lost there is the making of the link rather than
/// any of the reasoning.
#[cfg(unix)]
#[tokio::test]
async fn a_repository_reached_through_a_symlink_is_stored_where_it_really_is() {
    let root = tempfile::tempdir().unwrap();
    let (_dir, app) = workbench().await;

    let elsewhere = repository(root.path().join("elsewhere"));
    let link = root.path().join("looks-elsewhere");
    std::os::unix::fs::symlink(&elsewhere, &link).unwrap();

    added(register(&app, &link).await);

    assert_eq!(
        listed(&app).await[0].path,
        elsewhere.canonicalize().unwrap().to_str().unwrap()
    );
}

#[tokio::test]
async fn a_directory_that_is_not_a_git_repository_is_refused() {
    let src = tempfile::tempdir().unwrap();
    let (_dir, app) = workbench().await;

    let plain = src.path().join("notes");
    std::fs::create_dir(&plain).unwrap();

    assert_eq!(register(&app, &plain).await, Registered::NotARepository);
}

/// A directory *in* a repository is not the repository: everything Verkstead
/// later builds hangs off the root, so a subdirectory would put a Conversation's
/// worktree somewhere nobody meant.
#[tokio::test]
async fn a_subdirectory_of_a_repository_is_not_the_repository() {
    let src = tempfile::tempdir().unwrap();
    let (_dir, app) = workbench().await;
    let repo = repository(src.path().join("verkstead"));

    let inside = repo.join("crates");
    std::fs::create_dir(&inside).unwrap();

    assert_eq!(register(&app, &inside).await, Registered::NotARepository);
}

#[tokio::test]
async fn a_path_with_nothing_at_it_is_refused_as_missing() {
    let src = tempfile::tempdir().unwrap();
    let (_dir, app) = workbench().await;

    assert_eq!(
        register(&app, &src.path().join("never-made")).await,
        Registered::Missing
    );
}

#[tokio::test]
async fn a_relative_path_is_refused_rather_than_resolved() {
    let src = tempfile::tempdir().unwrap();
    let (_dir, app) = workbench().await;
    repository(src.path().join("verkstead"));

    assert_eq!(
        register_text(&app, "verkstead").await,
        Registered::NotAbsolute
    );
}

#[tokio::test]
async fn a_repo_already_registered_is_refused_however_its_path_is_spelled() {
    let src = tempfile::tempdir().unwrap();
    let (_dir, app) = workbench().await;
    let repo = repository(src.path().join("verkstead"));

    let first = added(register(&app, &repo).await);

    // The same directory, spelled its way out and back again.
    let roundabout = repo.join("..").join("verkstead");

    // And the same Repo comes back with the refusal, which is what makes it a
    // repository to land a draft on rather than a dead end: a dropdown that
    // registered a path somebody had registered already has the Repo it named.
    assert_eq!(already(register(&app, &roundabout).await), first);

    assert_eq!(listed(&app).await.len(), 1);
}

/// The standalone install: no unit, no flags, nothing configured anywhere. It
/// registers a repository like any other, which is what a bare binary being
/// usable out of the box means.
#[tokio::test]
async fn a_server_told_nothing_at_all_registers_all_the_same() {
    let src = tempfile::tempdir().unwrap();
    let (_dir, app) = workbench().await;
    let repo = repository(src.path().join("verkstead"));

    added(register(&app, &repo).await);
    assert_eq!(listed(&app).await.len(), 1);
}

/// What a Conversation branches from: the remote's idea of the default branch
/// wins over whatever happens to be checked out, because that is what everyone
/// working on the repository means by it.
#[tokio::test]
async fn the_default_branch_is_what_the_remote_calls_it() {
    let src = tempfile::tempdir().unwrap();
    let (_dir, app) = workbench().await;
    let repo = repository(src.path().join("verkstead"));

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

    added(register(&app, &repo).await);
    assert_eq!(listed(&app).await[0].default_branch, "main");
}

/// A repository with nothing checked out has no branch to work from, and
/// inventing one would put work on a branch nobody chose.
#[tokio::test]
async fn a_repository_with_no_branch_to_call_its_default_is_refused() {
    let src = tempfile::tempdir().unwrap();
    let (_dir, app) = workbench().await;
    let repo = repository(src.path().join("verkstead"));

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
    let src = tempfile::tempdir().unwrap();
    let (_dir, app) = workbench().await;
    let repo = repository(src.path().join("verkstead"));

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

    added(register(&app, &repo).await);
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
    let (_dir, app) = workbench().await;

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
    let src = tempfile::tempdir().unwrap();
    let (_dir, pool, app) = workbench_and_pool().await;
    let repo = repository(src.path().join("verkstead"));
    git(&repo, &["branch", "release"]);

    added(register(&app, &repo).await);
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
    let (_dir, app) = workbench().await;

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
    let src = tempfile::tempdir().unwrap();
    let (_dir, app) = workbench().await;
    let repo = repository(src.path().join("verkstead"));

    added(register(&app, &repo).await);
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
/// work — this one, the compose page's Repo dropdown behind it, and the
/// roadmaps there are to adopt, all of which are the same read.
#[tokio::test]
async fn a_removed_repo_is_off_the_list() {
    let src = tempfile::tempdir().unwrap();
    let (_dir, app) = workbench().await;
    let repo = repository(src.path().join("verkstead"));

    added(register(&app, &repo).await);
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
    let src = tempfile::tempdir().unwrap();
    let (_dir, pool, app) = workbench_and_pool().await;
    let repo = repository(src.path().join("verkstead"));

    added(register(&app, &repo).await);
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
    let src = tempfile::tempdir().unwrap();
    let (_dir, app) = workbench().await;
    let repo = repository(src.path().join("verkstead"));

    added(register(&app, &repo).await);
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
    let src = tempfile::tempdir().unwrap();
    let (_dir, app) = workbench().await;
    let repo = repository(src.path().join("verkstead"));

    added(register(&app, &repo).await);
    let id = listed(&app).await[0].id;
    assert_eq!(remove(&app, id).await, RepoRemoved::Removed);

    added(register(&app, &repo).await);

    let back = listed(&app).await;
    assert_eq!(back.len(), 1);
    assert_eq!(back[0].id, id, "the same Repo, under the id it always had");
}

/// Ask for a repository to be made, and read back what the server made of it.
///
/// With the GitHub half left off, which is the create every refusal below is
/// asked of: what those are about is the directory and the commit, and a create
/// that never got as far as one has nothing to push.
async fn create(app: &Router, parent: &Path, name: &str) -> Created {
    post(
        app,
        "/api/ui/repos/new",
        &serde_json::json!({ "parent": parent, "name": name, "github": false }),
    )
    .await
}

/// The same, for a parent that is not one the filesystem can hand back — a path
/// typed into the form.
async fn create_under(app: &Router, parent: &str, name: &str) -> Created {
    post(
        app,
        "/api/ui/repos/new",
        &serde_json::json!({ "parent": parent, "name": name, "github": false }),
    )
    .await
}

/// The Repo a create that landed hands back — and the assertion that it landed,
/// which is the same thing: the one outcome carrying a Repo is the one that
/// left a repository on the disk.
#[track_caller]
fn made(outcome: Created) -> RepoView {
    match outcome {
        Created::Made(repo) => repo,
        refused => panic!("the create was refused: {refused:?}"),
    }
}

/// What git says about `dir`, for the readings a test makes of a repository the
/// server built.
fn git_says(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .expect("git should be on the PATH for these tests");

    assert!(
        output.status.success(),
        "git {args:?} failed in {}",
        dir.display()
    );

    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

/// A workbench that has been told who to commit as, which a create needs and a
/// registration never does.
///
/// Written into the Data Directory the router was stood up over: the settings
/// files are read at the moment they are wanted, so an author saved after the
/// server came up is the author the next create uses.
async fn workbench_authored() -> (tempfile::TempDir, Router) {
    let (dir, app) = workbench().await;

    author(&dir, "Ada Lovelace", "ada@example.com");

    (dir, app)
}

/// Say who Verkstead commits as, in the file the settings page writes.
fn author(dir: &tempfile::TempDir, name: &str, email: &str) {
    std::fs::write(
        dir.path().join("config.yaml"),
        format!("git_author:\n  name: \"{name}\"\n  email: \"{email}\"\n"),
    )
    .unwrap();
}

/// A created repository is one a Conversation can be started on the moment it
/// comes back: `main` is there, it holds a commit, and the commit is by the
/// human the settings page was told about.
#[tokio::test]
async fn a_created_repository_holds_a_first_commit_by_the_configured_author() {
    let root = tempfile::tempdir().unwrap();
    let (_dir, app) = workbench_authored().await;

    let repo = made(create(&app, root.path(), "verkstead").await);

    let path = root.path().join("verkstead");
    assert_eq!(repo.name, "verkstead", "named for the directory it is");
    assert_eq!(repo.path, path.canonicalize().unwrap().to_str().unwrap());
    assert_eq!(repo.default_branch, "main");
    assert_eq!(repo.branches, vec!["main".to_owned()]);

    // One commit, on `main`, holding the README a fresh repository conventionally
    // holds — which is what a Conversation takes its base from.
    assert_eq!(git_says(&path, &["rev-list", "--count", "HEAD"]), "1");
    assert_eq!(
        git_says(&path, &["symbolic-ref", "--short", "HEAD"]),
        "main"
    );
    assert_eq!(
        git_says(&path, &["show", "--name-only", "--format=", "HEAD"]),
        "README.md"
    );
    assert_eq!(
        std::fs::read_to_string(path.join("README.md")).unwrap(),
        "# verkstead\n"
    );

    // By the configured author rather than by whatever this machine's git would
    // have signed it — and said on the command line, so the new repository's own
    // config is left holding nothing about who anybody is.
    assert_eq!(
        git_says(&path, &["log", "-1", "--format=%an <%ae>"]),
        "Ada Lovelace <ada@example.com>"
    );
    assert!(
        git_says(&path, &["config", "--local", "--list"])
            .lines()
            .all(|line| !line.starts_with("user.")),
        "the identity is said to the commit rather than written into the repository",
    );

    // And it is registered, under the directory's own name, like any other Repo.
    let repos = listed(&app).await;
    assert_eq!(repos.len(), 1);
    assert_eq!(repos[0].id, repo.id);
    assert_eq!(repos[0].name, "verkstead");
    assert_eq!(repos[0].path, repo.path);
}

/// A parent with nothing at it, and one that is not a path the server can mean:
/// one sentence, because either way there is no directory for the new one to go
/// in.
#[tokio::test]
async fn a_parent_that_is_not_there_is_refused() {
    let root = tempfile::tempdir().unwrap();
    let (_dir, app) = workbench_authored().await;

    let nowhere = root.path().join("never-made");
    assert_eq!(
        create(&app, &nowhere, "verkstead").await,
        Created::ParentMissing
    );
    assert_eq!(
        create_under(&app, "src", "verkstead").await,
        Created::ParentMissing
    );

    // A file is not a parent either.
    let file = root.path().join("notes.md");
    std::fs::write(&file, "nothing to see").unwrap();
    assert_eq!(
        create(&app, &file, "verkstead").await,
        Created::ParentMissing
    );

    assert!(listed(&app).await.is_empty(), "nothing was registered");
}

/// A directory of that name in that parent is somebody's, so the create says so
/// and touches neither it nor the registry.
#[tokio::test]
async fn a_directory_that_is_there_already_is_left_alone() {
    let root = tempfile::tempdir().unwrap();
    let (_dir, app) = workbench_authored().await;

    let taken = root.path().join("verkstead");
    std::fs::create_dir(&taken).unwrap();
    std::fs::write(taken.join("notes.md"), "somebody's").unwrap();

    assert_eq!(
        create(&app, root.path(), "verkstead").await,
        Created::AlreadyThere
    );

    assert_eq!(
        std::fs::read_to_string(taken.join("notes.md")).unwrap(),
        "somebody's",
        "what was there is what is there",
    );
    assert!(!taken.join(".git").exists(), "nothing was made in it");
    assert!(listed(&app).await.is_empty());

    // A file of that name is the same answer: something is there.
    std::fs::write(root.path().join("notes"), "somebody's").unwrap();
    assert_eq!(
        create(&app, root.path(), "notes").await,
        Created::AlreadyThere
    );
}

/// A name is a name rather than a path: nothing that would climb out of the
/// parent or land somewhere else makes a directory.
#[tokio::test]
async fn a_name_that_is_not_a_name_is_refused() {
    let root = tempfile::tempdir().unwrap();
    let (_dir, app) = workbench_authored().await;

    for name in ["", "   ", ".", "..", "src/verkstead", "../verkstead"] {
        assert_eq!(
            create(&app, root.path(), name).await,
            Created::BadName,
            "for {name:?}",
        );
    }

    assert!(listed(&app).await.is_empty(), "nothing was registered");
    assert_eq!(
        std::fs::read_dir(root.path()).unwrap().count(),
        0,
        "and nothing was made",
    );
}

/// A repository whose first commit is by nobody is that repository's history, so
/// a Verkstead nobody has told who they are refuses rather than signing it
/// itself.
#[tokio::test]
async fn nothing_is_made_without_an_author() {
    let root = tempfile::tempdir().unwrap();
    let (dir, app) = workbench().await;

    assert_eq!(
        create(&app, root.path(), "verkstead").await,
        Created::NoAuthor
    );
    assert!(
        !root.path().join("verkstead").exists(),
        "refused before anything was made",
    );
    assert!(listed(&app).await.is_empty());

    // Half an author is no author: git wants both, and the answer is about the
    // settings page rather than about git.
    std::fs::write(dir.path().join("config.yaml"), "git_author:\n  name: Ada\n").unwrap();
    assert_eq!(
        create(&app, root.path(), "verkstead").await,
        Created::NoAuthor
    );

    // And with both, the same call goes through — which is what says the two
    // refusals above were about the author and nothing else.
    author(&dir, "Ada Lovelace", "ada@example.com");
    made(create(&app, root.path(), "verkstead").await);
}

/// A git that will not do its half is named in git's own words, and what it got
/// half way through is taken back: nothing on the registry, and no directory
/// standing where the human asked for a repository.
///
/// The failure is a configured author git will not take — it strips the
/// disallowed characters and finds nothing left — which is a create that gets as
/// far as the directory and the `git init` and stops at the commit.
#[tokio::test]
async fn a_git_that_will_not_commit_leaves_nothing_behind() {
    let root = tempfile::tempdir().unwrap();
    let (dir, app) = workbench().await;

    author(&dir, "<", ">");

    let refused = create(&app, root.path(), "verkstead").await;

    let Created::Refused(why) = refused else {
        panic!("the create was not refused: {refused:?}");
    };
    assert!(
        why.contains("disallowed characters"),
        "git's own words rather than Verkstead's: {why}",
    );

    assert!(listed(&app).await.is_empty(), "nothing was registered");
    assert!(
        !root.path().join("verkstead").exists(),
        "the half-made directory was taken back",
    );
}

/// Ask for a repository on GitHub as well as on the disk, which is the tick in
/// the modal.
async fn create_on_github(app: &Router, parent: &Path, name: &str) -> Created {
    post(
        app,
        "/api/ui/repos/new",
        &serde_json::json!({ "parent": parent, "name": name, "github": true }),
    )
    .await
}

/// A workbench with an author configured and `gh` standing in for the real one,
/// authenticating as whatever token the Data Directory holds — which is how the
/// served router builds its own: the settings rather than the token, so one
/// saved through the page reaches the next call without a restart.
///
/// `notes` is where the stub writes down what it was asked; `answering` is the
/// body of the script, run with Verkstead's arguments from `$1`.
///
/// The script is a `/bin/sh` one, which is what leaves this and the three tests
/// that ask for it off Windows, the way the same stand-in is kept off it in
/// `sharing.rs` and `ui_content.rs`: a stand-in that is a shell script is a
/// stand-in for a machine with a shell at that path. What they are about — what
/// Verkstead asks GitHub for, and what it makes of a refusal — is nothing a
/// platform changes, and the rest of this file is asked wherever the suite runs.
#[cfg(unix)]
async fn workbench_with_gh(answering: &str) -> (tempfile::TempDir, tempfile::TempDir, Router) {
    let dir = tempfile::tempdir().unwrap();
    let notes = tempfile::tempdir().unwrap();

    author(&dir, "Ada Lovelace", "ada@example.com");
    std::fs::write(
        dir.path().join("secrets.yaml"),
        "github_token: ghp_thetokenthatwassaved\n",
    )
    .unwrap();

    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    let data_dir = dir.path().to_owned();
    let gh = Gh::running(vec![
        "/bin/sh".to_owned(),
        "-c".to_owned(),
        format!(
            r#"printf '%s\n' "$*" > "{notes}/asked"
               pwd > "{notes}/where"
               printf '%s\n' "${{GH_TOKEN-unset}}" > "{notes}/token"
               {answering}"#,
            notes = notes.path().display(),
        ),
        "gh".to_owned(),
    ])
    .authenticated_by(Settings::in_data_dir(&data_dir));

    (dir, notes, router_asking_github(pool, data_dir, gh))
}

/// What the stub wrote down under `notes`, one file at a time. Off Windows with
/// the stub itself, whose only readers are the creates that reach GitHub.
#[cfg(unix)]
fn noted(notes: &tempfile::TempDir, what: &str) -> String {
    std::fs::read_to_string(notes.path().join(what))
        .unwrap_or_else(|error| panic!("the stub wrote no {what}: {error}"))
        .trim()
        .to_owned()
}

/// The Repo a create that reached GitHub hands back, which is the same
/// [`Created::Made`] a create with nothing to push hands back: the tick is not a
/// second outcome, it is more of the one create.
#[cfg(unix)]
#[tokio::test]
async fn a_created_repository_is_private_on_github_with_origin_set_and_main_pushed() {
    let root = tempfile::tempdir().unwrap();
    let remote = tempfile::tempdir().unwrap();

    // A bare repository where GitHub's would be, so that the `--push` the stub
    // stands in for is a push that really lands: what comes back then carries
    // the remote-tracking branch, which is the whole claim about `origin`.
    let there = remote.path().join("widgets.git");
    git(remote.path(), &["init", "--bare", "--quiet", "widgets.git"]);

    let (_dir, notes, app) = workbench_with_gh(&format!(
        r#"git remote add origin "{there}"
           git push --quiet --set-upstream origin main"#,
        there = there.display(),
    ))
    .await;

    let repo = made(create_on_github(&app, root.path(), "widgets").await);

    // Private, sourced from the directory, `origin` written and the branch
    // pushed — one `gh` rather than a remote added by hand with a push behind
    // it, which would be two more ways to leave half a remote behind.
    assert_eq!(
        noted(&notes, "asked"),
        "repo create widgets --private --source . --remote origin --push",
    );

    // Run inside the repository it has just made, which is what `--source .`
    // means.
    assert_eq!(
        noted(&notes, "where"),
        repo.path,
        "`gh` was run in the repository it was making a remote for",
    );

    // And as the configured token, read at the moment of the call out of the
    // file the settings page writes.
    assert_eq!(noted(&notes, "token"), "ghp_thetokenthatwassaved");

    // The Repo comes back with the remote already on it: the push happens
    // before the registration, so what the modal is handed is a repository
    // whose `origin` is there rather than one it would have to re-read to see.
    assert_eq!(
        repo.branches,
        vec!["main".to_owned(), "origin/main".to_owned()]
    );
    assert_eq!(listed(&app).await.len(), 1, "and it is on the registry");
}

/// A create that was not asked for a remote does not go near `gh` at all.
#[cfg(unix)]
#[tokio::test]
async fn a_create_with_the_tick_off_asks_github_nothing() {
    let root = tempfile::tempdir().unwrap();
    let (_dir, notes, app) = workbench_with_gh("exit 0").await;

    made(create(&app, root.path(), "widgets").await);

    assert!(
        !notes.path().join("asked").exists(),
        "`gh` was never run for a create with nothing to push",
    );
}

/// A GitHub failure after the local repository exists is not a failed create.
///
/// The directory, the commit and the registration all stand — what is missing is
/// a remote that can be added afterwards — so the answer carries the Repo *and*
/// what failed rather than choosing between them.
#[cfg(unix)]
#[tokio::test]
async fn a_github_that_would_not_make_the_repository_leaves_the_local_one_registered() {
    let root = tempfile::tempdir().unwrap();
    let (_dir, _notes, app) = workbench_with_gh(
        r#"printf 'GraphQL: Name already exists on this account (createRepository)\n' >&2
           exit 1"#,
    )
    .await;

    let outcome = create_on_github(&app, root.path(), "widgets").await;

    let Created::MadeWithoutRemote { repo, why } = outcome else {
        panic!("the create did not answer with both halves: {outcome:?}");
    };

    // `gh`'s own words, the way a git that would not commit is answered in
    // git's: nothing here could put it better.
    assert!(
        why.contains("Name already exists on this account"),
        "what `gh` said rather than what Verkstead makes of it: {why}",
    );

    // The repository is there, on `main`, with its commit — and registered.
    let path = root.path().join("widgets");
    assert_eq!(repo.path, path.canonicalize().unwrap().to_str().unwrap());
    assert_eq!(
        git_says(&path, &["rev-parse", "--abbrev-ref", "HEAD"]),
        "main"
    );
    assert_eq!(
        listed(&app)
            .await
            .iter()
            .map(|repo| repo.id)
            .collect::<Vec<_>>(),
        vec![repo.id],
        "it is on the registry, so the draft it lands on has somewhere to be",
    );
}

/// And a machine with no `gh` on it at all is the same shape of answer: the
/// repository is made, and Verkstead says why there is no remote on it.
#[tokio::test]
async fn a_machine_with_no_gh_leaves_the_local_repository_registered_too() {
    let root = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();

    author(&dir, "Ada Lovelace", "ada@example.com");

    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();
    let data_dir = dir.path().to_owned();
    let app = router_asking_github(
        pool,
        data_dir,
        Gh::running(vec![
            dir.path().join("there-is-no-gh-here").display().to_string(),
        ]),
    );

    let outcome = create_on_github(&app, root.path(), "widgets").await;

    let Created::MadeWithoutRemote { repo, why } = outcome else {
        panic!("the create did not answer with both halves: {outcome:?}");
    };

    assert!(
        why.contains("no `gh` on this machine's PATH"),
        "the sentence that names what to go and do: {why}",
    );
    assert_eq!(listed(&app).await.len(), 1);
    assert_eq!(listed(&app).await[0].id, repo.id);
}
