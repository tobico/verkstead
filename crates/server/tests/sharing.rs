//! What a share carries: the record a colleague is handed, and what never
//! leaves the workbench.
//!
//! Asked of `/api/ui/conversations/{id}/share.json`, which is the payload the
//! shared file is built around. The two are one composition — see
//! `crates/server/src/sharing.rs` — and this is the half that says what is in
//! it: which Events board, and that nothing arrives with an action still on it.
//!
//! The file itself is not fetched here, and that is deliberate rather than a
//! gap. It is the share build of the viewer with this payload written into a
//! slot, and the share build is `pnpm build`'s second output — which `cargo
//! test` does not wait on and CI does not run. So the composing is proved where
//! it can be, in that module's own unit tests over a template of the right
//! shape, and what proves the built document has nothing outside it is the build
//! itself: `web/vite.share.config.ts` refuses to write one that still points at
//! a file beside it.

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde::de::DeserializeOwned;
use sqlx::SqlitePool;
use tower::ServiceExt;
use verkstead_render::{SharedConversation, TimelineEvent};
use verkstead_schema::QuestionSet;
use verkstead_server::{open_database, router, store};

/// A router over a database with nothing in it, plus the pool and the directory
/// keeping it alive.
async fn app() -> (tempfile::TempDir, SqlitePool, Router) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    (dir, pool.clone(), router(pool))
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
