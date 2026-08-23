//! What Verkstead is told, over the viewer's namespace: reading the git author
//! and the presence of a GitHub token, and writing either.
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

use std::path::Path;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use serde::de::DeserializeOwned;
use tower::ServiceExt;
use verkstead_render::{SettingsSaved, SettingsView, Verified};
use verkstead_server::{Gh, open_database, router_asking_github};

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
