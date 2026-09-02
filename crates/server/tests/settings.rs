//! What Verkstead is told, over the viewer's namespace: reading the git author,
//! the presence of a GitHub token, how the shared Rust build cache is set,
//! whether Done shares the record to the pull request and what paths it has
//! been given, and writing any of them.
//!
//! The paths are the one thing here said in two places at once — the
//! installation's flags and the file this page writes — so what those tests ask
//! is about the labelling as much as the values: which of the two said an entry,
//! and whether the server can see what it names.
//!
//! Asked of the *server*, through the endpoints, rather than of the settings
//! files underneath them. The one thing this half has to be trusted about is
//! that a token goes in and never comes back out, and a promise like that is
//! about what crosses the wire — so what these read is the response body, whole,
//! and what they assert about the token is its absence from it.
//!
//! GitHub is a shell script here, standing where `gh` goes. What a token
//! verifies as is what that script says, which is what lets a test have a good
//! token and a bad one without an account or a network — see [`app_asking`].
//!
//! **On the platforms with a shell at `/bin/sh`.** Everything here reaches
//! GitHub through a script standing where `gh` goes, and a script is what a
//! machine with a shell can be handed. What is asserted — which token verifies,
//! what a save writes — is nothing a platform changes.
#![cfg(unix)]

use std::path::Path;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use serde::de::DeserializeOwned;
use tower::ServiceExt;
use verkstead_render::{
    ConflictResolution, IgnoreRule, PathResolution, PathSource, RuleField, SettingsSaved,
    SettingsView, Verified,
};
use verkstead_server::sandbox::SandboxConfig;
use verkstead_server::{Gh, WatchedPaths, open_database, router_asking_github, router_installed};

/// A `gh` that answers `gh api user` with the token it was run with, as the
/// account's login.
///
/// The whole of what verifying does is hand a token to a child process and read
/// what came back, so a child that answers *with* the token is the witness that
/// the right one reached it.
const SAYS_ITS_TOKEN: &str = r#"printf '{"login":"%s"}' "${GH_TOKEN-unset}""#;

/// And one that refuses everything, in the words the real `gh` refuses a bad
/// token in.
const REFUSES: &str = r#"printf 'gh: Bad credentials (HTTP 401)\n' >&2; exit 1"#;

/// And one that answers as an account of its own, whatever token it was run
/// with — for the test that reads the body looking for the token, where a stub
/// that repeated it back would be the one doing the leaking.
const SAYS_AN_ACCOUNT: &str = r#"printf '{"login":"tobico"}'"#;

/// And one that answers with headers, naming the token it was run with as the
/// scopes GitHub gave it. What a test that is about scopes rather than about
/// accounts hands a scope list where a token goes.
const SAYS_ITS_SCOPES: &str = r#"printf 'HTTP/2.0 200 OK\r\nX-Oauth-Scopes: %s\r\n\r\n{"login":"tobico"}' "${GH_TOKEN-unset}""#;

/// A server keeping its settings files in a directory of its own, reaching
/// GitHub through `gh`.
async fn app_asking(gh: &str) -> (tempfile::TempDir, Router) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    let data_dir = dir.path().to_owned();

    let gh = Gh::running(vec![
        "/bin/sh".to_owned(),
        "-c".to_owned(),
        gh.to_owned(),
        // `sh -c` gives `$0` the script's own name, so what Verkstead passes
        // lands in `$1` onwards.
        "gh".to_owned(),
    ]);

    (dir, router_asking_github(pool, data_dir, gh))
}

/// The ordinary one: a `gh` that verifies whatever it is given.
async fn app() -> (tempfile::TempDir, Router) {
    app_asking(SAYS_ITS_TOKEN).await
}

/// And a second server over a Data Directory something has already been written
/// in, which is what a restart looks like from here: the files are where the
/// settings are, and nothing is carried from one process to the next.
async fn app_over(data_dir: &Path) -> Router {
    let pool = open_database(&data_dir.join("verkstead.db")).await.unwrap();

    router_asking_github(
        pool,
        data_dir.to_owned(),
        Gh::running(vec![
            "/bin/sh".to_owned(),
            "-c".to_owned(),
            SAYS_ITS_TOKEN.to_owned(),
            "gh".to_owned(),
        ]),
    )
}

async fn settings(app: &Router) -> SettingsView {
    get(app, "/api/ui/settings").await
}

/// Save an author and leave the token alone, which is what most saves are.
async fn save_author(app: &Router, name: &str, email: &str) -> SettingsSaved {
    save(
        app,
        &serde_json::json!({
            "git_author": { "name": name, "email": email },
            "github_token": "Keep",
            "rust_build_cache": { "enabled": true, "size": "" },
            "conflict_resolution": "Merge",
            "share_on_done": false,
            "watched_paths": [],
            "sandbox_binds": [],
            "ignored_comments": "Keep",
        }),
    )
    .await
}

/// Save a token, along with whatever the author fields hold — the page has one
/// button, so the author always rides along.
async fn save_token(app: &Router, token: &str) -> SettingsSaved {
    save(
        app,
        &serde_json::json!({
            "git_author": { "name": "", "email": "" },
            "github_token": { "Set": { "token": token } },
            "rust_build_cache": { "enabled": true, "size": "" },
            "conflict_resolution": "Merge",
            "share_on_done": false,
            "watched_paths": [],
            "sandbox_binds": [],
            "ignored_comments": "Keep",
        }),
    )
    .await
}

