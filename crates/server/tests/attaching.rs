//! The files the human puts on a Conversation and on the Answers to its
//! Question Sets: the upload, the removal, and every way each of them is
//! refused.
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
    AnswerAttached, AnswerAttachmentRemoved, Attached, AttachmentOrigin, AttachmentRemoved,
    AttachmentView, ConversationView, Registered, SetReading, Started,
};
use verkstead_server::{attachments::MAX_BYTES, open_database, router_keeping, store};

/// The device every Conversation started here is ranked by, named the way a
/// cluster names one (ADR-0020, *Ranks*).
const THIS_DEVICE: &str = "aa00bb11cc22dd33ee44ff5566778899";

/// A directory holding one registered repository, a Conversation drafting on
/// it, and the app over both.
///
/// Hands back that directory, the Data Directory, the app, the pool and the
/// Conversation — the last two because half of what these tests assert is on
/// disk under the Data Directory and the other half is a state only the store
/// can put a Conversation into.
async fn drafting() -> (
    tempfile::TempDir,
    tempfile::TempDir,
    Router,
    SqlitePool,
    i64,
) {
    let elsewhere = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();

    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();
    let app = router_keeping(pool.clone(), dir.path().to_owned());

    let repo = repository(elsewhere.path().join("verkstead"));
    let registered: Registered =
        post(&app, "/api/ui/repos", &serde_json::json!({ "path": repo })).await;
    assert!(matches!(registered, Registered::Added(_)));

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

    (elsewhere, dir, app, pool, id)
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
    let (_elsewhere, dir, app, _pool, id) = drafting().await;

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
    let (_elsewhere, _dir, app, _pool, id) = drafting().await;

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
    let (_elsewhere, dir, app, _pool, id) = drafting().await;

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
    let (_elsewhere, dir, app, _pool, id) = drafting().await;

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
    let (_elsewhere, _dir, app, _pool, id) = drafting().await;

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
    let (_elsewhere, dir, app, pool, mine) = drafting().await;

    let repos: Vec<verkstead_render::RepoEntry> = get(&app, "/api/ui/repos").await;
    let theirs = store::start_conversation(&pool, repos[0].id, "elsewhere", THIS_DEVICE)
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
    let (_elsewhere, dir, app, _pool, id) = drafting().await;

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
    let (_elsewhere, dir, app, pool, id) = drafting().await;

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
    let (_elsewhere, dir, app, pool, id) = drafting().await;

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
    let (_elsewhere, dir, app, _pool, id) = drafting().await;

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
    let (_elsewhere, _dir, app, _pool, _id) = drafting().await;

    assert_eq!(
        attach(&app, 404, "notes.md", b"nobody").await,
        Attached::NoSuchConversation,
    );
    assert_eq!(
        detach(&app, 404, 1).await,
        AttachmentRemoved::NoSuchConversation,
    );
}

/// And a directory under the attachments root that no Conversation names is
/// gone after a server start, while one a live Conversation names is not.
///
/// The backstop under the Cleanup's delete, which is the one thing that removes
/// a directory: a delete that could not have it logged the path and deleted the
/// rows anyway, and a database restored from before a file was attached names
/// none of what the machine still has. Nothing else is ever going to look at
/// either.
///
/// A second server over the same store and the same Data Directory, which is
/// what a restart is — the sweep runs as the router is built.
#[tokio::test]
async fn a_stray_directory_is_swept_at_a_server_start_and_a_live_ones_is_not() {
    let (_elsewhere, dir, app, pool, id) = drafting().await;

    kept(attach(&app, id, "notes.md", b"the human's own").await);

    // What a delete that could not finish leaves: a directory named for a
    // Conversation the record no longer has.
    let stray = dir.path().join("attachments").join("4242");
    std::fs::create_dir_all(&stray).unwrap();
    std::fs::write(stray.join("forgotten.md"), "nobody's\n").unwrap();

    let restarted = router_keeping(pool.clone(), dir.path().to_owned());

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);

    while stray.exists() {
        assert!(
            std::time::Instant::now() < deadline,
            "the sweep never took {}",
            stray.display(),
        );

        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }

    assert_eq!(
        on_disk(&dir, id),
        vec!["notes.md".to_owned()],
        "and the one a Conversation in the record names is where it was",
    );

    // Read through the second server rather than the first, so that what says
    // the file survived is the server that swept beside it.
    assert_eq!(
        attached(&restarted, id)
            .await
            .into_iter()
            .map(|file| file.name)
            .collect::<Vec<_>>(),
        vec!["notes.md".to_owned()],
    );
}

