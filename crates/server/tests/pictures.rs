//! A picture a diff names, served by its blob id (ADR-0026, *Fetched by blob
//! id*): the ids on a Set's composed Diff and on a commit's patch are whole,
//! and the endpoint hands back the bytes each one names — out of the object
//! database, or out of the Worktree while a file is only there.
//!
//! What is proven here rather than in the module's own tests is everything that
//! takes a *Conversation* to say: that the ids come off a Diff the server really
//! composed, that `repo` resolves to the Worktree that Diff was read out of, and
//! what goes out on the wire. The bound and the hashing are the module's.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use base64::prelude::{BASE64_STANDARD, Engine as _};
use http_body_util::BodyExt;
use sqlx::SqlitePool;
use tower::ServiceExt;
use verkstead_render::SharedConversation;
use verkstead_schema::{ApiError, SetCreated};
use verkstead_server::{open_database, router, store};

/// The device every Conversation started here is ranked by, named the way a
/// cluster names one (ADR-0020, *Ranks*).
const THIS_DEVICE: &str = "aa00bb11cc22dd33ee44ff5566778899";

/// A Set with nothing in it but a question, which is all it takes for the
/// server to compose a Diff onto it.
const A_SET: &str = r#"
title: How the logo looks
project: verkstead
branch: svg-diff-previews
questions:
  - label: Q1
    text: Is the new logo right?
"#;

const LOGO: &[u8] = b"<svg xmlns=\"http://www.w3.org/2000/svg\"><circle r=\"4\"/></svg>\n";
const REDRAWN: &[u8] = b"<svg xmlns=\"http://www.w3.org/2000/svg\"><rect width=\"8\"/></svg>\n";
const BADGE: &[u8] = b"<svg xmlns=\"http://www.w3.org/2000/svg\"><path d=\"M0 0\"/></svg>\n";

/// A router over a fresh database, and the directory holding both it and every
/// checkout a test makes.
async fn fresh_app() -> (tempfile::TempDir, SqlitePool, Router) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    (dir, pool.clone(), router(pool))
}