async fn clear_token(app: &Router) -> SettingsSaved {
    save(
        app,
        &serde_json::json!({
            "git_author": { "name": "", "email": "" },
            "github_token": "Clear",
            "rust_build_cache": { "enabled": true, "size": "" },
            "conflict_resolution": "Merge",
            "share_on_done": false,
            "watched_paths": [],
            "sandbox_binds": [],
            "ignored_comments": "Keep",
        }),
    )
    .await
}

/// The raw body of a save, for the one test that reads the JSON rather than the
/// shape.
async fn save_body(app: &Router, body: &serde_json::Value) -> String {
    let (status, body) = fetch(app, posting(body)).await;

    assert_eq!(status, StatusCode::OK, "the save failed: {body}");

    body
}

async fn save(app: &Router, body: &serde_json::Value) -> SettingsSaved {
    read(&save_body(app, body).await)
}

fn posting(body: &serde_json::Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri("/api/ui/settings")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(body).unwrap()))
        .unwrap()
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

async fn fetch(app: &Router, request: Request<Body>) -> (StatusCode, String) {
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8(bytes.to_vec()).unwrap())
}

fn read<T: DeserializeOwned>(body: &str) -> T {
    serde_json::from_str(body).unwrap_or_else(|err| panic!("reading {body:?}: {err}"))
}

/// The raw body of the read, for the tests that assert about what is *not* in
/// it.
async fn settings_body(app: &Router) -> String {
    let (status, body) = fetch(
        app,
        Request::builder()
            .uri("/api/ui/settings")
            .body(Body::empty())
            .unwrap(),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "the read failed: {body}");

    body
}

#[tokio::test]
async fn a_verkstead_nobody_has_told_anything_says_so() {
    let (_dir, app) = app().await;

    let settings = settings(&app).await;

    assert_eq!(settings.git_author.name, "");
    assert_eq!(settings.git_author.email, "");
    assert_eq!(settings.github_token, None);
}

#[tokio::test]
async fn the_author_goes_in_and_comes_back() {
    let (_dir, app) = app().await;

    let saved = save_author(&app, "Tobias Cohen", "tobi@tobico.net").await;

    assert_eq!(saved.settings.git_author.name, "Tobias Cohen");
    assert_eq!(saved.settings.git_author.email, "tobi@tobico.net");
    assert_eq!(
        saved.verified, None,
        "a save that was not about a token verified nothing"
    );

    let settings = settings(&app).await;

    assert_eq!(settings.git_author.name, "Tobias Cohen");
    assert_eq!(settings.git_author.email, "tobi@tobico.net");
}

/// The promise the whole page rests on: a token that has gone in is never in a
/// response body again, whichever body it is.
#[tokio::test]
async fn the_token_appears_in_no_answer_this_endpoint_gives() {
    // A `gh` answering as an account of its own, so that the only thing that
    // could put the token in the body is the server.
    let (_dir, app) = app_asking(SAYS_AN_ACCOUNT).await;

    let saving = save_body(
        &app,
        &serde_json::json!({
            "git_author": { "name": "Tobias Cohen", "email": "tobi@tobico.net" },
            "github_token": { "Set": { "token": "ghp_averysecrettoken" } },
            "rust_build_cache": { "enabled": true, "size": "" },
            "conflict_resolution": "Merge",
            "share_on_done": false,
            "watched_paths": [],
            "sandbox_binds": [],
            "ignored_comments": "Keep",
        }),
    )
    .await;

    assert!(
        !saving.contains("ghp_averysecrettoken"),
        "the save answered with the token: {saving}"
    );

    let reading = settings_body(&app).await;

    assert!(
        !reading.contains("ghp_averysecrettoken"),
        "the read answered with the token: {reading}"
    );
}

#[tokio::test]
async fn a_saved_token_comes_back_as_its_last_four_and_when_it_was_saved() {
    let (_dir, app) = app().await;

    let saved = save_token(&app, "ghp_averysecrettoken").await;

    let token = saved.settings.github_token.expect("a token is configured");

    assert_eq!(token.last_four, "oken");
    assert!(
        token.at.starts_with("20"),
        "an RFC 3339 stamp, not {:?}",
        token.at
    );
}

#[tokio::test]
async fn a_saved_token_is_verified_and_the_account_comes_back_with_the_save() {
    let (_dir, app) = app().await;

    let saved = save_token(&app, "ghp_thetoken").await;

    assert_eq!(
        saved.verified,
        Some(Verified::Account {
            // The stub answers with the token it was run with, so this is the
            // proof that the token just saved is the one GitHub was asked about.
            login: "ghp_thetoken".to_owned(),
            // And it names no scopes, which says nothing about what the token
            // may do — see [`SAYS_ITS_SCOPES`] for the half that does.
            missing: Vec::new(),
        }),
    );
}

/// And what a token that authenticates and cannot publish comes back as: the
/// account, and the one scope to go and tick.
///
/// A settings-page answer rather than a failure found later by a human pressing
/// Share. The `gist` scope is Verkstead's own — publishing a share is its own
/// write to GitHub — and a token issued for reading repositories does not carry
/// it.
#[tokio::test]
async fn a_token_that_cannot_write_a_gist_says_which_scope_is_missing() {
    let (_dir, app) = app_asking(SAYS_ITS_SCOPES).await;

    let saved = save_token(&app, "read:org, repo, workflow").await;

    assert_eq!(
        saved.verified,
        Some(Verified::Account {
            login: "tobico".to_owned(),
            missing: vec!["gist".to_owned()],
        }),
    );
}