/// The Set these tests put files on: one Question, one Sub-question and a
/// Heading over it, so that every way a label can be wrong has something real
/// to be wrong about.
const ASKED: &str = "
title: Where the counter lives
questions:
  - label: Q1
    text: Where should the count be kept?
    options:
      - n: 1
        text: In the process
      - n: 2
        text: In Redis
  - label: Q2
    text: And what about the window?
    subquestions:
      - letter: a
        text: How long is it?
";

/// One Question Set on that Conversation's Timeline, asked the way a session
/// asks one.
async fn asked(pool: &SqlitePool, conversation: i64) -> i64 {
    let set = verkstead_schema::QuestionSet::from_yaml(ASKED).expect("the fixture Set parses");

    store::ask(pool, conversation, &set, store::Ask::Blocking)
        .await
        .unwrap()
        .expect("the Conversation is there to ask from")
        .id
}

/// Put a file on one of that Set's Answers, the way the sheet does: the bytes as
/// the body, and the label and the name in the path.
async fn put_on(app: &Router, set: i64, label: &str, name: &str, body: &[u8]) -> AnswerAttached {
    let (status, said) = fetch(
        app,
        Request::builder()
            .method("POST")
            .uri(format!(
                "/api/ui/sets/{set}/answers/{}/attachments/{}",
                urlencoding(label),
                urlencoding(name),
            ))
            .header(header::CONTENT_TYPE, "application/octet-stream")
            .body(Body::from(body.to_vec()))
            .unwrap(),
    )
    .await;

    assert_eq!(
        status,
        StatusCode::OK,
        "putting {name:?} on {label} failed: {said}"
    );
    read(&said)
}

async fn take_off(app: &Router, set: i64, attachment: i64) -> AnswerAttachmentRemoved {
    post(
        app,
        &format!("/api/ui/sets/{set}/attachments/{attachment}/remove"),
        &serde_json::json!({}),
    )
    .await
}

/// The record the upload made, or the refusal said plainly.
fn on_the_answer(attached: AnswerAttached) -> AttachmentView {
    match attached {
        AnswerAttached::Attached { attachment } => attachment,
        other => panic!("expected the file to be attached, got {other:?}"),
    }
}

/// Every file the Set says it is holding, which is what the sheet and the record
/// after it are drawn from.
async fn on_the_set(app: &Router, set: i64) -> Vec<AttachmentView> {
    let reading: SetReading = get(app, &format!("/api/ui/sets/{set}")).await;

    match reading {
        SetReading::Set(view) => view.attachments,
        SetReading::Unreadable(unreadable) => {
            panic!("this build wrote that Set and cannot read it: {unreadable:?}")
        }
    }
}

/// Answer the Set, which is one of the three things that fixes its files.
async fn answer(app: &Router, set: i64) {
    let submitted: serde_json::Value = post(
        app,
        &format!("/api/ui/sets/{set}/response"),
        &serde_json::json!({
            "answers": [
                { "label": "Q1", "selected": 2 },
                { "label": "Q2a", "free_text": "a minute" },
            ],
        }),
    )
    .await;

    assert_eq!(
        submitted,
        serde_json::json!("Accepted"),
        "the Response was refused"
    );
}

