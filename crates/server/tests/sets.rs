//! Submitting a Question Set: what the store keeps, and what the API refuses.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use axum::response::Response;
use http_body_util::BodyExt;
use sqlx::SqlitePool;
use tower::ServiceExt;
use verkstead_schema::{ApiError, QuestionSet, SetCreated};
use verkstead_server::store;
use verkstead_server::{open_database, router};

const VALID_SET: &str = r#"
title: Storage layout for the pending list
preface: |
  We need to settle how Sets are stored before the UI lands.

  The candidates differ mainly in how much SQL the pending list needs.
project: verkstead
branch: api-core-and-cli
questions:
  - label: Q1
    text: How should a Question Set be stored?
    options:
      - n: 1
        text: One JSON body column
        recommended: true
      - n: 2
        text: Fully normalised tables
  - label: Q2
    text: What identifies a Set on the wire?
    subquestions:
      - letter: a
        text: Integer rowid or UUID?
        options:
          - n: 1
            text: Integer rowid
          - n: 2
            text: UUID
      - letter: b
        text: Should the id appear in URLs?
"#;

/// What a stored Set asked, insisting this build can still read it.
///
/// Every Set here was written by this build a moment ago, so a body it cannot
/// read is a broken test rather than the case
/// [`store::Asked::Unreadable`] is there for.
fn asked(stored: &store::StoredSet) -> &QuestionSet {
    stored
        .set
        .set()
        .expect("a Set this build just stored reads back")
}

/// A pool over a fresh database, plus the directory keeping it alive.
async fn fresh_pool() -> (tempfile::TempDir, SqlitePool) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();
    (dir, pool)
}

/// The Conversation these Sets are asked from, made once per database and found
/// again after that.
///
/// Every Set is asked from one — that is what the base URL a session is given
/// says — so a test posting one needs somewhere for it to land. What the
/// Conversation is does not matter to anything here; that there is one does.
async fn asking_from(pool: &SqlitePool) -> i64 {
    if let Some(row) = store::conversations(pool).await.unwrap().first() {
        return row.id;
    }

    let repo = store::register_repo(pool, Path::new("/srv/verkstead"), "verkstead", "main")
        .await
        .unwrap()
        .expect("nothing is registered at that path yet");

    store::start_conversation(pool, repo.id, "api-core-and-cli")
        .await
        .unwrap()
        .expect("the Repo was just registered")
}

/// A Conversation with a real Worktree on disk, and where that Worktree is.
///
/// What the Diff needs and [`asking_from`] has not got: the server reads the
/// Conversation's own checkout as a Set arrives, so a test about the Diff has to
/// give it one to read. A repository with a commit in it, a worktree git itself
/// made, and a Conversation grilling in it.
async fn asking_from_a_worktree(pool: &SqlitePool, dir: &Path) -> (i64, PathBuf) {
    let repo = repository(dir.join("askance"));
    let registered = store::register_repo(pool, &repo, "askance", "main")
        .await
        .unwrap()
        .expect("nothing is registered at that path yet");

    let conversation = store::start_conversation(pool, registered.id, "rate-limiting")
        .await
        .unwrap()
        .expect("the Repo was just registered");

    let worktree = dir.join("worktrees/askance-rate-limiting");
    let base = git(&repo, &["rev-parse", "HEAD"]).trim().to_owned();
    git(
        &repo,
        &[
            "worktree",
            "add",
            "-b",
            "rate-limiting",
            &worktree.to_string_lossy(),
            &base,
        ],
    );

    store::start_grilling(pool, conversation, &base, &worktree, &[])
        .await
        .unwrap();

    (conversation, worktree)
}