/// A Conversation grilling in a Worktree of its own, with a read-write companion
/// `askance` and a read-only companion `verkstead-site` checked out beside it —
/// the shapes a grill start really makes. Each repository's first commit holds
/// `logo.svg`.
///
/// Lifted from `tests/files.rs`, which needs the same for the Code pane.
async fn grilling(pool: &SqlitePool, dir: &Path) -> (i64, PathBuf, Vec<PathBuf>) {
    let repo = repository(dir.join("verkstead"));
    let registered = store::register_repo(pool, &repo, "verkstead", "main")
        .await
        .unwrap()
        .expect("nothing is registered at that path yet");

    let conversation =
        store::start_conversation(pool, registered.id, "svg-diff-previews", THIS_DEVICE)
            .await
            .unwrap()
            .expect("the Repo was just registered");

    let worktree = dir.join("worktrees/verkstead-svg-diff-previews");
    let base = git(&repo, &["rev-parse", "HEAD"]);
    git(
        &repo,
        &[
            "worktree",
            "add",
            "-b",
            "svg-diff-previews",
            &worktree.to_string_lossy(),
            &base,
        ],
    );

    let mut checkouts = Vec::new();

    for (name, mode) in [
        ("askance", store::CompanionMode::ReadWrite),
        ("verkstead-site", store::CompanionMode::ReadOnly),
    ] {
        let path = repository(dir.join(name));
        let companion = store::register_repo(pool, &path, name, "main")
            .await
            .unwrap()
            .expect("nothing is registered at that path yet");

        store::add_companion(pool, conversation, companion.id)
            .await
            .unwrap();
        store::configure_companion(pool, conversation, companion.id, store::Change::Mode(mode))
            .await
            .unwrap();

        let at = git(&path, &["rev-parse", "HEAD"]);
        let checkout = dir.join(format!("worktrees/{name}-svg-diff-previews"));

        let cut = match mode {
            store::CompanionMode::ReadOnly => vec!["worktree", "add", "--detach"],
            store::CompanionMode::ReadWrite => vec!["worktree", "add", "-b", "svg-diff-previews"],
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

/// A git repository with one commit in it holding `logo.svg`.
fn repository(path: PathBuf) -> PathBuf {
    std::fs::create_dir_all(&path).unwrap();
    git(&path, &["init", "--initial-branch", "main"]);
    git(&path, &["config", "user.email", "tests@verkstead.invalid"]);
    git(&path, &["config", "user.name", "Verkstead Tests"]);
    git(&path, &["config", "commit.gpgsign", "false"]);
    std::fs::write(path.join("logo.svg"), LOGO).unwrap();
    git(&path, &["add", "-A"]);
    git(&path, &["commit", "-m", "first"]);

    path
}

/// Run git in `dir`, insisting it worked, and take what it said trimmed.
/// Scaffolding rather than the code under test, so a failure here is a broken
/// test.
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

    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

/// Ask a Set from the Conversation, and take the Diff the server composed onto
/// it: each block's repository and patch.
async fn composed(pool: &SqlitePool, conversation: i64) -> Vec<(String, String)> {
    let response = router(pool.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/conversations/{conversation}/api/v1/sets"))
                .header(header::CONTENT_TYPE, "application/yaml")
                .body(Body::from(A_SET))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body = String::from_utf8(bytes.to_vec()).unwrap();
    assert_eq!(status, StatusCode::CREATED, "the Set was refused: {body}");

    let created: SetCreated = serde_saphyr::from_str(&body).unwrap();
    let stored = store::load_set(pool, created.id).await.unwrap().unwrap();

    stored
        .set
        .set()
        .expect("a Set this build just stored reads back")
        .diffs
        .iter()
        .map(|block| (block.repo.clone(), block.diff.clone()))
        .collect()
}

/// Every `index` line of a patch: the path it is about, the ids before the
/// `..` — one per parent on a combined diff — and the id after it.
fn indexed(patch: &str) -> Vec<(String, Vec<String>, String)> {
    let mut lines = Vec::new();
    let mut path = String::new();

    for line in patch.lines() {
        if let Some(header) = line.strip_prefix("diff --git ") {
            path = header
                .rsplit_once(" b/")
                .map(|(_, after)| after.to_owned())
                .unwrap_or_default();
        } else if let Some(header) = line.strip_prefix("diff --cc ") {
            path = header.to_owned();
        } else if let Some(ids) = line.strip_prefix("index ") {
            let ids = ids.split(' ').next().unwrap();
            let (before, after) = ids.split_once("..").unwrap();
            lines.push((
                path.clone(),
                before.split(',').map(str::to_owned).collect(),
                after.to_owned(),
            ));
        }
    }

    lines
}

/// Whether `id` is a whole SHA-1, which is what `--full-index` makes of every
/// id on the line.
fn whole(id: &str) -> bool {
    id.len() == 40 && id.bytes().all(|byte| byte.is_ascii_hexdigit())
}

/// Ask for one picture, and take back the status, the three headers that say
/// how it goes out, and the body.
async fn picture(
    app: &Router,
    conversation: i64,
    repo: &str,
    id: &str,
    path: &str,
) -> (StatusCode, [Option<String>; 3], Vec<u8>) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/ui/conversations/{conversation}/pictures?repo={repo}&id={id}&path={path}"
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    let headed = |name: header::HeaderName| {
        response
            .headers()
            .get(name)
            .map(|value| value.to_str().unwrap().to_owned())
    };
    let headers = [
        headed(header::CONTENT_TYPE),
        headed(header::X_CONTENT_TYPE_OPTIONS),
        headed(header::CONTENT_DISPOSITION),
    ];
    let bytes = response.into_body().collect().await.unwrap().to_bytes();

    (status, headers, bytes.to_vec())
}

/// The sentence a refusal carries.
fn said(body: &[u8]) -> String {
    serde_json::from_slice::<ApiError>(body)
        .unwrap_or_else(|_| panic!("not a refusal: {}", String::from_utf8_lossy(body)))
        .error
}

/// Every file in a composed Diff — a tracked edit, a file staged and never
/// committed, and one git has never heard of — names whole ids, in the
/// Conversation's own block and in a read-write companion's alike.
#[tokio::test]
async fn every_file_in_a_sets_diff_names_whole_ids() {
    let (dir, pool, _app) = fresh_app().await;
    let (conversation, worktree, companions) = grilling(&pool, dir.path()).await;

    std::fs::write(worktree.join("logo.svg"), REDRAWN).unwrap();
    std::fs::write(worktree.join("staged.svg"), BADGE).unwrap();
    git(&worktree, &["add", "staged.svg"]);
    std::fs::write(worktree.join("badge.svg"), BADGE).unwrap();
    std::fs::write(companions[0].join("logo.svg"), REDRAWN).unwrap();

    let blocks = composed(&pool, conversation).await;
    assert_eq!(
        blocks
            .iter()
            .map(|(repo, _)| repo.as_str())
            .collect::<Vec<_>>(),
        ["verkstead", "askance"]
    );

    let own = indexed(&blocks[0].1);
    assert_eq!(
        own.iter()
            .map(|(path, ..)| path.as_str())
            .collect::<Vec<_>>(),
        ["logo.svg", "staged.svg", "badge.svg"],
        "every file has its index line:\n{}",
        blocks[0].1
    );

    for (repo, patch) in &blocks {
        for (path, before, after) in indexed(patch) {
            assert!(
                before.iter().all(|id| whole(id)) && whole(&after),
                "{repo} {path}: {before:?}..{after}"
            );
        }
    }
}

/// A committed blob and a staged one, each named by a Set's Diff, come back
/// byte for byte — by the whole id and by seven characters of it — and out of
/// a companion's repository as well as the Conversation's own.
#[tokio::test]
async fn a_stored_blob_named_by_a_sets_diff_comes_back_byte_for_byte() {
    let (dir, pool, app) = fresh_app().await;
    let (conversation, worktree, companions) = grilling(&pool, dir.path()).await;

    std::fs::write(worktree.join("logo.svg"), REDRAWN).unwrap();
    git(&worktree, &["add", "logo.svg"]);
    std::fs::write(companions[0].join("logo.svg"), REDRAWN).unwrap();

    let blocks = composed(&pool, conversation).await;

    for (repo, patch) in &blocks {
        let [(path, before, after)] = &indexed(patch)[..] else {
            panic!("one file in {repo}'s block:\n{patch}");
        };

        for id in [&before[0], &before[0][..7].to_owned()] {
            let (status, headers, body) = picture(&app, conversation, repo, id, path).await;
            assert_eq!(status, StatusCode::OK, "{repo} {id}: {}", said(&body));
            assert_eq!(body, LOGO, "the committed side, out of {repo}");
            assert_eq!(
                headers,
                [
                    Some("application/octet-stream".to_owned()),
                    Some("nosniff".to_owned()),
                    Some("attachment".to_owned()),
                ],
                "a download, never a page"
            );
        }

        // Staged in the Conversation's own, so in the object database; only in
        // the Worktree in the companion's, which the stand-in serves.
        for id in [after, &after[..7].to_owned()] {
            let (status, _, body) = picture(&app, conversation, repo, id, path).await;
            assert_eq!(status, StatusCode::OK, "{repo} {id}: {}", said(&body));
            assert_eq!(body, REDRAWN, "the changed side, out of {repo}");
        }
    }

    // And the staged blob is served out of git, not off the disk: the Worktree
    // can move on and the staged id still names what was staged.
    std::fs::write(worktree.join("logo.svg"), BADGE).unwrap();
    let (_, _, staged) = indexed(&blocks[0].1).remove(0);
    let (status, _, body) = picture(&app, conversation, "verkstead", &staged, "logo.svg").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, REDRAWN);
}

/// A blob named by a commit's patch is read out of the same repository, a
/// merge's parents included, by the ids the commit pane's patch carries.
#[tokio::test]
async fn a_blob_named_by_a_commit_comes_back() {
    let (dir, pool, app) = fresh_app().await;
    let (conversation, worktree, _) = grilling(&pool, dir.path()).await;

    std::fs::write(worktree.join("logo.svg"), REDRAWN).unwrap();
    git(&worktree, &["commit", "-am", "redraw the logo"]);

    let patch = git(
        &worktree,
        &[
            "diff-tree",
            "--no-commit-id",
            "-p",
            "--cc",
            "--root",
            "--full-index",
            "HEAD",
        ],
    );
    let [(path, before, after)] = &indexed(&patch)[..] else {
        panic!("one file in the commit:\n{patch}");
    };

    for (id, bytes) in [(&before[0], LOGO), (after, REDRAWN)] {
        for asked in [id.as_str(), &id[..7]] {
            let (status, _, body) = picture(&app, conversation, "verkstead", asked, path).await;
            assert_eq!(status, StatusCode::OK, "{asked}: {}", said(&body));
            assert_eq!(body, bytes);
        }
    }
}

/// A file only in the Worktree is served while it still hashes to the id its
/// Diff named, and refused with a sentence once it has changed.
#[tokio::test]
async fn a_file_only_in_the_worktree_is_served_until_it_changes() {
    let (dir, pool, app) = fresh_app().await;
    let (conversation, worktree, _) = grilling(&pool, dir.path()).await;

    std::fs::write(worktree.join("badge.svg"), BADGE).unwrap();
    let blocks = composed(&pool, conversation).await;
    let [(path, _, after)] = &indexed(&blocks[0].1)[..] else {
        panic!("one untracked file:\n{}", blocks[0].1);
    };

    let (status, _, body) = picture(&app, conversation, "verkstead", after, path).await;
    assert_eq!(status, StatusCode::OK, "{}", said(&body));
    assert_eq!(body, BADGE);

    std::fs::write(worktree.join("badge.svg"), REDRAWN).unwrap();

    let (status, _, body) = picture(&app, conversation, "verkstead", after, path).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(said(&body), "This picture is no longer there.");
}

/// The stand-in reaches nothing outside the Worktree and nothing under `.git`,
/// and a repository the Conversation has no Worktree of — a read-only
/// companion's, or one it does not carry at all — is refused before anything
/// is read.
#[tokio::test]
async fn what_the_endpoint_will_not_reach_is_refused() {
    let (dir, pool, app) = fresh_app().await;
    let (conversation, worktree, _) = grilling(&pool, dir.path()).await;

    std::fs::write(dir.path().join("outside.svg"), BADGE).unwrap();
    std::fs::write(worktree.join("badge.svg"), BADGE).unwrap();
    let id = git(&worktree, &["hash-object", "badge.svg"]);

    let (status, _, body) =
        picture(&app, conversation, "verkstead", &id, "../../outside.svg").await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(said(&body), "That isn't in this repository's worktree.");

    // A Worktree's `.git` is a file naming the admin directory elsewhere, and
    // a path under it is refused for its spelling, whatever is there.
    let (status, _, body) = picture(&app, conversation, "verkstead", &id, ".git/badge.svg").await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(
        said(&body),
        "Pictures aren't read from inside a repository's .git."
    );

    for repo in ["verkstead-site", "elsewhere"] {
        let (status, _, body) = picture(&app, conversation, repo, &id, "badge.svg").await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{repo}");
        assert_eq!(
            said(&body),
            "This conversation has no worktree of that repository."
        );
    }
}

/// A blob over 2 MB is refused with a sentence, asked of git's own count of it,
/// and so is a Worktree file that size.
#[tokio::test]
async fn a_picture_over_two_megabytes_is_refused() {
    let (dir, pool, app) = fresh_app().await;
    let (conversation, worktree, _) = grilling(&pool, dir.path()).await;

    std::fs::write(worktree.join("large.png"), vec![0u8; 2_000_001]).unwrap();
    let id = git(&worktree, &["hash-object", "large.png"]);

    let (status, _, body) = picture(&app, conversation, "verkstead", &id, "large.png").await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        said(&body),
        "This picture is larger than 2 MB, so it isn't drawn."
    );

    git(&worktree, &["add", "large.png"]);
    std::fs::remove_file(worktree.join("large.png")).unwrap();

    let (status, _, body) = picture(&app, conversation, "verkstead", &id, "large.png").await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        said(&body),
        "This picture is larger than 2 MB, so it isn't drawn."
    );
}

