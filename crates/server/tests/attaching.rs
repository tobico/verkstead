//! The files the human puts on a Conversation: the upload, the removal, and
//! every way each of them is refused.
//!
//! Asked of the *server*, through the endpoints, because both halves of an
//! attachment are the server's: the row in the record and the file in the
//! Conversation's own directory under the Data Directory. What these assert is
//! what was actually left on disk beside what came back on the wire — a record
//! naming a file nobody wrote is the one failure a test of either half alone
//! would miss.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use serde::de::DeserializeOwned;
use sqlx::SqlitePool;
use tower::ServiceExt;
use verkstead_render::{
    Attached, AttachmentOrigin, AttachmentRemoved, AttachmentView, ConversationView, Registered,
    Started,
};
use verkstead_server::{
    WatchedPaths, attachments::MAX_BYTES, open_database, router_watching, store,
};

/// A watched directory holding one registered repository, a Conversation
/// drafting on it, and the app over both.
///
/// Hands back the watched directory, the Data Directory, the app, the pool and
/// the Conversation — the last two because half of what these tests assert is
/// on disk under the Data Directory and the other half is a state only the
/// store can put a Conversation into.
async fn drafting() -> (
    tempfile::TempDir,
    tempfile::TempDir,
    Router,
    SqlitePool,
    i64,
) {
    let watched = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();

    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();
    let app = router_watching(
        pool.clone(),
        WatchedPaths::resolve(&[watched.path().to_owned()]).unwrap(),
        dir.path().to_owned(),
    );

    let repo = repository(watched.path().join("verkstead"));
    let registered: Registered =
        post(&app, "/api/ui/repos", &serde_json::json!({ "path": repo })).await;
    assert_eq!(registered, Registered::Added);

    let repos: Vec<verkstead_render::RepoEntry> = get(&app, "/api/ui/repos").await;
    let started: Started = post(
        &app,
        "/api/ui/conversations",
        &serde_json::json!({ "repo_id": repos[0].id }),
    )
    .await;
    let Started::Started { id } = started else {
        panic!("expected the Conversation to start, got {started:?}");
    };

    (watched, dir, app, pool, id)
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

/// Attach one file, the way the composer does: the bytes as the body and the
/// name in the path.
async fn attach(app: &Router, id: i64, name: &str, body: &[u8]) -> Attached {
    let (status, said) = fetch(
        app,
        Request::builder()
            .method("POST")
            .uri(format!(
                "/api/ui/conversations/{id}/attachments/{}",
                urlencoding(name),
            ))
            .header(header::CONTENT_TYPE, "application/octet-stream")
            .body(Body::from(body.to_vec()))
            .unwrap(),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "attaching {name:?} failed: {said}");
    read(&said)
}

/// What a browser would put in the path for a file called this.
///
/// Enough of an encoder for the names these tests send: everything that is not
/// a plain letter, digit, dot, dash or underscore goes over as its bytes. Which
/// is what makes the separator cases reach the handler at all — a `/` left as
/// itself would be a path that matched no route.
fn urlencoding(name: &str) -> String {
    name.bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'.' | b'-' | b'_' => {
                (byte as char).to_string()
            }
            other => format!("%{other:02X}"),
        })
        .collect()
}

async fn detach(app: &Router, id: i64, attachment: i64) -> AttachmentRemoved {
    post(
        app,
        &format!("/api/ui/conversations/{id}/attachments/{attachment}/remove"),
        &serde_json::json!({}),
    )
    .await
}

/// The record the upload made, or the refusal said plainly.
fn kept(attached: Attached) -> AttachmentView {
    match attached {
        Attached::Attached { attachment } => attachment,
        other => panic!("expected the file to be attached, got {other:?}"),
    }
}

/// Every file the Conversation says it is holding.
async fn attached(app: &Router, id: i64) -> Vec<AttachmentView> {
    let view: ConversationView = get(app, &format!("/api/ui/conversations/{id}")).await;
    view.attachments
}