/// The whole of the path in one: the bytes land in the Conversation's own
/// directory beside the Brief's, the record says which Answer they are on, and
/// the Set says so on every read after it.
#[tokio::test]
async fn a_file_on_an_answer_lands_in_the_directory_and_on_the_set() {
    let (_elsewhere, dir, app, pool, id) = drafting().await;
    let set = asked(&pool, id).await;

    let attachment = on_the_answer(put_on(&app, set, "Q1", "counter.png", b"PNG bytes").await);

    assert_eq!(attachment.name, "counter.png");
    assert_eq!(attachment.bytes, 9);
    assert_eq!(attachment.origin, AttachmentOrigin::Answer);
    assert_eq!(attachment.label.as_deref(), Some("Q1"));

    assert_eq!(
        std::fs::read(directory(&dir, id).join("counter.png")).unwrap(),
        b"PNG bytes",
    );
    assert_eq!(on_the_set(&app, set).await, vec![attachment]);
}

/// A Sub-question is a Question to put a file on, and a second file of a name
/// the Brief already took counts up: one flat directory, and the Brief's file
/// is not replaced.
#[tokio::test]
async fn a_name_the_brief_took_counts_up_in_the_one_directory() {
    let (_elsewhere, dir, app, pool, id) = drafting().await;
    let set = asked(&pool, id).await;

    kept(attach(&app, id, "notes.md", b"the Brief's").await);
    let second = on_the_answer(put_on(&app, set, "Q2a", "notes.md", b"the Answer's").await);

    assert_eq!(second.name, "notes-2.md");
    assert_eq!(second.label.as_deref(), Some("Q2a"));

    assert_eq!(on_disk(&dir, id), ["notes-2.md", "notes.md"]);
    assert_eq!(
        std::fs::read(directory(&dir, id).join("notes.md")).unwrap(),
        b"the Brief's",
        "the Brief's file is still the Brief's file",
    );

    assert_eq!(
        attached(&app, id)
            .await
            .into_iter()
            .map(|file| file.name)
            .collect::<Vec<_>>(),
        vec!["notes.md".to_owned()],
        "and the row of pills under the Brief is the Brief's own",
    );
}

/// Removing one takes the row and the file together, and leaves the Brief's
/// alone.
#[tokio::test]
async fn removing_one_from_an_answer_takes_the_row_and_the_file() {
    let (_elsewhere, dir, app, pool, id) = drafting().await;
    let set = asked(&pool, id).await;

    kept(attach(&app, id, "brief.md", b"the Brief's").await);
    let put = on_the_answer(put_on(&app, set, "Q1", "answer.md", b"the Answer's").await);

    assert_eq!(
        take_off(&app, set, put.id).await,
        AnswerAttachmentRemoved::Removed
    );
    assert_eq!(on_the_set(&app, set).await, Vec::new());
    assert_eq!(on_disk(&dir, id), ["brief.md"]);
}

/// One Set's file is not another's to remove — not even another Set of the same
/// Conversation.
#[tokio::test]
async fn another_sets_attachment_is_not_this_ones_to_remove() {
    let (_elsewhere, dir, app, pool, id) = drafting().await;
    let mine = asked(&pool, id).await;
    let theirs = asked(&pool, id).await;

    let put = on_the_answer(put_on(&app, theirs, "Q1", "notes.md", b"theirs").await);

    assert_eq!(
        take_off(&app, mine, put.id).await,
        AnswerAttachmentRemoved::Removed,
        "there is no such file on this Set, which is what was asked for",
    );
    assert_eq!(
        on_the_set(&app, theirs).await.len(),
        1,
        "and theirs is intact"
    );
    assert_eq!(on_disk(&dir, id), ["notes.md"]);
}

/// A label the Set does not ask is refused, however it is not one — a Heading,
/// which asks nothing of its own, or a Question the Set never carried — and
/// nothing lands anywhere.
#[tokio::test]
async fn a_label_the_set_does_not_ask_is_refused() {
    let (_elsewhere, dir, app, pool, id) = drafting().await;
    let set = asked(&pool, id).await;

    for label in ["Q2", "Q9", "Q1a", ""] {
        assert_eq!(
            put_on(&app, set, label, "notes.md", b"nope").await,
            AnswerAttached::NoSuchLabel,
            "{label:?} is not a Question this Set asks",
        );
    }

    assert_eq!(on_the_set(&app, set).await, Vec::new());
    assert_eq!(on_disk(&dir, id), Vec::<String>::new());
}