/// And one that does carry it comes back with nothing to do.
#[tokio::test]
async fn a_token_that_can_write_a_gist_is_missing_nothing() {
    let (_dir, app) = app_asking(SAYS_ITS_SCOPES).await;

    let saved = save_token(&app, "repo, gist").await;

    assert_eq!(
        saved.verified,
        Some(Verified::Account {
            login: "tobico".to_owned(),
            missing: Vec::new(),
        }),
    );
}

/// The one that costs the human a trip back to GitHub if it goes wrong: a token
/// that would not verify is still written down, and what went wrong is said in
/// words beside it.
#[tokio::test]
async fn a_token_github_refuses_is_saved_anyway_and_the_refusal_is_in_words() {
    let (dir, app) = app_asking(REFUSES).await;

    let saved = save_token(&app, "ghp_thetoken").await;

    assert_eq!(
        saved.verified,
        Some(Verified::Refused {
            why: "`gh` said: gh: Bad credentials (HTTP 401)".to_owned(),
        }),
    );

    assert_eq!(
        saved
            .settings
            .github_token
            .expect("the token was saved regardless")
            .last_four,
        "oken",
    );

    assert!(
        std::fs::read_to_string(dir.path().join("secrets.yaml"))
            .unwrap()
            .contains("ghp_thetoken"),
        "the file holds the token that would not verify",
    );
}

#[tokio::test]
async fn clearing_takes_the_token_away_and_the_read_says_so() {
    let (_dir, app) = app().await;

    save_token(&app, "ghp_thetoken").await;
    assert!(settings(&app).await.github_token.is_some());

    let cleared = clear_token(&app).await;

    assert_eq!(cleared.settings.github_token, None);
    assert_eq!(cleared.verified, None, "clearing asks GitHub about nothing");
    assert_eq!(settings(&app).await.github_token, None);
}

/// Saving the author must not take the credentials away, which is why the
/// token's half of a save is an action rather than a value.
#[tokio::test]
async fn saving_the_author_leaves_the_token_where_it_was() {
    let (_dir, app) = app().await;

    save_token(&app, "ghp_thetoken").await;
    let saved = save_author(&app, "Tobias Cohen", "tobi@tobico.net").await;

    assert_eq!(
        saved
            .settings
            .github_token
            .expect("the token is still configured")
            .last_four,
        "oken",
    );
    assert_eq!(saved.verified, None);
}

#[tokio::test]
async fn the_secrets_file_lands_readable_by_nobody_else() {
    use std::os::unix::fs::PermissionsExt;

    let (dir, app) = app().await;

    save_token(&app, "ghp_thetoken").await;

    let mode = std::fs::metadata(dir.path().join("secrets.yaml"))
        .unwrap()
        .permissions()
        .mode();

    assert_eq!(mode & 0o777, 0o600);
}

/// The files are the source of truth, so the endpoint reads them rather than
/// remembering what it last wrote.
#[tokio::test]
async fn a_hand_edit_to_either_file_is_what_the_next_read_says() {
    let (dir, app) = app().await;

    save_token(&app, "ghp_thefirsttoken").await;
    save_author(&app, "Tobias Cohen", "tobi@tobico.net").await;

    hand_edit(dir.path(), "secrets.yaml", "github_token: by-hand-abcd\n");
    hand_edit(
        dir.path(),
        "config.yaml",
        "git_author:\n  name: By Hand\n  email: hand@tobico.net\n",
    );

    let settings = settings(&app).await;

    assert_eq!(
        settings
            .github_token
            .expect("the hand-edited token is configured")
            .last_four,
        "abcd",
    );
    assert_eq!(settings.git_author.name, "By Hand");
    assert_eq!(settings.git_author.email, "hand@tobico.net");
}

/// The paths are values now, so a save carrying them as they stand is what
/// leaves them alone — which is what the page does with every field the form in
/// front of the human is not about.
#[tokio::test]
async fn a_save_carrying_the_paths_as_they_stand_leaves_them() {
    let (dir, app) = app().await;

    hand_edit(
        dir.path(),
        "config.yaml",
        "sandbox_binds:\n  - /var/cache/verkstead-node\nwatched_paths:\n  - /home/ada/src\n",
    );

    save(
        &app,
        &serde_json::json!({
            "git_author": { "name": "Tobias Cohen", "email": "tobi@tobico.net" },
            "github_token": "Keep",
            "rust_build_cache": { "enabled": true, "size": "" },
            "conflict_resolution": "Merge",
            "share_on_done": false,
            "watched_paths": ["/home/ada/src"],
            "sandbox_binds": ["/var/cache/verkstead-node"],
            "ignored_comments": "Keep",
        }),
    )
    .await;

    let written = std::fs::read_to_string(dir.path().join("config.yaml")).unwrap();

    assert!(
        written.contains("/var/cache/verkstead-node") && written.contains("/home/ada/src"),
        "the save should have carried both lists through, got:\n{written}"
    );
}

fn hand_edit(data_dir: &Path, name: &str, text: &str) {
    std::fs::write(data_dir.join(name), text).unwrap();
}

/// A token typed with the whitespace that came with it out of GitHub's own page
/// is the token, and one that is *only* whitespace is nothing at all.
#[tokio::test]
async fn a_token_that_is_nothing_but_whitespace_configures_nothing() {
    let (_dir, app) = app().await;

    let saved = save_token(&app, "   \n").await;

    assert_eq!(saved.settings.github_token, None);
    assert_eq!(
        saved.verified, None,
        "there is no token to ask GitHub about"
    );
}