/// The Conversation's own directory under the Data Directory.
fn directory(dir: &tempfile::TempDir, id: i64) -> PathBuf {
    dir.path().join("attachments").join(id.to_string())
}

/// Every name in it, sorted — or nothing at all where the directory was never
/// made, which is what *nothing landed on disk* looks like.
fn on_disk(dir: &tempfile::TempDir, id: i64) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(directory(dir, id)) else {
        return Vec::new();
    };

    let mut names: Vec<String> = entries
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
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
    (status, String::from_utf8_lossy(&bytes).into_owned())
}

fn read<T: DeserializeOwned>(body: &str) -> T {
    serde_json::from_str(body).unwrap_or_else(|err| panic!("reading {body:?}: {err}"))
}

/// The whole of the path in one: the bytes land in the Conversation's own
/// directory, the record says they are there, and the Conversation says so on
/// every read after it.
#[tokio::test]
async fn a_file_lands_in_the_conversations_directory_and_on_its_record() {
    let (_watched, dir, app, _pool, id) = drafting().await;

    let attachment = kept(attach(&app, id, "wireframe.png", b"PNG bytes").await);

    assert_eq!(attachment.name, "wireframe.png");
    assert_eq!(attachment.bytes, 9);
    assert_eq!(attachment.origin, AttachmentOrigin::Brief);

    assert_eq!(
        std::fs::read(directory(&dir, id).join("wireframe.png")).unwrap(),
        b"PNG bytes",
    );
    assert_eq!(attached(&app, id).await, vec![attachment]);
}

/// The record survives a fresh read, which is what a reload is.
#[tokio::test]
async fn several_files_come_back_in_the_order_they_were_attached() {
    let (_watched, _dir, app, _pool, id) = drafting().await;

    for name in ["zebra.csv", "apple.png"] {
        kept(attach(&app, id, name, b"...").await);
    }

    let names: Vec<String> = attached(&app, id)
        .await
        .into_iter()
        .map(|attachment| attachment.name)
        .collect();

    assert_eq!(names, ["zebra.csv", "apple.png"]);
}

/// A name already in the directory is not replaced: the newcomer counts up over
/// its own stem, and both files are records.
#[tokio::test]
async fn the_same_name_twice_is_two_files_and_the_second_is_renamed() {
    let (_watched, dir, app, _pool, id) = drafting().await;

    let first = kept(attach(&app, id, "notes.md", b"first").await);
    let second = kept(attach(&app, id, "notes.md", b"second").await);

    assert_eq!(first.name, "notes.md");
    assert_eq!(second.name, "notes-2.md");
    assert_ne!(first.id, second.id);

    assert_eq!(attached(&app, id).await.len(), 2);
    assert_eq!(on_disk(&dir, id), ["notes-2.md", "notes.md"]);
    assert_eq!(
        std::fs::read(directory(&dir, id).join("notes.md")).unwrap(),
        b"first",
        "the first file is still the first file",
    );
}

/// Removing a pill takes the row and the file together.
#[tokio::test]
async fn removing_one_takes_the_row_and_the_file() {
    let (_watched, dir, app, _pool, id) = drafting().await;

    let attachment = kept(attach(&app, id, "notes.md", b"first").await);

    assert_eq!(
        detach(&app, id, attachment.id).await,
        AttachmentRemoved::Removed,
    );
    assert_eq!(attached(&app, id).await, Vec::new());
    assert_eq!(on_disk(&dir, id), Vec::<String>::new());
}

/// And removing what has already gone is what the press asked for, rather than
/// a refusal — the companion rows' rule, said again.
#[tokio::test]
async fn removing_one_that_has_already_gone_is_the_state_that_was_asked_for() {
    let (_watched, _dir, app, _pool, id) = drafting().await;

    let attachment = kept(attach(&app, id, "notes.md", b"first").await);

    detach(&app, id, attachment.id).await;
    assert_eq!(
        detach(&app, id, attachment.id).await,
        AttachmentRemoved::Removed,
    );
}

