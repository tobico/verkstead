//! The files API the Code pane's tree stands on, asked of a real router over a
//! real Conversation: which roots there are, what one folder of one of them
//! holds, what one file of one of those is — and that file saved back.
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
use base64::Engine;
use http_body_util::BodyExt;
use serde::de::DeserializeOwned;
use sqlx::SqlitePool;
use tower::ServiceExt;
use verkstead_render::{
    FileReading, FileRootsView, FileWrite, FileWritten, FolderEntry, FolderListing,
};
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

/// One file of one of those roots, asked for the way a tab opens one.
async fn file(app: &Router, conversation: i64, at: &Path) -> FileReading {
    get(
        app,
        &format!(
            "/api/ui/conversations/{conversation}/files/file?path={}",
            at.display()
        ),
    )
    .await
}

/// A one-pixel PNG: enough for the endpoint to have something to send back and
/// name, which is the whole of what the image half is about out here.
const PIXEL: &[u8] = &[
    0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, b'I', b'H', b'D', b'R',
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f, 0x15, 0xc4,
    0x89,
];

/// A file pressed in the tree comes back as what is in it, with a version —
/// which is what a write will later name itself as being over.
#[tokio::test]
async fn a_file_of_the_worktree_comes_back_with_its_contents_and_a_version() {
    let (dir, pool, app) = fresh_app().await;
    let (conversation, worktree, _) = grilling_alongside(&pool, dir.path(), &[]).await;

    std::fs::create_dir_all(worktree.join("crates/server")).unwrap();
    let at = worktree.join("crates/server/lib.rs");
    std::fs::write(&at, "//! the server\n").unwrap();

    let FileReading::Text {
        path,
        version,
        text,
        writable,
    } = file(&app, conversation, &at).await
    else {
        panic!("expected text");
    };

    assert_eq!(path, at.display().to_string());
    assert_eq!(text, "//! the server\n");
    assert!(writable, "the conversation's own worktree is written in");
    assert!(!version.is_empty());

    // And the version is of the bytes: a file somebody rewrote between two
    // reads comes back under a different one, which is what a refused stale
    // write stands on.
    std::fs::write(&at, "//! the server, rewritten\n").unwrap();

    let FileReading::Text { version: after, .. } = file(&app, conversation, &at).await else {
        panic!("expected text");
    };

    assert_ne!(version, after);
}

/// A file in a read-only companion opens read-only: the root's own flag, which
/// is what saves a human finding out by typing.
#[tokio::test]
async fn a_file_in_a_read_only_companion_opens_read_only() {
    let (dir, pool, app) = fresh_app().await;
    let (conversation, worktree, companions) = grilling_alongside(
        &pool,
        dir.path(),
        &[("askance", store::CompanionMode::ReadOnly)],
    )
    .await;

    let FileReading::Text { writable, .. } =
        file(&app, conversation, &companions[0].join("README.md")).await
    else {
        panic!("expected text");
    };

    assert!(!writable);

    // And the Conversation's own is written in, which is the other half of the
    // same sentence.
    let FileReading::Text { writable: own, .. } =
        file(&app, conversation, &worktree.join("README.md")).await
    else {
        panic!("expected text");
    };

    assert!(own);
}

/// And each of the other three kinds: a picture to draw, a binary that is a
/// line rather than bytes on the wire, and a file too large to open.
#[tokio::test]
async fn a_picture_a_binary_and_a_file_over_the_cap_each_say_what_they_are() {
    let (dir, pool, app) = fresh_app().await;
    let (conversation, worktree, _) = grilling_alongside(&pool, dir.path(), &[]).await;

    let picture = worktree.join("icon.png");
    std::fs::write(&picture, PIXEL).unwrap();

    let FileReading::Image {
        media_type, base64, ..
    } = file(&app, conversation, &picture).await
    else {
        panic!("expected an image");
    };

    assert_eq!(media_type, "image/png");
    assert_eq!(
        base64::engine::general_purpose::STANDARD
            .decode(base64)
            .unwrap(),
        PIXEL
    );

    let object = worktree.join("verkstead.o");
    std::fs::write(&object, [0x7f, b'E', b'L', b'F', 0x00, 0x01]).unwrap();
    assert_eq!(file(&app, conversation, &object).await, FileReading::Binary);

    let log = worktree.join("build.log");
    std::fs::write(&log, vec![b'x'; 2 * 1000 * 1000 + 1]).unwrap();
    assert_eq!(file(&app, conversation, &log).await, FileReading::TooLarge);
}

