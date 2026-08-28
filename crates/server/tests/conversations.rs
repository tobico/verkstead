//! Conversations over the viewer's namespace: starting one against a registered
//! Repo, and everything the human settles about it before any of it runs.
//!
//! Asked of the *server*, through the endpoints, because that is where the
//! decisions are: whether a name is one git would take for a branch, and whether
//! the repository really has the branch the work is to come off. A form that
//! checked either would be a courtesy.
//!
//! Starting the grilling is where that stops being true: the branch and the
//! worktree are made against a real repository, in a real data directory, and
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
    Adopted, BacklogPane, BaseRecorded, BranchRenamed, BriefSaved, CheckRollup, CompanionAdded,
    CompanionBaseRecorded, CompanionBranchRenamed, CompanionMode, CompanionModeChosen,
    CompanionRefusal, CompanionRemoved, ConversationArchived, ConversationClosed,
    ConversationEntry, ConversationSteered, ConversationUnarchived, ConversationView,
    GrillingStarted, Lifecycle, PinnedEvent, ProfileSaved, Registered, RoadmapPane,
    ShowingArchived, Standing, Started, SteerOpened, TimelineEvent,
};
use verkstead_server::{WatchedPaths, open_database, router_watching, store};

/// A router watching `watched`, plus the directory holding its database and its
/// data directory alive.
///
/// One directory holds both, which is what the real server does: the database is
/// `verkstead.db` inside the Data Directory.
async fn app_watching(watched: &Path) -> (tempfile::TempDir, Router) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();
    let watched = WatchedPaths::resolve(&[watched.to_owned()]).unwrap();
    let data_dir = dir.path().to_owned();

    (dir, router_watching(pool, watched, data_dir))
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

/// A watched directory holding one registered repository that was *cloned* from
/// an upstream, and the app over it.
///
/// The upstream is what "origin" means for the rest of a test: commits pushed
/// on to it are commits the clone has not seen, which is the whole state these
/// are about. It lives in the data directory's tempdir rather than the watched
/// one so that nothing registers it by accident.
///
/// Hands back the watched directory, the data directory, the app, the clone,
/// the upstream and the Repo's id.
async fn workbench_with_origin() -> (
    tempfile::TempDir,
    tempfile::TempDir,
    Router,
    PathBuf,
    PathBuf,
    i64,
) {
    let watched = tempfile::tempdir().unwrap();
    let (dir, app) = app_watching(watched.path()).await;

    let upstream = repository(dir.path().join("upstream"));
    let repo = watched.path().join("verkstead");
    git(
        watched.path(),
        &[
            "clone",
            &upstream.to_string_lossy(),
            &repo.to_string_lossy(),
        ],
    );

    // A clone inherits no identity, and a machine that has no global one — which
    // is every machine the checks run on — cannot commit in it. Set here rather
    // than in whichever test commits first, because being able to commit is what
    // a repository is for.
    git(&repo, &["config", "user.email", "test@verkstead.invalid"]);
    git(&repo, &["config", "user.name", "Verkstead Test"]);

    let registered: Registered =
        post(&app, "/api/ui/repos", &serde_json::json!({ "path": repo })).await;
    assert_eq!(registered, Registered::Added);

    let repo_id = listed_repos(&app).await;

    (watched, dir, app, repo, upstream, repo_id)
}