/// A git repository with one commit in it, at `path`.
fn repository(path: PathBuf) -> PathBuf {
    std::fs::create_dir_all(&path).unwrap();
    git(&path, &["init", "--initial-branch", "main"]);
    git(&path, &["config", "user.email", "tests@verkstead.invalid"]);
    git(&path, &["config", "user.name", "Verkstead Tests"]);
    git(&path, &["config", "commit.gpgsign", "false"]);
    std::fs::write(path.join("README.md"), "# a repository\n").unwrap();
    git(&path, &["add", "README.md"]);
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

async fn post_set(pool: &SqlitePool, yaml: &str) -> Response {
    let conversation = asking_from(pool).await;

    post_set_from(pool, conversation, yaml).await
}

async fn post_set_from(pool: &SqlitePool, conversation: i64, yaml: &str) -> Response {
    router(pool.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/conversations/{conversation}/api/v1/sets"))
                .header(header::CONTENT_TYPE, "application/yaml")
                .body(Body::from(yaml.to_owned()))
                .unwrap(),
        )
        .await
        .unwrap()
}

async fn body_text(response: Response) -> String {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    String::from_utf8(bytes.to_vec()).unwrap()
}

async fn stored_set_count(pool: &SqlitePool) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM question_sets")
        .fetch_one(pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn a_valid_set_is_stored_and_answered_with_its_id() {
    let (_dir, pool) = fresh_pool().await;

    let response = post_set(&pool, VALID_SET).await;

    assert_eq!(response.status(), StatusCode::CREATED);
    assert_eq!(
        response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok()),
        Some("application/yaml"),
    );

    let created: SetCreated = serde_saphyr::from_str(&body_text(response).await).unwrap();
    assert!(created.id > 0, "the server should stamp an id");
    assert!(
        created.created_at.starts_with("20"),
        "the server should stamp an RFC 3339 created_at, got {:?}",
        created.created_at
    );

    let stored = store::load_set(&pool, created.id)
        .await
        .unwrap()
        .expect("the Set should be in the store");
    assert_eq!(stored.id, created.id);
    assert_eq!(stored.created_at, created.created_at);
    assert_eq!(asked(&stored).title, "Storage layout for the pending list");
    assert_eq!(asked(&stored).questions.len(), 2);
    assert_eq!(asked(&stored).questions[1].subquestions.len(), 2);
    assert_eq!(asked(&stored).project.as_deref(), Some("verkstead"));
    assert_eq!(asked(&stored).branch.as_deref(), Some("api-core-and-cli"));
}

#[tokio::test]
async fn a_multi_line_preface_survives_the_store_byte_for_byte() {
    let (_dir, pool) = fresh_pool().await;
    let submitted = "First paragraph, with a trailing space. \n\
                     \n\
                     Second paragraph with `code` and a \"quote\".\n\
                     \x20   an indented line\n";

    let response = post_set(
        &pool,
        "
title: Round trip
preface: |
  First paragraph, with a trailing space.\x20

  Second paragraph with `code` and a \"quote\".
      an indented line
questions:
  - label: Q1
    text: Does it survive?
",
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    let created: SetCreated = serde_saphyr::from_str(&body_text(response).await).unwrap();
    let stored = store::load_set(&pool, created.id).await.unwrap().unwrap();

    assert_eq!(asked(&stored).preface.as_deref(), Some(submitted));
}

#[tokio::test]
async fn a_dirty_worktree_is_read_for_the_diff_as_the_set_arrives() {
    let (dir, pool) = fresh_pool().await;
    let (conversation, worktree) = asking_from_a_worktree(&pool, dir.path()).await;

    // A tracked file changed, and one git has never heard of.
    std::fs::write(
        worktree.join("README.md"),
        "# a repository\nand a second line\n",
    )
    .unwrap();
    std::fs::write(
        worktree.join("open-questions.md"),
        "a line only in the working tree\n",
    )
    .unwrap();

    let response = post_set_from(&pool, conversation, VALID_SET).await;
    assert_eq!(response.status(), StatusCode::CREATED);

    let created: SetCreated = serde_saphyr::from_str(&body_text(response).await).unwrap();
    let stored = store::load_set(&pool, created.id).await.unwrap().unwrap();

    let diff = asked(&stored)
        .diff
        .clone()
        .expect("a dirty Worktree carries a Diff");
    assert!(
        diff.contains("+++ b/README.md") && diff.contains("+and a second line"),
        "a tracked file's changes belong in the Diff, got:\n{diff}"
    );
    assert!(
        diff.contains("+++ b/open-questions.md")
            && diff.contains("+a line only in the working tree"),
        "an untracked file's contents belong in the Diff, got:\n{diff}"
    );
}

#[tokio::test]
async fn a_clean_worktree_carries_no_diff() {
    let (dir, pool) = fresh_pool().await;
    let (conversation, _worktree) = asking_from_a_worktree(&pool, dir.path()).await;

    let response = post_set_from(&pool, conversation, VALID_SET).await;
    assert_eq!(response.status(), StatusCode::CREATED);

    let created: SetCreated = serde_saphyr::from_str(&body_text(response).await).unwrap();
    let stored = store::load_set(&pool, created.id).await.unwrap().unwrap();

    assert_eq!(
        asked(&stored).diff,
        None,
        "there is nothing uncommitted to attach"
    );
}

#[tokio::test]
async fn a_claimed_diff_is_replaced_by_what_the_server_read() {
    let (dir, pool) = fresh_pool().await;
    let (conversation, worktree) = asking_from_a_worktree(&pool, dir.path()).await;

    std::fs::write(
        worktree.join("README.md"),
        "# a repository\nreally changed\n",
    )
    .unwrap();

    let response = post_set_from(
        &pool,
        conversation,
        "
title: Please review
diff: |
  diff --git a/invented.md b/invented.md
  @@ -1 +1,2 @@
   first
  +what the agent wishes were there
questions:
  - label: Q1
    text: Land it?
",
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    let created: SetCreated = serde_saphyr::from_str(&body_text(response).await).unwrap();
    let stored = store::load_set(&pool, created.id).await.unwrap().unwrap();

    let diff = asked(&stored)
        .diff
        .clone()
        .expect("the Worktree is dirty, so there is a Diff");
    assert!(
        diff.contains("+really changed"),
        "the Diff is the server's own read of the Worktree, got:\n{diff}"
    );
    assert!(
        !diff.contains("invented.md"),
        "what the Set claimed is overwritten rather than kept, got:\n{diff}"
    );
}

#[tokio::test]
async fn a_conversation_with_no_worktree_attaches_nothing_rather_than_refusing() {
    let (_dir, pool) = fresh_pool().await;

    // The Conversation every other test here asks from: started, never grilled,
    // so nothing has been checked out for it.
    let response = post_set(
        &pool,
        "
title: Please review
diff: |
  diff --git a/notes.md b/notes.md
  @@ -1 +1,2 @@
   first
  +second
questions:
  - label: Q1
    text: Land it?
",
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    let created: SetCreated = serde_saphyr::from_str(&body_text(response).await).unwrap();
    let stored = store::load_set(&pool, created.id).await.unwrap().unwrap();

    assert_eq!(
        asked(&stored).diff,
        None,
        "there is no Worktree to read, and the Set lands anyway"
    );
}

#[tokio::test]
async fn three_levels_of_nesting_are_refused_naming_the_sub_question() {
    let (_dir, pool) = fresh_pool().await;

    let response = post_set(
        &pool,
        "
title: Too deep
questions:
  - label: Q7
    text: Which storage layout?
    subquestions:
      - letter: a
        text: Rowid or UUID?
        subquestions:
          - letter: i
            text: And how wide?
",
    )
    .await;

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let error: ApiError = serde_saphyr::from_str(&body_text(response).await).unwrap();
    assert!(
        error
            .violations
            .iter()
            .any(|v| v.label.as_deref() == Some("Q7a")),
        "the refusal should name Q7a, got {error:?}"
    );
    assert_eq!(stored_set_count(&pool).await, 0);
}

#[tokio::test]
async fn two_recommended_options_are_refused_naming_the_question() {
    let (_dir, pool) = fresh_pool().await;

    let response = post_set(
        &pool,
        "
title: Two stars
questions:
  - label: Q3
    text: Which one?
    options:
      - n: 1
        text: This
        recommended: true
      - n: 2
        text: That
        recommended: true
",
    )
    .await;

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let error: ApiError = serde_saphyr::from_str(&body_text(response).await).unwrap();
    assert!(
        error
            .violations
            .iter()
            .any(|v| v.label.as_deref() == Some("Q3")),
        "the refusal should name Q3, got {error:?}"
    );
    assert_eq!(stored_set_count(&pool).await, 0);
}

#[tokio::test]
async fn an_empty_title_is_refused() {
    let (_dir, pool) = fresh_pool().await;

    let response = post_set(
        &pool,
        "
title: '  '
questions:
  - label: Q1
    text: Ship it?
",
    )
    .await;

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let error: ApiError = serde_saphyr::from_str(&body_text(response).await).unwrap();
    assert!(
        error.violations.iter().any(|v| v.message.contains("title")),
        "the refusal should mention the title, got {error:?}"
    );
    assert_eq!(stored_set_count(&pool).await, 0);
}

#[tokio::test]
async fn a_missing_title_is_refused_as_malformed() {
    let (_dir, pool) = fresh_pool().await;

    let response = post_set(
        &pool,
        "
questions:
  - label: Q1
    text: Ship it?
",
    )
    .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let error: ApiError = serde_saphyr::from_str(&body_text(response).await).unwrap();
    assert!(
        error.error.contains("title"),
        "the refusal should mention the missing title, got {error:?}"
    );
    assert_eq!(stored_set_count(&pool).await, 0);
}

#[tokio::test]
async fn a_body_that_is_not_yaml_is_refused_as_malformed() {
    let (_dir, pool) = fresh_pool().await;

    let response = post_set(&pool, "{{{ not yaml").await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(stored_set_count(&pool).await, 0);
}

#[tokio::test]
async fn sets_are_stamped_with_distinct_ids() {
    let (_dir, pool) = fresh_pool().await;

    let first: SetCreated =
        serde_saphyr::from_str(&body_text(post_set(&pool, VALID_SET).await).await).unwrap();
    let second: SetCreated =
        serde_saphyr::from_str(&body_text(post_set(&pool, VALID_SET).await).await).unwrap();

    assert_ne!(first.id, second.id);
    assert_eq!(stored_set_count(&pool).await, 2);
}

#[tokio::test]
async fn loading_an_unknown_set_finds_nothing() {
    let (_dir, pool) = fresh_pool().await;

    assert!(store::load_set(&pool, 404).await.unwrap().is_none());
}

#[tokio::test]
async fn the_store_keeps_the_pending_list_columns_alongside_the_body() {
    let (_dir, pool) = fresh_pool().await;
    let created: SetCreated =
        serde_saphyr::from_str(&body_text(post_set(&pool, VALID_SET).await).await).unwrap();

    let (title, project, branch): (String, Option<String>, Option<String>) =
        sqlx::query_as("SELECT title, project, branch FROM question_sets WHERE id = ?")
            .bind(created.id)
            .fetch_one(&pool)
            .await
            .unwrap();

    assert_eq!(title, "Storage layout for the pending list");
    assert_eq!(project.as_deref(), Some("verkstead"));
    assert_eq!(branch.as_deref(), Some("api-core-and-cli"));
}

#[tokio::test]
async fn the_schema_is_applied_to_an_existing_database() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("verkstead.db");

    let pool = open_database(&path).await.unwrap();
    let set = QuestionSet::from_yaml(VALID_SET).unwrap();
    let conversation = asking_from(&pool).await;
    let created = store::ask(&pool, conversation, &set, store::Ask::Blocking)
        .await
        .unwrap()
        .expect("the Conversation is there to ask from");
    pool.close().await;

    let pool = open_database(&path).await.unwrap();
    let stored = store::load_set(&pool, created.id).await.unwrap().unwrap();
    assert_eq!(asked(&stored), &set);
}
