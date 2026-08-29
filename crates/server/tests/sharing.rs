//! What a share carries: the record a colleague is handed, and what never
//! leaves the workbench.
//!
//! Asked of `/api/ui/conversations/{id}/share.json`, which is the payload the
//! shared file is built around. The two are one composition — see
//! `crates/server/src/sharing.rs` — and this is the half that says what is in
//! it: which Events board, the sheet of every Set and the pane of every commit
//! that did, and that nothing arrives with an action still on it.
//!
//! The file itself is not fetched here, and that is deliberate rather than a
//! gap. It is the share build of the viewer with this payload written into a
//! slot, and the share build is `pnpm build`'s second output — which `cargo
//! test` does not wait on and CI does not run. So the composing is proved where
//! it can be, in that module's own unit tests over a template of the right
//! shape, and what proves the built document has nothing outside it is the build
//! itself: `web/vite.share.config.ts` refuses to write one that still points at
//! a file beside it.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde::de::DeserializeOwned;
use sqlx::SqlitePool;
use tower::ServiceExt;
use verkstead_render::{SharedConversation, TimelineEvent};
use verkstead_schema::QuestionSet;
use verkstead_server::{Gh, open_database, router, router_asking_github, store};

/// A router over a database with nothing in it, plus the pool and the directory
/// keeping it alive.
async fn app() -> (tempfile::TempDir, SqlitePool, Router) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    (dir, pool.clone(), router(pool))
}

/// The same, reaching GitHub through a `gh` that answers nothing and writes down
/// every call it was asked to make.
///
/// For the presses that write: what they are being asked here is whether GitHub
/// was reached at all, and a script that records what it was asked is what can
/// say so. The directory it writes in is the one the router keeps its settings
/// in, which holds no token — so nothing here could publish even if it tried.
async fn app_asking_github() -> (tempfile::TempDir, SqlitePool, Router) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    let gh = Gh::running(vec![
        "/bin/sh".to_owned(),
        "-c".to_owned(),
        format!(
            r#"printf '%s\n' "$*" >> "{}/asked"; exit 1"#,
            dir.path().display(),
        ),
        // `sh -c` gives `$0` the script's own name, so what Verkstead passes
        // lands in `$1` onwards.
        "gh".to_owned(),
    ]);

    let data_dir = dir.path().to_owned();

    (dir, pool.clone(), router_asking_github(pool, data_dir, gh))
}

/// A Conversation with a Brief and nothing else: no pull request, which is what
/// the press that comments on them has to have nothing to do about.
async fn drafting(pool: &SqlitePool) -> i64 {
    let repo = repo(pool).await;

    let id = store::start_conversation(pool, repo, "sharing")
        .await
        .unwrap()
        .unwrap();

    store::save_brief(pool, id, "# Sharing\n\nA file to send.\n")
        .await
        .unwrap();

    id
}

/// Press something, and read what came back.
async fn post<T: DeserializeOwned>(app: &Router, path: &str) -> T {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(path)
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK, "POST {path}");

    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

/// A registered Repo to attach Conversations to. A path nothing is at, because
/// nothing here reads a repository: what is under test is what the record says.
async fn repo(pool: &SqlitePool) -> i64 {
    store::register_repo(
        pool,
        std::path::Path::new("/srv/verkstead"),
        "verkstead",
        "main",
    )
    .await
    .unwrap()
    .unwrap()
    .id
}

async fn get<T: DeserializeOwned>(app: &Router, path: &str) -> T {
    let response = app
        .clone()
        .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK, "GET {path}");

    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

async fn share(app: &Router, id: i64) -> SharedConversation {
    get(app, &format!("/api/ui/conversations/{id}/share.json")).await
}

/// The smallest Set there is: one that asks nothing, which is enough to put a
/// Question Set Event on a Timeline.
fn asked(title: &str) -> QuestionSet {
    serde_saphyr::from_str(&format!(
        "title: {title}\nproject: verkstead\nbranch: sharing\nquestions: []\n"
    ))
    .unwrap()
}

