//! Conversations over the viewer's namespace: starting one against a registered
//! Repo, and everything the human settles about it before any of it runs.
//!
//! Asked of the *server*, through the endpoints, because that is where the
//! decisions are: whether a name is one git would take for a branch, and whether
//! anything in the repository answers to what was typed as a base commit. A form
//! that checked either would be a courtesy.
//!
//! Nothing here creates a branch or a worktree, and nothing here should: a
//! Conversation is a record of work to be started later, and the stage that
//! starts it is the next one.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use serde::de::DeserializeOwned;
use tower::ServiceExt;
use verkstead_render::{
    BaseRecorded, BranchRenamed, BriefSaved, ConversationEntry, ConversationView, Lifecycle,
    Registered, Started, TimelineEvent,
};
use verkstead_server::{WatchedPaths, open_database, router_watching};

/// A router watching `watched`, plus the directory holding its database alive.
async fn app_watching(watched: &Path) -> (tempfile::TempDir, Router) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();
    let watched = WatchedPaths::resolve(&[watched.to_owned()]).unwrap();

    (dir, router_watching(pool, watched))
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

/// A watched directory holding one registered repository, and the app over it.
async fn workbench() -> (tempfile::TempDir, tempfile::TempDir, Router, PathBuf, i64) {
    let watched = tempfile::tempdir().unwrap();
    let (dir, app) = app_watching(watched.path()).await;
    let repo = repository(watched.path().join("verkstead"));

    let registered: Registered = post(&app, "/api/ui/repos", &serde_json::json!({ "path": repo })).await;
    assert_eq!(registered, Registered::Added);

    let repo_id = listed_repos(&app).await;

    (watched, dir, app, repo, repo_id)
}

/// The id of the one registered Repo.
async fn listed_repos(app: &Router) -> i64 {
    let repos: Vec<verkstead_render::RepoEntry> = get(app, "/api/ui/repos").await;
    assert_eq!(repos.len(), 1);
    repos[0].id
}

async fn start(app: &Router, repo_id: i64) -> Started {
    post(app, "/api/ui/conversations", &serde_json::json!({ "repo_id": repo_id })).await
}

/// Start one and take the id, for the tests that are about what happens next.
async fn started(app: &Router, repo_id: i64) -> i64 {
    match start(app, repo_id).await {
        Started::Started { id } => id,
        other => panic!("expected the Conversation to start, got {other:?}"),
    }
}

async fn sidebar(app: &Router) -> Vec<ConversationEntry> {
    get(app, "/api/ui/conversations").await
}

async fn opened(app: &Router, id: i64) -> ConversationView {
    get(app, &format!("/api/ui/conversations/{id}")).await
}

/// The Brief on a Conversation's Timeline, which is its only Event yet.
fn brief(view: &ConversationView) -> &verkstead_render::BriefEvent {
    assert_eq!(view.timeline.len(), 1, "the Brief should be the only Event");

    let TimelineEvent::Brief(brief) = &view.timeline[0];
    brief
}

async fn write_brief(app: &Router, id: i64, markdown: &str) -> BriefSaved {
    post(
        app,
        &format!("/api/ui/conversations/{id}/brief"),
        &serde_json::json!({ "markdown": markdown }),
    )
    .await
}

async fn rename(app: &Router, id: i64, branch: &str) -> BranchRenamed {
    post(
        app,
        &format!("/api/ui/conversations/{id}/branch"),
        &serde_json::json!({ "branch": branch }),
    )
    .await
}

