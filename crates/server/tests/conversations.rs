//! Conversations over the viewer's namespace: starting one against a registered
//! Repo, and everything the human settles about it before any of it runs.
//!
//! Asked of the *server*, through the endpoints, because that is where the
//! decisions are: whether a name is one git would take for a branch, and whether
//! anything in the repository answers to what was typed as a base commit. A form
//! that checked either would be a courtesy.
//!
//! Starting the grilling is where that stops being true: the branch and the
//! worktree are made against a real repository, in a real state directory, and
//! what these assert is what git was actually left holding.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use serde::de::DeserializeOwned;
use tower::ServiceExt;
use verkstead_render::{
    BaseRecorded, BranchRenamed, BriefSaved, ConversationAborted, ConversationEntry,
    ConversationView, DirectionChosen, GrillingStarted, Lifecycle, PinnedEvent, ProfileSaved,
    Registered, Started, TimelineEvent,
};
use verkstead_server::{WatchedPaths, open_database, router_watching};

/// A router watching `watched`, plus the directory holding its database and its
/// state directory alive.
///
/// The state directory is the database's own, which is where it falls for the
/// real server: `--state-dir` defaults beside `--database`.
async fn app_watching(watched: &Path) -> (tempfile::TempDir, Router) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();
    let watched = WatchedPaths::resolve(&[watched.to_owned()]).unwrap();
    let state_dir = dir.path().to_owned();

    (dir, router_watching(pool, watched, state_dir))
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

    let registered: Registered =
        post(&app, "/api/ui/repos", &serde_json::json!({ "path": repo })).await;
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
    post(
        app,
        "/api/ui/conversations",
        &serde_json::json!({ "repo_id": repo_id }),
    )
    .await
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

/// The Brief on a Conversation's Timeline.
///
/// Found rather than taken from the front: the Brief is the first Event, but the
/// moves that follow it are Events too.
fn brief(view: &ConversationView) -> &verkstead_render::BriefEvent {
    view.timeline
        .iter()
        .find_map(|event| match event {
            TimelineEvent::Brief(brief) => Some(brief),
            _ => None,
        })
        .expect("every Conversation has a Brief from the moment it exists")
}

/// The states a Conversation's Timeline says it has moved through, in order.
fn moves(view: &ConversationView) -> Vec<Lifecycle> {
    view.timeline
        .iter()
        .filter_map(|event| match event {
            TimelineEvent::Moved(moved) => Some(moved.state),
            _ => None,
        })
        .collect()
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
    assert_eq!(
        view.repo.path,
        repo.canonicalize().unwrap().to_str().unwrap()
    );
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
        write_brief(
            &app,
            id,
            "# Rate limiting\n\n`POST /v1/messages` has none.\n"
        )
        .await,
        BriefSaved::Saved
    );

    let view = opened(&app, id).await;
    let brief = brief(&view);

    assert_eq!(
        brief.markdown,
        "# Rate limiting\n\n`POST /v1/messages` has none.\n"
    );
    assert!(
        brief.html.contains("<h1"),
        "expected a heading: {}",
        brief.html
    );
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

    assert_eq!(
        rename(&app, id, "rate-limiting").await,
        BranchRenamed::Renamed
    );

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
    assert_eq!(
        opened(&app, id).await.base_commit.as_deref(),
        Some(&head[..])
    );
}

