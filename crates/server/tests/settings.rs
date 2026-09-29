//! What Verkstead is told, over the viewer's namespace: reading the git author,
//! the presence of a GitHub token, how the shared Rust build cache is set, what
//! the Cleanup does to an archived Conversation, whether Done shares the record
//! to the pull request, what paths it has been given and the one text every
//! session is given, and writing any of them.
//!
//! That last is the one setting here written for something other than Verkstead
//! to read — a harness is handed the words — so what those tests ask is whether
//! it survives the round trip as it was typed, blank lines and indents and all.
//!
//! The declared MCP servers are the other list a save can be turned down over,
//! and what those tests ask is the two things the refusal is for: a name that is
//! not lowercase letters, digits and hyphens, and one another declaration
//! already has. Neither writes a byte — a refusal is the whole request refused —
//! and a save from any other section carries the declarations along untouched,
//! which is what keeps a corrected email address from being refused over a name
//! somebody hand-edited into the file weeks ago.
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
    CompileCaching, ConflictResolution, IgnoreRule, McpHeader, McpServer, PathResolution,
    PathSource, RuleField, ServerField, SettingsSaved, SettingsView, Verified,
};
use verkstead_server::sandbox::SandboxConfig;
use verkstead_server::{Gh, open_database, router_asking_github, router_installed};

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
            "cleanup": cleanup_unset(),
            "conflict_resolution": "Merge",
            "share_on_done": false,
            "sandbox_binds": [],
            "ignored_comments": "Keep",
            "mcp_servers": "Keep",
            "instructions": "",
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
            "cleanup": cleanup_unset(),
            "conflict_resolution": "Merge",
            "share_on_done": false,
            "sandbox_binds": [],
            "ignored_comments": "Keep",
            "mcp_servers": "Keep",
            "instructions": "",
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
            "cleanup": cleanup_unset(),
            "conflict_resolution": "Merge",
            "share_on_done": false,
            "sandbox_binds": [],
            "ignored_comments": "Keep",
            "mcp_servers": "Keep",
            "instructions": "",
        }),
    )
    .await
}

/// The Cleanup as a save that is not about it sends it: both rows where nobody
/// has put them, which is the trim on and the delete off, with neither duration
/// typed.
///
/// One request writes the whole of `config.yaml`, so every save carries every
/// section — the viewer's own sections do the same thing with what the last read
/// told them, and these tests are about the other settings rather than about
/// this one.
fn cleanup_unset() -> serde_json::Value {
    serde_json::json!({
        "trim": { "enabled": true, "days": "" },
        "delete": { "enabled": false, "days": "" },
    })
}