async fn base(app: &Router, id: i64, commit: Option<&str>) -> BaseRecorded {
    post(
        app,
        &format!("/api/ui/conversations/{id}/base"),
        &serde_json::json!({ "commit": commit }),
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
async fn a_conversation_starts_against_a_registered_repo_and_appears_in_the_sidebar() {
    let (_watched, _dir, app, _repo, repo_id) = workbench().await;

    let id = started(&app, repo_id).await;

    let sidebar = sidebar(&app).await;
    assert_eq!(sidebar.len(), 1);
    assert_eq!(sidebar[0].id, id);
    assert_eq!(sidebar[0].repo, "verkstead");
    assert_eq!(sidebar[0].state, Lifecycle::Draft);

    // The branch is the row's name, so there has to be one from the start.
    assert!(!sidebar[0].branch.is_empty());
}

/// The prefill is a name, not a placeholder: the human may well leave it, and it
/// has to be one git will take when the branch is finally created.
#[tokio::test]
async fn the_prefilled_branch_name_is_two_words_git_would_take() {
    let (_watched, _dir, app, repo, repo_id) = workbench().await;

    let id = started(&app, repo_id).await;
    let branch = opened(&app, id).await.branch;

    assert!(branch.contains('-'), "expected two words, got {branch:?}");

    let taken = Command::new("git")
        .args(["check-ref-format", &format!("refs/heads/{branch}")])
        .current_dir(&repo)
        .status()
        .unwrap()
        .success();

    assert!(taken, "git would not take {branch:?} as a branch name");
}

#[tokio::test]
async fn a_conversation_cannot_be_started_against_a_repo_that_is_not_registered() {
    let (_watched, _dir, app, _repo, _repo_id) = workbench().await;

    assert_eq!(start(&app, 404).await, Started::NoSuchRepo);
    assert!(sidebar(&app).await.is_empty());
}

/// Everything a Conversation is attached to, in the one payload the middle and
/// the right-hand panes are drawn from.
#[tokio::test]
async fn opening_a_conversation_brings_its_repo_and_its_timeline_with_it() {
    let (_watched, _dir, app, repo, repo_id) = workbench().await;
    let id = started(&app, repo_id).await;

    let view = opened(&app, id).await;

    assert_eq!(view.id, id);
    assert_eq!(view.repo.name, "verkstead");
    assert_eq!(view.repo.path, repo.canonicalize().unwrap().to_str().unwrap());
    assert_eq!(view.repo.default_branch, "main");
    assert_eq!(view.state, Lifecycle::Draft);

    // No override, which is the default-branch rule rather than a missing value.
    assert_eq!(view.base_commit, None);

    // The Brief is the first Event from the start, empty though it is: it is
    // what the human writes into.
    assert_eq!(brief(&view).markdown, "");
}

/// The Brief crosses the wire rendered, like every other piece of markdown here
/// — and beside its own source, because it is the one the human edits.
#[tokio::test]
async fn a_written_brief_comes_back_as_markdown_and_as_html() {
    let (_watched, _dir, app, _repo, repo_id) = workbench().await;
    let id = started(&app, repo_id).await;

    assert_eq!(
        write_brief(&app, id, "# Rate limiting\n\n`POST /v1/messages` has none.\n").await,
        BriefSaved::Saved
    );

    let view = opened(&app, id).await;
    let brief = brief(&view);

    assert_eq!(
        brief.markdown,
        "# Rate limiting\n\n`POST /v1/messages` has none.\n"
    );
    assert!(brief.html.contains("<h1"), "expected a heading: {}", brief.html);
    assert!(
        brief.html.contains("<code>POST /v1/messages</code>"),
        "expected the code span: {}",
        brief.html
    );
}

/// The Brief is one document, not a growing pile of them: while a Conversation is
/// drafting there is one Brief and editing it rewrites it.
#[tokio::test]
async fn editing_the_brief_rewrites_the_one_event_rather_than_adding_another() {
    let (_watched, _dir, app, _repo, repo_id) = workbench().await;
    let id = started(&app, repo_id).await;

    write_brief(&app, id, "# First thought\n").await;
    write_brief(&app, id, "# Second thought\n").await;

    let view = opened(&app, id).await;
    assert_eq!(view.timeline.len(), 1);
    assert_eq!(brief(&view).markdown, "# Second thought\n");
}

/// What a human writes into a Brief is markdown, and markdown that came from a
/// browser is not automatically safe to put back in one.
#[tokio::test]
async fn a_brief_is_sanitised_on_the_way_out() {
    let (_watched, _dir, app, _repo, repo_id) = workbench().await;
    let id = started(&app, repo_id).await;

    write_brief(&app, id, "<script>alert('pwned')</script>\n").await;

    let view = opened(&app, id).await;
    assert!(
        !brief(&view).html.contains("alert('pwned')"),
        "the script should have been sanitised away: {}",
        brief(&view).html
    );
}

#[tokio::test]
async fn a_drafting_conversations_branch_is_the_humans_to_name() {
    let (_watched, _dir, app, _repo, repo_id) = workbench().await;
    let id = started(&app, repo_id).await;

    assert_eq!(rename(&app, id, "rate-limiting").await, BranchRenamed::Renamed);

    assert_eq!(opened(&app, id).await.branch, "rate-limiting");
    assert_eq!(sidebar(&app).await[0].branch, "rate-limiting");
}

/// The name is git's to judge, and a name it would not take is refused now
/// rather than when the branch is finally created.
#[tokio::test]
async fn a_name_git_would_not_take_for_a_branch_is_refused() {
    let (_watched, _dir, app, _repo, repo_id) = workbench().await;
    let id = started(&app, repo_id).await;
    let prefilled = opened(&app, id).await.branch;

    for refused in ["", "   ", "has space", "two..dots", "with~tilde", "-dash"] {
        assert_eq!(
            rename(&app, id, refused).await,
            BranchRenamed::NotABranchName,
            "{refused:?} should have been refused"
        );
    }

    assert_eq!(
        opened(&app, id).await.branch,
        prefilled,
        "nothing refused should have changed the name"
    );
}

#[tokio::test]
async fn the_name_is_taken_without_the_whitespace_around_it() {
    let (_watched, _dir, app, _repo, repo_id) = workbench().await;
    let id = started(&app, repo_id).await;

    assert_eq!(
        rename(&app, id, "  rate-limiting  ").await,
        BranchRenamed::Renamed
    );
    assert_eq!(opened(&app, id).await.branch, "rate-limiting");
}

/// The override is a commit, so what is recorded is the commit the repository
/// resolved — a tag or a branch would move, and pinning is the whole point of
/// overriding the rule.
#[tokio::test]
async fn a_base_commit_override_is_recorded_as_the_commit_the_repo_resolved() {
    let (_watched, _dir, app, repo, repo_id) = workbench().await;
    let id = started(&app, repo_id).await;

    let head = git(&repo, &["rev-parse", "HEAD"]).trim().to_owned();
    let short = &head[..8];

    assert_eq!(base(&app, id, Some(short)).await, BaseRecorded::Recorded);
    assert_eq!(opened(&app, id).await.base_commit.as_deref(), Some(&head[..]));
}

#[tokio::test]
async fn a_branch_name_resolves_to_the_commit_it_points_at() {
    let (_watched, _dir, app, repo, repo_id) = workbench().await;
    let id = started(&app, repo_id).await;

    let head = git(&repo, &["rev-parse", "HEAD"]).trim().to_owned();

    assert_eq!(base(&app, id, Some("main")).await, BaseRecorded::Recorded);
    assert_eq!(opened(&app, id).await.base_commit.as_deref(), Some(&head[..]));
}

/// Refused now rather than at grill start, where it would be a failure with
/// nobody watching.
#[tokio::test]
async fn a_base_commit_the_repository_has_never_heard_of_is_refused() {
    let (_watched, _dir, app, _repo, repo_id) = workbench().await;
    let id = started(&app, repo_id).await;

    assert_eq!(
        base(&app, id, Some("deadbeefdeadbeef")).await,
        BaseRecorded::NoSuchCommit
    );
    assert_eq!(opened(&app, id).await.base_commit, None);
}

/// Emptying the field is taking the override away, not naming a commit called
/// nothing — and what it goes back to is the rule.
#[tokio::test]
async fn clearing_the_base_commit_puts_the_conversation_back_on_the_rule() {
    let (_watched, _dir, app, _repo, repo_id) = workbench().await;
    let id = started(&app, repo_id).await;

    base(&app, id, Some("main")).await;

    for cleared in [None, Some("")] {
        assert_eq!(base(&app, id, cleared).await, BaseRecorded::Recorded);
        assert_eq!(opened(&app, id).await.base_commit, None);
    }
}

#[tokio::test]
async fn a_conversation_that_is_not_there_says_so_however_it_is_asked_about() {
    let (_watched, _dir, app, _repo, _repo_id) = workbench().await;

    let (status, _) = fetch(
        &app,
        Request::builder()
            .uri("/api/ui/conversations/404")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    assert_eq!(
        write_brief(&app, 404, "# Nothing\n").await,
        BriefSaved::NoSuchConversation
    );
    assert_eq!(
        rename(&app, 404, "nothing").await,
        BranchRenamed::NoSuchConversation
    );
    assert_eq!(
        base(&app, 404, None).await,
        BaseRecorded::NoSuchConversation
    );
}

/// An id out of a URL the human may have typed, which is not always a number.
#[tokio::test]
async fn an_id_that_is_not_a_number_names_no_conversation() {
    let (_watched, _dir, app, _repo, _repo_id) = workbench().await;

    let (status, _) = fetch(
        &app,
        Request::builder()
            .uri("/api/ui/conversations/nonsense")
            .body(Body::empty())
            .unwrap(),
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn nothing_started_means_an_empty_sidebar() {
    let (_watched, _dir, app, _repo, _repo_id) = workbench().await;

    assert!(sidebar(&app).await.is_empty());
}