/// The build cache the human has said nothing about: on, at the default size,
/// with the size marked as nobody's choice so the page can draw it as a
/// placeholder.
///
/// The whole point of the shape — a fresh install should not be the one paying
/// for every dependency to be compiled twice, and nothing here asks the human to
/// find a setting first.
#[tokio::test]
async fn a_build_cache_nobody_has_configured_is_on_at_the_default_size() {
    let (_dir, app) = app().await;

    let cache = settings(&app).await.rust_build_cache;

    assert!(cache.enabled, "on is what an untouched setting means");
    assert_eq!(cache.size, "30G");
    assert!(
        !cache.size_configured,
        "the default is shown rather than chosen"
    );
    assert!(
        !cache.compiles_cached,
        "this router runs no sessions, so it has no sccache to hand any"
    );
}

/// And what a save of it says: both halves come back off the file, and the
/// switch is what the next session is built against.
#[tokio::test]
async fn the_build_cache_switch_and_size_go_in_and_come_back() {
    let (dir, app) = app().await;

    let saved = save(
        &app,
        &serde_json::json!({
            "git_author": { "name": "", "email": "" },
            "github_token": "Keep",
            "rust_build_cache": { "enabled": false, "size": "5G" },
            "conflict_resolution": "Merge",
            "share_on_done": false,
            "watched_paths": [],
            "sandbox_binds": [],
            "ignored_comments": "Keep",
        }),
    )
    .await;

    assert!(!saved.settings.rust_build_cache.enabled);
    assert_eq!(saved.settings.rust_build_cache.size, "5G");
    assert!(saved.settings.rust_build_cache.size_configured);

    // In the file the next session reads, rather than only in the answer.
    let written = std::fs::read_to_string(dir.path().join("config.yaml")).unwrap();
    assert!(
        written.contains("enabled: false") && written.contains("5G"),
        "the switch and the size are in config.yaml: {written}"
    );

    let read_back = settings(&app).await.rust_build_cache;
    assert!(!read_back.enabled);
    assert_eq!(read_back.size, "5G");
}

/// How a conflict is resolved where nobody has said: a merge, which is the half
/// of the choice that rewrites nothing.
///
/// The whole point of the shape, as it is for the build cache above: a human who
/// has never found this section should not have a branch force-pushed under
/// whoever was reading it.
#[tokio::test]
async fn a_conflict_nobody_has_configured_is_merged() {
    let (_dir, app) = app().await;

    assert_eq!(
        settings(&app).await.conflict_resolution,
        ConflictResolution::Merge
    );
}

/// And what a save of it says: the word goes into the file the next resolution
/// session is dispatched out of, and comes back off it.
#[tokio::test]
async fn how_a_conflict_is_resolved_goes_in_and_comes_back() {
    let (dir, app) = app().await;

    let saved = save(
        &app,
        &serde_json::json!({
            "git_author": { "name": "", "email": "" },
            "github_token": "Keep",
            "rust_build_cache": { "enabled": true, "size": "" },
            "conflict_resolution": "Rebase",
            "share_on_done": false,
            "watched_paths": [],
            "sandbox_binds": [],
            "ignored_comments": "Keep",
        }),
    )
    .await;

    assert_eq!(
        saved.settings.conflict_resolution,
        ConflictResolution::Rebase
    );

    // In the file rather than only in the answer, and as the word a human
    // hand-editing it would write.
    let written = std::fs::read_to_string(dir.path().join("config.yaml")).unwrap();
    assert!(
        written.contains("conflict_resolution: rebase"),
        "the strategy is in config.yaml, in the file's own words: {written}"
    );

    assert_eq!(
        settings(&app).await.conflict_resolution,
        ConflictResolution::Rebase
    );

    // And back again, because a setting that could only be turned on would be a
    // setting nobody could undo from a phone.
    save(
        &app,
        &serde_json::json!({
            "git_author": { "name": "", "email": "" },
            "github_token": "Keep",
            "rust_build_cache": { "enabled": true, "size": "" },
            "conflict_resolution": "Merge",
            "share_on_done": false,
            "watched_paths": [],
            "sandbox_binds": [],
            "ignored_comments": "Keep",
        }),
    )
    .await;

    assert_eq!(
        settings(&app).await.conflict_resolution,
        ConflictResolution::Merge
    );
}

/// Whether Done shares the record to the pull request where nobody has said:
/// off, which is the other way about from the two settings above it.
///
/// The point of that shape: what this switch turns on publishes a gist under
/// the human's own account and comments on a pull request other people read, so
/// a Verkstead nobody has been to the settings page of does neither.
#[tokio::test]
async fn sharing_on_done_nobody_has_configured_is_off() {
    let (_dir, app) = app().await;

    assert!(!settings(&app).await.share_on_done);
}

/// And what a save of it says: the switch goes into the file the wrap-up reads,
/// and comes back off it — including from a router started afresh on the same
/// directory, which is what a restart is.
#[tokio::test]
async fn sharing_on_done_goes_in_and_comes_back() {
    let (dir, app) = app().await;

    let saved = save(
        &app,
        &serde_json::json!({
            "git_author": { "name": "", "email": "" },
            "github_token": "Keep",
            "rust_build_cache": { "enabled": true, "size": "" },
            "conflict_resolution": "Merge",
            "share_on_done": true,
            "watched_paths": [],
            "sandbox_binds": [],
            "ignored_comments": "Keep",
        }),
    )
    .await;

    assert!(saved.settings.share_on_done);

    // In the file rather than only in the answer, and in the words a human
    // hand-editing it would write.
    let written = std::fs::read_to_string(dir.path().join("config.yaml")).unwrap();
    assert!(
        written.contains("share_on_done: true"),
        "the switch is in config.yaml: {written}"
    );

    assert!(settings(&app).await.share_on_done);

    // And to a server that has just come up on the same Data Directory, which
    // is the whole of what surviving a restart means here.
    let restarted = restarted(dir.path()).await;
    assert!(settings(&restarted).await.share_on_done);
}

