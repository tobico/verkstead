//! Conversations: what starting one records, what a drafting one is still the
//! human's to change, and that all of it is still there after a restart.

use std::path::Path;

use sqlx::SqlitePool;
use verkstead_store::{
    Edited, Event, Lifecycle, conversations, load_conversation, open_database, register_repo,
    rename_branch, save_brief, set_base_commit, set_state, start_conversation, timeline,
};

/// A pool over a fresh database, plus the directory keeping it alive.
async fn fresh_pool() -> (tempfile::TempDir, SqlitePool) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();
    (dir, pool)
}

/// A registered Repo to hang a Conversation off, by name.
async fn repo(pool: &SqlitePool, name: &str) -> i64 {
    register_repo(pool, &Path::new("/watched").join(name), name, "main")
        .await
        .unwrap()
        .expect("nothing was registered at that path yet")
        .id
}

/// The markdown of a Conversation's Brief, read back off its Timeline.
async fn brief(pool: &SqlitePool, id: i64) -> String {
    let events = timeline(pool, id).await.unwrap();
    assert_eq!(events.len(), 1, "the Brief should be the only Event yet");

    let Event::Brief(markdown) = &events[0].event;
    markdown.clone()
}

#[tokio::test]
async fn a_started_conversation_holds_its_repo_its_branch_and_the_default_rule() {
    let (_dir, pool) = fresh_pool().await;
    let repo_id = repo(&pool, "verkstead").await;

    let id = start_conversation(&pool, repo_id, "amber-kestrel")
        .await
        .unwrap()
        .expect("the Repo is registered, so the Conversation should start");

    let conversation = load_conversation(&pool, id).await.unwrap().unwrap();
    assert_eq!(conversation.repo.id, repo_id);
    assert_eq!(conversation.repo.name, "verkstead");
    assert_eq!(conversation.branch, "amber-kestrel");
    assert_eq!(conversation.state, Lifecycle::Draft);

    // Not a missing value: no override means the default branch's tip at grill
    // start, which is a rule to resolve then rather than a commit to record now.
    assert_eq!(conversation.base_commit, None);
}

/// The Brief is the first Event from the moment there is a Conversation, empty
/// or not — it is what the human writes into, so it cannot wait to exist until
/// they have.
#[tokio::test]
async fn a_started_conversation_has_an_empty_brief_on_its_timeline() {
    let (_dir, pool) = fresh_pool().await;
    let repo_id = repo(&pool, "verkstead").await;

    let id = start_conversation(&pool, repo_id, "amber-kestrel")
        .await
        .unwrap()
        .unwrap();

    assert_eq!(brief(&pool, id).await, "");
}

#[tokio::test]
async fn a_conversation_cannot_be_started_against_a_repo_that_is_not_registered() {
    let (_dir, pool) = fresh_pool().await;

    assert!(
        start_conversation(&pool, 404, "amber-kestrel")
            .await
            .unwrap()
            .is_none()
    );
    assert!(conversations(&pool).await.unwrap().is_empty());
}

#[tokio::test]
async fn conversations_are_listed_newest_first_with_the_repo_they_are_against() {
    let (_dir, pool) = fresh_pool().await;
    let verkstead = repo(&pool, "verkstead").await;
    let askance = repo(&pool, "askance").await;

    start_conversation(&pool, verkstead, "amber-kestrel")
        .await
        .unwrap()
        .unwrap();
    start_conversation(&pool, askance, "quiet-harbour")
        .await
        .unwrap()
        .unwrap();

    let listed: Vec<(String, String)> = conversations(&pool)
        .await
        .unwrap()
        .into_iter()
        .map(|row| (row.branch, row.repo))
        .collect();

    assert_eq!(
        listed,
        [
            ("quiet-harbour".to_owned(), "askance".to_owned()),
            ("amber-kestrel".to_owned(), "verkstead".to_owned()),
        ]
    );
}

#[tokio::test]
async fn a_brief_is_rewritten_in_place_rather_than_added_to() {
    let (_dir, pool) = fresh_pool().await;
    let repo_id = repo(&pool, "verkstead").await;
    let id = start_conversation(&pool, repo_id, "amber-kestrel")
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        save_brief(&pool, id, "# Rate limiting\n\nThe API has none.\n")
            .await
            .unwrap(),
        Edited::Saved
    );
    assert_eq!(
        save_brief(&pool, id, "# Rate limiting\n\nStill none.\n")
            .await
            .unwrap(),
        Edited::Saved
    );

    assert_eq!(
        brief(&pool, id).await,
        "# Rate limiting\n\nStill none.\n",
        "one Brief, holding what was last written"
    );
}