/// And a Set this build cannot read asks nothing anybody here can name.
#[tokio::test]
async fn a_set_that_cannot_be_read_takes_no_files() {
    let (_elsewhere, dir, app, pool, id) = drafting().await;
    let set = asked(&pool, id).await;

    sqlx::query("UPDATE question_sets SET body = ? WHERE id = ?")
        .bind(r#"{"title":"from the future","questions":[],"telepathy":true}"#)
        .bind(set)
        .execute(&pool)
        .await
        .unwrap();

    assert_eq!(
        put_on(&app, set, "Q1", "notes.md", b"nope").await,
        AnswerAttached::NoSuchLabel,
    );
    assert_eq!(on_disk(&dir, id), Vec::<String>::new());
}

/// An answered Set is the record of what the human sent: both presses are
/// refused by name, and what was already on it stays where it is.
#[tokio::test]
async fn an_answered_set_takes_no_more_files_and_gives_none_up() {
    let (_elsewhere, dir, app, pool, id) = drafting().await;
    let set = asked(&pool, id).await;

    let put = on_the_answer(put_on(&app, set, "Q1", "counter.png", b"before").await);
    answer(&app, set).await;

    assert_eq!(
        put_on(&app, set, "Q1", "late.png", b"too late").await,
        AnswerAttached::Answered,
    );
    assert_eq!(
        take_off(&app, set, put.id).await,
        AnswerAttachmentRemoved::Answered,
    );

    assert_eq!(on_disk(&dir, id), ["counter.png"]);
    assert_eq!(
        on_the_set(&app, set).await.len(),
        1,
        "and the answered Set still carries what was put on it",
    );
}

/// A Set locked unanswered keeps its files and takes no more, which is the same
/// freeze said the other way.
#[tokio::test]
async fn a_locked_set_takes_no_more_files_and_gives_none_up() {
    let (_elsewhere, dir, app, pool, id) = drafting().await;
    let set = asked(&pool, id).await;

    let put = on_the_answer(put_on(&app, set, "Q1", "counter.png", b"before").await);

    let locked: serde_json::Value = post(
        &app,
        &format!("/api/ui/sets/{set}/lock"),
        &serde_json::json!({}),
    )
    .await;
    assert_eq!(
        locked,
        serde_json::json!("Closed"),
        "the Set was not locked"
    );

    assert_eq!(
        put_on(&app, set, "Q1", "late.png", b"too late").await,
        AnswerAttached::Locked,
    );
    assert_eq!(
        take_off(&app, set, put.id).await,
        AnswerAttachmentRemoved::Locked,
    );
    assert_eq!(on_disk(&dir, id), ["counter.png"]);
    assert_eq!(
        on_the_set(&app, set).await.len(),
        1,
        "and the locked Set still carries what was put on it, which is what the \
         record after it draws under the Question",
    );
}

/// And a Closed Conversation is work nothing is going to pick up: its Sets take
/// nothing, and give nothing up either.
#[tokio::test]
async fn a_set_of_a_closed_conversation_takes_no_files() {
    let (_elsewhere, dir, app, pool, id) = drafting().await;
    let set = asked(&pool, id).await;

    let put = on_the_answer(put_on(&app, set, "Q1", "counter.png", b"before").await);

    store::set_state(&pool, id, store::Lifecycle::Closed)
        .await
        .unwrap();

    assert_eq!(
        put_on(&app, set, "Q1", "late.png", b"too late").await,
        AnswerAttached::Closed,
    );
    assert_eq!(
        take_off(&app, set, put.id).await,
        AnswerAttachmentRemoved::Closed,
    );
    assert_eq!(on_disk(&dir, id), ["counter.png"]);
}

/// The cap and the name rule are the Brief's, said on the sheet.
#[tokio::test]
async fn a_file_over_the_cap_or_under_no_name_is_refused_on_an_answer() {
    let (_elsewhere, dir, app, pool, id) = drafting().await;
    let set = asked(&pool, id).await;

    assert_eq!(
        put_on(&app, set, "Q1", "huge.bin", &vec![0u8; MAX_BYTES + 1]).await,
        AnswerAttached::TooLarge,
    );

    for name in ["../escape.md", "sub/notes.md", ".hidden.md"] {
        assert_eq!(
            put_on(&app, set, "Q1", name, b"nope").await,
            AnswerAttached::NotAName,
            "{name:?} is not a plain base name",
        );
    }

    assert_eq!(on_the_set(&app, set).await, Vec::new());
    assert_eq!(on_disk(&dir, id), Vec::<String>::new());
    assert!(!dir.path().join("escape.md").exists());
}

/// A Set that is not there says so, both ways round.
#[tokio::test]
async fn a_file_on_no_set_says_so() {
    let (_elsewhere, _dir, app, _pool, _id) = drafting().await;

    assert_eq!(
        put_on(&app, 404, "Q1", "notes.md", b"nobody").await,
        AnswerAttached::NoSuchSet,
    );
    assert_eq!(
        take_off(&app, 404, 1).await,
        AnswerAttachmentRemoved::NoSuchSet,
    );
}

/// A file is an Answer: a Question with one on it and nothing typed or picked
/// comes back answered, and the server takes it.
///
/// The Response says nothing about the file — the sheet holds nothing about one
/// — so what the server counts is the rows on the Set. Which is the whole of
/// what is being asked here: the same entry, with a file and without one.
#[tokio::test]
async fn a_question_with_a_file_on_it_is_answered() {
    let (_elsewhere, _dir, app, pool, id) = drafting().await;

    let unattached = asked(&pool, id).await;

    assert!(
        matches!(
            submit(&app, unattached).await,
            serde_json::Value::Object(refused) if refused.contains_key("Rejected"),
        ),
        "an entry carrying nothing at all is a question left open without saying so",
    );

    let set = asked(&pool, id).await;
    on_the_answer(put_on(&app, set, "Q1", "counter.png", b"PNG bytes").await);

    assert_eq!(
        submit(&app, set).await,
        serde_json::json!("Accepted"),
        "the file put on Q1 is the Answer to it",
    );
}

/// A Response answering `Q2a` in words and leaving `Q1` carrying nothing at
/// all, which is an Answer to it only where a file was put on it.
async fn submit(app: &Router, set: i64) -> serde_json::Value {
    post(
        app,
        &format!("/api/ui/sets/{set}/response"),
        &serde_json::json!({
            "answers": [
                { "label": "Q1" },
                { "label": "Q2a", "free_text": "a minute" },
            ],
        }),
    )
    .await
}

/// A record written before an Answer could carry files opens, and what is in it
/// reads as the Brief's — which is what the two columns arriving empty means.
///
/// The columns are dropped and the database opened again, which is what a
/// Verkstead of before wrote and this one is handed. See the store's
/// `migrations`.
#[tokio::test]
async fn a_record_written_before_the_columns_opens_and_reads_as_the_briefs() {
    let (_elsewhere, dir, app, pool, id) = drafting().await;

    kept(attach(&app, id, "wireframe.png", b"the human's own").await);

    for column in ["set_id", "label"] {
        sqlx::query(&format!("ALTER TABLE attachments DROP COLUMN {column}"))
            .execute(&pool)
            .await
            .unwrap();
    }

    pool.close().await;

    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();
    let reopened = router_keeping(pool.clone(), dir.path().to_owned());

    let files = attached(&reopened, id).await;

    assert_eq!(
        files.len(),
        1,
        "the file attached before the columns is still there"
    );
    assert_eq!(files[0].name, "wireframe.png");
    assert_eq!(files[0].origin, AttachmentOrigin::Brief);
    assert_eq!(files[0].label, None);

    // And the columns are back, so an Answer of that Conversation takes files
    // like any other.
    let set = asked(&pool, id).await;
    let put = on_the_answer(put_on(&reopened, set, "Q1", "counter.png", b"after").await);

    assert_eq!(put.label.as_deref(), Some("Q1"));
    assert_eq!(on_the_set(&reopened, set).await, vec![put]);
}