/// And a file is bounded by the roots the way a folder is: this Conversation's
/// checkouts and nothing else on the machine.
#[tokio::test]
async fn a_file_outside_this_conversations_roots_is_refused() {
    let (dir, pool, app) = fresh_app().await;
    let (conversation, worktree, _) = grilling_alongside(&pool, dir.path(), &[]).await;

    // The Repo the Worktree was cut from: a directory of the human's that this
    // Conversation is not working in.
    assert_eq!(
        file(&app, conversation, &dir.path().join("verkstead/README.md")).await,
        FileReading::Outside
    );

    assert_eq!(
        file(&app, conversation, &worktree.join(".git/config")).await,
        FileReading::UnderGit
    );

    assert_eq!(
        file(&app, conversation, &worktree.join("never-written")).await,
        FileReading::Missing
    );

    // And a folder is a row the tree expands rather than a tab it opens.
    assert_eq!(
        file(&app, conversation, &worktree).await,
        FileReading::NotAFile
    );
}

/// One of them written back, the way Ctrl+S in a tab writes it: the path, the
/// version the read handed over, and the text.
async fn save(app: &Router, conversation: i64, at: &Path, over: &str, text: &str) -> FileWritten {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/ui/conversations/{conversation}/files/file"
                ))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&FileWrite {
                        path: at.display().to_string(),
                        version: over.to_owned(),
                        text: text.to_owned(),
                    })
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body = String::from_utf8(bytes.to_vec()).unwrap();

    assert_eq!(status, StatusCode::OK, "the save failed: {body}");

    serde_json::from_str(&body).unwrap_or_else(|error| panic!("the save answered {body}: {error}"))
}

/// The version a reading carries, or a panic saying what came back instead.
fn versioned(reading: FileReading) -> String {
    match reading {
        FileReading::Text { version, .. } => version,
        other => panic!("expected text, got {other:?}"),
    }
}

/// A save lands on the disk of the Worktree it was read out of, and answers
/// with the version that file now has — which is what the next save names.
#[tokio::test]
async fn a_save_over_the_version_that_was_read_lands_in_the_worktree() {
    let (dir, pool, app) = fresh_app().await;
    let (conversation, worktree, _) = grilling_alongside(&pool, dir.path(), &[]).await;

    let at = worktree.join("README.md");
    let was = versioned(file(&app, conversation, &at).await);

    let saved = save(&app, conversation, &at, &was, "# saved from the tab\n").await;

    let FileWritten::Written { version } = saved else {
        panic!("expected a write, got {saved:?}");
    };

    assert_eq!(
        std::fs::read_to_string(&at).unwrap(),
        "# saved from the tab\n"
    );
    assert_eq!(version, versioned(file(&app, conversation, &at).await));
}

/// And a save over a file the agent has changed since is refused — which is
/// what the Reload / Keep mine bar is drawn from, both halves of which come
/// back for a fresh read before the save that lands.
#[tokio::test]
async fn a_save_over_a_file_that_has_moved_is_refused() {
    let (dir, pool, app) = fresh_app().await;
    let (conversation, worktree, _) = grilling_alongside(&pool, dir.path(), &[]).await;

    let at = worktree.join("README.md");
    let was = versioned(file(&app, conversation, &at).await);

    // The agent, writing the same file while the human had it open — which on
    // this Conversation is a session in the Sandbox and here is the same write.
    std::fs::write(&at, "# the agent got here first\n").unwrap();

    assert_eq!(
        save(&app, conversation, &at, &was, "# mine\n").await,
        FileWritten::Stale
    );

    // Nothing was written: the collision is the point.
    assert_eq!(
        std::fs::read_to_string(&at).unwrap(),
        "# the agent got here first\n"
    );

    // And *Keep mine* is a fresh read followed by a save over the version it
    // hands over, which lands.
    let now = versioned(file(&app, conversation, &at).await);

    assert!(matches!(
        save(&app, conversation, &at, &now, "# mine\n").await,
        FileWritten::Written { .. }
    ));
    assert_eq!(std::fs::read_to_string(&at).unwrap(), "# mine\n");
}

