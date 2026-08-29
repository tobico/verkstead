//! Agent Profiles over the viewer's namespace: saving one, what is refused, and
//! the two a Conversation chooses before anything will grill it.
//!
//! Asked of the *server*, through the endpoints, because that is where the
//! decisions are: whether the pair is really there, and whether it is inside the
//! Watched Paths. A form that checked either would be a courtesy — this endpoint
//! is reachable without one, and the boundary is the server's.
//!
//! Nothing here mounts anything. A Profile is a record of an account a session
//! will later be run under, and the stage that runs one is the next one.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use serde::de::DeserializeOwned;
use tower::ServiceExt;
use verkstead_render::{
    AgentType, BriefSaved, Broken, ConversationView, PickedView, ProfileChosen, ProfileDeleted,
    ProfileEntry, ProfileSaved, Registered, Started,
};
use verkstead_server::{WatchedPaths, open_database, router_watching, store};

/// A watched directory, the app over it, and the directory holding the database
/// alive.
async fn workbench() -> (tempfile::TempDir, tempfile::TempDir, Router) {
    let watched = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();
    let paths = WatchedPaths::resolve(&[watched.path().to_owned()]).unwrap();

    // Beside the database, as it falls out for the real server. Nothing in this
    // file grills, so nothing is ever put in it.
    let data_dir = dir.path().to_owned();

    (watched, dir, router_watching(pool, paths, data_dir))
}

/// A claude dir and config file pair at `root`, as `work-sandbox` would find
/// one: a directory to mount over `~/.claude`, and a file over `~/.claude.json`.
fn pair(root: &Path, account: &str) -> (PathBuf, PathBuf) {
    let home = root.join(account);
    let claude_dir = home.join(".claude");
    let config_file = home.join(".claude.json");

    std::fs::create_dir_all(&claude_dir).unwrap();
    std::fs::write(&config_file, "{}\n").unwrap();

    (claude_dir, config_file)
}

/// The body a save takes, from a pair, a name and the models the account can
/// run.
fn edit(name: &str, claude_dir: &Path, config_file: &Path, models: &[&str]) -> serde_json::Value {
    serde_json::json!({
        "name": name,
        "claude_dir": claude_dir,
        "config_file": config_file,
        "models": models,
    })
}

async fn save(app: &Router, body: &serde_json::Value) -> ProfileSaved {
    post(app, "/api/ui/profiles", body).await
}

/// Two models apiece, so that a Pairing below can name one that is not the
/// first of the list — which is the whole of what a Pairing adds to a Profile.
const MODELS: [&str; 2] = ["claude-opus-5", "claude-fable-5"];

/// The one the Pairings here are made with: the second, for the reason above.
const MODEL: &str = MODELS[1];

/// Save one that ought to work, and hand back the row it became.
async fn saved(app: &Router, watched: &Path, name: &str) -> ProfileEntry {
    let (claude_dir, config_file) = pair(watched, name);

    assert_eq!(
        save(app, &edit(name, &claude_dir, &config_file, &MODELS)).await,
        ProfileSaved::Saved
    );

    listed(app)
        .await
        .into_iter()
        .find(|profile| profile.name == name)
        .expect("the Profile just saved should be on the list")
}

async fn listed(app: &Router) -> Vec<ProfileEntry> {
    get(app, "/api/ui/profiles").await
}

async fn remove(app: &Router, id: i64) -> ProfileDeleted {
    post(
        app,
        &format!("/api/ui/profiles/{id}/delete"),
        &serde_json::json!({}),
    )
    .await
}

/// A git repository at `path`, with one commit so it has a default branch.
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
    let ran = Command::new("git")
        .args(args)
        .current_dir(dir)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .stdout(Stdio::null())
        .status()
        .expect("git should be on the PATH for these tests");

    assert!(ran.success(), "git {args:?} failed in {}", dir.display());
}

/// A Conversation to choose Profiles on, in a repository inside the watched
/// directory.
async fn conversation(app: &Router, watched: &Path) -> i64 {
    let repo = repository(watched.join("verkstead"));

    let registered: Registered =
        post(app, "/api/ui/repos", &serde_json::json!({ "path": repo })).await;
    assert_eq!(registered, Registered::Added);

    let repos: Vec<verkstead_render::RepoEntry> = get(app, "/api/ui/repos").await;

    let started: Started = post(
        app,
        "/api/ui/conversations",
        &serde_json::json!({ "repo_id": repos[0].id }),
    )
    .await;

    let Started::Started { id } = started else {
        panic!("expected the Conversation to start, got {started:?}");
    };

    // A Brief, because readiness to grill turns on one as well as on the
    // Profiles — and what these tests are about is the Profiles. Written here so
    // that every readiness assertion below is answering about them alone.
    let saved: BriefSaved = post(
        app,
        &format!("/api/ui/conversations/{id}/brief"),
        &serde_json::json!({ "markdown": "# Rate limiting\n" }),
    )
    .await;
    assert_eq!(saved, BriefSaved::Saved);

    id
}

