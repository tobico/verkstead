//! The open pull requests Verkstead did not open: what `/api/ui/open-pull-requests`
//! answers with, and what it stays quiet about.
//!
//! The door into the pipeline for work that is already somewhere else — a pull
//! request opened by hand, by a contributor or by the old tools — which the
//! *Wrap up a pull request* level under the compose box is drawn from.
//!
//! Asked of the endpoint rather than of [`verkstead_server::pull_requests`],
//! because what is worth proving is the whole reading: `gh` run in each
//! registered Repo's own directory, what it said crossed with Verkstead's own
//! record of which pull requests are already held, and a Repo that could not
//! answer contributing nothing rather than failing the page.
//!
//! **`gh` is a shell script here**, which is what keeps this suite off Windows:
//! a stand-in for a program is a program, and its stdout, its stderr and its
//! exit status are the whole of the interface being read. Each Repo carries its
//! own answer as a file in its directory — `gh` is run with that directory as
//! its working directory, so one script answers differently in each of them,
//! which is what a machine with one `gh` and six repositories actually looks
//! like.

#![cfg(unix)]

use std::path::Path;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use sqlx::SqlitePool;
use tower::ServiceExt;
use verkstead_render::OpenPullRequestRepo;
use verkstead_server::{Gh, open_database, router_asking_github, store};

/// The device every Conversation started here is ranked by, named the way a
/// cluster names one (ADR-0020, *Ranks*).
const THIS_DEVICE: &str = "aa00bb11cc22dd33ee44ff5566778899";

/// A router whose `gh` answers out of whatever directory it is run in.
///
/// The script prints `gh-stdout` where the directory has one, and otherwise
/// prints `gh-stderr` and fails — which is every way a `gh` can decline: no
/// GitHub remote, nobody logged in, a GitHub that would not say. A directory
/// with neither file is a `gh` that said nothing at all and failed, which is
/// what the rest of the suite treats a Repo it never set up as.
///
/// What it was asked is appended to `asked` in the Data Directory, so a test can
/// say which question reached GitHub rather than only that one did.
async fn app() -> (tempfile::TempDir, SqlitePool, Router) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    let gh = Gh::running(vec![
        "/bin/sh".to_owned(),
        "-c".to_owned(),
        format!(
            r#"printf '%s\n' "$*" >> "{}/asked"; \
               if [ -f ./gh-stdout ]; then cat ./gh-stdout; exit 0; fi; \
               cat ./gh-stderr >&2 2>/dev/null; exit 1"#,
            dir.path().display(),
        ),
        // `sh -c` gives `$0` the script's own name, so what Verkstead passes
        // lands in `$1` onwards.
        "gh".to_owned(),
    ]);

    let data_dir = dir.path().to_owned();

    (dir, pool.clone(), router_asking_github(pool, data_dir, gh))
}

/// Register a Repo under `dir` whose `gh` answers `said` on stdout.
///
/// A plain directory rather than a git repository, and deliberately: this
/// endpoint reads nothing off the filesystem: it runs `gh` in the Repo's path
/// and reads what came back, so a repository here would be scenery.
async fn repo_answering(pool: &SqlitePool, dir: &Path, name: &str, said: &str) -> store::Repo {
    let path = dir.join(name);
    std::fs::create_dir_all(&path).unwrap();
    std::fs::write(path.join("gh-stdout"), said).unwrap();

    store::register_repo(pool, &path, name, "main")
        .await
        .unwrap()
        .expect("nothing is registered at that path yet")
}

/// And one whose `gh` declines with `wrong` on stderr.
async fn repo_refusing(pool: &SqlitePool, dir: &Path, name: &str, wrong: &str) -> store::Repo {
    let path = dir.join(name);
    std::fs::create_dir_all(&path).unwrap();
    std::fs::write(path.join("gh-stderr"), wrong).unwrap();

    store::register_repo(pool, &path, name, "main")
        .await
        .unwrap()
        .expect("nothing is registered at that path yet")
}