/// A save into a read-only companion is refused with its own sentence: the
/// root's own flag, which is the same thing that opened its editor read-only.
#[tokio::test]
async fn a_save_into_a_read_only_companion_is_refused() {
    let (dir, pool, app) = fresh_app().await;
    let (conversation, worktree, companions) = grilling_alongside(
        &pool,
        dir.path(),
        &[("askance", store::CompanionMode::ReadOnly)],
    )
    .await;

    let at = companions[0].join("README.md");
    let was = versioned(file(&app, conversation, &at).await);

    assert_eq!(
        save(&app, conversation, &at, &was, "# mine\n").await,
        FileWritten::ReadOnly
    );
    assert_ne!(std::fs::read_to_string(&at).unwrap(), "# mine\n");

    // And the Conversation's own Worktree takes the same save, which is the
    // other half of the sentence.
    let own = worktree.join("README.md");
    let over = versioned(file(&app, conversation, &own).await);

    assert!(matches!(
        save(&app, conversation, &own, &over, "# mine\n").await,
        FileWritten::Written { .. }
    ));
}

/// And a save is bounded by the roots the way a read is: this Conversation's
/// checkouts and nothing else on the machine, whichever way the bytes go.
#[tokio::test]
async fn a_save_outside_this_conversations_roots_is_refused() {
    let (dir, pool, app) = fresh_app().await;
    let (conversation, worktree, _) = grilling_alongside(&pool, dir.path(), &[]).await;

    // The Repo the Worktree was cut from: a directory of the human's that this
    // Conversation is not working in.
    let elsewhere = dir.path().join("verkstead/README.md");
    let before = std::fs::read_to_string(&elsewhere).unwrap();

    assert_eq!(
        save(&app, conversation, &elsewhere, "", "# mine\n").await,
        FileWritten::Outside
    );
    assert_eq!(std::fs::read_to_string(&elsewhere).unwrap(), before);

    assert_eq!(
        save(&app, conversation, &worktree.join(".git/config"), "", "").await,
        FileWritten::UnderGit
    );
    assert_eq!(
        save(&app, conversation, &worktree.join("never-written"), "", "").await,
        FileWritten::Missing
    );
}

/// And the largest file Code will open is one it will also save.
///
/// The read's cap is decimal and axum's own body limit is two *mebibytes*,
/// which is under it — so without a limit of its own on this route a file just
/// inside the cap would be one the tree opens, the editor takes typing in, and
/// the save is refused for by a status rather than by any of this API's
/// sentences. See `MAX_WRITE_BYTES` in the server's `files`.
///
/// Written in short lines, because that is where a text file's JSON escaping
/// comes from: every newline is two bytes on the wire, so a file of short lines
/// near the cap is a body well over two mebibytes — which is the whole of why
/// the limit is not simply the read's. A hundred and thirty thousand short
/// lines is a generated file or a column of data, not a contrivance.
#[tokio::test]
async fn a_file_at_the_top_of_what_code_opens_is_still_one_it_saves() {
    let (dir, pool, app) = fresh_app().await;
    let (conversation, worktree, _) = grilling_alongside(&pool, dir.path(), &[]).await;

    // Just inside the cap, and short enough that what JSON makes of the
    // newlines carries the body past two mebibytes on its own.
    let line = format!("{}\n", "x".repeat(14));
    let big: String = line.repeat(1_999_000 / line.len());

    let at = worktree.join("generated.rs");
    std::fs::write(&at, &big).unwrap();

    let was = versioned(file(&app, conversation, &at).await);
    let saved = save(&app, conversation, &at, &was, &big).await;

    let FileWritten::Written { .. } = saved else {
        panic!("expected a write, got {saved:?}");
    };

    assert_eq!(std::fs::read_to_string(&at).unwrap(), big);
}