async fn opened(app: &Router, id: i64) -> ConversationView {
    get(app, &format!("/api/ui/conversations/{id}")).await
}

async fn choose_grilling(app: &Router, id: i64, profile_id: i64, model: &str) -> ProfileChosen {
    post(
        app,
        &format!("/api/ui/conversations/{id}/grilling-pairing"),
        &serde_json::json!({
            "pairing": { "profile_id": profile_id, "model": model },
        }),
    )
    .await
}

/// And that picker's other row: no grilling at all.
async fn no_grilling(app: &Router, id: i64) -> ProfileChosen {
    post(
        app,
        &format!("/api/ui/conversations/{id}/grilling-pairing"),
        &serde_json::json!({ "pairing": null }),
    )
    .await
}

async fn choose_implementation(
    app: &Router,
    id: i64,
    profile_id: i64,
    model: &str,
) -> ProfileChosen {
    post(
        app,
        &format!("/api/ui/conversations/{id}/implementation-pairing"),
        &serde_json::json!({ "profile_id": profile_id, "model": model }),
    )
    .await
}

async fn choose_review(app: &Router, id: i64, profile_id: i64, model: &str) -> ProfileChosen {
    post(
        app,
        &format!("/api/ui/conversations/{id}/review-pairing"),
        &serde_json::json!({
            "pairing": { "profile_id": profile_id, "model": model },
        }),
    )
    .await
}

