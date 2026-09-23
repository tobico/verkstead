//! The files API the Code pane's tree stands on, asked of a real router over a
//! real Conversation: which roots there are, and what one folder of one of them
//! holds.
//!
//! What is worth proving out here rather than in the module's own tests is
//! everything that takes a *Conversation* to say. The roots are read off the
//! record — the Conversation's own Worktree and each companion's, in the order
//! it carries them, a read-only one marked — and the module is handed a list
//! that somebody else composed. So the checkouts here are real ones git made,
//! the companions are registered Repos in the modes a human picked, and what
//! comes back is what a browser would be handed.
//!
//! The bound itself is the module's: [`verkstead_server`]'s `files` unit tests
//! ask what one root does with a path that climbs out of it, with a symlink and
//! with `.git`. What is asked here is that the roots those tests are handed are
//! the ones this Conversation really has — which is the half a list built by
//! hand could never say.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde::de::DeserializeOwned;
use sqlx::SqlitePool;
use tower::ServiceExt;
use verkstead_render::{FileRootsView, FolderEntry, FolderListing};
use verkstead_server::{open_database, router, store};

/// A router over a fresh database, and the directory holding both it and every
/// checkout a test makes.
async fn fresh_app() -> (tempfile::TempDir, SqlitePool, Router) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    (dir, pool.clone(), router(pool))
}

/// A Conversation grilling in a Worktree of its own, with a companion checked
/// out beside it per entry in `companions`.
///
/// The shape a grill start really makes: a branch of its own for a read-write
/// companion, and a detached checkout for a read-only one, which has nothing to
/// commit. Lifted from `tests/sets.rs`, whose Diff needs the same thing for the
/// opposite reason — that one is about the Worktrees a session may *write* in,
/// and this is about the ones the human may read.
async fn grilling_alongside(
    pool: &SqlitePool,
    dir: &Path,
    companions: &[(&str, store::CompanionMode)],
) -> (i64, PathBuf, Vec<PathBuf>) {
    let repo = repository(dir.join("verkstead"));
    let registered = store::register_repo(pool, &repo, "verkstead", "main")
        .await
        .unwrap()
        .expect("nothing is registered at that path yet");

    let conversation = store::start_conversation(pool, registered.id, "code-pane")
        .await
        .unwrap()
        .expect("the Repo was just registered");

    let worktree = dir.join("worktrees/verkstead-code-pane");
    let base = git(&repo, &["rev-parse", "HEAD"]).trim().to_owned();
    git(
        &repo,
        &[
            "worktree",
            "add",
            "-b",
            "code-pane",
            &worktree.to_string_lossy(),
            &base,
        ],
    );

    let mut checkouts = Vec::new();

    for (name, mode) in companions {
        let path = repository(dir.join(name));
        let companion = store::register_repo(pool, &path, name, "main")
            .await
            .unwrap()
            .expect("nothing is registered at that path yet");

        store::add_companion(pool, conversation, companion.id)
            .await
            .unwrap();
        store::configure_companion(pool, conversation, companion.id, store::Change::Mode(*mode))
            .await
            .unwrap();

        let at = git(&path, &["rev-parse", "HEAD"]).trim().to_owned();
        let checkout = dir.join(format!("worktrees/{name}-code-pane"));

        let cut = match mode {
            store::CompanionMode::ReadOnly => vec!["worktree", "add", "--detach"],
            store::CompanionMode::ReadWrite => vec!["worktree", "add", "-b", "code-pane"],
        };
        git(
            &path,
            &[&cut[..], &[&checkout.to_string_lossy(), &at]].concat(),
        );

        checkouts.push(store::CompanionWorktree {
            repo_id: companion.id,
            path: checkout,
            base_commit: Some(at),
        });
    }

    store::start_grilling(pool, conversation, &base, &worktree, &checkouts)
        .await
        .unwrap();

    (
        conversation,
        worktree,
        checkouts.into_iter().map(|made| made.path).collect(),
    )
}