/// Save a Cleanup and leave everything else alone, which is what the Cleanup
/// pane's own two presses send.
async fn save_cleanup(app: &Router, trim: (bool, &str), delete: (bool, &str)) -> SettingsSaved {
    save(
        app,
        &serde_json::json!({
            "git_author": { "name": "", "email": "" },
            "github_token": "Keep",
            "rust_build_cache": { "enabled": true, "size": "" },
            "cleanup": {
                "trim": { "enabled": trim.0, "days": trim.1 },
                "delete": { "enabled": delete.0, "days": delete.1 },
            },
            "conflict_resolution": "Merge",
            "share_on_done": false,
            "sandbox_binds": [],
            "ignored_comments": "Keep",
            "mcp_servers": "Keep",
            "instructions": "",
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
    assert_eq!(
        settings.instructions, "",
        "and nothing every session is to be told, which is the section drawn \
         with an empty box rather than an error",
    );
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
            "cleanup": cleanup_unset(),
            "conflict_resolution": "Merge",
            "share_on_done": false,
            "sandbox_binds": [],
            "ignored_comments": "Keep",
            "mcp_servers": "Keep",
            "instructions": "",
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

/// The binds are values now, so a save carrying them as they stand is what
/// leaves them alone — which is what the page does with every field the form in
/// front of the human is not about.
#[tokio::test]
async fn a_save_carrying_the_binds_as_they_stand_leaves_them() {
    let (dir, app) = app().await;

    hand_edit(
        dir.path(),
        "config.yaml",
        "sandbox_binds:\n  - /var/cache/verkstead-node\n",
    );

    save(
        &app,
        &serde_json::json!({
            "git_author": { "name": "Tobias Cohen", "email": "tobi@tobico.net" },
            "github_token": "Keep",
            "rust_build_cache": { "enabled": true, "size": "" },
            "cleanup": cleanup_unset(),
            "conflict_resolution": "Merge",
            "share_on_done": false,
            "sandbox_binds": ["/var/cache/verkstead-node"],
            "ignored_comments": "Keep",
            "mcp_servers": "Keep",
            "instructions": "",
        }),
    )
    .await;

    let written = std::fs::read_to_string(dir.path().join("config.yaml")).unwrap();

    assert!(
        written.contains("/var/cache/verkstead-node"),
        "the save should have carried the list through, got:\n{written}"
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
    assert_ne!(
        cache.compiles,
        CompileCaching::Cached,
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
            "cleanup": cleanup_unset(),
            "conflict_resolution": "Merge",
            "share_on_done": false,
            "sandbox_binds": [],
            "ignored_comments": "Keep",
            "mcp_servers": "Keep",
            "instructions": "",
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

/// What the Cleanup does where nobody has said: it trims at three days and
/// deletes never.
///
/// The two halves of the section fall back the two different ways, which is
/// what a page reading this has to draw — and both durations come back as the
/// default with the flag beside them saying nobody chose it.
#[tokio::test]
async fn a_cleanup_nobody_has_configured_trims_and_never_deletes() {
    let (_dir, app) = app().await;

    let cleanup = settings(&app).await.cleanup;

    assert!(cleanup.trim.enabled, "on is what an untouched trim means");
    assert_eq!(cleanup.trim.days, 3);
    assert!(
        !cleanup.trim.days_configured,
        "the default is shown rather than chosen"
    );

    assert!(
        !cleanup.delete.enabled,
        "and off is what an untouched delete means, this being the half that \
         forgets"
    );
    assert_eq!(cleanup.delete.days, 30);
    assert!(!cleanup.delete.days_configured);
}

/// And what a save of it says: the two rows go into the file the sweep reads,
/// and come back off it — including from a router started afresh on the same
/// directory, which is what a restart is.
#[tokio::test]
async fn the_cleanup_switches_and_durations_go_in_and_come_back() {
    let (dir, app) = app().await;

    let saved = save_cleanup(&app, (false, "5"), (true, "90")).await;

    assert!(!saved.settings.cleanup.trim.enabled);
    assert_eq!(saved.settings.cleanup.trim.days, 5);
    assert!(saved.settings.cleanup.trim.days_configured);
    assert!(saved.settings.cleanup.delete.enabled);
    assert_eq!(saved.settings.cleanup.delete.days, 90);
    assert!(saved.settings.cleanup.delete.days_configured);

    // In the file the sweep reads, rather than only in the answer.
    let written = std::fs::read_to_string(dir.path().join("config.yaml")).unwrap();
    assert!(
        written.contains("cleanup:") && written.contains("days: 90"),
        "the switches and the durations are in config.yaml: {written}"
    );

    let read_back = settings(&app).await.cleanup;
    assert!(!read_back.trim.enabled);
    assert_eq!(read_back.trim.days, 5);

    // And to a server that has just come up on the same Data Directory, which
    // is the whole of what surviving a restart means here.
    let restarted = restarted(dir.path()).await;
    assert!(settings(&restarted).await.cleanup.delete.enabled);
}

/// A delete sooner than the trim saves like anything else: the two clocks run
/// from the archiving independently, so what it says is that the Conversation
/// goes before it was ever trimmed.
#[tokio::test]
async fn a_delete_sooner_than_the_trim_is_saved_as_it_was_typed() {
    let (_dir, app) = app().await;

    let saved = save_cleanup(&app, (true, "30"), (true, "2")).await;

    assert_eq!(saved.settings.cleanup.trim.days, 30);
    assert_eq!(saved.settings.cleanup.delete.days, 2);
    assert!(
        saved.refused.is_empty(),
        "there is nothing here to refuse: {:?}",
        saved.refused
    );
}

/// Clearing a duration is asking for the default back rather than for a
/// cleanup no days after the archiving — and so is anything that is not a
/// whole number of days, because the page sends what was typed.
#[tokio::test]
async fn a_cleanup_duration_cleared_is_the_default_again() {
    let (_dir, app) = app().await;

    save_cleanup(&app, (true, "5"), (true, "90")).await;

    let saved = save_cleanup(&app, (true, "  "), (true, "a fortnight")).await;

    assert_eq!(saved.settings.cleanup.trim.days, 3);
    assert!(!saved.settings.cleanup.trim.days_configured);
    assert_eq!(saved.settings.cleanup.delete.days, 30);
    assert!(!saved.settings.cleanup.delete.days_configured);
}

/// A save from another section carries the Cleanup as it stands, and that is
/// what leaves it alone: one request writes the whole of `config.yaml`, so a
/// section saying nothing about a setting would be a section that unset it.
#[tokio::test]
async fn a_save_carrying_the_cleanup_as_it_stands_leaves_it() {
    let (_dir, app) = app().await;

    save_cleanup(&app, (false, "5"), (true, "90")).await;

    // The build cache section's own save, which is about the size and carries
    // the Cleanup exactly as the page last read it.
    let saved = save(
        &app,
        &serde_json::json!({
            "git_author": { "name": "", "email": "" },
            "github_token": "Keep",
            "rust_build_cache": { "enabled": true, "size": "5G" },
            "cleanup": {
                "trim": { "enabled": false, "days": "5" },
                "delete": { "enabled": true, "days": "90" },
            },
            "conflict_resolution": "Merge",
            "share_on_done": false,
            "sandbox_binds": [],
            "ignored_comments": "Keep",
            "mcp_servers": "Keep",
            "instructions": "",
        }),
    )
    .await;

    assert_eq!(saved.settings.rust_build_cache.size, "5G");
    assert!(!saved.settings.cleanup.trim.enabled, "the switch stands");
    assert_eq!(saved.settings.cleanup.delete.days, 90, "and the duration");
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
            "cleanup": cleanup_unset(),
            "conflict_resolution": "Rebase",
            "share_on_done": false,
            "sandbox_binds": [],
            "ignored_comments": "Keep",
            "mcp_servers": "Keep",
            "instructions": "",
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
            "cleanup": cleanup_unset(),
            "conflict_resolution": "Merge",
            "share_on_done": false,
            "sandbox_binds": [],
            "ignored_comments": "Keep",
            "mcp_servers": "Keep",
            "instructions": "",
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
            "cleanup": cleanup_unset(),
            "conflict_resolution": "Merge",
            "share_on_done": true,
            "sandbox_binds": [],
            "ignored_comments": "Keep",
            "mcp_servers": "Keep",
            "instructions": "",
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
            "cleanup": cleanup_unset(),
            "conflict_resolution": "Merge",
            "share_on_done": true,
            "sandbox_binds": [],
            "ignored_comments": "Keep",
            "mcp_servers": "Keep",
            "instructions": "",
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
            "cleanup": cleanup_unset(),
            "conflict_resolution": "Merge",
            "share_on_done": true,
            "sandbox_binds": [],
            "ignored_comments": "Keep",
            "mcp_servers": "Keep",
            "instructions": "",
        }),
    )
    .await;

    assert_eq!(saved.settings.rust_build_cache.size, "5G");
    assert!(saved.settings.share_on_done, "the switch stands");
}

/// The text every session is given goes into the file and comes back off it,
/// line breaks and all — through a restart, because a session reads this file
/// afresh rather than reading anything the server has been holding.
///
/// The one setting on this page written for something other than Verkstead to
/// read: what is in the box is handed to a harness, so what survives the round
/// trip has to be the words that were typed rather than a tidied copy of them.
#[tokio::test]
async fn the_instructions_go_in_and_come_back_verbatim() {
    let (dir, app) = app().await;

    let text = "Prefer the smallest change that does the job.\n\nAnd:\n  - run the tests\n  - say what broke\n";

    let saved = save_instructions(&app, text).await;

    assert_eq!(saved.settings.instructions, text);
    assert!(
        saved.refused.is_empty(),
        "there is nothing in a paragraph of prose to be refused over",
    );

    assert_eq!(settings(&app).await.instructions, text);

    // And to a server that has just come up on the same Data Directory, which
    // is what the next session reading this file looks like from here.
    let restarted = restarted(dir.path()).await;
    assert_eq!(settings(&restarted).await.instructions, text);
}

/// And clearing the box takes the key out of the file rather than leaving an
/// empty one behind: a key holding nothing would read as a setting somebody
/// made.
#[tokio::test]
async fn clearing_the_instructions_writes_the_key_away() {
    let (dir, app) = app().await;

    save_instructions(&app, "Prefer the smallest change.").await;

    let config = dir.path().join("config.yaml");
    assert!(
        std::fs::read_to_string(&config)
            .unwrap()
            .contains("instructions"),
        "the text somebody typed is in config.yaml",
    );

    let saved = save_instructions(&app, "").await;

    assert_eq!(saved.settings.instructions, "");

    let written = std::fs::read_to_string(&config).unwrap();
    assert!(
        !written.contains("instructions"),
        "and clearing it takes the key away: {written}",
    );
}

/// And a save from another section leaves it where it is — the same contract
/// every other value on this page is saved under, because one request writes
/// the whole of `config.yaml`.
///
/// `session_path` rides along too, and it is the one key here no section has a
/// field for: a save built out of what a page sent would take away the
/// directory a harness Verkstead installed is really in.
#[tokio::test]
async fn a_save_carrying_the_instructions_as_they_stand_leaves_them() {
    let (dir, app) = app().await;

    // The one key the page cannot send, written the way an install writes it.
    hand_edit(
        dir.path(),
        "config.yaml",
        "session_path:\n  - /home/you/.local/bin\n",
    );

    save_instructions(&app, "Prefer the smallest change.").await;

    // The build cache section's own save, which is about the size and carries
    // everything else as the page last read it.
    let saved = save(
        &app,
        &serde_json::json!({
            "git_author": { "name": "", "email": "" },
            "github_token": "Keep",
            "rust_build_cache": { "enabled": true, "size": "5G" },
            "cleanup": cleanup_unset(),
            "conflict_resolution": "Merge",
            "share_on_done": false,
            "sandbox_binds": [],
            "ignored_comments": "Keep",
            "mcp_servers": "Keep",
            "instructions": "Prefer the smallest change.",
        }),
    )
    .await;

    assert_eq!(saved.settings.rust_build_cache.size, "5G");
    assert_eq!(
        saved.settings.instructions, "Prefer the smallest change.",
        "the text stands",
    );

    let written = std::fs::read_to_string(dir.path().join("config.yaml")).unwrap();
    assert!(
        written.contains("/home/you/.local/bin"),
        "and so does the one key no section has a field for: {written}",
    );
}

/// Save a text and leave everything else alone, which is what the instructions
/// pane's own press sends.
async fn save_instructions(app: &Router, instructions: &str) -> SettingsSaved {
    save(
        app,
        &serde_json::json!({
            "git_author": { "name": "", "email": "" },
            "github_token": "Keep",
            "rust_build_cache": { "enabled": true, "size": "" },
            "cleanup": cleanup_unset(),
            "conflict_resolution": "Merge",
            "share_on_done": false,
            "sandbox_binds": [],
            "ignored_comments": "Keep",
            "mcp_servers": "Keep",
            "instructions": instructions,
        }),
    )
    .await
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
            "cleanup": cleanup_unset(),
            "conflict_resolution": "Merge",
            "share_on_done": false,
            "sandbox_binds": [],
            "ignored_comments": "Keep",
            "mcp_servers": "Keep",
            "instructions": "",
        }),
    )
    .await;

    let saved = save(
        &app,
        &serde_json::json!({
            "git_author": { "name": "", "email": "" },
            "github_token": "Keep",
            "rust_build_cache": { "enabled": true, "size": "  " },
            "cleanup": cleanup_unset(),
            "conflict_resolution": "Merge",
            "share_on_done": false,
            "sandbox_binds": [],
            "ignored_comments": "Keep",
            "mcp_servers": "Keep",
            "instructions": "",
        }),
    )
    .await;

    assert_eq!(saved.settings.rust_build_cache.size, "30G");
    assert!(!saved.settings.rust_build_cache.size_configured);
}

/// The Sandbox binds half of the page: every Sandbox Configuration bind, from
/// both of the places one is said.
///
/// A server the installation configured as well as a file, because the whole of
/// what this reports is which of the two said an entry and whether the server
/// can see it — and a router that was only ever told things through the page
/// could not be asked the first of those.
async fn app_installed(binds: &[String]) -> (tempfile::TempDir, Router) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    let app = router_installed(
        pool,
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

/// Save the binds and leave the rest of both files alone, which is what the
/// Sandbox binds pane's own press sends.
async fn save_paths(app: &Router, binds: &[&str]) -> SettingsSaved {
    save(
        app,
        &serde_json::json!({
            "git_author": { "name": "", "email": "" },
            "github_token": "Keep",
            "rust_build_cache": { "enabled": true, "size": "" },
            "cleanup": cleanup_unset(),
            "conflict_resolution": "Merge",
            "share_on_done": false,
            "sandbox_binds": binds,
            "ignored_comments": "Keep",
            "mcp_servers": "Keep",
            "instructions": "",
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

    assert!(paths.binds.is_empty(), "{:?}", paths.binds);
}

/// What the unit said comes back labelled as the unit's, so the page can draw it
/// and refuse to let anybody edit it here.
#[tokio::test]
async fn the_installations_own_paths_come_back_as_the_installations() {
    let root = tempfile::tempdir().unwrap();
    let cache = made(root.path(), "node-cache");
    let cargo = made(root.path(), "cargo");

    let (_dir, app) =
        app_installed(&[cache.display().to_string(), cargo.display().to_string()]).await;

    let paths = settings(&app).await.paths;

    let [first, second] = &paths.binds[..] else {
        panic!("two binds, not {:?}", paths.binds);
    };

    assert_eq!(first.path, cache.display().to_string());
    assert_eq!(first.source, PathSource::Installation);
    assert_eq!(first.resolution, PathResolution::Resolves);

    assert_eq!(second.path, cargo.display().to_string());
    assert_eq!(second.source, PathSource::Installation);
}

/// And what the page saved comes back as the settings' own, beside it: the two
/// sources are one list, and the label is the whole of what tells them apart.
#[tokio::test]
async fn the_two_sources_come_back_as_one_list_saying_which_is_which() {
    let root = tempfile::tempdir().unwrap();
    let bound = made(root.path(), "node-cache");
    let cargo = made(root.path(), "cargo");

    let (_dir, app) = app_installed(&[bound.display().to_string()]).await;

    let saved = save_paths(&app, &[&cargo.display().to_string()]).await;

    let binds: Vec<_> = saved
        .settings
        .paths
        .binds
        .iter()
        .map(|entry| (entry.path.clone(), entry.source.clone()))
        .collect();

    assert_eq!(
        binds,
        vec![
            (bound.display().to_string(), PathSource::Installation),
            (cargo.display().to_string(), PathSource::Settings),
        ],
        "the installation's own first, then what the page saved"
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

    let (dir, app) = app().await;

    let saved = save_paths(&app, &[&never_made.display().to_string()]).await;

    // In the file, whatever the server makes of any of it — this is the half a
    // nix install depends on, where a path the hardened unit cannot see is saved
    // now and works when the installer widens the namespace.
    let written = std::fs::read_to_string(dir.path().join("config.yaml")).unwrap();
    assert!(written.contains("never-made"), "{written}");

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

    let saved = save_paths(&app, &["node-cache"]).await;

    let [bind] = &saved.settings.paths.binds[..] else {
        panic!("one bind, not {:?}", saved.settings.paths.binds);
    };

    assert_eq!(bind.path, "node-cache", "the entry as it was written");
    assert_eq!(bind.source, PathSource::Settings);
    assert!(
        why(&bind.resolution).contains("not an absolute path"),
        "{bind:?}"
    );
}

/// Except an entry in the retired `name=path` grammar, which draws nothing at
/// all.
///
/// It was configuration rather than a typo — somebody wrote it when Verkstead
/// read it — and nothing about it is a row worth drawing: it reaches no session,
/// there is no Repo to scope a bind to any more, and a row saying so on every
/// page load would be the settings explaining a setting that no longer exists.
/// `config.yaml` is where it stays until somebody takes it out, or until the
/// next save from this pane drops it.
#[tokio::test]
async fn a_bind_written_for_a_repo_draws_no_row_at_all() {
    let root = tempfile::tempdir().unwrap();
    let cargo = made(root.path(), "cargo");

    let (dir, app) = app().await;

    let saved = save_paths(
        &app,
        &[
            &cargo.display().to_string(),
            &format!("verkstead={}", cargo.display()),
        ],
    )
    .await;

    let [bind] = &saved.settings.paths.binds[..] else {
        panic!("one bind, not {:?}", saved.settings.paths.binds);
    };

    assert_eq!(bind.path, cargo.display().to_string());
    assert_eq!(bind.resolution, PathResolution::Resolves);

    // And it is still in the file, which is the whole of where it lives now: a
    // save lands whatever it was told, and this one was told it.
    let written = std::fs::read_to_string(dir.path().join("config.yaml")).unwrap();
    assert!(written.contains("verkstead="), "{written}");
}

/// A save says what the settings hold afterwards, and says nothing at all about
/// what the installation said: those are the unit's word, and this page has no
/// way to reach them.
#[tokio::test]
async fn a_save_replaces_the_settings_binds_and_leaves_the_installations() {
    let root = tempfile::tempdir().unwrap();
    let bound = made(root.path(), "node-cache");

    let (dir, app) = app_installed(&[bound.display().to_string()]).await;

    save_paths(&app, &["/var/cache/first"]).await;
    let saved = save_paths(&app, &["/var/cache/second"]).await;

    let written = std::fs::read_to_string(dir.path().join("config.yaml")).unwrap();

    assert!(written.contains("/var/cache/second"), "{written}");
    assert!(
        !written.contains("/var/cache/first"),
        "the first save's list is gone: {written}"
    );
    assert!(
        !written.contains(&bound.display().to_string()),
        "the installation's own was never in this file: {written}"
    );

    // And it still stands, because nothing here could have touched it.
    assert_eq!(
        saved
            .settings
            .paths
            .binds
            .iter()
            .map(|entry| entry.source.clone())
            .collect::<Vec<_>>(),
        vec![PathSource::Installation, PathSource::Settings],
        "the installation's bind first, then the one this save wrote"
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
            "cleanup": cleanup_unset(),
            "conflict_resolution": "Merge",
            "share_on_done": false,
            "sandbox_binds": [],
            "ignored_comments": { "Set": { "rules": rules } },
            "mcp_servers": "Keep",
            "instructions": "",
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
            "cleanup": cleanup_unset(),
            "conflict_resolution": "Merge",
            "share_on_done": false,
            "sandbox_binds": [],
            "ignored_comments": { "Set": { "rules": [rule("", "[oh")] } },
            "mcp_servers": "Keep",
            "instructions": "",
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

/// Save a list of MCP server declarations and leave the rest of both files
/// alone, which is what the section's own press sends.
async fn save_servers(app: &Router, servers: serde_json::Value) -> SettingsSaved {
    save(
        app,
        &serde_json::json!({
            "git_author": { "name": "", "email": "" },
            "github_token": "Keep",
            "rust_build_cache": { "enabled": true, "size": "" },
            "cleanup": cleanup_unset(),
            "conflict_resolution": "Merge",
            "share_on_done": false,
            "sandbox_binds": [],
            "ignored_comments": "Keep",
            "mcp_servers": { "Set": { "servers": servers } },
            "instructions": "",
        }),
    )
    .await
}

fn server(name: &str, url: &str) -> serde_json::Value {
    with_headers(name, url, serde_json::json!([]))
}

/// And one with headers on it, each of them a name and what is to become of the
/// value sent in it — see [`set`], [`keep`] and [`clear`].
fn with_headers(name: &str, url: &str, headers: serde_json::Value) -> serde_json::Value {
    serde_json::json!({ "name": name, "url": url, "headers": headers })
}

/// A header given a value, which is the only way one is ever typed.
fn set(name: &str, value: &str) -> serde_json::Value {
    serde_json::json!({ "name": name, "value": { "Set": { "value": value } } })
}

/// And one whose value box was left alone, which is what every header of a
/// server somebody only corrected the URL of sends.
fn keep(name: &str) -> serde_json::Value {
    serde_json::json!({ "name": name, "value": "Keep" })
}

/// And one whose value is being taken away, the header staying declared with
/// nothing to send in it.
fn clear(name: &str) -> serde_json::Value {
    serde_json::json!({ "name": name, "value": "Clear" })
}

/// And the declaration as it comes back, for comparing against: a name, a URL
/// and the names of its headers.
fn declared(name: &str, url: &str) -> McpServer {
    declaring(name, url, &[])
}

/// And one with headers on it, each its name and whether a value is kept to
/// send in it.
fn declaring(name: &str, url: &str, headers: &[(&str, bool)]) -> McpServer {
    McpServer {
        name: name.to_owned(),
        url: url.to_owned(),
        headers: headers
            .iter()
            .map(|(header, set)| McpHeader {
                name: (*header).to_owned(),
                set: *set,
            })
            .collect(),
    }
}

/// What `secrets.yaml` holds now, as text — which is where a header value is
/// and the one place it can be read back from.
fn secrets(dir: &tempfile::TempDir) -> String {
    std::fs::read_to_string(dir.path().join("secrets.yaml")).unwrap_or_default()
}

#[tokio::test]
async fn a_verkstead_nobody_has_declared_a_server_on_has_none() {
    let (_dir, app) = app().await;

    assert!(settings(&app).await.mcp_servers.is_empty());
}

#[tokio::test]
async fn the_declared_servers_go_in_and_come_back() {
    let (_dir, app) = app().await;

    let saved = save_servers(
        &app,
        serde_json::json!([
            server("docs", "https://mcp.example.com/docs"),
            server("tickets", "https://mcp.example.com/tickets"),
        ]),
    )
    .await;

    assert!(
        saved.refused_servers.is_empty(),
        "{:?}",
        saved.refused_servers
    );
    assert_eq!(
        saved.settings.mcp_servers,
        vec![
            declared("docs", "https://mcp.example.com/docs"),
            declared("tickets", "https://mcp.example.com/tickets"),
        ]
    );

    // And a read of its own says the same, which is the half that survives a
    // reload: the file is where the declarations are.
    assert_eq!(settings(&app).await.mcp_servers, saved.settings.mcp_servers);
}

/// The file rather than the process: a second server over the same Data
/// Directory reads what the first one wrote.
#[tokio::test]
async fn the_declarations_are_in_the_config_file_and_outlive_the_server() {
    let (dir, app) = app().await;

    save_servers(
        &app,
        serde_json::json!([server("docs", "https://mcp.example.com/docs")]),
    )
    .await;

    let written = std::fs::read_to_string(dir.path().join("config.yaml")).unwrap();

    assert!(written.contains("mcp_servers"), "{written}");
    assert!(
        written.contains("https://mcp.example.com/docs"),
        "{written}"
    );

    let restarted = app_over(dir.path()).await;

    assert_eq!(
        settings(&restarted).await.mcp_servers,
        vec![declared("docs", "https://mcp.example.com/docs")]
    );
}

/// A URL rewritten is the whole list sent again with that one changed, which is
/// the only editing this section does: the name it stands under is untouched.
#[tokio::test]
async fn a_declarations_url_can_be_changed() {
    let (_dir, app) = app().await;

    save_servers(
        &app,
        serde_json::json!([server("docs", "https://mcp.example.com/docs")]),
    )
    .await;

    let saved = save_servers(
        &app,
        serde_json::json!([server("docs", "https://docs.example.com/mcp")]),
    )
    .await;

    assert_eq!(
        saved.settings.mcp_servers,
        vec![declared("docs", "https://docs.example.com/mcp")]
    );
}

/// The last declaration taken off the page is the list emptied, which is a save
/// that sends none rather than one that says nothing.
#[tokio::test]
async fn sending_no_servers_takes_the_ones_that_were_there_away() {
    let (_dir, app) = app().await;

    save_servers(
        &app,
        serde_json::json!([server("docs", "https://mcp.example.com/docs")]),
    )
    .await;

    let saved = save_servers(&app, serde_json::json!([])).await;

    assert!(saved.settings.mcp_servers.is_empty());
    assert!(settings(&app).await.mcp_servers.is_empty());
}

/// The name is what a chip refers to a server by and what the agent sees in
/// front of its tool names, so anything but lowercase letters, digits and
/// hyphens is refused at the box it was typed in.
#[tokio::test]
async fn a_name_that_is_not_a_name_is_refused_at_its_own_field() {
    let (_dir, app) = app().await;

    for name in ["Docs", "docs server", "docs_server", "docs.example", ""] {
        let saved = save_servers(
            &app,
            serde_json::json!([server(name, "https://mcp.example.com/docs")]),
        )
        .await;

        assert_eq!(saved.refused_servers.len(), 1, "{name:?}");
        assert_eq!(saved.refused_servers[0].server, 0, "{name:?}");
        assert_eq!(
            saved.refused_servers[0].field,
            ServerField::Name,
            "{name:?}"
        );
        assert!(!saved.refused_servers[0].why.is_empty(), "{name:?}");

        assert!(
            settings(&app).await.mcp_servers.is_empty(),
            "nothing should have been written for {name:?}"
        );
    }
}

/// And a name another declaration already has, which is the row the human just
/// typed rather than the one that was there before them.
#[tokio::test]
async fn a_name_already_taken_is_refused_at_the_row_that_took_it() {
    let (_dir, app) = app().await;

    let saved = save_servers(
        &app,
        serde_json::json!([
            server("docs", "https://mcp.example.com/one"),
            server("docs", "https://mcp.example.com/two"),
        ]),
    )
    .await;

    assert_eq!(saved.refused_servers.len(), 1);
    assert_eq!(saved.refused_servers[0].server, 1);
    assert_eq!(saved.refused_servers[0].field, ServerField::Name);

    assert!(settings(&app).await.mcp_servers.is_empty());
}

/// And a declaration with nowhere to be reached, which is the other box.
#[tokio::test]
async fn a_declaration_with_no_url_is_refused_at_the_url() {
    let (_dir, app) = app().await;

    let saved = save_servers(&app, serde_json::json!([server("docs", "")])).await;

    assert_eq!(saved.refused_servers.len(), 1);
    assert_eq!(saved.refused_servers[0].field, ServerField::Url);
    assert!(settings(&app).await.mcp_servers.is_empty());
}

/// A refusal is the whole request refused, and not the author written while the
/// declarations were turned away.
#[tokio::test]
async fn a_save_refused_over_a_name_writes_nothing_at_all() {
    let (_dir, app) = app().await;

    save_author(&app, "Ada Lovelace", "ada@example.com").await;

    let saved = save(
        &app,
        &serde_json::json!({
            "git_author": { "name": "Tobias Cohen", "email": "tobi@tobico.net" },
            "github_token": { "Set": { "token": "ghp_thetoken" } },
            "rust_build_cache": { "enabled": true, "size": "" },
            "cleanup": cleanup_unset(),
            "conflict_resolution": "Merge",
            "share_on_done": false,
            "sandbox_binds": [],
            "ignored_comments": "Keep",
            "mcp_servers": { "Set": { "servers": [server("Docs", "https://example.com")] } },
            "instructions": "",
        }),
    )
    .await;

    assert_eq!(saved.refused_servers.len(), 1);
    assert!(saved.refused.is_empty());
    assert!(saved.verified.is_none(), "no token was tried");
    assert_eq!(saved.settings.git_author.name, "Ada Lovelace");
    assert!(saved.settings.github_token.is_none());
    assert_eq!(settings(&app).await.git_author.name, "Ada Lovelace");
}

/// What every section but this one sends, and what makes those saves ones that
/// cannot be refused over a declaration — asked of each of the sections in turn,
/// because every one of them writes the whole of `config.yaml` and a section
/// that left the declarations out would be a section that took them away.
#[tokio::test]
async fn a_save_from_another_section_leaves_the_declarations_where_they_are() {
    let (_dir, app) = app().await;

    save_servers(
        &app,
        serde_json::json!([server("docs", "https://mcp.example.com/docs")]),
    )
    .await;

    let declarations = vec![declared("docs", "https://mcp.example.com/docs")];

    for saved in [
        save_author(&app, "Ada Lovelace", "ada@example.com").await,
        save_instructions(&app, "Prefer the smallest change.\n").await,
        save_paths(&app, &["/var/cache/verkstead-node"]).await,
        save_cleanup(&app, (true, "5"), (true, "90")).await,
        save_rules(&app, serde_json::json!([rule("coderabbitai", "billing")])).await,
        save_token(&app, "ghp_thetoken").await,
    ] {
        assert!(
            saved.refused_servers.is_empty(),
            "{:?}",
            saved.refused_servers
        );
        assert_eq!(saved.settings.mcp_servers, declarations);
    }

    assert_eq!(settings(&app).await.mcp_servers, declarations);
}

/// And the other way about: this section's own save leaves what it was not told
/// about exactly where it was.
///
/// The ignore rules are what that comes to here. Everything else in
/// `config.yaml` travels as a value, so the page rides it along as the server
/// gave it and the sending is the section's business rather than the endpoint's;
/// the rules are the one other thing that travels as an action, and a save that
/// carried them away would be one that could.
#[tokio::test]
async fn saving_a_declaration_leaves_the_rules_where_they_are() {
    let (_dir, app) = app().await;

    save_rules(&app, serde_json::json!([rule("coderabbitai", "billing")])).await;

    let saved = save_servers(
        &app,
        serde_json::json!([server("docs", "https://mcp.example.com/docs")]),
    )
    .await;

    assert_eq!(
        saved.settings.ignored_comments,
        vec![IgnoreRule {
            author: "coderabbitai".to_owned(),
            body: "billing".to_owned(),
        }]
    );
    assert_eq!(
        saved.settings.mcp_servers,
        vec![declared("docs", "https://mcp.example.com/docs")]
    );
}

/// A declaration somebody hand-edited badly is the case this arrangement is for:
/// it comes back on the read so the human can correct it, and it refuses nothing
/// until they save the section it is on.
#[tokio::test]
async fn a_hand_edited_bad_name_reads_back_and_refuses_no_other_save() {
    let (dir, app) = app().await;

    std::fs::write(
        dir.path().join("config.yaml"),
        "mcp_servers:\n  - name: Docs Server\n    url: https://example.com\n",
    )
    .unwrap();

    assert_eq!(
        settings(&app).await.mcp_servers,
        vec![declared("Docs Server", "https://example.com")]
    );

    let saved = save_author(&app, "Ada Lovelace", "ada@example.com").await;

    assert!(saved.refused_servers.is_empty());
    assert_eq!(saved.settings.git_author.name, "Ada Lovelace");
    assert_eq!(
        saved.settings.mcp_servers,
        vec![declared("Docs Server", "https://example.com")]
    );
}

/// And the hand-edit the reading half does drop, because a declaration missing
/// either half is no declaration: one with no name is nothing a chip could refer
/// to, and one with no URL reaches nothing.
#[tokio::test]
async fn a_hand_edited_half_declaration_is_not_read_back() {
    let (dir, app) = app().await;

    std::fs::write(
        dir.path().join("config.yaml"),
        "mcp_servers:\n  - name: docs\n  - url: https://example.com\n  - name: tickets\n    url: https://mcp.example.com/tickets\n",
    )
    .unwrap();

    assert_eq!(
        settings(&app).await.mcp_servers,
        vec![declared("tickets", "https://mcp.example.com/tickets")]
    );
}

/// And a file nothing can parse is no servers rather than a read that fails,
/// which is the rule the whole of this file is read under.
#[tokio::test]
async fn a_config_file_nothing_can_parse_reads_as_no_servers() {
    let (dir, app) = app().await;

    std::fs::write(dir.path().join("config.yaml"), "mcp_servers: [oh\n").unwrap();

    assert!(settings(&app).await.mcp_servers.is_empty());
}

/// A header is added to a declaration by name and value, and what comes back is
/// the name alone: the value is a secret, and the page is never shown one again.
#[tokio::test]
async fn a_headers_name_comes_back_and_its_value_never_does() {
    let (dir, app) = app().await;

    let saved = save_servers(
        &app,
        serde_json::json!([with_headers(
            "docs",
            "https://mcp.example.com/docs",
            serde_json::json!([
                set("Authorization", "Bearer sk-averysecretkey"),
                set("X-Tenant", "verkstead"),
            ]),
        )]),
    )
    .await;

    assert!(
        saved.refused_servers.is_empty(),
        "{:?}",
        saved.refused_servers
    );
    assert_eq!(
        saved.settings.mcp_servers,
        vec![declaring(
            "docs",
            "https://mcp.example.com/docs",
            &[("Authorization", true), ("X-Tenant", true)],
        )],
        "the names, in the order they were declared in"
    );

    // And a read of its own says the same, which is the half that survives a
    // reload.
    assert_eq!(settings(&app).await.mcp_servers, saved.settings.mcp_servers,);

    // The values went where a secret goes, rather than nowhere at all: what the
    // page cannot read back is still what a session will be handed.
    let written = secrets(&dir);

    assert!(written.contains("Bearer sk-averysecretkey"), "{written}");
    assert!(written.contains("verkstead"), "{written}");

    // And `config.yaml` holds the names and no part of either value, the two
    // files being split by what is secret rather than by what it configures.
    let config = std::fs::read_to_string(dir.path().join("config.yaml")).unwrap();

    assert!(config.contains("Authorization"), "{config}");
    assert!(!config.contains("sk-averysecretkey"), "{config}");
}

/// The promise the token's own test makes, made again for a header: a value that
/// has gone in is never in a response body again, whichever body it is.
#[tokio::test]
async fn a_header_value_appears_in_no_answer_this_endpoint_gives() {
    let (_dir, app) = app_asking(SAYS_AN_ACCOUNT).await;

    let saving = save_body(
        &app,
        &serde_json::json!({
            "git_author": { "name": "", "email": "" },
            "github_token": "Keep",
            "rust_build_cache": { "enabled": true, "size": "" },
            "cleanup": cleanup_unset(),
            "conflict_resolution": "Merge",
            "share_on_done": false,
            "sandbox_binds": [],
            "ignored_comments": "Keep",
            "mcp_servers": { "Set": { "servers": [with_headers(
                "docs",
                "https://mcp.example.com/docs",
                serde_json::json!([set("Authorization", "Bearer sk-averysecretkey")]),
            )] } },
            "instructions": "",
        }),
    )
    .await;

    assert!(
        !saving.contains("sk-averysecretkey"),
        "the save answered with the header value: {saving}"
    );

    let reading = settings_body(&app).await;

    assert!(
        !reading.contains("sk-averysecretkey"),
        "the read answered with the header value: {reading}"
    );

    // Not even a tail of it, which is the one thing the token does give back:
    // there is nothing a human does with part of an API key, and the name is
    // what tells one header from another.
    assert!(!reading.contains("secretkey"), "{reading}");
}

/// A value box left blank keeps what is there, so correcting a URL does not take
/// a key away — the token's rule, said once per header.
#[tokio::test]
async fn a_blank_value_box_keeps_the_value_that_was_there() {
    let (dir, app) = app().await;

    save_servers(
        &app,
        serde_json::json!([with_headers(
            "docs",
            "https://mcp.example.com/docs",
            serde_json::json!([set("Authorization", "Bearer sk-averysecretkey")]),
        )]),
    )
    .await;

    let saved = save_servers(
        &app,
        serde_json::json!([with_headers(
            "docs",
            "https://docs.internal/mcp",
            serde_json::json!([keep("Authorization")]),
        )]),
    )
    .await;

    assert_eq!(
        saved.settings.mcp_servers,
        vec![declaring(
            "docs",
            "https://docs.internal/mcp",
            &[("Authorization", true)]
        )]
    );
    assert!(
        secrets(&dir).contains("Bearer sk-averysecretkey"),
        "the URL was corrected and the key stayed: {}",
        secrets(&dir)
    );
}

/// And each header is its own: one set, one kept and one cleared in the one
/// save, each of them doing what it was told and nothing to the others.
#[tokio::test]
async fn a_header_is_kept_set_or_cleared_on_its_own() {
    let (dir, app) = app().await;

    save_servers(
        &app,
        serde_json::json!([with_headers(
            "docs",
            "https://mcp.example.com/docs",
            serde_json::json!([
                set("Authorization", "Bearer sk-thefirstkey"),
                set("X-Tenant", "verkstead"),
                set("X-Spent", "gone-by-the-next-save"),
            ]),
        )]),
    )
    .await;

    let saved = save_servers(
        &app,
        serde_json::json!([with_headers(
            "docs",
            "https://mcp.example.com/docs",
            serde_json::json!([
                set("Authorization", "Bearer sk-thesecondkey"),
                keep("X-Tenant"),
                clear("X-Spent"),
            ]),
        )]),
    )
    .await;

    // Every one of them is still declared: clearing a value is not taking the
    // header off, and the page draws all three.
    assert_eq!(
        saved.settings.mcp_servers,
        vec![declaring(
            "docs",
            "https://mcp.example.com/docs",
            &[
                ("Authorization", true),
                ("X-Tenant", true),
                ("X-Spent", false)
            ],
        )]
    );

    let written = secrets(&dir);

    assert!(written.contains("Bearer sk-thesecondkey"), "{written}");
    assert!(!written.contains("sk-thefirstkey"), "{written}");
    assert!(written.contains("verkstead"), "{written}");
    assert!(!written.contains("gone-by-the-next-save"), "{written}");
}

/// And a header taken off the row is one the declaration no longer names, its
/// value gone with it.
#[tokio::test]
async fn a_header_taken_off_the_declaration_takes_its_value_with_it() {
    let (dir, app) = app().await;

    save_servers(
        &app,
        serde_json::json!([with_headers(
            "docs",
            "https://mcp.example.com/docs",
            serde_json::json!([set("Authorization", "Bearer sk-averysecretkey")]),
        )]),
    )
    .await;

    let saved = save_servers(
        &app,
        serde_json::json!([server("docs", "https://mcp.example.com/docs")]),
    )
    .await;

    assert_eq!(
        saved.settings.mcp_servers,
        vec![declared("docs", "https://mcp.example.com/docs")]
    );
    assert!(
        !secrets(&dir).contains("sk-averysecretkey"),
        "{}",
        secrets(&dir)
    );
}

/// Deleting a declaration takes its headers with it, so declaring that name
/// again starts with none — the secrets are held under the name that declared
/// them, and a name nobody declares holds nothing.
#[tokio::test]
async fn deleting_a_server_takes_its_header_values_with_it() {
    let (dir, app) = app().await;

    save_servers(
        &app,
        serde_json::json!([with_headers(
            "docs",
            "https://mcp.example.com/docs",
            serde_json::json!([set("Authorization", "Bearer sk-averysecretkey")]),
        )]),
    )
    .await;

    save_servers(&app, serde_json::json!([])).await;

    assert!(
        !secrets(&dir).contains("sk-averysecretkey"),
        "{}",
        secrets(&dir)
    );

    // And the name declared again is a server with nothing kept for it: a header
    // named now is one with no value until somebody types one.
    let saved = save_servers(
        &app,
        serde_json::json!([with_headers(
            "docs",
            "https://mcp.example.com/docs",
            serde_json::json!([keep("Authorization")]),
        )]),
    )
    .await;

    assert_eq!(
        saved.settings.mcp_servers,
        vec![declaring(
            "docs",
            "https://mcp.example.com/docs",
            &[("Authorization", false)]
        )],
        "the header is declared with nothing kept for it"
    );
    assert!(
        !secrets(&dir).contains("Authorization"),
        "and nothing is kept for it: {}",
        secrets(&dir)
    );
}

/// `secrets.yaml` is written whole, so the two hands that write it may not take
/// each other's work away: saving a server leaves the token where it was, and
/// saving the token leaves every server's headers where they were.
#[tokio::test]
async fn a_server_and_the_token_are_saved_without_taking_each_other_away() {
    let (dir, app) = app().await;

    save_token(&app, "ghp_thetoken").await;

    save_servers(
        &app,
        serde_json::json!([with_headers(
            "docs",
            "https://mcp.example.com/docs",
            serde_json::json!([set("Authorization", "Bearer sk-averysecretkey")]),
        )]),
    )
    .await;

    assert_eq!(
        settings(&app)
            .await
            .github_token
            .expect("the token survived the server's save")
            .last_four,
        "oken"
    );

    // And the other way about, which is the save this endpoint makes most often.
    let saved = save_token(&app, "ghp_anothertoken").await;

    assert_eq!(
        saved.settings.mcp_servers,
        vec![declaring(
            "docs",
            "https://mcp.example.com/docs",
            &[("Authorization", true)]
        )]
    );

    let written = secrets(&dir);

    assert!(written.contains("Bearer sk-averysecretkey"), "{written}");
    assert!(written.contains("ghp_anothertoken"), "{written}");

    // And a save from a section that is about neither leaves both.
    save_author(&app, "Ada Lovelace", "ada@example.com").await;

    let written = secrets(&dir);

    assert!(written.contains("Bearer sk-averysecretkey"), "{written}");
    assert!(written.contains("ghp_anothertoken"), "{written}");
}

/// A header value cleared where it was the only secret leaves the file saying
/// what an unwritten one says — which is what the token's own clearing does, and
/// what keeps a Verkstead that has been to the page and back looking like one
/// that never went.
#[tokio::test]
async fn clearing_the_last_header_leaves_nothing_configured() {
    let (dir, app) = app().await;

    save_servers(
        &app,
        serde_json::json!([with_headers(
            "docs",
            "https://mcp.example.com/docs",
            serde_json::json!([set("Authorization", "Bearer sk-averysecretkey")]),
        )]),
    )
    .await;

    save_servers(
        &app,
        serde_json::json!([with_headers(
            "docs",
            "https://mcp.example.com/docs",
            serde_json::json!([clear("Authorization")]),
        )]),
    )
    .await;

    assert!(secrets(&dir).trim().is_empty(), "{}", secrets(&dir));
}

/// A blank header name is no header, the way an emptied bind row is no bind: the
/// rows are the page's and an empty one is a row somebody added and left.
#[tokio::test]
async fn a_blank_header_name_is_not_a_header() {
    let (dir, app) = app().await;

    let saved = save_servers(
        &app,
        serde_json::json!([with_headers(
            "docs",
            "https://mcp.example.com/docs",
            serde_json::json!([
                set("   ", "Bearer sk-averysecretkey"),
                set("X-Tenant", "verkstead")
            ]),
        )]),
    )
    .await;

    assert_eq!(
        saved.settings.mcp_servers,
        vec![declaring(
            "docs",
            "https://mcp.example.com/docs",
            &[("X-Tenant", true)]
        )]
    );
    assert!(
        !secrets(&dir).contains("sk-averysecretkey"),
        "and nothing was written under a name nothing declares: {}",
        secrets(&dir)
    );
}

/// The file rather than the process, for the headers as for the declarations: a
/// second server over the same Data Directory reads back the names the first one
/// wrote, and is handed the values it wrote beside them.
#[tokio::test]
async fn the_headers_outlive_the_server() {
    let (dir, app) = app().await;

    save_servers(
        &app,
        serde_json::json!([with_headers(
            "docs",
            "https://mcp.example.com/docs",
            serde_json::json!([set("Authorization", "Bearer sk-averysecretkey")]),
        )]),
    )
    .await;

    let restarted = app_over(dir.path()).await;

    assert_eq!(
        settings(&restarted).await.mcp_servers,
        vec![declaring(
            "docs",
            "https://mcp.example.com/docs",
            &[("Authorization", true)]
        )]
    );

    // And a save made by *that* server keeps what it never saw, which is the
    // whole of why a value box left blank is a keeping.
    save_servers(
        &restarted,
        serde_json::json!([with_headers(
            "docs",
            "https://docs.internal/mcp",
            serde_json::json!([keep("Authorization")]),
        )]),
    )
    .await;

    assert!(
        secrets(&dir).contains("Bearer sk-averysecretkey"),
        "{}",
        secrets(&dir)
    );
}