/// One Conversation's file is not another's to remove.
#[tokio::test]
async fn another_conversations_attachment_is_not_this_ones_to_remove() {
    let (_watched, dir, app, pool, mine) = drafting().await;

    let repos: Vec<verkstead_render::RepoEntry> = get(&app, "/api/ui/repos").await;
    let theirs = store::start_conversation(&pool, repos[0].id, "elsewhere")
        .await
        .unwrap()
        .unwrap();

    let attachment = kept(attach(&app, theirs, "notes.md", b"theirs").await);

    assert_eq!(
        detach(&app, mine, attachment.id).await,
        AttachmentRemoved::Removed,
        "there is no such file on this Conversation, which is what was asked for",
    );
    assert_eq!(
        attached(&app, theirs).await.len(),
        1,
        "and theirs is intact"
    );
    assert_eq!(on_disk(&dir, theirs), ["notes.md"]);
}

/// A file over the cap is refused by name, so the composer has something to
/// say — and nothing lands on disk.
#[tokio::test]
async fn a_file_over_the_cap_is_refused() {
    let (_watched, dir, app, _pool, id) = drafting().await;

    let too_much = vec![0u8; MAX_BYTES + 1];

    assert_eq!(
        attach(&app, id, "huge.bin", &too_much).await,
        Attached::TooLarge
    );
    assert_eq!(attached(&app, id).await, Vec::new());
    assert_eq!(on_disk(&dir, id), Vec::<String>::new());
}

/// And once the Brief has frozen there is nothing to attach to: the files
/// freeze with it.
#[tokio::test]
async fn an_upload_to_a_frozen_brief_is_refused() {
    let (_watched, dir, app, pool, id) = drafting().await;

    store::set_state(&pool, id, store::Lifecycle::Implementing)
        .await
        .unwrap();

    assert_eq!(
        attach(&app, id, "notes.md", b"too late").await,
        Attached::NotDrafting,
    );
    assert_eq!(on_disk(&dir, id), Vec::<String>::new());
}

/// And a removal is refused by the same freeze, for the same reason: what
/// cannot be attached cannot be taken off either.
#[tokio::test]
async fn a_removal_on_a_frozen_brief_is_refused() {
    let (_watched, dir, app, pool, id) = drafting().await;

    let attachment = kept(attach(&app, id, "notes.md", b"first").await);

    store::set_state(&pool, id, store::Lifecycle::Implementing)
        .await
        .unwrap();

    assert_eq!(
        detach(&app, id, attachment.id).await,
        AttachmentRemoved::NotDrafting,
    );
    assert_eq!(on_disk(&dir, id), ["notes.md"]);
}

/// A name that is not a plain base name is refused, whichever way it is not
/// one — and nothing lands anywhere, which is the whole point of the check.
#[tokio::test]
async fn a_name_that_is_not_a_plain_base_name_is_refused() {
    let (_watched, dir, app, _pool, id) = drafting().await;

    for name in [
        "../escape.md",
        "sub/notes.md",
        "sub\\notes.md",
        "/etc/passwd",
        ".hidden.md",
        "..",
    ] {
        assert_eq!(
            attach(&app, id, name, b"nope").await,
            Attached::NotAName,
            "{name:?} is not a plain base name",
        );
    }

    assert_eq!(attached(&app, id).await, Vec::new());
    assert_eq!(on_disk(&dir, id), Vec::<String>::new());
    assert!(
        !dir.path().join("escape.md").exists(),
        "and nothing climbed out of the directory either",
    );
}

/// An upload to a Conversation that is not there says so.
#[tokio::test]
async fn an_upload_to_no_conversation_says_so() {
    let (_watched, _dir, app, _pool, _id) = drafting().await;

    assert_eq!(
        attach(&app, 404, "notes.md", b"nobody").await,
        Attached::NoSuchConversation,
    );
    assert_eq!(
        detach(&app, 404, 1).await,
        AttachmentRemoved::NoSuchConversation,
    );
}