/// What each Event on a Timeline is called, for the assertions below.
fn kind(event: &TimelineEvent) -> &'static str {
    match event {
        TimelineEvent::Brief(_) => "Brief",
        TimelineEvent::Moved(_) => "Moved",
        TimelineEvent::AgentOutput(_) => "AgentOutput",
        TimelineEvent::QuestionSet(_) => "QuestionSet",
        TimelineEvent::UnreadableSet(_) => "UnreadableSet",
        TimelineEvent::Handoff(_) => "Handoff",
        TimelineEvent::Commit(_) => "Commit",
        TimelineEvent::Notice(_) => "Notice",
        TimelineEvent::ManualTask(_) => "ManualTask",
        TimelineEvent::Steer(_) => "Steer",
        TimelineEvent::PullRequest(_) => "PullRequest",
        TimelineEvent::TaskList(_) => "TaskList",
        TimelineEvent::StageList(_) => "StageList",
    }
}

/// A Conversation carrying one of everything a Timeline can hold, so that what
/// a share leaves out is left out of something that was really there.
///
/// Walked through the states rather than written straight in, because half the
/// kinds are written by moving: a grilling that starts writes the move and
/// freezes the Brief, and a pull request needs a Conversation that has got as
/// far as wrapping up.
async fn everything(pool: &SqlitePool) -> i64 {
    let repo = repo(pool).await;

    let id = store::start_conversation(pool, repo, "sharing")
        .await
        .unwrap()
        .unwrap();

    store::save_brief(pool, id, "# Sharing a conversation\n\nA file to send.\n")
        .await
        .unwrap();

    // A session's output, which is the biggest thing on a Timeline and the
    // first thing a share leaves behind.
    store::start_capture(pool, id, Some("session-1"))
        .await
        .unwrap();

    store::ask(
        pool,
        id,
        &asked("Where should the counter live?"),
        store::Ask::Blocking,
    )
    .await
    .unwrap()
    .unwrap();

    store::start_grilling(
        pool,
        id,
        "6f32b11a0c4d1e8f5b3a97c2d0e4f6a8b1c3d5e7",
        std::path::Path::new("/var/lib/verkstead/worktrees/verkstead-sharing"),
        &[],
    )
    .await
    .unwrap();

    store::record_handoff(pool, id, "# Handoff\n\nWhat the grilling settled.\n")
        .await
        .unwrap();

    store::pick_direction(pool, id, verkstead_schema::Direction::TaskList)
        .await
        .unwrap();
    store::record_backlog(pool, id).await.unwrap();
    store::start_implementing(pool, id).await.unwrap();

    store::record_commit(
        pool,
        id,
        repo,
        &store::Commit {
            sha: "d41f8a3b6c2e91750f4a8c3d5b7e2f10a9c6d4b8".to_owned(),
            subject: "feat: share a conversation as one file".to_owned(),
            files: 7,
            insertions: 412,
            deletions: 3,
            summary: Some("The record travels as the viewer built to one file.".to_owned()),
            repo: None,
        },
    )
    .await
    .unwrap()
    .unwrap();

    store::record_pull_request(
        pool,
        id,
        repo,
        &store::PullRequest {
            number: 56,
            title: "Conversation sharing".to_owned(),
            url: "https://github.com/tobico/verkstead/pull/56".to_owned(),
            repo: None,
        },
    )
    .await
    .unwrap();

    // Verkstead talking on its own account, which is what a stop is read
    // through — and the badge that points at it, which a share must not carry
    // either.
    store::stop(
        pool,
        id,
        store::Decision::Verkstead,
        "**The wrap-up** stopped.\n\nthe session exited with status 1\n",
        None,
    )
    .await
    .unwrap()
    .unwrap();

    // And the human sending it somewhere, which is the one Event that stands
    // beside a move: the pair is the whole record of a steer.
    store::steer_conversation(
        pool,
        id,
        verkstead_store::Steer {
            target: store::Lifecycle::Implementing,
            pairings: &[],
            brief: None,
            instruction: Some("Take the diffs out of the bundle and see what it weighs."),
            direction: None,
            worktree: None,
            base_commit: None,
            companions: &[],
            opened: &[],
            checkouts: &[],
            said: None,
        },
    )
    .await
    .unwrap();

    id
}

