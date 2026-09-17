//! Agent Profiles over the viewer's namespace: saving one, what is refused, and
//! the two a Conversation chooses before anything will grill it.
//!
//! Asked of the *server*, through the endpoints, because that is where the
//! decisions are: whether the pair is really there, and whether each half is of
//! the shape its agent type keeps an account in. A form that checked either
//! would be a courtesy — this endpoint is reachable without one.
//!
//! Where an account is kept is not one of the decisions. Anywhere the server can
//! read is somewhere a Profile can name, the human's own `~/.claude` included.
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
    BriefSaved, Broken, ConversationView, PickedView, ProfileAccount, ProfileChosen,
    ProfileDeleted, ProfileEntry, ProfileSaved, Registered, RepoPairingsView, Started,
};
use verkstead_server::{open_database, router_keeping, store};

/// A directory to keep accounts and repositories in, the app over it, and the
/// directory holding the database alive.
///
/// The app is told to watch nothing, which is what a bare `verkstead serve` is:
/// every account below is saved from a directory the installation never heard
/// of.
async fn workbench() -> (tempfile::TempDir, tempfile::TempDir, Router) {
    let accounts = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    // Beside the database, as it falls out for the real server. Nothing in this
    // file grills, so nothing is ever put in it.
    let data_dir = dir.path().to_owned();

    (accounts, dir, router_keeping(pool, data_dir))
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

/// The body a save takes: a name, the models the account can run, and the
/// account itself — which says which agent type it is and carries that type's
/// own fields, Claude's pair here.
fn edit(name: &str, claude_dir: &Path, config_file: &Path, models: &[&str]) -> serde_json::Value {
    serde_json::json!({
        "name": name,
        "account": {
            "agent_type": "Claude",
            "claude_dir": claude_dir,
            "config_file": config_file,
        },
        "models": models,
    })
}

/// And the same body with no name in it at all, which is what the form sends
/// for an empty box: the null rather than the empty string.
fn unnamed(claude_dir: &Path, config_file: &Path) -> serde_json::Value {
    serde_json::json!({
        "name": null,
        "account": {
            "agent_type": "Claude",
            "claude_dir": claude_dir,
            "config_file": config_file,
        },
        "models": ["claude-opus-5"],
    })
}

/// A home at `root`, which is the whole of what a Codex account is.
fn home(root: &Path, account: &str) -> PathBuf {
    made(root.join(account).join(".codex"))
}

/// And the same for a Grok Build one, under the directory grok keeps an account
/// in.
fn grok_home(root: &Path, account: &str) -> PathBuf {
    made(root.join(account).join(".grok"))
}

/// And the same for an OpenCode one, whose home is judged by the two
/// directories opencode keeps an account in — so the fixture makes them, which
/// is what a `HOME=<it> opencode` run leaves behind.
fn opencode_home(root: &Path, account: &str) -> PathBuf {
    let home = root.join(account).join("opencode");
    made(home.join(".config/opencode"));
    made(home.join(".local/share/opencode"));
    home
}

fn made(home: PathBuf) -> PathBuf {
    std::fs::create_dir_all(&home).unwrap();
    home
}

/// And the body that saves a Profile of that type: the same fields around a
/// different account, because the type is what says which fields there are.
fn codex_edit(name: &str, home: &Path, models: &[&str]) -> serde_json::Value {
    serde_json::json!({
        "name": name,
        "account": { "agent_type": "Codex", "home": home },
        "models": models,
    })
}

/// And a Grok Build one, whose account is the same shape under a different word.
fn grok_edit(name: &str, home: &Path, models: &[&str]) -> serde_json::Value {
    serde_json::json!({
        "name": name,
        "account": { "agent_type": "Grok", "home": home },
        "models": models,
    })
}

/// And an OpenCode one, whose account is one home again under a third word.
fn opencode_edit(name: &str, home: &Path, models: &[&str]) -> serde_json::Value {
    serde_json::json!({
        "name": name,
        "account": { "agent_type": "OpenCode", "home": home },
        "models": models,
    })
}

async fn save(app: &Router, body: &serde_json::Value) -> ProfileSaved {
    post(app, "/api/ui/profiles", body).await
}

/// And the same body sent at a Profile that is already there, which is the one
/// form doing the other half of what it does.
async fn rewrite(app: &Router, id: i64, body: &serde_json::Value) -> ProfileSaved {
    post(app, &format!("/api/ui/profiles/{id}"), body).await
}

/// Two models apiece, so that a Pairing below can name one that is not the
/// first of the list — which is the whole of what a Pairing adds to a Profile.
const MODELS: [&str; 2] = ["claude-opus-5", "claude-fable-5"];

/// The one the Pairings here are made with: the second, for the reason above.
const MODEL: &str = MODELS[1];

/// Save one that ought to work, and hand back the row it became.
async fn saved(app: &Router, root: &Path, name: &str) -> ProfileEntry {
    let (claude_dir, config_file) = pair(root, name);

    assert_eq!(
        save(app, &edit(name, &claude_dir, &config_file, &MODELS)).await,
        ProfileSaved::Saved
    );

    listed(app)
        .await
        .into_iter()
        .find(|profile| profile.name.as_deref() == Some(name))
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

/// A Conversation to choose Profiles on, in a repository under `root`.
async fn conversation(app: &Router, root: &Path) -> i64 {
    let repo = repository(root.join("verkstead"));

    let registered: Registered =
        post(app, "/api/ui/repos", &serde_json::json!({ "path": repo })).await;
    assert!(matches!(registered, Registered::Added(_)));

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
    let (accounts, _dir, app) = workbench().await;

    let profile = saved(&app, accounts.path(), "work").await;

    assert_eq!(profile.name.as_deref(), Some("work"));
    assert_eq!(profile.models, MODELS);

    // The account, in the shape its agent type keeps one — and the resolved
    // paths in it, which are what will be bind-mounted.
    let resolved = accounts.path().canonicalize().unwrap();
    assert_eq!(
        profile.account,
        ProfileAccount::Claude {
            claude_dir: resolved.join("work/.claude").to_str().unwrap().to_owned(),
            config_file: resolved
                .join("work/.claude.json")
                .to_str()
                .unwrap()
                .to_owned(),
        }
    );

    // Its pair is where it was left, which is the ordinary case.
    assert_eq!(profile.broken, None);
}

/// The memory switch is on for a Profile saved without saying — what an older
/// client sends — and a save that switches it off reads back off, on the list
/// and after the server has been started again over the same database.
#[tokio::test]
async fn a_profile_saved_with_its_memory_off_reads_back_off() {
    let (accounts, dir, app) = workbench().await;

    let profile = saved(&app, accounts.path(), "work").await;
    assert!(
        profile.memory,
        "a save that says nothing about memory shares it"
    );

    let (claude_dir, config_file) = pair(accounts.path(), "work");
    let mut forgetting = edit("work", &claude_dir, &config_file, &MODELS);
    forgetting["memory"] = serde_json::Value::Bool(false);

    assert_eq!(
        rewrite(&app, profile.id, &forgetting).await,
        ProfileSaved::Saved
    );
    assert!(!listed(&app).await[0].memory);

    let reopened = router_keeping(
        open_database(&dir.path().join("verkstead.db"))
            .await
            .unwrap(),
        dir.path().to_owned(),
    );
    assert!(
        !listed(&reopened).await[0].memory,
        "and it is still off once the server is started again"
    );
}

#[tokio::test]
async fn a_profile_is_rewritten_whole_and_removed_when_nobody_is_running_under_it() {
    let (accounts, _dir, app) = workbench().await;
    let profile = saved(&app, accounts.path(), "work").await;
    let (claude_dir, config_file) = pair(accounts.path(), "anthropic");

    let rewritten: ProfileSaved = post(
        &app,
        &format!("/api/ui/profiles/{}", profile.id),
        &edit("anthropic", &claude_dir, &config_file, &["claude-fable-5"]),
    )
    .await;
    assert_eq!(rewritten, ProfileSaved::Saved);

    let rows = listed(&app).await;
    assert_eq!(rows.len(), 1, "rewriting one does not add another");
    assert_eq!(rows[0].name.as_deref(), Some("anthropic"));
    assert_eq!(rows[0].models, &["claude-fable-5"]);

    assert_eq!(remove(&app, profile.id).await, ProfileDeleted::Removed);
    assert!(listed(&app).await.is_empty());
}

#[tokio::test]
async fn profiles_come_back_by_name() {
    let (accounts, _dir, app) = workbench().await;
    saved(&app, accounts.path(), "work").await;
    saved(&app, accounts.path(), "anthropic").await;
    saved(&app, accounts.path(), "personal").await;

    let names: Vec<Option<String>> = listed(&app)
        .await
        .into_iter()
        .map(|profile| profile.name)
        .collect();

    assert_eq!(
        names,
        [
            Some("anthropic".to_owned()),
            Some("personal".to_owned()),
            Some("work".to_owned())
        ]
    );
}

/// Both halves have to be there, and each is refused by its own name: pointing
/// the config field at a directory is an easy mistake, and "that path is wrong"
/// would not say which one.
#[tokio::test]
async fn a_pair_that_is_not_there_is_refused_by_the_half_that_is_missing() {
    let (accounts, _dir, app) = workbench().await;
    let (claude_dir, config_file) = pair(accounts.path(), "work");
    let nowhere = accounts.path().join("never-made");

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
    let (accounts, _dir, app) = workbench().await;
    let (claude_dir, config_file) = pair(accounts.path(), "work");

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

/// The account a Profile most naturally names: the human's own login, at
/// `~/.claude` and `~/.claude.json`, which the server was never told about and
/// which nothing now refuses.
///
/// A home of its own rather than the machine's, because a test that read the
/// real `$HOME` would pass or fail on whether whoever ran it uses Claude.
#[tokio::test]
async fn a_pair_under_a_home_the_server_was_never_told_about_saves_and_reads_unbroken() {
    let (_accounts, _dir, app) = workbench().await;

    let home = tempfile::tempdir().unwrap();
    let claude_dir = home.path().join(".claude");
    let config_file = home.path().join(".claude.json");
    std::fs::create_dir(&claude_dir).unwrap();
    std::fs::write(&config_file, "{}\n").unwrap();

    assert_eq!(
        save(
            &app,
            &edit("mine", &claude_dir, &config_file, &["claude-opus-5"])
        )
        .await,
        ProfileSaved::Saved
    );

    let listed = listed(&app).await;
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].broken, None);

    let resolved = home.path().canonicalize().unwrap();
    assert_eq!(
        listed[0].account,
        ProfileAccount::Claude {
            claude_dir: resolved.join(".claude").to_str().unwrap().to_owned(),
            config_file: resolved.join(".claude.json").to_str().unwrap().to_owned(),
        }
    );
}

/// What is stored is the resolved pair rather than the one that was typed: a
/// symlink is followed first, so the row names the directory a session will
/// actually have mounted.
///
/// Made where a link can be made without asking anybody's permission, which is
/// both Unixes and not Windows. The resolving is the same everywhere — it is
/// `canonicalize` — so what is lost there is the making of the link rather than
/// any of the reasoning.
#[cfg(unix)]
#[tokio::test]
async fn a_pair_reached_through_a_symlink_is_stored_where_it_really_is() {
    let (accounts, _dir, app) = workbench().await;
    let (_, config_file) = pair(accounts.path(), "work");

    let elsewhere = tempfile::tempdir().unwrap();
    let (real_dir, _) = pair(elsewhere.path(), "real");

    let link = accounts.path().join("looks-local");
    std::os::unix::fs::symlink(&real_dir, &link).unwrap();

    assert_eq!(
        save(&app, &edit("work", &link, &config_file, &["claude-opus-5"])).await,
        ProfileSaved::Saved
    );

    let ProfileAccount::Claude { claude_dir, .. } = &listed(&app).await[0].account else {
        panic!("a Claude Profile should come back as a Claude account");
    };

    assert_eq!(
        claude_dir,
        real_dir.canonicalize().unwrap().to_str().unwrap()
    );
}

#[tokio::test]
async fn a_relative_path_is_refused_without_being_resolved() {
    let (accounts, _dir, app) = workbench().await;
    let (claude_dir, config_file) = pair(accounts.path(), "work");

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
    let (accounts, _dir, app) = workbench().await;
    let (claude_dir, config_file) = pair(accounts.path(), "work");

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

/// A Profile is run on one of its models, and a Profile naming none is one
/// nothing could be launched under.
///
/// A name is not the same kind of field. It tells two accounts of one harness
/// apart, and a harness with one account has nothing to tell apart — see
/// [`a_profile_nobody_named_is_saved_as_one`].
#[tokio::test]
async fn a_profile_with_no_models_is_refused() {
    let (accounts, _dir, app) = workbench().await;
    let (claude_dir, config_file) = pair(accounts.path(), "work");

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
    let (accounts, _dir, app) = workbench().await;
    saved(&app, accounts.path(), "work").await;

    let (claude_dir, config_file) = pair(accounts.path(), "second");
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

/// A name tells two accounts of one harness apart. A fresh Verkstead has one
/// account per harness and nobody has typed a word for it, so a Profile may go
/// without: the null goes down as the null and the row comes back with no name.
///
/// A box with nothing but spaces in it is the same thing said by a human — the
/// form's empty box either way — so it is saved as no name rather than as a name
/// nobody could tell from none.
#[tokio::test]
async fn a_profile_nobody_named_is_saved_as_one() {
    let (accounts, _dir, app) = workbench().await;
    let (claude_dir, config_file) = pair(accounts.path(), "work");

    assert_eq!(
        save(&app, &unnamed(&claude_dir, &config_file)).await,
        ProfileSaved::Saved
    );

    let rows = listed(&app).await;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].name, None);
    assert_eq!(rows[0].models, ["claude-opus-5"]);

    let (spaced_dir, spaced_config) = pair(accounts.path(), "spaced");
    assert_eq!(
        save(
            &app,
            &edit("   ", &spaced_dir, &spaced_config, &["gpt-5.2-codex"])
        )
        .await,
        ProfileSaved::DefaultTaken,
        "a box of spaces is an empty box, so this is the second unnamed Claude",
    );
}

/// One unnamed Profile per harness, which is the rule the unique name always
/// was: a second row nobody named would draw exactly as the first one does.
///
/// Per harness rather than outright, because the harness's own mark is what
/// tells them apart on the page — an unnamed Claude Code account beside an
/// unnamed Codex one is two rows nobody could confuse.
#[tokio::test]
async fn a_harness_takes_one_profile_nobody_named() {
    let (accounts, _dir, app) = workbench().await;
    let (claude_dir, config_file) = pair(accounts.path(), "work");

    assert_eq!(
        save(&app, &unnamed(&claude_dir, &config_file)).await,
        ProfileSaved::Saved
    );

    let (second_dir, second_config) = pair(accounts.path(), "second");
    assert_eq!(
        save(&app, &unnamed(&second_dir, &second_config)).await,
        ProfileSaved::DefaultTaken
    );

    assert_eq!(
        save(
            &app,
            &serde_json::json!({
                "name": null,
                "account": {
                    "agent_type": "Codex",
                    "home": home(accounts.path(), "codex"),
                },
                "models": ["gpt-5.2-codex"],
            })
        )
        .await,
        ProfileSaved::Saved,
        "another harness's unnamed account is not this one's",
    );

    assert_eq!(listed(&app).await.len(), 2);
}

/// And the rule holds on a rewrite as it does on a save: the one unnamed row a
/// harness has is not a row a second Profile can be turned into.
#[tokio::test]
async fn taking_the_name_off_a_second_profile_is_refused() {
    let (accounts, _dir, app) = workbench().await;
    let (claude_dir, config_file) = pair(accounts.path(), "work");
    assert_eq!(
        save(&app, &unnamed(&claude_dir, &config_file)).await,
        ProfileSaved::Saved
    );

    let named = saved(&app, accounts.path(), "anthropic").await;
    let (second_dir, second_config) = pair(accounts.path(), "anthropic");

    assert_eq!(
        rewrite(&app, named.id, &unnamed(&second_dir, &second_config)).await,
        ProfileSaved::DefaultTaken
    );

    // And off the only one there is, which is nobody's clash but its own.
    let alone = listed(&app).await;
    let unnamed_id = alone
        .iter()
        .find(|profile| profile.name.is_none())
        .expect("the unnamed one is on the list")
        .id;

    assert_eq!(
        rewrite(&app, unnamed_id, &unnamed(&claude_dir, &config_file)).await,
        ProfileSaved::Saved
    );
}

/// The pair was there when it was saved; a directory can be moved afterwards.
/// The list says so rather than leaving it to be found out when a session will
/// not start.
#[tokio::test]
async fn a_profile_whose_pair_has_disappeared_reads_as_broken() {
    let (accounts, _dir, app) = workbench().await;
    let profile = saved(&app, accounts.path(), "work").await;
    assert_eq!(profile.broken, None);

    std::fs::remove_dir_all(accounts.path().join("work/.claude")).unwrap();
    assert_eq!(listed(&app).await[0].broken, Some(Broken::DirMissing));

    // Put the directory back and take the file instead: the other half breaks it
    // just the same, and says which half it was.
    std::fs::create_dir_all(accounts.path().join("work/.claude")).unwrap();
    std::fs::remove_file(accounts.path().join("work/.claude.json")).unwrap();
    assert_eq!(listed(&app).await[0].broken, Some(Broken::ConfigMissing));
}

/// A Profile whose account is one home goes over the same endpoints and comes
/// back with the resolved home it was saved with.
///
/// The account is a shape rather than a pair every Profile is assumed to have,
/// so the whole of what differs is which fields the account carries — the name,
/// the models and every judgement about the path are the same either way.
#[tokio::test]
async fn a_profile_whose_account_is_one_home_saves_and_reads_back_with_it() {
    let (accounts, _dir, app) = workbench().await;

    assert_eq!(
        save(
            &app,
            &codex_edit("codex", &home(accounts.path(), "codex"), &MODELS)
        )
        .await,
        ProfileSaved::Saved
    );

    let profile = &listed(&app).await[0];
    let resolved = accounts.path().canonicalize().unwrap();

    assert_eq!(profile.name.as_deref(), Some("codex"));
    assert_eq!(profile.models, MODELS);
    assert_eq!(
        profile.account,
        ProfileAccount::Codex {
            home: resolved.join("codex/.codex").to_str().unwrap().to_owned(),
        }
    );
    assert_eq!(profile.broken, None);
}

/// Types keeping one home apiece stay the types they are over the wire.
///
/// The account carries the type it is, and all three of these carry the same
/// field under it — so what this settles is that a Profile saved as one of them
/// is listed as that one, rather than coming back as another type that keeps a
/// home.
#[tokio::test]
async fn a_home_account_saves_and_reads_back_as_its_own_type() {
    let (accounts, _dir, app) = workbench().await;

    assert_eq!(
        save(
            &app,
            &codex_edit("codex", &home(accounts.path(), "codex"), &MODELS)
        )
        .await,
        ProfileSaved::Saved
    );
    assert_eq!(
        save(
            &app,
            &grok_edit("grok", &grok_home(accounts.path(), "grok"), &MODELS)
        )
        .await,
        ProfileSaved::Saved
    );
    assert_eq!(
        save(
            &app,
            &opencode_edit(
                "opencode",
                &opencode_home(accounts.path(), "opencode"),
                &MODELS
            )
        )
        .await,
        ProfileSaved::Saved
    );

    let listed = listed(&app).await;
    let resolved = accounts.path().canonicalize().unwrap();

    assert_eq!(
        listed
            .iter()
            .map(|profile| profile.account.clone())
            .collect::<Vec<_>>(),
        vec![
            ProfileAccount::Codex {
                home: resolved.join("codex/.codex").to_str().unwrap().to_owned(),
            },
            ProfileAccount::Grok {
                home: resolved.join("grok/.grok").to_str().unwrap().to_owned(),
            },
            ProfileAccount::OpenCode {
                home: resolved
                    .join("opencode/opencode")
                    .to_str()
                    .unwrap()
                    .to_owned(),
            },
        ]
    );
    assert!(listed.iter().all(|profile| profile.broken.is_none()));
}

/// An OpenCode home is judged by what opencode keeps an account in rather than
/// by the directory holding it: those two are what a session mounts, and a
/// directory that has never had opencode run in it holds neither.
///
/// Refused when it is saved and read as broken afterwards, which is the same
/// pair of answers every other account's paths get — the failure moved forward
/// in time from the session that would otherwise fail to start with nobody
/// watching.
#[tokio::test]
async fn an_opencode_home_without_the_directories_opencode_reads_is_refused() {
    let (accounts, _dir, app) = workbench().await;
    let bare = made(accounts.path().join("bare/opencode"));

    assert_eq!(
        save(&app, &opencode_edit("bare", &bare, &MODELS)).await,
        ProfileSaved::HomeMissing
    );

    let kept = opencode_home(accounts.path(), "opencode");
    assert_eq!(
        save(&app, &opencode_edit("opencode", &kept, &MODELS)).await,
        ProfileSaved::Saved
    );
    assert_eq!(listed(&app).await[0].broken, None);

    // The account itself taken away, with the home it sat under left where it
    // was: the directory a Profile names is still there, and there is no
    // account in it any more.
    std::fs::remove_dir_all(kept.join(".local/share/opencode")).unwrap();
    assert_eq!(listed(&app).await[0].broken, Some(Broken::HomeMissing));
}

/// And its home is judged the way a pair's halves are: named by what it is when
/// it has gone.
///
/// The link half is made where one can be made without asking anybody's
/// permission, which is both Unixes and not Windows. The resolving is the same
/// everywhere — it is `canonicalize` — so what is lost there is the making of
/// the link rather than any of the reasoning.
#[cfg(unix)]
#[tokio::test]
async fn a_home_that_has_gone_reads_as_broken() {
    let (accounts, _dir, app) = workbench().await;
    let kept = home(accounts.path(), "codex");

    assert_eq!(
        save(&app, &codex_edit("codex", &kept, &MODELS)).await,
        ProfileSaved::Saved
    );
    assert_eq!(listed(&app).await[0].broken, None);

    std::fs::remove_dir_all(&kept).unwrap();
    assert_eq!(listed(&app).await[0].broken, Some(Broken::HomeMissing));

    // Back, but as a link to a directory nobody made: the link is there and what
    // it names is not, so there is still nothing to mount.
    let elsewhere = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(elsewhere.path().join("never-made"), &kept).unwrap();

    assert_eq!(listed(&app).await[0].broken, Some(Broken::HomeMissing));
}

/// Every way a home is refused a save, each named for the home rather than for
/// one of Claude's two paths — the refusal is what the human is shown, and a
/// Codex Profile has no config file to be told about.
#[tokio::test]
async fn a_home_is_refused_by_its_own_name() {
    let (accounts, _dir, app) = workbench().await;

    assert_eq!(
        save(
            &app,
            &codex_edit("codex", Path::new("accounts/.codex"), &MODELS)
        )
        .await,
        ProfileSaved::HomeNotAbsolute
    );
    assert_eq!(
        save(
            &app,
            &codex_edit("codex", &accounts.path().join("nowhere"), &MODELS)
        )
        .await,
        ProfileSaved::HomeMissing
    );

    // A file where the home goes: a home is a directory bind-mounted over, so
    // nothing else can stand in for one.
    let file = accounts.path().join("a-file");
    std::fs::write(&file, "not a home\n").unwrap();
    assert_eq!(
        save(&app, &codex_edit("codex", &file, &MODELS)).await,
        ProfileSaved::HomeNotADirectory
    );
}

/// Broken is asked by resolving rather than by asking whether something is
/// there: a directory replaced by a link to nowhere is a path a session cannot
/// be launched under, however much of it still reads.
///
/// Made where a link can be made without asking anybody's permission, which is
/// both Unixes and not Windows. The resolving is the same everywhere — it is
/// `canonicalize` — so what is lost there is the making of the link rather than
/// any of the reasoning.
#[cfg(unix)]
#[tokio::test]
async fn a_pair_whose_directory_is_now_a_link_to_nowhere_reads_as_broken() {
    let (accounts, _dir, app) = workbench().await;
    saved(&app, accounts.path(), "work").await;

    let claude_dir = accounts.path().join("work/.claude");
    std::fs::remove_dir_all(&claude_dir).unwrap();
    std::os::unix::fs::symlink(accounts.path().join("never-made"), &claude_dir).unwrap();

    assert_eq!(listed(&app).await[0].broken, Some(Broken::DirMissing));
}

/// They are separate choices because they are genuinely separate accounts and
/// models — grill on fable, implement on opus.
#[tokio::test]
async fn a_conversation_chooses_its_two_pairings_independently() {
    let (accounts, _dir, app) = workbench().await;
    let id = conversation(&app, accounts.path()).await;
    let fable = saved(&app, accounts.path(), "fable").await;
    let opus = saved(&app, accounts.path(), "opus").await;

    assert_eq!(
        choose_grilling(&app, id, fable.id, MODEL).await,
        ProfileChosen::Chosen
    );

    let half = opened(&app, id).await;
    assert_eq!(
        half.grilling_pairing
            .pairing()
            .and_then(|p| p.profile.name.as_deref()),
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
            .and_then(|p| p.profile.name.clone()),
        Some("fable".to_owned())
    );
    assert_eq!(
        both.implementation_pairing.and_then(|p| p.profile.name),
        Some("opus".to_owned())
    );
}

/// The model is half of the choice and it is the half that is picked: the pane
/// says back the model that was paired rather than whatever the Profile lists
/// first.
#[tokio::test]
async fn a_pairing_says_back_the_model_it_was_chosen_with() {
    let (accounts, _dir, app) = workbench().await;
    let id = conversation(&app, accounts.path()).await;
    let work = saved(&app, accounts.path(), "work").await;

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
    let (accounts, _dir, app) = workbench().await;
    let id = conversation(&app, accounts.path()).await;
    let work = saved(&app, accounts.path(), "work").await;

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
    let (accounts, _dir, app) = workbench().await;
    let id = conversation(&app, accounts.path()).await;
    let fable = saved(&app, accounts.path(), "fable").await;
    let opus = saved(&app, accounts.path(), "opus").await;
    let haiku = saved(&app, accounts.path(), "haiku").await;

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
    let (accounts, dir, app) = workbench().await;
    let id = conversation(&app, accounts.path()).await;
    let work = saved(&app, accounts.path(), "work").await;

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
    let (accounts, _dir, app) = workbench().await;
    let id = conversation(&app, accounts.path()).await;
    let fable = saved(&app, accounts.path(), "fable").await;
    let opus = saved(&app, accounts.path(), "opus").await;
    let haiku = saved(&app, accounts.path(), "haiku").await;

    choose_grilling(&app, id, fable.id, MODEL).await;
    choose_implementation(&app, id, opus.id, MODEL).await;
    choose_review(&app, id, haiku.id, MODEL).await;
    assert!(opened(&app, id).await.ready_to_grill);

    std::fs::remove_file(accounts.path().join("opus/.claude.json")).unwrap();

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
    let (accounts, _dir, app) = workbench().await;
    let id = conversation(&app, accounts.path()).await;
    let only = saved(&app, accounts.path(), "work").await;

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

/// Taken away rather than refused, and the Conversation that had chosen it
/// reads as one that has chosen nothing.
///
/// Which is the pane's own answer to the removal: the Configuration says *Not
/// chosen* where it said the account's name, and the Conversation is no longer
/// ready to grill — so the human is looking at the picker they have to fill in
/// again rather than at a run that will fail later for reasons nothing shows.
#[tokio::test]
async fn a_profile_a_conversation_has_chosen_is_removed_and_nulled_out_of_it() {
    let (accounts, _dir, app) = workbench().await;
    let id = conversation(&app, accounts.path()).await;
    let profile = saved(&app, accounts.path(), "work").await;

    choose_grilling(&app, id, profile.id, MODEL).await;
    choose_implementation(&app, id, profile.id, MODEL).await;
    choose_review(&app, id, profile.id, MODEL).await;

    assert!(opened(&app, id).await.ready_to_grill);

    assert_eq!(remove(&app, profile.id).await, ProfileDeleted::Removed);
    assert!(listed(&app).await.is_empty());

    let view = opened(&app, id).await;

    assert_eq!(view.grilling_pairing, PickedView::Nothing);
    assert_eq!(view.implementation_pairing, None);
    assert_eq!(view.review_pairing, PickedView::Nothing);
    assert!(
        !view.ready_to_grill,
        "a Conversation with no account to run under is not one to start",
    );
}

#[tokio::test]
async fn a_profile_that_is_not_there_says_so_however_it_is_asked_about() {
    let (accounts, _dir, app) = workbench().await;
    let id = conversation(&app, accounts.path()).await;
    let (claude_dir, config_file) = pair(accounts.path(), "work");

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
    let (accounts, _dir, app) = workbench().await;
    let profile = saved(&app, accounts.path(), "work").await;

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
    let (accounts, _dir, app) = workbench().await;
    let (claude_dir, config_file) = pair(accounts.path(), "work");

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
    let (_accounts, _dir, app) = workbench().await;

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
    let (accounts, dir, app) = workbench().await;
    let id = conversation(&app, accounts.path()).await;
    let fable = saved(&app, accounts.path(), "fable").await;
    let opus = saved(&app, accounts.path(), "opus").await;
    let haiku = saved(&app, accounts.path(), "haiku").await;

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
    let (accounts, dir, app) = workbench().await;
    let first = conversation(&app, accounts.path()).await;
    let fable = saved(&app, accounts.path(), "fable").await;
    let opus = saved(&app, accounts.path(), "opus").await;

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
    let (accounts, dir, app) = workbench().await;
    let id = conversation(&app, accounts.path()).await;
    let fable = saved(&app, accounts.path(), "fable").await;
    let opus = saved(&app, accounts.path(), "opus").await;

    choose_grilling(&app, id, fable.id, MODEL).await;
    choose_implementation(&app, id, opus.id, MODEL).await;
    grill(dir.path(), id).await;

    std::fs::remove_file(accounts.path().join("opus/.claude.json")).unwrap();

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
    let (accounts, dir, app) = workbench().await;
    let id = conversation(&app, accounts.path()).await;
    let work = saved(&app, accounts.path(), "work").await;

    choose_grilling(&app, id, work.id, MODEL).await;
    choose_implementation(&app, id, work.id, MODEL).await;
    grill(dir.path(), id).await;

    // Retyped without the model both halves were remembered with.
    let (claude_dir, config_file) = pair(accounts.path(), "work");
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
    let (accounts, dir, app) = workbench().await;
    let id = conversation(&app, accounts.path()).await;
    let fable = saved(&app, accounts.path(), "fable").await;

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
    let (accounts, dir, app) = workbench().await;
    let id = conversation(&app, accounts.path()).await;
    let fable = saved(&app, accounts.path(), "fable").await;

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
    let (accounts, dir, app) = workbench().await;
    let fable = saved(&app, accounts.path(), "fable").await;

    let first = conversation(&app, accounts.path()).await;
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

/// And the same memory read without a Conversation to have applied it to, which
/// is what a page composing one asks for: the prefill itself, off the Repo.
async fn prefill(app: &Router, repo_id: i64) -> RepoPairingsView {
    get(app, &format!("/api/ui/repos/{repo_id}/pairings")).await
}

/// The Repo the fixtures above register, which is the only one in them.
async fn only_repo(app: &Router) -> i64 {
    let repos: Vec<verkstead_render::RepoEntry> = get(app, "/api/ui/repos").await;
    repos[0].id
}

/// What a Conversation would arrive filled with, answered before there is one:
/// the same three the pane above draws, off the Repo rather than off a record.
#[tokio::test]
async fn a_repos_pairings_are_offered_before_anything_is_created() {
    let (accounts, dir, app) = workbench().await;
    let id = conversation(&app, accounts.path()).await;
    let fable = saved(&app, accounts.path(), "fable").await;
    let opus = saved(&app, accounts.path(), "opus").await;
    let haiku = saved(&app, accounts.path(), "haiku").await;

    choose_grilling(&app, id, fable.id, MODEL).await;
    choose_implementation(&app, id, opus.id, MODEL).await;
    choose_review(&app, id, haiku.id, MODEL).await;
    grill(dir.path(), id).await;

    let offered = prefill(&app, only_repo(&app).await).await;

    let grilling = offered
        .grilling
        .pairing()
        .expect("the grilling picker has something to show");
    assert_eq!(grilling.profile.id, fable.id);
    assert_eq!(grilling.model.as_deref(), Some(MODEL));

    let implementation = offered
        .implementation
        .clone()
        .expect("and so does the implementation one");
    assert_eq!(implementation.profile.id, opus.id);
    assert_eq!(implementation.model.as_deref(), Some(MODEL));

    let review = offered
        .review
        .pairing()
        .expect("and so does the review one");
    assert_eq!(review.profile.id, haiku.id);
    assert_eq!(review.model.as_deref(), Some(MODEL));

    // The very thing creation would have applied: a Conversation started on this
    // Repo now arrives showing exactly what was offered above.
    let view = opened(&app, another(&app).await).await;
    assert_eq!(view.grilling_pairing, offered.grilling);
    assert_eq!(view.implementation_pairing, offered.implementation);
    assert_eq!(view.review_pairing, offered.review);
}

/// And the row that runs no session, which is a pick like any other and comes
/// back as itself: there is no Profile in it to have gone.
#[tokio::test]
async fn a_repo_that_last_ran_no_review_offers_that() {
    let (accounts, dir, app) = workbench().await;
    let id = conversation(&app, accounts.path()).await;
    let fable = saved(&app, accounts.path(), "fable").await;

    choose_grilling(&app, id, fable.id, MODEL).await;
    choose_implementation(&app, id, fable.id, MODEL).await;
    assert_eq!(no_review(&app, id).await, ProfileChosen::Chosen);
    grill(dir.path(), id).await;

    assert_eq!(
        prefill(&app, only_repo(&app).await).await.review,
        PickedView::Skipped,
    );
}

/// A remembered Pairing whose Profile has broken is nothing to fill a picker
/// with, so the role comes back unchosen — the same nothing a Repo that has
/// never been grilled comes back as, and exactly what creation would have
/// silently skipped.
#[tokio::test]
async fn a_memory_that_no_longer_applies_is_offered_as_nothing() {
    let (accounts, dir, app) = workbench().await;
    let id = conversation(&app, accounts.path()).await;
    let fable = saved(&app, accounts.path(), "fable").await;
    let opus = saved(&app, accounts.path(), "opus").await;

    choose_grilling(&app, id, fable.id, MODEL).await;
    choose_implementation(&app, id, opus.id, MODEL).await;
    grill(dir.path(), id).await;

    std::fs::remove_file(accounts.path().join("opus/.claude.json")).unwrap();

    let offered = prefill(&app, only_repo(&app).await).await;
    assert_eq!(
        offered.grilling.pairing().map(|pairing| pairing.profile.id),
        Some(fable.id),
        "the half that is still there is still offered"
    );
    assert_eq!(offered.implementation, None);
}

/// And a remembered model the Profile has since stopped listing, the same way:
/// the Profile is fine, and that pairing of it is not one any more.
#[tokio::test]
async fn a_model_a_profile_no_longer_lists_is_offered_as_nothing() {
    let (accounts, dir, app) = workbench().await;
    let id = conversation(&app, accounts.path()).await;
    let work = saved(&app, accounts.path(), "work").await;

    choose_grilling(&app, id, work.id, MODEL).await;
    choose_implementation(&app, id, work.id, MODEL).await;
    grill(dir.path(), id).await;

    // Retyped without the model both halves were remembered with.
    let (claude_dir, config_file) = pair(accounts.path(), "work");
    let rewritten: ProfileSaved = post(
        &app,
        &format!("/api/ui/profiles/{}", work.id),
        &edit("work", &claude_dir, &config_file, &[MODELS[0]]),
    )
    .await;
    assert_eq!(rewritten, ProfileSaved::Saved);

    let offered = prefill(&app, only_repo(&app).await).await;
    assert_eq!(offered.grilling, PickedView::Nothing);
    assert_eq!(offered.implementation, None);
}

/// A Repo nothing has been grilled on has nothing to offer, which is three
/// empty pickers rather than a refusal: it is where every Repo starts.
#[tokio::test]
async fn a_repo_with_no_memory_offers_nothing_at_all() {
    let (accounts, _dir, app) = workbench().await;
    conversation(&app, accounts.path()).await;

    let offered = prefill(&app, only_repo(&app).await).await;
    assert_eq!(offered.grilling, PickedView::Nothing);
    assert_eq!(offered.implementation, None);
    assert_eq!(offered.review, PickedView::Nothing);
}

/// And a Repo that is not registered has no memory to read, which is a 404 for
/// the reason its branches are: the page has a repo to pick again rather than a
/// failure to report.
#[tokio::test]
async fn the_pairings_of_a_repo_that_is_not_there_are_refused() {
    let (_accounts, _dir, app) = workbench().await;

    for asked in ["404", "nonsense"] {
        let (status, _) = fetch(
            &app,
            Request::builder()
                .uri(format!("/api/ui/repos/{asked}/pairings"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;

        assert_eq!(status, StatusCode::NOT_FOUND);
    }
}

/// Another repository registered beside whatever is there, by name, handed back
/// as the Repo's id — a Repo nothing has been grilled on.
async fn fresh_repo(app: &Router, root: &Path, name: &str) -> i64 {
    let repo = repository(root.join(name));

    let registered: Registered =
        post(app, "/api/ui/repos", &serde_json::json!({ "path": repo })).await;
    assert!(matches!(registered, Registered::Added(_)));

    let repos: Vec<verkstead_render::RepoEntry> = get(app, "/api/ui/repos").await;
    repos
        .into_iter()
        .find(|entry| entry.name == name)
        .expect("the Repo just registered should be on the list")
        .id
}

/// And a Conversation started on a Repo by id, the way the human starts one.
async fn started_on(app: &Router, repo_id: i64) -> i64 {
    let started: Started = post(
        app,
        "/api/ui/conversations",
        &serde_json::json!({ "repo_id": repo_id }),
    )
    .await;

    let Started::Started { id } = started else {
        panic!("expected the Conversation to start, got {started:?}");
    };

    id
}

/// Which Profile and model a picker ended up on, for comparing a whole prefill
/// at a glance.
fn on(pairing: Option<&verkstead_render::PairingView>) -> Option<(i64, &str)> {
    pairing.map(|pairing| (pairing.profile.id, pairing.model.as_deref().unwrap()))
}

/// A Repo nothing has grilled arrives filled with what the last work anywhere
/// started under — offered on the compose page and applied to a Conversation
/// created on it alike.
#[tokio::test]
async fn a_repo_never_grilled_is_offered_what_the_last_start_anywhere_ran_under() {
    let (accounts, dir, app) = workbench().await;
    let id = conversation(&app, accounts.path()).await;
    let fable = saved(&app, accounts.path(), "fable").await;
    let opus = saved(&app, accounts.path(), "opus").await;
    let haiku = saved(&app, accounts.path(), "haiku").await;

    choose_grilling(&app, id, opus.id, MODEL).await;
    choose_implementation(&app, id, haiku.id, MODELS[0]).await;
    choose_review(&app, id, fable.id, MODEL).await;
    grill(dir.path(), id).await;

    let askance = fresh_repo(&app, accounts.path(), "askance").await;
    let offered = prefill(&app, askance).await;

    assert_eq!(on(offered.grilling.pairing()), Some((opus.id, MODEL)));
    assert_eq!(
        on(offered.implementation.as_ref()),
        Some((haiku.id, MODELS[0]))
    );
    assert_eq!(on(offered.review.pairing()), Some((fable.id, MODEL)));

    let view = opened(&app, started_on(&app, askance).await).await;
    assert_eq!(view.grilling_pairing, offered.grilling);
    assert_eq!(view.implementation_pairing, offered.implementation);
    assert_eq!(view.review_pairing, offered.review);
}

/// A draft is not a start, however recently its pickers were filled: the copy
/// is of the last Conversation whose work began.
#[tokio::test]
async fn a_draft_filled_since_the_last_start_is_not_what_a_fresh_repo_copies() {
    let (accounts, dir, app) = workbench().await;
    let id = conversation(&app, accounts.path()).await;
    let fable = saved(&app, accounts.path(), "fable").await;
    let opus = saved(&app, accounts.path(), "opus").await;

    choose_grilling(&app, id, opus.id, MODEL).await;
    choose_implementation(&app, id, opus.id, MODEL).await;
    choose_review(&app, id, opus.id, MODEL).await;
    grill(dir.path(), id).await;

    let draft = another(&app).await;
    choose_grilling(&app, draft, fable.id, MODELS[0]).await;
    choose_implementation(&app, draft, fable.id, MODELS[0]).await;
    choose_review(&app, draft, fable.id, MODELS[0]).await;

    let offered = prefill(&app, fresh_repo(&app, accounts.path(), "askance").await).await;
    assert_eq!(on(offered.grilling.pairing()), Some((opus.id, MODEL)));
    assert_eq!(on(offered.implementation.as_ref()), Some((opus.id, MODEL)));
    assert_eq!(on(offered.review.pairing()), Some((opus.id, MODEL)));
}

/// A role the last start picked away is not picked away on a fresh Repo: it
/// takes the platform default, as a role that start left empty does.
#[tokio::test]
async fn a_skip_on_the_last_start_is_not_carried_to_a_fresh_repo() {
    let (accounts, dir, app) = workbench().await;
    let id = conversation(&app, accounts.path()).await;
    let fable = saved(&app, accounts.path(), "fable").await;
    let opus = saved(&app, accounts.path(), "opus").await;

    assert_eq!(no_grilling(&app, id).await, ProfileChosen::Chosen);
    choose_implementation(&app, id, opus.id, MODEL).await;
    assert_eq!(no_review(&app, id).await, ProfileChosen::Chosen);
    build(dir.path(), id).await;

    let offered = prefill(&app, fresh_repo(&app, accounts.path(), "askance").await).await;

    // The default is the earliest Claude Profile, on the table's model per role —
    // both of which that Profile lists.
    assert_eq!(
        on(offered.grilling.pairing()),
        Some((fable.id, "claude-fable-5"))
    );
    assert_eq!(on(offered.implementation.as_ref()), Some((opus.id, MODEL)));
    assert_eq!(
        on(offered.review.pairing()),
        Some((fable.id, "claude-opus-5"))
    );
}

/// A Repo remembering anything at all — even one role, the others never picked
/// when it was grilled — was grilled, and gets its memory and nothing else.
#[tokio::test]
async fn a_repo_with_partial_memory_is_not_filled_from_anywhere_else() {
    let (accounts, dir, app) = workbench().await;
    let id = conversation(&app, accounts.path()).await;
    let fable = saved(&app, accounts.path(), "fable").await;

    choose_grilling(&app, id, fable.id, MODEL).await;
    grill(dir.path(), id).await;

    let offered = prefill(&app, only_repo(&app).await).await;
    assert_eq!(on(offered.grilling.pairing()), Some((fable.id, MODEL)));
    assert_eq!(offered.implementation, None);
    assert_eq!(offered.review, PickedView::Nothing);

    let view = opened(&app, another(&app).await).await;
    assert_eq!(view.implementation_pairing, None);
    assert_eq!(view.review_pairing, PickedView::Nothing);
}

/// With nothing started anywhere, the platform default: Claude Code before any
/// other harness whatever was saved first, and its unnamed Profile before a
/// named one saved earlier.
#[tokio::test]
async fn the_platform_default_prefers_claude_code_and_its_unnamed_profile() {
    let (accounts, _dir, app) = workbench().await;
    let repo = fresh_repo(&app, accounts.path(), "verkstead").await;

    let codex = home(accounts.path(), "codex");
    assert_eq!(
        save(&app, &codex_edit("codex", &codex, &["gpt-5-codex"])).await,
        ProfileSaved::Saved
    );
    saved(&app, accounts.path(), "work").await;

    let (claude_dir, config_file) = pair(accounts.path(), "default");
    assert_eq!(
        save(
            &app,
            &serde_json::json!({
                "name": null,
                "account": {
                    "agent_type": "Claude",
                    "claude_dir": claude_dir,
                    "config_file": config_file,
                },
                "models": MODELS,
            }),
        )
        .await,
        ProfileSaved::Saved
    );
    let unnamed = listed(&app)
        .await
        .into_iter()
        .find(|profile| profile.name.is_none())
        .unwrap();

    let offered = prefill(&app, repo).await;
    assert_eq!(
        on(offered.grilling.pairing()),
        Some((unnamed.id, "claude-fable-5"))
    );
    assert_eq!(
        on(offered.implementation.as_ref()),
        Some((unnamed.id, "claude-opus-5"))
    );
    assert_eq!(
        on(offered.review.pairing()),
        Some((unnamed.id, "claude-opus-5"))
    );
}

/// With no unnamed Profile on the harness, the earliest saved — by when it was
/// saved rather than by its name.
#[tokio::test]
async fn the_platform_default_takes_the_earliest_named_profile() {
    let (accounts, _dir, app) = workbench().await;
    let repo = fresh_repo(&app, accounts.path(), "verkstead").await;
    let zeta = saved(&app, accounts.path(), "zeta").await;
    saved(&app, accounts.path(), "alpha").await;

    let offered = prefill(&app, repo).await;
    assert_eq!(
        on(offered.grilling.pairing()),
        Some((zeta.id, "claude-fable-5"))
    );
    assert_eq!(
        on(offered.implementation.as_ref()),
        Some((zeta.id, "claude-opus-5"))
    );
}

/// Every harness has its own row in the table, and a Profile that does not list
/// the row's model is run on the first model it does list.
#[tokio::test]
async fn the_platform_default_falls_to_the_first_listed_model_where_its_own_is_not_listed() {
    let (accounts, _dir, app) = workbench().await;
    let repo = fresh_repo(&app, accounts.path(), "verkstead").await;

    let codex = home(accounts.path(), "codex");
    assert_eq!(
        save(
            &app,
            &codex_edit("codex", &codex, &["o4-mini", "gpt-5-codex"])
        )
        .await,
        ProfileSaved::Saved
    );

    let offered = prefill(&app, repo).await;
    assert_eq!(
        offered
            .implementation
            .as_ref()
            .and_then(|pairing| pairing.model.as_deref()),
        Some("gpt-5-codex"),
        "Codex's own model, which this Profile lists second",
    );

    let (claude_dir, config_file) = pair(accounts.path(), "work");
    assert_eq!(
        save(
            &app,
            &edit(
                "work",
                &claude_dir,
                &config_file,
                &["claude-sonnet-5", "claude-opus-5"]
            ),
        )
        .await,
        ProfileSaved::Saved
    );

    let offered = prefill(&app, repo).await;
    assert_eq!(
        offered
            .grilling
            .pairing()
            .and_then(|pairing| pairing.model.as_deref()),
        Some("claude-sonnet-5"),
        "Fable 5 is not on the list, so the first model that is",
    );
    assert_eq!(
        offered
            .implementation
            .as_ref()
            .and_then(|pairing| pairing.model.as_deref()),
        Some("claude-opus-5"),
    );
}

/// Each candidate is judged, and one that fails falls through a step for that
/// role alone: a last start's broken Profile to the platform default, and a
/// broken default to the empty picker — never on to another Profile that would
/// have passed.
#[tokio::test]
async fn an_unusable_candidate_falls_through_to_the_default_and_then_to_nothing() {
    let (accounts, dir, app) = workbench().await;
    let id = conversation(&app, accounts.path()).await;
    let fable = saved(&app, accounts.path(), "fable").await;
    let opus = saved(&app, accounts.path(), "opus").await;

    let codex = home(accounts.path(), "codex");
    assert_eq!(
        save(&app, &codex_edit("codex", &codex, &["gpt-5-codex"])).await,
        ProfileSaved::Saved
    );

    choose_grilling(&app, id, opus.id, MODEL).await;
    choose_implementation(&app, id, fable.id, MODEL).await;
    choose_review(&app, id, fable.id, MODEL).await;
    grill(dir.path(), id).await;

    let askance = fresh_repo(&app, accounts.path(), "askance").await;

    std::fs::remove_file(accounts.path().join("opus/.claude.json")).unwrap();

    let offered = prefill(&app, askance).await;
    assert_eq!(
        on(offered.grilling.pairing()),
        Some((fable.id, "claude-fable-5")),
        "the broken copy falls to the default",
    );
    assert_eq!(on(offered.implementation.as_ref()), Some((fable.id, MODEL)));

    std::fs::remove_file(accounts.path().join("fable/.claude.json")).unwrap();

    let offered = prefill(&app, askance).await;
    assert_eq!(
        offered.grilling,
        PickedView::Nothing,
        "the default is broken too, and the Codex Profile is not hunted for",
    );
    assert_eq!(offered.implementation, None);
    assert_eq!(offered.review, PickedView::Nothing);
}