/// A git repository with one commit in it, ignoring what a Rust checkout
/// ignores.
fn repository(path: PathBuf) -> PathBuf {
    std::fs::create_dir_all(&path).unwrap();
    git(&path, &["init", "--initial-branch", "main"]);
    git(&path, &["config", "user.email", "tests@verkstead.invalid"]);
    git(&path, &["config", "user.name", "Verkstead Tests"]);
    git(&path, &["config", "commit.gpgsign", "false"]);
    std::fs::write(path.join("README.md"), "# a repository\n").unwrap();
    std::fs::write(path.join(".gitignore"), "target/\n").unwrap();
    git(&path, &["add", "-A"]);
    git(&path, &["commit", "-m", "first"]);

    path
}

/// Run git in `dir`, insisting it worked. Scaffolding rather than the code under
/// test, so a failure here is a broken test.
fn git(dir: &Path, args: &[&str]) -> String {
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

    String::from_utf8(output.stdout).unwrap()
}

/// Ask the viewer's namespace for something, and read back what it answered.
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

    serde_json::from_str(&body).unwrap_or_else(|error| panic!("{path} answered {body}: {error}"))
}

/// One folder of a Conversation's roots, asked for the way the tree asks.
///
/// The path goes on the query string as it is spelled: every directory these
/// tests make is under a temporary one, whose name is letters and digits, and a
/// `/` is a character a query value holds without escaping.
async fn folder(app: &Router, conversation: i64, at: &Path) -> FolderListing {
    get(
        app,
        &format!(
            "/api/ui/conversations/{conversation}/files/folder?path={}",
            at.display()
        ),
    )
    .await
}

/// The rows of a listing, or a panic saying what came back instead.
fn listed(listing: FolderListing) -> Vec<FolderEntry> {
    match listing {
        FolderListing::Listed { entries, .. } => entries,
        other => panic!("expected a listing, got {other:?}"),
    }
}

/// Their names, which is what the tree draws.
fn names(listing: FolderListing) -> Vec<String> {
    listed(listing).into_iter().map(|row| row.name).collect()
}

/// The Conversation's own Worktree first and each companion's after it, in the
/// order the Conversation carries them — with a read-only companion among them,
/// marked, which is where this parts company with the Diff a Set carries.
#[tokio::test]
async fn the_roots_are_the_conversations_worktrees_own_first() {
    let (dir, pool, app) = fresh_app().await;
    let (conversation, worktree, companions) = grilling_alongside(
        &pool,
        dir.path(),
        &[
            ("askance", store::CompanionMode::ReadWrite),
            ("verkstead-site", store::CompanionMode::ReadOnly),
        ],
    )
    .await;

    let view: FileRootsView = get(
        &app,
        &format!("/api/ui/conversations/{conversation}/files/roots"),
    )
    .await;

    assert_eq!(
        view.roots
            .iter()
            .map(|root| (
                root.repo.as_str(),
                root.path.as_str(),
                root.own,
                root.writable
            ))
            .collect::<Vec<_>>(),
        vec![
            ("verkstead", worktree.to_str().unwrap(), true, true),
            ("askance", companions[0].to_str().unwrap(), false, true),
            (
                "verkstead-site",
                companions[1].to_str().unwrap(),
                false,
                false
            ),
        ]
    );
}

/// A Conversation with nothing checked out has no roots, which is a tree with
/// nothing in it rather than anything to report.
#[tokio::test]
async fn a_conversation_with_no_worktree_has_no_roots() {
    let (_dir, pool, app) = fresh_app().await;

    let repo = store::register_repo(&pool, Path::new("/srv/verkstead"), "verkstead", "main")
        .await
        .unwrap()
        .expect("nothing is registered at that path yet");
    let drafting = store::start_conversation(&pool, repo.id, "code-pane")
        .await
        .unwrap()
        .expect("the Repo was just registered");

    let view: FileRootsView = get(
        &app,
        &format!("/api/ui/conversations/{drafting}/files/roots"),
    )
    .await;

    assert!(view.roots.is_empty(), "{:?}", view.roots);

    // And a Conversation nothing at all knows about, which is the same nothing:
    // the roots are a reading of the record rather than a record of their own.
    let none: FileRootsView = get(&app, "/api/ui/conversations/404/files/roots").await;
    assert!(none.roots.is_empty(), "{:?}", none.roots);
}