#[tokio::test]
async fn a_share_carries_what_was_asked_answered_and_built() {
    let (_dir, pool, app) = app().await;
    let id = everything(&pool).await;

    // Every kind is really on the record, or the omissions below prove nothing.
    let whole: verkstead_render::ConversationView =
        get(&app, &format!("/api/ui/conversations/{id}")).await;
    let held: Vec<&str> = whole.timeline.iter().map(kind).collect();

    for expected in [
        "Brief",
        "Moved",
        "AgentOutput",
        "QuestionSet",
        "Handoff",
        "Commit",
        "Notice",
        "PullRequest",
        "TaskList",
        "Steer",
    ] {
        assert!(
            held.contains(&expected),
            "the record has no {expected}: {held:?}"
        );
    }

    let boarded: Vec<&str> = share(&app, id)
        .await
        .conversation
        .timeline
        .iter()
        .map(kind)
        .collect();

    // What a share is for: the brief, the asks, the commits, the human's own
    // steers, and the lifecycle lines that say where the work went.
    for expected in ["Brief", "QuestionSet", "Commit", "Steer", "Moved"] {
        assert!(
            boarded.contains(&expected),
            "a share left out {expected}: {boarded:?}"
        );
    }

    // And what is nobody else's to read, left out with no mark where it was: a
    // share is a curated record rather than a record with holes cut in it.
    for left in [
        "AgentOutput",
        "Handoff",
        "Notice",
        "PullRequest",
        "TaskList",
    ] {
        assert!(
            !boarded.contains(&left),
            "a share carried {left}: {boarded:?}"
        );
    }
}

#[tokio::test]
async fn a_share_has_nothing_left_on_it_to_act_on() {
    let (_dir, pool, app) = app().await;
    let id = everything(&pool).await;

    let shared = share(&app, id).await;
    let conversation = shared.conversation;

    // Nothing pinned: every pinned card is the current state of something the
    // work is against, and a reader has neither the worktree nor the pull
    // request behind any of them.
    assert!(conversation.pinned.is_empty(), "a share pinned something");

    // Nothing to press, in any of the ways the workbench decides there is
    // something to press.
    assert!(!conversation.ready_to_grill);
    assert!(!conversation.ready_to_resume);
    assert!(!conversation.ready_to_stop);
    assert!(!conversation.stop_asked);
    assert!(!conversation.ready_to_continue);
    assert!(!conversation.compiles_uncached);
    assert!(conversation.adopting.is_none());

    // And nothing that says something is happening right now, which in a file
    // is never true.
    assert!(!conversation.working);
    assert!(!conversation.driven);

    // And no mark pointing at a stop: the Notice it would open does not board,
    // and *blocked on you* said to somebody who cannot act is a mark asking the
    // wrong person.
    assert!(conversation.blocked_on.is_none());
    assert!(!conversation.stopped_by_hand);
    assert!(!conversation.waiting_on_checks);
    assert!(conversation.resets.is_none());

    // The record itself is untouched: what the work is, where it got to, and
    // which repository it was done in.
    assert_eq!(conversation.branch, "sharing");
    assert_eq!(conversation.repo.name, "verkstead");
    assert_eq!(
        conversation.state,
        verkstead_render::Lifecycle::Implementing
    );

    // And the moment it was taken, which is what makes it a snapshot rather
    // than a window.
    assert!(
        shared.exported_at.starts_with("20"),
        "a share is stamped: {}",
        shared.exported_at
    );
}

#[tokio::test]
async fn a_drafts_brief_boards_as_the_document_it_is() {
    let (_dir, pool, app) = app().await;
    let repo = repo(&pool).await;

    let id = store::start_conversation(&pool, repo, "still-drafting")
        .await
        .unwrap()
        .unwrap();
    store::save_brief(&pool, id, "# Still being written\n")
        .await
        .unwrap();

    // On the workbench this Brief is a field with the conversation's setup
    // under it, because nothing has frozen it yet.
    let whole: verkstead_render::ConversationView =
        get(&app, &format!("/api/ui/conversations/{id}")).await;
    assert!(
        whole.timeline.iter().any(|event| matches!(
            event,
            TimelineEvent::Brief(brief) if !brief.frozen
        )),
        "the drafting Brief should be open on the workbench"
    );

    // In a share it is the document it will be read as for the rest of the
    // record's life — there is no field to type into with no server behind it,
    // and no setup for anybody to settle.
    let shared = share(&app, id).await;
    assert!(
        shared.conversation.timeline.iter().any(|event| matches!(
            event,
            TimelineEvent::Brief(brief) if brief.frozen
        )),
        "a shared Brief should have frozen"
    );
}

#[tokio::test]
async fn a_conversation_that_is_gone_has_no_share() {
    let (_dir, _pool, app) = app().await;

    for path in [
        "/api/ui/conversations/404/share.json",
        "/api/ui/conversations/no-such-thing/share.json",
    ] {
        let response = app
            .clone()
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND, "GET {path}");
    }
}