#[tokio::test]
async fn a_drafting_conversations_branch_and_base_commit_are_the_humans_to_change() {
    let (_dir, pool) = fresh_pool().await;
    let repo_id = repo(&pool, "verkstead").await;
    let id = start_conversation(&pool, repo_id, "amber-kestrel")
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        rename_branch(&pool, id, "rate-limiting").await.unwrap(),
        Edited::Saved
    );
    assert_eq!(
        set_base_commit(&pool, id, Some("6f32b11")).await.unwrap(),
        Edited::Saved
    );

    let conversation = load_conversation(&pool, id).await.unwrap().unwrap();
    assert_eq!(conversation.branch, "rate-limiting");
    assert_eq!(conversation.base_commit.as_deref(), Some("6f32b11"));
}

/// Taking the override away puts the Conversation back on the rule, rather than
/// leaving a commit nobody chose behind.
#[tokio::test]
async fn clearing_the_base_commit_restores_the_default_branch_rule() {
    let (_dir, pool) = fresh_pool().await;
    let repo_id = repo(&pool, "verkstead").await;
    let id = start_conversation(&pool, repo_id, "amber-kestrel")
        .await
        .unwrap()
        .unwrap();

    set_base_commit(&pool, id, Some("6f32b11")).await.unwrap();
    assert_eq!(
        set_base_commit(&pool, id, None).await.unwrap(),
        Edited::Saved
    );

    assert_eq!(
        load_conversation(&pool, id).await.unwrap().unwrap().base_commit,
        None
    );
}

/// The freeze the design states, keeping its half of the bargain from the start:
/// once a Conversation is past drafting, none of the three is the human's any
/// more. Nothing in this stage moves one on, so the state is written here
/// directly — what is being asked is what the guard does when it is.
#[tokio::test]
async fn nothing_about_a_conversation_past_drafting_can_be_edited() {
    let (_dir, pool) = fresh_pool().await;
    let repo_id = repo(&pool, "verkstead").await;
    let id = start_conversation(&pool, repo_id, "amber-kestrel")
        .await
        .unwrap()
        .unwrap();
    save_brief(&pool, id, "# Rate limiting\n").await.unwrap();

    set_state(&pool, id, Lifecycle::Grilling).await.unwrap();

    assert_eq!(
        save_brief(&pool, id, "# Something else\n").await.unwrap(),
        Edited::NotDrafting
    );
    assert_eq!(
        rename_branch(&pool, id, "something-else").await.unwrap(),
        Edited::NotDrafting
    );
    assert_eq!(
        set_base_commit(&pool, id, Some("deadbee")).await.unwrap(),
        Edited::NotDrafting
    );

    let conversation = load_conversation(&pool, id).await.unwrap().unwrap();
    assert_eq!(conversation.branch, "amber-kestrel");
    assert_eq!(conversation.base_commit, None);
    assert_eq!(brief(&pool, id).await, "# Rate limiting\n");
}

#[tokio::test]
async fn editing_a_conversation_that_is_not_there_says_so() {
    let (_dir, pool) = fresh_pool().await;

    assert_eq!(
        save_brief(&pool, 404, "# Nothing\n").await.unwrap(),
        Edited::NoSuchConversation
    );
    assert_eq!(
        rename_branch(&pool, 404, "nothing").await.unwrap(),
        Edited::NoSuchConversation
    );
    assert_eq!(
        set_base_commit(&pool, 404, None).await.unwrap(),
        Edited::NoSuchConversation
    );
    assert!(load_conversation(&pool, 404).await.unwrap().is_none());
}

/// The point of the Brief being in SQLite rather than in a page's memory: the
/// server is a service that restarts, and what the human wrote must be there
/// afterwards.
#[tokio::test]
async fn a_conversation_and_its_brief_survive_the_database_being_reopened() {
    let dir = tempfile::tempdir().unwrap();
    let database = dir.path().join("verkstead.db");

    let pool = open_database(&database).await.unwrap();
    let repo_id = repo(&pool, "verkstead").await;
    let id = start_conversation(&pool, repo_id, "amber-kestrel")
        .await
        .unwrap()
        .unwrap();
    save_brief(&pool, id, "# Rate limiting\n\nThe API has none.\n")
        .await
        .unwrap();
    rename_branch(&pool, id, "rate-limiting").await.unwrap();
    pool.close().await;

    let pool = open_database(&database).await.unwrap();
    let conversation = load_conversation(&pool, id).await.unwrap().unwrap();

    assert_eq!(conversation.branch, "rate-limiting");
    assert_eq!(conversation.repo.name, "verkstead");
    assert_eq!(brief(&pool, id).await, "# Rate limiting\n\nThe API has none.\n");
}

#[tokio::test]
async fn nothing_started_means_nothing_listed() {
    let (_dir, pool) = fresh_pool().await;

    assert!(conversations(&pool).await.unwrap().is_empty());
}
