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
use sqlx::SqlitePool;
use tower::ServiceExt;
use verkstead_render::{
    Adopted, AgentType, BacklogPane, BaseRecorded, BranchRenamed, BriefSaved, CheckRollup,
    CompanionAdded, CompanionBaseRecorded, CompanionBranchRenamed, CompanionMode,
    CompanionModeChosen, CompanionRefusal, CompanionRemoved, ConversationArchived,
    ConversationClosed, ConversationEntry, ConversationSteered, ConversationStopped,
    ConversationUnarchived, ConversationView, GrillingStarted, Lifecycle, Merging, PickedView,
    PinnedEvent, Process, ProcessPicked, ProfileChosen, ProfileSaved, Registered, RepoEntry,
    RepoSwitched, Resolved, Resumed, RoadmapPane, ShowingArchived, Standing, Started,
    SteerCancelled, SteerCompanionRefusal, SteerOpened, SteerPairingView, SteerSaved, TakenUp,
    TargetRecorded, TimelineEvent,
};
use verkstead_server::{Gh, open_database, router_asking_github, router_keeping, store};

/// A router, plus the directory holding its database and its data directory
/// alive.
///
/// One directory holds both, which is what the real server does: the database is
/// `verkstead.db` inside the Data Directory.
async fn app_keeping() -> (tempfile::TempDir, Router) {
    let (dir, _pool, app) = app_and_pool_keeping().await;

    (dir, app)
}

/// The same, with the pool beside it — for the tests about a fact the viewer
/// reads and no endpoint of this namespace writes. A Cleanup's trim is the one
/// of those: the sweep does it in the background, and what the page has to say
/// about it is read back here.
async fn app_and_pool_keeping() -> (tempfile::TempDir, SqlitePool, Router) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();
    let data_dir = dir.path().to_owned();

    // An author, because every press that cuts a branch for new work is refused
    // without one: what Verkstead commits there is the clearing of whatever task
    // list the base was carrying. A workbench with nobody configured is what
    // [`no_author`] makes, and what the refusals are tested against.
    std::fs::write(dir.path().join(CONFIG), THE_AUTHOR).unwrap();

    (dir, pool.clone(), router_keeping(pool, data_dir))
}

/// Where the author a start commits as is written, under the Data Directory.
const CONFIG: &str = "config.yaml";

/// And who that author is, on every workbench here but the one that takes it
/// away.
const THE_AUTHOR: &str = "git_author:\n  name: Verkstead Test\n  email: test@verkstead.invalid\n";