/// A Set with something of every part a sheet is drawn from: a Preface, a
/// Question with Options and a recommendation, a Diff, and a Postscript. Enough
/// that a share carrying the whole sheet can be told from one carrying the
/// Timeline's table of it.
fn whole_set(preface: &str) -> QuestionSet {
    let mut set = asked("Where should the counter live?");

    set.preface = Some(preface.to_owned());
    set.postscript = Some("Happy to talk any of this through.".to_owned());
    set.questions = vec![verkstead_schema::Question {
        label: "Q1".to_owned(),
        text: "Where should the counter live?".to_owned(),
        columns: Vec::new(),
        options: vec![
            verkstead_schema::QuestionOption {
                n: 1,
                text: "In the process, as it is now.".to_owned(),
                recommended: false,
                cells: Vec::new(),
            },
            verkstead_schema::QuestionOption {
                n: 2,
                text: "In **Redis**, shared across instances.".to_owned(),
                recommended: true,
                cells: Vec::new(),
            },
        ],
        subquestions: Vec::new(),
    }];
    set.diffs = vec![verkstead_schema::RepoDiff {
        repo: "verkstead".to_owned(),
        own: true,
        diff: "diff --git a/limiter.rs b/limiter.rs\n\
               --- a/limiter.rs\n\
               +++ b/limiter.rs\n\
               @@ -1,2 +1,2 @@\n\
               -let count = 0;\n\
               +let count = redis.get(key);\n"
            .to_owned(),
    }];

    set
}

/// A Conversation with one Set on it, asked and unanswered.
async fn asking(pool: &SqlitePool, repo: i64, branch: &str, set: &QuestionSet) -> (i64, i64) {
    let id = store::start_conversation(pool, repo, branch)
        .await
        .unwrap()
        .unwrap();

    let asked = store::ask(pool, id, set, store::Ask::Blocking)
        .await
        .unwrap()
        .unwrap();

    (id, asked.id)
}

#[tokio::test]
async fn a_share_carries_the_whole_sheet_of_every_set_on_it() {
    let (_dir, pool, app) = app().await;
    let repo = repo(&pool).await;
    let (id, set_id) = asking(
        &pool,
        repo,
        "sharing",
        &whole_set("What the counter is for."),
    )
    .await;

    let shared = share(&app, id).await;

    assert_eq!(
        shared.sets.len(),
        1,
        "a share should carry one sheet per Set on its Timeline"
    );

    let sheet = &shared.sets[0];
    assert_eq!(sheet.id, set_id, "the sheet is the Set the row opens");

    // Everything the workbench draws a Set from, rendered the same way — this is
    // the endpoint's own reading rather than a second one.
    assert!(
        sheet
            .preface_html
            .as_deref()
            .is_some_and(|html| html.contains("What the counter is for.")),
        "the preface: {:?}",
        sheet.preface_html
    );
    assert!(
        sheet
            .postscript_html
            .as_deref()
            .is_some_and(|html| html.contains("talk any of this through")),
        "the postscript: {:?}",
        sheet.postscript_html
    );

    // The Questions with their Options, and which one the agent recommended —
    // what was turned down is half of what a decision was.
    let question = &sheet.questions[0];
    assert_eq!(question.ask.options.len(), 2);
    assert!(question.ask.options[1].recommended);
    assert!(question.ask.options[1].text_html.contains("<strong>Redis"));

    // And the worktree Diff the Set was decided against, which is what the
    // human approved over.
    assert_eq!(sheet.diff.len(), 1, "the Set's Diff should board with it");
    assert!(
        sheet.diff[0]
            .diff
            .paths
            .iter()
            .any(|path| path.contains("limiter"))
    );
}

#[tokio::test]
async fn a_set_still_waiting_boards_as_the_record_it_was() {
    let (_dir, pool, app) = app().await;
    let repo = repo(&pool).await;
    let (id, _) = asking(&pool, repo, "sharing", &whole_set("Still out.")).await;

    // Nothing has answered it, and the share is taken anyway: a Conversation is
    // shared while it is going on as often as after it.
    let shared = share(&app, id).await;

    assert!(
        matches!(
            shared.sets[0].standing,
            verkstead_render::Standing::Waiting(_)
        ),
        "how a Set stood is part of the record: {:?}",
        shared.sets[0].standing
    );
}

