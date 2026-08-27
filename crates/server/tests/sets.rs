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
    let (conversation, worktree, _) = asking_alongside(pool, dir, &[]).await;

    (conversation, worktree)
}

/// The same, with a companion repo checked out beside it per entry in
/// `companions`: what it is called and how far into it a session may reach.
///
/// The checkouts come back in the order they were asked for, so a test can dirty
/// the one it means. Each is the shape its mode gives it, exactly as a grill
/// start makes one: a branch of its own for a read-write companion, and a
/// detached checkout for a read-only one, which has nothing to commit.
async fn asking_alongside(
    pool: &SqlitePool,
    dir: &Path,
    companions: &[(&str, store::CompanionMode)],
) -> (i64, PathBuf, Vec<PathBuf>) {
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
        let checkout = dir.join(format!("worktrees/{name}-rate-limiting"));

        let cut = match mode {
            store::CompanionMode::ReadOnly => vec!["worktree", "add", "--detach"],
            store::CompanionMode::ReadWrite => vec!["worktree", "add", "-b", "rate-limiting"],
        };
        git(
            &path,
            &[&cut[..], &[&checkout.to_string_lossy(), &at]].concat(),
        );

        checkouts.push(store::CompanionWorktree {
            repo_id: companion.id,
            path: checkout,
            base_commit: at,
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

/// Leave something uncommitted in a checkout: a tracked file changed, and one
/// git has never heard of.
fn dirty(worktree: &Path, line: &str) {
    std::fs::write(
        worktree.join("README.md"),
        format!("# a repository\n{line}\n"),
    )
    .unwrap();
    std::fs::write(
        worktree.join("open-questions.md"),
        format!("{line}, and untracked\n"),
    )
    .unwrap();
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

    dirty(&worktree, "and a second line");

    let response = post_set_from(&pool, conversation, VALID_SET).await;
    assert_eq!(response.status(), StatusCode::CREATED);

    let created: SetCreated = serde_saphyr::from_str(&body_text(response).await).unwrap();
    let stored = store::load_set(&pool, created.id).await.unwrap().unwrap();

    let [block] = &asked(&stored).diffs[..] else {
        panic!(
            "one dirty Worktree is one block, got {:?}",
            asked(&stored).diffs
        );
    };
    assert_eq!(
        block.repo, "askance",
        "the block names the repository it was read out of"
    );
    assert!(block.own, "which here is the Conversation's own");
    assert!(
        block.diff.contains("+++ b/README.md") && block.diff.contains("+and a second line"),
        "a tracked file's changes belong in the Diff, got:\n{}",
        block.diff
    );
    assert!(
        block.diff.contains("+++ b/open-questions.md")
            && block.diff.contains("+and a second line, and untracked"),
        "an untracked file's contents belong in the Diff, got:\n{}",
        block.diff
    );
}

#[tokio::test]
async fn every_repository_a_session_may_write_in_is_read_in_turn() {
    let (dir, pool) = fresh_pool().await;
    let (conversation, worktree, companions) = asking_alongside(
        &pool,
        dir.path(),
        &[("verkstead", store::CompanionMode::ReadWrite)],
    )
    .await;

    dirty(&worktree, "the work's own repository");
    dirty(&companions[0], "the companion's");

    let response = post_set_from(&pool, conversation, VALID_SET).await;
    assert_eq!(response.status(), StatusCode::CREATED);

    let created: SetCreated = serde_saphyr::from_str(&body_text(response).await).unwrap();
    let stored = store::load_set(&pool, created.id).await.unwrap().unwrap();

    let diffs = &asked(&stored).diffs;
    assert_eq!(
        diffs.iter().map(|block| &*block.repo).collect::<Vec<_>>(),
        ["askance", "verkstead"],
        "the Conversation's own repository first, then its read-write companion"
    );
    assert_eq!(
        diffs.iter().map(|block| block.own).collect::<Vec<_>>(),
        [true, false],
        "and each says which of the two it is, which no name of a repository could"
    );
    assert!(
        diffs[0].diff.contains("+the work's own repository"),
        "each block is that repository's own read, got:\n{}",
        diffs[0].diff
    );
    assert!(
        diffs[1].diff.contains("+the companion's"),
        "each block is that repository's own read, got:\n{}",
        diffs[1].diff
    );
}

#[tokio::test]
async fn a_companion_with_nothing_uncommitted_contributes_no_block() {
    let (dir, pool) = fresh_pool().await;
    let (conversation, worktree, _companions) = asking_alongside(
        &pool,
        dir.path(),
        &[("verkstead", store::CompanionMode::ReadWrite)],
    )
    .await;

    dirty(&worktree, "only the work's own repository");

    let response = post_set_from(&pool, conversation, VALID_SET).await;
    assert_eq!(response.status(), StatusCode::CREATED);

    let created: SetCreated = serde_saphyr::from_str(&body_text(response).await).unwrap();
    let stored = store::load_set(&pool, created.id).await.unwrap().unwrap();

    assert_eq!(
        asked(&stored)
            .diffs
            .iter()
            .map(|block| &*block.repo)
            .collect::<Vec<_>>(),
        ["askance"],
        "a clean companion has nothing to show, and is not drawn saying so"
    );
}

/// And the other way about, which is what the block saying whether it is the
/// Conversation's own is for: a clean Worktree of the work's own leaves the
/// companion's block as the whole of the Diff, and it is still the companion's.
/// A block that could not say so would be drawn as the work's own repository's,
/// that being what an unlabeled block means.
#[tokio::test]
async fn a_lone_block_says_whether_it_is_the_conversations_own_repositorys() {
    let (dir, pool) = fresh_pool().await;
    let (conversation, _worktree, companions) = asking_alongside(
        &pool,
        dir.path(),
        &[("verkstead", store::CompanionMode::ReadWrite)],
    )
    .await;

    dirty(&companions[0], "only the companion's");

    let response = post_set_from(&pool, conversation, VALID_SET).await;
    assert_eq!(response.status(), StatusCode::CREATED);

    let created: SetCreated = serde_saphyr::from_str(&body_text(response).await).unwrap();
    let stored = store::load_set(&pool, created.id).await.unwrap().unwrap();

    let [block] = &asked(&stored).diffs[..] else {
        panic!(
            "one dirty Worktree is one block, got {:?}",
            asked(&stored).diffs
        );
    };
    assert_eq!(block.repo, "verkstead");
    assert!(
        !block.own,
        "the work's own repository had nothing uncommitted, so this is not its block"
    );
}

#[tokio::test]
async fn a_read_only_companion_is_not_read_at_all() {
    let (dir, pool) = fresh_pool().await;
    let (conversation, _worktree, companions) = asking_alongside(
        &pool,
        dir.path(),
        &[("verkstead", store::CompanionMode::ReadOnly)],
    )
    .await;

    // Something uncommitted in it all the same, which is not work a session put
    // there: its checkout is detached and its sandbox bind is read-only.
    dirty(&companions[0], "not this session's doing");

    let response = post_set_from(&pool, conversation, VALID_SET).await;
    assert_eq!(response.status(), StatusCode::CREATED);

    let created: SetCreated = serde_saphyr::from_str(&body_text(response).await).unwrap();
    let stored = store::load_set(&pool, created.id).await.unwrap().unwrap();

    assert!(
        asked(&stored).diffs.is_empty(),
        "a read-only companion is nothing to sweep, got {:?}",
        asked(&stored).diffs
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

    assert!(
        asked(&stored).diffs.is_empty(),
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

    let [block] = &asked(&stored).diffs[..] else {
        panic!(
            "the Worktree is dirty, so there is a Diff, got {:?}",
            asked(&stored).diffs
        );
    };
    assert!(
        block.diff.contains("+really changed"),
        "the Diff is the server's own read of the Worktree, got:\n{}",
        block.diff
    );
    assert!(
        !block.diff.contains("invented.md"),
        "what the Set claimed is overwritten rather than kept, got:\n{}",
        block.diff
    );
    assert_eq!(
        asked(&stored).diff,
        None,
        "the field a claim arrived in is left empty, being the record of the \
         Sets stored before the Diff was a list"
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

    assert!(
        asked(&stored).diff.is_none() && asked(&stored).diffs.is_empty(),
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