// And in a share, which is one file with no server behind it: every picture
// the server can read is written into its placeholder as the file is made
// (ADR-0026, *Shares*), by the rules the endpoint above serves it by.

/// The share of a Conversation, as the file is made from it.
async fn share(app: &Router, conversation: i64) -> SharedConversation {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/ui/conversations/{conversation}/share.json"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

/// What a placeholder carries once its picture has been read in.
fn inlined(media_type: &str, bytes: &[u8]) -> String {
    format!(
        r#"data-src="data:{media_type};base64,{}""#,
        BASE64_STANDARD.encode(bytes)
    )
}

/// Every Diff block of every Set the share carries, as one page of markup.
fn sheets(shared: &SharedConversation) -> String {
    shared
        .sets
        .iter()
        .flat_map(|set| &set.diff)
        .map(|block| block.diff.html.as_str())
        .collect()
}

/// A Set's pictures travel inside the file: a tracked edit's before out of the
/// object database and its after off the Worktree, a file git has never heard
/// of, and a read-write companion's, out of its own Worktree.
#[tokio::test]
async fn a_share_carries_the_pictures_of_its_sets_inside_it() {
    let (dir, pool, app) = fresh_app().await;
    let (conversation, worktree, companions) = grilling(&pool, dir.path()).await;

    std::fs::write(worktree.join("logo.svg"), REDRAWN).unwrap();
    std::fs::write(worktree.join("badge.svg"), BADGE).unwrap();
    std::fs::write(companions[0].join("logo.svg"), REDRAWN).unwrap();
    composed(&pool, conversation).await;

    let html = sheets(&share(&app, conversation).await);

    assert_eq!(
        html.matches(r#"class="diffPicture""#).count(),
        5,
        "a pair in each repository and the one new file:\n{html}"
    );
    assert_eq!(
        html.matches("data-src=").count(),
        5,
        "every one of them read in:\n{html}"
    );
    assert_eq!(html.matches(&inlined("image/svg+xml", LOGO)).count(), 2);
    assert_eq!(html.matches(&inlined("image/svg+xml", REDRAWN)).count(), 2);
    assert_eq!(html.matches(&inlined("image/svg+xml", BADGE)).count(), 1);
}

/// A commit's pictures travel inside the file too, read out of the repository
/// it landed in.
#[tokio::test]
async fn a_share_carries_the_pictures_of_its_commits_inside_it() {
    let (dir, pool, app) = fresh_app().await;
    let (conversation, worktree, _) = grilling(&pool, dir.path()).await;

    std::fs::write(worktree.join("logo.svg"), REDRAWN).unwrap();
    std::fs::write(worktree.join("icon.png"), b"\x89PNG\r\n\x1a\n").unwrap();
    git(&worktree, &["add", "-A"]);
    git(&worktree, &["commit", "-m", "redraw the logo"]);
    record(&pool, conversation, &git(&worktree, &["rev-parse", "HEAD"])).await;

    let shared = share(&app, conversation).await;
    let [commit] = &shared.commits[..] else {
        panic!("one commit on the Timeline");
    };
    let html = &commit.pane.diff.as_ref().expect("the commit's diff").html;

    assert_eq!(html.matches(r#"class="diffPicture""#).count(), 3, "{html}");
    assert!(html.contains(&inlined("image/svg+xml", LOGO)), "{html}");
    assert!(html.contains(&inlined("image/svg+xml", REDRAWN)), "{html}");
    assert!(
        html.contains(&inlined("image/png", b"\x89PNG\r\n\x1a\n")),
        "{html}"
    );
}

/// A picture that cannot be read as the share is made — the Worktree moved on
/// from the file its Diff named — is left a placeholder, and the SVG's hunks
/// still travel under it, for the viewer to fall back to as it does from one it
/// cannot fetch.
#[tokio::test]
async fn a_picture_gone_at_share_time_is_left_for_the_viewer_to_fall_back_from() {
    let (dir, pool, app) = fresh_app().await;
    let (conversation, worktree, _) = grilling(&pool, dir.path()).await;

    std::fs::write(worktree.join("badge.svg"), BADGE).unwrap();
    composed(&pool, conversation).await;
    std::fs::write(worktree.join("badge.svg"), REDRAWN).unwrap();

    let html = sheets(&share(&app, conversation).await);

    assert_eq!(html.matches(r#"class="diffPicture""#).count(), 1, "{html}");
    assert!(!html.contains("data-src="), "{html}");
    assert!(html.contains("diffPictureCode"), "{html}");
}

/// Record a commit made in the Conversation's own repository on its Timeline,
/// the way the sweep does.
async fn record(pool: &SqlitePool, conversation: i64, sha: &str) {
    let repo = store::load_conversation(pool, conversation)
        .await
        .unwrap()
        .unwrap()
        .repo
        .id;

    store::record_commit(
        pool,
        conversation,
        repo,
        &store::Commit {
            sha: sha.to_owned(),
            subject: "redraw the logo".to_owned(),
            files: 2,
            insertions: 1,
            deletions: 1,
            summary: None,
            repo: None,
            merge: false,
        },
    )
    .await
    .unwrap()
    .unwrap();
}