#[tokio::test]
async fn a_share_says_whether_it_needs_the_diagram_renderer() {
    let (_dir, pool, app) = app().await;

    // A Conversation whose Set has a Diagram in its Preface, and one whose Set
    // has only prose. Mermaid is three megabytes, so which of the two this is
    // decides whether it rides in the file at all.
    let repo = repo(&pool).await;
    let (drawn, _) = asking(
        &pool,
        repo,
        "drawn",
        &whole_set("What it does:\n\n```mermaid\nflowchart LR\n  a --> b\n```\n"),
    )
    .await;
    let (written, _) = asking(
        &pool,
        repo,
        "written",
        &whole_set("What it does, in words."),
    )
    .await;

    assert!(
        share(&app, drawn).await.sets[0].diagrams,
        "a Set with a Diagram should say so"
    );
    assert!(
        !share(&app, written).await.sets[0].diagrams,
        "a Set with no Diagram should not"
    );
}

#[tokio::test]
async fn a_commit_that_drew_the_delta_says_the_share_needs_the_renderer_too() {
    let (dir, pool, app) = app().await;

    let path = repository(dir.path().join("verkstead"));
    let repo = store::register_repo(&pool, &path, "verkstead", "main")
        .await
        .unwrap()
        .unwrap()
        .id;

    let id = store::start_conversation(&pool, repo, "sharing")
        .await
        .unwrap()
        .unwrap();

    // Nothing was asked here, so nothing but the commit can be what puts the
    // renderer in the file — which is the case worth having, agents drawing the
    // delta in a Commit Summary as often as in a Preface.
    let sha = commit(
        &path,
        "feat: draw it",
        &[("limiter.rs", "fn window() {}\n")],
    );

    store::record_commit(
        &pool,
        id,
        repo,
        &store::Commit {
            sha,
            subject: "feat: draw it".to_owned(),
            files: 1,
            insertions: 1,
            deletions: 0,
            summary: Some("What it does:\n\n```mermaid\nflowchart LR\n  a --> b\n```\n".to_owned()),
            repo: None,
        },
    )
    .await
    .unwrap()
    .unwrap();

    let shared = share(&app, id).await;

    assert!(shared.sets.is_empty(), "nothing was asked on this one");
    assert!(
        shared.commits[0].pane.diagrams,
        "a Commit Summary with a Diagram should say so"
    );
}

#[tokio::test]
async fn a_set_this_build_cannot_read_boards_neither_row_nor_sheet() {
    let (_dir, pool, app) = app().await;
    let repo = repo(&pool).await;

    let id = store::start_conversation(&pool, repo, "sharing")
        .await
        .unwrap()
        .unwrap();

    store::ask(&pool, id, &asked("Readable"), store::Ask::Blocking)
        .await
        .unwrap()
        .unwrap();

    // A stored body of a shape this build's schema will not take, which is what
    // an older Verkstead's Set looks like to a newer one.
    let unreadable = store::ask(&pool, id, &asked("Unreadable"), store::Ask::Blocking)
        .await
        .unwrap()
        .unwrap();
    // The real shape of one, rather than a made-up field: the `proposal` block
    // shrank when the direction chooser moved onto the Set, and
    // `deny_unknown_fields` means every Set stored before that is a body this
    // build will not take.
    sqlx::query("UPDATE question_sets SET body = ? WHERE id = ?")
        .bind(
            "{\"title\": \"Unreadable\", \"questions\": [], \"proposal\": \n\
             {\"direction\": \"task-list\", \"rationale\": \"Six changes.\", \n\
             \"accepted_by\": \"Q1.1\"}}",
        )
        .bind(unreadable.id)
        .execute(&pool)
        .await
        .unwrap();

    // It is on the workbench's Timeline as a row of its own — the ask happened.
    let whole: verkstead_render::ConversationView =
        get(&app, &format!("/api/ui/conversations/{id}")).await;
    assert!(
        whole
            .timeline
            .iter()
            .any(|event| matches!(event, TimelineEvent::UnreadableSet(_))),
        "the unreadable Set should be on the workbench's Timeline"
    );

    // And in a share it is neither a row nor a sheet: a share carries the record
    // its reader can read, and there is nothing here to draw.
    let shared = share(&app, id).await;
    assert_eq!(
        shared.sets.len(),
        1,
        "only the readable Set should have a sheet"
    );
    assert!(
        !shared
            .conversation
            .timeline
            .iter()
            .any(|event| matches!(event, TimelineEvent::UnreadableSet(_))),
        "an unreadable Set should not board"
    );
}