/// And the picker's other row: no review at all.
async fn no_review(app: &Router, id: i64) -> ProfileChosen {
    post(
        app,
        &format!("/api/ui/conversations/{id}/review-pairing"),
        &serde_json::json!({ "pairing": null }),
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
async fn a_saved_profile_appears_on_the_list_with_everything_it_was_given() {
    let (watched, _dir, app) = workbench().await;

    let profile = saved(&app, watched.path(), "work").await;

    assert_eq!(profile.name, "work");
    assert_eq!(profile.models, MODELS);
    assert_eq!(profile.agent_type, AgentType::Claude);

    // The resolved paths, which are what will be bind-mounted.
    assert_eq!(
        profile.claude_dir,
        watched
            .path()
            .canonicalize()
            .unwrap()
            .join("work/.claude")
            .to_str()
            .unwrap()
    );
    assert_eq!(
        profile.config_file,
        watched
            .path()
            .canonicalize()
            .unwrap()
            .join("work/.claude.json")
            .to_str()
            .unwrap()
    );

    // Its pair is where it was left, which is the ordinary case.
    assert_eq!(profile.broken, None);
}

#[tokio::test]
async fn a_profile_is_rewritten_whole_and_removed_when_nobody_is_running_under_it() {
    let (watched, _dir, app) = workbench().await;
    let profile = saved(&app, watched.path(), "work").await;
    let (claude_dir, config_file) = pair(watched.path(), "anthropic");

    let rewritten: ProfileSaved = post(
        &app,
        &format!("/api/ui/profiles/{}", profile.id),
        &edit("anthropic", &claude_dir, &config_file, &["claude-fable-5"]),
    )
    .await;
    assert_eq!(rewritten, ProfileSaved::Saved);

    let rows = listed(&app).await;
    assert_eq!(rows.len(), 1, "rewriting one does not add another");
    assert_eq!(rows[0].name, "anthropic");
    assert_eq!(rows[0].models, &["claude-fable-5"]);

    assert_eq!(remove(&app, profile.id).await, ProfileDeleted::Removed);
    assert!(listed(&app).await.is_empty());
}

#[tokio::test]
async fn profiles_come_back_by_name() {
    let (watched, _dir, app) = workbench().await;
    saved(&app, watched.path(), "work").await;
    saved(&app, watched.path(), "anthropic").await;
    saved(&app, watched.path(), "personal").await;

    let names: Vec<String> = listed(&app)
        .await
        .into_iter()
        .map(|profile| profile.name)
        .collect();

    assert_eq!(names, ["anthropic", "personal", "work"]);
}

/// Both halves have to be there, and each is refused by its own name: pointing
/// the config field at a directory is an easy mistake, and "that path is wrong"
/// would not say which one.
#[tokio::test]
async fn a_pair_that_is_not_there_is_refused_by_the_half_that_is_missing() {
    let (watched, _dir, app) = workbench().await;
    let (claude_dir, config_file) = pair(watched.path(), "work");
    let nowhere = watched.path().join("never-made");

    assert_eq!(
        save(
            &app,
            &edit("work", &nowhere, &config_file, &["claude-opus-5"])
        )
        .await,
        ProfileSaved::DirMissing
    );
    assert_eq!(
        save(
            &app,
            &edit("work", &claude_dir, &nowhere, &["claude-opus-5"])
        )
        .await,
        ProfileSaved::ConfigMissing
    );

    assert!(listed(&app).await.is_empty(), "nothing refused was saved");
}

/// The pair is a directory and a file, in that order. Swapping them is the
/// mistake this catches.
#[tokio::test]
async fn a_file_where_the_directory_goes_and_the_reverse_are_both_refused() {
    let (watched, _dir, app) = workbench().await;
    let (claude_dir, config_file) = pair(watched.path(), "work");

    assert_eq!(
        save(
            &app,
            &edit("work", &config_file, &config_file, &["claude-opus-5"])
        )
        .await,
        ProfileSaved::NotADirectory
    );
    assert_eq!(
        save(
            &app,
            &edit("work", &claude_dir, &claude_dir, &["claude-opus-5"])
        )
        .await,
        ProfileSaved::NotAFile
    );
}

/// The boundary is the server's, and it is the same boundary a Repo's path is
/// judged by: one rule about what Verkstead may touch, not one rule per feature.
#[tokio::test]
async fn a_pair_outside_the_watched_paths_is_refused_by_the_server() {
    let (watched, _dir, app) = workbench().await;
    let (inside_dir, inside_config) = pair(watched.path(), "work");

    let elsewhere = tempfile::tempdir().unwrap();
    let (outside_dir, outside_config) = pair(elsewhere.path(), "work");

    assert_eq!(
        save(
            &app,
            &edit("work", &outside_dir, &inside_config, &["claude-opus-5"])
        )
        .await,
        ProfileSaved::DirOutsideWatchedPaths
    );
    assert_eq!(
        save(
            &app,
            &edit("work", &inside_dir, &outside_config, &["claude-opus-5"])
        )
        .await,
        ProfileSaved::ConfigOutsideWatchedPaths
    );

    assert!(listed(&app).await.is_empty());
}

/// A path that merely *reads* as inside a Watched Path is not inside one: the
/// symlink is followed before the boundary is consulted, exactly as it is for a
/// Repo.
#[tokio::test]
async fn a_symlink_out_of_the_watched_paths_does_not_get_a_pair_in() {
    let (watched, _dir, app) = workbench().await;
    let (_, config_file) = pair(watched.path(), "work");

    let elsewhere = tempfile::tempdir().unwrap();
    let (outside_dir, _) = pair(elsewhere.path(), "escape");

    let looks_inside = watched.path().join("looks-inside");
    std::os::unix::fs::symlink(&outside_dir, &looks_inside).unwrap();

    assert_eq!(
        save(
            &app,
            &edit("work", &looks_inside, &config_file, &["claude-opus-5"])
        )
        .await,
        ProfileSaved::DirOutsideWatchedPaths
    );
}

#[tokio::test]
async fn a_relative_path_is_refused_without_being_resolved() {
    let (watched, _dir, app) = workbench().await;
    let (claude_dir, config_file) = pair(watched.path(), "work");

    assert_eq!(
        save(
            &app,
            &edit(
                "work",
                Path::new(".claude"),
                &config_file,
                &["claude-opus-5"]
            )
        )
        .await,
        ProfileSaved::DirNotAbsolute
    );
    assert_eq!(
        save(
            &app,
            &edit(
                "work",
                &claude_dir,
                Path::new(".claude.json"),
                &["claude-opus-5"],
            )
        )
        .await,
        ProfileSaved::ConfigNotAbsolute
    );
}

/// Every model an account can launch, saved and read back in the order it was
/// written. The list is the Profile's own, and no entry of it is preferred.
#[tokio::test]
async fn a_profile_lists_every_model_its_account_can_run() {
    let (watched, _dir, app) = workbench().await;
    let (claude_dir, config_file) = pair(watched.path(), "work");

    assert_eq!(
        save(
            &app,
            &edit(
                "work",
                &claude_dir,
                &config_file,
                // Blank lines and stray whitespace are the form's leavings, and
                // the server drops them rather than saving a model called "".
                &["claude-opus-5", "  ", " claude-fable-5 "]
            )
        )
        .await,
        ProfileSaved::Saved
    );

    let rows = listed(&app).await;
    assert_eq!(rows[0].models, ["claude-opus-5", "claude-fable-5"]);
}

/// A Profile is picked out of a list by its name and run on one of its models.
/// Neither is a field to leave empty.
#[tokio::test]
async fn a_profile_with_no_name_or_no_models_is_refused() {
    let (watched, _dir, app) = workbench().await;
    let (claude_dir, config_file) = pair(watched.path(), "work");

    assert_eq!(
        save(
            &app,
            &edit("   ", &claude_dir, &config_file, &["claude-opus-5"])
        )
        .await,
        ProfileSaved::Nameless
    );
    assert_eq!(
        save(&app, &edit("work", &claude_dir, &config_file, &["  "])).await,
        ProfileSaved::Modelless
    );
    assert_eq!(
        save(&app, &edit("work", &claude_dir, &config_file, &[])).await,
        ProfileSaved::Modelless
    );
    assert!(listed(&app).await.is_empty());
}

#[tokio::test]
async fn a_name_another_profile_already_has_is_refused() {
    let (watched, _dir, app) = workbench().await;
    saved(&app, watched.path(), "work").await;

    let (claude_dir, config_file) = pair(watched.path(), "second");
    assert_eq!(
        save(
            &app,
            &edit("work", &claude_dir, &config_file, &["claude-opus-5"])
        )
        .await,
        ProfileSaved::NameTaken
    );
    assert_eq!(listed(&app).await.len(), 1);
}

/// The pair was there when it was saved; a directory can be moved afterwards.
/// The list says so rather than leaving it to be found out when a session will
/// not start.
#[tokio::test]
async fn a_profile_whose_pair_has_disappeared_reads_as_broken() {
    let (watched, _dir, app) = workbench().await;
    let profile = saved(&app, watched.path(), "work").await;
    assert_eq!(profile.broken, None);

    std::fs::remove_dir_all(watched.path().join("work/.claude")).unwrap();
    assert_eq!(listed(&app).await[0].broken, Some(Broken::DirMissing));

    // Put the directory back and take the file instead: the other half breaks it
    // just the same, and says which half it was.
    std::fs::create_dir_all(watched.path().join("work/.claude")).unwrap();
    std::fs::remove_file(watched.path().join("work/.claude.json")).unwrap();
    assert_eq!(listed(&app).await[0].broken, Some(Broken::ConfigMissing));
}

/// Broken is asked of the boundary and not only of the filesystem: a directory
/// replaced by a symlink out of the Watched Paths still exists, and mounting it
/// would be reaching around a boundary with a path that was admitted once.
#[tokio::test]
async fn a_pair_that_now_points_out_of_the_watched_paths_reads_as_broken() {
    let (watched, _dir, app) = workbench().await;
    saved(&app, watched.path(), "work").await;

    let elsewhere = tempfile::tempdir().unwrap();
    let (outside_dir, _) = pair(elsewhere.path(), "escape");

    std::fs::remove_dir_all(watched.path().join("work/.claude")).unwrap();
    std::os::unix::fs::symlink(&outside_dir, watched.path().join("work/.claude")).unwrap();

    assert_eq!(
        listed(&app).await[0].broken,
        Some(Broken::OutsideWatchedPaths)
    );
}

/// They are separate choices because they are genuinely separate accounts and
/// models — grill on fable, implement on opus.
#[tokio::test]
async fn a_conversation_chooses_its_two_pairings_independently() {
    let (watched, _dir, app) = workbench().await;
    let id = conversation(&app, watched.path()).await;
    let fable = saved(&app, watched.path(), "fable").await;
    let opus = saved(&app, watched.path(), "opus").await;

    assert_eq!(
        choose_grilling(&app, id, fable.id, MODEL).await,
        ProfileChosen::Chosen
    );

    let half = opened(&app, id).await;
    assert_eq!(
        half.grilling_pairing
            .pairing()
            .map(|p| p.profile.name.as_str()),
        Some("fable")
    );
    assert_eq!(half.implementation_pairing, None);

    assert_eq!(
        choose_implementation(&app, id, opus.id, MODEL).await,
        ProfileChosen::Chosen
    );

    let both = opened(&app, id).await;
    assert_eq!(
        both.grilling_pairing
            .pairing()
            .map(|p| p.profile.name.clone()),
        Some("fable".to_owned())
    );
    assert_eq!(
        both.implementation_pairing.map(|p| p.profile.name),
        Some("opus".to_owned())
    );
}

/// The model is half of the choice and it is the half that is picked: the pane
/// says back the model that was paired rather than whatever the Profile lists
/// first.
#[tokio::test]
async fn a_pairing_says_back_the_model_it_was_chosen_with() {
    let (watched, _dir, app) = workbench().await;
    let id = conversation(&app, watched.path()).await;
    let work = saved(&app, watched.path(), "work").await;

    choose_grilling(&app, id, work.id, MODELS[1]).await;
    choose_implementation(&app, id, work.id, MODELS[0]).await;

    let view = opened(&app, id).await;
    assert_eq!(
        view.grilling_pairing
            .pairing()
            .and_then(|p| p.model.clone()),
        Some(MODELS[1].to_owned())
    );
    assert_eq!(
        view.implementation_pairing.and_then(|p| p.model),
        Some(MODELS[0].to_owned())
    );
}

/// A model that Profile does not list is refused, the way a Profile that is not
/// there is: a list edited between the page being drawn and the pick would
/// otherwise launch a session on something the account cannot run.
#[tokio::test]
async fn a_model_the_profile_does_not_list_cannot_be_paired_with_it() {
    let (watched, _dir, app) = workbench().await;
    let id = conversation(&app, watched.path()).await;
    let work = saved(&app, watched.path(), "work").await;

    assert_eq!(
        choose_grilling(&app, id, work.id, "claude-haiku-4-5").await,
        ProfileChosen::NoSuchModel
    );
    assert_eq!(
        choose_implementation(&app, id, work.id, "claude-haiku-4-5").await,
        ProfileChosen::NoSuchModel
    );

    let view = opened(&app, id).await;
    assert_eq!(view.grilling_pairing, PickedView::Nothing);
    assert_eq!(view.implementation_pairing, None);
}

/// The whole point of the record: a Conversation missing any one Pairing is not
/// something the next stage will grill.
#[tokio::test]
async fn a_conversation_is_not_ready_to_grill_until_every_pairing_is_chosen() {
    let (watched, _dir, app) = workbench().await;
    let id = conversation(&app, watched.path()).await;
    let fable = saved(&app, watched.path(), "fable").await;
    let opus = saved(&app, watched.path(), "opus").await;
    let haiku = saved(&app, watched.path(), "haiku").await;

    assert!(
        !opened(&app, id).await.ready_to_grill,
        "a fresh Conversation has chosen none of them"
    );

    choose_grilling(&app, id, fable.id, MODEL).await;
    assert!(
        !opened(&app, id).await.ready_to_grill,
        "one of the three is not all of them"
    );

    choose_implementation(&app, id, opus.id, MODEL).await;
    assert!(
        !opened(&app, id).await.ready_to_grill,
        "and neither is two of them: the review is a pick of its own"
    );

    choose_review(&app, id, haiku.id, MODEL).await;
    assert!(opened(&app, id).await.ready_to_grill);
}

/// A Profile chosen before pairings existed has no model beside it, so it is
/// not a Pairing: while the Conversation is drafting that is a choice to make
/// again, and the pane reads it as one.
#[tokio::test]
async fn a_drafting_conversation_with_an_unpaired_profile_is_not_ready_to_grill() {
    let (watched, dir, app) = workbench().await;
    let id = conversation(&app, watched.path()).await;
    let work = saved(&app, watched.path(), "work").await;

    choose_grilling(&app, id, work.id, MODEL).await;
    choose_implementation(&app, id, work.id, MODEL).await;
    choose_review(&app, id, work.id, MODEL).await;
    assert!(opened(&app, id).await.ready_to_grill);

    // The shape an old choice left behind: the Profile, and no model beside it.
    // Written through the store because there is no endpoint that will make one
    // any more — a Pairing is picked whole from here on.
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();
    store::set_grilling_pairing(&pool, id, work.id, None)
        .await
        .unwrap();

    let view = opened(&app, id).await;
    assert_eq!(
        view.grilling_pairing
            .pairing()
            .and_then(|p| p.model.clone()),
        None
    );
    assert!(!view.ready_to_grill);
}

/// A Profile whose pair has gone is not one to launch a session under, so
/// choosing it is not enough to be ready.
#[tokio::test]
async fn a_conversation_holding_a_broken_profile_is_not_ready_to_grill() {
    let (watched, _dir, app) = workbench().await;
    let id = conversation(&app, watched.path()).await;
    let fable = saved(&app, watched.path(), "fable").await;
    let opus = saved(&app, watched.path(), "opus").await;
    let haiku = saved(&app, watched.path(), "haiku").await;

    choose_grilling(&app, id, fable.id, MODEL).await;
    choose_implementation(&app, id, opus.id, MODEL).await;
    choose_review(&app, id, haiku.id, MODEL).await;
    assert!(opened(&app, id).await.ready_to_grill);

    std::fs::remove_file(watched.path().join("opus/.claude.json")).unwrap();

    let view = opened(&app, id).await;
    assert_eq!(
        view.implementation_pairing.map(|p| p.profile.broken),
        Some(Some(Broken::ConfigMissing)),
        "the pane says which Profile it is, and what is wrong with it"
    );
    assert!(!view.ready_to_grill);
}

/// The same Profile may fill every role: they are roles a Profile is used in,
/// not kinds of Profile.
#[tokio::test]
async fn one_profile_can_be_every_one_of_a_conversations_choices() {
    let (watched, _dir, app) = workbench().await;
    let id = conversation(&app, watched.path()).await;
    let only = saved(&app, watched.path(), "work").await;

    choose_grilling(&app, id, only.id, MODEL).await;
    choose_implementation(&app, id, only.id, MODEL).await;
    choose_review(&app, id, only.id, MODEL).await;

    let view = opened(&app, id).await;
    assert_eq!(
        view.grilling_pairing.pairing().map(|p| p.profile.id),
        Some(only.id)
    );
    assert_eq!(
        view.implementation_pairing.map(|p| p.profile.id),
        Some(only.id)
    );
    assert_eq!(
        view.review_pairing.pairing().map(|p| p.profile.id),
        Some(only.id)
    );
    assert!(view.ready_to_grill);
}

/// Refused rather than taken away from the Conversation: one pointing at a
/// Profile that is not there is a session that fails to start with nobody
/// watching.
#[tokio::test]
async fn a_profile_a_conversation_has_chosen_cannot_be_removed() {
    let (watched, _dir, app) = workbench().await;
    let id = conversation(&app, watched.path()).await;
    let profile = saved(&app, watched.path(), "work").await;

    choose_grilling(&app, id, profile.id, MODEL).await;

    assert_eq!(remove(&app, profile.id).await, ProfileDeleted::InUse);
    assert_eq!(listed(&app).await.len(), 1);
    assert!(opened(&app, id).await.grilling_pairing.pairing().is_some());
}

#[tokio::test]
async fn a_profile_that_is_not_there_says_so_however_it_is_asked_about() {
    let (watched, _dir, app) = workbench().await;
    let id = conversation(&app, watched.path()).await;
    let (claude_dir, config_file) = pair(watched.path(), "work");

    let rewritten: ProfileSaved = post(
        &app,
        "/api/ui/profiles/404",
        &edit("work", &claude_dir, &config_file, &["claude-opus-5"]),
    )
    .await;
    assert_eq!(rewritten, ProfileSaved::NoSuchProfile);

    assert_eq!(remove(&app, 404).await, ProfileDeleted::NoSuchProfile);

    // Chosen between the list a page read and the choice it made from it.
    assert_eq!(
        choose_grilling(&app, id, 404, MODEL).await,
        ProfileChosen::NoSuchProfile
    );
    assert_eq!(
        choose_implementation(&app, id, 404, MODEL).await,
        ProfileChosen::NoSuchProfile
    );
}

#[tokio::test]
async fn choosing_on_a_conversation_that_is_not_there_says_so() {
    let (watched, _dir, app) = workbench().await;
    let profile = saved(&app, watched.path(), "work").await;

    assert_eq!(
        choose_grilling(&app, 404, profile.id, MODEL).await,
        ProfileChosen::NoSuchConversation
    );
    assert_eq!(
        choose_implementation(&app, 404, profile.id, MODEL).await,
        ProfileChosen::NoSuchConversation
    );
}

/// An id out of a URL the human may have typed, which is not always a number.
#[tokio::test]
async fn an_id_that_is_not_a_number_names_no_profile() {
    let (watched, _dir, app) = workbench().await;
    let (claude_dir, config_file) = pair(watched.path(), "work");

    let rewritten: ProfileSaved = post(
        &app,
        "/api/ui/profiles/nonsense",
        &edit("work", &claude_dir, &config_file, &["claude-opus-5"]),
    )
    .await;
    assert_eq!(rewritten, ProfileSaved::NoSuchProfile);

    let removed: ProfileDeleted = post(
        &app,
        "/api/ui/profiles/nonsense/delete",
        &serde_json::json!({}),
    )
    .await;
    assert_eq!(removed, ProfileDeleted::NoSuchProfile);
}

#[tokio::test]
async fn nothing_saved_means_an_empty_list() {
    let (_watched, _dir, app) = workbench().await;

    assert!(listed(&app).await.is_empty());
}

/// What a Repo was last grilled with, filled into the pickers of the next
/// Conversation started on it.
///
/// The grilling itself is recorded through the store rather than pressed: what
/// these are about is the memory, and nothing in this file launches a session.
async fn grill(dir: &Path, id: i64) {
    let pool = open_database(&dir.join("verkstead.db")).await.unwrap();

    store::start_grilling(&pool, id, "deadbeef", &dir.join("worktree"), &[])
        .await
        .unwrap();
}

/// And the same moment on a Conversation that will not be grilled: the press
/// that takes its Brief straight to the work, which fixes the roles and writes
/// the memory exactly as the one above does.
async fn build(dir: &Path, id: i64) {
    let pool = open_database(&dir.join("verkstead.db")).await.unwrap();

    store::start_building(&pool, id, "deadbeef", &dir.join("worktree"), &[])
        .await
        .unwrap();
}

/// A second Conversation on the same Repo as the first, started the way the
/// human starts one.
async fn another(app: &Router) -> i64 {
    let repos: Vec<verkstead_render::RepoEntry> = get(app, "/api/ui/repos").await;

    let started: Started = post(
        app,
        "/api/ui/conversations",
        &serde_json::json!({ "repo_id": repos[0].id }),
    )
    .await;

    let Started::Started { id } = started else {
        panic!("expected the Conversation to start, got {started:?}");
    };

    id
}

/// The whole of it: grill once, and the next Conversation on that Repo arrives
/// with every picker already filled.
#[tokio::test]
async fn a_new_conversation_arrives_with_what_its_repo_was_last_grilled_with() {
    let (watched, dir, app) = workbench().await;
    let id = conversation(&app, watched.path()).await;
    let fable = saved(&app, watched.path(), "fable").await;
    let opus = saved(&app, watched.path(), "opus").await;
    let haiku = saved(&app, watched.path(), "haiku").await;

    choose_grilling(&app, id, fable.id, MODEL).await;
    choose_implementation(&app, id, opus.id, MODEL).await;
    choose_review(&app, id, haiku.id, MODEL).await;
    grill(dir.path(), id).await;

    let next = another(&app).await;
    let view = opened(&app, next).await;

    let grilling = view
        .grilling_pairing
        .pairing()
        .expect("the grilling picker is filled");
    assert_eq!(grilling.profile.id, fable.id);
    assert_eq!(grilling.model.as_deref(), Some(MODEL));

    let implementation = view
        .implementation_pairing
        .expect("and so is the implementation one");
    assert_eq!(implementation.profile.id, opus.id);
    assert_eq!(implementation.model.as_deref(), Some(MODEL));

    let review = view
        .review_pairing
        .pairing()
        .expect("and so is the review one");
    assert_eq!(review.profile.id, haiku.id);
    assert_eq!(review.model.as_deref(), Some(MODEL));

    // Real choices rather than a picture of three: with a Brief written, nothing
    // else stands between this Conversation and the grilling button.
    let saved: BriefSaved = post(
        &app,
        &format!("/api/ui/conversations/{next}/brief"),
        &serde_json::json!({ "markdown": "# Rate limiting\n" }),
    )
    .await;
    assert_eq!(saved, BriefSaved::Saved);
    assert!(opened(&app, next).await.ready_to_grill);
}

/// A default rather than a lock. The prefill is the human's to change, and what
/// they changed it to is what the Repo remembers next.
#[tokio::test]
async fn changing_the_prefill_before_grilling_is_what_gets_remembered() {
    let (watched, dir, app) = workbench().await;
    let first = conversation(&app, watched.path()).await;
    let fable = saved(&app, watched.path(), "fable").await;
    let opus = saved(&app, watched.path(), "opus").await;

    choose_grilling(&app, first, fable.id, MODEL).await;
    choose_implementation(&app, first, fable.id, MODEL).await;
    grill(dir.path(), first).await;

    // Prefilled with fable, changed to opus, and grilled on the change.
    let second = another(&app).await;
    assert_eq!(
        choose_implementation(&app, second, opus.id, MODELS[0]).await,
        ProfileChosen::Chosen
    );
    grill(dir.path(), second).await;

    let view = opened(&app, another(&app).await).await;
    assert_eq!(
        view.grilling_pairing.pairing().map(|p| p.profile.id),
        Some(fable.id),
        "the half nobody touched is still what it was"
    );

    let implementation = view.implementation_pairing.expect("and the changed half");
    assert_eq!(implementation.profile.id, opus.id);
    assert_eq!(implementation.model.as_deref(), Some(MODELS[0]));
}

/// A remembered Profile whose pair has gone is a session that would fail to
/// start, so the picker arrives unchosen rather than holding one.
#[tokio::test]
async fn a_remembered_profile_whose_pair_has_gone_is_not_prefilled() {
    let (watched, dir, app) = workbench().await;
    let id = conversation(&app, watched.path()).await;
    let fable = saved(&app, watched.path(), "fable").await;
    let opus = saved(&app, watched.path(), "opus").await;

    choose_grilling(&app, id, fable.id, MODEL).await;
    choose_implementation(&app, id, opus.id, MODEL).await;
    grill(dir.path(), id).await;

    std::fs::remove_file(watched.path().join("opus/.claude.json")).unwrap();

    let view = opened(&app, another(&app).await).await;
    assert_eq!(
        view.grilling_pairing.pairing().map(|p| p.profile.id),
        Some(fable.id),
        "the half that is still there is still prefilled"
    );
    assert_eq!(view.implementation_pairing, None);
}

/// And a remembered model the Profile has since stopped listing, the same way:
/// the Profile is fine, and that pairing of it is not one any more.
#[tokio::test]
async fn a_remembered_model_a_profile_no_longer_lists_is_not_prefilled() {
    let (watched, dir, app) = workbench().await;
    let id = conversation(&app, watched.path()).await;
    let work = saved(&app, watched.path(), "work").await;

    choose_grilling(&app, id, work.id, MODEL).await;
    choose_implementation(&app, id, work.id, MODEL).await;
    grill(dir.path(), id).await;

    // Retyped without the model both halves were remembered with.
    let (claude_dir, config_file) = pair(watched.path(), "work");
    let rewritten: ProfileSaved = post(
        &app,
        &format!("/api/ui/profiles/{}", work.id),
        &edit("work", &claude_dir, &config_file, &[MODELS[0]]),
    )
    .await;
    assert_eq!(rewritten, ProfileSaved::Saved);

    let view = opened(&app, another(&app).await).await;
    assert_eq!(view.grilling_pairing, PickedView::Nothing);
    assert_eq!(view.implementation_pairing, None);
}

/// And the row that runs no session is remembered the same way, because it is a
/// pick like any other: the next draft on that Repo arrives on it.
#[tokio::test]
async fn a_new_conversation_arrives_with_no_review_where_that_is_what_was_grilled() {
    let (watched, dir, app) = workbench().await;
    let id = conversation(&app, watched.path()).await;
    let fable = saved(&app, watched.path(), "fable").await;

    choose_grilling(&app, id, fable.id, MODEL).await;
    choose_implementation(&app, id, fable.id, MODEL).await;
    assert_eq!(no_review(&app, id).await, ProfileChosen::Chosen);
    grill(dir.path(), id).await;

    assert_eq!(
        opened(&app, another(&app).await).await.review_pairing,
        PickedView::Skipped,
        "what the human last picked, ready for them to change",
    );
}

/// And the Grilling picker's own such row, remembered and prefilled exactly as
/// the review one is: a Repo whose last work started from its Brief opens its
/// next draft on the same row.
#[tokio::test]
async fn a_new_conversation_arrives_with_no_grilling_where_that_is_what_was_started() {
    let (watched, dir, app) = workbench().await;
    let id = conversation(&app, watched.path()).await;
    let fable = saved(&app, watched.path(), "fable").await;

    assert_eq!(no_grilling(&app, id).await, ProfileChosen::Chosen);
    choose_implementation(&app, id, fable.id, MODEL).await;
    choose_review(&app, id, fable.id, MODEL).await;
    build(dir.path(), id).await;

    let view = opened(&app, another(&app).await).await;

    assert_eq!(
        view.grilling_pairing,
        PickedView::Skipped,
        "what the human last picked, ready for them to change",
    );
    assert_eq!(
        view.implementation_pairing
            .map(|pairing| pairing.profile.id),
        Some(fable.id),
        "and the pickers beside it are filled as they always were",
    );
}

/// And picking an account back on a Repo that remembers the row is what the
/// next draft after *that* arrives with.
#[tokio::test]
async fn a_grilling_pairing_started_after_no_grilling_is_what_gets_prefilled() {
    let (watched, dir, app) = workbench().await;
    let fable = saved(&app, watched.path(), "fable").await;

    let first = conversation(&app, watched.path()).await;
    assert_eq!(no_grilling(&app, first).await, ProfileChosen::Chosen);
    choose_implementation(&app, first, fable.id, MODEL).await;
    choose_review(&app, first, fable.id, MODEL).await;
    build(dir.path(), first).await;

    let second = another(&app).await;
    choose_grilling(&app, second, fable.id, MODEL).await;
    grill(dir.path(), second).await;

    assert_eq!(
        opened(&app, another(&app).await)
            .await
            .grilling_pairing
            .pairing()
            .map(|pairing| pairing.profile.id),
        Some(fable.id),
        "the account that interviewed last, with the row before it gone",
    );
}