/// A save from another section carries the switch as it stands, and that is
/// what leaves it alone — the same contract the paths above are saved under,
/// because one request writes the whole of `config.yaml`.
#[tokio::test]
async fn a_save_carrying_the_switch_as_it_stands_leaves_it() {
    let (_dir, app) = app().await;

    save(
        &app,
        &serde_json::json!({
            "git_author": { "name": "", "email": "" },
            "github_token": "Keep",
            "rust_build_cache": { "enabled": true, "size": "" },
            "conflict_resolution": "Merge",
            "share_on_done": true,
            "watched_paths": [],
            "sandbox_binds": [],
            "ignored_comments": "Keep",
        }),
    )
    .await;

    // The build cache section's own save, which is about the size and carries
    // everything else as the page read it.
    let saved = save(
        &app,
        &serde_json::json!({
            "git_author": { "name": "", "email": "" },
            "github_token": "Keep",
            "rust_build_cache": { "enabled": true, "size": "5G" },
            "conflict_resolution": "Merge",
            "share_on_done": true,
            "watched_paths": [],
            "sandbox_binds": [],
            "ignored_comments": "Keep",
        }),
    )
    .await;

    assert_eq!(saved.settings.rust_build_cache.size, "5G");
    assert!(saved.settings.share_on_done, "the switch stands");
}

/// A second server on the same Data Directory, which is what a restart looks
/// like from here: the files are the whole of what is kept, so a router built
/// afresh over them is the next boot reading them.
async fn restarted(data_dir: &Path) -> Router {
    let pool = open_database(&data_dir.join("verkstead.db")).await.unwrap();

    router_asking_github(
        pool,
        data_dir.to_owned(),
        Gh::running(vec![
            "/bin/sh".to_owned(),
            "-c".to_owned(),
            SAYS_ITS_TOKEN.to_owned(),
            "gh".to_owned(),
        ]),
    )
}

/// Clearing the size field is asking for the default back rather than asking
/// for a cache of no size at all.
#[tokio::test]
async fn a_size_cleared_is_the_default_again_and_not_a_size_of_nothing() {
    let (_dir, app) = app().await;

    save(
        &app,
        &serde_json::json!({
            "git_author": { "name": "", "email": "" },
            "github_token": "Keep",
            "rust_build_cache": { "enabled": true, "size": "5G" },
            "conflict_resolution": "Merge",
            "share_on_done": false,
            "watched_paths": [],
            "sandbox_binds": [],
            "ignored_comments": "Keep",
        }),
    )
    .await;

    let saved = save(
        &app,
        &serde_json::json!({
            "git_author": { "name": "", "email": "" },
            "github_token": "Keep",
            "rust_build_cache": { "enabled": true, "size": "  " },
            "conflict_resolution": "Merge",
            "share_on_done": false,
            "watched_paths": [],
            "sandbox_binds": [],
            "ignored_comments": "Keep",
        }),
    )
    .await;

    assert_eq!(saved.settings.rust_build_cache.size, "30G");
    assert!(!saved.settings.rust_build_cache.size_configured);
}

/// The Paths half of the page: every Watched Path and every Sandbox
/// Configuration bind, from both of the places either of them is said.
///
/// A server the installation configured as well as a file, because the whole of
/// what this reports is which of the two said an entry and whether the server
/// can see it — and a router that was only ever told things through the page
/// could not be asked the first of those.
async fn app_installed(watched: &[&Path], binds: &[String]) -> (tempfile::TempDir, Router) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    let paths: Vec<_> = watched.iter().map(|path| path.to_path_buf()).collect();

    let app = router_installed(
        pool,
        WatchedPaths::resolve(&paths).unwrap(),
        SandboxConfig::resolve(binds).unwrap(),
        dir.path().to_owned(),
        Gh::running(vec![
            "/bin/sh".to_owned(),
            "-c".to_owned(),
            SAYS_ITS_TOKEN.to_owned(),
            "gh".to_owned(),
        ]),
    );

    (dir, app)
}

/// Save the two lists and leave the rest of both files alone, which is what the
/// Paths pane's own press sends.
async fn save_paths(app: &Router, watched: &[&str], binds: &[&str]) -> SettingsSaved {
    save(
        app,
        &serde_json::json!({
            "git_author": { "name": "", "email": "" },
            "github_token": "Keep",
            "rust_build_cache": { "enabled": true, "size": "" },
            "conflict_resolution": "Merge",
            "share_on_done": false,
            "watched_paths": watched,
            "sandbox_binds": binds,
            "ignored_comments": "Keep",
        }),
    )
    .await
}

/// A directory the server can see, made inside `dir`.
fn made(dir: &Path, name: &str) -> std::path::PathBuf {
    let path = dir.join(name);
    std::fs::create_dir(&path).unwrap();

    path
}

fn why(resolution: &PathResolution) -> String {
    match resolution {
        PathResolution::Resolves => panic!("it resolved"),
        PathResolution::Unresolved { why } => why.clone(),
    }
}

/// A standalone install, which is the state the whole of this feature is for:
/// nothing said at the installation and nothing in the file either.
#[tokio::test]
async fn a_verkstead_configured_by_nobody_has_no_paths_at_all() {
    let (_dir, app) = app().await;

    let paths = settings(&app).await.paths;

    assert!(paths.watched.is_empty(), "{:?}", paths.watched);
    assert!(paths.binds.is_empty(), "{:?}", paths.binds);
}