/// A git repository at `path` with one commit on `main`, and the tools to put
/// more on it.
///
/// A real one, because a commit's diff is not in the store: it is read out of
/// git at export time, which is the thing under test below.
fn repository(path: PathBuf) -> PathBuf {
    std::fs::create_dir_all(&path).unwrap();
    git(&path, &["init", "--initial-branch", "main"]);
    git(&path, &["config", "user.email", "tests@verkstead.invalid"]);
    git(&path, &["config", "user.name", "Verkstead Tests"]);
    git(&path, &["config", "commit.gpgsign", "false"]);
    std::fs::write(path.join("README.md"), "# a repository\n").unwrap();
    git(&path, &["add", "README.md"]);
    git(&path, &["commit", "-m", "first"]);

    path
}

/// One commit on top of whatever is there: the files written, and the hash git
/// gave it.
fn commit(path: &Path, subject: &str, files: &[(&str, &str)]) -> String {
    for (name, contents) in files {
        let file = path.join(name);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, contents).unwrap();
    }

    git(path, &["add", "-A"]);
    git(path, &["commit", "-m", subject]);
    git(path, &["rev-parse", "HEAD"]).trim().to_owned()
}

/// Run git in `dir`, insisting it worked. Scaffolding rather than the code under
/// test, so a failure here is a broken test.
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