/// One pull request as `gh pr list --json` gives it, in `verkstead`.
fn listed(number: i64, title: &str, head: &str, author: &str, fork: bool) -> serde_json::Value {
    listed_in("verkstead", number, title, head, author, fork)
}

/// And the same in whichever repository, for the fixture that has two of them:
/// a URL says which repository a number is in, and two rows that named one
/// would say the list is grouped by nothing.
fn listed_in(
    repo: &str,
    number: i64,
    title: &str,
    head: &str,
    author: &str,
    fork: bool,
) -> serde_json::Value {
    serde_json::json!({
        "number": number,
        "title": title,
        // The description, which is what a load prefills the box under the
        // title with. Written off the title so a row's own body is plainly its
        // own — two rows sharing one would say nothing about which was loaded.
        "body": format!("What {title} is for, at some length."),
        "url": format!("https://github.com/tobico/{repo}/pull/{number}"),
        "headRefName": head,
        "baseRefName": "main",
        "author": { "login": author },
        "isCrossRepository": fork,
    })
}

/// What the endpoint answers with, read back as the payload the viewer gets.
async fn open(app: &Router) -> Vec<OpenPullRequestRepo> {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/ui/open-pull-requests")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

/// The ordinary reading: everything open in a Repo, less what cannot be worked
/// on over `origin`.
///
/// Two pull requests and one row, because the second one's head branch is in a
/// fork — see `OpenPullRequest`, where why that is left out is written down.
/// What the row carries is what a human picks a pull request by: which
/// repository, which number, what it is called, the branch the work is on, the
/// branch it goes into and whose it is.
#[tokio::test]
async fn every_open_pull_request_but_a_forks_comes_back_with_its_branches_and_its_author() {
    let (dir, pool, app) = app().await;

    repo_answering(
        &pool,
        dir.path(),
        "verkstead",
        &serde_json::Value::Array(vec![
            listed(41, "Rate limiting", "rate-limiting", "ada", false),
            listed(42, "Typo in the README", "readme-typo", "grace", true),
        ])
        .to_string(),
    )
    .await;

    let open = open(&app).await;

    assert_eq!(open.len(), 1, "one Repo answered: {open:?}");
    assert_eq!(open[0].repo, "verkstead");
    assert_eq!(
        open[0].pull_requests.len(),
        1,
        "the fork is left out: {:?}",
        open[0].pull_requests,
    );

    let one = &open[0].pull_requests[0];
    assert_eq!(one.number, 41);
    assert_eq!(one.title, "Rate limiting");
    assert_eq!(one.url, "https://github.com/tobico/verkstead/pull/41");
    assert_eq!(one.head, "rate-limiting");
    assert_eq!(one.base, "main");
    assert_eq!(one.author, "ada");
    assert_eq!(
        one.body, "What Rate limiting is for, at some length.",
        "and the description, which is what a load prefills the box with",
    );
    assert_eq!(
        one.conversation_id, None,
        "nothing is holding it, so a row loads it rather than leading anywhere",
    );

    // And the question that was asked is the one this list is about: the open
    // ones, with every field a row draws.
    let asked = std::fs::read_to_string(dir.path().join("asked")).unwrap();
    assert!(asked.contains("pr list"), "gh was asked: {asked}");
    assert!(asked.contains("--state open"), "for the open ones: {asked}");
    assert!(
        asked.contains("headRefName") && asked.contains("isCrossRepository"),
        "with the fields a row and the fork rule need: {asked}",
    );
    assert!(
        asked.contains("body"),
        "and the description a load prefills the box with: {asked}",
    );
}

/// A pull request Verkstead already has a record of is listed and says whose it
/// is, whatever state that Conversation has reached.
///
/// Closed here, which is the far end of the ladder and the case worth pinning:
/// the pull request is on that Conversation's record for good, so a second one
/// started over the same branch would be two wrap-ups pushing to it.
#[tokio::test]
async fn a_pull_request_a_closed_conversation_holds_says_which_one() {
    let (dir, pool, app) = app().await;

    let repo = repo_answering(
        &pool,
        dir.path(),
        "verkstead",
        &serde_json::Value::Array(vec![
            listed(41, "Rate limiting", "rate-limiting", "ada", false),
            listed(43, "Nothing has this one", "loose-end", "ada", false),
        ])
        .to_string(),
    )
    .await;

    // A Conversation that finished on #41 and was then closed out, which is the
    // shape of work the human is done with.
    let conversation = store::start_conversation(&pool, repo.id, "rate-limiting", THIS_DEVICE)
        .await
        .unwrap()
        .unwrap();

    store::set_state(&pool, conversation, store::Lifecycle::Implementing)
        .await
        .unwrap();

    store::record_pull_request(
        &pool,
        conversation,
        repo.id,
        &store::PullRequest {
            number: 41,
            title: "Rate limiting".to_owned(),
            url: "https://github.com/tobico/verkstead/pull/41".to_owned(),
            repo: None,
        },
    )
    .await
    .unwrap();

    store::set_state(&pool, conversation, store::Lifecycle::Closed)
        .await
        .unwrap();

    let open = open(&app).await;
    let rows = &open[0].pull_requests;

    assert_eq!(rows.len(), 2, "both are still listed: {rows:?}");
    assert_eq!(
        rows[0].conversation_id,
        Some(conversation),
        "the held one leads to the Conversation holding it",
    );
    assert_eq!(
        rows[1].conversation_id, None,
        "and the one beside it is still free",
    );
}

/// A number is a fact about one repository, so a Conversation holding `#41` in
/// one of them says nothing about `#41` in the next.
#[tokio::test]
async fn a_number_held_in_one_repo_leaves_the_same_number_free_in_another() {
    let (dir, pool, app) = app().await;

    let one = serde_json::Value::Array(vec![listed(
        41,
        "Rate limiting",
        "rate-limiting",
        "ada",
        false,
    )])
    .to_string();

    let verkstead = repo_answering(&pool, dir.path(), "verkstead", &one).await;
    repo_answering(&pool, dir.path(), "askance", &one).await;

    let conversation = store::start_conversation(&pool, verkstead.id, "rate-limiting", THIS_DEVICE)
        .await
        .unwrap()
        .unwrap();

    store::set_state(&pool, conversation, store::Lifecycle::Implementing)
        .await
        .unwrap();

    store::record_pull_request(
        &pool,
        conversation,
        verkstead.id,
        &store::PullRequest {
            number: 41,
            title: "Rate limiting".to_owned(),
            url: "https://github.com/tobico/verkstead/pull/41".to_owned(),
            repo: None,
        },
    )
    .await
    .unwrap();

    let open = open(&app).await;

    let held = open.iter().find(|group| group.repo == "verkstead").unwrap();
    let free = open.iter().find(|group| group.repo == "askance").unwrap();

    assert_eq!(held.pull_requests[0].conversation_id, Some(conversation));
    assert_eq!(free.pull_requests[0].conversation_id, None);
}

/// A Repo Verkstead cannot ask about is quiet rather than broken.
///
/// Three ways of not knowing, and one answer to all of them: no rows, no group
/// and a page that still draws. What Verkstead does not know is not an empty
/// list — but it is not a failure the human has to clear either, and a
/// repository they registered years ago and never gave a remote is not news.
#[tokio::test]
async fn a_repo_with_no_remote_and_one_whose_gh_will_not_answer_add_no_rows() {
    let (dir, pool, app) = app().await;

    repo_refusing(
        &pool,
        dir.path(),
        "no-remote",
        "none of the git remotes configured for this repository point to a known GitHub host",
    )
    .await;

    repo_refusing(
        &pool,
        dir.path(),
        "logged-out",
        "gh: To use GitHub CLI in a GitHub Actions workflow, run: gh auth login",
    )
    .await;

    // And one whose `gh` said nothing at all before failing, which is every
    // other way GitHub can be unreachable.
    let silent = dir.path().join("offline");
    std::fs::create_dir_all(&silent).unwrap();
    store::register_repo(&pool, &silent, "offline", "main")
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        open(&app).await,
        Vec::new(),
        "three Repos with nothing to say, and a list rather than a refusal",
    );
}