/// What the unit said comes back labelled as the unit's, so the page can draw it
/// and refuse to let anybody edit it here.
#[tokio::test]
async fn the_installations_own_paths_come_back_as_the_installations() {
    let root = tempfile::tempdir().unwrap();
    let watched = made(root.path(), "src");
    let cache = made(root.path(), "node-cache");
    let own = made(root.path(), "askance-cargo");

    let (_dir, app) = app_installed(
        &[&watched],
        &[
            cache.display().to_string(),
            format!("askance={}", own.display()),
        ],
    )
    .await;

    let paths = settings(&app).await.paths;

    let [watched_path] = &paths.watched[..] else {
        panic!("one watched path, not {:?}", paths.watched);
    };

    assert_eq!(
        watched_path.path,
        watched.canonicalize().unwrap().display().to_string()
    );
    assert_eq!(watched_path.source, PathSource::Installation);
    assert_eq!(watched_path.resolution, PathResolution::Resolves);

    let [global, per_repo] = &paths.binds[..] else {
        panic!("two binds, not {:?}", paths.binds);
    };

    assert_eq!(global.path, cache.display().to_string());
    assert_eq!(
        global.repo, None,
        "a bind every sandbox gets is nobody's own"
    );
    assert_eq!(global.source, PathSource::Installation);
    assert_eq!(global.resolution, PathResolution::Resolves);

    // And the other half of a bind's shape: which Repo it is scoped to.
    assert_eq!(per_repo.path, own.display().to_string());
    assert_eq!(per_repo.repo.as_deref(), Some("askance"));
    assert_eq!(per_repo.source, PathSource::Installation);
}

/// And what the page saved comes back as the settings' own, beside it: the two
/// sources are one list, and the label is the whole of what tells them apart.
#[tokio::test]
async fn the_two_sources_come_back_as_one_list_saying_which_is_which() {
    let root = tempfile::tempdir().unwrap();
    let installed = made(root.path(), "src");
    let bound = made(root.path(), "node-cache");
    let added = made(root.path(), "elsewhere");
    let cargo = made(root.path(), "cargo");

    let (_dir, app) = app_installed(&[&installed], &[bound.display().to_string()]).await;

    let saved = save_paths(
        &app,
        &[&added.display().to_string()],
        &[
            &cargo.display().to_string(),
            &format!("verkstead={}", cargo.display()),
        ],
    )
    .await;

    let sources: Vec<_> = saved
        .settings
        .paths
        .watched
        .iter()
        .map(|entry| (entry.path.clone(), entry.source.clone()))
        .collect();

    assert_eq!(
        sources,
        vec![
            (
                installed.canonicalize().unwrap().display().to_string(),
                PathSource::Installation
            ),
            (added.display().to_string(), PathSource::Settings),
        ],
        "the installation's own first, then what the page saved"
    );

    let binds: Vec<_> = saved
        .settings
        .paths
        .binds
        .iter()
        .map(|entry| (entry.path.clone(), entry.repo.clone(), entry.source.clone()))
        .collect();

    assert_eq!(
        binds,
        vec![
            (bound.display().to_string(), None, PathSource::Installation),
            (cargo.display().to_string(), None, PathSource::Settings),
            (
                cargo.display().to_string(),
                Some("verkstead".to_owned()),
                PathSource::Settings
            ),
        ]
    );

    // And the read that follows says the same, because the save's answer is a
    // read: what was written is in the file the next session will look at.
    assert_eq!(settings(&app).await.paths, saved.settings.paths);
}

/// The rule the whole settings side is on: a save lands whatever it was told,
/// and what the server cannot see is a report rather than a refusal.
#[tokio::test]
async fn a_path_the_server_cannot_see_is_saved_anyway_and_said_so() {
    let root = tempfile::tempdir().unwrap();
    let never_made = root.path().join("never-made");
    let file = root.path().join("notes.md");
    std::fs::write(&file, "not a directory\n").unwrap();

    let (dir, app) = app().await;

    let saved = save_paths(
        &app,
        &[
            &never_made.display().to_string(),
            &file.display().to_string(),
            "src",
        ],
        &[&never_made.display().to_string()],
    )
    .await;

    // In the file, whatever the server makes of any of it — this is the half a
    // nix install depends on, where a path the hardened unit cannot see is saved
    // now and works when the installer widens the namespace.
    let written = std::fs::read_to_string(dir.path().join("config.yaml")).unwrap();
    assert!(written.contains("never-made"), "{written}");

    let [missing, not_a_directory, relative] = &saved.settings.paths.watched[..] else {
        panic!(
            "three watched paths, not {:?}",
            saved.settings.paths.watched
        );
    };

    assert!(
        why(&missing.resolution).contains("cannot see it"),
        "{missing:?}"
    );
    assert!(
        why(&not_a_directory.resolution).contains("not a directory"),
        "{not_a_directory:?}"
    );
    assert!(
        why(&relative.resolution).contains("relative"),
        "{relative:?}"
    );

    let [bind] = &saved.settings.paths.binds[..] else {
        panic!("one bind, not {:?}", saved.settings.paths.binds);
    };

    assert!(
        why(&bind.resolution).contains("cannot see it"),
        "a bind the server cannot see says so: {bind:?}"
    );
}