/// Put another commit on `repo`'s checked-out branch, and say what it stands at.
fn commit(repo: &Path, file: &str) -> String {
    std::fs::write(repo.join(file), "# more\n").unwrap();
    git(repo, &["add", file]);
    git(repo, &["commit", "-m", "another"]);

    git(repo, &["rev-parse", "HEAD"]).trim().to_owned()
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

/// Every Brief on a Conversation's Timeline, in order: one per round.
fn briefs(view: &ConversationView) -> Vec<&verkstead_render::BriefEvent> {
    view.timeline
        .iter()
        .filter_map(|event| match event {
            TimelineEvent::Brief(brief) => Some(brief),
            _ => None,
        })
        .collect()
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

async fn base(app: &Router, id: i64, branch: Option<&str>) -> BaseRecorded {
    post(
        app,
        &format!("/api/ui/conversations/{id}/base"),
        &serde_json::json!({ "branch": branch }),
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

/// What is stored is the branch's name rather than where it stands: the whole
/// point of picking one is coming off whatever is on it when the work starts.
#[tokio::test]
async fn a_picked_branch_is_recorded_by_name() {
    let (_watched, _dir, app, repo, repo_id) = workbench().await;
    let id = started(&app, repo_id).await;

    git(&repo, &["branch", "release"]);

    assert_eq!(
        base(&app, id, Some("release")).await,
        BaseRecorded::Recorded
    );
    assert_eq!(
        opened(&app, id).await.base_commit.as_deref(),
        Some("release")
    );
}

/// A remote-tracking branch is as pickable as a local one: an unmerged branch
/// somebody else pushed is a thing to build on, and it is not checked out here.
#[tokio::test]
async fn a_remote_tracking_branch_is_pickable_too() {
    let (_watched, _dir, app, repo, repo_id) = workbench().await;
    let id = started(&app, repo_id).await;

    let head = git(&repo, &["rev-parse", "HEAD"]).trim().to_owned();
    git(&repo, &["update-ref", "refs/remotes/origin/theirs", &head]);

    assert_eq!(
        base(&app, id, Some("origin/theirs")).await,
        BaseRecorded::Recorded
    );
    assert_eq!(
        opened(&app, id).await.base_commit.as_deref(),
        Some("origin/theirs")
    );
}

/// Refused now rather than at grill start, where it would be a failure with
/// nobody watching — and refused for a commit that resolves perfectly well,
/// because a branch is the whole of what there is to pick.
#[tokio::test]
async fn anything_that_is_not_one_of_the_repos_branches_is_refused() {
    let (_watched, _dir, app, repo, repo_id) = workbench().await;
    let id = started(&app, repo_id).await;

    let head = git(&repo, &["rev-parse", "HEAD"]).trim().to_owned();
    git(&repo, &["tag", "v0.1.0"]);

    for asked in ["nowhere", "v0.1.0", head.as_str()] {
        assert_eq!(
            base(&app, id, Some(asked)).await,
            BaseRecorded::NoSuchBranch,
            "{asked} is not a branch of that repo"
        );
        assert_eq!(opened(&app, id).await.base_commit, None);
    }
}

/// Picking the first entry of the dropdown is the override taken away, not a
/// branch called nothing — and what it goes back to is the rule.
#[tokio::test]
async fn clearing_the_base_branch_puts_the_conversation_back_on_the_rule() {
    let (_watched, _dir, app, _repo, repo_id) = workbench().await;
    let id = started(&app, repo_id).await;

    base(&app, id, Some("main")).await;

    for cleared in [None, Some("")] {
        assert_eq!(base(&app, id, cleared).await, BaseRecorded::Recorded);
        assert_eq!(opened(&app, id).await.base_commit, None);
    }
}

/// A second registered repository in the same watched directory, for the tests
/// about working alongside one. Hands back its Repo id.
async fn second_repo(app: &Router, watched: &Path, name: &str) -> i64 {
    let path = repository(watched.join(name));

    let registered: Registered =
        post(app, "/api/ui/repos", &serde_json::json!({ "path": path })).await;
    assert_eq!(registered, Registered::Added);

    let repos: Vec<verkstead_render::RepoEntry> = get(app, "/api/ui/repos").await;
    repos
        .into_iter()
        .find(|repo| repo.name == name)
        .expect("it was just registered")
        .id
}

async fn add_companion(app: &Router, id: i64, repo_id: i64) -> CompanionAdded {
    post(
        app,
        &format!("/api/ui/conversations/{id}/companions"),
        &serde_json::json!({ "repo_id": repo_id }),
    )
    .await
}

async fn remove_companion(app: &Router, id: i64, repo_id: i64) -> CompanionRemoved {
    post(
        app,
        &format!("/api/ui/conversations/{id}/companions/{repo_id}/remove"),
        &serde_json::json!({}),
    )
    .await
}

/// What a Conversation says it works alongside, by Repo name.
async fn companions(app: &Router, id: i64) -> Vec<String> {
    opened(app, id)
        .await
        .companions
        .into_iter()
        .map(|companion| companion.repo.name)
        .collect()
}

/// The two ends of it: a registered Repo added to a drafting Conversation, and
/// taken away again.
#[tokio::test]
async fn a_repo_is_added_to_work_alongside_and_taken_away_again() {
    let (watched, _dir, app, _repo, repo_id) = workbench().await;
    let askance = second_repo(&app, watched.path(), "askance").await;
    let id = started(&app, repo_id).await;

    assert!(companions(&app, id).await.is_empty());

    assert_eq!(
        add_companion(&app, id, askance).await,
        CompanionAdded::Added
    );

    // With the least the human had to say filled in: read it, off its own
    // default branch, on no branch of its own.
    let added = opened(&app, id).await.companions;
    assert_eq!(added.len(), 1);
    assert_eq!(added[0].repo.id, askance);
    assert_eq!(added[0].repo.name, "askance");
    assert_eq!(added[0].mode, CompanionMode::ReadOnly);
    assert_eq!(added[0].base_ref, None);
    assert_eq!(added[0].branch, "");

    assert_eq!(
        remove_companion(&app, id, askance).await,
        CompanionRemoved::Removed
    );
    assert!(companions(&app, id).await.is_empty());
}

async fn companion_mode(
    app: &Router,
    id: i64,
    repo_id: i64,
    mode: CompanionMode,
) -> CompanionModeChosen {
    post(
        app,
        &format!("/api/ui/conversations/{id}/companions/{repo_id}/mode"),
        &serde_json::json!({ "mode": mode }),
    )
    .await
}

async fn companion_base(
    app: &Router,
    id: i64,
    repo_id: i64,
    branch: Option<&str>,
) -> CompanionBaseRecorded {
    post(
        app,
        &format!("/api/ui/conversations/{id}/companions/{repo_id}/base"),
        &serde_json::json!({ "branch": branch }),
    )
    .await
}

async fn companion_branch(
    app: &Router,
    id: i64,
    repo_id: i64,
    branch: &str,
) -> CompanionBranchRenamed {
    post(
        app,
        &format!("/api/ui/conversations/{id}/companions/{repo_id}/branch"),
        &serde_json::json!({ "branch": branch }),
    )
    .await
}

/// The one companion of a Conversation, for the tests that configure it.
async fn only_companion(app: &Router, id: i64) -> verkstead_render::CompanionView {
    let mut companions = opened(app, id).await.companions;
    assert_eq!(companions.len(), 1);
    companions.remove(0)
}

/// The three things a row settles about a companion, each landing on its own.
#[tokio::test]
async fn a_companion_is_configured_on_the_row_it_draws() {
    let (watched, _dir, app, _repo, repo_id) = workbench().await;
    let askance = second_repo(&app, watched.path(), "askance").await;
    let id = started(&app, repo_id).await;

    assert_eq!(
        add_companion(&app, id, askance).await,
        CompanionAdded::Added
    );

    assert_eq!(
        companion_mode(&app, id, askance, CompanionMode::ReadWrite).await,
        CompanionModeChosen::Chosen
    );
    assert_eq!(
        only_companion(&app, id).await.mode,
        CompanionMode::ReadWrite
    );

    // The branch of the *companion's* own repository, which is a different
    // repository with a list of its own.
    assert_eq!(
        companion_base(&app, id, askance, Some("main")).await,
        CompanionBaseRecorded::Recorded
    );
    assert_eq!(
        only_companion(&app, id).await.base_ref,
        Some("main".to_owned())
    );

    assert_eq!(
        companion_branch(&app, id, askance, "alongside").await,
        CompanionBranchRenamed::Renamed
    );
    assert_eq!(only_companion(&app, id).await.branch, "alongside");
}

/// Empty is not a name git is asked about: it is *mirroring* — the
/// Conversation's own branch name, followed as it is renamed — which is what a
/// companion starts on and what clearing the field goes back to.
#[tokio::test]
async fn an_empty_companion_branch_is_mirroring_rather_than_a_name() {
    let (watched, _dir, app, _repo, repo_id) = workbench().await;
    let askance = second_repo(&app, watched.path(), "askance").await;
    let id = started(&app, repo_id).await;

    add_companion(&app, id, askance).await;

    assert_eq!(only_companion(&app, id).await.branch, "");
    assert_eq!(
        companion_branch(&app, id, askance, "alongside").await,
        CompanionBranchRenamed::Renamed
    );
    assert_eq!(
        companion_branch(&app, id, askance, "").await,
        CompanionBranchRenamed::Renamed
    );
    assert_eq!(only_companion(&app, id).await.branch, "");

    // And a name git will not take is refused, as the Conversation's own is.
    assert_eq!(
        companion_branch(&app, id, askance, "not a branch").await,
        CompanionBranchRenamed::NotABranchName
    );
    assert_eq!(only_companion(&app, id).await.branch, "");
}

/// A read-only companion has no branch, being checked out detached — so the
/// name goes with the mode rather than sitting in the record for a branch
/// nobody will cut.
#[tokio::test]
async fn flipping_a_companion_back_to_read_only_takes_its_branch_name_with_it() {
    let (watched, _dir, app, _repo, repo_id) = workbench().await;
    let askance = second_repo(&app, watched.path(), "askance").await;
    let id = started(&app, repo_id).await;

    add_companion(&app, id, askance).await;
    companion_mode(&app, id, askance, CompanionMode::ReadWrite).await;
    companion_branch(&app, id, askance, "alongside").await;

    assert_eq!(
        companion_mode(&app, id, askance, CompanionMode::ReadOnly).await,
        CompanionModeChosen::Chosen
    );

    let companion = only_companion(&app, id).await;
    assert_eq!(companion.mode, CompanionMode::ReadOnly);
    assert_eq!(companion.branch, "");

    // The base is left where it was: what a checkout comes off is the same
    // question either way round.
    assert_eq!(companion.base_ref, None);
}

/// The base is one of the companion repository's own branches, and nothing
/// else: a sha or a tag resolves and is still not something there is a way to
/// pick.
#[tokio::test]
async fn a_companions_base_is_one_of_its_own_branches() {
    let (watched, _dir, app, repo, repo_id) = workbench().await;
    let askance = second_repo(&app, watched.path(), "askance").await;
    let id = started(&app, repo_id).await;

    add_companion(&app, id, askance).await;

    // A branch of the *Conversation's* repository is not a branch of the
    // companion's, however plausible it reads.
    git(&repo, &["branch", "release-1.4"]);
    assert_eq!(
        companion_base(&app, id, askance, Some("release-1.4")).await,
        CompanionBaseRecorded::NoSuchBranch
    );
    assert_eq!(only_companion(&app, id).await.base_ref, None);

    // And the first entry of the dropdown is the override taken away rather
    // than a branch called nothing.
    companion_base(&app, id, askance, Some("main")).await;
    for cleared in [None, Some("")] {
        assert_eq!(
            companion_base(&app, id, askance, cleared).await,
            CompanionBaseRecorded::Recorded
        );
        assert_eq!(only_companion(&app, id).await.base_ref, None);
    }
}

/// The whole configuration freezes together: past grill start every one of the
/// three is refused, whatever a stale page believed.
#[tokio::test]
async fn configuring_a_companion_is_settled_once_the_grilling_has_started() {
    let (watched, _dir, app, _repo, repo_id) = workbench().await;
    let askance = second_repo(&app, watched.path(), "askance").await;
    let id = ready(&app, watched.path(), repo_id).await;

    add_companion(&app, id, askance).await;
    assert_eq!(grill(&app, id).await, GrillingStarted::Started);

    assert_eq!(
        companion_mode(&app, id, askance, CompanionMode::ReadWrite).await,
        CompanionModeChosen::NotDrafting
    );
    assert_eq!(
        companion_base(&app, id, askance, Some("main")).await,
        CompanionBaseRecorded::NotDrafting
    );
    assert_eq!(
        companion_branch(&app, id, askance, "alongside").await,
        CompanionBranchRenamed::NotDrafting
    );

    let companion = only_companion(&app, id).await;
    assert_eq!(companion.mode, CompanionMode::ReadOnly);
    assert_eq!(companion.base_ref, None);
    assert_eq!(companion.branch, "");
}

/// A row taken off in one tab and configured in another: the press did nothing,
/// which is worth saying rather than reporting as done.
#[tokio::test]
async fn a_repo_that_is_not_a_companion_is_not_configured() {
    let (watched, _dir, app, _repo, repo_id) = workbench().await;
    let askance = second_repo(&app, watched.path(), "askance").await;
    let id = started(&app, repo_id).await;

    assert_eq!(
        companion_mode(&app, id, askance, CompanionMode::ReadWrite).await,
        CompanionModeChosen::NoSuchCompanion
    );
    assert_eq!(
        companion_base(&app, id, askance, Some("main")).await,
        CompanionBaseRecorded::NoSuchCompanion
    );
    assert_eq!(
        companion_branch(&app, id, askance, "alongside").await,
        CompanionBranchRenamed::NoSuchCompanion
    );
    assert!(companions(&app, id).await.is_empty());
}

/// The work is being done in its own repository already, so adding it beside
/// itself would be that repository twice in one sandbox.
#[tokio::test]
async fn a_conversation_is_not_a_companion_of_itself() {
    let (_watched, _dir, app, _repo, repo_id) = workbench().await;
    let id = started(&app, repo_id).await;

    assert_eq!(
        add_companion(&app, id, repo_id).await,
        CompanionAdded::OwnRepo
    );
    assert!(companions(&app, id).await.is_empty());
}

/// And one repository is one companion: a second press on the same row says so
/// rather than making a second checkout of it.
#[tokio::test]
async fn a_repo_already_added_is_not_added_twice() {
    let (watched, _dir, app, _repo, repo_id) = workbench().await;
    let askance = second_repo(&app, watched.path(), "askance").await;
    let id = started(&app, repo_id).await;

    assert_eq!(
        add_companion(&app, id, askance).await,
        CompanionAdded::Added
    );
    assert_eq!(
        add_companion(&app, id, askance).await,
        CompanionAdded::AlreadyAdded
    );

    assert_eq!(companions(&app, id).await, ["askance"]);
}

/// The registry is the trust boundary: what is not in it is not something a
/// Conversation may compose into its sandbox.
#[tokio::test]
async fn a_repo_that_is_not_registered_is_not_a_companion() {
    let (_watched, _dir, app, _repo, repo_id) = workbench().await;
    let id = started(&app, repo_id).await;

    assert_eq!(
        add_companion(&app, id, repo_id + 404).await,
        CompanionAdded::NoSuchRepo
    );
    assert!(companions(&app, id).await.is_empty());
}

/// The configuration freezes with the branch and the base: past grill start
/// there is no setup card to press, and every press is refused whatever a stale
/// page believed.
#[tokio::test]
async fn companions_are_settled_once_the_grilling_has_started() {
    let (watched, _dir, app, _repo, repo_id) = workbench().await;
    let askance = second_repo(&app, watched.path(), "askance").await;
    let alone = second_repo(&app, watched.path(), "alone").await;
    let id = ready(&app, watched.path(), repo_id).await;

    assert_eq!(
        add_companion(&app, id, askance).await,
        CompanionAdded::Added
    );
    assert_eq!(grill(&app, id).await, GrillingStarted::Started);

    assert_eq!(
        add_companion(&app, id, alone).await,
        CompanionAdded::NotDrafting
    );
    assert_eq!(
        remove_companion(&app, id, askance).await,
        CompanionRemoved::NotDrafting
    );

    // And what it was configured with is still exactly what it froze with.
    assert_eq!(companions(&app, id).await, ["askance"]);
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
    assert_eq!(
        add_companion(&app, 404, 1).await,
        CompanionAdded::NoSuchConversation
    );
    assert_eq!(
        remove_companion(&app, 404, 1).await,
        CompanionRemoved::NoSuchConversation
    );
    assert_eq!(
        companion_mode(&app, 404, 1, CompanionMode::ReadWrite).await,
        CompanionModeChosen::NoSuchConversation
    );
    assert_eq!(
        companion_base(&app, 404, 1, Some("main")).await,
        CompanionBaseRecorded::NoSuchConversation
    );
    assert_eq!(
        companion_branch(&app, 404, 1, "alongside").await,
        CompanionBranchRenamed::NoSuchConversation
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

/// The order of the sidebar as it comes back over the wire, by id.
async fn order(app: &Router) -> Vec<i64> {
    sidebar(app).await.into_iter().map(|row| row.id).collect()
}

/// Say where the whole list goes, which is what letting go of a dragged row
/// sends. Answered with nothing, because there is nothing to answer.
async fn place(app: &Router, ids: &[i64]) {
    let (status, body) = fetch(
        app,
        Request::builder()
            .method("POST")
            .uri("/api/ui/conversations/order")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({ "order": ids })).unwrap(),
            ))
            .unwrap(),
    )
    .await;

    assert_eq!(status, StatusCode::NO_CONTENT, "placing failed: {body}");
}

#[tokio::test]
async fn the_sidebar_comes_back_in_the_order_it_was_dragged_into() {
    let (_watched, _dir, app, _repo, repo_id) = workbench().await;
    let first = started(&app, repo_id).await;
    let second = started(&app, repo_id).await;
    let third = started(&app, repo_id).await;

    assert_eq!(
        order(&app).await,
        vec![third, second, first],
        "unplaced, the list is newest first",
    );

    place(&app, &[second, first, third]).await;

    assert_eq!(
        order(&app).await,
        vec![second, first, third],
        "and afterwards it is where the human put it — which is what a reload, a \
         restart and a second device each read",
    );
}

/// The one row nobody could have placed, because it did not exist when they
/// dragged. Above the order rather than at the end of it: it is where the work
/// they just started will be looked for.
#[tokio::test]
async fn a_conversation_started_after_the_order_lands_at_the_top() {
    let (_watched, _dir, app, _repo, repo_id) = workbench().await;
    let first = started(&app, repo_id).await;
    let second = started(&app, repo_id).await;

    place(&app, &[first, second]).await;
    let third = started(&app, repo_id).await;

    assert_eq!(order(&app).await, vec![third, first, second]);
}

/// A viewer sends the list it drew, and a row can be gone by the time it lands.
#[tokio::test]
async fn an_order_naming_a_conversation_that_is_not_there_is_still_taken() {
    let (_watched, _dir, app, _repo, repo_id) = workbench().await;
    let first = started(&app, repo_id).await;
    let second = started(&app, repo_id).await;

    place(&app, &[second, 9_999, first]).await;

    assert_eq!(order(&app).await, vec![second, first]);
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
            "models": ["claude-opus-5"],
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

/// Pair a Profile with the one model [`profile`] gives every Profile here, for
/// one of a Conversation's two roles.
async fn choose(app: &Router, id: i64, role: &str, profile_id: i64) {
    let chosen: verkstead_render::ProfileChosen = post(
        app,
        &format!("/api/ui/conversations/{id}/{role}-pairing"),
        &serde_json::json!({ "profile_id": profile_id, "model": "claude-opus-5" }),
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

async fn close(app: &Router, id: i64) -> ConversationClosed {
    post(
        app,
        &format!("/api/ui/conversations/{id}/close"),
        &serde_json::json!({}),
    )
    .await
}

/// And the row that does both at once, which says as little as either of them.
async fn close_and_archive(app: &Router, id: i64) -> ConversationClosed {
    post(
        app,
        &format!("/api/ui/conversations/{id}/close-and-archive"),
        &serde_json::json!({}),
    )
    .await
}

/// And put a closed one away, which is Close's neighbour in the same menu and
/// says as little for itself.
async fn archive(app: &Router, id: i64) -> ConversationArchived {
    post(
        app,
        &format!("/api/ui/conversations/{id}/archive"),
        &serde_json::json!({}),
    )
    .await
}

/// And take it back out again, which is the same row saying the other word.
async fn unarchive(app: &Router, id: i64) -> ConversationUnarchived {
    post(
        app,
        &format!("/api/ui/conversations/{id}/unarchive"),
        &serde_json::json!({}),
    )
    .await
}

/// Whether the sidebar is drawing what has been put away.
async fn showing_archived(app: &Router) -> bool {
    get::<ShowingArchived>(app, "/api/ui/conversations/archived")
        .await
        .showing
}

/// And putting that switch where the human has put it. Answered with nothing,
/// as the order is, because there is nothing to answer.
async fn show_archived(app: &Router, showing: bool) {
    let (status, body) = fetch(
        app,
        Request::builder()
            .method("POST")
            .uri("/api/ui/conversations/archived")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({ "showing": showing })).unwrap(),
            ))
            .unwrap(),
    )
    .await;

    assert_eq!(status, StatusCode::NO_CONTENT, "the toggle failed: {body}");
}

/// Click Steer, which is the press that stops the drive and opens the modal.
///
/// Nothing goes with it, as nothing goes with either stop: which Conversation it
/// is is the whole of what a click says.
async fn steer(app: &Router, id: i64) -> SteerOpened {
    post(
        app,
        &format!("/api/ui/conversations/{id}/steer"),
        &serde_json::json!({}),
    )
    .await
}

/// And submit the modal it opened: where the work goes, and what to do about
/// anything still running.
async fn steer_into(app: &Router, id: i64, target: &str, interrupt: bool) -> ConversationSteered {
    post(
        app,
        &format!("/api/ui/conversations/{id}/steer/submit"),
        &serde_json::json!({ "target": target, "interrupt": interrupt }),
    )
    .await
}

/// And the submit into Implementing with something written, which is the other
/// payload: what the session it starts is sent off to do.
async fn steer_instructed(app: &Router, id: i64, instruction: &str) -> ConversationSteered {
    post(
        app,
        &format!("/api/ui/conversations/{id}/steer/submit"),
        &serde_json::json!({
            "target": "Implementing",
            "interrupt": false,
            "instruction": instruction,
        }),
    )
    .await
}

/// And the submit into Follow-up, which carries the one payload that is always
/// required: the brief the session it starts opens the follow-up on.
async fn steer_following_up(app: &Router, id: i64, brief: Option<&str>) -> ConversationSteered {
    post(
        app,
        &format!("/api/ui/conversations/{id}/steer/submit"),
        &serde_json::json!({
            "target": "FollowUp",
            "interrupt": false,
            "follow_up": brief,
        }),
    )
    .await
}

/// And the same submit into Grilling, which is the one target that carries a
/// payload: the round's Brief where the human wrote one.
///
/// The digest is left off, that being the default and the one thing these cannot
/// read back — what it primes is a session's prompt, and no session runs here.
async fn steer_grilling(app: &Router, id: i64, brief: Option<&str>) -> ConversationSteered {
    post(
        app,
        &format!("/api/ui/conversations/{id}/steer/submit"),
        &serde_json::json!({
            "target": "Grilling",
            "interrupt": false,
            "brief": brief,
        }),
    )
    .await
}

/// What a Conversation's Timeline says about where the work went, in order: the
/// states the human steered it into and the states it moved to.
///
/// Both kinds together, because what a steer leaves is the pair — the human's
/// own line and the machine's move under it — and a reading that kept only one
/// of them could not say they stand beside each other.
fn steered(view: &ConversationView) -> Vec<(&'static str, Lifecycle)> {
    view.timeline
        .iter()
        .filter_map(|event| match event {
            TimelineEvent::Steer(steer) => Some(("steer", steer.target)),
            TimelineEvent::Moved(moved) => Some(("moved", moved.state)),
            _ => None,
        })
        .collect()
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
/// worktree registered with it under the data directory, and a Conversation
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

    // Named for the Repo and the branch, under the data directory — which is
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

/// The picked branch is what the work branches from, and it is not the default
/// branch's tip.
#[tokio::test]
async fn the_picked_branch_is_what_the_branch_is_made_off() {
    let (watched, _dir, app, repo, repo_id) = workbench().await;
    let id = ready(&app, watched.path(), repo_id).await;

    let first = git(&repo, &["rev-parse", "HEAD"]).trim().to_owned();
    git(&repo, &["branch", "release"]);
    std::fs::write(repo.join("second.md"), "# more\n").unwrap();
    git(&repo, &["add", "second.md"]);
    git(&repo, &["commit", "-m", "second"]);

    assert_eq!(
        base(&app, id, Some("release")).await,
        BaseRecorded::Recorded
    );
    assert_eq!(grill(&app, id).await, GrillingStarted::Started);

    // The name while it was a choice; the commit it stood at once the work is on
    // it, which is what the branch was actually made off.
    let view = opened(&app, id).await;
    assert_eq!(view.base_commit.as_deref(), Some(first.as_str()));

    let worktree = PathBuf::from(view.worktree.unwrap().path);
    assert_eq!(git(&worktree, &["rev-parse", "HEAD"]).trim(), first);
    assert!(
        !worktree.join("second.md").exists(),
        "the worktree should hold the commit it branched from, not the tip"
    );
}

/// A branch is a moving target and picking one says so: what the work comes off
/// is wherever it stands when grilling starts, not where it stood when it was
/// picked.
#[tokio::test]
async fn a_picked_branch_is_resolved_where_it_stands_at_grill_start() {
    let (watched, _dir, app, repo, repo_id) = workbench().await;
    let id = ready(&app, watched.path(), repo_id).await;

    git(&repo, &["branch", "release"]);
    assert_eq!(
        base(&app, id, Some("release")).await,
        BaseRecorded::Recorded
    );

    // The branch moves on after it was picked, which is the whole question.
    std::fs::write(repo.join("second.md"), "# more\n").unwrap();
    git(&repo, &["add", "second.md"]);
    git(&repo, &["commit", "-m", "second"]);
    let moved_to = git(&repo, &["rev-parse", "HEAD"]).trim().to_owned();
    git(&repo, &["branch", "--force", "release", &moved_to]);

    assert_eq!(grill(&app, id).await, GrillingStarted::Started);

    let view = opened(&app, id).await;
    assert_eq!(view.base_commit.as_deref(), Some(moved_to.as_str()));

    let worktree = PathBuf::from(view.worktree.unwrap().path);
    assert!(
        worktree.join("second.md").exists(),
        "the work should come off where the branch stands now"
    );
}

/// The default branch means what origin holds, so a start fetches before it
/// resolves anything: a local `main` that has not been pulled for a week is a
/// week behind the work the branch is meant to come off.
#[tokio::test]
async fn an_unpicked_base_comes_off_origins_tip_rather_than_the_local_branch() {
    let (watched, _dir, app, repo, upstream, repo_id) = workbench_with_origin().await;
    let id = ready(&app, watched.path(), repo_id).await;

    // Origin moves on, and this checkout hears nothing about it: neither its own
    // `main` nor its copy of origin's has any idea.
    let moved_to = commit(&upstream, "second.md");
    let behind = git(&repo, &["rev-parse", "HEAD"]).trim().to_owned();
    assert_ne!(behind, moved_to, "the clone should be behind at this point");

    assert_eq!(grill(&app, id).await, GrillingStarted::Started);

    let view = opened(&app, id).await;
    assert_eq!(
        view.base_commit.as_deref(),
        Some(moved_to.as_str()),
        "the work should come off what origin is holding now"
    );

    let worktree = PathBuf::from(view.worktree.unwrap().path);
    assert!(
        worktree.join("second.md").exists(),
        "and the checkout should hold it"
    );

    // The fetch moved the remote-tracking ref and nothing else: the human's own
    // branch is exactly where they left it.
    assert_eq!(git(&repo, &["rev-parse", "HEAD"]).trim(), behind);
    assert_eq!(git(&repo, &["rev-parse", "main"]).trim(), behind);
}

/// A picked base is still resolved exactly as picked. The fetch only means that
/// a picked remote-tracking branch stands where it now stands.
#[tokio::test]
async fn a_picked_base_is_still_the_one_the_work_comes_off() {
    let (watched, _dir, app, repo, upstream, repo_id) = workbench_with_origin().await;
    let id = ready(&app, watched.path(), repo_id).await;

    let held = git(&repo, &["rev-parse", "HEAD"]).trim().to_owned();
    git(&repo, &["branch", "release"]);
    commit(&upstream, "second.md");

    assert_eq!(
        base(&app, id, Some("release")).await,
        BaseRecorded::Recorded
    );
    assert_eq!(grill(&app, id).await, GrillingStarted::Started);

    assert_eq!(
        opened(&app, id).await.base_commit.as_deref(),
        Some(held.as_str()),
        "a local branch that was picked is not origin's default branch"
    );
}

/// A repository with no remote has nothing to fetch and nothing to be stale
/// against, so it comes off its own default branch and is never refused for it.
#[tokio::test]
async fn a_repo_with_no_remote_comes_off_its_local_default_branch() {
    let (watched, _dir, app, repo, repo_id) = workbench().await;
    let id = ready(&app, watched.path(), repo_id).await;

    let moved_to = commit(&repo, "second.md");

    assert_eq!(grill(&app, id).await, GrillingStarted::Started);

    assert_eq!(
        opened(&app, id).await.base_commit.as_deref(),
        Some(moved_to.as_str())
    );
}

/// Offline, or an authentication that has gone, refuses the press by name
/// rather than quietly branching off refs nobody can vouch for. Something the
/// human can go and fix, which is the whole reason it is named.
#[tokio::test]
async fn a_fetch_that_fails_refuses_the_start_by_name() {
    let (watched, dir, app, repo, _upstream, repo_id) = workbench_with_origin().await;
    let id = ready(&app, watched.path(), repo_id).await;

    let nowhere = dir.path().join("no-such-remote");
    git(
        &repo,
        &["remote", "set-url", "origin", &nowhere.to_string_lossy()],
    );

    assert_eq!(grill(&app, id).await, GrillingStarted::FetchFailed);

    // And nothing was made on the way to finding out.
    let view = opened(&app, id).await;
    assert_eq!(view.state, Lifecycle::Draft);
    assert_eq!(view.worktree, None);
    assert_eq!(worktrees(&repo).len(), 1, "only the repository itself");
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

/// A branch that was there when the human picked it can be gone by the time the
/// button is pressed, which is exactly why it is asked again.
#[tokio::test]
async fn starting_is_refused_when_the_base_branch_no_longer_resolves() {
    let (watched, _dir, app, repo, repo_id) = workbench().await;
    let id = ready(&app, watched.path(), repo_id).await;

    git(&repo, &["branch", "doomed"]);
    assert_eq!(base(&app, id, Some("doomed")).await, BaseRecorded::Recorded);

    git(&repo, &["branch", "-D", "doomed"]);

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

/// The companion of that name, as the Conversation reports it.
fn companion<'a>(view: &'a ConversationView, name: &str) -> &'a verkstead_render::CompanionView {
    view.companions
        .iter()
        .find(|companion| companion.repo.name == name)
        .unwrap_or_else(|| panic!("{name} should be a companion of this Conversation"))
}

/// And where it was checked out.
fn checked_out(view: &ConversationView, name: &str) -> PathBuf {
    let worktree = companion(view, name)
        .worktree
        .clone()
        .unwrap_or_else(|| panic!("{name} should have been checked out"));

    assert!(!worktree.missing, "{name}'s directory should be there");

    PathBuf::from(worktree.path)
}

/// Whether `repo` has a branch by that name.
///
/// `for-each-ref` rather than `rev-parse`, because a branch that is not there is
/// the answer this is asking for rather than a git call that failed.
fn has_branch(repo: &Path, branch: &str) -> bool {
    !git(
        repo,
        &[
            "for-each-ref",
            "--format=%(refname)",
            &format!("refs/heads/{branch}"),
        ],
    )
    .trim()
    .is_empty()
}

/// The whole of what a companion costs the grill start: a checkout of its own
/// under the data directory, detached where it is only read and on a branch of
/// its own where it is worked in, and a record of where each of them went.
#[tokio::test]
async fn starting_a_grilling_checks_every_companion_out() {
    let (watched, dir, app, _repo, repo_id) = workbench().await;
    let reading = second_repo(&app, watched.path(), "askance").await;
    let writing = second_repo(&app, watched.path(), "granit").await;
    let id = ready(&app, watched.path(), repo_id).await;

    add_companion(&app, id, reading).await;
    add_companion(&app, id, writing).await;
    assert_eq!(
        companion_mode(&app, id, writing, CompanionMode::ReadWrite).await,
        CompanionModeChosen::Chosen
    );

    let askance = watched.path().join("askance");
    let granit = watched.path().join("granit");
    let tip = git(&askance, &["rev-parse", "HEAD"]).trim().to_owned();

    assert_eq!(grill(&app, id).await, GrillingStarted::Started);

    let view = opened(&app, id).await;

    // Read-only: detached at the commit its base resolved to, holding no branch
    // at all. There is nothing to commit from it and no business taking a name
    // in somebody else's repository — so what names its directory is the base.
    let read = checked_out(&view, "askance");
    assert_eq!(read.parent(), Some(dir.path().join("worktrees").as_path()));
    assert_eq!(read.file_name().unwrap().to_string_lossy(), "askance-main");
    assert!(read.join("README.md").is_file());
    assert_eq!(git(&read, &["rev-parse", "HEAD"]).trim(), tip);
    assert_eq!(
        git(&read, &["branch", "--show-current"]).trim(),
        "",
        "a read-only companion holds no branch"
    );

    // Read-write: a branch of its own in its own repository, cut from its base
    // and mirroring the Conversation's name, because nobody typed one.
    let written = checked_out(&view, "granit");
    assert_eq!(
        written.parent(),
        Some(dir.path().join("worktrees").as_path())
    );
    assert_eq!(
        written.file_name().unwrap().to_string_lossy(),
        format!("granit-{}", view.branch)
    );
    assert_eq!(
        git(&written, &["branch", "--show-current"]).trim(),
        view.branch
    );
    assert!(
        has_branch(&granit, &view.branch),
        "the branch belongs in the companion's own repository"
    );

    // And git holds both as worktrees, which is what makes them worktrees rather
    // than copies of some files.
    assert!(worktrees(&askance).contains(&read.canonicalize().unwrap()));
    assert!(worktrees(&granit).contains(&written.canonicalize().unwrap()));

    // And what each of them came off is written down, which nothing else knows:
    // the base on a companion's row is a *name*, and the only moment the commit
    // that name stood at is knowable is the one that has just passed.
    assert_eq!(
        companion(&view, "askance").base_commit.as_deref(),
        Some(tip.as_str()),
        "a read-only companion is detached at a commit nothing else records"
    );
    assert_eq!(
        companion(&view, "granit").base_commit.as_deref(),
        Some(git(&granit, &["rev-parse", "HEAD"]).trim()),
        "and a read-write one says what its branch was cut from"
    );
}

/// What a companion's base came to, as the Conversation reports it once it has
/// been checked out.
#[tokio::test]
async fn a_companion_left_on_the_rule_records_what_the_rule_came_to() {
    let (watched, _dir, app, _repo, repo_id) = workbench().await;
    let reading = second_repo(&app, watched.path(), "askance").await;
    let id = ready(&app, watched.path(), repo_id).await;

    add_companion(&app, id, reading).await;

    let askance = watched.path().join("askance");

    // A commit made after the companion was added and before the start, so what
    // is recorded can only have come from resolving the rule at grill start
    // rather than from anything the row was holding.
    std::fs::write(askance.join("LATER.md"), "later\n").unwrap();
    git(&askance, &["add", "LATER.md"]);
    git(&askance, &["commit", "-m", "later"]);

    let moved_on = git(&askance, &["rev-parse", "HEAD"]).trim().to_owned();

    assert_eq!(grill(&app, id).await, GrillingStarted::Started);

    let view = opened(&app, id).await;

    assert_eq!(
        companion(&view, "askance").base_ref,
        None,
        "the row still holds the rule rather than a name"
    );
    assert_eq!(
        companion(&view, "askance").base_commit.as_deref(),
        Some(moved_on.as_str()),
        "and what the rule came to at the start is what was written down"
    );
}

/// Each of the three questions a companion can fail refuses the whole start, and
/// says which repository it was: *which one* is the difference between this and
/// the same failing on the Conversation's own repository.
///
/// And nothing at all is made on the way to finding out. Every question is asked
/// before any of them is answered, so a Conversation refused over its second
/// companion has no branch and no directory anywhere — not the companion's, not
/// the other companion's, and not its own.
#[tokio::test]
async fn a_companion_that_cannot_be_delivered_refuses_the_start_by_name() {
    for why in [
        CompanionRefusal::FetchFailed,
        CompanionRefusal::NoBaseCommit,
        CompanionRefusal::BranchExists,
    ] {
        let (watched, dir, app, repo, repo_id) = workbench().await;
        let companion = second_repo(&app, watched.path(), "askance").await;
        let id = ready(&app, watched.path(), repo_id).await;

        add_companion(&app, id, companion).await;

        let askance = watched.path().join("askance");

        match why {
            // A remote that answers to nothing: what a checkout would come off
            // cannot be trusted to be what the remote is holding.
            CompanionRefusal::FetchFailed => {
                let nowhere = dir.path().join("no-such-remote");
                git(
                    &askance,
                    &["remote", "add", "origin", &nowhere.to_string_lossy()],
                );
            }
            // A base picked while drafting that the repository has since lost.
            CompanionRefusal::NoBaseCommit => {
                git(&askance, &["branch", "doomed"]);
                assert_eq!(
                    companion_base(&app, id, companion, Some("doomed")).await,
                    CompanionBaseRecorded::Recorded
                );
                git(&askance, &["branch", "-D", "doomed"]);
            }
            // And a name in that repository that is already somebody's work.
            _ => {
                companion_mode(&app, id, companion, CompanionMode::ReadWrite).await;
                assert_eq!(
                    companion_branch(&app, id, companion, "alongside").await,
                    CompanionBranchRenamed::Renamed
                );
                git(&askance, &["branch", "alongside"]);
            }
        }

        assert_eq!(
            grill(&app, id).await,
            GrillingStarted::Companion {
                repo: "askance".to_owned(),
                why,
            }
        );

        let view = opened(&app, id).await;
        assert_eq!(view.state, Lifecycle::Draft, "{why:?}");
        assert_eq!(view.worktree, None, "{why:?}");
        assert_eq!(view.companions[0].worktree, None, "{why:?}");
        assert!(!has_branch(&repo, &view.branch), "{why:?}");
        assert_eq!(worktrees(&repo).len(), 1, "only the repository itself");
        assert_eq!(worktrees(&askance).len(), 1, "and only the companion");
    }
}

/// A start refused over the *last* companion leaves nothing behind either — not
/// the checkouts already made, and not the branches they were cut on.
///
/// Which is the case asking every question first cannot cover: this one gets
/// past the asking, because what git refuses is the making. `feature/x` is a
/// name no branch answers to and git will still not take, `feature` being a ref
/// in the way of the directory it would need.
#[tokio::test]
async fn a_start_refused_over_a_companion_unmakes_the_checkouts_it_had_made() {
    let (watched, dir, app, repo, repo_id) = workbench().await;
    let first = second_repo(&app, watched.path(), "askance").await;
    let last = second_repo(&app, watched.path(), "granit").await;
    let id = ready(&app, watched.path(), repo_id).await;

    for companion in [first, last] {
        add_companion(&app, id, companion).await;
        companion_mode(&app, id, companion, CompanionMode::ReadWrite).await;
    }

    let askance = watched.path().join("askance");
    let granit = watched.path().join("granit");

    git(&granit, &["branch", "feature"]);
    assert_eq!(
        companion_branch(&app, id, last, "feature/x").await,
        CompanionBranchRenamed::Renamed
    );

    let branch = opened(&app, id).await.branch;

    assert_eq!(
        grill(&app, id).await,
        GrillingStarted::Companion {
            repo: "granit".to_owned(),
            why: CompanionRefusal::WorktreeRefused,
        }
    );

    // The Conversation is where it was, and so is every repository it touched.
    let view = opened(&app, id).await;
    assert_eq!(view.state, Lifecycle::Draft);
    assert_eq!(view.worktree, None);
    assert!(view.companions.iter().all(|one| one.worktree.is_none()));

    for (name, path) in [("verkstead", &repo), ("askance", &askance)] {
        assert!(
            !has_branch(path, &branch),
            "{name} should have no branch from a start that refused"
        );
        assert_eq!(
            worktrees(path).len(),
            1,
            "{name} should hold only the repository itself"
        );
    }

    // Down to the directories themselves: what was made and then taken back
    // leaves the data directory exactly as empty as it started.
    let made: Vec<_> = std::fs::read_dir(dir.path().join("worktrees"))
        .map(|entries| entries.map(|entry| entry.unwrap().path()).collect())
        .unwrap_or_default();

    assert!(
        made.is_empty(),
        "no directory should be left behind: {made:?}"
    );
}

/// Clicking Steer stops the drive, and cancelling leaves it stopped.
///
/// The click is a press of its own rather than the first half of the submit:
/// nothing new launches while the human composes, so the world the modal was
/// drawn against is the world the submit arrives in. Cancel is then no press at
/// all — the Conversation stays where the click left it, with Resume drawn on it,
/// which is accepted rather than a bug.
///
/// Nothing is running in these fixtures, so the click stops the run where it
/// stands and says as much: what **Interrupt current task** is offered against is
/// a session, and there is none.
#[tokio::test]
async fn clicking_steer_stops_the_drive_and_leaves_it_stopped_when_nothing_follows() {
    let (watched, _dir, app, _repo, repo_id) = workbench().await;
    let id = ready(&app, watched.path(), repo_id).await;
    assert_eq!(grill(&app, id).await, GrillingStarted::Started);

    assert_eq!(
        steer(&app, id).await,
        SteerOpened::Opened { working: false },
        "the modal opens with nothing to interrupt behind it",
    );

    let view = opened(&app, id).await;

    assert_eq!(view.state, Lifecycle::Grilling, "the click moves nothing");
    assert!(
        view.blocked_on.is_some(),
        "the drive has stopped, and the Notice saying so is what the badge points at",
    );
    assert!(
        view.ready_to_resume,
        "so the one press that undoes a click nobody followed up is drawn on it",
    );
    assert!(
        !view.ready_to_stop,
        "and there is nothing left to stop: the click already did",
    );
    assert_eq!(
        steered(&view),
        [("moved", Lifecycle::Grilling)],
        "and nothing was steered, so nothing on the record says it was",
    );
}

/// Submitting into Done: the Conversation moves, the human's own line stands
/// beside the machine's move, and the stop the click wrote is gone.
///
/// Nothing runs in Done, so nothing is started and no Pairing is settled — a
/// steer into Done is the move alone. Which is also why the stop has to go: a
/// Conversation Verkstead has finished with cannot be resumed, so a badge left
/// on one would be a badge with no press to answer it.
#[tokio::test]
async fn steering_into_done_moves_it_and_starts_nothing() {
    let (watched, _dir, app, _repo, repo_id) = workbench().await;
    let id = ready(&app, watched.path(), repo_id).await;
    assert_eq!(grill(&app, id).await, GrillingStarted::Started);

    assert_eq!(
        steer(&app, id).await,
        SteerOpened::Opened { working: false }
    );
    assert_eq!(
        steer_into(&app, id, "Done", false).await,
        ConversationSteered::Steered,
    );

    let view = opened(&app, id).await;

    assert_eq!(view.state, Lifecycle::Done);
    assert_eq!(
        steered(&view),
        [
            ("moved", Lifecycle::Grilling),
            ("steer", Lifecycle::Done),
            ("moved", Lifecycle::Done),
        ],
        "the human's own Event carrying the target, and the plain move under it",
    );
    assert_eq!(
        view.blocked_on, None,
        "the stop the click wrote is gone: nothing is waiting on the human here",
    );
    assert!(
        !view.ready_to_resume && !view.ready_to_stop,
        "and there is nothing to drive in Done, so neither press is offered",
    );
    assert!(
        !unseen(&app, id).await,
        "and no news mark: this Done is the human's own act, so there is nothing \
         to tell them about",
    );
}

/// Every state is a source, which is the one thing that makes a steer different
/// from every other move: a draft nothing has ever run in is somewhere to steer
/// from as much as a run in flight.
///
/// The click finds nothing to stop there and opens the modal anyway. Nothing was
/// driving a draft, so there is no drive to stop and nothing about that is a
/// refusal.
#[tokio::test]
async fn a_draft_is_somewhere_to_steer_from_too() {
    let (watched, _dir, app, _repo, repo_id) = workbench().await;
    let id = ready(&app, watched.path(), repo_id).await;

    assert_eq!(
        steer(&app, id).await,
        SteerOpened::Opened { working: false }
    );

    let view = opened(&app, id).await;
    assert_eq!(view.state, Lifecycle::Draft);
    assert_eq!(
        view.blocked_on, None,
        "there was no drive to stop, so nothing was written down as stopped",
    );

    assert_eq!(
        steer_into(&app, id, "Done", false).await,
        ConversationSteered::Steered,
    );

    let view = opened(&app, id).await;
    assert_eq!(view.state, Lifecycle::Done);
    assert_eq!(
        steered(&view),
        [("steer", Lifecycle::Done), ("moved", Lifecycle::Done)],
    );
}

/// A Draft steered into Grilling gets everything a pressed *Start grilling*
/// would have given it: the branch cut off the base, the worktree checked out on
/// it, and the commit it came off recorded.
///
/// The source with the least behind it, and the reason recreating is part of a
/// steer at all. What the human fixed while drafting is a *branch*, and this is
/// the moment it resolves to a commit — the same rule
/// [`starting_a_grilling_makes_the_branch_and_the_worktree`] asks of the button.
///
/// And the round it opens is opened with the brief they typed in the modal: a
/// second Brief beside the draft's own, frozen where it lands, because the round
/// it belongs to has no Draft to leave.
#[tokio::test]
async fn steering_a_draft_into_grilling_makes_its_branch_and_worktree() {
    let (watched, dir, app, repo, repo_id) = workbench().await;
    let id = ready(&app, watched.path(), repo_id).await;

    let tip = git(&repo, &["rev-parse", "HEAD"]).trim().to_owned();

    assert_eq!(
        steer(&app, id).await,
        SteerOpened::Opened { working: false }
    );
    assert_eq!(
        steer_grilling(&app, id, Some("# Retries\n\nThe backoff is wrong.\n")).await,
        ConversationSteered::Steered,
    );

    let view = opened(&app, id).await;

    assert_eq!(view.state, Lifecycle::Grilling);
    assert_eq!(
        steered(&view),
        [
            ("steer", Lifecycle::Grilling),
            ("moved", Lifecycle::Grilling)
        ],
    );
    assert_eq!(
        view.base_commit.as_deref(),
        Some(tip.as_str()),
        "the base the human left on the default branch, resolved at the steer",
    );

    // The branch is in the Repo's own git directory, cut where the base said.
    assert_eq!(
        git(
            &repo,
            &["rev-parse", &format!("refs/heads/{}", view.branch)]
        )
        .trim(),
        tip,
    );

    let worktree = view
        .worktree
        .as_ref()
        .expect("the steer checked one out and recorded it");
    let path = PathBuf::from(&worktree.path);

    assert!(!worktree.missing);
    assert_eq!(path.parent(), Some(dir.path().join("worktrees").as_path()));
    assert!(path.join("README.md").is_file());
    assert_eq!(
        git(&path, &["symbolic-ref", "--short", "HEAD"]).trim(),
        view.branch,
    );
    assert!(
        worktrees(&repo).contains(&path.canonicalize().unwrap()),
        "git knows about it, which is what makes it a worktree: {:?}",
        worktrees(&repo),
    );

    // And the round's own Brief, beside the one the draft was written with
    // rather than over the top of it.
    let briefs = briefs(&view);

    assert_eq!(briefs.len(), 2);
    assert_eq!(briefs[1].markdown, "# Retries\n\nThe backoff is wrong.\n");
    assert!(
        briefs.iter().all(|brief| brief.frozen),
        "both of them: the round this opened is past drafting from the moment \
         it opened",
    );
}

/// And a steer without one leaves the Steer Event alone: the round starts on the
/// Brief that is already there.
#[tokio::test]
async fn steering_into_grilling_without_a_brief_writes_none() {
    let (watched, _dir, app, _repo, repo_id) = workbench().await;
    let id = ready(&app, watched.path(), repo_id).await;

    assert_eq!(
        steer(&app, id).await,
        SteerOpened::Opened { working: false }
    );
    assert_eq!(
        steer_grilling(&app, id, None).await,
        ConversationSteered::Steered,
    );

    let view = opened(&app, id).await;

    assert_eq!(view.state, Lifecycle::Grilling);
    assert_eq!(
        briefs(&view)
            .iter()
            .map(|brief| brief.markdown.as_str())
            .collect::<Vec<_>>(),
        [brief(&view).markdown.as_str()],
        "the one it was drafted with, and nothing written over it",
    );
}

/// And a steer into Grilling with nothing written on either side is refused by
/// name: a grilling starts from a Brief, and there is no Brief here.
///
/// The rule a pressed *Start grilling* is refused by — see
/// [`starting_is_refused_when_the_brief_is_empty`] — asked of the other way in.
/// It has to
/// be asked at the steer rather than left to the session, because the Brief a
/// steered round lands with is frozen where it lands: a round opened on an empty
/// one is an interview about nothing that nothing can go back and write into.
///
/// A Draft is where this happens, every Conversation being created with a Brief
/// nobody has written yet. Everything past drafting was grilled out of one
/// somebody wrote.
#[tokio::test]
async fn steering_into_grilling_with_no_brief_anywhere_is_refused_by_name() {
    let (watched, _dir, app, _repo, repo_id) = workbench().await;
    let id = started(&app, repo_id).await;

    let grilling = profile(&app, watched.path(), "fable").await;
    let implementation = profile(&app, watched.path(), "opus").await;
    choose(&app, id, "grilling", grilling).await;
    choose(&app, id, "implementation", implementation).await;

    assert_eq!(
        steer(&app, id).await,
        SteerOpened::Opened { working: false }
    );
    assert_eq!(
        steer_grilling(&app, id, None).await,
        ConversationSteered::EmptyBrief,
    );

    let view = opened(&app, id).await;

    assert_eq!(view.state, Lifecycle::Draft, "it is where it was");
    assert_eq!(
        steered(&view),
        [],
        "and nothing on the record says a steer happened",
    );

    // The same steer with the round's Brief written in the modal, which is what
    // that field is for on a Conversation holding none.
    assert_eq!(
        steer_grilling(&app, id, Some("# Retries\n\nThe backoff is wrong.\n")).await,
        ConversationSteered::Steered,
    );

    let view = opened(&app, id).await;

    assert_eq!(view.state, Lifecycle::Grilling);
    assert_eq!(
        briefs(&view)
            .iter()
            .map(|brief| brief.markdown.as_str())
            .collect::<Vec<_>>(),
        ["", "# Retries\n\nThe backoff is wrong.\n"],
        "beside the empty one the draft was created with rather than over it",
    );
}

/// The Pairing picked in the modal for a steer into Grilling is the *grilling*
/// one, and it is recorded as the Conversation's own.
///
/// Which of the two follows the target, an interview running under the one and
/// everything that builds running under the other — and the role not steered
/// into is nobody's to re-settle here.
#[tokio::test]
async fn steering_into_grilling_settles_the_grilling_pairing() {
    let (watched, _dir, app, _repo, repo_id) = workbench().await;
    let id = ready(&app, watched.path(), repo_id).await;

    let picked = profile(&app, watched.path(), "steering").await;
    let building = opened(&app, id)
        .await
        .implementation_pairing
        .expect("the fixture picks one per role");

    assert_eq!(
        steer(&app, id).await,
        SteerOpened::Opened { working: false }
    );

    let steered: ConversationSteered = post(
        &app,
        &format!("/api/ui/conversations/{id}/steer/submit"),
        &serde_json::json!({
            "target": "Grilling",
            "interrupt": false,
            "pairing": { "profile_id": picked, "model": "claude-opus-5" },
        }),
    )
    .await;

    assert_eq!(steered, ConversationSteered::Steered);

    let view = opened(&app, id).await;
    let interviewing = view
        .grilling_pairing
        .expect("the steer settled the role it was steered into");

    assert_eq!(interviewing.profile.id, picked);
    assert_eq!(
        interviewing.model.as_deref(),
        Some("claude-opus-5"),
        "both halves of it: either alone is not something to launch a session with",
    );
    assert_eq!(
        view.implementation_pairing
            .map(|pairing| pairing.profile.id),
        Some(building.profile.id),
        "and the other is exactly where it was",
    );
}

/// A steer into Implementing either carries on what the branch holds or does
/// what the human wrote, so a submit with neither is refused by name.
///
/// What stands is a backlog with work left in it or a roadmap the branch has
/// written, and a Conversation still being grilled has neither: the session
/// that would write one is the session the click just stopped. So the modal
/// requires the instruction there — [`ConversationView::ready_to_continue`] is
/// what it reads that off — and the submit says the same thing again, this
/// being the press that could have been made against a page read a moment
/// earlier.
///
/// Nothing moves on a refusal. The refusals are asked before anything is ended,
/// rebuilt or cleared, so a Conversation refused here is exactly the one the
/// click left: stopped, where it stood.
#[tokio::test]
async fn steering_into_implementing_with_nothing_to_do_is_refused_by_name() {
    let (watched, _dir, app, _repo, repo_id) = workbench().await;
    let id = ready(&app, watched.path(), repo_id).await;
    assert_eq!(grill(&app, id).await, GrillingStarted::Started);

    assert!(
        !opened(&app, id).await.ready_to_continue,
        "there is no backlog and no roadmap on the branch, so the modal has \
         nothing to offer carrying on",
    );

    assert_eq!(
        steer(&app, id).await,
        SteerOpened::Opened { working: false }
    );
    assert_eq!(
        steer_into(&app, id, "Implementing", false).await,
        ConversationSteered::NoInstruction,
    );
    assert_eq!(
        steer_instructed(&app, id, "   \n").await,
        ConversationSteered::NoInstruction,
        "and a textarea somebody tabbed through is nothing written: whitespace \
         alone is not an instruction",
    );

    let view = opened(&app, id).await;

    assert_eq!(view.state, Lifecycle::Grilling, "so nothing moved");
    assert_eq!(
        steered(&view),
        [("moved", Lifecycle::Grilling)],
        "and nothing on the record says it was steered",
    );
    assert!(
        view.blocked_on.is_some() && view.ready_to_resume,
        "the Conversation is where the click left it: stopped, with Resume on \
         offer",
    );
}

/// A steer into Implementing with something written puts the instruction on the
/// Steer Event and says how the work is built.
///
/// **The instruction is the Event's own body**, rendered like every other
/// document the human writes: what the session was sent off to do is read back
/// off the Timeline, above whatever it went on to print.
///
/// **And the direction is recorded as inline**, because there was none. An
/// instruction session is the whole of the work in one session, which is what
/// inline means — and a Conversation implementing with nothing saying how its
/// work is built is one a pressed Resume refuses on by name, so a steer that
/// left it unsaid would be a Conversation nobody could start again.
#[tokio::test]
async fn steering_into_implementing_with_an_instruction_records_what_was_asked_for() {
    let (watched, _dir, app, _repo, repo_id) = workbench().await;
    let id = ready(&app, watched.path(), repo_id).await;
    assert_eq!(grill(&app, id).await, GrillingStarted::Started);

    assert_eq!(
        steer(&app, id).await,
        SteerOpened::Opened { working: false }
    );
    assert_eq!(
        steer_instructed(&app, id, "Rebase this onto `main`.\n").await,
        ConversationSteered::Steered,
    );

    let view = opened(&app, id).await;

    assert_eq!(view.state, Lifecycle::Implementing);
    assert_eq!(
        steered(&view),
        [
            ("moved", Lifecycle::Grilling),
            ("steer", Lifecycle::Implementing),
            ("moved", Lifecycle::Implementing),
        ],
    );

    let instruction = view
        .timeline
        .iter()
        .find_map(|event| match event {
            TimelineEvent::Steer(steer) => steer.html.clone(),
            _ => None,
        })
        .expect("the steer carries what was written on it");

    assert!(
        instruction.contains("Rebase this onto <code>main</code>."),
        "rendered like every other document the human writes: {instruction:?}",
    );

    assert_eq!(
        view.direction,
        Some(verkstead_schema::Direction::Inline),
        "an instruction session is the whole of the work in one session, and a \
         Conversation that had never said how its work is built has now said",
    );
}

/// A closed Conversation is a source like any other: its Worktree was deleted
/// and its branch kept, so a steer checks the branch out again into one.
///
/// The branch is what carries the work, and it is the half closing leaves
/// standing — so nothing is cut afresh here and nothing is started over. What
/// the steer makes is the directory, at the path a first grilling would have
/// chosen, on the branch that is already there.
#[tokio::test]
async fn steering_a_closed_conversation_into_grilling_gives_it_a_worktree_back() {
    let (watched, _dir, app, repo, repo_id) = workbench().await;
    let id = ready(&app, watched.path(), repo_id).await;
    assert_eq!(grill(&app, id).await, GrillingStarted::Started);

    let view = opened(&app, id).await;
    let branch = view.branch.clone();
    let base = view.base_commit.clone();
    let worked_in = PathBuf::from(view.worktree.unwrap().path);

    assert_eq!(close(&app, id).await, ConversationClosed::Closed);
    assert!(!worked_in.exists(), "closing took the directory away");
    assert!(opened(&app, id).await.worktree.is_none());

    assert_eq!(
        steer(&app, id).await,
        SteerOpened::Opened { working: false }
    );
    assert_eq!(
        steer_grilling(&app, id, Some("# Rate limiting, per account\n")).await,
        ConversationSteered::Steered,
    );

    let view = opened(&app, id).await;

    assert_eq!(view.state, Lifecycle::Grilling);
    assert_eq!(view.branch, branch, "on the branch closing kept");

    // And a round of its own, which is the whole of what a closed Conversation
    // is steered back in for: the Brief the first round was built from stays on
    // the record, and the one written in the modal is frozen where it landed.
    let briefs = briefs(&view);
    assert_eq!(
        briefs.len(),
        2,
        "the frozen one, and the round starting here"
    );
    assert_eq!(briefs[0].markdown, "# Rate limiting\n\nThe API has none.\n");
    assert_eq!(briefs[1].markdown, "# Rate limiting, per account\n");
    assert!(
        briefs.iter().all(|brief| brief.frozen),
        "the round it opens is past drafting, so neither is being written",
    );
    assert_eq!(
        view.base_commit, base,
        "and what it branched from is what it always branched from: nothing was \
         cut here to resolve again",
    );

    let worktree = view.worktree.expect("the steer made one");

    assert!(!worktree.missing);
    assert_eq!(
        git(
            Path::new(&worktree.path),
            &["symbolic-ref", "--short", "HEAD"]
        )
        .trim(),
        branch,
    );
    assert!(
        worktrees(&repo).contains(&PathBuf::from(&worktree.path).canonicalize().unwrap()),
        "git knows about it: {:?}",
        worktrees(&repo),
    );
}

/// A Conversation Verkstead has finished with is steered back in the same way a
/// closed one is: into Grilling, which opens a second round.
///
/// The one door into work that is over. Done keeps its Worktree — only closing
/// takes one away — so nothing is checked out here, and what the steer leaves is
/// the round: the human's own line saying they moved it, the move under that,
/// and the new Brief under the move.
#[tokio::test]
async fn a_finished_conversation_steered_into_grilling_opens_a_second_round() {
    let (watched, _dir, app, repo, repo_id) = workbench().await;
    let id = ready(&app, watched.path(), repo_id).await;
    assert_eq!(grill(&app, id).await, GrillingStarted::Started);

    assert_eq!(
        steer(&app, id).await,
        SteerOpened::Opened { working: false }
    );
    assert_eq!(
        steer_into(&app, id, "Done", false).await,
        ConversationSteered::Steered,
    );

    let worked_in = opened(&app, id)
        .await
        .worktree
        .expect("Done keeps one")
        .path;

    assert_eq!(
        steer(&app, id).await,
        SteerOpened::Opened { working: false }
    );
    assert_eq!(
        steer_grilling(&app, id, Some("# Rate limiting, per account\n")).await,
        ConversationSteered::Steered,
    );

    let view = opened(&app, id).await;

    assert_eq!(view.state, Lifecycle::Grilling);
    assert_eq!(
        view.worktree.as_ref().map(|worktree| worktree.path.clone()),
        Some(worked_in),
        "the round before it worked here, and so does this one",
    );
    assert_eq!(worktrees(&repo).len(), 2, "the repository and one worktree");

    let briefs = briefs(&view);
    assert_eq!(
        briefs.len(),
        2,
        "the frozen one, and the round starting here"
    );
    assert_eq!(briefs[1].markdown, "# Rate limiting, per account\n");
    assert!(briefs.iter().all(|brief| brief.frozen));

    // The human's own line, and the move that came of it under it — which is
    // where the round boundary falls, and what a reader tells the two rounds
    // apart by.
    assert_eq!(
        steered(&view),
        [
            ("moved", Lifecycle::Grilling),
            ("steer", Lifecycle::Done),
            ("moved", Lifecycle::Done),
            ("steer", Lifecycle::Grilling),
            ("moved", Lifecycle::Grilling),
        ],
    );
}

/// A steer into Follow-up moves a Conversation Verkstead has finished with, and
/// keeps the brief the human wrote as the Steer Event's own body.
///
/// The one state with no other way in. What it is *for* is the work being on a
/// pull request and there being something more to say about it — so the record
/// it turns on is that pull request, and what it starts is whatever the human
/// wrote.
///
/// **The brief is the Event**, rendered like every other document they write,
/// which is what makes reading the Timeline back reading what the follow-up was
/// opened about. Not a Brief of the Conversation's: a Brief is what a round is
/// grilled about, and this is one session's whole job.
#[tokio::test]
async fn steering_a_finished_conversation_into_follow_up_records_the_brief() {
    let (watched, dir, app, _repo, repo_id) = workbench().await;
    let id = grilling(&app, watched.path(), repo_id).await;
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    assert_eq!(
        store::record_pull_request(
            &pool,
            id,
            repo_id,
            &store::PullRequest {
                number: 41,
                title: "Rate limiting".to_owned(),
                url: "https://github.com/tobico/verkstead/pull/41".to_owned(),
                repo: None,
            },
        )
        .await
        .unwrap(),
        store::Wrapping::Started,
    );

    pool.close().await;

    // Finished with, which is where a follow-up is steered from in the ordinary
    // case: the wrap-up settled, the human read the pull request, and there is
    // one more thing to ask about it.
    assert_eq!(
        steer(&app, id).await,
        SteerOpened::Opened { working: false }
    );
    assert_eq!(
        steer_into(&app, id, "Done", false).await,
        ConversationSteered::Steered,
    );

    assert_eq!(
        steer(&app, id).await,
        SteerOpened::Opened { working: false }
    );
    assert_eq!(
        steer_following_up(&app, id, Some("Does it count the `429`s it sends?\n")).await,
        ConversationSteered::Steered,
    );

    let view = opened(&app, id).await;

    assert_eq!(view.state, Lifecycle::FollowUp);
    assert_eq!(
        steered(&view),
        [
            ("moved", Lifecycle::Grilling),
            ("moved", Lifecycle::Wrapping),
            ("steer", Lifecycle::Done),
            ("moved", Lifecycle::Done),
            ("steer", Lifecycle::FollowUp),
            ("moved", Lifecycle::FollowUp),
        ],
        "the human's own line, and the plain move under it",
    );

    let brief = view
        .timeline
        .iter()
        .rev()
        .find_map(|event| match event {
            TimelineEvent::Steer(steer) => steer.html.clone(),
            _ => None,
        })
        .expect("the steer carries what was written on it");

    assert!(
        brief.contains("Does it count the <code>429</code>s it sends?"),
        "rendered like every other document the human writes: {brief:?}",
    );
    assert!(
        !view.timeline.iter().any(
            |event| matches!(event, TimelineEvent::Brief(brief) if brief.markdown.contains("429"))
        ),
        "and it is the steer's own body rather than a Brief of the \
         Conversation's: what a round is grilled about has not changed",
    );
    assert_eq!(
        view.blocked_on, None,
        "and the stop the click wrote is gone",
    );
}

/// A follow-up is whatever the human wrote it about, so a submit with nothing
/// written is refused by name — and so is one on work nobody can see.
///
/// The one written payload with no quiet meaning. An empty instruction carries
/// the branch on and an empty brief grills the one already written; a follow-up
/// has nothing of its own to fall back on, being a thing the human wanted rather
/// than a step of the run. And the pull request is the same rule Wrapping is
/// refused by, asked of the target that turns on the same fact.
#[tokio::test]
async fn steering_into_follow_up_with_nothing_to_follow_up_is_refused_by_name() {
    let (watched, dir, app, _repo, repo_id) = workbench().await;
    let id = grilling(&app, watched.path(), repo_id).await;

    assert_eq!(
        steer(&app, id).await,
        SteerOpened::Opened { working: false }
    );
    assert_eq!(
        steer_following_up(&app, id, Some("Does it count the 429s?\n")).await,
        ConversationSteered::NoPullRequest,
        "there is nothing pushed to follow up on",
    );

    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    store::record_pull_request(
        &pool,
        id,
        repo_id,
        &store::PullRequest {
            number: 41,
            title: "Rate limiting".to_owned(),
            url: "https://github.com/tobico/verkstead/pull/41".to_owned(),
            repo: None,
        },
    )
    .await
    .unwrap();

    pool.close().await;

    assert_eq!(
        steer_following_up(&app, id, None).await,
        ConversationSteered::NoFollowUpBrief,
        "and a pull request with nothing said about it is a session with \
         nothing to do",
    );
    assert_eq!(
        steer_following_up(&app, id, Some("   \n")).await,
        ConversationSteered::NoFollowUpBrief,
        "a textarea somebody tabbed through included",
    );

    let view = opened(&app, id).await;

    assert_eq!(view.state, Lifecycle::Wrapping, "so nothing moved");
    assert_eq!(
        steered(&view),
        [
            ("moved", Lifecycle::Grilling),
            ("moved", Lifecycle::Wrapping),
        ],
        "and nothing on the record says it was steered",
    );
}

/// Wrapping up is a move onto a pull request that is already there, so a submit
/// naming it on work that is on none is refused by name.
///
/// A wrapping Conversation is defined by the pull request under it — the record
/// writes the move and the pull-request row as one act — so there would be
/// nothing to wrap up here. The modal does not offer the target on such a
/// Conversation; this is that same rule asked again on arrival, the way every
/// named refusal here is.
///
/// And the refusal comes before anything is done: the stop the click wrote is
/// still there, and the Conversation is still grilling.
#[tokio::test]
async fn steering_into_wrapping_without_a_pull_request_is_refused_by_name() {
    let (watched, _dir, app, _repo, repo_id) = workbench().await;
    let id = ready(&app, watched.path(), repo_id).await;
    assert_eq!(grill(&app, id).await, GrillingStarted::Started);

    assert_eq!(
        steer(&app, id).await,
        SteerOpened::Opened { working: false }
    );
    assert_eq!(
        steer_into(&app, id, "Wrapping", false).await,
        ConversationSteered::NoPullRequest,
    );

    let view = opened(&app, id).await;

    assert_eq!(view.state, Lifecycle::Grilling, "nothing moved");
    assert_eq!(
        steered(&view),
        [("moved", Lifecycle::Grilling)],
        "and nothing on the record says it was steered",
    );
    assert!(
        view.blocked_on.is_some() && view.ready_to_resume,
        "the click's stop is where it was, with the press that undoes it drawn: \
         a refusal leaves the world as the click left it",
    );
}

/// A draft has no pull request to be steered onto either, which is the same
/// refusal read from the other end of the ladder.
///
/// Every state is somewhere to steer *from* — that much is unchanged — and it is
/// the target that is refused rather than the source: nothing has ever run in
/// this Conversation, so there is no branch, no pull request, and nothing to
/// wrap up.
#[tokio::test]
async fn a_draft_has_no_pull_request_to_be_steered_onto() {
    let (watched, _dir, app, _repo, repo_id) = workbench().await;
    let id = ready(&app, watched.path(), repo_id).await;

    assert_eq!(
        steer(&app, id).await,
        SteerOpened::Opened { working: false }
    );
    assert_eq!(
        steer_into(&app, id, "Wrapping", false).await,
        ConversationSteered::NoPullRequest,
    );

    assert_eq!(opened(&app, id).await.state, Lifecycle::Draft);

    // And Done is still there to steer it into, the refusal being about the one
    // target rather than about the Conversation.
    assert_eq!(
        steer_into(&app, id, "Done", false).await,
        ConversationSteered::Steered,
    );
}

/// Both presses answer for a Conversation that is not there, and for an id that
/// could never name one — the id comes out of a URL the human may have typed.
#[tokio::test]
async fn steering_a_conversation_that_is_not_there_says_so() {
    let (_watched, _dir, app, _repo, _repo_id) = workbench().await;

    assert_eq!(steer(&app, 404).await, SteerOpened::NoSuchConversation);
    assert_eq!(
        steer_into(&app, 404, "Done", false).await,
        ConversationSteered::NoSuchConversation,
    );

    let refused: SteerOpened = post(
        &app,
        "/api/ui/conversations/nonsense/steer",
        &serde_json::json!({}),
    )
    .await;
    assert_eq!(refused, SteerOpened::NoSuchConversation);

    let refused: ConversationSteered = post(
        &app,
        "/api/ui/conversations/nonsense/steer/submit",
        &serde_json::json!({ "target": "Done", "interrupt": false }),
    )
    .await;
    assert_eq!(refused, ConversationSteered::NoSuchConversation);
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

/// Closing takes the directory away and leaves the branch, because a branch is
/// cheap and may hold work worth reading.
#[tokio::test]
async fn closing_removes_the_worktree_and_keeps_the_branch() {
    let (watched, _dir, app, repo, repo_id) = workbench().await;
    let id = ready(&app, watched.path(), repo_id).await;
    grill(&app, id).await;

    let view = opened(&app, id).await;
    let branch = view.branch.clone();
    let path = PathBuf::from(view.worktree.unwrap().path);

    assert_eq!(close(&app, id).await, ConversationClosed::Closed);

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
    assert_eq!(view.state, Lifecycle::Closed);
    assert_eq!(view.worktree, None);
    assert_eq!(moves(&view), [Lifecycle::Grilling, Lifecycle::Closed]);
}

/// And every companion's goes the same way, keeping every companion's branch.
///
/// The same bargain the Conversation's own worktree is closed on: a directory is
/// somewhere the work was given to happen and the work has stopped, while a
/// branch is a name and a commit that may hold work worth reading.
#[tokio::test]
async fn closing_removes_every_companion_worktree_and_keeps_their_branches() {
    let (watched, _dir, app, _repo, repo_id) = workbench().await;
    let reading = second_repo(&app, watched.path(), "askance").await;
    let writing = second_repo(&app, watched.path(), "granit").await;
    let id = ready(&app, watched.path(), repo_id).await;

    add_companion(&app, id, reading).await;
    add_companion(&app, id, writing).await;
    companion_mode(&app, id, writing, CompanionMode::ReadWrite).await;

    assert_eq!(grill(&app, id).await, GrillingStarted::Started);

    let view = opened(&app, id).await;
    let branch = view.branch.clone();
    let read = checked_out(&view, "askance");
    let written = checked_out(&view, "granit");

    assert_eq!(close(&app, id).await, ConversationClosed::Closed);

    let askance = watched.path().join("askance");
    let granit = watched.path().join("granit");

    assert!(!read.exists(), "the read-only directory should be gone");
    assert!(!written.exists(), "and so should the read-write one");
    assert_eq!(worktrees(&askance).len(), 1, "git should hold neither");
    assert_eq!(worktrees(&granit).len(), 1);

    assert!(
        has_branch(&granit, &branch),
        "the branch the companion was worked on is what is kept"
    );

    // And the Conversation has none of them any more, which is the same fact the
    // record tells about its own.
    let view = opened(&app, id).await;
    assert_eq!(view.state, Lifecycle::Closed);
    assert!(view.companions.iter().all(|one| one.worktree.is_none()));
}

#[tokio::test]
async fn closing_twice_is_not_an_error() {
    let (watched, _dir, app, _repo, repo_id) = workbench().await;
    let id = ready(&app, watched.path(), repo_id).await;
    grill(&app, id).await;

    assert_eq!(close(&app, id).await, ConversationClosed::Closed);
    assert_eq!(close(&app, id).await, ConversationClosed::AlreadyClosed);

    assert_eq!(
        moves(&opened(&app, id).await),
        [Lifecycle::Grilling, Lifecycle::Closed]
    );
}

/// Closing is reachable from every state this stage can reach, including the
/// one where nothing was ever made.
#[tokio::test]
async fn a_drafting_conversation_can_be_closed() {
    let (_watched, _dir, app, _repo, repo_id) = workbench().await;
    let id = started(&app, repo_id).await;

    assert_eq!(close(&app, id).await, ConversationClosed::Closed);

    let view = opened(&app, id).await;
    assert_eq!(view.state, Lifecycle::Closed);
    assert_eq!(view.worktree, None);
    assert!(!view.ready_to_grill);
}

/// A worktree the human deleted by hand is still a close that works: what was
/// asked for is that the directory be gone, and it is.
#[tokio::test]
async fn closing_a_conversation_whose_worktree_has_already_gone_works() {
    let (watched, _dir, app, repo, repo_id) = workbench().await;
    let id = ready(&app, watched.path(), repo_id).await;
    grill(&app, id).await;

    let path = PathBuf::from(opened(&app, id).await.worktree.unwrap().path);
    std::fs::remove_dir_all(&path).unwrap();

    assert_eq!(close(&app, id).await, ConversationClosed::Closed);
    assert_eq!(opened(&app, id).await.state, Lifecycle::Closed);
    assert_eq!(worktrees(&repo).len(), 1, "git should have let it go too");
}

/// And a worktree git will not let go of is a close that works too. A directory
/// hollowed out — its `.git` file gone — is one git refuses to remove and one
/// the human has every reason to want the end of: the close goes through, and
/// what is left on disk is left for them.
#[tokio::test]
async fn closing_a_conversation_whose_worktree_git_will_not_remove_still_closes() {
    let (watched, _dir, app, _repo, repo_id) = workbench().await;
    let id = ready(&app, watched.path(), repo_id).await;
    grill(&app, id).await;

    let path = PathBuf::from(opened(&app, id).await.worktree.unwrap().path);
    std::fs::remove_file(path.join(".git")).unwrap();

    assert_eq!(close(&app, id).await, ConversationClosed::Closed);

    let view = opened(&app, id).await;
    assert_eq!(view.state, Lifecycle::Closed);
    assert_eq!(view.worktree, None);
    assert!(
        path.exists(),
        "the directory git would not remove should still be there to be found"
    );
}

/// Close and archive is the two rows in one press: the Conversation ends and
/// comes off the sidebar, and the record is the record either press leaves.
#[tokio::test]
async fn closing_and_archiving_in_one_press_does_both() {
    let (watched, _dir, app, repo, repo_id) = workbench().await;
    let id = ready(&app, watched.path(), repo_id).await;
    grill(&app, id).await;

    let path = PathBuf::from(opened(&app, id).await.worktree.unwrap().path);

    assert_eq!(
        close_and_archive(&app, id).await,
        ConversationClosed::Closed
    );

    assert!(!path.exists(), "the worktree directory should be gone");
    assert_eq!(
        worktrees(&repo).len(),
        1,
        "git should hold only the repository"
    );
    assert!(sidebar(&app).await.is_empty());

    let view = opened(&app, id).await;
    assert_eq!(view.state, Lifecycle::Closed);
    assert!(view.archived);
    assert_eq!(view.worktree, None);
    assert_eq!(moves(&view), [Lifecycle::Grilling, Lifecycle::Closed]);
}

/// On one that is closed already it is the archive alone, which is the whole
/// point of saying so rather than refusing: what the human asked for holds.
#[tokio::test]
async fn closing_and_archiving_one_already_closed_puts_it_away() {
    let (_watched, _dir, app, _repo, repo_id) = workbench().await;
    let id = started(&app, repo_id).await;
    close(&app, id).await;

    assert_eq!(
        close_and_archive(&app, id).await,
        ConversationClosed::AlreadyClosed
    );

    assert!(sidebar(&app).await.is_empty());
    assert!(opened(&app, id).await.archived);
}

#[tokio::test]
async fn closing_and_archiving_a_conversation_that_is_not_there_says_so() {
    let (_watched, _dir, app, _repo, _repo_id) = workbench().await;

    assert_eq!(
        close_and_archive(&app, 404).await,
        ConversationClosed::NoSuchConversation
    );
}

#[tokio::test]
async fn closing_a_conversation_that_is_not_there_says_so() {
    let (_watched, _dir, app, _repo, _repo_id) = workbench().await;

    assert_eq!(
        close(&app, 404).await,
        ConversationClosed::NoSuchConversation
    );
}

/// Archiving is what a Closed Conversation is for: it comes off the sidebar,
/// and everything else about it — its state, its Timeline, its branch — is
/// where it was. Nothing leaves a Timeline.
#[tokio::test]
async fn archiving_a_closed_conversation_takes_it_off_the_sidebar() {
    let (watched, _dir, app, _repo, repo_id) = workbench().await;
    let id = ready(&app, watched.path(), repo_id).await;
    close(&app, id).await;

    assert_eq!(archive(&app, id).await, ConversationArchived::Archived);

    assert!(sidebar(&app).await.is_empty());

    let view = opened(&app, id).await;
    assert_eq!(view.state, Lifecycle::Closed);
    assert_eq!(
        brief(&view).markdown,
        "# Rate limiting\n\nThe API has none.\n"
    );
}

/// Archiving twice is not an error — what the human asked for holds either way.
#[tokio::test]
async fn archiving_twice_is_not_an_error() {
    let (_watched, _dir, app, _repo, repo_id) = workbench().await;
    let id = started(&app, repo_id).await;
    close(&app, id).await;

    assert_eq!(archive(&app, id).await, ConversationArchived::Archived);
    assert_eq!(
        archive(&app, id).await,
        ConversationArchived::AlreadyArchived
    );
    assert!(sidebar(&app).await.is_empty());
}

/// A Conversation still being worked on belongs on the list it is being worked
/// from: it is closed first and archived after.
#[tokio::test]
async fn a_conversation_that_is_not_closed_cannot_be_archived() {
    let (watched, _dir, app, _repo, repo_id) = workbench().await;
    let id = ready(&app, watched.path(), repo_id).await;

    assert_eq!(archive(&app, id).await, ConversationArchived::NotClosed);

    grill(&app, id).await;

    assert_eq!(archive(&app, id).await, ConversationArchived::NotClosed);
    assert_eq!(sidebar(&app).await.len(), 1);
}

#[tokio::test]
async fn archiving_a_conversation_that_is_not_there_says_so() {
    let (_watched, _dir, app, _repo, _repo_id) = workbench().await;

    assert_eq!(
        archive(&app, 404).await,
        ConversationArchived::NoSuchConversation
    );
}

/// The toggle is the way to see what has been put away without taking it back:
/// on, the archived Conversations are on the list in their ordinary places; off,
/// they are not drawn at all.
#[tokio::test]
async fn the_toggle_shows_and_hides_what_has_been_archived() {
    let (_watched, _dir, app, _repo, repo_id) = workbench().await;
    let kept = started(&app, repo_id).await;
    let put_away = started(&app, repo_id).await;
    close(&app, put_away).await;
    archive(&app, put_away).await;

    assert!(!showing_archived(&app).await);
    assert_eq!(order(&app).await, vec![kept]);

    show_archived(&app, true).await;

    assert!(showing_archived(&app).await);
    assert_eq!(order(&app).await, vec![put_away, kept]);

    show_archived(&app, false).await;

    assert!(!showing_archived(&app).await);
    assert_eq!(order(&app).await, vec![kept]);
}

/// It is the human's standing choice rather than one device's, so it is read
/// back off the server — which is what a second viewer opening the sidebar is.
#[tokio::test]
async fn the_toggle_is_read_back_off_the_server() {
    let (_watched, _dir, app, _repo, repo_id) = workbench().await;
    let id = started(&app, repo_id).await;
    close(&app, id).await;
    archive(&app, id).await;

    show_archived(&app, true).await;

    // Said twice, because a switch says where it stands rather than asking for
    // a flip: the position asked for is the position it ends in.
    show_archived(&app, true).await;

    assert!(showing_archived(&app).await);
    assert_eq!(order(&app).await, vec![id]);
}

/// Unarchiving is the other way back, and the lasting one: the Conversation is
/// on the list again with the toggle off.
#[tokio::test]
async fn unarchiving_returns_a_conversation_to_the_ordinary_list() {
    let (watched, _dir, app, _repo, repo_id) = workbench().await;
    let id = ready(&app, watched.path(), repo_id).await;
    close(&app, id).await;
    archive(&app, id).await;

    assert!(sidebar(&app).await.is_empty());
    assert!(opened(&app, id).await.archived);

    assert_eq!(
        unarchive(&app, id).await,
        ConversationUnarchived::Unarchived
    );

    assert!(!showing_archived(&app).await);
    assert_eq!(order(&app).await, vec![id]);

    let view = opened(&app, id).await;
    assert!(!view.archived);
    assert_eq!(view.state, Lifecycle::Closed);
}

/// Unarchiving one that was never put away is not an error — what the human
/// asked for holds either way.
#[tokio::test]
async fn unarchiving_one_that_is_not_archived_is_not_an_error() {
    let (_watched, _dir, app, _repo, repo_id) = workbench().await;
    let id = started(&app, repo_id).await;

    assert_eq!(
        unarchive(&app, id).await,
        ConversationUnarchived::NotArchived
    );
    assert_eq!(order(&app).await, vec![id]);
}

#[tokio::test]
async fn unarchiving_a_conversation_that_is_not_there_says_so() {
    let (_watched, _dir, app, _repo, _repo_id) = workbench().await;

    assert_eq!(
        unarchive(&app, 404).await,
        ConversationUnarchived::NoSuchConversation
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
    // refused — Verkstead made that one — so the name is freed by closing the
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
    text: Anything still open before we build it?
    options:
      - n: 1
        text: Nothing from me
        recommended: true
      - n: 2
        text: Yes, see below
proposal:
  direction: task-list
  rationale: |
    Six changes, each independently testable.
"#;

/// The same, recommending a different direction — for the test that picks
/// against the recommendation.
const RECOMMENDING_INLINE: &str = r#"
title: Ready to build the rate limiter
questions:
  - label: Q9
    text: Anything still open before we build it?
    options:
      - n: 1
        text: Nothing from me
        recommended: true
      - n: 2
        text: Yes, see below
proposal:
  direction: inline
  rationale: |
    One change, in one file, with one test.
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
    asking(app, conversation, yaml, "").await
}

/// The same Set asked as a **Deferred Ask**: on the Timeline to be answered like
/// any other, with nobody waiting on the Answer.
async fn defer(app: &Router, conversation: i64, yaml: &str) -> i64 {
    asking(app, conversation, yaml, "?deferred=true").await
}

async fn asking(app: &Router, conversation: i64, yaml: &str, kind: &str) -> i64 {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/conversations/{conversation}/api/v1/sets{kind}"))
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

/// Answer it from the browser, which is the path the human's own reply takes —
/// picking a direction on the chooser, which is the whole of accepting a
/// proposal.
///
/// Inline, which is a direction whose pick hands the work over there and then:
/// the tests about what accepting *moves* are asking about one of those, and a
/// task list is its own case below — the session that proposed writes its
/// backlog, so nothing about the Conversation moves until it has.
async fn answer(app: &Router, set_id: i64) -> verkstead_render::Submitted {
    picking(app, set_id, "inline").await
}

/// The same, with the direction of the test's own choosing.
async fn picking(app: &Router, set_id: i64, direction: &str) -> verkstead_render::Submitted {
    post(
        app,
        &format!("/api/ui/sets/{set_id}/response"),
        &serde_json::json!({
            "answers": [{ "label": "Q9", "selected": 1 }],
            "direction": direction,
        }),
    )
    .await
}

/// Answer an ordinary round of grilling, which has no chooser on it to pick
/// anything with.
async fn answer_ordinary(app: &Router, set_id: i64) -> verkstead_render::Submitted {
    answered(
        app,
        set_id,
        serde_json::json!({ "label": "Q9", "selected": 1 }),
    )
    .await
}

/// And with no pick at all, which is every way of sending a proposal back: the
/// Answer is the test's own, and what makes it a refusal is what is missing.
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

/// Write a handoff where a grilling session would have written one: inside the
/// Conversation's own directory under the Data Directory, which is bound into
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

/// Picking a direction on the closing Set is the whole of accepting it: the
/// direction is settled off the one answer, with no second trip to the Timeline.
///
/// What it does *not* do is move anything. The pick informs the session that
/// proposed, which is still running and still holding the thread; what moves the
/// Conversation is the artifact that session goes on to produce.
#[tokio::test]
async fn picking_a_direction_on_the_closing_set_settles_it() {
    let (watched, _dir, app, _repo, repo_id) = workbench().await;
    let id = grilling(&app, watched.path(), repo_id).await;

    let set = ask(&app, id, PROPOSING).await;
    assert_eq!(
        answer(&app, set).await,
        verkstead_render::Submitted::Accepted
    );

    let view = opened(&app, id).await;

    assert_eq!(
        view.direction,
        Some(verkstead_schema::Direction::Inline),
        "nothing on this page was pressed to get here: the agent proposed and the human picked",
    );
    assert_eq!(
        view.state,
        Lifecycle::Grilling,
        "and the grilling is what is still happening: the pick informs it",
    );
    assert_eq!(
        moves(&view),
        [Lifecycle::Grilling],
        "with no rung in between and none reached: nothing was ever waiting to be chosen",
    );
}

/// The human is not held to the recommendation, and picking against it accepts
/// the proposal exactly as agreeing with it does.
#[tokio::test]
async fn a_pick_the_agent_did_not_recommend_is_the_one_that_runs() {
    let (watched, _dir, app, _repo, repo_id) = workbench().await;
    let id = grilling(&app, watched.path(), repo_id).await;

    picking(&app, ask(&app, id, RECOMMENDING_INLINE).await, "roadmap").await;

    let view = opened(&app, id).await;

    assert_eq!(
        view.direction,
        Some(verkstead_schema::Direction::Roadmap),
        "what the human picked is what the Conversation is being built as, \
         whatever the agent argued for",
    );
    assert_eq!(
        view.state,
        Lifecycle::Grilling,
        "and the grilling session writes what was picked for itself, so the pick \
         records the direction and moves nothing",
    );
}

/// The one row of the sidebar, for the tests about what a row says of itself.
async fn only_row(app: &Router) -> ConversationEntry {
    let sidebar = sidebar(app).await;
    assert_eq!(sidebar.len(), 1, "these tests keep one Conversation");
    sidebar[0].clone()
}

/// Lock a Set the way the human does with one nobody is waiting on.
async fn lock(app: &Router, set_id: i64) -> verkstead_render::Locked {
    post(
        app,
        &format!("/api/ui/sets/{set_id}/lock"),
        &serde_json::json!({}),
    )
    .await
}

/// The sidebar says a Conversation is waiting on the human for as long as there
/// is a Set on its Timeline nobody has settled — and stops the moment one is,
/// whichever way it was settled.
///
/// Nothing here asks how the Set was put: a Blocking Ask and a Deferred Ask are
/// the same row in the same table, and what draws the human is that there is
/// something answerable rather than whether a session is idling on the answer.
#[tokio::test]
async fn a_conversation_with_an_unanswered_set_is_waiting_on_the_human() {
    let (watched, _dir, app, _repo, repo_id) = workbench().await;
    let id = grilling(&app, watched.path(), repo_id).await;

    assert!(!only_row(&app).await.waiting, "nothing has been asked yet");

    let set = ask(&app, id, ORDINARY).await;
    assert!(only_row(&app).await.waiting);

    answer_ordinary(&app, set).await;
    assert!(
        !only_row(&app).await.waiting,
        "an answered Set is a decision taken, not one outstanding",
    );
}

#[tokio::test]
async fn a_set_that_was_locked_unanswered_stops_drawing_the_human_too() {
    let (watched, _dir, app, _repo, repo_id) = workbench().await;
    let id = grilling(&app, watched.path(), repo_id).await;

    let set = ask(&app, id, ORDINARY).await;
    assert!(only_row(&app).await.waiting);

    lock(&app, set).await;
    assert!(!only_row(&app).await.waiting);
}

/// The closing Set is what the human is waiting on, and answering it is the
/// whole of it: there is no second thing to press behind the Set, so nothing is
/// left drawing them once it is answered.
#[tokio::test]
async fn a_closing_set_stops_drawing_the_human_the_moment_it_is_picked_on() {
    let (watched, _dir, app, _repo, repo_id) = workbench().await;
    let id = grilling(&app, watched.path(), repo_id).await;

    let set = ask(&app, id, PROPOSING).await;
    assert!(only_row(&app).await.waiting);

    answer(&app, set).await;

    let row = only_row(&app).await;
    assert_eq!(row.state, Lifecycle::Grilling);
    assert!(
        !row.waiting,
        "the pick settled the direction as it settled the Set",
    );
}

/// A Draft is waiting on the human in the ordinary sense — nobody has written its
/// Brief — and the sidebar says so by drawing it as a draft rather than by
/// marking it as an ask. So the flag stays off, whatever else is true of it.
#[tokio::test]
async fn a_draft_is_never_marked_as_waiting() {
    let (_watched, _dir, app, _repo, repo_id) = workbench().await;
    let id = started(&app, repo_id).await;

    let set = ask(&app, id, ORDINARY).await;
    let row = only_row(&app).await;

    assert_eq!(row.state, Lifecycle::Draft);
    assert!(!row.waiting);

    // And the Set is genuinely unanswered: what is being read here is the draft
    // rule and not an empty Timeline.
    assert_eq!(
        answer_ordinary(&app, set).await,
        verkstead_render::Submitted::Accepted
    );
}

/// How each Question Set on a Conversation's Timeline stands, in the order it
/// was asked.
fn standings(view: &ConversationView) -> Vec<&Standing> {
    view.timeline
        .iter()
        .filter_map(|event| match event {
            TimelineEvent::QuestionSet(asked) => Some(&asked.standing),
            _ => None,
        })
        .collect()
}

/// Closing shuts whatever the Conversation was still asking. The sessions that
/// asked are gone for good and no other is coming, so a Set left open would be
/// one the human could write an Answer into that nothing would ever read.
///
/// Every kind of ask, which is where this differs from a grilling being
/// relaunched: that leaves a Deferred Ask standing for the session after it, and
/// closing has no session after it to leave one for.
#[tokio::test]
async fn closing_locks_every_set_it_finds_open() {
    let (watched, _dir, app, _repo, repo_id) = workbench().await;
    let id = grilling(&app, watched.path(), repo_id).await;

    let answered = ask(&app, id, ORDINARY).await;
    answer_ordinary(&app, answered).await;
    ask(&app, id, ORDINARY).await;
    defer(&app, id, ORDINARY).await;

    assert_eq!(close(&app, id).await, ConversationClosed::Closed);

    let view = opened(&app, id).await;
    let standings = standings(&view);

    assert!(
        matches!(standings[0], Standing::Answered(_)),
        "what the human decided is left exactly as they decided it: {:?}",
        standings[0],
    );
    assert!(
        matches!(standings[1], Standing::LockedUnanswered(_)),
        "the blocking Ask nobody answered is closed unanswered: {:?}",
        standings[1],
    );
    assert!(
        matches!(standings[2], Standing::LockedUnanswered(_)),
        "and so is the Deferred one, there being no session left to fold an \
         Answer into: {:?}",
        standings[2],
    );
    assert!(
        !only_row(&app).await.waiting,
        "so nothing on the Conversation is left drawing the human",
    );
}

/// And a closed Conversation carries neither waiting mark, whatever stopped it
/// on the way.
///
/// Closing is the human saying the work is over wherever it had got to, so the
/// stop stops being something to come back to: the marks mean *there is
/// something here for you*, and there is not. The stop itself is untouched —
/// it is what happened, and the Notice explaining it is still on the Timeline.
#[tokio::test]
async fn a_closed_conversation_carries_neither_waiting_mark() {
    let (watched, _dir, app, _repo, repo_id) = workbench().await;
    let id = grilling(&app, watched.path(), repo_id).await;

    // The click is the shortest way to a stop written down: it stops the drive
    // and opens the modal, and nothing here submits one.
    assert_eq!(
        steer(&app, id).await,
        SteerOpened::Opened { working: false }
    );
    assert!(
        opened(&app, id).await.blocked_on.is_some(),
        "the drive has stopped, and the header says so until it is closed",
    );

    assert_eq!(close(&app, id).await, ConversationClosed::Closed);

    let view = opened(&app, id).await;

    assert_eq!(view.state, Lifecycle::Closed);
    assert_eq!(view.blocked_on, None, "so there is no header mark to press");
    assert!(!view.stopped_by_hand, "of either kind");
    assert!(!only_row(&app).await.waiting, "and no disc beside the row");
    assert!(
        view.timeline
            .iter()
            .any(|event| matches!(event, TimelineEvent::Notice(_))),
        "with the Notice the stop wrote still on the record: closing reads the \
         stop and writes nothing over it",
    );
}

/// And the news mark goes with them, which is the third thing a row can draw the
/// human with.
///
/// The case it is really for: a wrap-up carries the work to Done and stamps the
/// Conversation unseen, and the human closes it from the sidebar without ever
/// opening it — so the press that takes the mark off is one they never made. A
/// disc on the Conversation they have just put away is exactly the disc that
/// teaches them to stop reading the discs.
#[tokio::test]
async fn closing_takes_the_news_off_the_row_the_human_never_opened() {
    let (_watched, dir, app, _repo, repo_id) = workbench().await;
    let id = started(&app, repo_id).await;
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    store::stamp_unseen(&pool, id).await.unwrap();

    assert!(
        only_row(&app).await.unseen,
        "Verkstead told them the work was done, and they have not looked",
    );

    assert_eq!(close(&app, id).await, ConversationClosed::Closed);

    let row = only_row(&app).await;

    assert_eq!(row.state, Lifecycle::Closed);
    assert!(
        !row.unseen,
        "and closing it is them being done with it, so there is no news to go back for",
    );
    assert!(!row.waiting, "with neither waiting mark either");

    pool.close().await;
}

/// **Done is not Closed here**, and the difference is what the marks are for: a
/// Done Conversation is one Verkstead has finished with rather than one the
/// human has put away, and its Sets are still there to be answered. An
/// answerable ask is still an ask.
#[tokio::test]
async fn a_done_conversation_with_an_open_set_is_still_waiting() {
    let (watched, _dir, app, _repo, repo_id) = workbench().await;
    let id = grilling(&app, watched.path(), repo_id).await;

    ask(&app, id, ORDINARY).await;

    assert_eq!(
        steer(&app, id).await,
        SteerOpened::Opened { working: false }
    );
    assert_eq!(
        steer_into(&app, id, "Done", false).await,
        ConversationSteered::Steered,
    );

    let row = only_row(&app).await;

    assert_eq!(row.state, Lifecycle::Done);
    assert!(row.waiting, "the Set is open, and nothing has closed it");
    assert!(
        matches!(standings(&opened(&app, id).await)[0], Standing::Waiting(_)),
        "because it is still there to answer",
    );
}

/// A server running no sessions at all — which is every one of these — has none
/// to report. What a running one does to the row is `sessions.rs`'s to say, being
/// the file with an agent in it.
#[tokio::test]
async fn a_conversation_with_no_session_running_is_not_working() {
    let (watched, _dir, app, _repo, repo_id) = workbench().await;
    grilling(&app, watched.path(), repo_id).await;

    assert!(!only_row(&app).await.working);
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
        "and by the pick that decides what becomes of it",
    );
}

/// No answer takes the handoff, whichever way it was answered.
///
/// The handoff is written on the far side of the pick now — an inline session
/// writes it once it knows that is what was picked — so a document sitting there
/// when a Response lands is one from a round that has already been superseded,
/// and nothing about answering is the moment to take it. What takes it is the
/// session ending, which `sessions.rs` is where to look for.
#[tokio::test]
async fn no_answer_takes_the_handoff_the_grilling_wrote() {
    for (how, response) in [
        (
            "picked on",
            serde_json::json!({
                "answers": [{ "label": "Q9", "selected": 1 }],
                "direction": "inline",
            }),
        ),
        (
            "sent back",
            serde_json::json!({ "answers": [{ "label": "Q9", "selected": 2 }] }),
        ),
    ] {
        let (watched, dir, app, _repo, repo_id) = workbench().await;
        let id = grilling(&app, watched.path(), repo_id).await;

        let written = handoff_written(dir.path(), id, "# What we settled\n");
        let set = ask(&app, id, PROPOSING).await;

        assert_eq!(
            post::<verkstead_render::Submitted>(
                &app,
                &format!("/api/ui/sets/{set}/response"),
                &response,
            )
            .await,
            verkstead_render::Submitted::Accepted,
            "a Response {how} is taken either way",
        );

        assert!(written.exists(), "nothing was taken, {how}");
        assert_eq!(handoff(&opened(&app, id).await), None, "{how}");
    }
}

#[tokio::test]
async fn answering_an_ordinary_grilling_set_leaves_the_grilling_running() {
    let (watched, _dir, app, _repo, repo_id) = workbench().await;
    let id = grilling(&app, watched.path(), repo_id).await;

    let set = ask(&app, id, ORDINARY).await;
    assert_eq!(
        answer_ordinary(&app, set).await,
        verkstead_render::Submitted::Accepted
    );

    let view = opened(&app, id).await;

    assert_eq!(view.state, Lifecycle::Grilling);
    assert_eq!(view.direction, None);
    assert_eq!(moves(&view), [Lifecycle::Grilling]);
}

/// Every pick stays where it is. The session that proposed writes the backlog,
/// the roadmap or the handoff itself, so the grilling is still what is happening
/// — and the handoff standing in its directory is still its own, because it has
/// not finished with it.
///
/// One test over the three, because what the pick does is the same for each: it
/// is what the tail is watched for, and nothing else. What ends that session and
/// moves the Conversation is the artifact landing, which wants an agent to write
/// it: `sessions.rs` is where each is asked end to end.
#[tokio::test]
async fn a_pick_leaves_the_conversation_grilling() {
    for (picked, direction) in [
        ("inline", verkstead_schema::Direction::Inline),
        ("task-list", verkstead_schema::Direction::TaskList),
        ("roadmap", verkstead_schema::Direction::Roadmap),
    ] {
        let (watched, dir, app, _repo, repo_id) = workbench().await;
        let id = grilling(&app, watched.path(), repo_id).await;

        let written = handoff_written(dir.path(), id, "# What we settled\n");

        assert_eq!(
            picking(&app, ask(&app, id, PROPOSING).await, picked).await,
            verkstead_render::Submitted::Accepted,
        );

        let view = opened(&app, id).await;

        assert_eq!(
            view.direction,
            Some(direction),
            "the pick is recorded: it is what the artifact is watched for — picking {picked}",
        );
        assert_eq!(
            view.state,
            Lifecycle::Grilling,
            "and nothing moved, because the grilling is what is still happening \
             — picking {picked}",
        );
        assert_eq!(moves(&view), [Lifecycle::Grilling], "picking {picked}");

        assert!(
            written.exists() && handoff(&view).is_none(),
            "the handoff is taken when the session ends, and it has not ended \
             — picking {picked}",
        );
    }
}

/// There is nowhere left to press a direction: the standalone chooser and the
/// endpoint that served it are gone with the state they belonged to.
#[tokio::test]
async fn there_is_no_endpoint_left_to_choose_a_direction_on() {
    let (watched, _dir, app, _repo, repo_id) = workbench().await;
    let id = grilling(&app, watched.path(), repo_id).await;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/ui/conversations/{id}/direction"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({ "direction": "inline" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        opened(&app, id).await.direction,
        None,
        "and nothing was recorded by trying",
    );
}

#[tokio::test]
async fn disagreeing_with_a_proposal_leaves_the_grilling_running() {
    let (watched, _dir, app, _repo, repo_id) = workbench().await;
    let id = grilling(&app, watched.path(), repo_id).await;

    let set = ask(&app, id, PROPOSING).await;

    // Nothing picked on the chooser, and words of their own beside the question
    // — which is the shape of a human saying what is still open.
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
        "only a pick ends a grilling",
    );
    assert_eq!(moves(&view), [Lifecycle::Grilling]);
    assert_eq!(view.direction, None, "and nothing was picked");
}

#[tokio::test]
async fn a_proposal_put_again_after_a_refusal_can_be_picked_on() {
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
    picking(&app, ask(&app, id, &again).await, "inline").await;

    let view = opened(&app, id).await;

    assert_eq!(
        moves(&view),
        [Lifecycle::Grilling],
        "neither the refusal nor the pick moved anything: what moves a \
         Conversation is the artifact the pick asked for",
    );
    assert_eq!(
        view.direction,
        Some(verkstead_schema::Direction::Inline),
        "and what stands is the pick on the second proposal, not the refused one",
    );
}

/// A proposal with nothing to read beside the recommendation is refused as it
/// arrives, because the chooser would draw the human a bare word to decide
/// against.
#[tokio::test]
async fn a_proposal_with_no_reasoning_is_refused_as_it_arrives() {
    let (watched, _dir, app, _repo, repo_id) = workbench().await;
    let id = grilling(&app, watched.path(), repo_id).await;

    let unreasoned = PROPOSING
        .split("  rationale:")
        .next()
        .expect("the fixture has a rationale to cut off")
        .to_owned()
        + "  rationale: \"  \"\n";

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/conversations/{id}/api/v1/sets"))
                .header(header::CONTENT_TYPE, "application/yaml")
                .body(Body::from(unreasoned))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let refusal = String::from_utf8(body.to_vec()).unwrap();
    assert!(
        refusal.contains("rationale"),
        "the refusal should say what is missing, got: {refusal}"
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

/// The backlog opened: what the details pane fetches when somebody presses the
/// task-list card.
async fn backlog_pane(app: &Router, id: i64) -> BacklogPane {
    get(app, &format!("/api/ui/conversations/{id}/backlog")).await
}

/// The card says which tasks there are; the pane says what each of them is. Both
/// are one reading of `.tasks/`, so the entries line up.
#[tokio::test]
async fn the_task_list_opens_as_every_task_document_it_names() {
    let (watched, _dir, app, _repo, repo_id) = workbench().await;
    let id = ready(&app, watched.path(), repo_id).await;
    grill(&app, id).await;

    let worktree = PathBuf::from(opened(&app, id).await.worktree.unwrap().path);
    plan(&worktree, BACKLOG, &[]);
    std::fs::write(
        worktree.join(".tasks/02-refusal.md"),
        "# 2. What a refused request is told\n\n\
         ## What to build\n\n\
         A `429` with the window in `Retry-After`.\n",
    )
    .unwrap();

    let pane = backlog_pane(&app, id).await;

    assert_eq!(pane.feature, "Rate limiting");
    assert_eq!(
        pane.tasks
            .iter()
            .map(|task| (task.number.as_str(), task.title.as_str()))
            .collect::<Vec<_>>(),
        [
            ("01", "The counter"),
            ("02", "What a refused request is told"),
        ],
        "the list's own order, which is the order they get worked in",
    );

    assert_eq!(
        pane.tasks[0].html, None,
        "the list names a file nobody wrote, so there is nothing to render",
    );

    let html = pane.tasks[1].html.as_deref().expect("that file is there");

    assert!(
        html.contains("<h1>2. What a refused request is told</h1>"),
        "rendered by the server, like every other document on this wire: {html}",
    );
    assert!(html.contains("<code>429</code>"), "{html}");
    assert!(!pane.diagrams, "and nothing in it draws");
}

/// The three ways there is nothing to open, refused the same way: what the human
/// would do about each of them is the same nothing.
#[tokio::test]
async fn a_conversation_with_no_backlog_has_no_pane_to_open() {
    let (watched, _dir, app, _repo, repo_id) = workbench().await;
    let id = ready(&app, watched.path(), repo_id).await;

    // Before there is a worktree at all.
    assert_eq!(refused_backlog(&app, id).await, StatusCode::NOT_FOUND);

    // And with one that holds no `.tasks/`.
    grill(&app, id).await;
    assert_eq!(refused_backlog(&app, id).await, StatusCode::NOT_FOUND);

    // And once the finished feature's list has been taken away, which is what
    // the last commit of a backlog does.
    let worktree = PathBuf::from(opened(&app, id).await.worktree.unwrap().path);
    plan(&worktree, BACKLOG, &["02-refusal.md"]);
    assert!(!backlog_pane(&app, id).await.tasks.is_empty());

    std::fs::remove_dir_all(worktree.join(".tasks")).unwrap();
    assert_eq!(refused_backlog(&app, id).await, StatusCode::NOT_FOUND);

    // And a Conversation that is not there, or an id out of a URL somebody
    // typed, which name no backlog either.
    assert_eq!(refused_backlog(&app, 404).await, StatusCode::NOT_FOUND);

    let (status, _) = fetch(
        &app,
        Request::builder()
            .uri("/api/ui/conversations/nonsense/backlog")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

/// What the refusal came back as, for the cases where there is no pane.
async fn refused_backlog(app: &Router, id: i64) -> StatusCode {
    let (status, body) = fetch(
        app,
        Request::builder()
            .uri(format!("/api/ui/conversations/{id}/backlog"))
            .body(Body::empty())
            .unwrap(),
    )
    .await;

    assert!(
        body.contains("no backlog"),
        "the refusal should say what is missing, got: {body}"
    );

    status
}

/// The roadmap a session wrote into the worktree, as the staging skill writes
/// one: the index, and a stage brief per entry that has one.
///
/// Uncommitted, which is what a session part-way through leaves — and which the
/// reading behind both the card and the pane takes as this branch's own.
fn staged(worktree: &Path, name: &str, index: &str, briefs: &[(&str, &str)]) {
    let directory = worktree.join("docs/roadmaps").join(name);
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(directory.join("ROADMAP.md"), index).unwrap();

    for (file, markdown) in briefs {
        std::fs::write(directory.join(file), markdown).unwrap();
    }
}

/// The roadmap opened: what the details pane fetches when somebody presses the
/// stage-list card.
async fn roadmap_pane(app: &Router, id: i64, name: &str) -> RoadmapPane {
    get(app, &format!("/api/ui/conversations/{id}/roadmap/{name}")).await
}

/// The card says which stages there are; the pane says what each of them is for.
/// Both are one reading of `docs/roadmaps/`, so the entries line up.
#[tokio::test]
async fn the_stage_list_opens_as_every_stage_brief_it_names() {
    let (watched, _dir, app, _repo, repo_id) = workbench().await;
    let id = ready(&app, watched.path(), repo_id).await;
    grill(&app, id).await;

    let worktree = PathBuf::from(opened(&app, id).await.worktree.unwrap().path);
    staged(
        &worktree,
        "mvp",
        OPEN_AT_THREE,
        &[
            ("01-workbench.md", "# 1. Workbench\n\nThree panes.\n"),
            (
                "03-implementation.md",
                "# 3. Implementation\n\n## What to build\n\nA `runner` that drives it.\n",
            ),
        ],
    );

    let pane = roadmap_pane(&app, id, "mvp").await;

    assert_eq!(pane.name, "mvp");
    assert_eq!(pane.title, "MVP roadmap");
    assert_eq!(
        pane.stages
            .iter()
            .map(|stage| (stage.number.as_str(), stage.title.as_str(), stage.done))
            .collect::<Vec<_>>(),
        [
            ("01", "Workbench", true),
            ("02", "Grilling", true),
            ("03", "Implementation", false),
            ("04", "Wrap-up", false),
        ],
        "the roadmap's own order, which is the order they get worked in",
    );

    // A stage's brief stays where it is for ever, so a done stage has its
    // document like any other — the other way round from a finished task.
    assert!(
        pane.stages[0]
            .html
            .as_deref()
            .expect("the done stage's brief is still there")
            .contains("<h1>1. Workbench</h1>"),
    );

    let html = pane.stages[2].html.as_deref().expect("that file is there");

    assert!(
        html.contains("<h1>3. Implementation</h1>"),
        "rendered by the server, like every other document on this wire: {html}",
    );
    assert!(html.contains("<code>runner</code>"), "{html}");
    assert!(!pane.diagrams, "and nothing in it draws");

    // And the two the roadmap names briefs for that nobody wrote, which the pane
    // says in words rather than drawing a gap.
    assert_eq!(pane.stages[1].html, None);
    assert_eq!(pane.stages[3].html, None);
}

/// The ways there is nothing to open, refused the same way: what the human would
/// do about each of them is the same nothing.
#[tokio::test]
async fn a_conversation_with_no_such_roadmap_has_no_pane_to_open() {
    let (watched, _dir, app, _repo, repo_id) = workbench().await;
    let id = ready(&app, watched.path(), repo_id).await;

    // Before there is a worktree at all.
    assert_eq!(
        refused_roadmap(&app, id, "mvp").await,
        StatusCode::NOT_FOUND
    );

    // And with one whose branch has written no roadmap.
    grill(&app, id).await;
    assert_eq!(
        refused_roadmap(&app, id, "mvp").await,
        StatusCode::NOT_FOUND
    );

    let worktree = PathBuf::from(opened(&app, id).await.worktree.unwrap().path);
    staged(&worktree, "mvp", OPEN_AT_THREE, &[]);
    assert!(!roadmap_pane(&app, id, "mvp").await.stages.is_empty());

    // A name this branch has not written to is nothing to open, whether it is
    // another roadmap of the repository's or a path somebody typed: the check is
    // what keeps either from being joined onto anything.
    assert_eq!(
        refused_roadmap(&app, id, "public-release").await,
        StatusCode::NOT_FOUND,
    );
    assert_eq!(
        refused_roadmap(&app, id, "..%2F..%2Fetc").await,
        StatusCode::NOT_FOUND,
    );

    // And once the whole directory has gone.
    std::fs::remove_dir_all(worktree.join("docs/roadmaps")).unwrap();
    assert_eq!(
        refused_roadmap(&app, id, "mvp").await,
        StatusCode::NOT_FOUND
    );

    // And a Conversation that is not there, or an id out of a URL somebody
    // typed, which name no roadmap either.
    assert_eq!(
        refused_roadmap(&app, 404, "mvp").await,
        StatusCode::NOT_FOUND
    );

    let (status, _) = fetch(
        &app,
        Request::builder()
            .uri("/api/ui/conversations/nonsense/roadmap/mvp")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

/// What the refusal came back as, for the cases where there is no pane.
async fn refused_roadmap(app: &Router, id: i64, name: &str) -> StatusCode {
    let (status, body) = fetch(
        app,
        Request::builder()
            .uri(format!("/api/ui/conversations/{id}/roadmap/{name}"))
            .body(Body::empty())
            .unwrap(),
    )
    .await;

    assert!(
        body.contains("no roadmap"),
        "the refusal should say what is missing, got: {body}"
    );

    status
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
    git(&repo, &["branch", "predecessor", &before]);

    // The default branch moves on: stage 03 is ticked off there.
    roadmap(&repo, OPEN_AT_FOUR, &[]);

    let id = adopting(&app, repo_id, "mvp").await;
    assert_eq!(
        stage_of(&opened(&app, id).await).label,
        "04",
        "with no override, the default branch's tip is what is read",
    );

    assert_eq!(
        base(&app, id, Some("predecessor")).await,
        BaseRecorded::Recorded
    );
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

async fn press_adopt(app: &Router, id: i64) -> Adopted {
    post(
        app,
        &format!("/api/ui/conversations/{id}/adopt"),
        &serde_json::json!({}),
    )
    .await
}

/// Everything an adoption needs before the press: both Profiles chosen, which
/// is the whole of what an adopting Conversation has to settle — the Brief is
/// the stage brief and it arrives with the stage.
async fn ready_to_adopt(app: &Router, watched: &Path, repo_id: i64, name: &str) -> i64 {
    let id = adopting(app, repo_id, name).await;

    let grilling = profile(app, watched, "fable").await;
    let implementation = profile(app, watched, "opus").await;
    choose(app, id, "grilling", grilling).await;
    choose(app, id, "implementation", implementation).await;

    id
}

/// What Verkstead has said on a Timeline on its own account.
fn notices(view: &ConversationView) -> Vec<String> {
    view.timeline
        .iter()
        .filter_map(|event| match event {
            TimelineEvent::Notice(notice) => Some(notice.html.clone()),
            _ => None,
        })
        .collect()
}

/// The whole of what pressing Adopt does: the stage's own branch off the base
/// commit, a worktree with it, the stage brief as the Brief, and a Conversation
/// that is implementing the stage.
#[tokio::test]
async fn adopting_starts_the_stage_on_its_own_branch_off_the_base_commit() {
    let (watched, dir, app, repo, repo_id) = workbench().await;
    roadmap(
        &repo,
        OPEN_AT_THREE,
        &["03-implementation.md", "04-wrap-up.md"],
    );
    let tip = git(&repo, &["rev-parse", "HEAD"]).trim().to_owned();

    let id = ready_to_adopt(&app, watched.path(), repo_id, "mvp").await;

    assert_eq!(press_adopt(&app, id).await, Adopted::Adopted);

    let view = opened(&app, id).await;

    // The stage's own slug, rather than the name the server invented for the
    // row: `03-implementation.md` without its number.
    assert_eq!(view.branch, "implementation");
    assert_eq!(view.state, Lifecycle::Implementing);
    assert_eq!(
        moves(&view),
        [Lifecycle::Implementing],
        "straight to Implementing: there was no grilling and no direction to choose",
    );

    // Branched from what the base resolved to, which with no override is the
    // default branch's tip.
    assert_eq!(view.base_commit.as_deref(), Some(tip.as_str()));

    // The branch is in the Repo's own git directory, standing on that commit
    // and nothing else — adoption never stacks.
    assert_eq!(
        git(&repo, &["rev-parse", "refs/heads/implementation"]).trim(),
        tip,
    );

    // And the worktree is git's, under the data directory.
    let worktree = PathBuf::from(view.worktree.expect("a stage has a Worktree").path);

    assert!(worktree.starts_with(dir.path()));
    assert!(worktrees(&repo).contains(&worktree));
    assert!(
        worktree
            .join("docs/roadmaps/mvp/03-implementation.md")
            .exists()
    );
}

/// A stage Conversation steered into a second round is not a stage to adopt
/// again. Adopting is how that work *started*, so a second press is not another
/// adoption: what the steered round has is a Brief of its own, grilled the
/// ordinary way.
#[tokio::test]
async fn a_stage_steered_into_a_second_round_is_not_a_stage_to_adopt_again() {
    let watched = tempfile::tempdir().unwrap();
    let (_dir, app) = app_watching(watched.path()).await;
    let repo = repository(watched.path().join("verkstead"));

    let registered: Registered =
        post(&app, "/api/ui/repos", &serde_json::json!({ "path": repo })).await;
    assert_eq!(registered, Registered::Added);

    let repo_id = listed_repos(&app).await;
    roadmap(
        &repo,
        OPEN_AT_THREE,
        &["03-implementation.md", "04-wrap-up.md"],
    );

    let id = ready_to_adopt(&app, watched.path(), repo_id, "mvp").await;
    assert_eq!(press_adopt(&app, id).await, Adopted::Adopted);

    assert_eq!(
        steer(&app, id).await,
        SteerOpened::Opened { working: false }
    );
    assert_eq!(
        steer_grilling(&app, id, Some("# The implementation, again\n")).await,
        ConversationSteered::Steered,
    );

    let view = opened(&app, id).await;
    assert_eq!(view.state, Lifecycle::Grilling);
    assert_eq!(
        view.adopting, None,
        "the adoption has happened; what is offered now is the ordinary start",
    );

    assert_eq!(
        press_adopt(&app, id).await,
        Adopted::NotDrafting,
        "and the press behind it is refused, however it was reached",
    );

    assert_eq!(
        briefs(&view).len(),
        2,
        "the stage brief, and the round steered into",
    );
}

/// The Timeline gets both records: the stage brief as the Brief the work runs
/// from, and what was adopted from where.
#[tokio::test]
async fn an_adopted_stage_carries_its_brief_and_says_what_it_adopted() {
    let (watched, _dir, app, repo, repo_id) = workbench().await;
    roadmap(
        &repo,
        OPEN_AT_THREE,
        &["03-implementation.md", "04-wrap-up.md"],
    );

    let id = ready_to_adopt(&app, watched.path(), repo_id, "mvp").await;
    assert_eq!(press_adopt(&app, id).await, Adopted::Adopted);

    let view = opened(&app, id).await;

    assert_eq!(
        brief(&view).markdown,
        "# 03-implementation.md\n",
        "the stage brief itself, as the repository holds it",
    );

    let said = notices(&view).join("\n");

    assert!(
        said.contains("Stage 03") && said.contains("<code>mvp</code>"),
        "the record says which stage of which roadmap: {said:?}",
    );
    assert!(
        said.contains("<code>docs/roadmaps/mvp/03-implementation.md</code>"),
        "and which brief it was adopted from: {said:?}",
    );
    assert!(
        said.contains("<code>implementation</code>") && said.contains("<code>main</code>"),
        "and where its branch came off: {said:?}",
    );
}

/// The stage is read again at the press rather than taken from what the page
/// showed: a base the human fixed to an earlier commit is adopted by the stage
/// that is next *there*.
#[tokio::test]
async fn the_stage_adopted_is_the_one_the_base_commit_has_open() {
    let (watched, _dir, app, repo, repo_id) = workbench().await;
    roadmap(
        &repo,
        OPEN_AT_THREE,
        &["03-implementation.md", "04-wrap-up.md"],
    );
    let before = git(&repo, &["rev-parse", "HEAD"]).trim().to_owned();
    git(&repo, &["branch", "predecessor", &before]);

    // The default branch moves on: stage 03 is ticked off there, so 04 is what
    // the tip has open.
    roadmap(&repo, OPEN_AT_FOUR, &[]);

    let id = ready_to_adopt(&app, watched.path(), repo_id, "mvp").await;
    assert_eq!(
        base(&app, id, Some("predecessor")).await,
        BaseRecorded::Recorded
    );

    assert_eq!(press_adopt(&app, id).await, Adopted::Adopted);

    let view = opened(&app, id).await;

    assert_eq!(view.branch, "implementation");
    assert_eq!(view.base_commit.as_deref(), Some(before.as_str()));
}

/// The `mvp` roadmap with every box ticked, which is what the stage after the
/// last one leaves behind.
const ALL_DONE: &str = "\
# MVP roadmap

Turns this askance clone into Verkstead.

## Stages

- [x] 01: Workbench — [brief](01-workbench.md)
- [x] 02: Grilling — [brief](02-grilling.md)
- [x] 03: Implementation — [brief](03-implementation.md)
- [x] 04: Wrap-up — [brief](04-wrap-up.md)
";

/// And with stage 03 marked as somebody's, in the words `/next-stage`
/// annotates one with.
const TAKEN_AT_THREE: &str = "\
# MVP roadmap

Turns this askance clone into Verkstead.

## Stages

- [x] 01: Workbench — [brief](01-workbench.md)
- [x] 02: Grilling — [brief](02-grilling.md)
- [ ] 03: Implementation — [brief](03-implementation.md) *(in progress: `someone-elses`)*
- [ ] 04: Wrap-up — [brief](04-wrap-up.md)
";

/// What a refused press has to leave behind: a Conversation that has not moved,
/// nothing checked out, and no branch where the stage's would have gone.
async fn nothing_adopted(app: &Router, id: i64, repo: &Path) {
    let view = opened(app, id).await;

    assert_eq!(view.state, Lifecycle::Draft);
    assert_eq!(view.worktree, None);
    assert!(
        worktrees(repo).len() == 1,
        "only the repository itself is checked out anywhere",
    );
    assert!(
        git(repo, &["branch", "--list", "implementation"])
            .trim()
            .is_empty(),
        "and the stage's own branch was never made",
    );
    assert_eq!(
        brief(&view).markdown,
        "",
        "and the stage brief was never taken as this Conversation's Brief",
    );
}

/// Both Profiles are fixed before adopting, exactly as they are before
/// grilling: the implementation one is what the stage's work runs under, and
/// the grilling one is carried because every stage after it inherits both.
#[tokio::test]
async fn adopting_is_refused_by_name_when_a_profile_is_unchosen() {
    let (watched, _dir, app, repo, repo_id) = workbench().await;
    roadmap(
        &repo,
        OPEN_AT_THREE,
        &["03-implementation.md", "04-wrap-up.md"],
    );

    let id = adopting(&app, repo_id, "mvp").await;

    assert_eq!(press_adopt(&app, id).await, Adopted::NoGrillingProfile);
    nothing_adopted(&app, id, &repo).await;

    choose(
        &app,
        id,
        "grilling",
        profile(&app, watched.path(), "fable").await,
    )
    .await;

    assert_eq!(
        press_adopt(&app, id).await,
        Adopted::NoImplementationProfile
    );
    nothing_adopted(&app, id, &repo).await;

    choose(
        &app,
        id,
        "implementation",
        profile(&app, watched.path(), "opus").await,
    )
    .await;

    assert_eq!(press_adopt(&app, id).await, Adopted::Adopted);
}

/// A Profile whose pair has gone is no account to run a session under, which is
/// a different job from choosing one.
#[tokio::test]
async fn adopting_is_refused_when_a_chosen_profiles_pair_has_gone() {
    let (watched, _dir, app, repo, repo_id) = workbench().await;
    roadmap(
        &repo,
        OPEN_AT_THREE,
        &["03-implementation.md", "04-wrap-up.md"],
    );

    let id = ready_to_adopt(&app, watched.path(), repo_id, "mvp").await;
    std::fs::remove_dir_all(watched.path().join("fable")).unwrap();

    assert_eq!(press_adopt(&app, id).await, Adopted::ProfileBroken);
    nothing_adopted(&app, id, &repo).await;
}

/// A Conversation that began with a Brief and a grilling has no roadmap to take
/// a stage from, and one that has been adopted already has been started once.
#[tokio::test]
async fn only_a_drafting_adopting_conversation_can_be_adopted() {
    let (watched, _dir, app, repo, repo_id) = workbench().await;
    roadmap(
        &repo,
        OPEN_AT_THREE,
        &["03-implementation.md", "04-wrap-up.md"],
    );

    let ordinary = started(&app, repo_id).await;

    assert_eq!(
        press_adopt(&app, ordinary).await,
        Adopted::NotAdopting,
        "and it is answered before the Profiles are, which it has none of",
    );
    nothing_adopted(&app, ordinary, &repo).await;

    let id = ready_to_adopt(&app, watched.path(), repo_id, "mvp").await;

    assert_eq!(press_adopt(&app, id).await, Adopted::Adopted);
    assert_eq!(
        press_adopt(&app, id).await,
        Adopted::NotDrafting,
        "two branches and two worktrees for one stage is what adopting twice would mean",
    );

    assert_eq!(worktrees(&repo).len(), 2, "the repository and one worktree");
}

/// A branch that was there when the human picked it can be gone by the time the
/// button is pressed, which is exactly why it is asked again.
#[tokio::test]
async fn adopting_is_refused_when_the_base_branch_no_longer_resolves() {
    let (watched, _dir, app, repo, repo_id) = workbench().await;
    roadmap(
        &repo,
        OPEN_AT_THREE,
        &["03-implementation.md", "04-wrap-up.md"],
    );

    let id = ready_to_adopt(&app, watched.path(), repo_id, "mvp").await;

    git(&repo, &["branch", "doomed"]);
    assert_eq!(base(&app, id, Some("doomed")).await, BaseRecorded::Recorded);

    git(&repo, &["branch", "-D", "doomed"]);

    assert_eq!(press_adopt(&app, id).await, Adopted::NoBaseCommit);
    nothing_adopted(&app, id, &repo).await;
}

/// A roadmap is read at origin's tip of the default branch rather than at this
/// checkout's copy of it — on the page and again at the press, both of them
/// fetching first.
///
/// The case this is for is the ordinary one: a roadmap somebody else pushed, or
/// a stage somebody else ticked, on a machine that has not pulled since.
#[tokio::test]
async fn adopting_reads_the_roadmap_at_origins_tip() {
    let (watched, _dir, app, _repo, upstream, repo_id) = workbench_with_origin().await;

    // The roadmap is committed on origin and nowhere else: this checkout has
    // heard nothing about it, and neither has its copy of `origin/main`.
    roadmap(
        &upstream,
        OPEN_AT_THREE,
        &["03-implementation.md", "04-wrap-up.md"],
    );
    let tip = git(&upstream, &["rev-parse", "HEAD"]).trim().to_owned();

    let id = ready_to_adopt(&app, watched.path(), repo_id, "mvp").await;

    assert_eq!(
        stage_of(&opened(&app, id).await).label,
        "03",
        "the page fetches before it reads, so it names the stage origin is holding",
    );

    assert_eq!(press_adopt(&app, id).await, Adopted::Adopted);

    let view = opened(&app, id).await;
    assert_eq!(view.base_commit.as_deref(), Some(tip.as_str()));

    let worktree = PathBuf::from(view.worktree.expect("a stage has a Worktree").path);
    assert!(
        worktree
            .join("docs/roadmaps/mvp/03-implementation.md")
            .exists(),
        "and the stage is worked on what origin is holding",
    );
}

/// There is a human at this button, so a fetch git would not make refuses the
/// press by name rather than adopting a stage judged against refs nobody can
/// vouch for. Being offline, or having lost an authentication, is theirs to fix.
#[tokio::test]
async fn adopting_is_refused_by_name_when_the_fetch_fails() {
    let (watched, dir, app, repo, _upstream, repo_id) = workbench_with_origin().await;

    // Committed here, so that what refuses the press is the fetch and not the
    // roadmap being missing.
    roadmap(
        &repo,
        OPEN_AT_THREE,
        &["03-implementation.md", "04-wrap-up.md"],
    );

    let id = ready_to_adopt(&app, watched.path(), repo_id, "mvp").await;

    let nowhere = dir.path().join("no-such-remote");
    git(
        &repo,
        &["remote", "set-url", "origin", &nowhere.to_string_lossy()],
    );

    assert_eq!(press_adopt(&app, id).await, Adopted::FetchFailed);
    nothing_adopted(&app, id, &repo).await;
}

/// The three ways a stage can stop being startable between the notice being
/// drawn and the button being pressed, each its own thing to go and do about
/// it: somebody ticked the last box, somebody moved the brief, somebody took
/// the stage.
#[tokio::test]
async fn adopting_is_refused_by_name_for_each_way_the_stage_has_gone() {
    let (watched, _dir, app, repo, repo_id) = workbench().await;
    roadmap(
        &repo,
        OPEN_AT_THREE,
        &["03-implementation.md", "04-wrap-up.md"],
    );

    let id = ready_to_adopt(&app, watched.path(), repo_id, "mvp").await;

    // Somebody finished the roadmap by hand while the page stood open.
    roadmap(&repo, ALL_DONE, &[]);

    assert_eq!(press_adopt(&app, id).await, Adopted::RoadmapComplete);
    nothing_adopted(&app, id, &repo).await;

    // Or moved the brief the next stage names, which is the roadmap's own to
    // fix: starting the stage after it would be Verkstead deciding to skip work.
    roadmap(&repo, OPEN_AT_THREE, &[]);
    std::fs::remove_file(repo.join("docs/roadmaps/mvp/03-implementation.md")).unwrap();
    roadmap(&repo, OPEN_AT_THREE, &[]);

    assert_eq!(press_adopt(&app, id).await, Adopted::NoBrief);
    nothing_adopted(&app, id, &repo).await;

    // Or started it themselves and said so, with the branch to prove it — the
    // annotation is prose, and the branch inside its backticks is the fact.
    roadmap(&repo, TAKEN_AT_THREE, &["03-implementation.md"]);
    git(&repo, &["branch", "someone-elses"]);

    assert_eq!(press_adopt(&app, id).await, Adopted::StageInFlight);
    nothing_adopted(&app, id, &repo).await;

    // And a note left over from an attempt that was abandoned too stops
    // nothing: the branch is the fact, and it is not there.
    git(&repo, &["branch", "-D", "someone-elses"]);

    assert_eq!(press_adopt(&app, id).await, Adopted::Adopted);
}

/// Verkstead did not make the branch, so it will not take it over: what is on
/// it is somebody's work, whatever the roadmap's boxes say.
#[tokio::test]
async fn adopting_is_refused_when_the_stages_own_branch_is_taken() {
    let (watched, _dir, app, repo, repo_id) = workbench().await;
    roadmap(
        &repo,
        OPEN_AT_THREE,
        &["03-implementation.md", "04-wrap-up.md"],
    );

    let id = ready_to_adopt(&app, watched.path(), repo_id, "mvp").await;
    git(&repo, &["branch", "implementation"]);

    assert_eq!(press_adopt(&app, id).await, Adopted::BranchExists);

    let view = opened(&app, id).await;
    assert_eq!(view.state, Lifecycle::Draft);
    assert_eq!(view.worktree, None);
    assert_eq!(worktrees(&repo).len(), 1, "only the repository itself");
    assert_eq!(
        git(&repo, &["rev-parse", "refs/heads/implementation"]).trim(),
        git(&repo, &["rev-parse", "HEAD"]).trim(),
        "and the branch that was there is where it was",
    );
}

/// A roadmap the base commit knows nothing about is not a roadmap that
/// finished, and saying so would send the human looking at the wrong document.
#[tokio::test]
async fn adopting_is_refused_when_no_such_roadmap_is_at_the_base() {
    let (watched, _dir, app, repo, repo_id) = workbench().await;
    roadmap(
        &repo,
        OPEN_AT_THREE,
        &["03-implementation.md", "04-wrap-up.md"],
    );

    let id = ready_to_adopt(&app, watched.path(), repo_id, "public-release").await;

    assert_eq!(press_adopt(&app, id).await, Adopted::NoRoadmap);
    nothing_adopted(&app, id, &repo).await;
}

/// Cheap-first, and provably so: a Conversation with no Profiles, against a
/// roadmap that has finished and whose branch is taken besides, is answered
/// about its Profiles. Everything git is paid for is asked after the record's
/// own state and the pair of accounts it would run under.
#[tokio::test]
async fn the_cheap_refusals_are_answered_before_the_ones_git_is_paid_for() {
    let (_watched, _dir, app, repo, repo_id) = workbench().await;
    roadmap(&repo, ALL_DONE, &["03-implementation.md"]);
    git(&repo, &["branch", "implementation"]);

    let id = adopting(&app, repo_id, "mvp").await;

    assert_eq!(press_adopt(&app, id).await, Adopted::NoGrillingProfile);
    assert_eq!(
        opened(&app, id).await.state,
        Lifecycle::Draft,
        "and nothing about the roadmap was read to find that out",
    );
}

/// How a pull request's checks are is carried to both copies of its card: the
/// one pinned above the record and the one at the moment it opened.
///
/// Walked through the store rather than watched for, as the narrowing below is:
/// what is under test is the reading, and asking GitHub is `src/checks.rs`'s.
/// The aggregate and nothing else — what every check is called belongs to the
/// details pane.
#[tokio::test]
async fn how_a_pull_requests_checks_are_reaches_both_copies_of_its_card() {
    let (watched, dir, app, _repo, repo_id) = workbench().await;
    let id = grilling(&app, watched.path(), repo_id).await;
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    store::record_pull_request(
        &pool,
        id,
        repo_id,
        &store::PullRequest {
            number: 41,
            title: "Rate limiting".to_owned(),
            url: "https://github.com/tobico/verkstead/pull/41".to_owned(),
            repo: None,
        },
    )
    .await
    .unwrap();

    assert_eq!(
        checks(&opened(&app, id).await),
        [None, None],
        "nothing has asked GitHub yet, and a card with nothing to say draws no icon",
    );

    for (asked, drawn) in [
        (store::Rollup::Running, CheckRollup::Running),
        (store::Rollup::Failed, CheckRollup::Failed),
        (store::Rollup::Passed, CheckRollup::Passed),
    ] {
        store::record_check_rollup(&pool, id, asked).await.unwrap();

        assert_eq!(
            checks(&opened(&app, id).await),
            [Some(drawn), Some(drawn)],
            "the card follows the poll, in both places it is drawn",
        );
    }
}

/// How the checks are on each copy of the pull request card a view carries: the
/// pinned one first, then the one on the record.
fn checks(view: &ConversationView) -> [Option<CheckRollup>; 2] {
    let pinned = view.pinned.iter().find_map(|event| match event {
        PinnedEvent::PullRequest(opened) => Some(opened.checks),
        _ => None,
    });

    let reached = view.timeline.iter().find_map(|event| match event {
        TimelineEvent::PullRequest(opened) => Some(opened.checks),
        _ => None,
    });

    [pinned.flatten(), reached.flatten()]
}

/// A wrap-up that has narrowed to its checks says so where the human reads a
/// Conversation: on its card, and on the row in the sidebar they find it by.
///
/// Walked through the store rather than run, because what is under test is the
/// reading rather than the watchers: nothing runs sessions here, so the
/// Conversation sits in Wrapping with exactly the settle facts it is given. What
/// the watchers make of the same facts is `sessions.rs`'s.
///
/// The condition is Wrapping's own and never a state: the Lifecycle does not
/// move at either end of it.
#[tokio::test]
async fn a_wrap_up_down_to_its_checks_says_so_on_the_card_and_in_the_sidebar() {
    let (watched, dir, app, _repo, repo_id) = workbench().await;
    let id = grilling(&app, watched.path(), repo_id).await;
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    assert_eq!(
        store::record_pull_request(
            &pool,
            id,
            repo_id,
            &store::PullRequest {
                number: 41,
                title: "Rate limiting".to_owned(),
                url: "https://github.com/tobico/verkstead/pull/41".to_owned(),
                repo: None,
            },
        )
        .await
        .unwrap(),
        store::Wrapping::Started,
    );

    let view = opened(&app, id).await;
    assert_eq!(view.state, Lifecycle::Wrapping);
    assert!(
        !view.waiting_on_checks,
        "a wrap-up nobody has read yet is waiting on all three of them",
    );

    for waiting_on in [
        store::WaitingOn::Review,
        store::WaitingOn::Comments(repo_id),
    ] {
        store::settle_wrap_up(&pool, id, waiting_on).await.unwrap();
    }

    let view = opened(&app, id).await;
    assert!(
        view.waiting_on_checks,
        "the checks are the whole of what is left, so that is what it is waiting on",
    );
    assert_eq!(
        view.state,
        Lifecycle::Wrapping,
        "which is a condition of Wrapping and not a rung of its own",
    );
    assert!(
        sidebar(&app)
            .await
            .into_iter()
            .find(|row| row.id == id)
            .expect("the Conversation is on the sidebar")
            .waiting_on_checks,
        "and the row says the same thing the card does",
    );

    store::settle_wrap_up(&pool, id, store::WaitingOn::Checks(repo_id))
        .await
        .unwrap();

    assert!(
        !opened(&app, id).await.waiting_on_checks,
        "nothing is waiting on checks that have come in",
    );

    pool.close().await;
}

/// The line saying a wrap-up is down to its checks is written once per
/// narrowing: not once per poll, and not once ever.
///
/// The rule is the store's — the settling loop asks it on a cadence and writes
/// the Notice when it is told to — so it is asked here as that loop asks it,
/// including with a session running, which is the half of the condition no row
/// can answer. A fix session working a red check is a wrap-up getting on with
/// it, and the label is for one with nobody in it.
#[tokio::test]
async fn a_wrap_up_that_narrows_twice_is_worth_saying_so_twice() {
    let (watched, dir, app, _repo, repo_id) = workbench().await;
    let id = grilling(&app, watched.path(), repo_id).await;
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    assert_eq!(
        store::narrowing(&pool, id, false).await.unwrap(),
        store::Narrowing::NotNarrowed,
        "a Conversation that is not wrapping up is waiting on nothing",
    );

    store::record_pull_request(
        &pool,
        id,
        repo_id,
        &store::PullRequest {
            number: 41,
            title: "Rate limiting".to_owned(),
            url: "https://github.com/tobico/verkstead/pull/41".to_owned(),
            repo: None,
        },
    )
    .await
    .unwrap();

    for waiting_on in [
        store::WaitingOn::Review,
        store::WaitingOn::Comments(repo_id),
    ] {
        store::settle_wrap_up(&pool, id, waiting_on).await.unwrap();
    }

    assert_eq!(
        store::narrowing(&pool, id, false).await.unwrap(),
        store::Narrowing::Narrowed,
        "the first look is the one that owes the Timeline a line",
    );
    assert_eq!(
        store::narrowing(&pool, id, false).await.unwrap(),
        store::Narrowing::NoticedAlready,
        "and every look after it finds the line written",
    );

    assert_eq!(
        store::narrowing(&pool, id, true).await.unwrap(),
        store::Narrowing::NotNarrowed,
        "a fix session in the Worktree is a wrap-up getting on with it",
    );
    assert_eq!(
        store::narrowing(&pool, id, false).await.unwrap(),
        store::Narrowing::Narrowed,
        "and the wrap-up going quiet again is worth saying afresh",
    );

    store::unsettle_wrap_up(&pool, id, store::WaitingOn::Comments(repo_id))
        .await
        .unwrap();

    assert_eq!(
        store::narrowing(&pool, id, false).await.unwrap(),
        store::Narrowing::NotNarrowed,
        "a comment landing is something else to deal with, so it is not the checks alone",
    );

    store::settle_wrap_up(&pool, id, store::WaitingOn::Comments(repo_id))
        .await
        .unwrap();

    assert_eq!(
        store::narrowing(&pool, id, false).await.unwrap(),
        store::Narrowing::Narrowed,
        "and dealing with it narrows the wrap-up a second time, which is a second line",
    );

    pool.close().await;
}

/// The browser saying the human has looked at a Conversation takes the news
/// mark off its row, and it does not come back.
///
/// Walked through the store at the writing end, because what is under test is
/// the press: what puts the mark on is the wrap-up reaching Done, which
/// `sessions.rs` runs for real.
///
/// Refused for nothing, and that matters more than it looks: the press rides
/// every opening of every Conversation, and one that answered an error for a
/// row with nothing to clear would be an error the human saw for reading their
/// own list.
#[tokio::test]
async fn looking_at_a_conversation_takes_the_news_off_its_row() {
    let (_watched, dir, app, _repo, repo_id) = workbench().await;
    let id = started(&app, repo_id).await;
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    assert!(
        !unseen(&app, id).await,
        "nothing has been said about it yet"
    );

    // The press before anything is marked, which is every opening of every
    // Conversation on an ordinary day.
    see(&app, &id.to_string()).await;
    assert!(!unseen(&app, id).await);

    store::stamp_unseen(&pool, id).await.unwrap();
    assert!(
        unseen(&app, id).await,
        "and the row says there is news on it",
    );

    see(&app, &id.to_string()).await;
    assert!(!unseen(&app, id).await, "which looking at it takes off");

    see(&app, &id.to_string()).await;
    assert!(
        !unseen(&app, id).await,
        "and nothing brings it back: the mark is the one Done, not a counter",
    );

    // An id out of a URL the human may have typed, and one naming nothing:
    // neither is something to refuse for, because looking at something is not a
    // claim that it is there.
    see(&app, "404").await;
    see(&app, "nonsense").await;

    pool.close().await;
}

/// The news mark and *waiting on you* are two facts, and the row carries both:
/// one is something to answer and the other is something to read, and folding
/// either into the other would lose the one the human can act on.
#[tokio::test]
async fn news_on_a_row_leaves_what_is_waiting_on_it_alone() {
    let (watched, dir, app, _repo, repo_id) = workbench().await;
    let id = grilling(&app, watched.path(), repo_id).await;
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    store::stop(
        &pool,
        id,
        store::Decision::Verkstead,
        "The checks would not go green.\n",
        None,
    )
    .await
    .unwrap()
    .expect("the Conversation was running");
    store::stamp_unseen(&pool, id).await.unwrap();

    let both = row(&app, id).await;
    assert!(both.waiting, "Verkstead's brake is waiting on the human");
    assert!(both.unseen, "and there is news on the same Conversation");

    see(&app, &id.to_string()).await;

    let read = row(&app, id).await;
    assert!(
        read.waiting,
        "looking at it read the news; it did not answer the stop",
    );
    assert!(!read.unseen);

    pool.close().await;
}

/// Say the human has looked at one. Answers nothing, and is refused for
/// nothing — see the two tests above.
async fn see(app: &Router, id: &str) {
    let (status, body) = fetch(
        app,
        Request::builder()
            .method("POST")
            .uri(format!("/api/ui/conversations/{id}/seen"))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from("{}"))
            .unwrap(),
    )
    .await;

    assert_eq!(status, StatusCode::NO_CONTENT, "the press failed: {body}");
}

/// One Conversation's row on the sidebar.
async fn row(app: &Router, id: i64) -> ConversationEntry {
    sidebar(app)
        .await
        .into_iter()
        .find(|row| row.id == id)
        .expect("the Conversation is on the sidebar")
}

/// And whether that row says there is news on it.
async fn unseen(app: &Router, id: i64) -> bool {
    row(app, id).await.unseen
}