#[tokio::test]
async fn a_branch_name_resolves_to_the_commit_it_points_at() {
    let (_watched, _dir, app, repo, repo_id) = workbench().await;
    let id = started(&app, repo_id).await;

    let head = git(&repo, &["rev-parse", "HEAD"]).trim().to_owned();

    assert_eq!(base(&app, id, Some("main")).await, BaseRecorded::Recorded);
    assert_eq!(
        opened(&app, id).await.base_commit.as_deref(),
        Some(&head[..])
    );
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

/// A claude dir and config file pair inside `watched`, so a Profile saved from
/// it is one a session could actually be run under.
fn pair(watched: &Path, account: &str) -> (PathBuf, PathBuf) {
    let home = watched.join(account);
    let claude_dir = home.join(".claude");
    let config_file = home.join(".claude.json");

    std::fs::create_dir_all(&claude_dir).unwrap();
    std::fs::write(&config_file, "{}\n").unwrap();

    (claude_dir, config_file)
}

/// Save an Agent Profile and hand back its id.
async fn profile(app: &Router, watched: &Path, name: &str) -> i64 {
    let (claude_dir, config_file) = pair(watched, name);

    let saved: ProfileSaved = post(
        app,
        "/api/ui/profiles",
        &serde_json::json!({
            "name": name,
            "claude_dir": claude_dir,
            "config_file": config_file,
            "model": "claude-opus-5",
        }),
    )
    .await;
    assert_eq!(saved, ProfileSaved::Saved);

    let profiles: Vec<verkstead_render::ProfileEntry> = get(app, "/api/ui/profiles").await;
    profiles
        .into_iter()
        .find(|profile| profile.name == name)
        .expect("the Profile just saved should be on the list")
        .id
}

async fn choose(app: &Router, id: i64, role: &str, profile_id: i64) {
    let chosen: verkstead_render::ProfileChosen = post(
        app,
        &format!("/api/ui/conversations/{id}/{role}-profile"),
        &serde_json::json!({ "profile_id": profile_id }),
    )
    .await;
    assert_eq!(chosen, verkstead_render::ProfileChosen::Chosen);
}

async fn grill(app: &Router, id: i64) -> GrillingStarted {
    post(
        app,
        &format!("/api/ui/conversations/{id}/grill"),
        &serde_json::json!({}),
    )
    .await
}

async fn abort(app: &Router, id: i64) -> ConversationAborted {
    post(
        app,
        &format!("/api/ui/conversations/{id}/abort"),
        &serde_json::json!({}),
    )
    .await
}

/// Everything a Conversation needs before it will grill: both Profiles chosen
/// and a Brief written. Hands back the Conversation's id.
async fn ready(app: &Router, watched: &Path, repo_id: i64) -> i64 {
    let id = started(app, repo_id).await;

    let grilling = profile(app, watched, "fable").await;
    let implementation = profile(app, watched, "opus").await;
    choose(app, id, "grilling", grilling).await;
    choose(app, id, "implementation", implementation).await;

    assert_eq!(
        write_brief(app, id, "# Rate limiting\n\nThe API has none.\n").await,
        BriefSaved::Saved
    );

    id
}

/// What git in `repo` says its worktrees are, by path.
fn worktrees(repo: &Path) -> Vec<PathBuf> {
    git(repo, &["worktree", "list", "--porcelain"])
        .lines()
        .filter_map(|line| line.strip_prefix("worktree "))
        .map(PathBuf::from)
        .collect()
}

/// The whole of what pressing the button does: a branch off the base commit, a
/// worktree registered with it under the state directory, and a Conversation
/// that says it is grilling.
#[tokio::test]
async fn starting_a_grilling_makes_the_branch_and_the_worktree() {
    let (watched, dir, app, repo, repo_id) = workbench().await;
    let id = ready(&app, watched.path(), repo_id).await;

    assert_eq!(grill(&app, id).await, GrillingStarted::Started);

    let view = opened(&app, id).await;
    assert_eq!(view.state, Lifecycle::Grilling);
    assert_eq!(moves(&view), [Lifecycle::Grilling]);

    // The branch is in the Repo's own git directory, not in the worktree.
    assert!(
        git(
            &repo,
            &[
                "rev-parse",
                "--verify",
                &format!("refs/heads/{}", view.branch)
            ]
        )
        .trim()
        .len()
            == 40,
        "the branch should be in the repository git already had"
    );

    // Named for the Repo and the branch, under the state directory — which is
    // where the database is, and not inside any Watched Path.
    let worktree = view
        .worktree
        .expect("a grilling Conversation has a worktree");
    let path = PathBuf::from(&worktree.path);
    assert!(!worktree.missing);
    assert_eq!(path.parent(), Some(dir.path().join("worktrees").as_path()));
    assert_eq!(
        path.file_name().unwrap().to_string_lossy(),
        format!("verkstead-{}", view.branch)
    );

    // And git knows about it, which is what makes it a worktree rather than a
    // copy of some files.
    assert!(
        worktrees(&repo).contains(&path.canonicalize().unwrap()),
        "git should have the worktree registered: {:?}",
        worktrees(&repo)
    );

    // The files are actually there, checked out on the branch.
    assert!(path.join("README.md").is_file());
    assert_eq!(
        git(&path, &["symbolic-ref", "--short", "HEAD"]).trim(),
        view.branch
    );
}

/// The rule the workbench already states — the default branch's tip *at grill
/// start* — resolving for the first time.
#[tokio::test]
async fn starting_records_the_commit_the_work_branched_from() {
    let (watched, _dir, app, repo, repo_id) = workbench().await;
    let id = ready(&app, watched.path(), repo_id).await;

    assert_eq!(
        opened(&app, id).await.base_commit,
        None,
        "nothing was overridden, so there is only the rule"
    );

    let tip = git(&repo, &["rev-parse", "HEAD"]).trim().to_owned();

    assert_eq!(grill(&app, id).await, GrillingStarted::Started);

    assert_eq!(
        opened(&app, id).await.base_commit.as_deref(),
        Some(tip.as_str())
    );
}

/// An overridden commit is what the work branches from, and it is not the tip.
#[tokio::test]
async fn an_overridden_base_commit_is_what_the_branch_is_made_off() {
    let (watched, _dir, app, repo, repo_id) = workbench().await;
    let id = ready(&app, watched.path(), repo_id).await;

    let first = git(&repo, &["rev-parse", "HEAD"]).trim().to_owned();
    std::fs::write(repo.join("second.md"), "# more\n").unwrap();
    git(&repo, &["add", "second.md"]);
    git(&repo, &["commit", "-m", "second"]);

    assert_eq!(base(&app, id, Some(&first)).await, BaseRecorded::Recorded);
    assert_eq!(grill(&app, id).await, GrillingStarted::Started);

    let view = opened(&app, id).await;
    assert_eq!(view.base_commit.as_deref(), Some(first.as_str()));

    let worktree = PathBuf::from(view.worktree.unwrap().path);
    assert_eq!(git(&worktree, &["rev-parse", "HEAD"]).trim(), first);
    assert!(
        !worktree.join("second.md").exists(),
        "the worktree should hold the commit it branched from, not the tip"
    );
}

/// Each precondition refuses by its own name, because each of them is something
/// different for the human to go and do.
#[tokio::test]
async fn starting_is_refused_by_name_when_a_profile_is_unchosen() {
    let (watched, _dir, app, _repo, repo_id) = workbench().await;
    let id = started(&app, repo_id).await;
    write_brief(&app, id, "# Rate limiting\n").await;

    assert_eq!(grill(&app, id).await, GrillingStarted::NoGrillingProfile);

    choose(
        &app,
        id,
        "grilling",
        profile(&app, watched.path(), "fable").await,
    )
    .await;
    assert_eq!(
        grill(&app, id).await,
        GrillingStarted::NoImplementationProfile
    );

    choose(
        &app,
        id,
        "implementation",
        profile(&app, watched.path(), "opus").await,
    )
    .await;
    assert_eq!(grill(&app, id).await, GrillingStarted::Started);
}

/// A Profile whose pair has gone is no account to run a session under, and the
/// pane says so — so pressing the button anyway has to say the same thing.
#[tokio::test]
async fn starting_is_refused_when_a_chosen_profiles_pair_has_gone() {
    let (watched, _dir, app, _repo, repo_id) = workbench().await;
    let id = ready(&app, watched.path(), repo_id).await;

    std::fs::remove_dir_all(watched.path().join("fable")).unwrap();

    assert!(!opened(&app, id).await.ready_to_grill);
    assert_eq!(grill(&app, id).await, GrillingStarted::ProfileBroken);
}

/// The Brief is what the grilling starts from, and freezing an empty one would
/// freeze nothing worth having.
#[tokio::test]
async fn starting_is_refused_when_the_brief_is_empty() {
    let (watched, _dir, app, _repo, repo_id) = workbench().await;
    let id = started(&app, repo_id).await;
    choose(
        &app,
        id,
        "grilling",
        profile(&app, watched.path(), "fable").await,
    )
    .await;
    choose(
        &app,
        id,
        "implementation",
        profile(&app, watched.path(), "opus").await,
    )
    .await;

    assert!(!opened(&app, id).await.ready_to_grill);
    assert_eq!(grill(&app, id).await, GrillingStarted::EmptyBrief);

    // Whitespace is not a Brief either.
    write_brief(&app, id, "   \n\n").await;
    assert_eq!(grill(&app, id).await, GrillingStarted::EmptyBrief);

    write_brief(&app, id, "# Rate limiting\n").await;
    assert_eq!(grill(&app, id).await, GrillingStarted::Started);
}

/// A commit that resolved when the human typed it can be gone by the time the
/// button is pressed, which is exactly why it is asked again.
#[tokio::test]
async fn starting_is_refused_when_the_base_commit_no_longer_resolves() {
    let (watched, _dir, app, repo, repo_id) = workbench().await;
    let id = ready(&app, watched.path(), repo_id).await;

    // A commit that exists to be recorded and then is reachable from nothing, so
    // that expiring the reflog and pruning takes it away for good.
    std::fs::write(repo.join("second.md"), "# more\n").unwrap();
    git(&repo, &["add", "second.md"]);
    git(&repo, &["commit", "-m", "second"]);
    let doomed = git(&repo, &["rev-parse", "HEAD"]).trim().to_owned();

    assert_eq!(base(&app, id, Some(&doomed)).await, BaseRecorded::Recorded);

    git(&repo, &["reset", "--hard", "HEAD~1"]);
    git(&repo, &["reflog", "expire", "--expire=now", "--all"]);
    git(&repo, &["gc", "--prune=now", "--quiet"]);

    assert_eq!(grill(&app, id).await, GrillingStarted::NoBaseCommit);

    // And nothing was made on the way to finding out.
    let view = opened(&app, id).await;
    assert_eq!(view.state, Lifecycle::Draft);
    assert_eq!(view.worktree, None);
    assert!(worktrees(&repo).len() == 1, "only the repository itself");
}

/// Verkstead did not make the branch, so it will not take it over: what is on it
/// is somebody's work.
#[tokio::test]
async fn starting_is_refused_when_the_branch_is_already_there() {
    let (watched, _dir, app, repo, repo_id) = workbench().await;
    let id = ready(&app, watched.path(), repo_id).await;

    let branch = opened(&app, id).await.branch;
    git(&repo, &["branch", &branch]);

    assert_eq!(grill(&app, id).await, GrillingStarted::BranchExists);
    assert_eq!(opened(&app, id).await.state, Lifecycle::Draft);
}

/// Two branches and two worktrees for one piece of work is what starting twice
/// would mean.
#[tokio::test]
async fn a_conversation_that_has_started_cannot_start_again() {
    let (watched, _dir, app, repo, repo_id) = workbench().await;
    let id = ready(&app, watched.path(), repo_id).await;

    assert_eq!(grill(&app, id).await, GrillingStarted::Started);
    assert_eq!(grill(&app, id).await, GrillingStarted::NotDrafting);

    assert_eq!(worktrees(&repo).len(), 2, "the repository and one worktree");
}

#[tokio::test]
async fn grilling_a_conversation_that_is_not_there_says_so() {
    let (_watched, _dir, app, _repo, _repo_id) = workbench().await;

    assert_eq!(grill(&app, 404).await, GrillingStarted::NoSuchConversation);

    // An id that is not a number cannot name a Conversation, and gets the same
    // answer — the id comes out of a URL the human may have typed.
    let refused: GrillingStarted = post(
        &app,
        "/api/ui/conversations/nonsense/grill",
        &serde_json::json!({}),
    )
    .await;
    assert_eq!(refused, GrillingStarted::NoSuchConversation);
}

/// The freeze the design states, tripped for the first time: past drafting, the
/// Brief and the branch name stop being the human's to change.
#[tokio::test]
async fn grilling_freezes_the_brief_and_the_branch_name() {
    let (watched, _dir, app, _repo, repo_id) = workbench().await;
    let id = ready(&app, watched.path(), repo_id).await;
    let branch = opened(&app, id).await.branch;

    grill(&app, id).await;

    assert_eq!(
        write_brief(&app, id, "# Something else\n").await,
        BriefSaved::NotDrafting
    );
    assert_eq!(
        rename(&app, id, "something-else").await,
        BranchRenamed::NotDrafting
    );
    assert_eq!(
        base(&app, id, Some("HEAD")).await,
        BaseRecorded::NotDrafting
    );

    let view = opened(&app, id).await;
    assert_eq!(view.branch, branch);
    assert_eq!(
        brief(&view).markdown,
        "# Rate limiting\n\nThe API has none.\n"
    );
}

/// Aborting takes the directory away and leaves the branch, because a branch is
/// cheap and may hold work worth reading.
#[tokio::test]
async fn aborting_removes_the_worktree_and_keeps_the_branch() {
    let (watched, _dir, app, repo, repo_id) = workbench().await;
    let id = ready(&app, watched.path(), repo_id).await;
    grill(&app, id).await;

    let view = opened(&app, id).await;
    let branch = view.branch.clone();
    let path = PathBuf::from(view.worktree.unwrap().path);

    assert_eq!(abort(&app, id).await, ConversationAborted::Aborted);

    assert!(!path.exists(), "the worktree directory should be gone");
    assert_eq!(
        worktrees(&repo).len(),
        1,
        "git should hold only the repository"
    );
    assert!(
        !git(
            &repo,
            &["rev-parse", "--verify", &format!("refs/heads/{branch}")]
        )
        .trim()
        .is_empty(),
        "the branch should still be there"
    );

    let view = opened(&app, id).await;
    assert_eq!(view.state, Lifecycle::Aborted);
    assert_eq!(view.worktree, None);
    assert_eq!(moves(&view), [Lifecycle::Grilling, Lifecycle::Aborted]);
}

#[tokio::test]
async fn aborting_twice_is_not_an_error() {
    let (watched, _dir, app, _repo, repo_id) = workbench().await;
    let id = ready(&app, watched.path(), repo_id).await;
    grill(&app, id).await;

    assert_eq!(abort(&app, id).await, ConversationAborted::Aborted);
    assert_eq!(abort(&app, id).await, ConversationAborted::AlreadyAborted);

    assert_eq!(
        moves(&opened(&app, id).await),
        [Lifecycle::Grilling, Lifecycle::Aborted]
    );
}

/// Aborting is reachable from every state this stage can reach, including the
/// one where nothing was ever made.
#[tokio::test]
async fn a_drafting_conversation_can_be_aborted() {
    let (_watched, _dir, app, _repo, repo_id) = workbench().await;
    let id = started(&app, repo_id).await;

    assert_eq!(abort(&app, id).await, ConversationAborted::Aborted);

    let view = opened(&app, id).await;
    assert_eq!(view.state, Lifecycle::Aborted);
    assert_eq!(view.worktree, None);
    assert!(!view.ready_to_grill);
}

/// A worktree the human deleted by hand is still an abort that works: what was
/// asked for is that the directory be gone, and it is.
#[tokio::test]
async fn aborting_a_conversation_whose_worktree_has_already_gone_works() {
    let (watched, _dir, app, repo, repo_id) = workbench().await;
    let id = ready(&app, watched.path(), repo_id).await;
    grill(&app, id).await;

    let path = PathBuf::from(opened(&app, id).await.worktree.unwrap().path);
    std::fs::remove_dir_all(&path).unwrap();

    assert_eq!(abort(&app, id).await, ConversationAborted::Aborted);
    assert_eq!(opened(&app, id).await.state, Lifecycle::Aborted);
    assert_eq!(worktrees(&repo).len(), 1, "git should have let it go too");
}

#[tokio::test]
async fn aborting_a_conversation_that_is_not_there_says_so() {
    let (_watched, _dir, app, _repo, _repo_id) = workbench().await;

    assert_eq!(
        abort(&app, 404).await,
        ConversationAborted::NoSuchConversation
    );
}

/// A worktree removed from under Verkstead is a thing to say, not a thing to
/// fail on later.
#[tokio::test]
async fn a_conversation_whose_worktree_has_gone_says_so() {
    let (watched, _dir, app, _repo, repo_id) = workbench().await;
    let id = ready(&app, watched.path(), repo_id).await;
    grill(&app, id).await;

    let worktree = opened(&app, id).await.worktree.unwrap();
    assert!(!worktree.missing);

    std::fs::remove_dir_all(&worktree.path).unwrap();

    let gone = opened(&app, id).await.worktree.expect("still recorded");
    assert_eq!(gone.path, worktree.path, "it still says where it went");
    assert!(gone.missing);
}

/// Two Conversations on one branch name in one Repo cannot share a directory.
#[tokio::test]
async fn two_conversations_wanting_one_name_get_a_directory_each() {
    let (watched, _dir, app, _repo, repo_id) = workbench().await;

    let first = ready(&app, watched.path(), repo_id).await;
    assert_eq!(
        rename(&app, first, "rate-limiting").await,
        BranchRenamed::Renamed
    );
    assert_eq!(grill(&app, first).await, GrillingStarted::Started);

    // The same branch name on a second Conversation. The branch itself is
    // refused — Verkstead made that one — so the name is freed by aborting the
    // first, which keeps the directory taken.
    let second = started(&app, repo_id).await;
    write_brief(&app, second, "# Another\n").await;
    let profiles: Vec<verkstead_render::ProfileEntry> = get(&app, "/api/ui/profiles").await;
    choose(&app, second, "grilling", profiles[0].id).await;
    choose(&app, second, "implementation", profiles[1].id).await;
    assert_eq!(
        rename(&app, second, "rate-limiting-2").await,
        BranchRenamed::Renamed
    );

    // Take the first directory's name for the second by hand, which is the
    // collision the fallback is for.
    let first_path = PathBuf::from(opened(&app, first).await.worktree.unwrap().path);
    let wanted = first_path.with_file_name("verkstead-rate-limiting-2");
    std::fs::create_dir_all(&wanted).unwrap();

    assert_eq!(grill(&app, second).await, GrillingStarted::Started);

    let path = PathBuf::from(opened(&app, second).await.worktree.unwrap().path);
    assert_ne!(
        path, wanted,
        "it should not have taken the directory already there"
    );
    assert_eq!(
        path.file_name().unwrap().to_string_lossy(),
        format!("verkstead-rate-limiting-2-{second}")
    );
}

/// The grilling's closing move as it reaches the server: YAML on the agents'
/// half, exactly as the CLI sends one.
///
/// Sent through the agent endpoint rather than pressed into the store, because
/// what these are about is the whole path — an agent's Set in one end, the
/// human's Answer in the other, and a Conversation that has moved.
const PROPOSING: &str = r#"
title: Ready to build the rate limiter
questions:
  - label: Q9
    text: Ready to build it this way?
    options:
      - n: 1
        text: Yes, go ahead
        recommended: true
      - n: 2
        text: Not yet — more to work through
proposal:
  direction: task-list
  accepted_by: Q9.1
  rationale: |
    Six changes, each independently testable.
"#;

/// And an ordinary round of grilling, which carries no proposal at all.
const ORDINARY: &str = r#"
title: Where the request counter lives
questions:
  - label: Q9
    text: Where should it live?
    options:
      - n: 1
        text: In-process
        recommended: true
"#;

/// Put a Set to the human the way a session does, and hand back its id.
async fn ask(app: &Router, conversation: i64, yaml: &str) -> i64 {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/conversations/{conversation}/api/v1/sets"))
                .header(header::CONTENT_TYPE, "application/yaml")
                .body(Body::from(yaml.to_owned()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let created: verkstead_schema::SetCreated =
        serde_saphyr::from_str(std::str::from_utf8(&body).unwrap()).unwrap();

    created.id
}

/// Answer it from the browser, which is the path the human's own reply takes.
async fn answer(app: &Router, set_id: i64) -> verkstead_render::Submitted {
    answered(
        app,
        set_id,
        serde_json::json!({ "label": "Q9", "selected": 1 }),
    )
    .await
}

/// The same, with an Answer of the test's own choosing — for the ways of
/// answering that are not the Option `accepted_by` names.
async fn answered(
    app: &Router,
    set_id: i64,
    answer: serde_json::Value,
) -> verkstead_render::Submitted {
    post(
        app,
        &format!("/api/ui/sets/{set_id}/response"),
        &serde_json::json!({ "answers": [answer] }),
    )
    .await
}

async fn direct(app: &Router, id: i64, direction: &str) -> DirectionChosen {
    post(
        app,
        &format!("/api/ui/conversations/{id}/direction"),
        &serde_json::json!({ "direction": direction }),
    )
    .await
}

/// The directions a Conversation's Timeline says were chosen, in order.
fn directions(view: &ConversationView) -> Vec<verkstead_schema::Direction> {
    view.timeline
        .iter()
        .filter_map(|event| match event {
            TimelineEvent::Directed(directed) => Some(directed.direction),
            _ => None,
        })
        .collect()
}

/// Write a handoff where a grilling session would have written one: inside the
/// Conversation's own directory under the State Directory, which is bound into
/// its sandbox at `/tmp/verkstead`.
///
/// Written from out here because there is no session in these tests to write it
/// — what they ask is what Verkstead does with the document, not how it came to
/// be there. Hands back where it went, for the tests that ask whether it is still
/// there afterwards.
fn handoff_written(state: &Path, id: i64, markdown: &str) -> PathBuf {
    let directory = state.join("handoffs").join(id.to_string());
    std::fs::create_dir_all(&directory).unwrap();

    let path = directory.join("handoff.md");
    std::fs::write(&path, markdown).unwrap();

    path
}

/// The handoff on a Conversation's Timeline, where a grilling has handed one
/// over.
fn handoff(view: &ConversationView) -> Option<&verkstead_render::HandoffEvent> {
    view.timeline.iter().find_map(|event| match event {
        TimelineEvent::Handoff(handoff) => Some(handoff),
        _ => None,
    })
}

/// A Conversation that is grilling for real: branch, worktree and all.
async fn grilling(app: &Router, watched: &Path, repo_id: i64) -> i64 {
    let id = ready(app, watched, repo_id).await;
    assert_eq!(grill(app, id).await, GrillingStarted::Started);
    id
}

#[tokio::test]
async fn answering_the_closing_proposal_hands_the_work_over_to_the_human() {
    let (watched, _dir, app, _repo, repo_id) = workbench().await;
    let id = grilling(&app, watched.path(), repo_id).await;

    let set = ask(&app, id, PROPOSING).await;
    assert_eq!(
        answer(&app, set).await,
        verkstead_render::Submitted::Accepted
    );

    let view = opened(&app, id).await;

    assert_eq!(view.state, Lifecycle::Direction);
    assert_eq!(
        moves(&view),
        [Lifecycle::Grilling, Lifecycle::Direction],
        "nothing on this page was pressed to get here: the agent proposed and the human answered",
    );

    let proposal = view.proposal.expect("the closing Set proposed a direction");
    assert_eq!(proposal.direction, verkstead_schema::Direction::TaskList);
    assert!(
        proposal.rationale_html.contains("independently testable"),
        "the chooser draws the agent's reasoning, got: {}",
        proposal.rationale_html
    );
    assert!(
        proposal.rationale_html.contains("<p>"),
        "and it arrives as HTML, like every other piece of agent markdown: {}",
        proposal.rationale_html
    );

    assert_eq!(
        view.direction, None,
        "the recommendation is marked, never chosen on the human's behalf",
    );
}

/// The other half of the closing move: the document the grilling wrote before it
/// proposed. Verkstead takes it as the proposal is accepted, and from then on the
/// Timeline holds it — which is what the human reads and what the implementation
/// session is primed with.
#[tokio::test]
async fn the_handoff_the_grilling_wrote_reaches_the_timeline_when_the_proposal_is_accepted() {
    let (watched, dir, app, _repo, repo_id) = workbench().await;
    let id = grilling(&app, watched.path(), repo_id).await;

    let written = handoff_written(
        dir.path(),
        id,
        "# What we settled\n\nAn in-process counter.\n",
    );

    answer(&app, ask(&app, id, PROPOSING).await).await;

    let view = opened(&app, id).await;
    let handoff = handoff(&view).expect("the handoff is on the Timeline");

    assert!(
        handoff.html.contains("in-process counter"),
        "the handoff arrives rendered, like every other piece of agent markdown: {}",
        handoff.html
    );

    assert!(
        !written.exists(),
        "the document is taken rather than copied: the Timeline holds the only one now",
    );
}

/// The handoff is written outside the checkout on purpose. What proves it is git
/// having nothing to say about the worktree afterwards — an agent that later runs
/// `git add -A` is the whole reason the file is not in there.
#[tokio::test]
async fn a_handoff_never_lands_in_the_repository() {
    let (watched, dir, app, _repo, repo_id) = workbench().await;
    let id = grilling(&app, watched.path(), repo_id).await;

    handoff_written(dir.path(), id, "# What we settled\n");

    let view = opened(&app, id).await;
    let worktree = PathBuf::from(view.worktree.expect("a grilling Conversation has one").path);

    assert_eq!(
        git(&worktree, &["status", "--porcelain"]),
        "",
        "the worktree is untouched by a handoff being written",
    );

    answer(&app, ask(&app, id, PROPOSING).await).await;

    assert_eq!(
        git(&worktree, &["status", "--porcelain"]),
        "",
        "and by it being taken",
    );
}

/// A grilling that skipped half its closing move still hands the work over: the
/// Conversation moves, and what follows is primed with the Brief alone.
#[tokio::test]
async fn a_grilling_that_wrote_no_handoff_still_hands_over() {
    let (watched, _dir, app, _repo, repo_id) = workbench().await;
    let id = grilling(&app, watched.path(), repo_id).await;

    answer(&app, ask(&app, id, PROPOSING).await).await;

    let view = opened(&app, id).await;

    assert_eq!(view.state, Lifecycle::Direction);
    assert_eq!(handoff(&view), None);
}

/// A proposal the human sent back is not the grilling ending, so the handoff
/// stays where it is: the session is still holding the thread and will rewrite it
/// before it proposes again.
#[tokio::test]
async fn a_proposal_sent_back_leaves_the_handoff_where_it_was_written() {
    let (watched, dir, app, _repo, repo_id) = workbench().await;
    let id = grilling(&app, watched.path(), repo_id).await;

    let written = handoff_written(dir.path(), id, "# What we settled\n");
    let set = ask(&app, id, PROPOSING).await;

    answered(
        &app,
        set,
        serde_json::json!({ "label": "Q9", "selected": 2 }),
    )
    .await;

    assert!(written.exists(), "nothing was taken");
    assert_eq!(handoff(&opened(&app, id).await), None);
}

#[tokio::test]
async fn answering_an_ordinary_grilling_set_leaves_the_grilling_running() {
    let (watched, _dir, app, _repo, repo_id) = workbench().await;
    let id = grilling(&app, watched.path(), repo_id).await;

    let set = ask(&app, id, ORDINARY).await;
    assert_eq!(
        answer(&app, set).await,
        verkstead_render::Submitted::Accepted
    );

    let view = opened(&app, id).await;

    assert_eq!(view.state, Lifecycle::Grilling);
    assert_eq!(view.proposal, None);
    assert_eq!(moves(&view), [Lifecycle::Grilling]);
}

/// A task list runs off the press too: what starts is the session that writes
/// the backlog, and writing it is the work beginning rather than a step in front
/// of it.
#[tokio::test]
async fn choosing_a_task_list_sets_the_conversation_implementing() {
    let (watched, _dir, app, _repo, repo_id) = workbench().await;
    let id = grilling(&app, watched.path(), repo_id).await;
    answer(&app, ask(&app, id, PROPOSING).await).await;

    assert_eq!(direct(&app, id, "task-list").await, DirectionChosen::Chosen);

    let view = opened(&app, id).await;

    assert_eq!(view.direction, Some(verkstead_schema::Direction::TaskList));
    assert_eq!(directions(&view), [verkstead_schema::Direction::TaskList]);
    assert_eq!(view.state, Lifecycle::Implementing);
    assert_eq!(
        moves(&view),
        [
            Lifecycle::Grilling,
            Lifecycle::Direction,
            Lifecycle::Implementing
        ],
        "the choice is an Event of its own and the work starting is a move",
    );
}

/// Inline is the other, so the Conversation leaves Direction the moment it is
/// chosen. The session itself is another matter — this router has no way to run
/// one, which is exactly why the move and the launch are separate things.
#[tokio::test]
async fn choosing_inline_sets_the_conversation_implementing() {
    let (watched, _dir, app, _repo, repo_id) = workbench().await;
    let id = grilling(&app, watched.path(), repo_id).await;
    answer(&app, ask(&app, id, PROPOSING).await).await;

    assert_eq!(direct(&app, id, "inline").await, DirectionChosen::Chosen);

    let view = opened(&app, id).await;

    assert_eq!(view.direction, Some(verkstead_schema::Direction::Inline));
    assert_eq!(directions(&view), [verkstead_schema::Direction::Inline]);
    assert_eq!(view.state, Lifecycle::Implementing);
    assert_eq!(
        moves(&view),
        [
            Lifecycle::Grilling,
            Lifecycle::Direction,
            Lifecycle::Implementing
        ],
        "the choice is an Event of its own and the work starting is a move",
    );
}

/// The work has started, so there is no longer a choice here to make — however
/// the second press arrived.
#[tokio::test]
async fn choosing_inline_again_once_the_work_has_started_is_refused() {
    let (watched, _dir, app, _repo, repo_id) = workbench().await;
    let id = grilling(&app, watched.path(), repo_id).await;
    answer(&app, ask(&app, id, PROPOSING).await).await;

    assert_eq!(direct(&app, id, "inline").await, DirectionChosen::Chosen);
    assert_eq!(
        direct(&app, id, "inline").await,
        DirectionChosen::NotChoosing
    );

    let view = opened(&app, id).await;
    assert_eq!(
        directions(&view),
        [verkstead_schema::Direction::Inline],
        "the refused press records nothing",
    );
}

/// And the third is no different from the other two now that there is something
/// for it to start: the session it launches writes `docs/roadmaps/`, and writing
/// the roadmap *is* this Conversation's work rather than a step in front of it.
#[tokio::test]
async fn choosing_a_staged_roadmap_sets_the_conversation_implementing() {
    let (watched, _dir, app, _repo, repo_id) = workbench().await;
    let id = grilling(&app, watched.path(), repo_id).await;
    answer(&app, ask(&app, id, PROPOSING).await).await;

    assert_eq!(direct(&app, id, "roadmap").await, DirectionChosen::Chosen);

    let view = opened(&app, id).await;

    assert_eq!(view.direction, Some(verkstead_schema::Direction::Roadmap));
    assert_eq!(directions(&view), [verkstead_schema::Direction::Roadmap]);
    assert_eq!(view.state, Lifecycle::Implementing);
    assert_eq!(
        moves(&view),
        [
            Lifecycle::Grilling,
            Lifecycle::Direction,
            Lifecycle::Implementing
        ],
        "the choice is an Event of its own and the work starting is a move",
    );
}

#[tokio::test]
async fn a_conversation_still_grilling_has_no_direction_to_choose() {
    let (watched, _dir, app, _repo, repo_id) = workbench().await;
    let id = grilling(&app, watched.path(), repo_id).await;

    assert_eq!(
        direct(&app, id, "inline").await,
        DirectionChosen::NotChoosing
    );
    assert_eq!(opened(&app, id).await.direction, None);
}

#[tokio::test]
async fn a_direction_for_a_conversation_that_is_not_there() {
    let (_watched, _dir, app, _repo, _repo_id) = workbench().await;

    assert_eq!(
        direct(&app, 404, "inline").await,
        DirectionChosen::NoSuchConversation
    );

    // And an id that is not a number at all, which is what a typed URL holds.
    let chosen: DirectionChosen = post(
        &app,
        "/api/ui/conversations/nonsense/direction",
        &serde_json::json!({ "direction": "inline" }),
    )
    .await;
    assert_eq!(chosen, DirectionChosen::NoSuchConversation);
}

#[tokio::test]
async fn disagreeing_with_a_proposal_leaves_the_grilling_running() {
    let (watched, _dir, app, _repo, repo_id) = workbench().await;
    let id = grilling(&app, watched.path(), repo_id).await;

    let set = ask(&app, id, PROPOSING).await;

    // Not the Option `accepted_by` names, and words of their own beside it —
    // which is the shape of a human saying what is still open.
    assert_eq!(
        answered(
            &app,
            set,
            serde_json::json!({
                "label": "Q9",
                "selected": 2,
                "free_text": "The migration is still hand-wavy.",
            }),
        )
        .await,
        verkstead_render::Submitted::Accepted,
        "the Response is taken either way: it is the agent's to read",
    );

    let view = opened(&app, id).await;

    assert_eq!(
        view.state,
        Lifecycle::Grilling,
        "only the Option the proposal named ends a grilling",
    );
    assert_eq!(moves(&view), [Lifecycle::Grilling]);
    assert_eq!(
        view.proposal, None,
        "and there is no chooser to draw: nothing was accepted",
    );
}

#[tokio::test]
async fn a_proposal_put_again_after_a_refusal_reaches_the_chooser() {
    let (watched, _dir, app, _repo, repo_id) = workbench().await;
    let id = grilling(&app, watched.path(), repo_id).await;

    // Refused, so the grilling carries on holding the thread.
    answered(
        &app,
        ask(&app, id, PROPOSING).await,
        serde_json::json!({ "label": "Q9", "selected": 2 }),
    )
    .await;
    assert_eq!(opened(&app, id).await.state, Lifecycle::Grilling);

    // The agent read the Response, went back down the branch, and proposed
    // again — this time recommending something else.
    let again = PROPOSING.replace("direction: task-list", "direction: inline");
    answer(&app, ask(&app, id, &again).await).await;

    let view = opened(&app, id).await;

    assert_eq!(view.state, Lifecycle::Direction);
    assert_eq!(
        moves(&view),
        [Lifecycle::Grilling, Lifecycle::Direction],
        "the refusal moved nothing, so it got here once",
    );
    assert_eq!(
        view.proposal
            .expect("the second proposal is the one in force")
            .direction,
        verkstead_schema::Direction::Inline,
        "the chooser is about the latest proposal, not the one that was refused",
    );
}

#[tokio::test]
async fn a_proposal_naming_an_option_the_set_does_not_offer_is_refused_as_it_arrives() {
    let (watched, _dir, app, _repo, repo_id) = workbench().await;
    let id = grilling(&app, watched.path(), repo_id).await;

    // Nothing could ever accept it, so the grilling it was meant to end would
    // run until somebody aborted the conversation.
    let unacceptable = PROPOSING.replace("accepted_by: Q9.1", "accepted_by: Q9.7");

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/conversations/{id}/api/v1/sets"))
                .header(header::CONTENT_TYPE, "application/yaml")
                .body(Body::from(unacceptable))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let refusal = String::from_utf8(body.to_vec()).unwrap();
    assert!(
        refusal.contains("Q9"),
        "the refusal should name the question at fault, got: {refusal}"
    );
}

/// The backlog a session wrote into the worktree, as the breaking-down skill
/// writes one: the list, and a task file per task still to do.
fn plan(worktree: &Path, list: &str, files: &[&str]) {
    let tasks = worktree.join(".tasks");
    std::fs::create_dir_all(&tasks).unwrap();
    std::fs::write(tasks.join("TODO.md"), list).unwrap();

    for file in files {
        std::fs::write(tasks.join(file), "# a task\n\n## What to build\n").unwrap();
    }
}

const BACKLOG: &str = "\
# Rate limiting

Where the counter lives and what a refused request is told.

## Tasks

- [x] 01: The counter — [details](01-counter.md)
- [ ] 02: What a refused request is told — [details](02-refusal.md)
";

/// The task list a view is carrying, of whatever is pinned to it.
fn pinned(view: &ConversationView) -> Option<&verkstead_render::TaskListEvent> {
    view.pinned.iter().find_map(|event| match event {
        PinnedEvent::TaskList(list) => Some(list),
        _ => None,
    })
}

/// A Conversation whose worktree holds a backlog shows it, and shows it pinned
/// rather than as one more thing on the record.
#[tokio::test]
async fn a_backlog_in_the_worktree_is_pinned_to_the_timeline() {
    let (watched, _dir, app, _repo, repo_id) = workbench().await;
    let id = ready(&app, watched.path(), repo_id).await;
    grill(&app, id).await;

    let worktree = PathBuf::from(opened(&app, id).await.worktree.unwrap().path);
    plan(&worktree, BACKLOG, &["02-refusal.md"]);

    let view = opened(&app, id).await;
    let list = pinned(&view).expect("the worktree has a backlog");

    assert_eq!(list.feature, "Rate limiting");
    assert_eq!(
        list.tasks
            .iter()
            .map(|task| (task.number.as_str(), task.title.as_str(), task.done))
            .collect::<Vec<_>>(),
        [
            ("01", "The counter", true),
            ("02", "What a refused request is told", false),
        ]
    );

    // Pinned, which is a thing it is rather than a place it is drawn: nothing on
    // the Timeline itself is the backlog.
    assert!(
        !view
            .timeline
            .iter()
            .any(|event| format!("{event:?}").contains("Rate limiting\"")),
        "the backlog belongs to the pinned set, not to the record"
    );
}

/// What makes it worth pinning: it is the worktree as it stands, so finishing a
/// task moves it without anything being written down.
#[tokio::test]
async fn the_task_list_follows_the_worktree_as_it_changes() {
    let (watched, _dir, app, _repo, repo_id) = workbench().await;
    let id = ready(&app, watched.path(), repo_id).await;
    grill(&app, id).await;

    let worktree = PathBuf::from(opened(&app, id).await.worktree.unwrap().path);
    plan(&worktree, BACKLOG, &["02-refusal.md"]);

    assert!(!pinned(&opened(&app, id).await).unwrap().tasks[1].done);

    // What a session finishing a task does: the file goes, and the entry is
    // ticked off in the same commit.
    std::fs::remove_file(worktree.join(".tasks/02-refusal.md")).unwrap();
    std::fs::write(
        worktree.join(".tasks/TODO.md"),
        BACKLOG.replace("- [ ] 02", "- [x] 02"),
    )
    .unwrap();

    assert!(pinned(&opened(&app, id).await).unwrap().tasks[1].done);

    // And the whole backlog going — which is what finishing a feature does —
    // leaves nothing pinned at all.
    std::fs::remove_dir_all(worktree.join(".tasks")).unwrap();

    assert!(opened(&app, id).await.pinned.is_empty());
}

/// The ordinary case, and the one every Conversation starts in.
#[tokio::test]
async fn a_conversation_with_no_backlog_has_nothing_pinned() {
    let (watched, _dir, app, _repo, repo_id) = workbench().await;
    let id = ready(&app, watched.path(), repo_id).await;

    // Before there is a worktree at all, and after there is one with nothing in
    // it: both are a Conversation with no backlog.
    assert!(opened(&app, id).await.pinned.is_empty());

    grill(&app, id).await;

    assert!(opened(&app, id).await.pinned.is_empty());
}

/// A roadmap committed on a repository's default branch, as the old tools or a
/// human left it: an index with a stage left to do, and the brief to start it
/// from.
///
/// Committed rather than merely written, because that is the whole difference
/// adoption is about — a roadmap Verkstead's own reading sees nothing of,
/// because no branch it knows ever touched it.
fn roadmap(repo: &Path, index: &str, briefs: &[&str]) {
    let directory = repo.join("docs/roadmaps/mvp");
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(directory.join("ROADMAP.md"), index).unwrap();

    for brief in briefs {
        std::fs::write(directory.join(brief), format!("# {brief}\n")).unwrap();
    }

    git(repo, &["add", "-A"]);
    git(repo, &["commit", "-m", "docs: the roadmap as it stands"]);
}

/// The `mvp` roadmap with its third stage still open.
const OPEN_AT_THREE: &str = "\
# MVP roadmap

Turns this askance clone into Verkstead.

## Stages

- [x] 01: Workbench — [brief](01-workbench.md)
- [x] 02: Grilling — [brief](02-grilling.md)
- [ ] 03: Implementation — [brief](03-implementation.md)
- [ ] 04: Wrap-up — [brief](04-wrap-up.md)
";

/// And with that stage ticked off, which is what the stage after it leaves.
const OPEN_AT_FOUR: &str = "\
# MVP roadmap

Turns this askance clone into Verkstead.

## Stages

- [x] 01: Workbench — [brief](01-workbench.md)
- [x] 02: Grilling — [brief](02-grilling.md)
- [x] 03: Implementation — [brief](03-implementation.md)
- [ ] 04: Wrap-up — [brief](04-wrap-up.md)
";

async fn adopt(app: &Router, repo_id: i64, name: &str) -> Started {
    post(
        app,
        "/api/ui/adoptions",
        &serde_json::json!({ "repo_id": repo_id, "roadmap": name }),
    )
    .await
}

async fn adopting(app: &Router, repo_id: i64, name: &str) -> i64 {
    match adopt(app, repo_id, name).await {
        Started::Started { id } => id,
        other => panic!("expected the Conversation to start, got {other:?}"),
    }
}

/// What clicking a roadmap in the abandoned-roadmaps notice makes: a Draft
/// against that Repo, marked as adopting that roadmap, whose page names the
/// roadmap and the stage adopting would start.
#[tokio::test]
async fn adopting_a_roadmap_starts_a_draft_naming_it_and_its_next_stage() {
    let (_watched, _dir, app, repo, repo_id) = workbench().await;
    roadmap(
        &repo,
        OPEN_AT_THREE,
        &["03-implementation.md", "04-wrap-up.md"],
    );

    let id = adopting(&app, repo_id, "mvp").await;

    let sidebar = sidebar(&app).await;
    assert_eq!(sidebar.len(), 1);
    assert_eq!(sidebar[0].id, id);
    assert_eq!(sidebar[0].state, Lifecycle::Draft);

    let view = opened(&app, id).await;
    let adopting = view
        .adopting
        .clone()
        .expect("this Conversation is adopting one");

    assert_eq!(adopting.roadmap, "mvp");
    assert_eq!(adopting.title, "MVP roadmap");

    let stage = adopting.stage.expect("that stage is startable");
    assert_eq!(stage.label, "03");
    assert_eq!(stage.title, "Implementation");
    assert_eq!(stage.brief_path, "docs/roadmaps/mvp/03-implementation.md");
    assert_eq!(
        stage.branch, "implementation",
        "the stage's own slug, which the press names the branch by",
    );

    // And nothing has been adopted by starting it: the Brief is still empty,
    // because the stage brief arrives when the stage does.
    assert_eq!(brief(&view).markdown, "");
    assert!(!view.ready_to_grill);
}

/// An ordinary Conversation is adopting nothing, which is what puts its page on
/// the shape with a Brief to write and a grilling to start.
#[tokio::test]
async fn a_conversation_started_the_ordinary_way_is_adopting_nothing() {
    let (_watched, _dir, app, repo, repo_id) = workbench().await;
    roadmap(
        &repo,
        OPEN_AT_THREE,
        &["03-implementation.md", "04-wrap-up.md"],
    );

    let id = started(&app, repo_id).await;

    assert_eq!(opened(&app, id).await.adopting, None);
}

/// The stage is re-read at whatever the base resolves to, rather than carried
/// over from what the notice showed: a base where the roadmap reads differently
/// — an unmerged predecessor's tip being the case this is for — changes the
/// stage the page names.
#[tokio::test]
async fn the_stage_an_adoption_names_is_read_at_the_base_commit() {
    let (_watched, _dir, app, repo, repo_id) = workbench().await;
    roadmap(
        &repo,
        OPEN_AT_THREE,
        &["03-implementation.md", "04-wrap-up.md"],
    );
    let before = git(&repo, &["rev-parse", "HEAD"]).trim().to_owned();

    // The default branch moves on: stage 03 is ticked off there.
    roadmap(&repo, OPEN_AT_FOUR, &[]);

    let id = adopting(&app, repo_id, "mvp").await;
    assert_eq!(
        stage_of(&opened(&app, id).await).label,
        "04",
        "with no override, the default branch's tip is what is read",
    );

    assert_eq!(base(&app, id, Some(&before)).await, BaseRecorded::Recorded);
    assert_eq!(
        stage_of(&opened(&app, id).await).label,
        "03",
        "read again at the base the human named, where 03 is still open",
    );
}

/// The roadmap is named whatever the repository says about it, and a roadmap
/// with no stage to start at that commit is the roadmap with nothing under it.
/// Which of the ways it can be is the press's to say by name.
#[tokio::test]
async fn an_adoption_names_no_stage_where_the_roadmap_has_none_to_start() {
    let (_watched, _dir, app, repo, repo_id) = workbench().await;
    roadmap(
        &repo,
        OPEN_AT_THREE,
        &["03-implementation.md", "04-wrap-up.md"],
    );

    let id = adopting(&app, repo_id, "public-release").await;
    let adopting = opened(&app, id)
        .await
        .adopting
        .expect("it is adopting one, whatever the repository holds");

    assert_eq!(adopting.roadmap, "public-release");
    assert_eq!(adopting.stage, None);
}

#[tokio::test]
async fn adopting_against_a_repo_that_is_not_registered_says_so() {
    let (_watched, _dir, app, _repo, _repo_id) = workbench().await;

    assert_eq!(adopt(&app, 404, "mvp").await, Started::NoSuchRepo);
    assert!(sidebar(&app).await.is_empty());
}

/// The stage the page names, for the tests that are about which one it is.
fn stage_of(view: &ConversationView) -> &verkstead_render::AdoptedStage {
    view.adopting
        .as_ref()
        .expect("this Conversation is adopting one")
        .stage
        .as_ref()
        .expect("that stage is startable")
}