/// The pane a share carries for one commit.
fn pane<'a>(shared: &'a SharedConversation, subject: &str) -> &'a verkstead_render::SharedCommit {
    let event = shared
        .conversation
        .timeline
        .iter()
        .find_map(|event| match event {
            TimelineEvent::Commit(commit) if commit.subject == subject => Some(commit.id),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no commit called {subject} on the Timeline"));

    shared
        .commits
        .iter()
        .find(|commit| commit.id == event)
        .unwrap_or_else(|| panic!("no pane for the commit called {subject}"))
}

#[tokio::test]
async fn a_share_carries_the_whole_diff_of_every_commit_on_it() {
    let (dir, pool, app) = app().await;

    let path = repository(dir.path().join("verkstead"));
    let repo = store::register_repo(&pool, &path, "verkstead", "main")
        .await
        .unwrap()
        .unwrap()
        .id;

    let id = store::start_conversation(&pool, repo, "sharing")
        .await
        .unwrap()
        .unwrap();

    // A branch of some size, because that is where a share is asked to do
    // something a page never is: every commit's diff at once. Twelve commits of
    // four files each is enough that a cap anybody was tempted to put in would
    // show up as something missing below.
    let mut written = Vec::new();

    for n in 1..=12 {
        let files: Vec<(String, String)> = (1..=4)
            .map(|file| {
                (
                    format!("crates/limiter/step-{n}/file-{file}.rs"),
                    format!("// commit {n}, file {file}\nfn counted() -> usize {{ {n} }}\n"),
                )
            })
            .collect();

        let subject = format!("feat: step {n} of the limiter");
        let sha = commit(
            &path,
            &subject,
            &files
                .iter()
                .map(|(name, contents)| (name.as_str(), contents.as_str()))
                .collect::<Vec<_>>(),
        );

        store::record_commit(
            &pool,
            id,
            repo,
            &store::Commit {
                sha: sha.clone(),
                subject: subject.clone(),
                files: 4,
                insertions: 8,
                deletions: 0,
                summary: Some(format!(
                    "Step {n}.\n\n```mermaid\nflowchart LR\n  a --> b\n```\n"
                )),
                repo: None,
            },
        )
        .await
        .unwrap()
        .unwrap();

        written.push(subject);
    }

    let shared = share(&app, id).await;

    assert_eq!(
        shared.commits.len(),
        written.len(),
        "every commit on the Timeline should have a pane"
    );

    for subject in &written {
        let carried = pane(&shared, subject);

        assert!(carried.held, "{subject} should have been read out of git");

        // The Commit Summary, rendered as the workbench renders one — and the
        // Diagram in it flagged, which is what says the file needs the renderer.
        let summary = carried.pane.summary.as_deref().unwrap_or_default();
        assert!(
            summary.contains("<p>"),
            "the summary of {subject}: {summary}"
        );
        assert!(
            carried.pane.diagrams,
            "the Diagram in {subject} should be flagged"
        );

        // And the whole diff: four files, each with its own fold, and nothing
        // cut off at a size.
        let diff = carried
            .pane
            .diff
            .as_ref()
            .unwrap_or_else(|| panic!("{subject} carried no diff"));

        assert_eq!(
            diff.paths.len(),
            4,
            "every file of {subject} should be in it"
        );

        // The lines themselves, which is what "no cap" means — asked for by a
        // word rather than by a line, because the highlighter has been through
        // it and every token in a line is inside a span of its own.
        assert!(
            diff.html.contains("counted"),
            "the lines of {subject} should be in it"
        );
    }
}

#[tokio::test]
async fn a_companions_commit_carries_its_own_repositorys_diff() {
    let (dir, pool, app) = app().await;

    let own = repository(dir.path().join("verkstead"));
    let alongside = repository(dir.path().join("askance"));

    let repo = store::register_repo(&pool, &own, "verkstead", "main")
        .await
        .unwrap()
        .unwrap()
        .id;
    let companion = store::register_repo(&pool, &alongside, "askance", "main")
        .await
        .unwrap()
        .unwrap()
        .id;

    let id = store::start_conversation(&pool, repo, "sharing")
        .await
        .unwrap()
        .unwrap();
    store::add_companion(&pool, id, companion).await.unwrap();

    let here = commit(
        &own,
        "feat: count the window",
        &[("limiter.rs", "fn window() {}\n")],
    );
    let there = commit(
        &alongside,
        "feat: name the account",
        &[("account.rs", "fn named() {}\n")],
    );

    for (repo, sha, subject) in [
        (repo, &here, "feat: count the window"),
        (companion, &there, "feat: name the account"),
    ] {
        store::record_commit(
            &pool,
            id,
            repo,
            &store::Commit {
                sha: sha.clone(),
                subject: subject.to_owned(),
                files: 1,
                insertions: 1,
                deletions: 0,
                summary: None,
                repo: None,
            },
        )
        .await
        .unwrap()
        .unwrap();
    }

    let shared = share(&app, id).await;

    // Each diff comes out of the repository its commit landed in, which is the
    // whole of why a companion's is worth carrying: the Conversation's own
    // repository knows nothing about it.
    let ours = pane(&shared, "feat: count the window");
    assert_eq!(ours.pane.diff.as_ref().unwrap().paths, vec!["limiter.rs"]);

    let theirs = pane(&shared, "feat: name the account");
    assert_eq!(theirs.pane.diff.as_ref().unwrap().paths, vec!["account.rs"]);

    // And which of the two a card is labelled by, which is the Timeline's own
    // answer and unchanged by any of this: the companion's is named and the
    // work's own is not.
    let labels: Vec<Option<String>> = shared
        .conversation
        .timeline
        .iter()
        .filter_map(|event| match event {
            TimelineEvent::Commit(commit) => Some(commit.repo.clone()),
            _ => None,
        })
        .collect();

    assert_eq!(labels, vec![None, Some("askance".to_owned())]);
}

#[tokio::test]
async fn a_commit_the_repository_has_lost_says_so_rather_than_stopping_the_export() {
    let (dir, pool, app) = app().await;

    let path = repository(dir.path().join("verkstead"));
    let repo = store::register_repo(&pool, &path, "verkstead", "main")
        .await
        .unwrap()
        .unwrap()
        .id;

    let id = store::start_conversation(&pool, repo, "sharing")
        .await
        .unwrap()
        .unwrap();

    let held = commit(&path, "feat: still here", &[("here.rs", "fn here() {}\n")]);

    for (sha, subject, summary) in [
        (held, "feat: still here", None),
        (
            // A hash the repository has never held, which is what a commit
            // rebased away or collected looks like by the time somebody shares.
            "0123456789abcdef0123456789abcdef01234567".to_owned(),
            "feat: rebased away",
            Some("What it did before somebody rewrote the branch."),
        ),
    ] {
        store::record_commit(
            &pool,
            id,
            repo,
            &store::Commit {
                sha,
                subject: subject.to_owned(),
                files: 1,
                insertions: 1,
                deletions: 0,
                summary: summary.map(ToOwned::to_owned),
                repo: None,
            },
        )
        .await
        .unwrap()
        .unwrap();
    }

    // The export happens: one commit nobody can read is no reason to refuse the
    // record it is part of.
    let shared = share(&app, id).await;
    assert_eq!(shared.commits.len(), 2);

    let here = pane(&shared, "feat: still here");
    assert!(here.held);
    assert!(here.pane.diff.is_some());

    let gone = pane(&shared, "feat: rebased away");
    assert!(!gone.held, "a commit git has lost should say so");
    assert!(gone.pane.diff.is_none());

    // And what the store kept about it travels regardless: the Commit Summary
    // was written down when the commit was recorded rather than read back out
    // of git, so its own account of itself is still there to read.
    assert!(
        gone.pane
            .summary
            .as_deref()
            .is_some_and(|html| html.contains("somebody rewrote the branch")),
        "the summary of a lost commit: {:?}",
        gone.pane.summary
    );
}

/// Where a share was published is the workbench's fact about a Conversation,
/// drawn beside the Share row so the human can send the same link twice without
/// publishing twice.
#[tokio::test]
async fn a_published_share_is_on_the_conversation_the_workbench_draws() {
    let (_dir, pool, app) = app().await;
    let id = everything(&pool).await;

    let before: verkstead_render::ConversationView =
        get(&app, &format!("/api/ui/conversations/{id}")).await;

    assert_eq!(
        before.shared, None,
        "a Conversation nobody has published one of has no link",
    );

    store::record_share(&pool, id, "https://gist.github.com/tobico/9f1")
        .await
        .unwrap();

    let after: verkstead_render::ConversationView =
        get(&app, &format!("/api/ui/conversations/{id}")).await;
    let shared = after.shared.expect("the published share");

    assert_eq!(shared.url, "https://gist.github.com/tobico/9f1");
    assert!(
        shared.at.starts_with("20"),
        "an RFC 3339 stamp, not {:?}",
        shared.at,
    );
}

/// Publishing again is a fresh snapshot, so the record holds where to send
/// somebody *now* — one link at a time rather than a history of them. What was
/// already sent goes on standing at its own URL; this is only what the next
/// comment is written from.
#[tokio::test]
async fn publishing_again_replaces_the_link_the_workbench_draws() {
    let (_dir, pool, app) = app().await;
    let id = everything(&pool).await;

    store::record_share(&pool, id, "https://gist.github.com/tobico/9f1")
        .await
        .unwrap();
    store::record_share(&pool, id, "https://gist.github.com/tobico/a20")
        .await
        .unwrap();

    let whole: verkstead_render::ConversationView =
        get(&app, &format!("/api/ui/conversations/{id}")).await;

    assert_eq!(
        whole.shared.map(|shared| shared.url),
        Some("https://gist.github.com/tobico/a20".to_owned()),
    );
}

/// And it stays in the workbench: a share carrying the link to another share is
/// handing on a URL nobody meant to give the reader.
#[tokio::test]
async fn a_share_does_not_carry_the_link_to_a_share() {
    let (_dir, pool, app) = app().await;
    let id = everything(&pool).await;

    store::record_share(&pool, id, "https://gist.github.com/tobico/9f1")
        .await
        .unwrap();

    assert_eq!(share(&app, id).await.conversation.shared, None);
}

/// The one-click share, asked of a Conversation whose work is on no pull
/// request.
///
/// Nothing happens, and *nothing* is the point: the press is offered only where
/// the record holds a pull request, so reaching this is a page drawn against a
/// Conversation that has since moved — and a share published for nobody would
/// be a gist left in somebody's account for nothing. The pull requests are read
/// before the file is built, which is what makes that true.
#[tokio::test]
async fn a_conversation_on_no_pull_request_is_not_published_at_all() {
    let (dir, pool, app) = app_asking_github().await;
    let id = drafting(&pool).await;

    let outcome: verkstead_render::ShareCommented =
        post(&app, &format!("/api/ui/conversations/{id}/share/comment")).await;

    assert_eq!(outcome, verkstead_render::ShareCommented::NoPullRequest);

    assert!(
        !dir.path().join("asked").exists(),
        "GitHub was asked something: {:?}",
        std::fs::read_to_string(dir.path().join("asked")).ok(),
    );
}

/// And an id that names no Conversation is the same 404 every other reading of
/// one is: there is nothing to compose a share of, let alone comment.
#[tokio::test]
async fn a_conversation_that_is_not_there_is_a_miss() {
    let (_dir, _pool, app) = app_asking_github().await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/ui/conversations/4821/share/comment")
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