/// Expanding a folder reads that folder alone, and what git ignores is not in
/// it — nor is the git directory.
#[tokio::test]
async fn a_folder_is_read_alone_without_what_git_ignores_or_the_git_directory() {
    let (dir, pool, app) = fresh_app().await;
    let (conversation, worktree, _) = grilling_alongside(&pool, dir.path(), &[]).await;

    std::fs::create_dir_all(worktree.join("crates/server")).unwrap();
    std::fs::create_dir_all(worktree.join("target/debug")).unwrap();
    std::fs::write(worktree.join("Cargo.toml"), "[workspace]\n").unwrap();
    std::fs::write(worktree.join("crates/server/lib.rs"), "").unwrap();

    // The root itself: folders first, then by name, with `target/` and the
    // checkout's `.git` file left out.
    assert_eq!(
        names(folder(&app, conversation, &worktree).await),
        ["crates", ".gitignore", "Cargo.toml", "README.md"]
    );

    // And one folder down, which is one reading of one directory: what is under
    // `crates/server` is not in the answer for `crates`.
    assert_eq!(
        names(folder(&app, conversation, &worktree.join("crates")).await),
        ["server"]
    );
}

/// A companion's folders are read the same way, which is what says the tree is
/// over every Worktree rather than over the work's own.
#[tokio::test]
async fn a_companions_folders_are_read_like_the_conversations_own() {
    let (dir, pool, app) = fresh_app().await;
    let (conversation, _, companions) = grilling_alongside(
        &pool,
        dir.path(),
        &[("askance", store::CompanionMode::ReadOnly)],
    )
    .await;

    std::fs::write(companions[0].join("ASKING.md"), "# asking\n").unwrap();

    assert_eq!(
        names(folder(&app, conversation, &companions[0]).await),
        [".gitignore", "ASKING.md", "README.md"]
    );
}

/// And the refusals, each its own sentence in the body rather than a status
/// code: a path under no root of this Conversation, and a path under `.git`.
#[tokio::test]
async fn a_path_outside_a_root_and_one_under_git_each_say_what_they_are() {
    let (dir, pool, app) = fresh_app().await;
    let (conversation, worktree, _) = grilling_alongside(&pool, dir.path(), &[]).await;

    // The Repo the Worktree was cut from is not a root: it is a directory of the
    // human's that this Conversation is not working in.
    assert_eq!(
        folder(&app, conversation, &dir.path().join("verkstead")).await,
        FolderListing::Outside
    );

    assert_eq!(
        folder(&app, conversation, &worktree.join(".git")).await,
        FolderListing::UnderGit
    );

    // Another Conversation's Worktree is outside this one's roots too, which is
    // what conversation-scoping the endpoint means.
    let (other, mine, _) = grilling_alongside(&pool, &dir.path().join("second"), &[]).await;
    assert_eq!(
        folder(&app, conversation, &mine).await,
        FolderListing::Outside
    );
    assert!(matches!(
        folder(&app, other, &mine).await,
        FolderListing::Listed { .. }
    ));
}

/// A Worktree that has gone says so, rather than reading as a folder that has:
/// one filesystem answer, two different things for a human to be told.
#[tokio::test]
async fn a_worktree_that_is_gone_is_told_apart_from_a_folder_that_is() {
    let (dir, pool, app) = fresh_app().await;
    let (conversation, worktree, _) = grilling_alongside(&pool, dir.path(), &[]).await;

    assert_eq!(
        folder(&app, conversation, &worktree.join("never-made")).await,
        FolderListing::Missing
    );

    std::fs::remove_dir_all(&worktree).unwrap();

    assert_eq!(
        folder(&app, conversation, &worktree).await,
        FolderListing::RootGone
    );
}