/// Take the author back off a workbench, leaving one configured the way a
/// machine that skipped the settings page is.
fn no_author(dir: &tempfile::TempDir) {
    std::fs::remove_file(dir.path().join(CONFIG)).unwrap();
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

/// A directory of its own holding one registered repository, and the app over
/// it.
async fn workbench() -> (tempfile::TempDir, tempfile::TempDir, Router, PathBuf, i64) {
    let elsewhere = tempfile::tempdir().unwrap();
    let (dir, app) = app_keeping().await;
    let repo = repository(elsewhere.path().join("verkstead"));

    let registered: Registered =
        post(&app, "/api/ui/repos", &serde_json::json!({ "path": repo })).await;
    assert!(matches!(registered, Registered::Added(_)));

    let repo_id = listed_repos(&app).await;

    (elsewhere, dir, app, repo, repo_id)
}

/// The same, with the pool beside it — see [`app_and_pool_keeping`].
async fn workbench_and_pool() -> (
    tempfile::TempDir,
    tempfile::TempDir,
    Router,
    SqlitePool,
    i64,
) {
    let elsewhere = tempfile::tempdir().unwrap();
    let (dir, pool, app) = app_and_pool_keeping().await;
    let repo = repository(elsewhere.path().join("verkstead"));

    let registered: Registered =
        post(&app, "/api/ui/repos", &serde_json::json!({ "path": repo })).await;
    assert!(matches!(registered, Registered::Added(_)));

    let repo_id = listed_repos(&app).await;

    (elsewhere, dir, app, pool, repo_id)
}

/// A directory of its own holding one registered repository that was *cloned*
/// from an upstream, and the app over it.
///
/// The upstream is what "origin" means for the rest of a test: commits pushed
/// on to it are commits the clone has not seen, which is the whole state these
/// are about. It lives in the data directory's tempdir rather than beside the
/// clone so that nothing registers it by accident.
///
/// Hands back that directory, the data directory, the app, the clone, the
/// upstream and the Repo's id.
async fn workbench_with_origin() -> (
    tempfile::TempDir,
    tempfile::TempDir,
    Router,
    PathBuf,
    PathBuf,
    i64,
) {
    cloned_workbench(Gh::on_path()).await
}

/// The same with a `gh` of its own behind it, which is what a **Review** wants:
/// its target is named in the Brief and resolved through GitHub at the press.
///
/// Unix only, because a stand-in for a program is a program — see the
/// `pull_requests` suite, which is off Windows for the same reason.
#[cfg(unix)]
async fn workbench_reviewing() -> (
    tempfile::TempDir,
    tempfile::TempDir,
    Router,
    PathBuf,
    PathBuf,
    i64,
) {
    cloned_workbench(gh_answering()).await
}

/// What both of those are, `gh` apart.
async fn cloned_workbench(
    gh: Gh,
) -> (
    tempfile::TempDir,
    tempfile::TempDir,
    Router,
    PathBuf,
    PathBuf,
    i64,
) {
    let elsewhere = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();
    std::fs::write(dir.path().join(CONFIG), THE_AUTHOR).unwrap();
    let app = router_asking_github(pool, dir.path().to_owned(), gh);

    let upstream = repository(dir.path().join("upstream"));
    let repo = elsewhere.path().join("verkstead");
    git(
        elsewhere.path(),
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
    assert!(matches!(registered, Registered::Added(_)));

    let repo_id = listed_repos(&app).await;

    (elsewhere, dir, app, repo, upstream, repo_id)
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

async fn name_target(app: &Router, id: i64, target: &str) -> TargetRecorded {
    post(
        app,
        &format!("/api/ui/conversations/{id}/target"),
        &serde_json::json!({ "target": target }),
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

async fn pick_process(app: &Router, id: i64, process: Process) -> ProcessPicked {
    post(
        app,
        &format!("/api/ui/conversations/{id}/process"),
        &serde_json::json!({ "process": process }),
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
    let (_elsewhere, _dir, app, _repo, repo_id) = workbench().await;

    let id = started(&app, repo_id).await;

    let sidebar = sidebar(&app).await;
    assert_eq!(sidebar.len(), 1);
    assert_eq!(sidebar[0].id, id);
    assert_eq!(sidebar[0].repo, "verkstead");
    assert_eq!(sidebar[0].state, Lifecycle::Draft);

    // The branch is the row's name, so there has to be one from the start.
    assert!(!sidebar[0].branch.is_empty());
}

/// The name a Conversation starts on is Verkstead's own: there has to be a
/// branch to cut, and nobody has thought about the work yet. What says so is the
/// record rather than the shape of the name, and it is what the row is drawn as
/// a Draft off.
#[tokio::test]
async fn a_new_conversation_is_started_on_a_name_nobody_has_settled_on() {
    let (_elsewhere, _dir, app, _repo, repo_id) = workbench().await;

    let id = started(&app, repo_id).await;

    let view = opened(&app, id).await;
    assert!(
        !view.branch.is_empty(),
        "there is a branch to cut all the same"
    );
    assert!(!view.branch_named);

    let sidebar = sidebar(&app).await;
    assert_eq!(sidebar[0].branch, view.branch);
    assert!(!sidebar[0].branch_named);
}

/// The prefill is a name, not a placeholder: the human may well leave it, and it
/// has to be one git will take when the branch is finally created.
#[tokio::test]
async fn the_prefilled_branch_name_is_two_words_git_would_take() {
    let (_elsewhere, _dir, app, repo, repo_id) = workbench().await;

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
    let (_elsewhere, _dir, app, _repo, _repo_id) = workbench().await;

    assert_eq!(start(&app, 404).await, Started::NoSuchRepo);
    assert!(sidebar(&app).await.is_empty());
}

/// Everything a Conversation is attached to, in the one payload the middle and
/// the right-hand panes are drawn from.
#[tokio::test]
async fn opening_a_conversation_brings_its_repo_and_its_timeline_with_it() {
    let (_elsewhere, _dir, app, repo, repo_id) = workbench().await;
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
    let (_elsewhere, _dir, app, _repo, repo_id) = workbench().await;
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
    let (_elsewhere, _dir, app, _repo, repo_id) = workbench().await;
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
    let (_elsewhere, _dir, app, _repo, repo_id) = workbench().await;
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
    let (_elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = started(&app, repo_id).await;

    assert_eq!(
        rename(&app, id, "rate-limiting").await,
        BranchRenamed::Renamed
    );

    let view = opened(&app, id).await;
    assert_eq!(view.branch, "rate-limiting");
    assert!(
        view.branch_named,
        "a name they typed is theirs from that moment"
    );

    let sidebar = sidebar(&app).await;
    assert_eq!(sidebar[0].branch, "rate-limiting");
    assert!(sidebar[0].branch_named);
}

/// And clearing the field hands it back. What stands again is the name the
/// Conversation started on rather than another one invented, so a human who
/// typed a name and thought better of it is where they began.
#[tokio::test]
async fn clearing_the_branch_field_hands_the_name_back_to_verkstead() {
    let (_elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = started(&app, repo_id).await;
    let prefilled = opened(&app, id).await.branch;

    assert_eq!(
        rename(&app, id, "rate-limiting").await,
        BranchRenamed::Renamed
    );

    for cleared in ["", "   "] {
        assert_eq!(
            rename(&app, id, cleared).await,
            BranchRenamed::Renamed,
            "{cleared:?} is the field emptied rather than a name to refuse"
        );

        let view = opened(&app, id).await;
        assert_eq!(view.branch, prefilled);
        assert!(!view.branch_named);

        let sidebar = sidebar(&app).await;
        assert_eq!(sidebar[0].branch, prefilled);
        assert!(!sidebar[0].branch_named);

        // And typing one again settles it again, so the next round of the loop
        // has a name to clear.
        assert_eq!(
            rename(&app, id, "rate-limiting").await,
            BranchRenamed::Renamed
        );
    }
}

/// The name is git's to judge, and a name it would not take is refused now
/// rather than when the branch is finally created.
#[tokio::test]
async fn a_name_git_would_not_take_for_a_branch_is_refused() {
    let (_elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = started(&app, repo_id).await;
    let prefilled = opened(&app, id).await.branch;

    for refused in ["has space", "two..dots", "with~tilde", "-dash"] {
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
    let (_elsewhere, _dir, app, _repo, repo_id) = workbench().await;
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
    let (_elsewhere, _dir, app, repo, repo_id) = workbench().await;
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
    let (_elsewhere, _dir, app, repo, repo_id) = workbench().await;
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
    let (_elsewhere, _dir, app, repo, repo_id) = workbench().await;
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
    let (_elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = started(&app, repo_id).await;

    base(&app, id, Some("main")).await;

    for cleared in [None, Some("")] {
        assert_eq!(base(&app, id, cleared).await, BaseRecorded::Recorded);
        assert_eq!(opened(&app, id).await.base_commit, None);
    }
}

/// Every Conversation has a Process, and a new one is a Develop: the ladder as
/// it has always run, which is what the composer defaults to and what every
/// Conversation from before there were Processes reads as.
#[tokio::test]
async fn a_new_conversation_is_a_develop_one() {
    let (_elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = started(&app, repo_id).await;

    assert_eq!(opened(&app, id).await.process, Process::Develop);
}

/// And one started off a pull request is a Review, that being the Process its
/// path already was: Draft to Wrapping over somebody else's branch.
///
/// Read rather than written, so this is as true of the Conversations started
/// before there were Processes as of the one started here.
#[tokio::test]
async fn a_conversation_wrapping_up_a_pull_request_is_a_review_one() {
    let (_elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = wrapping_up(&app, repo_id, 41).await;

    assert_eq!(opened(&app, id).await.process, Process::Review);
}

/// What kind of work it is is the Draft's to say, and what the picker sends is
/// what the pane reads back.
#[tokio::test]
async fn a_drafting_conversations_process_is_the_humans_to_pick() {
    let (_elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = started(&app, repo_id).await;

    assert_eq!(
        pick_process(&app, id, Process::Develop).await,
        ProcessPicked::Picked
    );
    assert_eq!(opened(&app, id).await.process, Process::Develop);
}

/// And **Tinker** is the second row the picker offers, its stage having landed:
/// what the record takes is what the composer draws.
#[tokio::test]
async fn a_draft_can_be_set_to_tinker() {
    let (_elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = started(&app, repo_id).await;

    assert_eq!(
        pick_process(&app, id, Process::Tinker).await,
        ProcessPicked::Picked
    );
    assert_eq!(opened(&app, id).await.process, Process::Tinker);
}

/// And a draft can be set to Investigate, the third Process whose stage has
/// landed.
#[tokio::test]
async fn a_draft_can_be_set_to_investigate() {
    let (_elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = started(&app, repo_id).await;

    assert_eq!(
        pick_process(&app, id, Process::Investigate).await,
        ProcessPicked::Picked
    );
    assert_eq!(opened(&app, id).await.process, Process::Investigate);
}

/// And a draft can be set to Review, the fourth Process whose stage has landed
/// — the wrap-up run over a pull request its Brief names.
#[tokio::test]
async fn a_draft_can_be_set_to_review() {
    let (_elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = started(&app, repo_id).await;

    assert_eq!(
        pick_process(&app, id, Process::Review).await,
        ProcessPicked::Picked
    );
    assert_eq!(opened(&app, id).await.process, Process::Review);
}

/// The one whose stage has not landed is refused by a name of its own, rather
/// than under the refusal about this Conversation: nothing the human does here
/// makes it pickable, and what it is waiting on is Verkstead.
///
/// The endpoint is reachable without the picker, so this is the server's
/// refusal rather than a control that simply drew no row — and what the
/// Conversation is is untouched by an ask it refused.
#[tokio::test]
async fn a_process_whose_stage_has_not_landed_is_refused_by_name() {
    let (_elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = started(&app, repo_id).await;

    assert_eq!(
        pick_process(&app, id, Process::FixMergeIssues).await,
        ProcessPicked::NotLanded,
        "Fix Merge Issues has no stage behind it yet"
    );
    assert_eq!(opened(&app, id).await.process, Process::Develop);
}

/// And the freeze: past drafting, the Process stops being the human's to change,
/// exactly as the branch name and the base do.
///
/// Nothing is written when the work starts — the refusal is the whole of the
/// freeze — so a Develop Conversation nobody touched the picker on reads Develop
/// on either side of it.
#[tokio::test]
async fn a_process_is_settled_once_the_grilling_has_started() {
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = ready(&app, elsewhere.path(), repo_id).await;

    assert_eq!(grill(&app, id).await, GrillingStarted::Started);

    assert_eq!(
        pick_process(&app, id, Process::Develop).await,
        ProcessPicked::NotDrafting
    );
    assert_eq!(opened(&app, id).await.process, Process::Develop);
}

/// A second registered repository in the same directory, for the tests
/// about working alongside one. Hands back its Repo id.
async fn second_repo(app: &Router, elsewhere: &Path, name: &str) -> i64 {
    let path = repository(elsewhere.join(name));

    let registered: Registered =
        post(app, "/api/ui/repos", &serde_json::json!({ "path": path })).await;
    assert!(matches!(registered, Registered::Added(_)));

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
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let askance = second_repo(&app, elsewhere.path(), "askance").await;
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
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let askance = second_repo(&app, elsewhere.path(), "askance").await;
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
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let askance = second_repo(&app, elsewhere.path(), "askance").await;
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
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let askance = second_repo(&app, elsewhere.path(), "askance").await;
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
    let (elsewhere, _dir, app, repo, repo_id) = workbench().await;
    let askance = second_repo(&app, elsewhere.path(), "askance").await;
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
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let askance = second_repo(&app, elsewhere.path(), "askance").await;
    let id = ready(&app, elsewhere.path(), repo_id).await;

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
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let askance = second_repo(&app, elsewhere.path(), "askance").await;
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
    let (_elsewhere, _dir, app, _repo, repo_id) = workbench().await;
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
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let askance = second_repo(&app, elsewhere.path(), "askance").await;
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
    let (_elsewhere, _dir, app, _repo, repo_id) = workbench().await;
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
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let askance = second_repo(&app, elsewhere.path(), "askance").await;
    let alone = second_repo(&app, elsewhere.path(), "alone").await;
    let id = ready(&app, elsewhere.path(), repo_id).await;

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
    let (_elsewhere, _dir, app, _repo, _repo_id) = workbench().await;

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
        pick_process(&app, 404, Process::Develop).await,
        ProcessPicked::NoSuchConversation
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
    let (_elsewhere, _dir, app, _repo, _repo_id) = workbench().await;

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
    let (_elsewhere, _dir, app, _repo, _repo_id) = workbench().await;

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
    let (_elsewhere, _dir, app, _repo, repo_id) = workbench().await;
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
    let (_elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let first = started(&app, repo_id).await;
    let second = started(&app, repo_id).await;

    place(&app, &[first, second]).await;
    let third = started(&app, repo_id).await;

    assert_eq!(order(&app).await, vec![third, first, second]);
}

/// A viewer sends the list it drew, and a row can be gone by the time it lands.
#[tokio::test]
async fn an_order_naming_a_conversation_that_is_not_there_is_still_taken() {
    let (_elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let first = started(&app, repo_id).await;
    let second = started(&app, repo_id).await;

    place(&app, &[second, 9_999, first]).await;

    assert_eq!(order(&app).await, vec![second, first]);
}

/// A claude dir and config file pair inside `elsewhere`, so a Profile saved from
/// it is one a session could actually be run under.
fn pair(elsewhere: &Path, account: &str) -> (PathBuf, PathBuf) {
    let home = elsewhere.join(account);
    let claude_dir = home.join(".claude");
    let config_file = home.join(".claude.json");

    std::fs::create_dir_all(&claude_dir).unwrap();
    std::fs::write(&config_file, "{}\n").unwrap();

    (claude_dir, config_file)
}

/// Save an Agent Profile and hand back its id.
async fn profile(app: &Router, elsewhere: &Path, name: &str) -> i64 {
    let (claude_dir, config_file) = pair(elsewhere, name);

    let saved: ProfileSaved = post(
        app,
        "/api/ui/profiles",
        &serde_json::json!({
            "name": name,
            "account": {
                "agent_type": "Claude",
                "claude_dir": claude_dir,
                "config_file": config_file,
            },
            "models": ["claude-opus-5"],
        }),
    )
    .await;
    assert_eq!(saved, ProfileSaved::Saved);

    let profiles: Vec<verkstead_render::ProfileEntry> = get(app, "/api/ui/profiles").await;
    profiles
        .into_iter()
        .find(|profile| profile.name.as_deref() == Some(name))
        .expect("the Profile just saved should be on the list")
        .id
}

/// Pair a Profile with the one model [`profile`] gives every Profile here, for
/// one of a Conversation's roles.
async fn choose(app: &Router, id: i64, role: &str, profile_id: i64) {
    let pairing = serde_json::json!({ "profile_id": profile_id, "model": "claude-opus-5" });

    // The review picker offers a row that is not an account, so what it sends is
    // which row was picked rather than a Pairing outright — see [`no_review`].
    let picked = match role {
        "review" => serde_json::json!({ "pairing": pairing }),
        _ => pairing,
    };

    let chosen: verkstead_render::ProfileChosen = post(
        app,
        &format!("/api/ui/conversations/{id}/{role}-pairing"),
        &picked,
    )
    .await;
    assert_eq!(chosen, verkstead_render::ProfileChosen::Chosen);
}

/// Pick the Review picker's other row: this Conversation is not to be reviewed.
async fn no_review(app: &Router, id: i64) -> verkstead_render::ProfileChosen {
    post(
        app,
        &format!("/api/ui/conversations/{id}/review-pairing"),
        &serde_json::json!({ "pairing": null }),
    )
    .await
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

/// Press Resume, which recomputes what ought to be driving the Conversation and
/// starts it. Nothing goes with it, as nothing goes with any of these presses.
async fn resume(app: &Router, id: i64) -> Resumed {
    post(
        app,
        &format!("/api/ui/conversations/{id}/resume"),
        &serde_json::json!({}),
    )
    .await
}

/// And the two stops beside it: the one that sees a session out, and the one
/// that ends it where it stands.
async fn stop(app: &Router, id: i64) -> ConversationStopped {
    post(
        app,
        &format!("/api/ui/conversations/{id}/stop"),
        &serde_json::json!({}),
    )
    .await
}

async fn force_stop(app: &Router, id: i64) -> ConversationStopped {
    post(
        app,
        &format!("/api/ui/conversations/{id}/force-stop"),
        &serde_json::json!({}),
    )
    .await
}

/// And remove an Agent Profile, whoever had chosen it — which is how a
/// Conversation comes to be one Resume refuses by name.
async fn remove_profile(app: &Router, profile_id: i64) -> verkstead_render::ProfileDeleted {
    post(
        app,
        &format!("/api/ui/profiles/{profile_id}/delete"),
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

/// And whether there is anything behind that switch, which the same read
/// answers: the list is filtered by the switch, so it cannot say for itself.
async fn anything_archived(app: &Router) -> bool {
    get::<ShowingArchived>(app, "/api/ui/conversations/archived")
        .await
        .any
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

/// Press Steer, which stops the drive and opens the pending steer the form is
/// written on.
///
/// Nothing goes with it, as nothing goes with either stop: which Conversation it
/// is is the whole of what a press says.
async fn steer(app: &Router, id: i64) -> SteerOpened {
    post(
        app,
        &format!("/api/ui/conversations/{id}/steer"),
        &serde_json::json!({}),
    )
    .await
}

/// Keep the form as it stands, which is what the pane posts as it is typed.
///
/// The whole form as the body rather than the field that moved: a row holding
/// the target of one keystroke beside the instruction of another would be a
/// form that was never on anybody's screen.
async fn save_steer(app: &Router, id: i64, form: &serde_json::Value) -> SteerSaved {
    post(app, &format!("/api/ui/conversations/{id}/steer/save"), form).await
}

/// And cancel it, which takes the pending steer away and leaves the
/// Conversation stopped. No body either, for the same reason.
async fn cancel_steer(app: &Router, id: i64) -> SteerCancelled {
    post(
        app,
        &format!("/api/ui/conversations/{id}/steer/cancel"),
        &serde_json::json!({}),
    )
    .await
}

/// And submit the form it opened: where the work goes, and what to do about
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

/// And the submit into Investigating, which carries the payload beside the
/// follow-up's: the question the session it starts is set going on.
async fn steer_investigating(app: &Router, id: i64, brief: Option<&str>) -> ConversationSteered {
    post(
        app,
        &format!("/api/ui/conversations/{id}/steer/submit"),
        &serde_json::json!({
            "target": "Investigating",
            "interrupt": false,
            "investigation": brief,
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

/// Everything a Conversation needs before it will grill: every Profile chosen
/// and a Brief written. Hands back the Conversation's id.
async fn ready(app: &Router, elsewhere: &Path, repo_id: i64) -> i64 {
    let id = started(app, repo_id).await;

    let grilling = profile(app, elsewhere, "fable").await;
    let implementation = profile(app, elsewhere, "opus").await;
    let review = profile(app, elsewhere, "haiku").await;
    choose(app, id, "grilling", grilling).await;
    choose(app, id, "implementation", implementation).await;
    choose(app, id, "review", review).await;

    assert_eq!(
        write_brief(app, id, "# Rate limiting\n\nThe API has none.\n").await,
        BriefSaved::Saved
    );

    id
}

/// What git in `repo` says its worktrees are, by path, each in the
/// filesystem's own spelling of it.
///
/// git prints a path in git's normalisation rather than the host's: on Windows
/// that is forward slashes, and whichever short name the directory was reached
/// through left as it stands. The tests hold paths the Rust side built, so the
/// two are the same directory written two ways and compare unequal. This
/// canonicalises what git said, which is the form `Path::canonicalize` gives
/// the other side. A worktree whose directory has gone cannot be canonicalised
/// and stays as git spelled it, because it is one git is holding either way.
fn worktrees(repo: &Path) -> Vec<PathBuf> {
    git(repo, &["worktree", "list", "--porcelain"])
        .lines()
        .filter_map(|line| line.strip_prefix("worktree "))
        .map(|path| {
            let path = PathBuf::from(path);
            path.canonicalize().unwrap_or(path)
        })
        .collect()
}

/// The whole of what pressing the button does: a branch off the base commit, a
/// worktree registered with it under the data directory, and a Conversation
/// that says it is grilling.
#[tokio::test]
async fn starting_a_grilling_makes_the_branch_and_the_worktree() {
    let (elsewhere, dir, app, repo, repo_id) = workbench().await;
    let id = ready(&app, elsewhere.path(), repo_id).await;

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
    // where the database is, and not inside the repository it was cut from.
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

/// Everything a **Tinker** needs before it will start, which is two roles and a
/// Brief: it is never interviewed, so no Grilling picker is drawn for it and
/// none is answered here. Hands back the Conversation's id.
async fn tinker(app: &Router, elsewhere: &Path, repo_id: i64) -> i64 {
    let id = started(app, repo_id).await;

    assert_eq!(
        pick_process(app, id, Process::Tinker).await,
        ProcessPicked::Picked
    );

    let implementation = profile(app, elsewhere, "opus").await;
    let review = profile(app, elsewhere, "haiku").await;
    choose(app, id, "implementation", implementation).await;
    choose(app, id, "review", review).await;

    assert_eq!(
        write_brief(app, id, "# Rate limiting\n\nThe API has none.\n").await,
        BriefSaved::Saved
    );

    id
}

/// A Tinker waits on the two roles it is run under and never on the grilling
/// one — the same reading on the button and under the press.
///
/// The Grilling picker is drawn for one nowhere, so a Tinker that waited on it
/// would be a Start nothing on the page could ever satisfy.
#[tokio::test]
async fn a_tinker_waits_on_two_roles_and_never_on_the_grilling_one() {
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = started(&app, repo_id).await;

    assert_eq!(
        pick_process(&app, id, Process::Tinker).await,
        ProcessPicked::Picked
    );

    assert_eq!(
        grill(&app, id).await,
        GrillingStarted::NoImplementationProfile,
        "the role the work runs under is asked for first, and nothing asks \
         about a grilling",
    );

    choose(
        &app,
        id,
        "implementation",
        profile(&app, elsewhere.path(), "opus").await,
    )
    .await;

    assert_eq!(
        grill(&app, id).await,
        GrillingStarted::NoReviewProfile,
        "and then the role the wrap-up's review reads under",
    );
    assert!(!opened(&app, id).await.ready_to_grill);

    assert_eq!(no_review(&app, id).await, ProfileChosen::Chosen);

    assert_eq!(
        grill(&app, id).await,
        GrillingStarted::EmptyBrief,
        "a Review picked away is a Review answered, so what is left is the Brief",
    );
    assert!(
        !opened(&app, id).await.ready_to_grill,
        "which the button waits on too",
    );

    assert_eq!(
        write_brief(&app, id, "# Rate limiting\n\nThe API has none.\n").await,
        BriefSaved::Saved
    );

    assert!(
        opened(&app, id).await.ready_to_grill,
        "two roles and a brief is the whole of what a Tinker waits on: the \
         Grilling picker nobody drew is never one of them",
    );
    assert_eq!(grill(&app, id).await, GrillingStarted::Started);
}

/// And the press does the grill start's own sequence, landing Follow-up: the
/// branch, the worktree, the frozen Brief and the move on the Timeline, with no
/// grilling anywhere in it.
#[tokio::test]
async fn starting_a_tinker_lands_it_in_follow_up() {
    let (elsewhere, dir, app, repo, repo_id) = workbench().await;
    let id = tinker(&app, elsewhere.path(), repo_id).await;

    assert_eq!(grill(&app, id).await, GrillingStarted::Started);

    let view = opened(&app, id).await;
    assert_eq!(view.state, Lifecycle::FollowUp);
    assert_eq!(
        moves(&view),
        [Lifecycle::FollowUp],
        "one move, straight past the state a Develop would have been grilled in",
    );

    assert_eq!(
        git(
            &repo,
            &[
                "rev-parse",
                "--verify",
                &format!("refs/heads/{}", view.branch)
            ]
        )
        .trim()
        .len(),
        40,
        "the branch is cut exactly as a grill start cuts one",
    );

    let worktree = view.worktree.expect("a Tinker is worked in a Worktree");
    let path = PathBuf::from(&worktree.path);
    assert!(!worktree.missing);
    assert_eq!(path.parent(), Some(dir.path().join("worktrees").as_path()));
    assert!(
        worktrees(&repo).contains(&path.canonicalize().unwrap()),
        "and git has it registered: {:?}",
        worktrees(&repo),
    );
    assert_eq!(
        git(&path, &["symbolic-ref", "--short", "HEAD"]).trim(),
        view.branch,
        "checked out on the branch the work is on",
    );

    assert!(
        view.base_commit.is_some(),
        "with what it branched from settled, which is the rule resolving",
    );

    // And everything the press freezes is frozen: the Brief, the Pairings and
    // the Process, exactly as they are for a grilling that has started.
    assert_eq!(
        write_brief(&app, id, "# Something else\n").await,
        BriefSaved::NotDrafting
    );
    assert_eq!(no_review(&app, id).await, ProfileChosen::NotDrafting);
    assert_eq!(
        pick_process(&app, id, Process::Develop).await,
        ProcessPicked::NotDrafting
    );
}

/// A Tinker checks its companions out as a grill start does, every one of them
/// on a branch of its own.
#[tokio::test]
async fn starting_a_tinker_checks_its_companions_out_too() {
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let askance = second_repo(&app, elsewhere.path(), "askance").await;

    let id = tinker(&app, elsewhere.path(), repo_id).await;
    assert_eq!(
        add_companion(&app, id, askance).await,
        CompanionAdded::Added
    );

    assert_eq!(grill(&app, id).await, GrillingStarted::Started);

    let view = opened(&app, id).await;
    assert_eq!(view.state, Lifecycle::FollowUp);
    assert_eq!(view.companions.len(), 1);

    let worktree = view.companions[0]
        .worktree
        .as_ref()
        .expect("a companion is checked out when the Conversation's own is");
    assert!(!worktree.missing);
    assert!(PathBuf::from(&worktree.path).join("README.md").is_file());
}

/// A drafting Conversation ready to be investigated: the Process picked, the one
/// role it is run under answered, and a Brief to find out about.
///
/// No Grilling Pairing and no Review Pairing: an Investigate draws neither
/// picker, so neither is ever chosen on one.
async fn investigate(app: &Router, elsewhere: &Path, repo_id: i64) -> i64 {
    let id = started(app, repo_id).await;

    assert_eq!(
        pick_process(app, id, Process::Investigate).await,
        ProcessPicked::Picked
    );

    let implementation = profile(app, elsewhere, "opus").await;
    choose(app, id, "implementation", implementation).await;

    assert_eq!(
        write_brief(app, id, "# Rate limiting\n\nThe API has none.\n").await,
        BriefSaved::Saved
    );

    id
}

/// An Investigate waits on a brief and on the one role it is run under, and on
/// nothing else — the same reading on the button and under the press.
///
/// Neither the Grilling picker nor the Review one is drawn for one, so an
/// Investigate that waited on either would be a Start nothing on the page could
/// ever satisfy.
#[tokio::test]
async fn an_investigate_waits_on_one_role_and_a_brief_and_nothing_else() {
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = started(&app, repo_id).await;

    assert_eq!(
        pick_process(&app, id, Process::Investigate).await,
        ProcessPicked::Picked
    );

    assert_eq!(
        grill(&app, id).await,
        GrillingStarted::NoImplementationProfile,
        "the role the session runs under is the only one asked about",
    );
    assert!(!opened(&app, id).await.ready_to_grill);

    choose(
        &app,
        id,
        "implementation",
        profile(&app, elsewhere.path(), "opus").await,
    )
    .await;

    assert_eq!(
        grill(&app, id).await,
        GrillingStarted::EmptyBrief,
        "and with it answered what is left is the Brief: no grilling and no \
         review is ever waited on",
    );
    assert!(
        !opened(&app, id).await.ready_to_grill,
        "which the button waits on too",
    );

    assert_eq!(
        write_brief(&app, id, "# Rate limiting\n\nThe API has none.\n").await,
        BriefSaved::Saved
    );

    assert!(
        opened(&app, id).await.ready_to_grill,
        "one role and a brief is the whole of what an Investigate waits on, \
         with the two pickers nobody drew never among them",
    );
    assert_eq!(grill(&app, id).await, GrillingStarted::Started);
}

/// And the press does the grill start's own sequence, landing Investigating: the
/// branch, the worktree, the frozen Brief and the move on the Timeline, with
/// neither a grilling nor a follow-up anywhere in it.
#[tokio::test]
async fn starting_an_investigate_lands_it_in_investigating() {
    let (elsewhere, dir, app, repo, repo_id) = workbench().await;
    let id = investigate(&app, elsewhere.path(), repo_id).await;

    assert_eq!(grill(&app, id).await, GrillingStarted::Started);

    let view = opened(&app, id).await;
    assert_eq!(view.state, Lifecycle::Investigating);
    assert_eq!(
        moves(&view),
        [Lifecycle::Investigating],
        "one move, straight past the state a Develop would have been grilled in",
    );

    assert_eq!(
        git(
            &repo,
            &[
                "rev-parse",
                "--verify",
                &format!("refs/heads/{}", view.branch)
            ]
        )
        .trim()
        .len(),
        40,
        "the branch is cut exactly as a grill start cuts one",
    );

    let worktree = view
        .worktree
        .expect("an investigation is worked in a Worktree: it writes probes and runs them");
    let path = PathBuf::from(&worktree.path);
    assert!(!worktree.missing);
    assert_eq!(path.parent(), Some(dir.path().join("worktrees").as_path()));
    assert!(
        worktrees(&repo).contains(&path.canonicalize().unwrap()),
        "and git has it registered: {:?}",
        worktrees(&repo),
    );
    assert_eq!(
        git(&path, &["symbolic-ref", "--short", "HEAD"]).trim(),
        view.branch,
        "checked out on the branch the work is on",
    );

    assert!(
        view.base_commit.is_some(),
        "with what it branched from settled, which is the rule resolving",
    );

    // And everything the press freezes is frozen: the Brief, the Pairing and the
    // Process, exactly as they are for a grilling that has started.
    assert_eq!(
        write_brief(&app, id, "# Something else\n").await,
        BriefSaved::NotDrafting
    );
    assert_eq!(
        pick_process(&app, id, Process::Develop).await,
        ProcessPicked::NotDrafting
    );
}

/// An Investigate checks its companions out as a grill start does, every one of
/// them on a branch of its own — an investigation may have to run something in
/// one to answer the question.
#[tokio::test]
async fn starting_an_investigate_checks_its_companions_out_too() {
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let askance = second_repo(&app, elsewhere.path(), "askance").await;

    let id = investigate(&app, elsewhere.path(), repo_id).await;
    assert_eq!(
        add_companion(&app, id, askance).await,
        CompanionAdded::Added
    );

    assert_eq!(grill(&app, id).await, GrillingStarted::Started);

    let view = opened(&app, id).await;
    assert_eq!(view.state, Lifecycle::Investigating);
    assert_eq!(view.companions.len(), 1);

    let worktree = view.companions[0]
        .worktree
        .as_ref()
        .expect("a companion is checked out when the Conversation's own is");
    assert!(!worktree.missing);
    assert!(PathBuf::from(&worktree.path).join("README.md").is_file());
}

/// The rule the workbench already states — the default branch's tip *at grill
/// start* — resolving for the first time.
#[tokio::test]
async fn starting_records_the_commit_the_work_branched_from() {
    let (elsewhere, _dir, app, repo, repo_id) = workbench().await;
    let id = ready(&app, elsewhere.path(), repo_id).await;

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

/// The branch the base resolved through is recorded beside the commit it
/// resolved to.
///
/// A sha cannot say what branch it came off, and what wants the name is the
/// commit sweep: a resolution session that merges the base branch in brings
/// every commit the base has gained with it, and none of that is the
/// Conversation's work. See the server's `commits` module.
#[tokio::test]
async fn starting_records_the_branch_the_base_resolved_through() {
    let (elsewhere, dir, app, repo, repo_id) = workbench().await;
    let id = ready(&app, elsewhere.path(), repo_id).await;

    git(&repo, &["branch", "release"]);
    assert_eq!(
        base(&app, id, Some("release")).await,
        BaseRecorded::Recorded
    );
    assert_eq!(grill(&app, id).await, GrillingStarted::Started);

    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    assert_eq!(
        store::load_conversation(&pool, id)
            .await
            .unwrap()
            .unwrap()
            .base_ref
            .as_deref(),
        Some("release"),
        "the branch they picked, rather than the commit it stood at",
    );
}

/// And where they picked none, it is the Repo's default branch as origin holds
/// it — which is the rule an unpicked base resolved by anyway.
#[tokio::test]
async fn starting_from_no_pick_records_the_default_branch() {
    let (elsewhere, dir, app, _repo, repo_id) = workbench().await;
    let id = ready(&app, elsewhere.path(), repo_id).await;

    assert_eq!(grill(&app, id).await, GrillingStarted::Started);

    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    assert_eq!(
        store::load_conversation(&pool, id)
            .await
            .unwrap()
            .unwrap()
            .base_ref
            .as_deref(),
        // No origin in this repository, so what origin holds is what it holds.
        Some("main"),
        "an unpicked base is the default branch, and the name says which",
    );
}

/// The picked branch is what the work branches from, and it is not the default
/// branch's tip.
#[tokio::test]
async fn the_picked_branch_is_what_the_branch_is_made_off() {
    let (elsewhere, _dir, app, repo, repo_id) = workbench().await;
    let id = ready(&app, elsewhere.path(), repo_id).await;

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
    let (elsewhere, _dir, app, repo, repo_id) = workbench().await;
    let id = ready(&app, elsewhere.path(), repo_id).await;

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
    let (elsewhere, _dir, app, repo, upstream, repo_id) = workbench_with_origin().await;
    let id = ready(&app, elsewhere.path(), repo_id).await;

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
    let (elsewhere, _dir, app, repo, upstream, repo_id) = workbench_with_origin().await;
    let id = ready(&app, elsewhere.path(), repo_id).await;

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
    let (elsewhere, _dir, app, repo, repo_id) = workbench().await;
    let id = ready(&app, elsewhere.path(), repo_id).await;

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
    let (elsewhere, dir, app, repo, _upstream, repo_id) = workbench_with_origin().await;
    let id = ready(&app, elsewhere.path(), repo_id).await;

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
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = started(&app, repo_id).await;
    write_brief(&app, id, "# Rate limiting\n").await;

    assert_eq!(grill(&app, id).await, GrillingStarted::NoGrillingProfile);

    choose(
        &app,
        id,
        "grilling",
        profile(&app, elsewhere.path(), "fable").await,
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
        profile(&app, elsewhere.path(), "opus").await,
    )
    .await;
    assert_eq!(grill(&app, id).await, GrillingStarted::NoReviewProfile);

    choose(
        &app,
        id,
        "review",
        profile(&app, elsewhere.path(), "haiku").await,
    )
    .await;
    assert_eq!(grill(&app, id).await, GrillingStarted::Started);
}

/// A Profile whose pair has gone is no account to run a session under, and the
/// pane says so — so pressing the button anyway has to say the same thing.
#[tokio::test]
async fn starting_is_refused_when_a_chosen_profiles_pair_has_gone() {
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = ready(&app, elsewhere.path(), repo_id).await;

    std::fs::remove_dir_all(elsewhere.path().join("fable")).unwrap();

    assert!(!opened(&app, id).await.ready_to_grill);
    assert_eq!(grill(&app, id).await, GrillingStarted::ProfileBroken);
}

/// The Brief is what the grilling starts from, and freezing an empty one would
/// freeze nothing worth having.
#[tokio::test]
async fn starting_is_refused_when_the_brief_is_empty() {
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = started(&app, repo_id).await;
    choose(
        &app,
        id,
        "grilling",
        profile(&app, elsewhere.path(), "fable").await,
    )
    .await;
    choose(
        &app,
        id,
        "implementation",
        profile(&app, elsewhere.path(), "opus").await,
    )
    .await;
    choose(
        &app,
        id,
        "review",
        profile(&app, elsewhere.path(), "haiku").await,
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
    let (elsewhere, _dir, app, repo, repo_id) = workbench().await;
    let id = ready(&app, elsewhere.path(), repo_id).await;

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
/// is somebody's work, and the name is one the human typed and meant.
#[tokio::test]
async fn starting_is_refused_when_the_named_branch_is_already_there() {
    let (elsewhere, _dir, app, repo, repo_id) = workbench().await;
    let id = ready(&app, elsewhere.path(), repo_id).await;

    assert_eq!(
        rename(&app, id, "rate-limiting").await,
        BranchRenamed::Renamed
    );
    git(&repo, &["branch", "rate-limiting"]);

    assert_eq!(grill(&app, id).await, GrillingStarted::BranchExists);

    let view = opened(&app, id).await;
    assert_eq!(view.state, Lifecycle::Draft);
    assert_eq!(view.branch, "rate-limiting", "and the name is still theirs");
}

/// And a name git will not give anybody, which is not the same question. It
/// keeps a branch as a file under `refs/heads/`, so a name with refs beneath it
/// is a name no branch can be cut at: a repository holding a stage's
/// `roadmaps/mvp/01-packaging` is one where `roadmaps` cannot be a branch, ever.
///
/// Refused here rather than left to the `git worktree add` further down, whose
/// complaint reaches the server log and leaves the human with nothing but *the
/// worktree could not be made* — the failure stage branches were moved under a
/// fixed component to stop happening, asked from the other side.
#[tokio::test]
async fn starting_is_refused_when_the_named_branch_is_a_path_git_will_not_give() {
    let (elsewhere, _dir, app, repo, repo_id) = workbench().await;
    let id = ready(&app, elsewhere.path(), repo_id).await;

    assert_eq!(rename(&app, id, "roadmaps").await, BranchRenamed::Renamed);
    git(&repo, &["branch", "roadmaps/mvp/01-packaging"]);

    assert_eq!(grill(&app, id).await, GrillingStarted::BranchExists);

    let view = opened(&app, id).await;
    assert_eq!(view.state, Lifecycle::Draft);
    assert_eq!(view.worktree, None);
    assert_eq!(worktrees(&repo).len(), 1, "only the repository itself");
}

/// But a name Verkstead invented is nobody's, so a repository that already has a
/// branch by it is a reason to invent another rather than a reason to refuse:
/// the human never saw that name and cannot have meant it.
#[tokio::test]
async fn a_start_invents_another_name_where_the_one_it_has_is_taken() {
    let (elsewhere, _dir, app, repo, repo_id) = workbench().await;
    let id = ready(&app, elsewhere.path(), repo_id).await;

    let carried = opened(&app, id).await.branch;
    git(&repo, &["branch", &carried]);

    assert_eq!(grill(&app, id).await, GrillingStarted::Started);

    let view = opened(&app, id).await;

    assert_ne!(view.branch, carried, "the work is on a name of its own");
    assert!(
        has_branch(&repo, &view.branch),
        "and that name is the branch that was cut",
    );

    // Still Verkstead's name and still the first session's to replace: what
    // happened here is a prefill being swapped for another prefill.
    assert!(!view.branch_named);
    assert!(view.naming);

    // And the work is where the record says it is, under the name it settled on.
    let worktree = PathBuf::from(view.worktree.expect("a start makes one").path);
    assert!(
        worktree.ends_with(format!("verkstead-{}", view.branch)),
        "the directory is named for the branch, not for the one it started with",
    );
    assert_eq!(
        git(&worktree, &["rev-parse", "--abbrev-ref", "HEAD"]).trim(),
        view.branch,
    );
}

/// The same question asked of every repository the invented name is about to be
/// cut in. A companion mirroring the Conversation's branch takes that name too,
/// so one already holding it is answered by picking again rather than by the
/// refusal a companion's own typed name would get.
#[tokio::test]
async fn a_start_invents_around_a_companion_that_holds_the_name() {
    let (elsewhere, _dir, app, repo, repo_id) = workbench().await;
    let companion = second_repo(&app, elsewhere.path(), "askance").await;
    let id = ready(&app, elsewhere.path(), repo_id).await;

    add_companion(&app, id, companion).await;
    companion_mode(&app, id, companion, CompanionMode::ReadWrite).await;

    let askance = elsewhere.path().join("askance");
    let carried = opened(&app, id).await.branch;

    // Free in the Conversation's own repository and taken in the companion's,
    // which is a name this start cannot use either.
    git(&askance, &["branch", &carried]);

    assert_eq!(grill(&app, id).await, GrillingStarted::Started);

    let view = opened(&app, id).await;

    assert_ne!(view.branch, carried);
    assert!(has_branch(&repo, &view.branch));
    assert!(
        has_branch(&askance, &view.branch),
        "the companion mirrors it, so it is cut there under the same name",
    );
}

/// And a name only the remote holds is picked around as well, though nothing
/// local is in the way of cutting it: the branch that would be pushed to it is
/// somebody's, and there is no shortage of other names.
#[tokio::test]
async fn a_start_invents_around_a_name_only_the_remote_holds() {
    let (elsewhere, _dir, app, repo, repo_id) = workbench().await;
    let id = ready(&app, elsewhere.path(), repo_id).await;

    let carried = opened(&app, id).await.branch;
    let tip = git(&repo, &["rev-parse", "HEAD"]).trim().to_owned();

    git(
        &repo,
        &[
            "update-ref",
            &format!("refs/remotes/origin/{carried}"),
            &tip,
        ],
    );

    assert_eq!(grill(&app, id).await, GrillingStarted::Started);

    let view = opened(&app, id).await;
    assert_ne!(view.branch, carried);
    assert!(has_branch(&repo, &view.branch));
    assert!(
        !has_branch(&repo, &carried),
        "and nothing was cut under the name it was carrying",
    );
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
    let (elsewhere, dir, app, _repo, repo_id) = workbench().await;
    let reading = second_repo(&app, elsewhere.path(), "askance").await;
    let writing = second_repo(&app, elsewhere.path(), "granit").await;
    let id = ready(&app, elsewhere.path(), repo_id).await;

    add_companion(&app, id, reading).await;
    add_companion(&app, id, writing).await;
    assert_eq!(
        companion_mode(&app, id, writing, CompanionMode::ReadWrite).await,
        CompanionModeChosen::Chosen
    );

    let askance = elsewhere.path().join("askance");
    let granit = elsewhere.path().join("granit");
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
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let reading = second_repo(&app, elsewhere.path(), "askance").await;
    let id = ready(&app, elsewhere.path(), repo_id).await;

    add_companion(&app, id, reading).await;

    let askance = elsewhere.path().join("askance");

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
        let (elsewhere, dir, app, repo, repo_id) = workbench().await;
        let companion = second_repo(&app, elsewhere.path(), "askance").await;
        let id = ready(&app, elsewhere.path(), repo_id).await;

        add_companion(&app, id, companion).await;

        let askance = elsewhere.path().join("askance");

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
                why: why.clone(),
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

/// And a branch of that repository standing where a component of the companion's
/// own branch path goes, which is refused by name — both names, the repository
/// and the branch.
///
/// A stage's branch is mirrored into its read-write companions whole,
/// `roadmaps/` and all, so the one collision that scheme leaves behind is the
/// companion's to have too. Left to git it is *git would not make its checkout*
/// and a line in the server log, which is the refusal this one exists to be
/// instead.
#[tokio::test]
async fn a_companion_with_a_branch_in_the_way_refuses_the_start_by_both_names() {
    for blocker in ["roadmaps", "roadmaps/mvp"] {
        let (elsewhere, _dir, app, repo, repo_id) = workbench().await;
        let companion = second_repo(&app, elsewhere.path(), "askance").await;
        let id = ready(&app, elsewhere.path(), repo_id).await;

        add_companion(&app, id, companion).await;
        companion_mode(&app, id, companion, CompanionMode::ReadWrite).await;
        assert_eq!(
            companion_branch(&app, id, companion, "roadmaps/mvp/01-packaging").await,
            CompanionBranchRenamed::Renamed
        );

        let askance = elsewhere.path().join("askance");
        git(&askance, &["branch", blocker]);

        assert_eq!(
            grill(&app, id).await,
            GrillingStarted::Companion {
                repo: "askance".to_owned(),
                why: CompanionRefusal::BranchInTheWay {
                    by: blocker.to_owned(),
                },
            },
        );

        // And nothing made anywhere on the way to finding out, the same as every
        // other question asked before the making.
        let view = opened(&app, id).await;
        assert_eq!(view.state, Lifecycle::Draft, "{blocker}");
        assert_eq!(view.worktree, None, "{blocker}");
        assert_eq!(view.companions[0].worktree, None, "{blocker}");
        assert!(!has_branch(&repo, &view.branch), "{blocker}");
        assert_eq!(worktrees(&repo).len(), 1, "only the repository itself");
        assert_eq!(worktrees(&askance).len(), 1, "and only the companion");
    }
}

/// A start refused over the *last* companion leaves nothing behind either — not
/// the checkouts already made, and not the branches they were cut on.
///
/// Which is the case asking every question first cannot cover: this one gets
/// past the asking, because what git refuses is the making. `feature` is a name
/// no branch answers to and git will still not take, `feature/x` being a ref
/// beneath the file it would have to be — and it is the one half of that
/// collision nothing asks about, the other being
/// [`CompanionRefusal::BranchInTheWay`].
#[tokio::test]
async fn a_start_refused_over_a_companion_unmakes_the_checkouts_it_had_made() {
    let (elsewhere, dir, app, repo, repo_id) = workbench().await;
    let first = second_repo(&app, elsewhere.path(), "askance").await;
    let last = second_repo(&app, elsewhere.path(), "granit").await;
    let id = ready(&app, elsewhere.path(), repo_id).await;

    for companion in [first, last] {
        add_companion(&app, id, companion).await;
        companion_mode(&app, id, companion, CompanionMode::ReadWrite).await;
    }

    let askance = elsewhere.path().join("askance");
    let granit = elsewhere.path().join("granit");

    git(&granit, &["branch", "feature/x"]);
    assert_eq!(
        companion_branch(&app, id, last, "feature").await,
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

/// A task list a stage left part way through, committed on the branch a
/// Conversation is about to be cut from.
const INHERITED: &str = "\
# Grant filters

What the stage before this one was part way through.

## Tasks

- [x] 01: The first task — [details](01-the-first-task.md)
- [ ] 02: The second task — [details](02-the-second-task.md)
";

/// Put `list` on `repo`'s checked-out branch, with the task file the entry that
/// is done names — which is what a backlog somebody stopped working looks like.
fn task_list(repo: &Path, list: &str) {
    let tasks = repo.join(".tasks");

    std::fs::create_dir_all(&tasks).unwrap();
    std::fs::write(tasks.join("TODO.md"), list).unwrap();
    std::fs::write(tasks.join("01-the-first-task.md"), "# 01. The first task\n").unwrap();

    git(repo, &["add", "-A"]);
    git(repo, &["commit", "-m", "chore: plan the work"]);
}

/// What a Worktree's branch has at its tip: the subject, and who the commit is
/// by.
fn tip(worktree: &Path) -> Vec<String> {
    git(worktree, &["log", "-1", "--format=%s%n%an%n%ae"])
        .lines()
        .map(str::to_owned)
        .collect()
}

/// A branch cut from a base that already carries a task list arrives without
/// one, and the removal is a commit of its own by the configured author.
///
/// The whole of what this is for: the watcher that ends a planning session
/// checks that `.tasks/TODO.md` is at the tip and committed, and cannot tell an
/// inherited list from one this branch wrote — so a session would be ended
/// before it had asked anything.
#[tokio::test]
async fn starting_clears_the_task_list_the_base_carried() {
    let (elsewhere, _dir, app, repo, repo_id) = workbench().await;
    task_list(&repo, INHERITED);

    let base = git(&repo, &["rev-parse", "HEAD"]).trim().to_owned();
    let id = ready(&app, elsewhere.path(), repo_id).await;

    assert_eq!(grill(&app, id).await, GrillingStarted::Started);

    let view = opened(&app, id).await;
    let worktree = PathBuf::from(view.worktree.clone().expect("a start makes one").path);

    assert!(
        !worktree.join(".tasks").exists(),
        "the inherited list is gone from the tree the session will work in",
    );
    assert_eq!(
        tip(&worktree),
        [
            "chore: clear the task list inherited from main",
            "Verkstead Test",
            "test@verkstead.invalid",
        ],
        "as a commit of its own, by the configured git author",
    );
    assert_eq!(
        git(&worktree, &["log", "--format=%H"]).lines().count(),
        3,
        "one commit on top of the base, and no more",
    );
    assert_eq!(
        view.base_commit.as_deref(),
        Some(base.as_str()),
        "and the recorded base is still the commit the branch came off",
    );
    assert_eq!(
        git(&repo, &["rev-parse", "refs/heads/main"]).trim(),
        base,
        "the base branch itself is untouched: the clearing is this branch's",
    );

    let said = notices(&view).join("\n");

    assert!(
        said.contains("<code>main</code>") && said.contains("<strong>Grant filters</strong>"),
        "the Timeline says which base carried which list: {said:?}",
    );
    assert!(
        said.contains("1 of 2 entries still open"),
        "and how far through it was: {said:?}",
    );
}

/// A base carrying no list is started with nothing extra: no commit and no
/// notice. The ordinary start is every start, and it must arrive holding
/// exactly what its base held.
#[tokio::test]
async fn a_base_with_no_task_list_is_started_untouched() {
    let (elsewhere, _dir, app, repo, repo_id) = workbench().await;

    let base = git(&repo, &["rev-parse", "HEAD"]).trim().to_owned();
    let id = ready(&app, elsewhere.path(), repo_id).await;

    assert_eq!(grill(&app, id).await, GrillingStarted::Started);

    let view = opened(&app, id).await;
    let worktree = PathBuf::from(view.worktree.clone().expect("a start makes one").path);

    assert_eq!(
        git(&worktree, &["rev-parse", "HEAD"]).trim(),
        base,
        "the branch stands where it was cut",
    );
    assert_eq!(
        notices(&view),
        Vec::<String>::new(),
        "and the Timeline has nothing to say about a list there never was",
    );
}

/// With no git author configured there is nobody to commit the clearing as, so
/// the press is refused by name before anything is made.
///
/// Refused whether or not the base carries a list — this one does not — because
/// onboarding collects an author: a start without one is a misconfiguration to
/// name rather than a case to work around.
#[tokio::test]
async fn starting_with_no_git_author_is_refused_by_name() {
    let (elsewhere, dir, app, repo, repo_id) = workbench().await;
    no_author(&dir);

    let id = ready(&app, elsewhere.path(), repo_id).await;
    let branch = opened(&app, id).await.branch;

    assert_eq!(grill(&app, id).await, GrillingStarted::NoGitAuthor);

    let view = opened(&app, id).await;

    assert_eq!(view.state, Lifecycle::Draft, "the press did not happen");
    assert_eq!(view.worktree, None);
    assert!(
        !has_branch(&repo, &branch),
        "and no branch was cut for it either",
    );
    assert_eq!(
        worktrees(&repo).len(),
        1,
        "only the repository itself: {:?}",
        worktrees(&repo),
    );
}
/// Pressing Steer stops the drive, writes the pending steer, and leaves the
/// Conversation stopped for as long as the form stands.
///
/// The press is an act of its own rather than the first half of the submit:
/// nothing new launches while the human composes, so the world the form is
/// written against is the world the submit arrives in. What it leaves behind is
/// the pending steer — the item at the end of the Timeline, whose details pane
/// is the form — and a Conversation with Resume drawn on it.
///
/// Nothing is running in these fixtures, so the press stops the run where it
/// stands and says as much: what **Interrupt current task** is offered against
/// is a session, and there is none.
#[tokio::test]
async fn pressing_steer_stops_the_drive_and_opens_a_pending_steer() {
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = ready(&app, elsewhere.path(), repo_id).await;
    assert_eq!(grill(&app, id).await, GrillingStarted::Started);

    assert_eq!(
        steer(&app, id).await,
        SteerOpened::Opened,
        "the form opens with nothing to interrupt behind it",
    );

    let view = opened(&app, id).await;

    assert_eq!(view.state, Lifecycle::Grilling, "the press moves nothing");
    assert!(
        view.blocked_on.is_some(),
        "the drive has stopped, and the Notice saying so is what the badge points at",
    );
    assert!(
        view.ready_to_resume,
        "so the one press that undoes a press nobody followed up is drawn on it",
    );
    assert!(
        !view.ready_to_stop,
        "and there is nothing left to stop: the press already did",
    );
    assert_eq!(
        steered(&view),
        [("moved", Lifecycle::Grilling)],
        "and nothing was steered, so nothing on the record says it was",
    );

    let pending = view
        .pending_steer
        .expect("the press wrote the form's own row");

    assert!(!pending.at.is_empty(), "it says when the press was made");
    assert_eq!(
        pending.form,
        verkstead_render::SteerForm::default(),
        "and the form is empty: nothing picked, nothing written, nothing ticked",
    );
}

/// A second press makes nothing and reports the one there is.
///
/// A Conversation already carrying a half-written form is not one to start
/// another beside, so the press says where that form is and the page goes to
/// it. Nothing is stopped a second time either — there is nothing left to stop
/// — and the row is left exactly as it stands, down to when it was opened.
#[tokio::test]
async fn a_second_press_on_steer_finds_the_one_there_is() {
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = ready(&app, elsewhere.path(), repo_id).await;
    assert_eq!(grill(&app, id).await, GrillingStarted::Started);

    assert_eq!(steer(&app, id).await, SteerOpened::Opened);

    let opened_at = opened(&app, id).await.pending_steer.unwrap().at;

    assert_eq!(
        steer(&app, id).await,
        SteerOpened::Opened,
        "the second press answers as the first did: there is a form to go to",
    );

    let view = opened(&app, id).await;

    assert_eq!(
        view.pending_steer
            .as_ref()
            .map(|pending| pending.at.clone()),
        Some(opened_at),
        "and it is the same row, rather than a second one written over it",
    );
    assert_eq!(
        steered(&view),
        [("moved", Lifecycle::Grilling)],
        "neither press put anything on the record",
    );
}

/// Cancel takes the pending steer away and leaves the Conversation stopped.
///
/// The press is what froze it, so unfreezing is Resume's to do: Cancel decides
/// nothing about the work, and posts nothing to the Timeline — a steer that
/// decided nothing is no Event, and the stop's own Notice already says a human
/// pressed.
///
/// A second cancel is the same answer. One landing behind a submit or behind
/// another device's cancel has got what it asked for.
#[tokio::test]
async fn cancelling_takes_the_pending_steer_away_and_leaves_it_stopped() {
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = ready(&app, elsewhere.path(), repo_id).await;
    assert_eq!(grill(&app, id).await, GrillingStarted::Started);

    assert_eq!(steer(&app, id).await, SteerOpened::Opened);
    assert_eq!(cancel_steer(&app, id).await, SteerCancelled::Cancelled);

    let view = opened(&app, id).await;

    assert!(
        view.pending_steer.is_none(),
        "the form is gone, so nothing is drawn at the end of the Timeline",
    );
    assert_eq!(view.state, Lifecycle::Grilling, "and nothing moved");
    assert!(
        view.ready_to_resume,
        "the Conversation is left stopped, with Resume on offer",
    );
    assert_eq!(
        steered(&view),
        [("moved", Lifecycle::Grilling)],
        "and a steer that decided nothing is no Event",
    );

    assert_eq!(
        cancel_steer(&app, id).await,
        SteerCancelled::Cancelled,
        "a cancel with nothing to cancel has got what it asked for",
    );
    assert_eq!(
        cancel_steer(&app, 404).await,
        SteerCancelled::NoSuchConversation,
        "and a Conversation that is not there is refused by name",
    );
}

/// And the submit takes it away with the record it became: the Steer and the
/// Moved line land, and the form they were filled in on is gone from the view.
#[tokio::test]
async fn submitting_takes_the_pending_steer_away_with_the_record() {
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = ready(&app, elsewhere.path(), repo_id).await;
    assert_eq!(grill(&app, id).await, GrillingStarted::Started);

    assert_eq!(steer(&app, id).await, SteerOpened::Opened);
    assert_eq!(
        steer_into(&app, id, "Done", false).await,
        ConversationSteered::Steered,
    );

    let view = opened(&app, id).await;

    assert!(
        view.pending_steer.is_none(),
        "the form is the record now, so there is nothing left at the end of the Timeline",
    );
    assert_eq!(
        steered(&view).last(),
        Some(&("moved", Lifecycle::Done)),
        "and the pair the steer leaves is on the record",
    );
}

/// And what it became is the whole form: the ticks, the Pairing picked and the
/// companion rows asked for, beside the target and body the Event has always
/// carried.
///
/// Which is what makes the pane a steer opens the form read back rather than a
/// sentence about it. Read off the Conversation's own view, because that is
/// where the page reads it — the Profile as a row, named the way the picker
/// names it, and each repository by name.
#[tokio::test]
async fn the_record_a_submit_leaves_is_the_whole_form() {
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let askance = second_repo(&app, elsewhere.path(), "askance").await;
    let id = ready(&app, elsewhere.path(), repo_id).await;
    let picked = profile(&app, elsewhere.path(), "steering").await;

    assert_eq!(grill(&app, id).await, GrillingStarted::Started);

    assert_eq!(steer(&app, id).await, SteerOpened::Opened);

    let steered: ConversationSteered = post(
        &app,
        &format!("/api/ui/conversations/{id}/steer/submit"),
        &serde_json::json!({
            "target": "Grilling",
            "interrupt": true,
            "digest": true,
            "pairing": { "profile_id": picked, "model": "claude-opus-5" },
            "added": [alongside(askance, "ReadOnly")],
            "upgraded": [],
        }),
    )
    .await;

    assert_eq!(steered, ConversationSteered::Steered);

    let view = opened(&app, id).await;
    let record = view
        .timeline
        .iter()
        .find_map(|event| match event {
            TimelineEvent::Steer(steer) => steer.record.clone(),
            _ => None,
        })
        .expect("the Steer Event carries the form it was made with");

    assert!(record.digest, "the tick that primes the round it opened");
    assert!(record.interrupt);

    let SteerPairingView::Under(paired) = record.pairing else {
        panic!("the account the picker was on is on the record");
    };

    assert_eq!(paired.profile.id, picked);
    assert_eq!(paired.model.as_deref(), Some("claude-opus-5"));

    assert_eq!(record.added.len(), 1);
    assert_eq!(record.added[0].repo, "askance");
    assert_eq!(record.added[0].mode, CompanionMode::ReadOnly);
    assert_eq!(
        record.added[0].base_ref, None,
        "the rule the row was left on: that repository's own default branch",
    );

    assert!(record.upgraded.is_empty(), "it opened nothing up");
}

/// Resume is the opposite decision, so a press that starts something takes the
/// pending steer with it.
///
/// The form was written against a run that had stopped, and this is the run
/// starting again: a form left standing under it would be written against a
/// world that has gone, and the item drawn for it would go on saying the drive
/// had stopped while a session worked in the Worktree. Somebody who wants both
/// cancels the steer.
#[tokio::test]
async fn resuming_discards_the_pending_steer_it_starts_over() {
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = ready(&app, elsewhere.path(), repo_id).await;
    assert_eq!(grill(&app, id).await, GrillingStarted::Started);

    assert_eq!(steer(&app, id).await, SteerOpened::Opened);
    assert_eq!(resume(&app, id).await, Resumed::Resumed);

    let view = opened(&app, id).await;

    assert!(
        view.pending_steer.is_none(),
        "the form goes with the decision that replaced it",
    );
    assert!(
        view.blocked_on.is_none(),
        "and the stop the press made goes too, which is what Resume is",
    );
    assert!(
        !view.waiting,
        "so nothing about it is waiting on the human any more",
    );
    assert_eq!(
        steered(&view),
        [("moved", Lifecycle::Grilling)],
        "and a steer nobody submitted is still no Event",
    );
}

/// And a Resume refused by name leaves it exactly where it stands.
///
/// The press decided nothing, so neither did it decide anything about the form:
/// a human who is told the account their work grills under has gone is being
/// pointed back at the steer, and a steer thrown away on the way would be the
/// workbench taking the answer with the question.
#[tokio::test]
async fn a_refused_resume_leaves_the_pending_steer() {
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = started(&app, repo_id).await;

    let grilling = profile(&app, elsewhere.path(), "fable").await;
    let implementation = profile(&app, elsewhere.path(), "opus").await;
    let review = profile(&app, elsewhere.path(), "haiku").await;
    choose(&app, id, "grilling", grilling).await;
    choose(&app, id, "implementation", implementation).await;
    choose(&app, id, "review", review).await;
    assert_eq!(
        write_brief(&app, id, "# Rate limiting\n\nThe API has none.\n").await,
        BriefSaved::Saved
    );
    assert_eq!(grill(&app, id).await, GrillingStarted::Started);

    assert_eq!(steer(&app, id).await, SteerOpened::Opened);
    assert_eq!(
        save_steer(
            &app,
            id,
            &serde_json::json!({
                "target": "Implementing",
                "instruction": "Take the modal out",
                "digest": false,
                "interrupt": false,
                "added": [],
                "upgraded": [],
            }),
        )
        .await,
        SteerSaved::Saved,
    );

    // The account this Conversation grills under, taken away: the one thing
    // that makes Resume refuse a Conversation there is otherwise nothing wrong
    // with, and the refusal that sends the human back to the steer.
    assert_eq!(
        remove_profile(&app, grilling).await,
        verkstead_render::ProfileDeleted::Removed,
    );

    assert_eq!(resume(&app, id).await, Resumed::NoGrillingPairing);

    let view = opened(&app, id).await;
    let pending = view
        .pending_steer
        .expect("the refusal left the form where it stands");

    assert_eq!(
        pending.form.instruction.as_deref(),
        Some("Take the modal out"),
        "down to what had been written into it",
    );
    assert!(
        view.blocked_on.is_some(),
        "and the Conversation is still stopped, nothing having changed",
    );
}

/// Close takes the pending steer away in the act that closes the Conversation.
///
/// Closing takes away every session there will ever be, the way it shuts every
/// Question Set it finds open: a form asking where the work goes next is a form
/// about work that is over.
#[tokio::test]
async fn closing_takes_the_pending_steer_away() {
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = ready(&app, elsewhere.path(), repo_id).await;
    assert_eq!(grill(&app, id).await, GrillingStarted::Started);

    assert_eq!(steer(&app, id).await, SteerOpened::Opened);
    assert_eq!(close(&app, id).await, ConversationClosed::Closed);

    let view = opened(&app, id).await;

    assert_eq!(view.state, Lifecycle::Closed);
    assert!(
        view.pending_steer.is_none(),
        "the form is gone with the work it was about",
    );
    assert!(!view.waiting, "so nothing is left waiting on anybody",);
}

/// And the two stops leave it exactly where it stands.
///
/// Both are about the run rather than about the move: the press that opened the
/// form already stopped the drive, and neither of these decides anything about
/// where the work is headed. A Stop that took the form with it would be a
/// second press undoing the first one's work.
#[tokio::test]
async fn the_stops_leave_the_pending_steer_alone() {
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = ready(&app, elsewhere.path(), repo_id).await;
    assert_eq!(grill(&app, id).await, GrillingStarted::Started);

    assert_eq!(steer(&app, id).await, SteerOpened::Opened);

    // Already stopped by the press, which is what both of these answer here —
    // and the point: neither press is a second decision about the form.
    assert_eq!(stop(&app, id).await, ConversationStopped::AlreadyStopped);
    assert!(
        opened(&app, id).await.pending_steer.is_some(),
        "Stop is about the run, so the form is left where it stands",
    );

    assert_eq!(
        force_stop(&app, id).await,
        ConversationStopped::AlreadyStopped
    );
    assert!(
        opened(&app, id).await.pending_steer.is_some(),
        "and so is Force stop",
    );
}

/// A Conversation with a pending steer waits on the human, which is the one
/// rule the sidebar's disc and the status button's word are both read from.
///
/// The press stopped the drive and nothing starts again until they submit or
/// cancel, so the Conversation is theirs to finish. The stop it made is their
/// own press and says nothing in the marks by itself — so without this a form
/// left half written would read as quiet from the sidebar, which is the one
/// place somebody who left it yesterday will look.
#[tokio::test]
async fn a_pending_steer_reads_as_waiting_on_the_human() {
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = ready(&app, elsewhere.path(), repo_id).await;
    assert_eq!(grill(&app, id).await, GrillingStarted::Started);

    let row = |rows: Vec<ConversationEntry>| {
        rows.into_iter()
            .find(|row| row.id == id)
            .expect("the Conversation is on the sidebar's list")
    };

    assert!(
        !row(sidebar(&app).await).waiting,
        "nothing is waiting on anybody before the press",
    );

    assert_eq!(steer(&app, id).await, SteerOpened::Opened);

    let view = opened(&app, id).await;

    assert!(
        view.waiting,
        "the form is theirs to finish, so the page says so",
    );
    assert!(
        view.stopped_by_hand,
        "and the stop under it is their own press rather than a mark of its own",
    );
    assert!(
        row(sidebar(&app).await).waiting,
        "and the sidebar's row carries the disc, the two being one rule",
    );

    assert_eq!(cancel_steer(&app, id).await, SteerCancelled::Cancelled);
    assert!(
        !opened(&app, id).await.waiting,
        "and it goes with the form rather than outliving it",
    );
}

/// The form is written onto the pending steer and read back off the
/// Conversation, whole: the pane is prefilled from it, so a human who left the
/// item half written finds it as they left it on whatever device they pick up.
///
/// Every field, because a save carries every field. What is asked here is that
/// none of them is dropped on the way through — the target's two vocabularies,
/// the ticks, the Pairing's two halves and both halves of the companion
/// section.
#[tokio::test]
async fn the_form_is_saved_onto_the_pending_steer_and_read_back() {
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = ready(&app, elsewhere.path(), repo_id).await;
    assert_eq!(grill(&app, id).await, GrillingStarted::Started);

    // A second repository, so that the companion rows name something other than
    // the Conversation's own: the save writes what it is told, and which repos a
    // steer could actually take is the submit's to refuse.
    let askance = repository(elsewhere.path().join("askance"));
    let registered: Registered = post(
        &app,
        "/api/ui/repos",
        &serde_json::json!({ "path": askance }),
    )
    .await;
    assert!(matches!(registered, Registered::Added(_)));

    let alongside_id = get::<Vec<RepoEntry>>(&app, "/api/ui/repos")
        .await
        .into_iter()
        .find(|entry| entry.name == "askance")
        .expect("both are registered")
        .id;

    let running = profile(&app, elsewhere.path(), "sonnet").await;

    assert_eq!(steer(&app, id).await, SteerOpened::Opened);

    let form = serde_json::json!({
        "target": "Implementing",
        "brief": "# Retries\n",
        "digest": true,
        "instruction": "Rebase this onto main.",
        "follow_up": "Does it count the 429s it sends?",
        "investigation": "Where does the count come from?",
        "pairing": { "profile_id": running, "model": "claude-opus-5" },
        "interrupt": true,
        "added": [{
            "repo_id": alongside_id,
            "mode": "ReadWrite",
            "base_ref": "trunk",
            "branch": "alongside",
        }],
        "upgraded": [{ "repo_id": repo_id, "branch": "opened" }],
    });

    assert_eq!(save_steer(&app, id, &form).await, SteerSaved::Saved);

    let pending = opened(&app, id)
        .await
        .pending_steer
        .expect("the form is still being written");

    assert_eq!(
        serde_json::to_value(&pending.form).unwrap(),
        form,
        "the form comes back off the Conversation as it was saved",
    );

    // And a second save is the whole form again rather than an edit of the row:
    // what is unticked has to leave, and there is no second call to notice it
    // went.
    let emptied = serde_json::json!({
        "target": serde_json::Value::Null,
        "brief": serde_json::Value::Null,
        "digest": false,
        "instruction": serde_json::Value::Null,
        "follow_up": serde_json::Value::Null,
        "investigation": serde_json::Value::Null,
        "pairing": serde_json::Value::Null,
        "interrupt": false,
        "added": [],
        "upgraded": [],
    });

    assert_eq!(save_steer(&app, id, &emptied).await, SteerSaved::Saved);

    let pending = opened(&app, id).await.pending_steer.unwrap();

    assert_eq!(
        serde_json::to_value(&pending.form).unwrap(),
        emptied,
        "a form emptied is a form with nothing on it, rows and all",
    );
    assert!(
        !pending.at.is_empty(),
        "and when the press was made is not the form's to move",
    );
}

/// A save with nothing to save into is refused by name, and the field stops for
/// good.
///
/// Two refusals rather than one, because what the human should go and look at
/// differs: a Conversation that is gone, and a pending steer somebody submitted
/// or cancelled from another device — which is much the commoner of the two, and
/// is the one thing that can happen to a form sitting open for an afternoon.
#[tokio::test]
async fn a_save_with_no_pending_steer_behind_it_is_refused_by_name() {
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = ready(&app, elsewhere.path(), repo_id).await;
    assert_eq!(grill(&app, id).await, GrillingStarted::Started);

    let form = serde_json::json!({ "target": "Done" });

    assert_eq!(
        save_steer(&app, id, &form).await,
        SteerSaved::NoPendingSteer,
        "nobody has pressed Steer, so there is no form to save",
    );
    assert_eq!(
        save_steer(&app, 404, &form).await,
        SteerSaved::NoSuchConversation,
    );

    assert_eq!(steer(&app, id).await, SteerOpened::Opened);
    assert_eq!(save_steer(&app, id, &form).await, SteerSaved::Saved);

    // And the cancel takes the row away, which is what a second device doing it
    // looks like from here: the next save has nothing to land on.
    assert_eq!(cancel_steer(&app, id).await, SteerCancelled::Cancelled);
    assert_eq!(
        save_steer(&app, id, &form).await,
        SteerSaved::NoPendingSteer,
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
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = ready(&app, elsewhere.path(), repo_id).await;
    assert_eq!(grill(&app, id).await, GrillingStarted::Started);

    assert_eq!(steer(&app, id).await, SteerOpened::Opened);
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
/// The click finds nothing to stop there and opens the form anyway. Nothing was
/// driving a draft, so there is no drive to stop and nothing about that is a
/// refusal.
#[tokio::test]
async fn a_draft_is_somewhere_to_steer_from_too() {
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = ready(&app, elsewhere.path(), repo_id).await;

    assert_eq!(steer(&app, id).await, SteerOpened::Opened);

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
/// And the round it opens is opened with the brief they typed on the form: a
/// second Brief beside the draft's own, frozen where it lands, because the round
/// it belongs to has no Draft to leave.
#[tokio::test]
async fn steering_a_draft_into_grilling_makes_its_branch_and_worktree() {
    let (elsewhere, dir, app, repo, repo_id) = workbench().await;
    let id = ready(&app, elsewhere.path(), repo_id).await;

    let tip = git(&repo, &["rev-parse", "HEAD"]).trim().to_owned();

    assert_eq!(steer(&app, id).await, SteerOpened::Opened);
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
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = ready(&app, elsewhere.path(), repo_id).await;

    assert_eq!(steer(&app, id).await, SteerOpened::Opened);
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
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = started(&app, repo_id).await;

    let grilling = profile(&app, elsewhere.path(), "fable").await;
    let implementation = profile(&app, elsewhere.path(), "opus").await;
    let review = profile(&app, elsewhere.path(), "haiku").await;
    choose(&app, id, "grilling", grilling).await;
    choose(&app, id, "implementation", implementation).await;
    choose(&app, id, "review", review).await;

    assert_eq!(steer(&app, id).await, SteerOpened::Opened);
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

    // The same steer with the round's Brief written on the form, which is what
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

/// A Conversation whose Agent Profile was removed out from under it is rescued
/// by a steer, which is the whole of what removing one costs.
///
/// The steer is refused with nothing picked, because a Conversation with no
/// account settled is one nothing could be started in — and that refusal is the
/// form asking for the account, so the same submit carrying one goes through
/// and the work runs under it. Nothing else about the Conversation moved: it is
/// where it was, on the branch it was on, with the round it was in.
#[tokio::test]
async fn a_conversation_whose_profile_was_removed_is_steered_back_onto_another() {
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = grilling(&app, elsewhere.path(), repo_id).await;

    let interviewing = opened(&app, id)
        .await
        .grilling_pairing
        .as_ref()
        .expect("the fixture picks one per role")
        .profile
        .id;

    let removed: verkstead_render::ProfileDeleted = post(
        &app,
        &format!("/api/ui/profiles/{interviewing}/delete"),
        &serde_json::json!({}),
    )
    .await;
    assert_eq!(removed, verkstead_render::ProfileDeleted::Removed);

    assert_eq!(
        opened(&app, id).await.grilling_pairing,
        None,
        "the role that named it has nothing settled for it any more",
    );

    assert_eq!(steer(&app, id).await, SteerOpened::Opened);
    assert_eq!(
        steer_grilling(&app, id, None).await,
        ConversationSteered::NoPairing,
        "a state a session runs in needs one, and the form is where it is picked",
    );

    let rescue = profile(&app, elsewhere.path(), "rescue").await;

    let steered: ConversationSteered = post(
        &app,
        &format!("/api/ui/conversations/{id}/steer/submit"),
        &serde_json::json!({
            "target": "Grilling",
            "interrupt": false,
            "pairing": { "profile_id": rescue, "model": "claude-opus-5" },
        }),
    )
    .await;
    assert_eq!(steered, ConversationSteered::Steered);

    let view = opened(&app, id).await;

    assert_eq!(
        view.grilling_pairing
            .as_ref()
            .expect("the steer settled the role that had lost its account")
            .profile
            .id,
        rescue,
    );
    assert_eq!(view.state, Lifecycle::Grilling);
}

/// The Pairing picked on the form for a steer into Grilling is the *grilling*
/// one, and it is recorded as the Conversation's own.
///
/// Which role follows the target, an interview running under the one and
/// everything that builds running under the other — and the roles not steered
/// into are nobody's to re-settle here.
#[tokio::test]
async fn steering_into_grilling_settles_the_grilling_pairing() {
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = ready(&app, elsewhere.path(), repo_id).await;

    let picked = profile(&app, elsewhere.path(), "steering").await;
    let building = opened(&app, id)
        .await
        .implementation_pairing
        .expect("the fixture picks one per role");

    assert_eq!(steer(&app, id).await, SteerOpened::Opened);

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
        .as_ref()
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

/// A wrap-up both builds and reviews, and the one Pairing picked for a steer
/// into Wrapping settles what builds — leaving a review account the human chose
/// exactly where it is.
///
/// The picker is labelled for the state's own work and opens on what builds, so
/// somebody who steers a wrap-up and changes nothing has said nothing about the
/// review. Writing that prefill over an account they picked on the setup card to
/// be a fresh set of eyes would undo the whole reason for picking it apart.
#[tokio::test]
async fn steering_into_wrapping_leaves_a_review_account_the_human_chose_alone() {
    let (elsewhere, dir, app, _repo, repo_id) = workbench().await;
    let id = grilling(&app, elsewhere.path(), repo_id).await;
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

    let picked = profile(&app, elsewhere.path(), "steering").await;
    let before = opened(&app, id).await;
    let interviewing = before
        .grilling_pairing
        .as_ref()
        .cloned()
        .expect("the fixture picks one per role");
    let reviewing = before
        .review_pairing
        .pairing()
        .cloned()
        .expect("and one of them is an account of its own for the review");

    assert_eq!(steer(&app, id).await, SteerOpened::Opened);

    let steered: ConversationSteered = post(
        &app,
        &format!("/api/ui/conversations/{id}/steer/submit"),
        &serde_json::json!({
            "target": "Wrapping",
            "interrupt": false,
            "pairing": { "profile_id": picked, "model": "claude-opus-5" },
        }),
    )
    .await;

    assert_eq!(steered, ConversationSteered::Steered);

    let view = opened(&app, id).await;

    assert_eq!(
        view.implementation_pairing
            .map(|pairing| pairing.profile.id),
        Some(picked),
        "what the fixes run under",
    );
    assert_eq!(
        view.review_pairing
            .pairing()
            .map(|pairing| pairing.profile.id),
        Some(reviewing.profile.id),
        "and the account chosen to review is still the one that reviews",
    );
    assert_eq!(
        view.grilling_pairing
            .as_ref()
            .map(|pairing| pairing.profile.id),
        Some(interviewing.profile.id),
        "and the role nothing wraps under is exactly where it was",
    );
}

/// And where nothing was ever picked to review, the same pick fills it — which
/// is what lets a Conversation that never fixed a review Pairing be steered into
/// a wrap-up at all.
///
/// A steered Draft is how one gets here: Implementing runs one role and settles
/// the one it runs, so the review is still unpicked by the time the work is on a
/// pull request. Filling is not replacing — there was no choice to undo.
#[tokio::test]
async fn steering_into_wrapping_fills_a_review_nobody_picked_an_account_for() {
    let (elsewhere, dir, app, _repo, repo_id) = workbench().await;
    let id = started(&app, repo_id).await;

    // Everything but the review, which is what a Conversation steered out of
    // Draft has: the pickers freeze at the move, and Implementing settles only
    // what builds.
    let building = profile(&app, elsewhere.path(), "opus").await;
    choose(
        &app,
        id,
        "grilling",
        profile(&app, elsewhere.path(), "fable").await,
    )
    .await;
    choose(&app, id, "implementation", building).await;

    assert_eq!(steer(&app, id).await, SteerOpened::Opened);

    let steered: ConversationSteered = post(
        &app,
        &format!("/api/ui/conversations/{id}/steer/submit"),
        &serde_json::json!({
            "target": "Implementing",
            "interrupt": false,
            "instruction": "Add the rate limiter.",
        }),
    )
    .await;

    assert_eq!(steered, ConversationSteered::Steered);
    assert_eq!(
        opened(&app, id).await.review_pairing,
        PickedView::Nothing,
        "the move settled what builds and left the review unpicked",
    );

    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    assert_eq!(
        store::record_pull_request(
            &pool,
            id,
            repo_id,
            &store::PullRequest {
                number: 42,
                title: "Rate limiting".to_owned(),
                url: "https://github.com/tobico/verkstead/pull/42".to_owned(),
                repo: None,
            },
        )
        .await
        .unwrap(),
        store::Wrapping::Started,
    );

    pool.close().await;

    assert_eq!(steer(&app, id).await, SteerOpened::Opened);

    let steered: ConversationSteered = post(
        &app,
        &format!("/api/ui/conversations/{id}/steer/submit"),
        &serde_json::json!({
            "target": "Wrapping",
            "interrupt": false,
            "pairing": { "profile_id": building, "model": "claude-opus-5" },
        }),
    )
    .await;

    assert_eq!(steered, ConversationSteered::Steered);

    assert_eq!(
        opened(&app, id)
            .await
            .review_pairing
            .pairing()
            .map(|pairing| pairing.profile.id),
        Some(building),
        "a role with nothing picked for it takes the pick",
    );
}

/// And a Conversation whose human picked *No review* keeps that through a steer
/// into Wrapping: the form's one pick is what the sessions run under, and a
/// role that runs none is not among them.
///
/// It is also settled, so the steer is not refused for a Pairing that is missing
/// — there is nothing missing.
#[tokio::test]
async fn steering_into_wrapping_leaves_a_conversation_with_no_review_unreviewed() {
    let (elsewhere, dir, app, _repo, repo_id) = workbench().await;
    let id = ready(&app, elsewhere.path(), repo_id).await;

    assert_eq!(no_review(&app, id).await, ProfileChosen::Chosen);
    assert_eq!(grill(&app, id).await, GrillingStarted::Started);

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

    let picked = profile(&app, elsewhere.path(), "steering").await;

    assert_eq!(steer(&app, id).await, SteerOpened::Opened);

    let steered: ConversationSteered = post(
        &app,
        &format!("/api/ui/conversations/{id}/steer/submit"),
        &serde_json::json!({
            "target": "Wrapping",
            "interrupt": false,
            "pairing": { "profile_id": picked, "model": "claude-opus-5" },
        }),
    )
    .await;

    assert_eq!(steered, ConversationSteered::Steered);

    let view = opened(&app, id).await;

    assert_eq!(
        view.implementation_pairing
            .map(|pairing| pairing.profile.id),
        Some(picked),
        "what the fixes run under, settled off the pick as ever",
    );
    assert_eq!(
        view.review_pairing,
        PickedView::Skipped,
        "and the review the human turned off stays off",
    );
}

/// A steer into Implementing either carries on what the branch holds or does
/// what the human wrote, so a submit with neither is refused by name.
///
/// What stands is a backlog with work left in it or a roadmap the branch has
/// written, and a Conversation still being grilled has neither: the session
/// that would write one is the session the click just stopped. So the form
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
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = ready(&app, elsewhere.path(), repo_id).await;
    assert_eq!(grill(&app, id).await, GrillingStarted::Started);

    assert!(
        !opened(&app, id).await.ready_to_continue,
        "there is no backlog and no roadmap on the branch, so the form has \
         nothing to offer carrying on",
    );

    assert_eq!(steer(&app, id).await, SteerOpened::Opened);
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
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = ready(&app, elsewhere.path(), repo_id).await;
    assert_eq!(grill(&app, id).await, GrillingStarted::Started);

    assert_eq!(steer(&app, id).await, SteerOpened::Opened);
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
    let (elsewhere, _dir, app, repo, repo_id) = workbench().await;
    let id = ready(&app, elsewhere.path(), repo_id).await;
    assert_eq!(grill(&app, id).await, GrillingStarted::Started);

    let view = opened(&app, id).await;
    let branch = view.branch.clone();
    let base = view.base_commit.clone();
    let worked_in = PathBuf::from(view.worktree.unwrap().path);

    assert_eq!(close(&app, id).await, ConversationClosed::Closed);
    assert!(!worked_in.exists(), "closing took the directory away");
    assert!(opened(&app, id).await.worktree.is_none());

    assert_eq!(steer(&app, id).await, SteerOpened::Opened);
    assert_eq!(
        steer_grilling(&app, id, Some("# Rate limiting, per account\n")).await,
        ConversationSteered::Steered,
    );

    let view = opened(&app, id).await;

    assert_eq!(view.state, Lifecycle::Grilling);
    assert_eq!(view.branch, branch, "on the branch closing kept");

    // And a round of its own, which is the whole of what a closed Conversation
    // is steered back in for: the Brief the first round was built from stays on
    // the record, and the one written on the form is frozen where it landed.
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
    let (elsewhere, _dir, app, repo, repo_id) = workbench().await;
    let id = ready(&app, elsewhere.path(), repo_id).await;
    assert_eq!(grill(&app, id).await, GrillingStarted::Started);

    assert_eq!(steer(&app, id).await, SteerOpened::Opened);
    assert_eq!(
        steer_into(&app, id, "Done", false).await,
        ConversationSteered::Steered,
    );

    let worked_in = opened(&app, id)
        .await
        .worktree
        .expect("Done keeps one")
        .path;

    assert_eq!(steer(&app, id).await, SteerOpened::Opened);
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
    let (elsewhere, dir, app, _repo, repo_id) = workbench().await;
    let id = grilling(&app, elsewhere.path(), repo_id).await;
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
    assert_eq!(steer(&app, id).await, SteerOpened::Opened);
    assert_eq!(
        steer_into(&app, id, "Done", false).await,
        ConversationSteered::Steered,
    );

    assert_eq!(steer(&app, id).await, SteerOpened::Opened);
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
    let (elsewhere, dir, app, _repo, repo_id) = workbench().await;
    let id = grilling(&app, elsewhere.path(), repo_id).await;

    assert_eq!(steer(&app, id).await, SteerOpened::Opened);
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
/// nothing to wrap up here. The form does not offer the target on such a
/// Conversation; this is that same rule asked again on arrival, the way every
/// named refusal here is.
///
/// And the refusal comes before anything is done: the stop the click wrote is
/// still there, and the Conversation is still grilling.
#[tokio::test]
async fn steering_into_wrapping_without_a_pull_request_is_refused_by_name() {
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = ready(&app, elsewhere.path(), repo_id).await;
    assert_eq!(grill(&app, id).await, GrillingStarted::Started);

    assert_eq!(steer(&app, id).await, SteerOpened::Opened);
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
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = ready(&app, elsewhere.path(), repo_id).await;

    assert_eq!(steer(&app, id).await, SteerOpened::Opened);
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
    let (_elsewhere, _dir, app, _repo, _repo_id) = workbench().await;

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
    let (elsewhere, _dir, app, repo, repo_id) = workbench().await;
    let id = ready(&app, elsewhere.path(), repo_id).await;

    assert_eq!(grill(&app, id).await, GrillingStarted::Started);
    assert_eq!(grill(&app, id).await, GrillingStarted::NotDrafting);

    assert_eq!(worktrees(&repo).len(), 2, "the repository and one worktree");
}

#[tokio::test]
async fn grilling_a_conversation_that_is_not_there_says_so() {
    let (_elsewhere, _dir, app, _repo, _repo_id) = workbench().await;

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
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = ready(&app, elsewhere.path(), repo_id).await;
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
    let (elsewhere, _dir, app, repo, repo_id) = workbench().await;
    let id = ready(&app, elsewhere.path(), repo_id).await;
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
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let reading = second_repo(&app, elsewhere.path(), "askance").await;
    let writing = second_repo(&app, elsewhere.path(), "granit").await;
    let id = ready(&app, elsewhere.path(), repo_id).await;

    add_companion(&app, id, reading).await;
    add_companion(&app, id, writing).await;
    companion_mode(&app, id, writing, CompanionMode::ReadWrite).await;

    assert_eq!(grill(&app, id).await, GrillingStarted::Started);

    let view = opened(&app, id).await;
    let branch = view.branch.clone();
    let read = checked_out(&view, "askance");
    let written = checked_out(&view, "granit");

    assert_eq!(close(&app, id).await, ConversationClosed::Closed);

    let askance = elsewhere.path().join("askance");
    let granit = elsewhere.path().join("granit");

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
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = ready(&app, elsewhere.path(), repo_id).await;
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
    let (_elsewhere, _dir, app, _repo, repo_id) = workbench().await;
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
    let (elsewhere, _dir, app, repo, repo_id) = workbench().await;
    let id = ready(&app, elsewhere.path(), repo_id).await;
    grill(&app, id).await;

    let path = PathBuf::from(opened(&app, id).await.worktree.unwrap().path);
    std::fs::remove_dir_all(&path).unwrap();

    assert_eq!(close(&app, id).await, ConversationClosed::Closed);
    assert_eq!(opened(&app, id).await.state, Lifecycle::Closed);
    assert_eq!(worktrees(&repo).len(), 1, "git should have let it go too");
}

/// And a worktree git will not let go of is a close that works too — and the
/// directory does not outlive it. A directory hollowed out — its `.git` file
/// gone — is one git refuses to remove and one the human has every reason to
/// want the end of: the close's own removal cannot touch it and says so in the
/// log, and the sweep that follows deletes it outright.
#[tokio::test]
async fn closing_a_conversation_whose_worktree_git_will_not_remove_sweeps_it_away() {
    let (elsewhere, _dir, app, repo, repo_id) = workbench().await;
    let id = ready(&app, elsewhere.path(), repo_id).await;
    grill(&app, id).await;

    let path = PathBuf::from(opened(&app, id).await.worktree.unwrap().path);
    std::fs::remove_file(path.join(".git")).unwrap();

    assert_eq!(close(&app, id).await, ConversationClosed::Closed);

    let view = opened(&app, id).await;
    assert_eq!(view.state, Lifecycle::Closed);
    assert_eq!(view.worktree, None);
    assert!(
        !path.exists(),
        "the directory git would not remove is what the sweep is for"
    );
    assert_eq!(
        worktrees(&repo).len(),
        1,
        "and the prune cleared the registration it left behind"
    );
}

/// And a close whose browser left part-way through still closes.
///
/// The page draws a close at the press now and stops saying *not yet* — see the
/// viewer's `eager.ts` — so a human who presses Close because they are finished
/// with something and then shuts the tab is doing exactly what they were
/// invited to. That takes the request away, and the work here is seconds long:
/// a session to end, a worktree per repository to give back, a directory to
/// sweep. Run on the request it would go with it, and it gives the worktrees
/// back *before* it writes the record — so what a cancelled close would leave
/// is a Conversation with no checkout and an open record, which is nowhere to
/// work and what Resume then refuses. It runs on a task instead, and a task is
/// nobody's to cancel.
#[tokio::test]
async fn a_close_the_browser_left_part_way_through_still_finishes() {
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = ready(&app, elsewhere.path(), repo_id).await;
    grill(&app, id).await;

    let path = PathBuf::from(opened(&app, id).await.worktree.unwrap().path);

    // One poll of the request and then nothing, which is what a browser that
    // went away leaves behind. It is still Pending at that point — the answer
    // has not been worked out yet — and dropping it here is the connection
    // going.
    let leaving = app.clone().oneshot(
        Request::builder()
            .method("POST")
            .uri(format!("/api/ui/conversations/{id}/close"))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from("{}"))
            .unwrap(),
    );

    assert!(
        tokio::time::timeout(std::time::Duration::ZERO, leaving)
            .await
            .is_err(),
        "the close should still have been running when the caller left"
    );

    // And it finishes all the same: the record says so, and the worktree it
    // gave back on the way is gone.
    //
    // Both asked of the same loop rather than the second asserted the moment the
    // first is true. A close writes the record and *then* sweeps — the sweep is
    // what reclaims a worktree git refused to remove, and it has to run after
    // the record because the rows this close deletes are what make its own
    // directories orphans. So `Closed` is not yet the promise that the directory
    // has gone, and a test that read it as one would be asserting an ordering
    // the close does not give. What it does give is that both are true once it
    // has finished, and that is what is waited for.
    let mut closed = false;

    for _ in 0..200 {
        closed = opened(&app, id).await.state == Lifecycle::Closed;

        if closed && !path.exists() {
            return;
        }

        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }

    assert!(
        closed,
        "the close nobody was left waiting for never finished"
    );

    panic!("the close finished and the worktree should have gone with it");
}

/// A close sweeps the whole worktrees directory, not just its own: whatever an
/// earlier close or a crash left unrecorded goes with it, and every checkout a
/// live Conversation is still working in stays.
///
/// Which is the pair the whole thing turns on. Nothing under the Data Directory
/// that no record names has any business surviving, and nothing a record names
/// may be touched however the sweep was triggered.
#[tokio::test]
async fn closing_one_conversation_sweeps_the_strays_and_leaves_the_live_worktrees() {
    let (elsewhere, dir, app, _repo, repo_id) = workbench().await;

    let closing = ready(&app, elsewhere.path(), repo_id).await;
    grill(&app, closing).await;
    let closing_path = PathBuf::from(opened(&app, closing).await.worktree.unwrap().path);

    // A second Conversation off the Profiles the first one made, `ready` making
    // those by name and a name being takeable once.
    let working = started(&app, repo_id).await;
    write_brief(&app, working, "# Another\n\nThe API still has none.\n").await;
    let profiles: Vec<verkstead_render::ProfileEntry> = get(&app, "/api/ui/profiles").await;
    choose(&app, working, "grilling", profiles[0].id).await;
    choose(&app, working, "implementation", profiles[1].id).await;
    assert_eq!(grill(&app, working).await, GrillingStarted::Started);

    let working_path = PathBuf::from(opened(&app, working).await.worktree.unwrap().path);

    // What a crash leaves: a directory under the worktrees directory that was
    // never a checkout and that no record has ever named.
    let stray = dir.path().join("worktrees/verkstead-from-a-crash");
    std::fs::create_dir_all(stray.join("src")).unwrap();
    std::fs::write(stray.join("src/main.rs"), "fn main() {}\n").unwrap();

    assert_eq!(close(&app, closing).await, ConversationClosed::Closed);

    assert!(!closing_path.exists(), "the Conversation that closed");
    assert!(
        !stray.exists(),
        "and the stray nobody would ever come back for"
    );
    assert!(
        working_path.exists(),
        "while the Conversation still working is untouched",
    );
}

/// Close and archive is the two rows in one press: the Conversation ends and
/// comes off the sidebar, and the record is the record either press leaves.
#[tokio::test]
async fn closing_and_archiving_in_one_press_does_both() {
    let (elsewhere, _dir, app, repo, repo_id) = workbench().await;
    let id = ready(&app, elsewhere.path(), repo_id).await;
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
    let (_elsewhere, _dir, app, _repo, repo_id) = workbench().await;
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
    let (_elsewhere, _dir, app, _repo, _repo_id) = workbench().await;

    assert_eq!(
        close_and_archive(&app, 404).await,
        ConversationClosed::NoSuchConversation
    );
}

#[tokio::test]
async fn closing_a_conversation_that_is_not_there_says_so() {
    let (_elsewhere, _dir, app, _repo, _repo_id) = workbench().await;

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
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = ready(&app, elsewhere.path(), repo_id).await;
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
    let (_elsewhere, _dir, app, _repo, repo_id) = workbench().await;
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
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = ready(&app, elsewhere.path(), repo_id).await;

    assert_eq!(archive(&app, id).await, ConversationArchived::NotClosed);

    grill(&app, id).await;

    assert_eq!(archive(&app, id).await, ConversationArchived::NotClosed);
    assert_eq!(sidebar(&app).await.len(), 1);
}

#[tokio::test]
async fn archiving_a_conversation_that_is_not_there_says_so() {
    let (_elsewhere, _dir, app, _repo, _repo_id) = workbench().await;

    assert_eq!(
        archive(&app, 404).await,
        ConversationArchived::NoSuchConversation
    );
}

/// Write a state word into a Conversation's row that no Verkstead knows, and
/// say what the column holds afterwards.
///
/// However it got there — a database restored from before a migration, one
/// written by a Verkstead ahead of this one, a row edited by hand — the human
/// is left with a Conversation every ordinary read refuses. Which is the state
/// the two tests below are about, and the reason there is an escape hatch on
/// the pane at all.
async fn corrupt_the_state(dir: &tempfile::TempDir, id: i64) {
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    sqlx::query("UPDATE conversations SET state = ? WHERE id = ?")
        .bind("meandering")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();

    pool.close().await;
}

async fn stored_state(dir: &tempfile::TempDir, id: i64) -> String {
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    let (state,): (String,) = sqlx::query_as("SELECT state FROM conversations WHERE id = ?")
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();

    pool.close().await;

    state
}

/// A Conversation whose state word nothing can parse is one the human can still
/// see, still reach, and still end.
///
/// Its own page refuses — the read that draws it will not guess at where the
/// work stands — which is the error state the pane draws its escape hatch in.
/// What has to hold for that hatch to be reachable and to work is all here: the
/// sidebar still draws the row, close-and-archive goes through, and what the
/// close leaves behind is a row holding `closed`, so the Conversation reads
/// again afterwards.
#[tokio::test]
async fn a_conversation_whose_state_word_is_unreadable_can_still_be_closed_and_archived() {
    let (elsewhere, dir, app, repo, repo_id) = workbench().await;
    let id = ready(&app, elsewhere.path(), repo_id).await;
    grill(&app, id).await;

    let path = PathBuf::from(opened(&app, id).await.worktree.unwrap().path);
    corrupt_the_state(&dir, id).await;

    // The pane cannot be drawn, which is what puts the human in front of the
    // hatch rather than the ordinary ⋯ menu.
    let (status, _) = fetch(
        &app,
        Request::builder()
            .uri(format!("/api/ui/conversations/{id}"))
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);

    // And the sidebar is still there, with the row on it — drawn as a draft,
    // which is the fallback the hatch reads as *not closed* and so offers
    // Close and archive for.
    let row = only_row(&app).await;
    assert_eq!(row.id, id);
    assert_eq!(row.state, Lifecycle::Draft);

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

    // The close wrote `closed` over the word nobody could read, so everything
    // that was locked behind that column reads again.
    assert_eq!(stored_state(&dir, id).await, "closed");

    let view = opened(&app, id).await;
    assert_eq!(view.state, Lifecycle::Closed);
    assert!(view.archived);
    assert_eq!(view.worktree, None);
}

/// Archiving one on its own says it is not closed, rather than failing.
///
/// The safe way round: archiving is *hide it from the list*, and hiding a
/// Conversation whose worktree may still be live would put the work out of
/// sight without ending it. Which is why the hatch offers Close and archive
/// wherever it cannot read the state.
#[tokio::test]
async fn archiving_a_conversation_whose_state_word_is_unreadable_says_it_is_not_closed() {
    let (_elsewhere, dir, app, _repo, repo_id) = workbench().await;
    let id = started(&app, repo_id).await;
    corrupt_the_state(&dir, id).await;

    assert_eq!(archive(&app, id).await, ConversationArchived::NotClosed);
    assert_eq!(sidebar(&app).await.len(), 1);
}

/// The toggle is the way to see what has been put away without taking it back:
/// on, the archived Conversations are on the list in their ordinary places; off,
/// they are not drawn at all.
#[tokio::test]
async fn the_toggle_shows_and_hides_what_has_been_archived() {
    let (_elsewhere, _dir, app, _repo, repo_id) = workbench().await;
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

/// And the same read says whether there is anything behind the switch at all,
/// which the list itself cannot: it is filtered by the switch, so an empty list
/// is the same empty list whether nothing has been put away or everything has.
///
/// Which is what a page with no sidebar to hang the switch under needs — the
/// zero state draws it only where there is something for it to bring back.
#[tokio::test]
async fn the_toggle_says_whether_there_is_anything_behind_it() {
    let (_elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = started(&app, repo_id).await;

    assert!(!anything_archived(&app).await);

    close(&app, id).await;
    archive(&app, id).await;

    // Nothing on the list, and something behind the switch: the two empties
    // told apart.
    assert!(sidebar(&app).await.is_empty());
    assert!(anything_archived(&app).await);

    // And it is about the archiving rather than about the switch, so flipping
    // the switch does not move it.
    show_archived(&app, true).await;
    assert!(anything_archived(&app).await);

    unarchive(&app, id).await;
    assert!(!anything_archived(&app).await);
}

/// It is the human's standing choice rather than one device's, so it is read
/// back off the server — which is what a second viewer opening the sidebar is.
#[tokio::test]
async fn the_toggle_is_read_back_off_the_server() {
    let (_elsewhere, _dir, app, _repo, repo_id) = workbench().await;
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
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = ready(&app, elsewhere.path(), repo_id).await;
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

/// And what a Cleanup has taken is on the view too, which is what lets the page
/// name the record Trimmed and a session's card say why its drill-down is
/// missing.
///
/// Trimmed through the store rather than through an endpoint, because there is
/// no endpoint: a trim is the sweep's, and the viewer only ever reads what it
/// did.
#[tokio::test]
async fn a_trimmed_conversation_says_so_on_its_page() {
    let (elsewhere, _dir, app, pool, repo_id) = workbench_and_pool().await;
    let id = ready(&app, elsewhere.path(), repo_id).await;
    close(&app, id).await;
    archive(&app, id).await;

    assert!(
        !opened(&app, id).await.trimmed,
        "nothing has been taken out of it yet",
    );

    assert_eq!(
        store::trim_conversation(&pool, id).await.unwrap(),
        store::Trimming::Trimmed,
    );

    let view = opened(&app, id).await;
    assert!(view.trimmed);
    assert!(view.archived, "and it is still put away");

    // And the mark outlasts the archiving it was made under: what was taken is
    // gone, so the page goes on being able to account for it.
    assert_eq!(
        unarchive(&app, id).await,
        ConversationUnarchived::Unarchived
    );

    let view = opened(&app, id).await;
    assert!(view.trimmed);
    assert!(!view.archived);
}

/// Unarchiving one that was never put away is not an error — what the human
/// asked for holds either way.
#[tokio::test]
async fn unarchiving_one_that_is_not_archived_is_not_an_error() {
    let (_elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = started(&app, repo_id).await;

    assert_eq!(
        unarchive(&app, id).await,
        ConversationUnarchived::NotArchived
    );
    assert_eq!(order(&app).await, vec![id]);
}

#[tokio::test]
async fn unarchiving_a_conversation_that_is_not_there_says_so() {
    let (_elsewhere, _dir, app, _repo, _repo_id) = workbench().await;

    assert_eq!(
        unarchive(&app, 404).await,
        ConversationUnarchived::NoSuchConversation
    );
}

/// A worktree removed from under Verkstead is a thing to say, not a thing to
/// fail on later.
#[tokio::test]
async fn a_conversation_whose_worktree_has_gone_says_so() {
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = ready(&app, elsewhere.path(), repo_id).await;
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
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;

    let first = ready(&app, elsewhere.path(), repo_id).await;
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
async fn grilling(app: &Router, elsewhere: &Path, repo_id: i64) -> i64 {
    let id = ready(app, elsewhere, repo_id).await;
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
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = grilling(&app, elsewhere.path(), repo_id).await;

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
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = grilling(&app, elsewhere.path(), repo_id).await;

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
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = grilling(&app, elsewhere.path(), repo_id).await;

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
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = grilling(&app, elsewhere.path(), repo_id).await;

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
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = grilling(&app, elsewhere.path(), repo_id).await;

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
    let (_elsewhere, _dir, app, _repo, repo_id) = workbench().await;
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
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = grilling(&app, elsewhere.path(), repo_id).await;

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
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = grilling(&app, elsewhere.path(), repo_id).await;

    // The click is the shortest way to a stop written down: it stops the drive
    // and opens the form, and nothing here submits one.
    assert_eq!(steer(&app, id).await, SteerOpened::Opened);
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
    let (_elsewhere, dir, app, _repo, repo_id) = workbench().await;
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
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = grilling(&app, elsewhere.path(), repo_id).await;

    ask(&app, id, ORDINARY).await;

    assert_eq!(steer(&app, id).await, SteerOpened::Opened);
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

/// The Conversation's own page carries the same fact its row does, folded by the
/// same rule in the same place — see the store's `waits_on_the_human`.
///
/// A grilling nobody has asked anything on waits on nothing: it is being worked,
/// and being worked is not wanting the human. The ask is what turns it on, and
/// the answer is what turns it off again.
#[tokio::test]
async fn the_conversation_view_says_when_an_open_ask_is_waiting_on_the_human() {
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = grilling(&app, elsewhere.path(), repo_id).await;

    assert!(
        !waits(&app, id).await,
        "a grilling with nothing outstanding wants nobody",
    );

    let set = ask(&app, id, ORDINARY).await;
    assert!(waits(&app, id).await, "there is something answerable now");

    answer_ordinary(&app, set).await;
    assert!(
        !waits(&app, id).await,
        "and the answer is the end of it: nothing is left to come back for",
    );
}

/// And a stop is the other source, read the same way on the page as on the row:
/// what happened without the human draws them, and their own press does not.
///
/// Beside `stopped_by_hand`, which is the same stop asked a narrower question —
/// *which* mark the head draws. This is whether there is one at all.
#[tokio::test]
async fn the_conversation_view_says_when_a_stop_is_waiting_on_the_human() {
    let (elsewhere, dir, app, _repo, repo_id) = workbench().await;
    let id = grilling(&app, elsewhere.path(), repo_id).await;
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    store::stop(
        &pool,
        id,
        store::Decision::Human,
        "You pressed Stop.\n",
        None,
    )
    .await
    .unwrap()
    .expect("the Conversation was running");

    assert!(
        opened(&app, id).await.blocked_on.is_some(),
        "the drive has stopped, and the head says so",
    );
    assert!(
        !waits(&app, id).await,
        "but they pressed it themselves, so there is nothing here they have not heard",
    );

    store::clear_stop(&pool, id).await.unwrap();
    store::stop(
        &pool,
        id,
        store::Decision::Verkstead,
        "The checks would not go green.\n",
        None,
    )
    .await
    .unwrap()
    .expect("the Conversation is running again");

    assert!(
        waits(&app, id).await,
        "Verkstead's own brake stopped it, and nobody has been told but the page",
    );

    pool.close().await;
}

/// Whether a Conversation waits on the human, asked of its page and of its row
/// together.
///
/// The two are one fold in the store rather than two readings that happen to
/// agree — see the store's `waits_on_the_human` — so a test of either is a test
/// of both, and this asserts as much on every read.
async fn waits(app: &Router, id: i64) -> bool {
    let waiting = opened(app, id).await.waiting;

    assert_eq!(
        waiting,
        row(app, id).await.waiting,
        "the page and the sidebar row disagreed about the same Conversation",
    );

    waiting
}

/// What a session was launched under is stamped onto its Event as that Event is
/// opened, so the record says what actually ran — see the store's `RanUnder`.
///
/// Written down rather than looked up afterwards: a Conversation's Pairing is a
/// thing the human can repick and a Profile is a thing they can rename or
/// delete, and none of that changes what a session that has already run was
/// running. Nothing in the viewer draws it yet — the StatusButton is what will.
///
/// The Capture is opened through the store rather than by running an agent, for
/// the reason the worktrees here are: whether a session starts at all is
/// `sessions.rs`'s subject, and what an Event opened under a Pairing puts on the
/// wire is this file's.
#[tokio::test]
async fn a_sessions_event_says_what_it_was_launched_under() {
    let (elsewhere, dir, app, _repo, repo_id) = workbench().await;
    let id = grilling(&app, elsewhere.path(), repo_id).await;
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    // The Pairing this Conversation's grilling was settled under, which is what
    // the server has in hand at the moment it starts a session.
    let picked = opened(&app, id)
        .await
        .grilling_pairing
        .as_ref()
        .expect("the grilling was paired before it started")
        .clone();
    let pairing = store::Pairing {
        profile: store::load_profile(&pool, picked.profile.id)
            .await
            .unwrap()
            .expect("the Profile is still there"),
        model: picked.model,
    };

    store::start_capture(&pool, id, Some("a-session"), Some(&pairing))
        .await
        .unwrap();

    let output = printed(&app, id).await;
    assert_eq!(
        output.profile.as_deref(),
        Some("fable"),
        "the name of the Profile the session was launched from",
    );
    assert_eq!(
        output.model.as_deref(),
        Some("claude-opus-5"),
        "and the model id raw, prettifying being the viewer's alone",
    );
    assert_eq!(
        output.agent_type,
        Some(AgentType::Claude),
        "and which agent ran it, which is what the harness mark is drawn from",
    );

    pool.close().await;
}

/// And it says which agent ran it whatever that agent is, the whole point of
/// recording one being to tell two harnesses apart.
///
/// Launched under a Profile of a type nothing else in this file uses, because a
/// record that answered "Claude" for everything would pass every assertion
/// above without knowing anything.
#[tokio::test]
async fn a_sessions_event_says_which_harness_ran_it() {
    let (elsewhere, dir, app, _repo, repo_id) = workbench().await;
    let id = grilling(&app, elsewhere.path(), repo_id).await;
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    let profile_id = codex_profile(&app, elsewhere.path(), "hopper").await;
    let pairing = store::Pairing {
        profile: store::load_profile(&pool, profile_id)
            .await
            .unwrap()
            .expect("the Profile just saved is there"),
        model: Some("gpt-5.2-codex".to_owned()),
    };

    store::start_capture(&pool, id, Some("a-session"), Some(&pairing))
        .await
        .unwrap();

    let output = printed(&app, id).await;
    assert_eq!(
        (output.profile.as_deref(), output.model.as_deref()),
        (Some("hopper"), Some("gpt-5.2-codex")),
        "the Profile and the model this one was launched under",
    );
    assert_eq!(
        output.agent_type,
        Some(AgentType::Codex),
        "and the agent that ran it, which is the Profile's own type and not a default",
    );

    pool.close().await;
}

/// What became of the Profile afterwards changes nothing: the Event is the
/// account of a session that has already run.
///
/// Rewritten whole rather than renamed, which is the strongest version of the
/// same press — a new name over another harness's account. Deleting it is the
/// other half of the same rule and is refused while a Conversation has it
/// chosen, so what a rewrite proves is what there is to prove: none of the three
/// things recorded is looked up when the Timeline is read.
#[tokio::test]
async fn what_the_profile_became_afterwards_changes_nothing() {
    let (elsewhere, dir, app, _repo, repo_id) = workbench().await;
    let id = grilling(&app, elsewhere.path(), repo_id).await;
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    let picked = opened(&app, id)
        .await
        .grilling_pairing
        .as_ref()
        .expect("the grilling was paired before it started")
        .clone();
    let pairing = store::Pairing {
        profile: store::load_profile(&pool, picked.profile.id)
            .await
            .unwrap()
            .expect("the Profile is still there"),
        model: picked.model,
    };

    store::start_capture(&pool, id, Some("a-session"), Some(&pairing))
        .await
        .unwrap();

    let home = codex_home(elsewhere.path(), "fable");
    let saved: ProfileSaved = post(
        &app,
        &format!("/api/ui/profiles/{}", picked.profile.id),
        &serde_json::json!({
            "name": "renamed",
            "account": { "agent_type": "Codex", "home": home },
            "models": ["gpt-5.2-codex"],
        }),
    )
    .await;
    assert_eq!(saved, ProfileSaved::Saved);

    let output = printed(&app, id).await;
    assert_eq!(
        (output.profile.as_deref(), output.model.as_deref()),
        (Some("fable"), Some("claude-opus-5")),
        "the name and the model as they read when the session started",
    );
    assert_eq!(
        output.agent_type,
        Some(AgentType::Claude),
        "and the agent that ran it, whatever the Profile has since become",
    );

    pool.close().await;
}

/// A Codex home inside `elsewhere`, which is the whole of what a Codex account
/// is — see `tests/profiles.rs`, where the shape of each type's account is the
/// subject.
fn codex_home(elsewhere: &Path, account: &str) -> PathBuf {
    let home = elsewhere.join(account).join(".codex");
    std::fs::create_dir_all(&home).unwrap();
    home
}

/// Save a Codex Profile and hand back its id, [`profile`]'s way: through the
/// endpoint the human saves one through, so what is recorded is what a save
/// leaves behind.
async fn codex_profile(app: &Router, elsewhere: &Path, name: &str) -> i64 {
    let saved: ProfileSaved = post(
        app,
        "/api/ui/profiles",
        &serde_json::json!({
            "name": name,
            "account": { "agent_type": "Codex", "home": codex_home(elsewhere, name) },
            "models": ["gpt-5.2-codex"],
        }),
    )
    .await;
    assert_eq!(saved, ProfileSaved::Saved);

    let profiles: Vec<verkstead_render::ProfileEntry> = get(app, "/api/ui/profiles").await;
    profiles
        .into_iter()
        .find(|profile| profile.name.as_deref() == Some(name))
        .expect("the Profile just saved should be on the list")
        .id
}

/// And a session from before any of that was written down says nothing about
/// it, which is every Event on every Timeline that is already there.
///
/// Absent rather than guessed at: the Conversation's Pairing now is what the
/// *next* session would run under, and answering with it would be Verkstead
/// making up a history it does not have. The rest of the Event is unchanged, so
/// everything drawn from one goes on being drawn.
#[tokio::test]
async fn a_session_that_was_never_paired_says_nothing_about_it() {
    let (elsewhere, dir, app, _repo, repo_id) = workbench().await;
    let id = grilling(&app, elsewhere.path(), repo_id).await;
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    store::start_capture(&pool, id, None, None).await.unwrap();

    let output = printed(&app, id).await;
    assert_eq!(output.profile, None, "nothing was paired with this one");
    assert_eq!(output.model, None, "so there is no model to name either");
    assert_eq!(
        output.agent_type, None,
        "and no agent to name, which is a reading drawn without a mark",
    );
    assert_eq!(
        (output.lines, output.turns, output.latest.as_str()),
        (0, None, ""),
        "and the Event is otherwise the one it always was",
    );

    pool.close().await;
}

/// And a session paired before the agent was written down keeps its pairing and
/// says nothing about the agent, which is every Event recorded between the two.
///
/// The row is taken away by hand because nothing writes one of these any more:
/// the agent arrived in a table beside the pairing rather than as a column in
/// it, so a Timeline from between the two is a pairing with no agent beside it.
#[tokio::test]
async fn a_session_paired_before_the_agent_was_recorded_says_nothing_about_it() {
    let (elsewhere, dir, app, _repo, repo_id) = workbench().await;
    let id = grilling(&app, elsewhere.path(), repo_id).await;
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    let picked = opened(&app, id)
        .await
        .grilling_pairing
        .as_ref()
        .expect("the grilling was paired before it started")
        .clone();
    let pairing = store::Pairing {
        profile: store::load_profile(&pool, picked.profile.id)
            .await
            .unwrap()
            .expect("the Profile is still there"),
        model: picked.model,
    };

    let event = store::start_capture(&pool, id, Some("a-session"), Some(&pairing))
        .await
        .unwrap();
    sqlx::query("DELETE FROM session_agents WHERE event_id = ?")
        .bind(event)
        .execute(&pool)
        .await
        .unwrap();

    let output = printed(&app, id).await;
    assert_eq!(
        (output.profile.as_deref(), output.model.as_deref()),
        (Some("fable"), Some("claude-opus-5")),
        "the pairing is there, being what was recorded at the time",
    );
    assert_eq!(
        output.agent_type, None,
        "and the agent is not, which is nothing rather than an error",
    );

    pool.close().await;
}

/// The one session's output on a Conversation's page.
async fn printed(app: &Router, id: i64) -> verkstead_render::AgentOutputEvent {
    opened(app, id)
        .await
        .timeline
        .into_iter()
        .find_map(|event| match event {
            TimelineEvent::AgentOutput(output) => Some(output),
            _ => None,
        })
        .expect("the Conversation has a session's output on its Timeline")
}

/// A server running no sessions at all — which is every one of these — has none
/// to report. What a running one does to the row is `sessions.rs`'s to say, being
/// the file with an agent in it.
#[tokio::test]
async fn a_conversation_with_no_session_running_is_not_working() {
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    grilling(&app, elsewhere.path(), repo_id).await;

    assert!(!only_row(&app).await.working);
}

/// The handoff is written outside the checkout on purpose. What proves it is git
/// having nothing to say about the worktree afterwards — an agent that later runs
/// `git add -A` is the whole reason the file is not in there.
#[tokio::test]
async fn a_handoff_never_lands_in_the_repository() {
    let (elsewhere, dir, app, _repo, repo_id) = workbench().await;
    let id = grilling(&app, elsewhere.path(), repo_id).await;

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
        let (elsewhere, dir, app, _repo, repo_id) = workbench().await;
        let id = grilling(&app, elsewhere.path(), repo_id).await;

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
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = grilling(&app, elsewhere.path(), repo_id).await;

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
/// is what the tail is elsewhere for, and nothing else. What ends that session and
/// moves the Conversation is the artifact landing, which wants an agent to write
/// it: `sessions.rs` is where each is asked end to end.
#[tokio::test]
async fn a_pick_leaves_the_conversation_grilling() {
    for (picked, direction) in [
        ("inline", verkstead_schema::Direction::Inline),
        ("task-list", verkstead_schema::Direction::TaskList),
        ("roadmap", verkstead_schema::Direction::Roadmap),
    ] {
        let (elsewhere, dir, app, _repo, repo_id) = workbench().await;
        let id = grilling(&app, elsewhere.path(), repo_id).await;

        let written = handoff_written(dir.path(), id, "# What we settled\n");

        assert_eq!(
            picking(&app, ask(&app, id, PROPOSING).await, picked).await,
            verkstead_render::Submitted::Accepted,
        );

        let view = opened(&app, id).await;

        assert_eq!(
            view.direction,
            Some(direction),
            "the pick is recorded: it is what the artifact is elsewhere for — picking {picked}",
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
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = grilling(&app, elsewhere.path(), repo_id).await;

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
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = grilling(&app, elsewhere.path(), repo_id).await;

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
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = grilling(&app, elsewhere.path(), repo_id).await;

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
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = grilling(&app, elsewhere.path(), repo_id).await;

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
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = ready(&app, elsewhere.path(), repo_id).await;
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
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = ready(&app, elsewhere.path(), repo_id).await;
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
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = ready(&app, elsewhere.path(), repo_id).await;

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
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = ready(&app, elsewhere.path(), repo_id).await;
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
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = ready(&app, elsewhere.path(), repo_id).await;

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
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = ready(&app, elsewhere.path(), repo_id).await;
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
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = ready(&app, elsewhere.path(), repo_id).await;

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
    let (_elsewhere, _dir, app, repo, repo_id) = workbench().await;
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
        stage.branch, "roadmaps/mvp/03-implementation",
        "the stage's own name, which the press names the branch by",
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
    let (_elsewhere, _dir, app, repo, repo_id) = workbench().await;
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
    let (_elsewhere, _dir, app, repo, repo_id) = workbench().await;
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
    let (_elsewhere, _dir, app, repo, repo_id) = workbench().await;
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
    let (_elsewhere, _dir, app, _repo, _repo_id) = workbench().await;

    assert_eq!(adopt(&app, 404, "mvp").await, Started::NoSuchRepo);
    assert!(sidebar(&app).await.is_empty());
}

/// Pressing a free row of the *Wrap up a pull request* level: a Draft against
/// that Repo holding the pull request the row named, whose page draws it.
///
/// Nothing about the repository is touched by the press. The head branch is not
/// checked out, no base is fixed and the Brief is empty — what the human edits
/// arrives with the create's replay, and everything git is the take-up's.
#[tokio::test]
async fn wrapping_a_pull_request_up_starts_a_draft_naming_it() {
    let (_elsewhere, _dir, app, _repo, repo_id) = workbench().await;

    let id = wrapping_up(&app, repo_id, 41).await;

    let sidebar = sidebar(&app).await;
    assert_eq!(sidebar.len(), 1);
    assert_eq!(sidebar[0].id, id);
    assert_eq!(sidebar[0].state, Lifecycle::Draft);

    let view = opened(&app, id).await;
    let held = view
        .adopting_pull_request
        .clone()
        .expect("this Conversation is holding one");

    assert_eq!(held.number, 41);
    assert_eq!(held.title, "Rate limiting for the public API");
    assert_eq!(held.url, "https://github.com/tobico/verkstead/pull/41");
    assert_eq!(held.head, "rate-limiting");
    assert_eq!(held.base, "main");

    // A pull request is the other thing a Draft adopts rather than a second
    // thing beside a roadmap, and its Brief is the human's to write.
    assert_eq!(view.adopting, None);
    assert_eq!(brief(&view).markdown, "");
    assert_eq!(view.base_commit, None);
    assert_eq!(view.worktree, None);
}

/// And the Repo picker on that Draft refuses a move by name: `#41` is a fact
/// about one repository, and the same number over there is a different pull
/// request or none at all.
#[tokio::test]
async fn a_draft_holding_a_pull_request_refuses_a_repo_move() {
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;

    let askance = repository(elsewhere.path().join("askance"));
    let registered: Registered = post(
        &app,
        "/api/ui/repos",
        &serde_json::json!({ "path": askance }),
    )
    .await;
    assert!(matches!(registered, Registered::Added(_)));

    let id = wrapping_up(&app, repo_id, 41).await;

    let elsewhere_id = get::<Vec<RepoEntry>>(&app, "/api/ui/repos")
        .await
        .into_iter()
        .find(|entry| entry.name == "askance")
        .expect("both are registered")
        .id;

    let moved: RepoSwitched = post(
        &app,
        &format!("/api/ui/conversations/{id}/repo"),
        &serde_json::json!({ "repo_id": elsewhere_id }),
    )
    .await;

    assert_eq!(moved, RepoSwitched::HoldingPullRequest);
    assert_eq!(opened(&app, id).await.repo.id, repo_id);
}

/// An ordinary Conversation is holding no pull request, which is what keeps its
/// page on the shape with a branch, a base and a grilling to settle.
#[tokio::test]
async fn a_conversation_started_the_ordinary_way_holds_no_pull_request() {
    let (_elsewhere, _dir, app, _repo, repo_id) = workbench().await;

    let id = started(&app, repo_id).await;

    assert_eq!(opened(&app, id).await.adopting_pull_request, None);
}

#[tokio::test]
async fn wrapping_up_against_a_repo_that_is_not_registered_says_so() {
    let (_elsewhere, _dir, app, _repo, _repo_id) = workbench().await;

    assert_eq!(wrap_up(&app, 404, 41).await, Started::NoSuchRepo);
    assert!(sidebar(&app).await.is_empty());
}

/// One row of that level, sent as the page sends it.
async fn wrap_up(app: &Router, repo_id: i64, number: i64) -> Started {
    post(
        app,
        "/api/ui/pull-request-adoptions",
        &serde_json::json!({
            "repo_id": repo_id,
            "number": number,
            "title": "Rate limiting for the public API",
            "url": format!("https://github.com/tobico/verkstead/pull/{number}"),
            "head": "rate-limiting",
            "base": "main",
        }),
    )
    .await
}

async fn wrapping_up(app: &Router, repo_id: i64, number: i64) -> i64 {
    match wrap_up(app, repo_id, number).await {
        Started::Started { id } => id,
        other => panic!("expected the Conversation to start, got {other:?}"),
    }
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
async fn ready_to_adopt(app: &Router, elsewhere: &Path, repo_id: i64, name: &str) -> i64 {
    let id = adopting(app, repo_id, name).await;

    let grilling = profile(app, elsewhere, "fable").await;
    let implementation = profile(app, elsewhere, "opus").await;
    let review = profile(app, elsewhere, "haiku").await;
    choose(app, id, "grilling", grilling).await;
    choose(app, id, "implementation", implementation).await;
    choose(app, id, "review", review).await;

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

/// Submit the form with a companion section filled in: where the work goes, and
/// which registered Repos go into the sandbox with it.
///
/// The rows as the form sends them — the Repo, how far in, the branch its
/// checkout comes off and what a read-write one's branch is called — because
/// that is the whole of what a setup row settles and this is the one other
/// moment it can be settled.
async fn steer_alongside(
    app: &Router,
    id: i64,
    target: &str,
    added: serde_json::Value,
) -> ConversationSteered {
    steer_companions(app, id, target, added, serde_json::json!([])).await
}

/// And with its other half filled in: which of the companions already there are
/// being opened up, and what the branch cut in each is called.
///
/// No mode on those rows, because there is one direction: read-only is not
/// something the form can ask for, so a downgrade cannot be spelled at all.
async fn steer_opening(
    app: &Router,
    id: i64,
    target: &str,
    upgraded: serde_json::Value,
) -> ConversationSteered {
    steer_companions(app, id, target, serde_json::json!([]), upgraded).await
}

/// Both halves at once, which is what the form always sends.
async fn steer_companions(
    app: &Router,
    id: i64,
    target: &str,
    added: serde_json::Value,
    upgraded: serde_json::Value,
) -> ConversationSteered {
    post(
        app,
        &format!("/api/ui/conversations/{id}/steer/submit"),
        &serde_json::json!({
            "target": target,
            "interrupt": false,
            "added": added,
            "upgraded": upgraded,
        }),
    )
    .await
}

/// One row of the opening half, with the least a human has to say about it: the
/// branch mirroring the Conversation's own.
fn opening(repo_id: i64) -> serde_json::Value {
    serde_json::json!({ "repo_id": repo_id, "branch": "" })
}

/// One row of that section, with the least a human has to say about it.
fn alongside(repo_id: i64, mode: &str) -> serde_json::Value {
    serde_json::json!({
        "repo_id": repo_id,
        "mode": mode,
        "base_ref": serde_json::Value::Null,
        "branch": "",
    })
}

/// The whole of what a companion costs a steer: a checkout of its own beside the
/// Conversation's, detached where it is only read and on a branch of its own
/// where it is worked in, the row and the worktree recorded in the same act as
/// the move, and a line under the Steer saying what went in.
///
/// The one moment those questions can be asked past drafting. What the setup
/// card's rows settle is frozen when grilling starts, and this is the section
/// that asks them again — of a repository joining now, and of nothing else.
#[tokio::test]
async fn steering_puts_a_companion_in_and_checks_it_out() {
    let (elsewhere, dir, app, _repo, repo_id) = workbench().await;
    let reading = second_repo(&app, elsewhere.path(), "askance").await;
    let writing = second_repo(&app, elsewhere.path(), "granit").await;
    let id = ready(&app, elsewhere.path(), repo_id).await;

    assert_eq!(grill(&app, id).await, GrillingStarted::Started);
    assert!(companions(&app, id).await.is_empty());

    assert_eq!(steer(&app, id).await, SteerOpened::Opened);
    assert_eq!(
        steer_alongside(
            &app,
            id,
            "Grilling",
            serde_json::json!([
                alongside(reading, "ReadOnly"),
                alongside(writing, "ReadWrite"),
            ]),
        )
        .await,
        ConversationSteered::Steered,
    );

    let view = opened(&app, id).await;

    assert_eq!(view.state, Lifecycle::Grilling);
    assert_eq!(companions(&app, id).await, ["askance", "granit"]);

    // The read-only one is detached at whatever its base came to, and holds no
    // branch in somebody else's repository.
    let askance = elsewhere.path().join("askance");
    let detached = checked_out(&view, "askance");

    assert_eq!(companion(&view, "askance").mode, CompanionMode::ReadOnly);
    assert_eq!(companion(&view, "askance").branch, "");
    assert_eq!(
        git(&detached, &["rev-parse", "HEAD"]).trim(),
        git(&askance, &["rev-parse", "HEAD"]).trim(),
        "detached at its default branch's tip, resolved at the steer",
    );
    assert_eq!(
        companion(&view, "askance").base_commit.as_deref(),
        Some(git(&askance, &["rev-parse", "HEAD"]).trim()),
    );

    // And the read-write one is on a branch of its own, mirroring the
    // Conversation's because nothing was typed in the field.
    let granit = elsewhere.path().join("granit");
    let worked = checked_out(&view, "granit");

    assert_eq!(companion(&view, "granit").mode, CompanionMode::ReadWrite);
    assert_eq!(
        git(&worked, &["symbolic-ref", "--short", "HEAD"]).trim(),
        view.branch,
    );
    assert!(has_branch(&granit, &view.branch));

    // Both under the data directory, and both registered with git — which is
    // what makes them worktrees rather than copies.
    for path in [&detached, &worked] {
        assert_eq!(
            path.parent(),
            Some(dir.path().join("worktrees").as_path()),
            "{path:?}",
        );
    }

    assert!(worktrees(&askance).contains(&detached.canonicalize().unwrap()));
    assert!(worktrees(&granit).contains(&worked.canonicalize().unwrap()));

    // And the Timeline says what went in and at which mode, directly under the
    // human's own line rather than beside it.
    let mut after = view
        .timeline
        .iter()
        .skip_while(|event| !matches!(event, TimelineEvent::Steer(_)))
        .skip(1);

    let Some(TimelineEvent::Notice(said)) = after.next() else {
        panic!("what stands under the Steer is the line saying what went in");
    };

    assert!(
        said.html.contains("askance") && said.html.contains("read-only"),
        "the read-only one is named with its mode: {}",
        said.html,
    );
    assert!(
        said.html.contains("granit") && said.html.contains("read-write"),
        "and so is the read-write one: {}",
        said.html,
    );
}

/// And a steer into Done puts nothing in and opens nothing up, whatever the
/// submit carried.
///
/// Nothing runs there, so there is no sandbox to set up and nothing a companion
/// could be for — which is why the form draws no section on that target. A
/// submit carrying one anyway is a page sending a field it should not have
/// drawn, and it is answered the way a brief beside a wrap-up is: ignored rather
/// than obeyed.
#[tokio::test]
async fn steering_into_done_puts_no_companion_in() {
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let joining = second_repo(&app, elsewhere.path(), "granit").await;
    let reading = second_repo(&app, elsewhere.path(), "askance").await;
    let id = ready(&app, elsewhere.path(), repo_id).await;

    add_companion(&app, id, reading).await;
    assert_eq!(grill(&app, id).await, GrillingStarted::Started);

    let detached = checked_out(&opened(&app, id).await, "askance");

    assert_eq!(steer(&app, id).await, SteerOpened::Opened);
    assert_eq!(
        steer_companions(
            &app,
            id,
            "Done",
            serde_json::json!([alongside(joining, "ReadWrite")]),
            serde_json::json!([opening(reading)]),
        )
        .await,
        ConversationSteered::Steered,
    );

    let view = opened(&app, id).await;

    assert_eq!(view.state, Lifecycle::Done);
    assert_eq!(companions(&app, id).await, ["askance"]);
    assert!(
        notices(&view).iter().all(|said| !said.contains("granit")),
        "and nothing on the Timeline says a repository went anywhere",
    );
    assert!(
        !has_branch(&elsewhere.path().join("granit"), &view.branch),
        "nor is there a branch in it",
    );

    // And the one that was there is untouched: still read-only, still detached
    // where it was, and no branch cut in its repository either.
    assert_eq!(companion(&view, "askance").mode, CompanionMode::ReadOnly);
    assert_eq!(checked_out(&view, "askance"), detached);
    assert!(!has_branch(&elsewhere.path().join("askance"), &view.branch));
}

/// Each of the three questions git is asked about a companion refuses the whole
/// steer and says which repository it was — and leaves nothing behind: no
/// directory, no branch, no row, and the Conversation exactly where it stood.
#[tokio::test]
async fn a_companion_a_steer_cannot_deliver_refuses_it_by_name() {
    for why in [
        SteerCompanionRefusal::FetchFailed,
        SteerCompanionRefusal::NoBaseCommit,
        SteerCompanionRefusal::BranchExists,
    ] {
        let (elsewhere, dir, app, _repo, repo_id) = workbench().await;
        let joining = second_repo(&app, elsewhere.path(), "askance").await;
        let id = ready(&app, elsewhere.path(), repo_id).await;

        assert_eq!(grill(&app, id).await, GrillingStarted::Started);

        let askance = elsewhere.path().join("askance");
        let branch = opened(&app, id).await.branch;

        let row = match why {
            // A remote that answers to nothing: what the checkout would come off
            // cannot be trusted to be what the remote is holding.
            SteerCompanionRefusal::FetchFailed => {
                let nowhere = dir.path().join("no-such-remote");
                git(
                    &askance,
                    &["remote", "add", "origin", &nowhere.to_string_lossy()],
                );

                alongside(joining, "ReadOnly")
            }
            // A base picked on the form that the repository does not have.
            SteerCompanionRefusal::NoBaseCommit => serde_json::json!({
                "repo_id": joining,
                "mode": "ReadOnly",
                "base_ref": "release-1.4",
                "branch": "",
            }),
            // And a name in that repository that is already somebody's work.
            _ => {
                git(&askance, &["branch", &branch]);

                alongside(joining, "ReadWrite")
            }
        };

        assert_eq!(steer(&app, id).await, SteerOpened::Opened);
        assert_eq!(
            steer_alongside(&app, id, "Grilling", serde_json::json!([row])).await,
            ConversationSteered::Companion {
                repo: "askance".to_owned(),
                why,
            },
        );

        // The press did not happen: no row, no checkout, no move, and the stop
        // the click wrote is still there for the human to resume out of.
        let view = opened(&app, id).await;

        assert!(companions(&app, id).await.is_empty(), "{why:?}");
        assert_eq!(view.state, Lifecycle::Grilling, "{why:?}");
        assert_eq!(
            worktrees(&askance).len(),
            1,
            "only the companion repository itself: {why:?}",
        );
        assert!(
            view.blocked_on.is_some(),
            "the stop is still there: {why:?}"
        );
    }
}

/// The three questions the record answers about a companion, each refused by
/// name — and the repository said wherever there is one to say.
///
/// Nothing here changes a row that is already on the Conversation: the frozen
/// set only widens, so a submit naming one that is already there is refused
/// rather than obeyed.
#[tokio::test]
async fn a_repo_a_steer_cannot_put_in_is_refused_by_name() {
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let already = second_repo(&app, elsewhere.path(), "askance").await;
    let id = ready(&app, elsewhere.path(), repo_id).await;

    add_companion(&app, id, already).await;
    companion_mode(&app, id, already, CompanionMode::ReadWrite).await;
    assert_eq!(grill(&app, id).await, GrillingStarted::Started);

    let asked = [
        (
            repo_id,
            ConversationSteered::Companion {
                repo: "verkstead".to_owned(),
                why: SteerCompanionRefusal::OwnRepo,
            },
        ),
        (
            already,
            ConversationSteered::Companion {
                repo: "askance".to_owned(),
                why: SteerCompanionRefusal::AlreadyAdded,
            },
        ),
        (repo_id + 404, ConversationSteered::NoSuchCompanionRepo),
    ];

    for (repo, refusal) in asked {
        // The form stands through a refused submit, so every press after the
        // first finds the one it left.
        assert!(matches!(steer(&app, id).await, SteerOpened::Opened));
        assert_eq!(
            steer_alongside(
                &app,
                id,
                "Grilling",
                serde_json::json!([alongside(repo, "ReadOnly")]),
            )
            .await,
            refusal,
        );

        // And the one that was there is exactly as it was: no downgrade, no
        // removal, and no move.
        let view = opened(&app, id).await;

        assert_eq!(companions(&app, id).await, ["askance"]);
        assert_eq!(companion(&view, "askance").mode, CompanionMode::ReadWrite);
        assert_eq!(view.state, Lifecycle::Grilling);
    }
}

/// The whole of what opening a companion up costs a steer: the row moves to
/// read-write with the branch it was given, a branch is cut off its base as that
/// stands *now*, the detached checkout it was read through is replaced, and a
/// line under the Steer says which repository was opened and on what.
///
/// Both branch names in one press, because they are one rule read two ways: the
/// name typed in the field, and the Conversation's own where nothing was typed.
///
/// **The upgrade is fresh rather than pinned**, which is what the commit made in
/// each companion between the grill start and the steer is here to show: the
/// detached checkout stands at where that repository was when the Conversation
/// started, and the branch is cut from where it stands at the steer.
#[tokio::test]
async fn steering_opens_a_read_only_companion_up() {
    let (elsewhere, dir, app, _repo, repo_id) = workbench().await;
    let named = second_repo(&app, elsewhere.path(), "askance").await;
    let mirroring = second_repo(&app, elsewhere.path(), "granit").await;
    let id = ready(&app, elsewhere.path(), repo_id).await;

    add_companion(&app, id, named).await;
    add_companion(&app, id, mirroring).await;
    assert_eq!(grill(&app, id).await, GrillingStarted::Started);

    let view = opened(&app, id).await;
    let branch = view.branch.clone();
    let askance = elsewhere.path().join("askance");
    let granit = elsewhere.path().join("granit");

    // Where each of them was read through until now: detached, at the commit
    // its base came to when the Conversation started.
    let detached = [checked_out(&view, "askance"), checked_out(&view, "granit")];

    for name in ["askance", "granit"] {
        assert_eq!(companion(&view, name).mode, CompanionMode::ReadOnly);
    }

    // And then both repositories move on while the Conversation runs, which is
    // the whole of what *fresh rather than pinned* means: what the upgrade cuts
    // from is here, and not where the detached checkouts were left.
    let moved_on = [commit(&askance, "halves.md"), commit(&granit, "halves.md")];

    assert_eq!(steer(&app, id).await, SteerOpened::Opened);
    assert_eq!(
        steer_opening(
            &app,
            id,
            "Grilling",
            serde_json::json!([
                { "repo_id": named, "branch": "alongside" },
                opening(mirroring),
            ]),
        )
        .await,
        ConversationSteered::Steered,
    );

    let view = opened(&app, id).await;

    assert_eq!(view.state, Lifecycle::Grilling);
    assert_eq!(companions(&app, id).await, ["askance", "granit"]);

    // The name typed, and the Conversation's own where nothing was — which is
    // what mirroring comes to, exactly as at draft time: the row holds the empty
    // name and the branch that was cut is the Conversation's.
    let cut = [
        ("askance", "alongside", "alongside"),
        ("granit", "", branch.as_str()),
    ];

    for ((name, named, cut), (repo, (was, tip))) in cut.into_iter().zip([
        (&askance, (&detached[0], &moved_on[0])),
        (&granit, (&detached[1], &moved_on[1])),
    ]) {
        let row = companion(&view, name);
        let worked = checked_out(&view, name);

        assert_eq!(row.mode, CompanionMode::ReadWrite, "{name}");
        assert_eq!(row.branch, named, "{name}");
        assert!(has_branch(repo, cut), "{name}");
        assert_eq!(
            git(&worked, &["symbolic-ref", "--short", "HEAD"]).trim(),
            cut,
            "{name} is worked on the branch it was given",
        );

        // Cut from the tip as it stands at the steer rather than from the
        // commit the detached checkout was left at.
        assert_eq!(
            git(&worked, &["rev-parse", "HEAD"]).trim(),
            tip,
            "{name} comes off its base as that stands now",
        );
        assert_eq!(row.base_commit.as_deref(), Some(tip.as_str()), "{name}");

        // One companion is one checkout: the detached directory it was read
        // through is replaced rather than left beside the new one.
        assert_ne!(&worked, was, "{name}");
        assert!(!was.exists(), "{name}'s detached directory is gone");
        assert_eq!(
            worktrees(repo),
            vec![repo.canonicalize().unwrap(), worked.canonicalize().unwrap()],
            "{name} has the repository itself and the one new checkout",
        );
        assert_eq!(
            worked.parent(),
            Some(dir.path().join("worktrees").as_path()),
            "{name}",
        );
    }

    // And the Timeline says which repositories were opened and on what branch,
    // directly under the human's own line.
    let mut after = view
        .timeline
        .iter()
        .skip_while(|event| !matches!(event, TimelineEvent::Steer(_)))
        .skip(1);

    let Some(TimelineEvent::Notice(said)) = after.next() else {
        panic!("what stands under the Steer is the line saying what was opened");
    };

    assert!(
        said.html.contains("askance") && said.html.contains("alongside"),
        "the one that was named says the name it was given: {}",
        said.html,
    );
    assert!(
        said.html.contains("granit") && said.html.contains(&branch),
        "and the mirroring one says the branch mirroring came to: {}",
        said.html,
    );
}

/// Each of the three questions git is asked about an upgrade refuses the whole
/// steer and says which repository it was — and leaves that companion read-only
/// with its checkout exactly where it stood.
///
/// The same three [`alongside`] asks of a companion joining now, because an
/// upgrade *is* one joining now: what it had was a detached checkout of where
/// that repository stood when the Conversation started, and there is no branch
/// in it to be carried forward.
#[tokio::test]
async fn an_upgrade_git_will_not_make_refuses_the_steer_by_name() {
    for why in [
        SteerCompanionRefusal::FetchFailed,
        SteerCompanionRefusal::NoBaseCommit,
        SteerCompanionRefusal::BranchExists,
    ] {
        let (elsewhere, dir, app, _repo, repo_id) = workbench().await;
        let reading = second_repo(&app, elsewhere.path(), "askance").await;
        let id = ready(&app, elsewhere.path(), repo_id).await;
        let askance = elsewhere.path().join("askance");

        add_companion(&app, id, reading).await;

        // A base of its own for the one refusal that needs somewhere to point:
        // the branch is there while the Conversation starts and gone by the
        // steer, which is a base that resolves to nothing.
        if why == SteerCompanionRefusal::NoBaseCommit {
            git(&askance, &["branch", "release-1.4"]);
            companion_base(&app, id, reading, Some("release-1.4")).await;
        }

        assert_eq!(grill(&app, id).await, GrillingStarted::Started);

        let before = opened(&app, id).await;
        let detached = checked_out(&before, "askance");
        let branch = before.branch.clone();

        match why {
            // A remote that answers to nothing: what the branch would come off
            // cannot be trusted to be what the remote is holding.
            SteerCompanionRefusal::FetchFailed => {
                let nowhere = dir.path().join("no-such-remote");
                git(
                    &askance,
                    &["remote", "add", "origin", &nowhere.to_string_lossy()],
                );
            }
            // The base on its row, taken away between the start and the steer.
            SteerCompanionRefusal::NoBaseCommit => {
                git(&askance, &["branch", "-D", "release-1.4"]);
            }
            // And a name in that repository that is already somebody's work.
            _ => {
                git(&askance, &["branch", &branch]);
            }
        }

        // The form stands through a refused submit, so every press after the
        // first finds the one it left — which is what `already` is for.
        assert!(
            matches!(steer(&app, id).await, SteerOpened::Opened),
            "the press stops a drive that is not running and opens the form",
        );
        assert_eq!(
            steer_opening(&app, id, "Grilling", serde_json::json!([opening(reading)])).await,
            ConversationSteered::Companion {
                repo: "askance".to_owned(),
                why,
            },
        );

        // The press did not happen: the row is read-only, its checkout is the
        // detached one it always had, and no move was made.
        let view = opened(&app, id).await;
        let row = companion(&view, "askance");

        assert_eq!(row.mode, CompanionMode::ReadOnly, "{why:?}");
        assert_eq!(row.branch, "", "{why:?}");
        assert_eq!(checked_out(&view, "askance"), detached, "{why:?}");
        assert!(detached.exists(), "{why:?}");
        assert_eq!(
            worktrees(&askance).len(),
            2,
            "the repository itself and the detached checkout, and nothing new: {why:?}",
        );
        assert_eq!(view.state, Lifecycle::Grilling, "{why:?}");
        assert!(
            view.blocked_on.is_some(),
            "the stop is still there: {why:?}"
        );
    }
}

/// And there is no way back down: nothing offers read-only, nothing offers
/// removal, and every way of asking for one of them is refused rather than
/// obeyed.
///
/// Four asks and four refusals. An upgrade of a row that is read-write already
/// has nothing left to open — and obeying it would cut its branch a second time
/// over whatever has been committed to the first, which is the taking-back the
/// whole of this is written to prevent. An upgrade of a Repo the Conversation
/// has not got is the mirror of an add of one it already has. And an *add* of a
/// companion that is there, at read-only, is how a downgrade would have to be
/// spelled if it could be spelled at all.
#[tokio::test]
async fn no_downgrade_and_no_removal_is_obeyed() {
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let writing = second_repo(&app, elsewhere.path(), "askance").await;
    let reading = second_repo(&app, elsewhere.path(), "granit").await;
    let outside = second_repo(&app, elsewhere.path(), "ember").await;
    let id = ready(&app, elsewhere.path(), repo_id).await;

    add_companion(&app, id, writing).await;
    companion_mode(&app, id, writing, CompanionMode::ReadWrite).await;
    add_companion(&app, id, reading).await;
    assert_eq!(grill(&app, id).await, GrillingStarted::Started);

    let view = opened(&app, id).await;
    let worked = checked_out(&view, "askance");
    let detached = checked_out(&view, "granit");

    let asked: Vec<(serde_json::Value, serde_json::Value, ConversationSteered)> = vec![
        // Already as open as a companion gets.
        (
            serde_json::json!([]),
            serde_json::json!([opening(writing)]),
            ConversationSteered::Companion {
                repo: "askance".to_owned(),
                why: SteerCompanionRefusal::AlreadyReadWrite,
            },
        ),
        // And the same by one page naming a read-only one twice, the second row
        // meeting the one this submit has already opened.
        (
            serde_json::json!([]),
            serde_json::json!([opening(reading), opening(reading)]),
            ConversationSteered::Companion {
                repo: "granit".to_owned(),
                why: SteerCompanionRefusal::AlreadyReadWrite,
            },
        ),
        // A registered Repo that is no companion of this Conversation, and the
        // Conversation's own, which is the work's repository rather than
        // something beside it.
        (
            serde_json::json!([]),
            serde_json::json!([opening(outside)]),
            ConversationSteered::Companion {
                repo: "ember".to_owned(),
                why: SteerCompanionRefusal::NotACompanion,
            },
        ),
        (
            serde_json::json!([]),
            serde_json::json!([opening(repo_id)]),
            ConversationSteered::Companion {
                repo: "verkstead".to_owned(),
                why: SteerCompanionRefusal::NotACompanion,
            },
        ),
        // An id nothing answers to at all, which is the refusal with no
        // repository in it.
        (
            serde_json::json!([]),
            serde_json::json!([opening(repo_id + 404)]),
            ConversationSteered::NoSuchCompanionRepo,
        ),
        // And the only way a downgrade could be spelled: an add over the row
        // that is there, at the mode it would be taken back to.
        (
            serde_json::json!([alongside(writing, "ReadOnly")]),
            serde_json::json!([]),
            ConversationSteered::Companion {
                repo: "askance".to_owned(),
                why: SteerCompanionRefusal::AlreadyAdded,
            },
        ),
    ];

    for (added, upgraded, refusal) in asked {
        // The form stands through a refused submit, so every press after the
        // first finds the one it left.
        assert!(matches!(steer(&app, id).await, SteerOpened::Opened));
        assert_eq!(
            steer_companions(&app, id, "Grilling", added, upgraded.clone()).await,
            refusal,
            "{upgraded}",
        );

        // And both that were there are exactly as they were: neither narrowed,
        // neither opened, neither taken away, and each in the directory it had.
        let view = opened(&app, id).await;

        assert_eq!(companions(&app, id).await, ["askance", "granit"]);
        assert_eq!(companion(&view, "askance").mode, CompanionMode::ReadWrite);
        assert_eq!(checked_out(&view, "askance"), worked);
        assert_eq!(companion(&view, "granit").mode, CompanionMode::ReadOnly);
        assert_eq!(checked_out(&view, "granit"), detached);
        assert_eq!(view.state, Lifecycle::Grilling);
    }
}

/// A steered Draft's companions are checked out with its own, which is the
/// source with nothing on disk at all: they were recorded on the setup card and
/// nothing has ever made them.
///
/// Without this the Conversation would reach a running state with companions the
/// sandbox skips in silence — a session quietly missing the repository it was
/// given.
#[tokio::test]
async fn steering_a_draft_checks_out_the_companions_it_was_configured_with() {
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let reading = second_repo(&app, elsewhere.path(), "askance").await;
    let writing = second_repo(&app, elsewhere.path(), "granit").await;
    let id = ready(&app, elsewhere.path(), repo_id).await;

    add_companion(&app, id, reading).await;
    add_companion(&app, id, writing).await;
    companion_mode(&app, id, writing, CompanionMode::ReadWrite).await;

    assert!(
        opened(&app, id)
            .await
            .companions
            .iter()
            .all(|companion| companion.worktree.is_none()),
        "nothing has been checked out while it drafts",
    );

    assert_eq!(steer(&app, id).await, SteerOpened::Opened);
    assert_eq!(
        steer_grilling(&app, id, None).await,
        ConversationSteered::Steered,
    );

    let view = opened(&app, id).await;

    assert_eq!(view.state, Lifecycle::Grilling);
    checked_out(&view, "askance");

    let worked = checked_out(&view, "granit");

    assert_eq!(
        git(&worked, &["symbolic-ref", "--short", "HEAD"]).trim(),
        view.branch,
    );

    // Nothing was added, so there is nothing for the Timeline to announce: what
    // this steer did was make what the record already said.
    assert!(notices(&view).is_empty());
}

/// A steered Draft settles its name the way a grill start does: a name Verkstead
/// invented that the repository already answers to is another Conversation's or
/// a stranger's, never this one's own coming back, so another is invented and
/// the branch already there is left alone.
#[tokio::test]
async fn steering_a_draft_invents_around_a_name_the_repository_holds() {
    let (elsewhere, _dir, app, repo, repo_id) = workbench().await;
    let id = ready(&app, elsewhere.path(), repo_id).await;

    let carried = opened(&app, id).await.branch;

    git(&repo, &["branch", &carried]);
    let stranger = git(&repo, &["rev-parse", &carried]).trim().to_owned();

    assert_eq!(steer(&app, id).await, SteerOpened::Opened);
    assert_eq!(
        steer_grilling(&app, id, None).await,
        ConversationSteered::Steered,
    );

    let view = opened(&app, id).await;

    assert_ne!(view.branch, carried, "the work is on a name of its own");
    assert!(!view.branch_named, "and it is still Verkstead's name");
    assert!(has_branch(&repo, &view.branch));

    let worked = PathBuf::from(view.worktree.expect("a steered Draft gets one").path);

    assert_eq!(
        git(&worked, &["symbolic-ref", "--short", "HEAD"]).trim(),
        view.branch,
        "and the checkout is on it rather than on the branch that was there",
    );
    assert_eq!(
        git(&repo, &["rev-parse", &carried]).trim(),
        stranger,
        "which is where it was left",
    );
}

/// But a Conversation that has worked before picks its own branch back up, name
/// and all — which is the whole point of that path, and what the settling above
/// must not reach.
#[tokio::test]
async fn steering_a_conversation_that_has_worked_keeps_the_branch_it_was_on() {
    let (elsewhere, _dir, app, repo, repo_id) = workbench().await;
    let id = ready(&app, elsewhere.path(), repo_id).await;

    assert_eq!(grill(&app, id).await, GrillingStarted::Started);

    let view = opened(&app, id).await;
    let worked = PathBuf::from(view.worktree.expect("a start makes one").path);
    let branch = view.branch.clone();

    // Closed and steered back: the directory goes and the branch is kept, which
    // is the case a name settled twice would have worked over.
    assert_eq!(close(&app, id).await, ConversationClosed::Closed);
    assert!(!worked.exists());
    assert!(has_branch(&repo, &branch));

    assert_eq!(steer(&app, id).await, SteerOpened::Opened);
    assert_eq!(
        steer_grilling(&app, id, None).await,
        ConversationSteered::Steered,
    );

    let view = opened(&app, id).await;

    assert_eq!(view.branch, branch, "the branch it worked on is its own");

    let back = PathBuf::from(view.worktree.expect("a steer checks it out again").path);

    assert_eq!(
        git(&back, &["symbolic-ref", "--short", "HEAD"]).trim(),
        branch,
    );
}

/// And a Conversation steered back out of Closed gets every companion checked
/// out again, the read-write ones on the branches they kept.
///
/// Closing removed the directories and forgot the rows while keeping the
/// branches, so what is on those branches is what the work committed there — and
/// a branch that is still there is checked out again rather than cut over.
#[tokio::test]
async fn steering_a_closed_conversation_checks_its_companions_out_again() {
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let reading = second_repo(&app, elsewhere.path(), "askance").await;
    let writing = second_repo(&app, elsewhere.path(), "granit").await;
    let id = ready(&app, elsewhere.path(), repo_id).await;

    add_companion(&app, id, reading).await;
    add_companion(&app, id, writing).await;
    companion_mode(&app, id, writing, CompanionMode::ReadWrite).await;

    assert_eq!(grill(&app, id).await, GrillingStarted::Started);

    let branch = opened(&app, id).await.branch;
    let granit = elsewhere.path().join("granit");

    assert_eq!(close(&app, id).await, ConversationClosed::Closed);
    assert!(
        opened(&app, id)
            .await
            .companions
            .iter()
            .all(|companion| companion.worktree.is_none()),
        "the directories went with the close and the rows were forgotten",
    );
    assert!(
        has_branch(&granit, &branch),
        "and the branch the companion was worked on was kept",
    );

    assert_eq!(steer(&app, id).await, SteerOpened::Opened);
    assert_eq!(
        steer_into(&app, id, "Grilling", false).await,
        ConversationSteered::Steered,
    );

    let view = opened(&app, id).await;

    assert_eq!(view.state, Lifecycle::Grilling);
    checked_out(&view, "askance");

    let worked = checked_out(&view, "granit");

    assert_eq!(
        git(&worked, &["symbolic-ref", "--short", "HEAD"]).trim(),
        branch,
        "on the branch it kept rather than one cut over the top of it",
    );
}

/// The whole of what pressing Adopt does: the stage's own branch off the base
/// commit, a worktree with it, the stage brief as the Brief, and a Conversation
/// that is implementing the stage.
#[tokio::test]
async fn adopting_starts_the_stage_on_its_own_branch_off_the_base_commit() {
    let (elsewhere, dir, app, repo, repo_id) = workbench().await;
    roadmap(
        &repo,
        OPEN_AT_THREE,
        &["03-implementation.md", "04-wrap-up.md"],
    );
    let tip = git(&repo, &["rev-parse", "HEAD"]).trim().to_owned();

    let id = ready_to_adopt(&app, elsewhere.path(), repo_id, "mvp").await;

    assert_eq!(press_adopt(&app, id).await, Adopted::Adopted);

    let view = opened(&app, id).await;

    // The stage's own name, rather than the one the server invented for the
    // row: its brief under the roadmap it belongs to.
    assert_eq!(view.branch, "roadmaps/mvp/03-implementation");
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
        git(
            &repo,
            &["rev-parse", "refs/heads/roadmaps/mvp/03-implementation"]
        )
        .trim(),
        tip,
    );

    // And the worktree is git's, under the data directory.
    let worktree = PathBuf::from(view.worktree.expect("a stage has a Worktree").path);

    assert!(worktree.starts_with(dir.path()));
    assert!(worktrees(&repo).contains(&worktree.canonicalize().unwrap()));
    assert!(
        worktree
            .join("docs/roadmaps/mvp/03-implementation.md")
            .exists()
    );
}

/// And the companions the human configured while it drafted are checked out
/// with it, exactly as a grill start's are.
///
/// An adopting Conversation is a Draft like any other, so its setup card put
/// those rows there like any other's — and adoption is the other press that
/// takes a Draft past drafting. Without this the stage would reach Implementing
/// with rows the sandbox skips in silence, which is a session quietly missing a
/// repository the human put there.
#[tokio::test]
async fn adopting_checks_out_the_companions_it_was_configured_with() {
    let (elsewhere, dir, app, repo, repo_id) = workbench().await;
    roadmap(
        &repo,
        OPEN_AT_THREE,
        &["03-implementation.md", "04-wrap-up.md"],
    );

    let reading = second_repo(&app, elsewhere.path(), "askance").await;
    let writing = second_repo(&app, elsewhere.path(), "granit").await;
    let id = ready_to_adopt(&app, elsewhere.path(), repo_id, "mvp").await;

    add_companion(&app, id, reading).await;
    add_companion(&app, id, writing).await;
    companion_mode(&app, id, writing, CompanionMode::ReadWrite).await;

    assert!(
        opened(&app, id)
            .await
            .companions
            .iter()
            .all(|companion| companion.worktree.is_none()),
        "nothing has been checked out while it drafts",
    );

    assert_eq!(press_adopt(&app, id).await, Adopted::Adopted);

    let view = opened(&app, id).await;

    assert_eq!(view.branch, "roadmaps/mvp/03-implementation");
    assert_eq!(view.state, Lifecycle::Implementing);
    assert_eq!(companions(&app, id).await, ["askance", "granit"]);

    // The read-only one is detached at whatever its base came to, and holds no
    // branch in somebody else's repository.
    let askance = elsewhere.path().join("askance");
    let detached = checked_out(&view, "askance");

    assert_eq!(
        git(&detached, &["rev-parse", "HEAD"]).trim(),
        git(&askance, &["rev-parse", "HEAD"]).trim(),
    );
    assert_eq!(
        companion(&view, "askance").base_commit.as_deref(),
        Some(git(&askance, &["rev-parse", "HEAD"]).trim()),
        "and the record says which commit that was, nothing else being able to",
    );
    assert!(!has_branch(&askance, &view.branch));

    // And the read-write one is on a branch of its own, mirroring the stage's
    // own rather than the name the row was invented under.
    let granit = elsewhere.path().join("granit");
    let worked = checked_out(&view, "granit");

    assert_eq!(
        git(&worked, &["symbolic-ref", "--short", "HEAD"]).trim(),
        "roadmaps/mvp/03-implementation",
    );
    assert!(has_branch(&granit, "roadmaps/mvp/03-implementation"));

    // Both under the data directory, and both registered with git — which is
    // what makes them worktrees rather than copies.
    for path in [&detached, &worked] {
        assert_eq!(
            path.parent(),
            Some(dir.path().join("worktrees").as_path()),
            "{path:?}",
        );
    }

    assert!(worktrees(&askance).contains(&detached.canonicalize().unwrap()));
    assert!(worktrees(&granit).contains(&worked.canonicalize().unwrap()));
}

/// And a companion adoption cannot deliver refuses the press by name, leaving
/// the Conversation drafting with nothing checked out anywhere.
///
/// The grill start's four refusals at the other door, said in the words of the
/// press that was made: the human is standing at Adopt, and *which repository*
/// is the whole of what they need.
#[tokio::test]
async fn a_companion_adoption_cannot_deliver_refuses_the_press_by_name() {
    let (elsewhere, _dir, app, repo, repo_id) = workbench().await;
    roadmap(
        &repo,
        OPEN_AT_THREE,
        &["03-implementation.md", "04-wrap-up.md"],
    );

    let writing = second_repo(&app, elsewhere.path(), "askance").await;
    let id = ready_to_adopt(&app, elsewhere.path(), repo_id, "mvp").await;

    add_companion(&app, id, writing).await;
    companion_mode(&app, id, writing, CompanionMode::ReadWrite).await;

    // Somebody else's branch, by the name the companion's would take: the
    // stage's own name, which is what mirroring comes to here.
    let askance = elsewhere.path().join("askance");
    git(&askance, &["branch", "roadmaps/mvp/03-implementation"]);

    assert_eq!(
        press_adopt(&app, id).await,
        Adopted::Companion {
            repo: "askance".to_owned(),
            why: CompanionRefusal::BranchExists,
        },
    );

    // The press did not happen: still drafting, nothing checked out anywhere,
    // and the stage's own branch never cut either — every question is asked
    // before any of them is answered.
    let view = opened(&app, id).await;

    assert_eq!(view.state, Lifecycle::Draft);
    assert!(view.worktree.is_none());
    assert!(companion(&view, "askance").worktree.is_none());
    assert!(!has_branch(&repo, "roadmaps/mvp/03-implementation"));
    assert_eq!(
        worktrees(&askance).len(),
        1,
        "only the companion repository itself",
    );
}

/// A stage adopted from a base that carries a task list arrives without one —
/// the same step a grill start takes, at the other door.
///
/// This is the press the whole feature is about: continuing a roadmap whose
/// last stage stopped part way leaves the list that stage was working on the
/// default branch, and the stage started next would be read as planned the
/// moment its planning session launched.
#[tokio::test]
async fn adopting_clears_the_task_list_the_base_carried() {
    let (elsewhere, _dir, app, repo, repo_id) = workbench().await;
    roadmap(
        &repo,
        OPEN_AT_THREE,
        &["03-implementation.md", "04-wrap-up.md"],
    );
    task_list(&repo, INHERITED);

    let base = git(&repo, &["rev-parse", "HEAD"]).trim().to_owned();
    let id = ready_to_adopt(&app, elsewhere.path(), repo_id, "mvp").await;

    assert_eq!(press_adopt(&app, id).await, Adopted::Adopted);

    let view = opened(&app, id).await;
    let worktree = PathBuf::from(view.worktree.clone().expect("a stage has one").path);

    assert!(
        !worktree.join(".tasks").exists(),
        "the planning session about to run sees no plan",
    );
    assert_eq!(
        tip(&worktree),
        [
            "chore: clear the task list inherited from main",
            "Verkstead Test",
            "test@verkstead.invalid",
        ],
    );
    assert_eq!(
        view.base_commit.as_deref(),
        Some(base.as_str()),
        "the recorded base is still the commit the stage branched off",
    );

    let said = notices(&view).join("\n");

    assert!(
        said.contains("<strong>Grant filters</strong>") && said.contains("1 of 2 entries"),
        "the Timeline says what was cleared: {said:?}",
    );
    assert!(
        said.contains("Stage 03"),
        "beside what was adopted: {said:?}",
    );
}

/// And adopting from a base with no list makes no commit and says nothing.
#[tokio::test]
async fn adopting_from_a_base_with_no_task_list_is_untouched() {
    let (elsewhere, _dir, app, repo, repo_id) = workbench().await;
    roadmap(
        &repo,
        OPEN_AT_THREE,
        &["03-implementation.md", "04-wrap-up.md"],
    );

    let base = git(&repo, &["rev-parse", "HEAD"]).trim().to_owned();
    let id = ready_to_adopt(&app, elsewhere.path(), repo_id, "mvp").await;

    assert_eq!(press_adopt(&app, id).await, Adopted::Adopted);

    let view = opened(&app, id).await;
    let worktree = PathBuf::from(view.worktree.clone().expect("a stage has one").path);

    assert_eq!(git(&worktree, &["rev-parse", "HEAD"]).trim(), base);
    assert!(
        !notices(&view).join("\n").contains("carried a task list"),
        "nothing was cleared, so nothing is said about clearing",
    );
}

/// And with no git author configured the press is refused by name, before the
/// stage's branch or its worktree is made.
#[tokio::test]
async fn adopting_with_no_git_author_is_refused_by_name() {
    let (elsewhere, dir, app, repo, repo_id) = workbench().await;
    roadmap(
        &repo,
        OPEN_AT_THREE,
        &["03-implementation.md", "04-wrap-up.md"],
    );
    no_author(&dir);

    let id = ready_to_adopt(&app, elsewhere.path(), repo_id, "mvp").await;

    assert_eq!(press_adopt(&app, id).await, Adopted::NoGitAuthor);

    let view = opened(&app, id).await;

    assert_eq!(view.state, Lifecycle::Draft);
    assert_eq!(view.worktree, None);
    assert!(!has_branch(&repo, "roadmaps/mvp/03-implementation"));
    assert_eq!(
        worktrees(&repo).len(),
        1,
        "only the repository itself: {:?}",
        worktrees(&repo),
    );
}

/// A stage Conversation steered into a second round is not a stage to adopt
/// again. Adopting is how that work *started*, so a second press is not another
/// adoption: what the steered round has is a Brief of its own, grilled the
/// ordinary way.
#[tokio::test]
async fn a_stage_steered_into_a_second_round_is_not_a_stage_to_adopt_again() {
    let elsewhere = tempfile::tempdir().unwrap();
    let (_dir, app) = app_keeping().await;
    let repo = repository(elsewhere.path().join("verkstead"));

    let registered: Registered =
        post(&app, "/api/ui/repos", &serde_json::json!({ "path": repo })).await;
    assert!(matches!(registered, Registered::Added(_)));

    let repo_id = listed_repos(&app).await;
    roadmap(
        &repo,
        OPEN_AT_THREE,
        &["03-implementation.md", "04-wrap-up.md"],
    );

    let id = ready_to_adopt(&app, elsewhere.path(), repo_id, "mvp").await;
    assert_eq!(press_adopt(&app, id).await, Adopted::Adopted);

    assert_eq!(steer(&app, id).await, SteerOpened::Opened);
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
    let (elsewhere, _dir, app, repo, repo_id) = workbench().await;
    roadmap(
        &repo,
        OPEN_AT_THREE,
        &["03-implementation.md", "04-wrap-up.md"],
    );

    let id = ready_to_adopt(&app, elsewhere.path(), repo_id, "mvp").await;
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
        said.contains("<code>roadmaps/mvp/03-implementation</code>")
            && said.contains("<code>main</code>"),
        "and where its branch came off: {said:?}",
    );
}

/// The stage is read again at the press rather than taken from what the page
/// showed: a base the human fixed to an earlier commit is adopted by the stage
/// that is next *there*.
#[tokio::test]
async fn the_stage_adopted_is_the_one_the_base_commit_has_open() {
    let (elsewhere, _dir, app, repo, repo_id) = workbench().await;
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

    let id = ready_to_adopt(&app, elsewhere.path(), repo_id, "mvp").await;
    assert_eq!(
        base(&app, id, Some("predecessor")).await,
        BaseRecorded::Recorded
    );

    assert_eq!(press_adopt(&app, id).await, Adopted::Adopted);

    let view = opened(&app, id).await;

    assert_eq!(view.branch, "roadmaps/mvp/03-implementation");
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
        git(
            repo,
            &["branch", "--list", "roadmaps/mvp/03-implementation"]
        )
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
    let (elsewhere, _dir, app, repo, repo_id) = workbench().await;
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
        profile(&app, elsewhere.path(), "fable").await,
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
        profile(&app, elsewhere.path(), "opus").await,
    )
    .await;

    assert_eq!(press_adopt(&app, id).await, Adopted::NoReviewProfile);
    nothing_adopted(&app, id, &repo).await;

    choose(
        &app,
        id,
        "review",
        profile(&app, elsewhere.path(), "haiku").await,
    )
    .await;

    assert_eq!(press_adopt(&app, id).await, Adopted::Adopted);
}

/// A Profile whose pair has gone is no account to run a session under, which is
/// a different job from choosing one.
#[tokio::test]
async fn adopting_is_refused_when_a_chosen_profiles_pair_has_gone() {
    let (elsewhere, _dir, app, repo, repo_id) = workbench().await;
    roadmap(
        &repo,
        OPEN_AT_THREE,
        &["03-implementation.md", "04-wrap-up.md"],
    );

    let id = ready_to_adopt(&app, elsewhere.path(), repo_id, "mvp").await;
    std::fs::remove_dir_all(elsewhere.path().join("fable")).unwrap();

    assert_eq!(press_adopt(&app, id).await, Adopted::ProfileBroken);
    nothing_adopted(&app, id, &repo).await;
}

/// A Conversation that began with a Brief and a grilling has no roadmap to take
/// a stage from, and one that has been adopted already has been started once.
#[tokio::test]
async fn only_a_drafting_adopting_conversation_can_be_adopted() {
    let (elsewhere, _dir, app, repo, repo_id) = workbench().await;
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

    let id = ready_to_adopt(&app, elsewhere.path(), repo_id, "mvp").await;

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
    let (elsewhere, _dir, app, repo, repo_id) = workbench().await;
    roadmap(
        &repo,
        OPEN_AT_THREE,
        &["03-implementation.md", "04-wrap-up.md"],
    );

    let id = ready_to_adopt(&app, elsewhere.path(), repo_id, "mvp").await;

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
    let (elsewhere, _dir, app, _repo, upstream, repo_id) = workbench_with_origin().await;

    // The roadmap is committed on origin and nowhere else: this checkout has
    // heard nothing about it, and neither has its copy of `origin/main`.
    roadmap(
        &upstream,
        OPEN_AT_THREE,
        &["03-implementation.md", "04-wrap-up.md"],
    );
    let tip = git(&upstream, &["rev-parse", "HEAD"]).trim().to_owned();

    let id = ready_to_adopt(&app, elsewhere.path(), repo_id, "mvp").await;

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
    let (elsewhere, dir, app, repo, _upstream, repo_id) = workbench_with_origin().await;

    // Committed here, so that what refuses the press is the fetch and not the
    // roadmap being missing.
    roadmap(
        &repo,
        OPEN_AT_THREE,
        &["03-implementation.md", "04-wrap-up.md"],
    );

    let id = ready_to_adopt(&app, elsewhere.path(), repo_id, "mvp").await;

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
    let (elsewhere, _dir, app, repo, repo_id) = workbench().await;
    roadmap(
        &repo,
        OPEN_AT_THREE,
        &["03-implementation.md", "04-wrap-up.md"],
    );

    let id = ready_to_adopt(&app, elsewhere.path(), repo_id, "mvp").await;

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
    let (elsewhere, _dir, app, repo, repo_id) = workbench().await;
    roadmap(
        &repo,
        OPEN_AT_THREE,
        &["03-implementation.md", "04-wrap-up.md"],
    );

    let id = ready_to_adopt(&app, elsewhere.path(), repo_id, "mvp").await;
    git(&repo, &["branch", "roadmaps/mvp/03-implementation"]);

    assert_eq!(press_adopt(&app, id).await, Adopted::BranchExists);

    let view = opened(&app, id).await;
    assert_eq!(view.state, Lifecycle::Draft);
    assert_eq!(view.worktree, None);
    assert_eq!(worktrees(&repo).len(), 1, "only the repository itself");
    assert_eq!(
        git(
            &repo,
            &["rev-parse", "refs/heads/roadmaps/mvp/03-implementation"]
        )
        .trim(),
        git(&repo, &["rev-parse", "HEAD"]).trim(),
        "and the branch that was there is where it was",
    );
}

/// And under the name a stage was given before `roadmaps/` went in front of the
/// scheme, which is where a stage started a week ago still is. Its plan commit
/// ticking the box rides on that branch until its pull request merges, so the
/// branch is the only thing saying the stage is under way — and adopting it a
/// second time would be two Conversations on one stage.
#[tokio::test]
async fn adopting_is_refused_when_the_stage_is_taken_under_its_former_name() {
    let (elsewhere, _dir, app, repo, repo_id) = workbench().await;
    roadmap(
        &repo,
        OPEN_AT_THREE,
        &["03-implementation.md", "04-wrap-up.md"],
    );

    let id = ready_to_adopt(&app, elsewhere.path(), repo_id, "mvp").await;
    git(&repo, &["branch", "mvp/03-implementation"]);

    assert_eq!(press_adopt(&app, id).await, Adopted::BranchExists);
    nothing_adopted(&app, id, &repo).await;
}

/// A branch standing where a component of the stage's own branch path goes is
/// one git will not make at all: it keeps a branch as a file under
/// `refs/heads/`, so `refs/heads/roadmaps` being a file is `refs/heads/roadmaps/`
/// never being a directory.
///
/// The one refusal that carries a name, because it is the one the page cannot
/// work out for itself: everything else it refuses for is the roadmap, the brief
/// or the stage it is already showing, and this is a branch somewhere else in
/// the repository that the human has to go and move.
#[tokio::test]
async fn adopting_is_refused_by_name_when_a_branch_stands_in_the_stages_way() {
    for blocker in ["roadmaps", "roadmaps/mvp"] {
        let (elsewhere, _dir, app, repo, repo_id) = workbench().await;
        roadmap(
            &repo,
            OPEN_AT_THREE,
            &["03-implementation.md", "04-wrap-up.md"],
        );

        let id = ready_to_adopt(&app, elsewhere.path(), repo_id, "mvp").await;
        git(&repo, &["branch", blocker]);

        assert_eq!(
            press_adopt(&app, id).await,
            Adopted::BranchInTheWay {
                by: blocker.to_owned(),
            },
        );
        nothing_adopted(&app, id, &repo).await;
    }
}

/// A roadmap the base commit knows nothing about is not a roadmap that
/// finished, and saying so would send the human looking at the wrong document.
#[tokio::test]
async fn adopting_is_refused_when_no_such_roadmap_is_at_the_base() {
    let (elsewhere, _dir, app, repo, repo_id) = workbench().await;
    roadmap(
        &repo,
        OPEN_AT_THREE,
        &["03-implementation.md", "04-wrap-up.md"],
    );

    let id = ready_to_adopt(&app, elsewhere.path(), repo_id, "public-release").await;

    assert_eq!(press_adopt(&app, id).await, Adopted::NoRoadmap);
    nothing_adopted(&app, id, &repo).await;
}

/// Cheap-first, and provably so: a Conversation with no Profiles, against a
/// roadmap that has finished and whose branch is taken besides, is answered
/// about its Profiles. Everything git is paid for is asked after the record's
/// own state and the pair of accounts it would run under.
#[tokio::test]
async fn the_cheap_refusals_are_answered_before_the_ones_git_is_paid_for() {
    let (_elsewhere, _dir, app, repo, repo_id) = workbench().await;
    roadmap(&repo, ALL_DONE, &["03-implementation.md"]);
    git(&repo, &["branch", "roadmaps/mvp/03-implementation"]);

    let id = adopting(&app, repo_id, "mvp").await;

    assert_eq!(press_adopt(&app, id).await, Adopted::NoGrillingProfile);
    assert_eq!(
        opened(&app, id).await.state,
        Lifecycle::Draft,
        "and nothing about the roadmap was read to find that out",
    );
}

/// The press on a Draft holding a pull request, sent as its page sends it.
async fn press_take_up(app: &Router, id: i64) -> TakenUp {
    post(
        app,
        &format!("/api/ui/conversations/{id}/take-up"),
        &serde_json::json!({}),
    )
    .await
}

/// Everything a take-up needs before the press: the two Profiles the wrap-up
/// runs under, which is the whole of what a Conversation holding a pull request
/// has to settle — its Brief was written on the compose page, and its branch and
/// its base are the pull request's.
///
/// Two rather than three, and no grilling among them: the work on a pull request
/// is built, so there is no round for one to open and the picker is drawn
/// nowhere.
async fn ready_to_take_up(app: &Router, elsewhere: &Path, repo_id: i64, number: i64) -> i64 {
    let id = wrapping_up(app, repo_id, number).await;

    let implementation = profile(app, elsewhere, "opus").await;
    let review = profile(app, elsewhere, "haiku").await;
    choose(app, id, "implementation", implementation).await;
    choose(app, id, "review", review).await;

    id
}

/// Put the pull request's head branch on the upstream, which is where a pull
/// request's branch lives: this checkout has heard nothing about it until it
/// fetches.
///
/// Hands back what it stands at.
fn head_on_origin(upstream: &Path, branch: &str) -> String {
    git(upstream, &["checkout", "-b", branch]);
    let head = commit(upstream, "limits.md");
    git(upstream, &["checkout", "main"]);

    head
}

/// What a refused press has to leave behind: a Conversation that has not moved,
/// nothing checked out, and no pull request on its record.
async fn nothing_taken_up(app: &Router, id: i64, repo: &Path) {
    let view = opened(app, id).await;

    assert_eq!(view.state, Lifecycle::Draft);
    assert_eq!(view.worktree, None);
    assert!(
        worktrees(repo).len() == 1,
        "only the repository itself is checked out anywhere",
    );
    assert!(
        !view
            .timeline
            .iter()
            .any(|event| matches!(event, TimelineEvent::PullRequest(_))),
        "and nothing was pinned on the Timeline",
    );
}

/// The whole of what the press does where nothing local stands in the way: the
/// head branch cut off origin's and tracking it, a worktree on it, the pull
/// request pinned, and a Conversation that says it is wrapping up.
#[tokio::test]
async fn taking_a_pull_request_up_checks_its_head_branch_out_and_wraps_it_up() {
    let (elsewhere, dir, app, repo, upstream, repo_id) = workbench_with_origin().await;
    let head = head_on_origin(&upstream, "rate-limiting");

    let id = ready_to_take_up(&app, elsewhere.path(), repo_id, 41).await;

    assert_eq!(press_take_up(&app, id).await, TakenUp::TakenUp);

    let view = opened(&app, id).await;
    assert_eq!(view.state, Lifecycle::Wrapping);
    assert_eq!(moves(&view), [Lifecycle::Wrapping]);
    assert_eq!(
        view.branch, "rate-limiting",
        "the Conversation is named for the pull request's branch, settled rather than invented",
    );

    // The branch is in the Repo's own git directory, cut off origin's copy of it
    // — and tracking that copy, so a push from the wrap-up goes where the pull
    // request is.
    assert_eq!(
        git(&repo, &["rev-parse", "refs/heads/rate-limiting"]).trim(),
        head,
    );
    assert_eq!(
        git(
            &repo,
            &["rev-parse", "--abbrev-ref", "rate-limiting@{upstream}"]
        )
        .trim(),
        "origin/rate-limiting",
    );

    // And a worktree on it, under the data directory where every other one goes.
    let worktree = view
        .worktree
        .clone()
        .expect("a taken-up Conversation has a worktree");
    let path = PathBuf::from(&worktree.path);
    assert!(!worktree.missing);
    assert_eq!(path.parent(), Some(dir.path().join("worktrees").as_path()));
    assert_eq!(
        git(&path, &["symbolic-ref", "--short", "HEAD"]).trim(),
        "rate-limiting",
    );
    assert!(path.join("limits.md").is_file(), "the branch's own work");

    // The pull request is pinned, and the two Pairings the wrap-up runs under
    // are on the record.
    let pinned = view
        .pinned
        .iter()
        .find_map(|event| match event {
            PinnedEvent::PullRequest(opened) => Some(opened),
            _ => None,
        })
        .expect("a wrapping Conversation pins its pull request");

    assert_eq!(pinned.number, 41);
    assert_eq!(pinned.title, "Rate limiting for the public API");
    assert_eq!(pinned.url, "https://github.com/tobico/verkstead/pull/41");

    assert!(view.implementation_pairing.is_some());
    assert!(matches!(view.review_pairing, PickedView::Under(_)));
}

/// The base recorded is the head *at take-up*, with GitHub's base branch beside
/// it — which is what leaves the pull request's own commits off this Timeline
/// and keeps them off it when the base branch moves.
///
/// The reading asserted here is the commit sweep's own: everything on the branch
/// that is not on the base commit and not on the base branch. See the server's
/// `commits` module, where that list is built.
#[tokio::test]
async fn a_taken_up_conversation_draws_none_of_the_pull_requests_own_commits() {
    let (elsewhere, _dir, app, repo, upstream, repo_id) = workbench_with_origin().await;
    let head = head_on_origin(&upstream, "rate-limiting");

    let id = ready_to_take_up(&app, elsewhere.path(), repo_id, 41).await;
    assert_eq!(press_take_up(&app, id).await, TakenUp::TakenUp);

    let view = opened(&app, id).await;
    assert_eq!(
        view.base_commit.as_deref(),
        Some(head.as_str()),
        "the pull request's head at take-up",
    );

    assert!(
        view.timeline
            .iter()
            .all(|event| !matches!(event, TimelineEvent::Commit(_))),
        "and nothing already on the pull request is drawn as this Conversation's",
    );

    // The base branch gains a commit, which is the ordinary thing for it to do
    // while a wrap-up runs.
    commit(&upstream, "unrelated.md");
    git(&repo, &["fetch", "origin"]);

    assert!(
        git(
            &repo,
            &[
                "rev-list",
                "rate-limiting",
                &format!("^{head}"),
                "^main",
                "^origin/main",
            ],
        )
        .trim()
        .is_empty(),
        "and the sweep's own reading still finds nothing to draw",
    );
}

/// A local branch of that name standing behind origin's is caught up and taken:
/// the pull request's work is on the remote, and this checkout's copy is a copy.
#[tokio::test]
async fn a_head_branch_behind_origin_is_fast_forwarded_and_taken() {
    let (elsewhere, _dir, app, repo, upstream, repo_id) = workbench_with_origin().await;
    let head = head_on_origin(&upstream, "rate-limiting");

    // The checkout has an older copy of it: the branch as it was before the last
    // commit landed on origin.
    git(&repo, &["fetch", "origin"]);
    let behind = git(&repo, &["rev-parse", "origin/rate-limiting~1"])
        .trim()
        .to_owned();
    git(&repo, &["branch", "rate-limiting", &behind]);

    let id = ready_to_take_up(&app, elsewhere.path(), repo_id, 41).await;

    assert_eq!(press_take_up(&app, id).await, TakenUp::TakenUp);

    assert_eq!(
        git(&repo, &["rev-parse", "refs/heads/rate-limiting"]).trim(),
        head,
        "the branch was moved on to origin's rather than worked at where it was",
    );

    assert_eq!(
        upstream_of(&repo, "rate-limiting"),
        "origin/rate-limiting",
        "and it was pointed at origin's, the `git branch` that made it having \
         left it tracking nothing",
    );

    let view = opened(&app, id).await;
    assert_eq!(view.state, Lifecycle::Wrapping);
    assert_eq!(view.base_commit.as_deref(), Some(head.as_str()));
}

/// And one already level with origin's is taken where it stands — and pointed
/// at origin's all the same, whether or not whoever made it left it tracking
/// anything.
///
/// Which is the whole of what a branch that was already here needs and a branch
/// cut off origin's gets for nothing. The wrap-up's sessions push with a bare
/// `git push`, and what gives every other Conversation its upstream is the
/// implementing session's `push -u`: a take-up runs no such session, so this is
/// the only place a branch like this can get one.
#[tokio::test]
async fn a_head_branch_level_with_origin_is_taken_where_it_stands_and_pointed_at_it() {
    let (elsewhere, _dir, app, repo, upstream, repo_id) = workbench_with_origin().await;
    let head = head_on_origin(&upstream, "rate-limiting");

    // Made off the commit rather than off `origin/rate-limiting`, which is what
    // leaves it tracking nothing at all — a `git branch` off a sha, or a fetch
    // into a name.
    git(&repo, &["fetch", "origin"]);
    git(&repo, &["branch", "rate-limiting", &head]);
    assert_eq!(
        upstream_of(&repo, "rate-limiting"),
        "",
        "the branch this is about is one nothing has pointed anywhere",
    );

    let id = ready_to_take_up(&app, elsewhere.path(), repo_id, 41).await;

    assert_eq!(press_take_up(&app, id).await, TakenUp::TakenUp);

    assert_eq!(
        git(&repo, &["rev-parse", "refs/heads/rate-limiting"]).trim(),
        head,
        "the branch was taken where it stood rather than moved",
    );
    assert_eq!(upstream_of(&repo, "rate-limiting"), "origin/rate-limiting");

    // Which is the point of it: the worktree the wrap-up works in can push.
    let worktree = opened(&app, id)
        .await
        .worktree
        .expect("a taken-up Conversation has a worktree");

    assert_eq!(
        git(
            &PathBuf::from(&worktree.path),
            &["rev-parse", "--abbrev-ref", "HEAD@{upstream}"],
        )
        .trim(),
        "origin/rate-limiting",
    );
}

/// What `branch` is pointed at in `repo`, or the empty string where nothing has
/// pointed it anywhere.
///
/// Asked as a listing rather than as `@{upstream}`, which is a revision git
/// fails on when there is none: what is being told apart here is *tracking* from
/// *not*, and both are answers.
fn upstream_of(repo: &Path, branch: &str) -> String {
    git(
        repo,
        &["branch", "--format=%(upstream:short)", "--list", branch],
    )
    .trim()
    .to_owned()
}

/// And one that is ahead of origin is refused by name. Those commits are
/// somebody's and unpushed, and moving the branch under them would be Verkstead
/// throwing work away.
#[tokio::test]
async fn a_head_branch_ahead_of_origin_refuses_the_press_by_name() {
    let (elsewhere, _dir, app, repo, upstream, repo_id) = workbench_with_origin().await;
    head_on_origin(&upstream, "rate-limiting");

    git(&repo, &["fetch", "origin"]);
    git(
        &repo,
        &["checkout", "-b", "rate-limiting", "origin/rate-limiting"],
    );
    let ahead = commit(&repo, "unpushed.md");
    git(&repo, &["checkout", "main"]);

    let id = ready_to_take_up(&app, elsewhere.path(), repo_id, 41).await;

    assert_eq!(press_take_up(&app, id).await, TakenUp::BranchAhead);
    nothing_taken_up(&app, id, &repo).await;
    assert_eq!(
        git(&repo, &["rev-parse", "refs/heads/rate-limiting"]).trim(),
        ahead,
        "and the branch is exactly where it was",
    );
}

/// And one that has gone its own way is refused as that rather than as either
/// of the two beside it: each of the branches holds commits the other has not,
/// so there is no fast-forward to be had and nothing here for Verkstead to
/// decide.
///
/// Which is the third of the three ways a local head branch can stand against
/// origin's, and it is the arm that never takes a branch it should not — a git
/// that would not say how the two stand reads as this as well. Told apart from
/// *ahead* by asking the same containment the other way round, so the two are
/// worth proving apart.
#[tokio::test]
async fn a_head_branch_that_has_diverged_from_origin_refuses_the_press_by_name() {
    let (elsewhere, _dir, app, repo, upstream, repo_id) = workbench_with_origin().await;
    head_on_origin(&upstream, "rate-limiting");

    // A commit of its own on each side of the same branch, which is the whole
    // of *diverged*: neither tip has the other in its history.
    git(&repo, &["fetch", "origin"]);
    git(
        &repo,
        &["checkout", "-b", "rate-limiting", "origin/rate-limiting"],
    );
    let mine = commit(&repo, "unpushed.md");
    git(&repo, &["checkout", "main"]);

    git(&upstream, &["checkout", "rate-limiting"]);
    commit(&upstream, "theirs.md");
    git(&upstream, &["checkout", "main"]);

    let id = ready_to_take_up(&app, elsewhere.path(), repo_id, 41).await;

    assert_eq!(press_take_up(&app, id).await, TakenUp::BranchDiverged);
    nothing_taken_up(&app, id, &repo).await;
    assert_eq!(
        git(&repo, &["rev-parse", "refs/heads/rate-limiting"]).trim(),
        mine,
        "and the branch is exactly where it was",
    );
}

/// A branch somebody is standing on is refused naming the place: git holds one
/// checkout per branch, and *which one* is the whole of what the human needs.
#[tokio::test]
async fn a_head_branch_checked_out_elsewhere_refuses_naming_the_place() {
    let (elsewhere, _dir, app, repo, upstream, repo_id) = workbench_with_origin().await;
    head_on_origin(&upstream, "rate-limiting");

    git(&repo, &["fetch", "origin"]);
    git(&repo, &["branch", "rate-limiting", "origin/rate-limiting"]);

    // A worktree of the human's own, beside the checkout Verkstead knows about.
    let theirs = elsewhere.path().join("their-worktree");
    git(
        &repo,
        &[
            "worktree",
            "add",
            &theirs.to_string_lossy(),
            "rate-limiting",
        ],
    );

    let id = ready_to_take_up(&app, elsewhere.path(), repo_id, 41).await;

    let refused = press_take_up(&app, id).await;
    let TakenUp::CheckedOutElsewhere { at } = &refused else {
        panic!("expected the place to be named, got {refused:?}");
    };

    assert_eq!(
        PathBuf::from(at).canonicalize().unwrap(),
        theirs.canonicalize().unwrap(),
    );

    let view = opened(&app, id).await;
    assert_eq!(view.state, Lifecycle::Draft);
    assert_eq!(view.worktree, None);
}

/// The two Profiles the wrap-up runs under are fixed before the press, and no
/// third: a pull request has no round for a grilling to open.
#[tokio::test]
async fn taking_up_is_refused_by_name_when_a_profile_is_unchosen() {
    let (elsewhere, _dir, app, repo, upstream, repo_id) = workbench_with_origin().await;
    head_on_origin(&upstream, "rate-limiting");

    let id = wrapping_up(&app, repo_id, 41).await;

    assert_eq!(
        press_take_up(&app, id).await,
        TakenUp::NoImplementationProfile
    );
    nothing_taken_up(&app, id, &repo).await;

    choose(
        &app,
        id,
        "implementation",
        profile(&app, elsewhere.path(), "opus").await,
    )
    .await;

    assert_eq!(press_take_up(&app, id).await, TakenUp::NoReviewProfile);
    nothing_taken_up(&app, id, &repo).await;

    choose(
        &app,
        id,
        "review",
        profile(&app, elsewhere.path(), "haiku").await,
    )
    .await;

    assert_eq!(
        press_take_up(&app, id).await,
        TakenUp::TakenUp,
        "and never a word about the grilling, which this page never asked for",
    );
}

/// A Conversation that began with a Brief and a grilling is holding no pull
/// request, and one that has been taken up already has been started once.
#[tokio::test]
async fn only_a_drafting_conversation_holding_one_can_be_taken_up() {
    let (elsewhere, _dir, app, repo, upstream, repo_id) = workbench_with_origin().await;
    head_on_origin(&upstream, "rate-limiting");

    let ordinary = started(&app, repo_id).await;

    assert_eq!(
        press_take_up(&app, ordinary).await,
        TakenUp::NotHoldingOne,
        "and it is answered before the Profiles are, which it has none of",
    );

    let id = ready_to_take_up(&app, elsewhere.path(), repo_id, 41).await;

    assert_eq!(press_take_up(&app, id).await, TakenUp::TakenUp);
    assert_eq!(
        press_take_up(&app, id).await,
        TakenUp::NotDrafting,
        "two worktrees on one branch is what taking it up twice would mean",
    );

    assert_eq!(worktrees(&repo).len(), 2, "the repository and one worktree");
}

/// Origin having no branch by the name GitHub gave is its own refusal: the
/// branch was deleted, or was never pushed to this remote, and either way there
/// is nothing here to check out.
#[tokio::test]
async fn taking_up_is_refused_when_origin_has_no_such_branch() {
    let (elsewhere, _dir, app, repo, _upstream, repo_id) = workbench_with_origin().await;

    let id = ready_to_take_up(&app, elsewhere.path(), repo_id, 41).await;

    assert_eq!(press_take_up(&app, id).await, TakenUp::NoHeadBranch);
    nothing_taken_up(&app, id, &repo).await;
}

/// There is a human at this button, so a fetch git would not make refuses the
/// press by name rather than taking a branch up against refs nobody can vouch
/// for.
#[tokio::test]
async fn taking_up_is_refused_by_name_when_the_fetch_fails() {
    let (elsewhere, dir, app, repo, upstream, repo_id) = workbench_with_origin().await;
    head_on_origin(&upstream, "rate-limiting");

    let id = ready_to_take_up(&app, elsewhere.path(), repo_id, 41).await;

    let nowhere = dir.path().join("no-such-remote");
    git(
        &repo,
        &["remote", "set-url", "origin", &nowhere.to_string_lossy()],
    );

    assert_eq!(press_take_up(&app, id).await, TakenUp::FetchFailed);
    nothing_taken_up(&app, id, &repo).await;
}

/// A `gh` that answers about a pull request by number out of a file in the Repo
/// it is run in — `pr-41.json` for `#41`, and `gh`'s own way of saying there is
/// nothing there where no such file was written.
///
/// A stand-in for a program is a program, which is what keeps this off Windows;
/// the `pull_requests` suite is off it for the same reason. `sh -c` gives `$0`
/// the script's own name, so what Verkstead passes lands in `$1` onwards and the
/// number of `pr view <n> --json …` is `$3`.
#[cfg(unix)]
fn gh_answering() -> Gh {
    Gh::running(vec![
        "/bin/sh".to_owned(),
        "-c".to_owned(),
        r#"if [ -f "./pr-$3.json" ]; then cat "./pr-$3.json"; exit 0; fi
           echo 'no pull requests found' >&2
           exit 1"#
            .to_owned(),
        "gh".to_owned(),
    ])
}

/// Put an open pull request where that `gh` will find it.
#[cfg(unix)]
fn opened_on_github(repo: &Path, number: i64, head: &str) {
    on_github(repo, number, head, "main", "OPEN", false);
}

/// And one in whichever state, into whichever branch, from a fork or not —
/// which is the whole of what GitHub says that a take-up turns on.
#[cfg(unix)]
fn on_github(repo: &Path, number: i64, head: &str, base: &str, state: &str, fork: bool) {
    let said = serde_json::json!({
        "number": number,
        "title": "Rate limiting for the public API",
        // Where it is, which is also GitHub's own statement of which repository
        // it answered about: a URL in the Brief is checked against this.
        "url": format!("https://github.com/tobico/verkstead/pull/{number}"),
        "headRefName": head,
        "baseRefName": base,
        "isCrossRepository": fork,
        "state": state,
    });

    std::fs::write(repo.join(format!("pr-{number}.json")), said.to_string()).unwrap();
}

/// Everything a Review needs before the press: the Process picked, the Brief
/// that names the target, and the two Profiles the wrap-up runs under.
///
/// Two rather than three, and no grilling among them: the work on a pull request
/// is built, so there is no round for one to open and the picker is drawn
/// nowhere.
#[cfg(unix)]
async fn ready_to_review(app: &Router, elsewhere: &Path, repo_id: i64, brief: &str) -> i64 {
    let implementation = profile(app, elsewhere, "opus").await;
    let review = profile(app, elsewhere, "haiku").await;

    ready_to_review_under(app, repo_id, brief, implementation, review).await
}

/// The same over Profiles already saved, which is what a test wanting two
/// Review drafts wants: a Profile's name is unique across the workbench, so
/// saving the pair twice is a refusal rather than a second pair.
#[cfg(unix)]
async fn ready_to_review_under(
    app: &Router,
    repo_id: i64,
    brief: &str,
    implementation: i64,
    review: i64,
) -> i64 {
    let id = started(app, repo_id).await;

    assert_eq!(
        pick_process(app, id, Process::Review).await,
        ProcessPicked::Picked
    );
    assert_eq!(write_brief(app, id, brief).await, BriefSaved::Saved);

    choose(app, id, "implementation", implementation).await;
    choose(app, id, "review", review).await;

    id
}

/// A Review's Start is the take-up, and what it takes up is the pull request its
/// Brief names by URL: the head branch checked out, the pull request recorded,
/// and the Conversation wrapping it up.
///
/// The whole of the press in one test, because the whole of it is one act —
/// the Brief filling the Target as it is saved, the field read, GitHub asked,
/// and today's take-up run on the answer.
#[cfg(unix)]
#[tokio::test]
async fn a_review_takes_up_the_pull_request_its_brief_names() {
    let (elsewhere, dir, app, repo, upstream, repo_id) = workbench_reviewing().await;
    let head = head_on_origin(&upstream, "rate-limiting");
    opened_on_github(&repo, 41, "rate-limiting");

    let id = ready_to_review(
        &app,
        elsewhere.path(),
        repo_id,
        "Wrap up https://github.com/tobico/verkstead/pull/41 — the limiter needs a read.\n",
    )
    .await;

    assert_eq!(press_take_up(&app, id).await, TakenUp::TakenUp);

    let view = opened(&app, id).await;
    assert_eq!(view.state, Lifecycle::Wrapping);
    assert_eq!(view.process, Process::Review);
    assert_eq!(
        view.branch, "rate-limiting",
        "the Conversation is named for the pull request's branch, which take-up decided",
    );

    // The base is the head at take-up, with GitHub's base branch beside it, so
    // the Timeline draws only what Verkstead adds from here.
    assert_eq!(view.base_commit.as_deref(), Some(head.as_str()));

    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();
    let record = store::load_conversation(&pool, id)
        .await
        .unwrap()
        .expect("it is on the record");
    assert_eq!(
        record.base_ref.as_deref(),
        Some("main"),
        "the branch GitHub says it merges into, which is what a conflict is measured against",
    );

    let worktree = view
        .worktree
        .clone()
        .expect("a taken-up Conversation has a worktree");
    let path = PathBuf::from(&worktree.path);
    assert_eq!(path.parent(), Some(dir.path().join("worktrees").as_path()));
    assert_eq!(
        git(&path, &["symbolic-ref", "--short", "HEAD"]).trim(),
        "rate-limiting",
    );

    // And the pull request is on the record, pinned as every wrapping
    // Conversation's is — written out of what `gh` answered rather than out of a
    // row that was pressed.
    let pinned = view
        .pinned
        .iter()
        .find_map(|event| match event {
            PinnedEvent::PullRequest(opened) => Some(opened),
            _ => None,
        })
        .expect("a wrapping Conversation pins its pull request");

    assert_eq!(pinned.number, 41);
    assert_eq!(pinned.title, "Rate limiting for the public API");
    assert_eq!(pinned.url, "https://github.com/tobico/verkstead/pull/41");
}

/// And a bare `#number` names it just as well: a number is unambiguous in the
/// repository it is read in, which is the Conversation's own Repo.
#[cfg(unix)]
#[tokio::test]
async fn a_review_takes_up_a_pull_request_its_brief_names_by_number() {
    let (elsewhere, _dir, app, repo, upstream, repo_id) = workbench_reviewing().await;
    head_on_origin(&upstream, "rate-limiting");
    opened_on_github(&repo, 41, "rate-limiting");

    let id = ready_to_review(
        &app,
        elsewhere.path(),
        repo_id,
        "# Rate limiting\n\nPlease wrap #41 up.\n",
    )
    .await;

    assert_eq!(press_take_up(&app, id).await, TakenUp::TakenUp);

    let view = opened(&app, id).await;
    assert_eq!(view.state, Lifecycle::Wrapping);
    assert_eq!(view.branch, "rate-limiting");
}

/// A Review whose Brief names no pull request leaves the Target empty, and the
/// press is refused by its own name. Inert on the page while the field is
/// empty; this is the answer a page whose copy of the world went stale gets.
#[cfg(unix)]
#[tokio::test]
async fn a_review_whose_brief_names_nothing_is_refused_by_name() {
    let (elsewhere, _dir, app, repo, upstream, repo_id) = workbench_reviewing().await;
    head_on_origin(&upstream, "rate-limiting");
    opened_on_github(&repo, 41, "rate-limiting");

    let id = ready_to_review(
        &app,
        elsewhere.path(),
        repo_id,
        "# Rate limiting\n\nThe public API wants a ceiling on it.\n",
    )
    .await;

    assert_eq!(press_take_up(&app, id).await, TakenUp::NoTarget);
    nothing_taken_up(&app, id, &repo).await;
}

/// A URL naming another repository is refused naming it: `gh` answers for this
/// Repo's origin, so a number asked of it would be a pull request of somewhere
/// else entirely — or nothing at all.
#[cfg(unix)]
#[tokio::test]
async fn a_review_naming_another_repositorys_pull_request_is_refused_by_name() {
    let (elsewhere, _dir, app, repo, upstream, repo_id) = workbench_reviewing().await;
    head_on_origin(&upstream, "rate-limiting");
    opened_on_github(&repo, 41, "rate-limiting");

    let id = ready_to_review(
        &app,
        elsewhere.path(),
        repo_id,
        "Wrap up https://github.com/tobico/askance/pull/41.\n",
    )
    .await;

    assert_eq!(
        press_take_up(&app, id).await,
        TakenUp::AnotherRepository {
            named: "tobico/askance".to_owned(),
        },
    );
    nothing_taken_up(&app, id, &repo).await;
}

/// A number GitHub has nothing open under is refused by name, whether it never
/// existed or has since been merged — the two being one thing to the human and
/// told apart only because one of them is `gh` failing.
#[cfg(unix)]
#[tokio::test]
async fn a_review_naming_a_number_nothing_is_open_under_is_refused_by_name() {
    let (elsewhere, _dir, app, repo, upstream, repo_id) = workbench_reviewing().await;
    head_on_origin(&upstream, "rate-limiting");

    let implementation = profile(&app, elsewhere.path(), "opus").await;
    let review = profile(&app, elsewhere.path(), "haiku").await;

    let nothing =
        ready_to_review_under(&app, repo_id, "Wrap #41 up.\n", implementation, review).await;

    assert_eq!(
        press_take_up(&app, nothing).await,
        TakenUp::NoSuchPullRequest { number: 41 },
        "nothing was ever opened under it",
    );
    nothing_taken_up(&app, nothing, &repo).await;

    on_github(&repo, 41, "rate-limiting", "main", "MERGED", false);

    let merged =
        ready_to_review_under(&app, repo_id, "Wrap #41 up.\n", implementation, review).await;

    assert_eq!(
        press_take_up(&app, merged).await,
        TakenUp::NoSuchPullRequest { number: 41 },
        "and it is merged, which is nothing to wrap up either",
    );
    nothing_taken_up(&app, merged, &repo).await;
}

/// A pull request from a fork is refused by name: its head branch is in another
/// repository, so nothing a wrap-up fixed could be pushed to it.
#[cfg(unix)]
#[tokio::test]
async fn a_review_over_a_fork_is_refused_by_name() {
    let (elsewhere, _dir, app, repo, upstream, repo_id) = workbench_reviewing().await;
    head_on_origin(&upstream, "rate-limiting");
    on_github(&repo, 41, "rate-limiting", "main", "OPEN", true);

    let id = ready_to_review(&app, elsewhere.path(), repo_id, "Wrap #41 up.\n").await;

    assert_eq!(press_take_up(&app, id).await, TakenUp::Fork);
    nothing_taken_up(&app, id, &repo).await;
}

/// And the branch refusals are the same road: a Review's press runs today's
/// take-up on what GitHub answered, so a head branch that has gone its own way
/// is refused here by exactly the name it is refused by at the other door.
#[cfg(unix)]
#[tokio::test]
async fn a_review_whose_head_branch_has_diverged_is_refused_by_name() {
    let (elsewhere, _dir, app, repo, upstream, repo_id) = workbench_reviewing().await;
    head_on_origin(&upstream, "rate-limiting");
    opened_on_github(&repo, 41, "rate-limiting");

    // A local branch of that name with a commit of its own on it, off a base
    // origin's copy never had.
    git(&repo, &["checkout", "-b", "rate-limiting"]);
    commit(&repo, "elsewhere.md");
    git(&repo, &["checkout", "main"]);

    let id = ready_to_review(&app, elsewhere.path(), repo_id, "Wrap #41 up.\n").await;

    assert_eq!(press_take_up(&app, id).await, TakenUp::BranchDiverged);
    assert_eq!(opened(&app, id).await.state, Lifecycle::Draft);
}

/// And a pull request another Conversation is already on is refused naming that
/// Conversation: there is one Conversation per piece of work, so the way on is
/// the one that has it rather than a second wrap-up over the same branch.
#[cfg(unix)]
#[tokio::test]
async fn a_review_over_a_pull_request_another_conversation_holds_leads_there() {
    let (elsewhere, _dir, app, repo, upstream, repo_id) = workbench_reviewing().await;
    head_on_origin(&upstream, "rate-limiting");
    opened_on_github(&repo, 41, "rate-limiting");

    let implementation = profile(&app, elsewhere.path(), "opus").await;
    let review = profile(&app, elsewhere.path(), "haiku").await;

    let first =
        ready_to_review_under(&app, repo_id, "Wrap #41 up.\n", implementation, review).await;
    assert_eq!(press_take_up(&app, first).await, TakenUp::TakenUp);

    let second =
        ready_to_review_under(&app, repo_id, "Wrap #41 up too.\n", implementation, review).await;

    assert_eq!(
        press_take_up(&app, second).await,
        TakenUp::AlreadyHeld {
            conversation: first,
        },
    );
    assert_eq!(opened(&app, second).await.state, Lifecycle::Draft);
    assert_eq!(opened(&app, second).await.worktree, None);
}

/// A Review whose Target is a branch on origin lands Wrapping over that branch
/// with no pull request recorded at all: the same take-up, the head at take-up as
/// the base commit, and the base the picker holds as the branch it goes into.
///
/// Which is the whole of what a bare branch changes about the press. The work is
/// built and pushed and nobody opened anything, so there is nothing for GitHub to
/// be asked and nothing to record — and the move into Wrapping is the take-up's
/// own, recording a pull request being the door every other ending comes through.
#[cfg(unix)]
#[tokio::test]
async fn a_review_takes_up_a_branch_on_origin_with_no_pull_request() {
    let (elsewhere, dir, app, repo, upstream, repo_id) = workbench_reviewing().await;
    let head = head_on_origin(&upstream, "rate-limiting");

    // A branch of the repository's own for the base to be picked out of, so that
    // what is recorded beside the base commit is provably the picker's choice
    // rather than the default branch the rule would have fallen to.
    git(&repo, &["branch", "release/2.1"]);

    let id = ready_to_review(
        &app,
        elsewhere.path(),
        repo_id,
        "# Rate limiting\n\nThe limiter is built and on no pull request.\n",
    )
    .await;

    assert_eq!(
        name_target(&app, id, "rate-limiting").await,
        TargetRecorded::Recorded,
    );
    assert_eq!(
        base(&app, id, Some("release/2.1")).await,
        BaseRecorded::Recorded
    );

    assert_eq!(press_take_up(&app, id).await, TakenUp::TakenUp);

    let view = opened(&app, id).await;
    assert_eq!(view.state, Lifecycle::Wrapping);
    assert_eq!(moves(&view), [Lifecycle::Wrapping]);
    assert_eq!(
        view.branch, "rate-limiting",
        "the Conversation is on the branch the field named",
    );

    // The branch is cut off origin's copy of it and tracking that copy, exactly as
    // a pull request's head is: the wrap-up pushes to where the work already is.
    assert_eq!(
        git(&repo, &["rev-parse", "refs/heads/rate-limiting"]).trim(),
        head,
    );
    assert_eq!(
        git(
            &repo,
            &["rev-parse", "--abbrev-ref", "rate-limiting@{upstream}"]
        )
        .trim(),
        "origin/rate-limiting",
    );

    let worktree = view
        .worktree
        .clone()
        .expect("a taken-up Conversation has a worktree");
    let path = PathBuf::from(&worktree.path);
    assert_eq!(path.parent(), Some(dir.path().join("worktrees").as_path()));
    assert_eq!(
        git(&path, &["symbolic-ref", "--short", "HEAD"]).trim(),
        "rate-limiting",
    );
    assert!(path.join("limits.md").is_file(), "the branch's own work");

    // The base is the head at take-up, so the Timeline draws only what Verkstead
    // adds — and the name beside it is the branch the picker held, which is what
    // the pull request will be opened against.
    assert_eq!(view.base_commit.as_deref(), Some(head.as_str()));

    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();
    let record = store::load_conversation(&pool, id)
        .await
        .unwrap()
        .expect("it is on the record");
    assert_eq!(record.base_ref.as_deref(), Some("release/2.1"));

    // And nothing about a pull request anywhere: none pinned, and nothing held as
    // taken up either.
    assert!(
        !view
            .pinned
            .iter()
            .any(|event| matches!(event, PinnedEvent::PullRequest(_))),
        "a branch is on no pull request, so there is none to pin: {:?}",
        view.pinned,
    );
    assert_eq!(
        store::adopted_pull_request(&pool, id).await.unwrap(),
        None,
        "and nothing was written down as taken up",
    );
}

/// With no base picked, the branch the pull request will be opened against is the
/// Repo's default branch — which is the rule the picker's first entry stands for,
/// read here rather than left as nothing.
#[cfg(unix)]
#[tokio::test]
async fn a_review_over_a_branch_with_no_base_picked_falls_to_the_default_branch() {
    let (elsewhere, dir, app, _repo, upstream, repo_id) = workbench_reviewing().await;
    head_on_origin(&upstream, "rate-limiting");

    let id = ready_to_review(&app, elsewhere.path(), repo_id, "The limiter is built.\n").await;

    assert_eq!(
        name_target(&app, id, "rate-limiting").await,
        TargetRecorded::Recorded,
    );

    assert_eq!(press_take_up(&app, id).await, TakenUp::TakenUp);

    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();
    let record = store::load_conversation(&pool, id)
        .await
        .unwrap()
        .expect("it is on the record");

    assert_eq!(record.state, store::Lifecycle::Wrapping);
    assert_eq!(record.base_ref.as_deref(), Some("main"));
}

/// A Target naming a branch origin has nothing under is refused by name: a branch
/// that is not on origin is nothing to wrap up, there being nowhere for a review
/// to happen.
///
/// Which is also what prose left in the field comes back as. Anything that is not
/// a pull request is read as a branch, so the field is never guessed about — it is
/// asked of origin, and origin says no.
#[cfg(unix)]
#[tokio::test]
async fn a_review_over_a_branch_that_is_not_on_origin_is_refused_by_name() {
    let (elsewhere, _dir, app, repo, upstream, repo_id) = workbench_reviewing().await;
    head_on_origin(&upstream, "rate-limiting");

    let implementation = profile(&app, elsewhere.path(), "opus").await;
    let review = profile(&app, elsewhere.path(), "haiku").await;

    for named in ["rate-limits", "wrap up the limiter work please"] {
        let id = ready_to_review_under(
            &app,
            repo_id,
            "The limiter is built.\n",
            implementation,
            review,
        )
        .await;

        assert_eq!(name_target(&app, id, named).await, TargetRecorded::Recorded);

        assert_eq!(
            press_take_up(&app, id).await,
            TakenUp::NoHeadBranch,
            "origin has nothing under {named:?}",
        );
        nothing_taken_up(&app, id, &repo).await;
    }
}

/// And the local branch refusals are the same road over a bare branch as over a
/// pull request's head: one that is ahead, one that has gone its own way and one
/// checked out somewhere else are each refused by the name they already had.
///
/// Three in one test because they are one question asked of one branch — see the
/// server's `settled` — and what is worth proving is that a branch target reaches
/// it, rather than each answer over again.
#[cfg(unix)]
#[tokio::test]
async fn a_review_over_a_branch_git_will_not_move_is_refused_by_name() {
    let (elsewhere, _dir, app, repo, upstream, repo_id) = workbench_reviewing().await;

    // Three branches on origin, one per way a local copy of one can stand against
    // it, and a local copy of each made off origin's.
    for branch in ["ahead", "gone-its-own-way", "standing"] {
        head_on_origin(&upstream, branch);
    }

    git(&repo, &["fetch", "origin"]);

    for branch in ["ahead", "gone-its-own-way", "standing"] {
        git(&repo, &["branch", branch, &format!("origin/{branch}")]);
    }

    let implementation = profile(&app, elsewhere.path(), "opus").await;
    let review = profile(&app, elsewhere.path(), "haiku").await;

    // Ahead: a commit of its own on top of origin's tip, which is somebody's work
    // and not Verkstead's to move the branch out from under.
    git(&repo, &["checkout", "ahead"]);
    commit(&repo, "unpushed.md");
    git(&repo, &["checkout", "main"]);

    // Gone its own way: a commit of its own here, and one origin has that this
    // copy has not.
    git(&repo, &["checkout", "gone-its-own-way"]);
    commit(&repo, "mine.md");
    git(&repo, &["checkout", "main"]);
    git(&upstream, &["checkout", "gone-its-own-way"]);
    commit(&upstream, "theirs.md");
    git(&upstream, &["checkout", "main"]);

    for (branch, expected) in [
        ("ahead", TakenUp::BranchAhead),
        ("gone-its-own-way", TakenUp::BranchDiverged),
    ] {
        let id = ready_to_review_under(
            &app,
            repo_id,
            "The limiter is built.\n",
            implementation,
            review,
        )
        .await;
        assert_eq!(
            name_target(&app, id, branch).await,
            TargetRecorded::Recorded
        );

        assert_eq!(press_take_up(&app, id).await, expected);
        nothing_taken_up(&app, id, &repo).await;
    }

    // And standing: a worktree of the human's own on it, git holding one checkout
    // per branch. Made after the two above, so that what they are checked against
    // is a repository with one checkout in it.
    let theirs = elsewhere.path().join("their-worktree");
    git(
        &repo,
        &["worktree", "add", &theirs.to_string_lossy(), "standing"],
    );

    let id = ready_to_review_under(
        &app,
        repo_id,
        "The limiter is built.\n",
        implementation,
        review,
    )
    .await;
    assert_eq!(
        name_target(&app, id, "standing").await,
        TargetRecorded::Recorded,
    );

    let refused = press_take_up(&app, id).await;
    let TakenUp::CheckedOutElsewhere { at } = &refused else {
        panic!("expected the place to be named, got {refused:?}");
    };
    assert_eq!(
        PathBuf::from(at).canonicalize().unwrap(),
        theirs.canonicalize().unwrap(),
    );
    assert_eq!(opened(&app, id).await.state, Lifecycle::Draft);
}

/// The Target field takes a pull request URL — which is the one value the
/// Branch field beside it will not have — and takes a bare branch just as
/// readily.
///
/// The two asserted together because the pair is the whole reason the field
/// exists: git refuses the URL over its colon, and a target that went through
/// the rename would be a Review that could never name what it is for.
#[tokio::test]
async fn a_target_takes_a_url_the_branch_field_refuses_and_a_branch_besides() {
    let (_elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = started(&app, repo_id).await;

    let url = "https://github.com/tobico/verkstead/pull/41";

    assert_eq!(
        rename(&app, id, url).await,
        BranchRenamed::NotABranchName,
        "which is why the target is a field of its own",
    );

    assert_eq!(name_target(&app, id, url).await, TargetRecorded::Recorded);
    assert_eq!(opened(&app, id).await.target.as_deref(), Some(url));

    // And a branch, which is the other thing a Review is pointed at: the same
    // field, and nothing here asks git whether either is well formed.
    assert_eq!(
        name_target(&app, id, "rate-limiting").await,
        TargetRecorded::Recorded,
    );
    assert_eq!(
        opened(&app, id).await.target.as_deref(),
        Some("rate-limiting"),
    );

    // And blank is the field cleared, which is the target taken away rather
    // than one called nothing.
    assert_eq!(name_target(&app, id, "  ").await, TargetRecorded::Recorded);
    assert_eq!(opened(&app, id).await.target, None);
}

/// And it is a Draft's to change and nobody else's: the Branch field's own
/// rule, because it is the same kind of fact — read once, when the work starts.
#[tokio::test]
async fn a_target_is_settled_once_the_work_has_started() {
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = ready(&app, elsewhere.path(), repo_id).await;

    assert_eq!(grill(&app, id).await, GrillingStarted::Started);

    assert_eq!(
        name_target(&app, id, "#41").await,
        TargetRecorded::NotDrafting,
    );
    assert_eq!(opened(&app, id).await.target, None);
}

/// A Brief naming a pull request fills an empty Target with it, so the field
/// holds what Start will read rather than standing empty over it.
#[tokio::test]
async fn a_brief_naming_a_pull_request_fills_an_empty_target() {
    let (_elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = started(&app, repo_id).await;

    assert_eq!(
        write_brief(
            &app,
            id,
            "Wrap up https://github.com/tobico/verkstead/pull/41 — the limiter.\n",
        )
        .await,
        BriefSaved::Saved,
    );

    assert_eq!(
        opened(&app, id).await.target.as_deref(),
        Some("https://github.com/tobico/verkstead/pull/41"),
    );

    // A bare number goes in as a bare number: what is written back is the name
    // the human used, theirs to read and to correct.
    let second = started(&app, repo_id).await;
    assert_eq!(
        write_brief(&app, second, "# Rate limiting\n\nPlease wrap #41 up.\n").await,
        BriefSaved::Saved,
    );
    assert_eq!(opened(&app, second).await.target.as_deref(), Some("#41"));

    // And a Brief that names none leaves the field alone.
    let third = started(&app, repo_id).await;
    assert_eq!(
        write_brief(&app, third, "The public API wants a ceiling.\n").await,
        BriefSaved::Saved,
    );
    assert_eq!(opened(&app, third).await.target, None);
}

/// And it never writes over what the human typed: a branch somebody named
/// survives a URL arriving in the Brief afterwards.
#[tokio::test]
async fn a_brief_leaves_a_target_somebody_typed_exactly_as_it_was() {
    let (_elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = started(&app, repo_id).await;

    assert_eq!(
        name_target(&app, id, "rate-limiting").await,
        TargetRecorded::Recorded,
    );

    assert_eq!(
        write_brief(
            &app,
            id,
            "Like https://github.com/tobico/verkstead/pull/41, but on the branch.\n",
        )
        .await,
        BriefSaved::Saved,
    );

    assert_eq!(
        opened(&app, id).await.target.as_deref(),
        Some("rate-limiting"),
        "the human's own typing is what Start reads",
    );
}

/// A Review's Start waits on the Target as well as on the Brief and the two
/// Pairings, and the press takes up what the *field* names — not what the Brief
/// happens to say.
#[cfg(unix)]
#[tokio::test]
async fn a_review_waits_on_its_target_and_takes_up_what_it_names() {
    let (elsewhere, _dir, app, repo, upstream, repo_id) = workbench_reviewing().await;
    head_on_origin(&upstream, "rate-limiting");
    opened_on_github(&repo, 41, "rate-limiting");

    let implementation = profile(&app, elsewhere.path(), "opus").await;
    let review = profile(&app, elsewhere.path(), "haiku").await;

    let id = started(&app, repo_id).await;
    assert_eq!(
        pick_process(&app, id, Process::Review).await,
        ProcessPicked::Picked
    );

    // A Brief naming nothing, so nothing fills the field: everything else is
    // settled and the press is still inert.
    assert_eq!(
        write_brief(&app, id, "The limiter wants a read.\n").await,
        BriefSaved::Saved,
    );
    choose(&app, id, "implementation", implementation).await;
    choose(&app, id, "review", review).await;

    assert!(
        !opened(&app, id).await.ready_to_grill,
        "a brief and both roles, and nothing to take up",
    );
    assert_eq!(
        press_take_up(&app, id).await,
        TakenUp::NoTarget,
        "and a page whose copy of the world went stale is told so",
    );
    nothing_taken_up(&app, id, &repo).await;

    assert_eq!(name_target(&app, id, "#41").await, TargetRecorded::Recorded);
    assert!(opened(&app, id).await.ready_to_grill);

    assert_eq!(press_take_up(&app, id).await, TakenUp::TakenUp);

    let view = opened(&app, id).await;
    assert_eq!(view.state, Lifecycle::Wrapping);
    assert_eq!(view.branch, "rate-limiting");
}

/// What a Review's Start waits on: a Brief, and both Pairings — the two a
/// wrap-up has always waited on, and no grilling among them.
#[cfg(unix)]
#[tokio::test]
async fn a_review_is_ready_on_a_brief_and_both_pairings() {
    let (elsewhere, _dir, app, _repo, _upstream, repo_id) = workbench_reviewing().await;
    let id = started(&app, repo_id).await;

    assert_eq!(
        pick_process(&app, id, Process::Review).await,
        ProcessPicked::Picked
    );
    assert!(!opened(&app, id).await.ready_to_grill, "nothing is settled");

    assert_eq!(
        write_brief(&app, id, "Wrap #41 up.\n").await,
        BriefSaved::Saved
    );
    assert!(
        !opened(&app, id).await.ready_to_grill,
        "a brief and no accounts",
    );

    let implementation = profile(&app, elsewhere.path(), "opus").await;
    choose(&app, id, "implementation", implementation).await;
    assert!(
        !opened(&app, id).await.ready_to_grill,
        "and one of the two roles",
    );

    let review = profile(&app, elsewhere.path(), "haiku").await;
    choose(&app, id, "review", review).await;

    let view = opened(&app, id).await;
    assert!(view.ready_to_grill);
    assert!(
        view.grilling_pairing.is_none(),
        "and no grilling was ever asked for",
    );
}

/// And the press says the same thing: a Review pressed with nothing chosen is
/// refused about its Profiles rather than about its target, the cheap answers
/// coming before anything GitHub is asked.
#[cfg(unix)]
#[tokio::test]
async fn a_review_pressed_with_no_profiles_is_refused_about_them_first() {
    let (_elsewhere, _dir, app, repo, upstream, repo_id) = workbench_reviewing().await;
    head_on_origin(&upstream, "rate-limiting");

    let id = started(&app, repo_id).await;
    assert_eq!(
        pick_process(&app, id, Process::Review).await,
        ProcessPicked::Picked
    );

    assert_eq!(
        press_take_up(&app, id).await,
        TakenUp::NoImplementationProfile,
        "and nothing about GitHub was asked to find that out",
    );
    nothing_taken_up(&app, id, &repo).await;
}

/// A Conversation that is neither a Review nor one started holding a pull
/// request has nothing to wrap up, and the endpoint says so.
#[tokio::test]
async fn a_develop_draft_has_nothing_to_take_up() {
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = ready(&app, elsewhere.path(), repo_id).await;

    assert_eq!(press_take_up(&app, id).await, TakenUp::NotHoldingOne);
}

/// How a pull request's checks are is carried to both copies of its card: the
/// one pinned above the record and the one at the moment it opened.
///
/// Walked through the store rather than elsewhere for, as the narrowing below is:
/// what is under test is the reading, and asking GitHub is `src/checks.rs`'s.
/// The aggregate and nothing else — what every check is called belongs to the
/// details pane.
#[tokio::test]
async fn how_a_pull_requests_checks_are_reaches_both_copies_of_its_card() {
    let (elsewhere, dir, app, _repo, repo_id) = workbench().await;
    let id = grilling(&app, elsewhere.path(), repo_id).await;
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

/// And whether it merges is carried the same way, to the same two copies of the
/// same card — and drawn in whatever state the Conversation is in, this one
/// never leaving Wrapping at all.
///
/// Walked through the store rather than elsewhere for, as the checks above are:
/// what is under test is the reading, and asking GitHub is `src/checks.rs`'s and
/// `src/merges.rs`'s.
///
/// Both words go over, because both are what GitHub said. Which of them is drawn
/// is the viewer's — a card marks the conflict and says nothing about a pull
/// request that merges — and a wire that carried the conflict alone could not
/// tell *it merges* from *nobody asked*.
#[tokio::test]
async fn whether_a_pull_request_merges_reaches_both_copies_of_its_card() {
    let (elsewhere, dir, app, _repo, repo_id) = workbench().await;
    let id = grilling(&app, elsewhere.path(), repo_id).await;
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
        merges(&opened(&app, id).await),
        [None, None],
        "nothing has asked GitHub yet, and a card with nothing to say draws no mark",
    );

    for (asked, drawn) in [
        (store::Merging::Conflicting, Merging::Conflicting),
        // And back again, because a conflict that has been resolved is not a
        // conflict: the reading is written over, so the mark goes.
        (store::Merging::Cleanly, Merging::Cleanly),
    ] {
        store::record_merging(&pool, id, repo_id, asked)
            .await
            .unwrap();

        assert_eq!(
            merges(&opened(&app, id).await),
            [Some(drawn), Some(drawn)],
            "the card follows the poll, in both places it is drawn",
        );
    }

    // And on to Done, where the sweep goes on asking long after the wrap-up's
    // own watcher stopped: a base moving under a branch nobody is working on is
    // exactly what that sweep is for, so the mark is drawn in this state as
    // readily as in the one above.
    for waiting_on in store::WAITED_ON.into_iter().chain([
        store::WaitingOn::Checks(repo_id),
        store::WaitingOn::Comments(repo_id),
        store::WaitingOn::Mergeable(repo_id),
    ]) {
        store::settle_wrap_up(&pool, id, waiting_on).await.unwrap();
    }
    store::finish_wrap_up(&pool, id).await.unwrap();

    store::record_merging(&pool, id, repo_id, store::Merging::Conflicting)
        .await
        .unwrap();

    let view = opened(&app, id).await;

    assert_eq!(view.state, Lifecycle::Done);
    assert_eq!(
        merges(&view),
        [Some(Merging::Conflicting), Some(Merging::Conflicting)],
        "the work being finished with is no reason to stop saying the branch will not merge",
    );
}

/// Whether the pull request merges on each copy of its card, in the same order:
/// the pinned one first, then the one on the record.
fn merges(view: &ConversationView) -> [Option<Merging>; 2] {
    let pinned = view.pinned.iter().find_map(|event| match event {
        PinnedEvent::PullRequest(opened) => Some(opened.merging),
        _ => None,
    });

    let reached = view.timeline.iter().find_map(|event| match event {
        TimelineEvent::PullRequest(opened) => Some(opened.merging),
        _ => None,
    });

    [pinned.flatten(), reached.flatten()]
}

/// A companion's pull request carries its own reading, which is where this
/// parts company with the rollup beside it: a rollup is written down per
/// Conversation, and whether a branch merges is a fact about that branch.
///
/// So a wrap-up ending on two pull requests can have one conflicted and one
/// clean, which is the ordinary shape of it — a base having moved in one
/// repository and not in the other — and each card says what is true of its own.
#[tokio::test]
async fn each_pull_request_carries_whether_its_own_branch_merges() {
    let (elsewhere, dir, app, _repo, repo_id) = workbench().await;
    let id = grilling(&app, elsewhere.path(), repo_id).await;
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    // A second registered repository, standing in for the read-write companion
    // the work also committed in.
    let beside = second_repo(&app, elsewhere.path(), "askance").await;

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

    store::record_another_pull_request(
        &pool,
        id,
        beside,
        &store::PullRequest {
            number: 7,
            title: "Rate limiting".to_owned(),
            url: "https://github.com/tobico/askance/pull/7".to_owned(),
            repo: None,
        },
    )
    .await
    .unwrap();

    store::record_merging(&pool, id, repo_id, store::Merging::Cleanly)
        .await
        .unwrap();
    store::record_merging(&pool, id, beside, store::Merging::Conflicting)
        .await
        .unwrap();

    let view = opened(&app, id).await;

    let each: Vec<(Option<String>, Option<Merging>)> = view
        .pinned
        .iter()
        .filter_map(|event| match event {
            PinnedEvent::PullRequest(opened) => Some((opened.repo.clone(), opened.merging)),
            _ => None,
        })
        .collect();

    assert_eq!(
        each,
        vec![
            (None, Some(Merging::Cleanly)),
            (Some("askance".to_owned()), Some(Merging::Conflicting)),
        ],
        "each card says whether its own branch merges, whatever the other's does",
    );
}

/// The press that gets a finished Conversation's conflict resolved is refused
/// where there is nothing to resolve, and where the Conversation has moved since
/// the pane was drawn.
///
/// Every refusal, because a press that quietly did nothing is what the named
/// outcomes exist to prevent — and both of these are readings that have moved on
/// rather than anything for the human to correct: the button is drawn off the
/// record, and the record is what the press is answered from.
///
/// The press that *works* is not here. It starts the wrap-up's watchers, which
/// go to GitHub — so what it does end to end is `sessions.rs`'s, over a `gh` of
/// its own. What these ask is the reading in front of it.
#[tokio::test]
async fn resolving_a_conflict_is_refused_where_there_is_none_to_resolve() {
    let (elsewhere, dir, app, _repo, repo_id) = workbench().await;
    let id = grilling(&app, elsewhere.path(), repo_id).await;
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    assert_eq!(
        resolving(&app, 404).await,
        Resolved::NoSuchConversation,
        "there is nothing there to press anything on",
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
    store::record_merging(&pool, id, repo_id, store::Merging::Conflicting)
        .await
        .unwrap();

    assert_eq!(
        resolving(&app, id).await,
        Resolved::NotDone,
        "this one is wrapping up, so its own watchers have the conflict in hand",
    );

    // And on to Done, which is where the press is offered.
    for waiting_on in store::WAITED_ON.into_iter().chain([
        store::WaitingOn::Checks(repo_id),
        store::WaitingOn::Comments(repo_id),
        store::WaitingOn::Mergeable(repo_id),
    ]) {
        store::settle_wrap_up(&pool, id, waiting_on).await.unwrap();
    }
    store::finish_wrap_up(&pool, id).await.unwrap();

    // Where somebody has resolved it in the meantime, or the freshening the pane
    // does as it opens found the conflict gone.
    store::record_merging(&pool, id, repo_id, store::Merging::Cleanly)
        .await
        .unwrap();

    assert_eq!(
        resolving(&app, id).await,
        Resolved::NothingConflicts,
        "and a press that found nothing to resolve moves nothing",
    );

    assert_eq!(
        opened(&app, id).await.state,
        Lifecycle::Done,
        "so the Conversation is where the press found it",
    );
}

/// And refused where there is nowhere to resolve it — the Worktree gone and git
/// unable to make it again.
///
/// The refusal this press has that no other reading gives it. A Conversation
/// stays Done for as long as nobody merges its pull request, which is weeks in
/// the case the press is for, and a directory goes in that time: deleted by
/// hand, hollowed out, or left behind by a repository that is no longer there.
/// A press that moved the work back into a wrap-up over one would dispatch a
/// resolution session at a path that is not there, spend both of the pull
/// request's goes on a sandbox nothing could build, and stop the run with a
/// Notice blaming the conflict.
///
/// So the checkout is seen to before the move, and where it cannot be made the
/// press refuses and the Conversation stays exactly where it was. The
/// repository itself is taken away here because that is the one way to make git
/// refuse for certain — what is being asked is what the press does when it
/// refuses, rather than which of the ways a checkout goes was this one.
#[tokio::test]
async fn resolving_a_conflict_is_refused_where_there_is_nowhere_to_resolve_it() {
    let (elsewhere, dir, app, repo, repo_id) = workbench().await;
    let id = grilling(&app, elsewhere.path(), repo_id).await;
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
    store::record_merging(&pool, id, repo_id, store::Merging::Conflicting)
        .await
        .unwrap();

    for waiting_on in store::WAITED_ON.into_iter().chain([
        store::WaitingOn::Checks(repo_id),
        store::WaitingOn::Comments(repo_id),
        store::WaitingOn::Mergeable(repo_id),
    ]) {
        store::settle_wrap_up(&pool, id, waiting_on).await.unwrap();
    }
    store::finish_wrap_up(&pool, id).await.unwrap();

    // The work is finished with, the pull request conflicts, and there is
    // nothing left on this machine to do anything about it in.
    std::fs::remove_dir_all(repo.join(".git")).unwrap();

    assert_eq!(
        resolving(&app, id).await,
        Resolved::WorktreeRefused,
        "git cannot make the checkout again, so the press says so rather than \
         moving the work into a wrap-up with nowhere to work",
    );

    assert_eq!(
        opened(&app, id).await.state,
        Lifecycle::Done,
        "and the Conversation is where the press found it",
    );
}

/// Press **Resolve conflicts**, the way the button on a Done pull request's
/// details pane does. Nothing goes with it: which Conversation it is is the
/// whole of what it says.
async fn resolving(app: &Router, id: i64) -> Resolved {
    post(
        app,
        &format!("/api/ui/conversations/{id}/resolve-conflicts"),
        &serde_json::json!({}),
    )
    .await
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
    let (elsewhere, dir, app, _repo, repo_id) = workbench().await;
    let id = grilling(&app, elsewhere.path(), repo_id).await;
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
        "a wrap-up nobody has read yet is waiting on all four of them",
    );

    for waiting_on in [
        store::WaitingOn::Review,
        store::WaitingOn::Comments(repo_id),
        store::WaitingOn::Mergeable(repo_id),
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
    let (elsewhere, dir, app, _repo, repo_id) = workbench().await;
    let id = grilling(&app, elsewhere.path(), repo_id).await;
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
        store::Narrowing::NotNarrowed,
        "a pull request nothing has said merges is not one waiting on its checks",
    );

    store::settle_wrap_up(&pool, id, store::WaitingOn::Mergeable(repo_id))
        .await
        .unwrap();

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

    store::unsettle_wrap_up(&pool, id, store::WaitingOn::Mergeable(repo_id))
        .await
        .unwrap();

    assert_eq!(
        store::narrowing(&pool, id, false).await.unwrap(),
        store::Narrowing::NotNarrowed,
        "and a pull request that has fallen into conflict is not waiting on GitHub \
         to finish anything: what it needs is a resolution",
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
    let (_elsewhere, dir, app, _repo, repo_id) = workbench().await;
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
    let (elsewhere, dir, app, _repo, repo_id) = workbench().await;
    let id = grilling(&app, elsewhere.path(), repo_id).await;
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

/// *No review* satisfies the same rule a Pairing does, and an empty picker
/// still refuses: the two look alike on the record and only one of them is a
/// choice the human made.
#[tokio::test]
async fn no_review_makes_a_draft_as_ready_to_start_as_a_review_pairing_does() {
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = started(&app, repo_id).await;
    write_brief(&app, id, "# Rate limiting\n").await;

    choose(
        &app,
        id,
        "grilling",
        profile(&app, elsewhere.path(), "fable").await,
    )
    .await;
    choose(
        &app,
        id,
        "implementation",
        profile(&app, elsewhere.path(), "opus").await,
    )
    .await;

    assert_eq!(
        grill(&app, id).await,
        GrillingStarted::NoReviewProfile,
        "the picker nobody has touched refuses the start",
    );
    assert!(!opened(&app, id).await.ready_to_grill);

    assert_eq!(no_review(&app, id).await, ProfileChosen::Chosen);

    let view = opened(&app, id).await;
    assert_eq!(
        view.review_pairing,
        PickedView::Skipped,
        "the row that runs nothing, read back as the choice it was",
    );
    assert!(
        view.ready_to_grill,
        "and a draft that will not be reviewed is a draft that can start",
    );

    assert_eq!(grill(&app, id).await, GrillingStarted::Started);
}

/// And it is fixed when grilling starts, exactly as the Pairings beside it are.
#[tokio::test]
async fn no_review_is_fixed_once_the_grilling_has_started() {
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = ready(&app, elsewhere.path(), repo_id).await;

    assert_eq!(no_review(&app, id).await, ProfileChosen::Chosen);
    assert_eq!(grill(&app, id).await, GrillingStarted::Started);

    assert_eq!(
        no_review(&app, id).await,
        ProfileChosen::NotDrafting,
        "there is no picking left once the work has started",
    );
    assert_eq!(
        opened(&app, id).await.review_pairing,
        PickedView::Skipped,
        "and what it started under is exactly where it was",
    );
}
/// An empty Grilling picker refuses the start, and the only thing that answers
/// it is an account: *No grilling* is retired, so there is no second way to
/// satisfy this role and a Brief that wants no interview is a **Tinker**.
#[tokio::test]
async fn an_empty_grilling_picker_refuses_the_start_until_a_pairing_answers_it() {
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = started(&app, repo_id).await;
    write_brief(&app, id, "# Rate limiting\n").await;

    choose(
        &app,
        id,
        "implementation",
        profile(&app, elsewhere.path(), "opus").await,
    )
    .await;
    choose(
        &app,
        id,
        "review",
        profile(&app, elsewhere.path(), "haiku").await,
    )
    .await;

    assert_eq!(
        grill(&app, id).await,
        GrillingStarted::NoGrillingProfile,
        "the picker nobody has touched refuses the start",
    );
    assert!(!opened(&app, id).await.ready_to_grill);

    choose(
        &app,
        id,
        "grilling",
        profile(&app, elsewhere.path(), "fable").await,
    )
    .await;

    let view = opened(&app, id).await;
    assert!(
        view.grilling_pairing.is_some(),
        "an account is what answers the role",
    );
    assert!(view.ready_to_grill);

    assert_eq!(grill(&app, id).await, GrillingStarted::Started);
    assert_eq!(
        opened(&app, id).await.state,
        Lifecycle::Grilling,
        "and every Conversation this press starts is grilled",
    );
}

/// A Conversation Verkstead has finished with is steered into Investigating, and
/// the record says where the question came from as well as what was asked.
///
/// **Offered from everywhere, which is the target's whole shape.** A wrap-up and
/// a follow-up both turn on a pull request; a question about the work turns on
/// nothing, so there is nowhere the work can have got to that makes asking one
/// wrong — and this Conversation has none, being a Develop one steered to Done off
/// its grilling.
///
/// **The question is the Event**, rendered like every other document the human
/// writes, exactly as a follow-up's brief is: reading the Timeline back is
/// reading what was asked.
///
/// **And the state it was steered out of is on the record beside that Event**,
/// which is the fact nothing else holds: the state column says Investigating from
/// the moment the move lands, so where the work came from would be gone. Read
/// back out of the database rather than off the page — the ending that wants it is
/// the reader, and it may be hours and a restart away.
#[tokio::test]
async fn steering_a_finished_conversation_into_investigating_records_where_it_came_from() {
    let (elsewhere, dir, app, _repo, repo_id) = workbench().await;
    let id = grilling(&app, elsewhere.path(), repo_id).await;

    assert_eq!(steer(&app, id).await, SteerOpened::Opened);
    assert_eq!(
        steer_into(&app, id, "Done", false).await,
        ConversationSteered::Steered,
    );

    assert_eq!(steer(&app, id).await, SteerOpened::Opened);
    assert_eq!(
        steer_investigating(&app, id, Some("Where does the `429` count come from?\n")).await,
        ConversationSteered::Steered,
        "on no pull request at all, which is what this target is offered despite",
    );

    let view = opened(&app, id).await;

    assert_eq!(view.state, Lifecycle::Investigating);
    assert_eq!(
        steered(&view),
        [
            ("moved", Lifecycle::Grilling),
            ("steer", Lifecycle::Done),
            ("moved", Lifecycle::Done),
            ("steer", Lifecycle::Investigating),
            ("moved", Lifecycle::Investigating),
        ],
        "the human's own line, and the plain move under it",
    );

    let asked = view
        .timeline
        .iter()
        .rev()
        .find_map(|event| match event {
            TimelineEvent::Steer(steer) => steer.html.clone(),
            _ => None,
        })
        .expect("the steer carries what was written on it");

    assert!(
        asked.contains("Where does the <code>429</code> count come from?"),
        "rendered like every other document the human writes: {asked:?}",
    );

    assert!(
        view.worktree.is_some(),
        "and it has somewhere to write its probes: a steer into a state work goes \
         on in checks one out where none stands",
    );
    assert_eq!(
        view.blocked_on, None,
        "and the stop the click wrote is gone"
    );

    // The half of the record no page draws, read the way the ending will read it:
    // out of the database, after the server that wrote it has gone.
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    // The store's own word for a state rather than the viewer's: what is being
    // read here is the record, one side of the seam the view types are the other
    // side of.
    let sources: Vec<(store::Lifecycle, Option<store::Lifecycle>)> = store::timeline(&pool, id)
        .await
        .unwrap()
        .into_iter()
        .filter_map(|event| match event.event {
            store::Event::Steer(target, _, recorded) => {
                Some((target, recorded.and_then(|record| record.source)))
            }
            _ => None,
        })
        .collect();

    pool.close().await;

    assert_eq!(
        sources,
        [
            (store::Lifecycle::Done, Some(store::Lifecycle::Grilling)),
            (
                store::Lifecycle::Investigating,
                Some(store::Lifecycle::Done)
            ),
        ],
        "each press says where it went and where it found the work",
    );
}

/// An investigation is whatever the human asked about, so a submit with nothing
/// written is refused by name — in its own words rather than the follow-up's.
///
/// The second of the two written payloads with no quiet meaning: an empty
/// instruction carries the branch on and an empty brief grills the one already
/// written, and a question nobody asked is a session with nothing to find out.
/// Its own refusal because the follow-up's names a pull request, and an
/// investigation is steerable from work that is on none — which is this
/// Conversation.
#[tokio::test]
async fn steering_into_investigating_with_nothing_to_find_out_is_refused_by_name() {
    let (elsewhere, _dir, app, _repo, repo_id) = workbench().await;
    let id = grilling(&app, elsewhere.path(), repo_id).await;

    assert_eq!(steer(&app, id).await, SteerOpened::Opened);
    assert_eq!(
        steer_investigating(&app, id, None).await,
        ConversationSteered::NoInvestigationBrief,
        "there is nothing for the session to find out",
    );
    assert_eq!(
        steer_investigating(&app, id, Some("   \n")).await,
        ConversationSteered::NoInvestigationBrief,
        "a textarea somebody tabbed through included",
    );

    let view = opened(&app, id).await;

    assert_eq!(view.state, Lifecycle::Grilling, "so nothing moved");
    assert_eq!(
        steered(&view),
        [("moved", Lifecycle::Grilling)],
        "and nothing on the record says it was steered",
    );
}
