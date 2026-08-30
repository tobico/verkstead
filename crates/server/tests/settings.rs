//! What Verkstead is told, over the viewer's namespace: reading the git author,
//! the presence of a GitHub token, how the shared Rust build cache is set and
//! where the human hosts a share viewer of their own, and writing any of them.
//! The viewer page itself is handed over from here as well, that being the
//! other half of the setting recording where it went.
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
use verkstead_render::{Resolution, SettingsSaved, SettingsView, Verified};
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
            "share_viewer_url": "",
            "conflict_resolution": "Merge",
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
            "share_viewer_url": "",
            "conflict_resolution": "Merge",
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
            "share_viewer_url": "",
            "conflict_resolution": "Merge",
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
            "share_viewer_url": "",
            "conflict_resolution": "Merge",
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
            "share_viewer_url": "",
            "conflict_resolution": "Merge",
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

    assert_eq!(settings(&app).await.conflict_resolution, Resolution::Merge);
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
            "share_viewer_url": "",
            "conflict_resolution": "Rebase",
        }),
    )
    .await;

    assert_eq!(saved.settings.conflict_resolution, Resolution::Rebase);

    // In the file rather than only in the answer, and as the word a human
    // hand-editing it would write.
    let written = std::fs::read_to_string(dir.path().join("config.yaml")).unwrap();
    assert!(
        written.contains("conflict_resolution: rebase"),
        "the strategy is in config.yaml, in the file's own words: {written}"
    );

    assert_eq!(settings(&app).await.conflict_resolution, Resolution::Rebase);

    // And back again, because a setting that could only be turned on would be a
    // setting nobody could undo from a phone.
    save(
        &app,
        &serde_json::json!({
            "git_author": { "name": "", "email": "" },
            "github_token": "Keep",
            "rust_build_cache": { "enabled": true, "size": "" },
            "share_viewer_url": "",
            "conflict_resolution": "Merge",
        }),
    )
    .await;

    assert_eq!(settings(&app).await.conflict_resolution, Resolution::Merge);
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
            "share_viewer_url": "",
            "conflict_resolution": "Merge",
        }),
    )
    .await;

    let saved = save(
        &app,
        &serde_json::json!({
            "git_author": { "name": "", "email": "" },
            "github_token": "Keep",
            "rust_build_cache": { "enabled": true, "size": "  " },
            "share_viewer_url": "",
            "conflict_resolution": "Merge",
        }),
    )
    .await;

    assert_eq!(saved.settings.rust_build_cache.size, "30G");
    assert!(!saved.settings.rust_build_cache.size_configured);
}

/// Where the human hosts a share viewer of their own, which is the plainest
/// setting on the page: written as it was typed and read back as itself.
///
/// It is not a secret — it is a public page, and its URL goes into a comment on
/// a pull request the moment a share is published through it — so unlike the
/// token there is nothing here that must not come back out.
#[tokio::test]
async fn where_the_share_viewer_is_hosted_goes_in_and_comes_back() {
    let (dir, app) = app().await;

    let saved = save_viewer(&app, "https://ada.github.io/verkstead-shares/").await;

    assert_eq!(
        saved.settings.share_viewer_url,
        "https://ada.github.io/verkstead-shares/"
    );

    // In the file rather than only in the answer: what a link is composed
    // through is the file, read at the moment the link is drawn.
    let written = std::fs::read_to_string(dir.path().join("config.yaml")).unwrap();
    assert!(
        written.contains("https://ada.github.io/verkstead-shares/"),
        "the URL is in config.yaml: {written}"
    );

    assert_eq!(
        settings(&app).await.share_viewer_url,
        "https://ada.github.io/verkstead-shares/"
    );
}

/// A Verkstead nobody has hosted one for says so as an empty field rather than
/// as a guess: the *setting* has no default, because nobody but the human knows
/// where their own site is, and a field filled in with an address nobody typed
/// is a setting they cannot tell they have not chosen.
///
/// What an empty one *means* is another matter, and not this page's: links are
/// composed through the copy Verkstead hosts — `HOSTED` in
/// `crates/server/src/sharing.rs`, and `tests/sharing.rs` is where that is
/// asked about.
#[tokio::test]
async fn a_share_viewer_nobody_has_hosted_comes_back_empty() {
    let (_dir, app) = app().await;

    assert_eq!(settings(&app).await.share_viewer_url, "");
}

/// And clearing the field takes it away, which is what an empty one means on the
/// way in as well as on the way out.
#[tokio::test]
async fn clearing_the_share_viewer_url_takes_it_away() {
    let (_dir, app) = app().await;

    save_viewer(&app, "https://ada.github.io/verkstead-shares/").await;
    let cleared = save_viewer(&app, "  ").await;

    assert_eq!(cleared.settings.share_viewer_url, "");
    assert_eq!(settings(&app).await.share_viewer_url, "");
}

/// Saving where the viewer is hosted must not disturb the credentials, for the
/// reason saving the author must not: the page has one button and the server
/// writes both files.
#[tokio::test]
async fn saving_the_share_viewer_url_leaves_the_token_where_it_was() {
    let (_dir, app) = app().await;

    save_token(&app, "ghp_thetoken").await;
    let saved = save_viewer(&app, "https://ada.github.io/verkstead-shares/").await;

    assert_eq!(
        saved
            .settings
            .github_token
            .expect("the token is still configured")
            .last_four,
        "oken",
    );
}

/// Save where the share viewer is hosted, with everything else as it stands.
async fn save_viewer(app: &Router, url: &str) -> SettingsSaved {
    save(
        app,
        &serde_json::json!({
            "git_author": { "name": "", "email": "" },
            "github_token": "Keep",
            "rust_build_cache": { "enabled": true, "size": "" },
            "share_viewer_url": url,
            "conflict_resolution": "Merge",
        }),
    )
    .await
}

/// The viewer itself, which is the other half of that setting: a human filling
/// the field in is a human hosting this page, so it has to be obtainable from
/// here.
///
/// An attachment, because the point of the press is having the file — a viewer
/// that opened in the browser would be one served off the tailnet, where nobody
/// a share is sent to can reach it.
#[tokio::test]
async fn the_share_viewer_page_is_handed_over_to_be_hosted() {
    let (_dir, app) = app().await;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/ui/share-viewer.html")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get(header::CONTENT_DISPOSITION)
            .and_then(|value| value.to_str().ok()),
        Some("attachment; filename=\"verkstead-share-viewer.html\""),
    );

    let page = String::from_utf8(
        response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec(),
    )
    .unwrap();

    // The three things the page is: a document, a fetch of the gist straight
    // from GitHub, and a frame the share is drawn in without this page's origin.
    assert!(page.starts_with("<!doctype html>"), "{page}");
    assert!(page.contains("https://api.github.com/gists/"));
    assert!(page.contains(r#"sandbox="allow-scripts""#));

    // And what it is not: anything asked of any other host. Every URL in it is
    // GitHub's, so a recipient reading a share tells the page's host nothing
    // beyond that they opened it.
    for line in page.lines() {
        if let Some(at) = line.find("https://") {
            let url = &line[at..];
            assert!(
                url.starts_with("https://api.github.com/"),
                "the viewer reaches for something that is not GitHub: {line}"
            );
        }
    }
}