/// And a Repo with nothing open contributes no group, exactly as a Repo with no
/// roadmap to continue draws no notice: an empty group is a heading with nothing
/// under it.
#[tokio::test]
async fn a_repo_with_nothing_open_contributes_no_group() {
    let (dir, pool, app) = app().await;

    repo_answering(&pool, dir.path(), "quiet", "[]").await;

    assert_eq!(open(&app).await, Vec::new());
}

/// Where the golden fixtures are written, relative to this crate — the same
/// directory `ui_content`, `nudges` and `onboarding` write the other endpoints'
/// payloads to.
const FIXTURES: &str = "../../web/tests/fixtures";

/// Leave the viewer's component tests the payload this endpoint answers with,
/// exactly as this server writes one.
///
/// Committed, and rewritten by every run of this test: the diff is the review.
/// The level's own tests are fed from this rather than from a payload somebody
/// typed out, so a field added on this side that nobody carried across shows up
/// as a failing fixture rather than as a level drawing the wrong thing.
///
/// Two repositories, because the list is grouped by one and a single group
/// would not say that it is: one with a pull request nothing holds, one taken up
/// already, and a fork that is left out. And a third registration whose `gh`
/// will not answer, which is the shape of a Repo with nothing to say — no group
/// at all rather than an empty one.
#[tokio::test]
async fn the_viewers_own_tests_are_fed_from_here() {
    let (dir, pool, app) = app().await;

    let verkstead = repo_answering(
        &pool,
        dir.path(),
        "verkstead",
        &serde_json::Value::Array(vec![
            listed(
                41,
                "Rate limiting for the public API",
                "rate-limiting",
                "ada",
                false,
            ),
            listed(
                38,
                "Retry the flaky sandbox probe",
                "flaky-probe",
                "grace",
                false,
            ),
            listed(44, "Fix a typo in the README", "readme-typo", "linus", true),
        ])
        .to_string(),
    )
    .await;

    repo_answering(
        &pool,
        dir.path(),
        "askance",
        &serde_json::Value::Array(vec![listed_in(
            "askance",
            7,
            "Answer sheets on a phone",
            "phone-sheets",
            "ada",
            false,
        )])
        .to_string(),
    )
    .await;

    repo_refusing(
        &pool,
        dir.path(),
        "widgets",
        "none of the git remotes configured for this repository point to a known GitHub host",
    )
    .await;

    // The one already in the pipeline, which is the row that leads to a
    // Conversation rather than loading anything.
    let conversation = store::start_conversation(&pool, verkstead.id, "flaky-probe", THIS_DEVICE)
        .await
        .unwrap()
        .unwrap();

    store::set_state(&pool, conversation, store::Lifecycle::Implementing)
        .await
        .unwrap();

    store::record_pull_request(
        &pool,
        conversation,
        verkstead.id,
        &store::PullRequest {
            number: 38,
            title: "Retry the flaky sandbox probe".to_owned(),
            url: "https://github.com/tobico/verkstead/pull/38".to_owned(),
            repo: None,
        },
    )
    .await
    .unwrap();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/ui/open-pull-requests")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();

    write("open-pull-requests.json", &String::from_utf8_lossy(&bytes));
}

/// Write one fixture, indented so that a review of it is a review of the shape.
fn write(name: &str, json: &str) {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(FIXTURES);
    std::fs::create_dir_all(&dir).unwrap();

    let payload: serde_json::Value = serde_json::from_str(json).unwrap();
    let mut pretty = serde_json::to_string_pretty(&payload).unwrap();
    pretty.push('\n');

    std::fs::write(dir.join(name), pretty).unwrap();
}