/// An entry nothing can be read out of is a row like any other. It has to be:
/// a typo that vanished from the page would be a typo nobody could correct.
#[tokio::test]
async fn a_bind_that_will_not_read_is_still_a_row() {
    let (_dir, app) = app().await;

    let saved = save_paths(&app, &[], &["node-cache"]).await;

    let [bind] = &saved.settings.paths.binds[..] else {
        panic!("one bind, not {:?}", saved.settings.paths.binds);
    };

    assert_eq!(bind.path, "node-cache", "the entry as it was written");
    assert_eq!(bind.repo, None);
    assert_eq!(bind.source, PathSource::Settings);
    assert!(
        why(&bind.resolution).contains("neither an absolute path"),
        "{bind:?}"
    );
}

/// A save says what the settings hold afterwards, and says nothing at all about
/// what the installation said: those are the unit's word, and this page has no
/// way to reach them.
#[tokio::test]
async fn a_save_replaces_the_settings_paths_and_leaves_the_installations() {
    let root = tempfile::tempdir().unwrap();
    let installed = made(root.path(), "src");
    let bound = made(root.path(), "node-cache");

    let (dir, app) = app_installed(&[&installed], &[bound.display().to_string()]).await;

    save_paths(&app, &["/home/ada/first"], &["/var/cache/first"]).await;
    let saved = save_paths(&app, &["/home/ada/second"], &[]).await;

    let written = std::fs::read_to_string(dir.path().join("config.yaml")).unwrap();

    assert!(written.contains("/home/ada/second"), "{written}");
    assert!(
        !written.contains("/home/ada/first"),
        "the first save's list is gone: {written}"
    );
    assert!(
        !written.contains("/var/cache/first"),
        "and so is its bind: {written}"
    );
    assert!(
        !written.contains(&installed.display().to_string()),
        "the installation's own was never in this file: {written}"
    );

    // And it is still the boundary, because nothing here could have touched it.
    let watched: Vec<_> = saved
        .settings
        .paths
        .watched
        .iter()
        .map(|entry| entry.source.clone())
        .collect();

    assert_eq!(
        watched,
        vec![PathSource::Installation, PathSource::Settings]
    );
    assert_eq!(
        saved
            .settings
            .paths
            .binds
            .iter()
            .map(|entry| entry.source.clone())
            .collect::<Vec<_>>(),
        vec![PathSource::Installation],
        "the installation's bind stands and the settings' is gone"
    );
}

/// Save a list of ignore rules and leave the rest of both files alone, which is
/// what the rules section's own press sends.
async fn save_rules(app: &Router, rules: serde_json::Value) -> SettingsSaved {
    save(
        app,
        &serde_json::json!({
            "git_author": { "name": "", "email": "" },
            "github_token": "Keep",
            "rust_build_cache": { "enabled": true, "size": "" },
            "conflict_resolution": "Merge",
            "share_on_done": false,
            "watched_paths": [],
            "sandbox_binds": [],
            "ignored_comments": { "Set": { "rules": rules } },
        }),
    )
    .await
}

fn rule(author: &str, body: &str) -> serde_json::Value {
    serde_json::json!({ "author": author, "body": body })
}

#[tokio::test]
async fn a_verkstead_nobody_has_told_to_ignore_anything_ignores_nothing() {
    let (_dir, app) = app().await;

    assert!(settings(&app).await.ignored_comments.is_empty());
}

#[tokio::test]
async fn the_ignore_rules_go_in_and_come_back() {
    let (_dir, app) = app().await;

    let saved = save_rules(
        &app,
        serde_json::json!([rule("coderabbitai", "billing"), rule("", "^nit:")]),
    )
    .await;

    assert!(saved.refused.is_empty(), "{:?}", saved.refused);
    assert_eq!(
        saved.settings.ignored_comments,
        vec![
            IgnoreRule {
                author: "coderabbitai".to_owned(),
                body: "billing".to_owned(),
            },
            IgnoreRule {
                author: String::new(),
                body: "^nit:".to_owned(),
            },
        ]
    );

    // And a read of its own says the same, which is the half that survives a
    // restart: the file is where they are, and nothing is held in the process.
    assert_eq!(
        settings(&app).await.ignored_comments,
        saved.settings.ignored_comments
    );
}

/// The file rather than the process, said as plainly as a test can say it: a
/// second server over the same Data Directory reads what the first one wrote.
#[tokio::test]
async fn the_rules_are_in_the_config_file_and_outlive_the_server() {
    let (dir, app) = app().await;

    save_rules(&app, serde_json::json!([rule("coderabbitai", "billing")])).await;

    let written = std::fs::read_to_string(dir.path().join("config.yaml")).unwrap();

    assert!(written.contains("ignored_comments"), "{written}");
    assert!(written.contains("coderabbitai"), "{written}");

    let restarted = app_over(dir.path()).await;

    assert_eq!(
        settings(&restarted).await.ignored_comments,
        vec![IgnoreRule {
            author: "coderabbitai".to_owned(),
            body: "billing".to_owned(),
        }]
    );
}

/// The one refusal either settings file has, and it turns the whole save down:
/// a pattern nothing can compile is a rule that would silence nothing while
/// reading as though it silenced something.
#[tokio::test]
async fn a_pattern_that_will_not_compile_is_refused_at_its_own_row() {
    let (_dir, app) = app().await;

    save_rules(&app, serde_json::json!([rule("coderabbitai", "billing")])).await;

    let saved = save_rules(
        &app,
        serde_json::json!([rule("dependabot", ""), rule("", "[oh")]),
    )
    .await;

    assert_eq!(saved.refused.len(), 1, "{:?}", saved.refused);
    assert_eq!(saved.refused[0].rule, 1);
    assert_eq!(saved.refused[0].field, Some(RuleField::Body));
    assert!(!saved.refused[0].why.is_empty());

    // And nothing was written: what was there is what is there.
    assert_eq!(
        saved.settings.ignored_comments,
        vec![IgnoreRule {
            author: "coderabbitai".to_owned(),
            body: "billing".to_owned(),
        }]
    );
    assert_eq!(
        settings(&app).await.ignored_comments,
        saved.settings.ignored_comments
    );
}

/// The other way a rule is refused, and the one that has no box to draw the
/// error at: a rule constraining nothing matches every comment there is.
#[tokio::test]
async fn a_rule_with_both_fields_empty_is_refused_as_a_whole() {
    let (_dir, app) = app().await;

    let saved = save_rules(&app, serde_json::json!([rule("", "")])).await;

    assert_eq!(saved.refused.len(), 1, "{:?}", saved.refused);
    assert_eq!(saved.refused[0].rule, 0);
    assert_eq!(saved.refused[0].field, None);
    assert!(saved.settings.ignored_comments.is_empty());
}

/// Every row at fault rather than the first, because the page draws the error
/// at the row and a human who mistyped two should be told about two.
#[tokio::test]
async fn every_refused_row_is_named() {
    let (_dir, app) = app().await;

    let saved = save_rules(
        &app,
        serde_json::json!([rule("[oh", ""), rule("ada", "fine"), rule("", "(")]),
    )
    .await;

    assert_eq!(
        saved
            .refused
            .iter()
            .map(|refused| (refused.rule, refused.field))
            .collect::<Vec<_>>(),
        vec![(0, Some(RuleField::Author)), (2, Some(RuleField::Body))]
    );
}

/// A refusal is the whole request refused, and not the author written while the
/// rules were turned away.
#[tokio::test]
async fn a_refused_save_writes_nothing_at_all() {
    let (_dir, app) = app().await;

    save_author(&app, "Ada Lovelace", "ada@example.com").await;

    let saved = save(
        &app,
        &serde_json::json!({
            "git_author": { "name": "Tobias Cohen", "email": "tobi@tobico.net" },
            "github_token": { "Set": { "token": "ghp_thetoken" } },
            "rust_build_cache": { "enabled": true, "size": "" },
            "conflict_resolution": "Merge",
            "share_on_done": false,
            "watched_paths": [],
            "sandbox_binds": [],
            "ignored_comments": { "Set": { "rules": [rule("", "[oh")] } },
        }),
    )
    .await;

    assert_eq!(saved.refused.len(), 1);
    assert!(saved.verified.is_none(), "no token was tried");
    assert_eq!(saved.settings.git_author.name, "Ada Lovelace");
    assert!(saved.settings.github_token.is_none());
    assert_eq!(settings(&app).await.git_author.name, "Ada Lovelace");
}

/// What every section but the rules' own sends, and what makes those saves ones
/// that cannot be refused.
#[tokio::test]
async fn a_save_from_another_section_leaves_the_rules_where_they_are() {
    let (_dir, app) = app().await;

    save_rules(&app, serde_json::json!([rule("coderabbitai", "billing")])).await;

    let saved = save_author(&app, "Ada Lovelace", "ada@example.com").await;

    assert!(saved.refused.is_empty());
    assert_eq!(
        saved.settings.ignored_comments,
        vec![IgnoreRule {
            author: "coderabbitai".to_owned(),
            body: "billing".to_owned(),
        }]
    );
}

/// A rule somebody hand-edited badly is the case this whole arrangement is for:
/// it comes back on the read so the human can correct it, and it refuses
/// nothing until they save the section it is on.
#[tokio::test]
async fn a_hand_edited_bad_pattern_reads_back_and_refuses_no_other_save() {
    let (dir, app) = app().await;

    std::fs::write(
        dir.path().join("config.yaml"),
        "ignored_comments:\n  - body: '[oh'\n",
    )
    .unwrap();

    assert_eq!(
        settings(&app).await.ignored_comments,
        vec![IgnoreRule {
            author: String::new(),
            body: "[oh".to_owned(),
        }]
    );

    let saved = save_author(&app, "Ada Lovelace", "ada@example.com").await;

    assert!(saved.refused.is_empty(), "{:?}", saved.refused);
    assert_eq!(saved.settings.git_author.name, "Ada Lovelace");
    assert_eq!(
        saved.settings.ignored_comments,
        vec![IgnoreRule {
            author: String::new(),
            body: "[oh".to_owned(),
        }]
    );
}

/// And the hand-edit the reading half does drop, because it is the one that
/// fails the other way: a rule constraining nothing would silence every comment
/// on every pull request.
#[tokio::test]
async fn a_hand_edited_rule_that_constrains_nothing_is_not_read_back() {
    let (dir, app) = app().await;

    std::fs::write(
        dir.path().join("config.yaml"),
        "ignored_comments:\n  - author: ''\n    body: ''\n  - author: dependabot\n",
    )
    .unwrap();

    assert_eq!(
        settings(&app).await.ignored_comments,
        vec![IgnoreRule {
            author: "dependabot".to_owned(),
            body: String::new(),
        }]
    );
}

/// The last rule taken off the page is the list emptied, which is a save that
/// sends none rather than a save that says nothing.
#[tokio::test]
async fn sending_no_rules_takes_the_ones_that_were_there_away() {
    let (_dir, app) = app().await;

    save_rules(&app, serde_json::json!([rule("coderabbitai", "billing")])).await;

    let saved = save_rules(&app, serde_json::json!([])).await;

    assert!(saved.settings.ignored_comments.is_empty());
    assert!(settings(&app).await.ignored_comments.is_empty());
}
