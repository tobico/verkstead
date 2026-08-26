//! Conversations: what starting one records, what a drafting one is still the
//! human's to change, and that all of it is still there after a restart.

use std::path::Path;

use sqlx::SqlitePool;
use verkstead_store::{
    Archiving, Closing, Edited, Event, Grilling, Lifecycle, adopting, archive_conversation,
    close_conversation, conversations, load_conversation, open_database, register_repo,
    rename_branch, save_brief, set_base_commit, set_state, start_adoption, start_conversation,
    start_grilling, timeline,
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
///
/// Found rather than taken from the front, because a Timeline grows: the Brief
/// is the first Event, but the moves that follow it are Events too. The first of
/// them, which is the only one until a steer opens a second round — see
/// [`briefs`] for the reading that tells one round from the next.
async fn brief(pool: &SqlitePool, id: i64) -> String {
    briefs(pool, id)
        .await
        .into_iter()
        .next()
        .expect("every Conversation has a Brief from the moment it exists")
}

/// Every Brief on a Conversation's Timeline, in order: one per round.
async fn briefs(pool: &SqlitePool, id: i64) -> Vec<String> {
    timeline(pool, id)
        .await
        .unwrap()
        .iter()
        .filter_map(|event| match &event.event {
            Event::Brief(markdown) => Some(markdown.clone()),
            _ => None,
        })
        .collect()
}

/// The states a Conversation's Timeline says it has moved through, in order.
async fn moves(pool: &SqlitePool, id: i64) -> Vec<Lifecycle> {
    timeline(pool, id)
        .await
        .unwrap()
        .iter()
        .filter_map(|event| match event.event {
            Event::Moved(state) => Some(state),
            _ => None,
        })
        .collect()
}

/// A drafting Conversation with a Brief written, ready to be grilled.
async fn drafted(pool: &SqlitePool) -> i64 {
    let repo_id = repo(pool, "verkstead").await;
    let id = start_conversation(pool, repo_id, "rate-limiting")
        .await
        .unwrap()
        .unwrap();
    save_brief(pool, id, "# Rate limiting\n").await.unwrap();
    id
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
        load_conversation(&pool, id)
            .await
            .unwrap()
            .unwrap()
            .base_commit,
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
    assert_eq!(
        brief(&pool, id).await,
        "# Rate limiting\n\nThe API has none.\n"
    );
}

#[tokio::test]
async fn nothing_started_means_nothing_listed() {
    let (_dir, pool) = fresh_pool().await;

    assert!(conversations(&pool).await.unwrap().is_empty());
}

/// Starting to grill records the three things that were not facts before it: the
/// commit the work branched from, where its worktree went, and that it has moved.
#[tokio::test]
async fn starting_to_grill_records_the_base_commit_the_worktree_and_the_move() {
    let (_dir, pool) = fresh_pool().await;
    let id = drafted(&pool).await;

    assert_eq!(
        start_grilling(
            &pool,
            id,
            "deadbeef",
            Path::new("/state/worktrees/verkstead-rate-limiting")
        )
        .await
        .unwrap(),
        Grilling::Started
    );

    let conversation = load_conversation(&pool, id).await.unwrap().unwrap();
    assert_eq!(conversation.state, Lifecycle::Grilling);
    assert_eq!(conversation.base_commit.as_deref(), Some("deadbeef"));
    assert_eq!(
        conversation.worktree.as_deref(),
        Some(Path::new("/state/worktrees/verkstead-rate-limiting"))
    );
    assert_eq!(moves(&pool, id).await, [Lifecycle::Grilling]);
}

/// The rule that the base commit is the default branch's tip *at grill start*
/// resolves here and nowhere else: before this there is a rule, after it a fact.
#[tokio::test]
async fn the_base_commit_is_written_even_where_the_human_overrode_nothing() {
    let (_dir, pool) = fresh_pool().await;
    let id = drafted(&pool).await;

    assert_eq!(
        load_conversation(&pool, id)
            .await
            .unwrap()
            .unwrap()
            .base_commit,
        None,
        "nothing was overridden, so there is only the rule"
    );

    start_grilling(&pool, id, "0123456", Path::new("/state/worktrees/x"))
        .await
        .unwrap();

    assert_eq!(
        load_conversation(&pool, id)
            .await
            .unwrap()
            .unwrap()
            .base_commit
            .as_deref(),
        Some("0123456")
    );
}

/// A Conversation cannot be started twice. The second attempt would be a second
/// branch and a second worktree for one piece of work.
#[tokio::test]
async fn a_conversation_that_is_not_drafting_cannot_start_grilling() {
    let (_dir, pool) = fresh_pool().await;
    let id = drafted(&pool).await;

    start_grilling(&pool, id, "deadbeef", Path::new("/state/worktrees/x"))
        .await
        .unwrap();

    assert_eq!(
        start_grilling(&pool, id, "cafe", Path::new("/state/worktrees/y"))
            .await
            .unwrap(),
        Grilling::NotDrafting
    );

    // And nothing of the second attempt was written.
    let conversation = load_conversation(&pool, id).await.unwrap().unwrap();
    assert_eq!(conversation.base_commit.as_deref(), Some("deadbeef"));
    assert_eq!(
        conversation.worktree.as_deref(),
        Some(Path::new("/state/worktrees/x"))
    );
    assert_eq!(moves(&pool, id).await, [Lifecycle::Grilling]);
}

#[tokio::test]
async fn grilling_a_conversation_that_is_not_there_says_so() {
    let (_dir, pool) = fresh_pool().await;

    assert_eq!(
        start_grilling(&pool, 404, "deadbeef", Path::new("/state/worktrees/x"))
            .await
            .unwrap(),
        Grilling::NoSuchConversation
    );
}

/// The Brief and the branch name stop being the human's the moment grilling
/// starts. The refusals were written in the stage before this one; this is the
/// first thing that actually trips them.
#[tokio::test]
async fn grilling_freezes_the_brief_and_the_branch_name() {
    let (_dir, pool) = fresh_pool().await;
    let id = drafted(&pool).await;

    start_grilling(&pool, id, "deadbeef", Path::new("/state/worktrees/x"))
        .await
        .unwrap();

    assert_eq!(
        save_brief(&pool, id, "# Something else\n").await.unwrap(),
        Edited::NotDrafting
    );
    assert_eq!(
        rename_branch(&pool, id, "something-else").await.unwrap(),
        Edited::NotDrafting
    );
    assert_eq!(
        set_base_commit(&pool, id, Some("cafe")).await.unwrap(),
        Edited::NotDrafting
    );

    let conversation = load_conversation(&pool, id).await.unwrap().unwrap();
    assert_eq!(conversation.branch, "rate-limiting");
    assert_eq!(conversation.base_commit.as_deref(), Some("deadbeef"));
    assert_eq!(brief(&pool, id).await, "# Rate limiting\n");
}

/// Closing forgets the worktree and keeps everything that says what the work
/// was: the branch it was on, the Brief it started from, the commit it branched
/// from.
#[tokio::test]
async fn closing_forgets_the_worktree_and_keeps_the_branch() {
    let (_dir, pool) = fresh_pool().await;
    let id = drafted(&pool).await;
    start_grilling(&pool, id, "deadbeef", Path::new("/state/worktrees/x"))
        .await
        .unwrap();

    assert_eq!(
        close_conversation(&pool, id).await.unwrap(),
        Closing::Closed
    );

    let conversation = load_conversation(&pool, id).await.unwrap().unwrap();
    assert_eq!(conversation.state, Lifecycle::Closed);
    assert_eq!(conversation.worktree, None);
    assert_eq!(conversation.branch, "rate-limiting");
    assert_eq!(conversation.base_commit.as_deref(), Some("deadbeef"));
    assert_eq!(
        moves(&pool, id).await,
        [Lifecycle::Grilling, Lifecycle::Closed]
    );
}

/// Closing twice is not an error — the human asked for it to be stopped, and it
/// is. The second one records nothing, so the Timeline says it happened once.
#[tokio::test]
async fn closing_twice_is_not_an_error() {
    let (_dir, pool) = fresh_pool().await;
    let id = drafted(&pool).await;
    start_grilling(&pool, id, "deadbeef", Path::new("/state/worktrees/x"))
        .await
        .unwrap();

    close_conversation(&pool, id).await.unwrap();
    assert_eq!(
        close_conversation(&pool, id).await.unwrap(),
        Closing::AlreadyClosed
    );

    assert_eq!(
        moves(&pool, id).await,
        [Lifecycle::Grilling, Lifecycle::Closed]
    );
}

/// Closing is reachable from every state this stage can reach, which includes
/// the one where nothing has been made yet.
#[tokio::test]
async fn a_drafting_conversation_can_be_closed_without_ever_having_grilled() {
    let (_dir, pool) = fresh_pool().await;
    let id = drafted(&pool).await;

    assert_eq!(
        close_conversation(&pool, id).await.unwrap(),
        Closing::Closed
    );

    let conversation = load_conversation(&pool, id).await.unwrap().unwrap();
    assert_eq!(conversation.state, Lifecycle::Closed);
    assert_eq!(conversation.worktree, None);
    assert_eq!(moves(&pool, id).await, [Lifecycle::Closed]);
}

/// A closed Conversation is past drafting, so it cannot be started either.
#[tokio::test]
async fn a_closed_conversation_cannot_start_grilling() {
    let (_dir, pool) = fresh_pool().await;
    let id = drafted(&pool).await;
    close_conversation(&pool, id).await.unwrap();

    assert_eq!(
        start_grilling(&pool, id, "deadbeef", Path::new("/state/worktrees/x"))
            .await
            .unwrap(),
        Grilling::NotDrafting
    );
}

#[tokio::test]
async fn closing_a_conversation_that_is_not_there_says_so() {
    let (_dir, pool) = fresh_pool().await;

    assert_eq!(
        close_conversation(&pool, 404).await.unwrap(),
        Closing::NoSuchConversation
    );
}

/// Archiving takes a Closed Conversation off the sidebar and leaves everything
/// else about it where it was: nothing leaves a Timeline, and the branch is
/// still the branch.
#[tokio::test]
async fn archiving_a_closed_conversation_takes_it_off_the_list() {
    let (_dir, pool) = fresh_pool().await;
    let id = drafted(&pool).await;
    close_conversation(&pool, id).await.unwrap();

    assert_eq!(
        archive_conversation(&pool, id).await.unwrap(),
        Archiving::Archived
    );

    assert!(conversations(&pool).await.unwrap().is_empty());

    let conversation = load_conversation(&pool, id).await.unwrap().unwrap();
    assert_eq!(conversation.state, Lifecycle::Closed);
    assert_eq!(conversation.branch, "rate-limiting");
    assert_eq!(brief(&pool, id).await, "# Rate limiting\n");
}

/// And it survives the process, which is the whole point of writing it down: a
/// list that forgot what had been put away would put it back on the next
/// reload.
#[tokio::test]
async fn what_was_archived_is_still_archived_after_a_restart() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("verkstead.db");

    let id = {
        let pool = open_database(&path).await.unwrap();
        let id = drafted(&pool).await;
        close_conversation(&pool, id).await.unwrap();
        archive_conversation(&pool, id).await.unwrap();
        pool.close().await;
        id
    };

    let pool = open_database(&path).await.unwrap();

    assert!(conversations(&pool).await.unwrap().is_empty());
    assert_eq!(
        archive_conversation(&pool, id).await.unwrap(),
        Archiving::AlreadyArchived
    );
}

/// Archiving twice is not an error — what the human asked for holds either way
/// — and the second one writes nothing.
#[tokio::test]
async fn archiving_twice_is_not_an_error() {
    let (_dir, pool) = fresh_pool().await;
    let id = drafted(&pool).await;
    close_conversation(&pool, id).await.unwrap();

    archive_conversation(&pool, id).await.unwrap();

    assert_eq!(
        archive_conversation(&pool, id).await.unwrap(),
        Archiving::AlreadyArchived
    );
}

/// A Conversation still being worked on belongs on the list it is being worked
/// from, so it is closed first and archived after.
#[tokio::test]
async fn a_conversation_that_is_not_closed_cannot_be_archived() {
    let (_dir, pool) = fresh_pool().await;
    let id = drafted(&pool).await;

    assert_eq!(
        archive_conversation(&pool, id).await.unwrap(),
        Archiving::NotClosed
    );

    start_grilling(&pool, id, "deadbeef", Path::new("/state/worktrees/x"))
        .await
        .unwrap();

    assert_eq!(
        archive_conversation(&pool, id).await.unwrap(),
        Archiving::NotClosed
    );

    assert_eq!(conversations(&pool).await.unwrap().len(), 1);
}

#[tokio::test]
async fn archiving_a_conversation_that_is_not_there_says_so() {
    let (_dir, pool) = fresh_pool().await;

    assert_eq!(
        archive_conversation(&pool, 404).await.unwrap(),
        Archiving::NoSuchConversation
    );
}

/// Where the worktree went outlives the process that made it — it is a directory
/// on disk, and the thing that knows to clean it up is a restarted server.
#[tokio::test]
async fn a_worktree_survives_the_database_being_reopened() {
    let dir = tempfile::tempdir().unwrap();
    let database = dir.path().join("verkstead.db");

    let pool = open_database(&database).await.unwrap();
    let id = drafted(&pool).await;
    start_grilling(
        &pool,
        id,
        "deadbeef",
        Path::new("/state/worktrees/verkstead-rate-limiting"),
    )
    .await
    .unwrap();
    pool.close().await;

    let pool = open_database(&database).await.unwrap();
    let conversation = load_conversation(&pool, id).await.unwrap().unwrap();

    assert_eq!(conversation.state, Lifecycle::Grilling);
    assert_eq!(
        conversation.worktree.as_deref(),
        Some(Path::new("/state/worktrees/verkstead-rate-limiting"))
    );
}

/// The mark, which is the whole of what adoption stores: which roadmap, and
/// nothing about what that roadmap says.
#[tokio::test]
async fn an_adopting_conversation_records_the_roadmap_it_is_adopting() {
    let (_dir, pool) = fresh_pool().await;
    let repo_id = repo(&pool, "verkstead").await;

    let id = start_adoption(&pool, repo_id, "spring-otter", "mvp")
        .await
        .unwrap()
        .expect("the Repo is registered, so the Conversation should start");

    let conversation = load_conversation(&pool, id).await.unwrap().unwrap();
    assert_eq!(conversation.adopting.as_deref(), Some("mvp"));

    // And it is a Conversation like any other otherwise: drafting, on the branch
    // it was given, with an empty Brief nobody here writes.
    assert_eq!(conversation.state, Lifecycle::Draft);
    assert_eq!(conversation.branch, "spring-otter");
    assert_eq!(brief(&pool, id).await, "");
}

#[tokio::test]
async fn a_conversation_started_the_ordinary_way_is_adopting_nothing() {
    let (_dir, pool) = fresh_pool().await;
    let repo_id = repo(&pool, "verkstead").await;

    let id = start_conversation(&pool, repo_id, "rate-limiting")
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        load_conversation(&pool, id)
            .await
            .unwrap()
            .unwrap()
            .adopting,
        None
    );
    assert_eq!(adopting(&pool, id).await.unwrap(), None);
}

#[tokio::test]
async fn an_adoption_cannot_be_started_against_a_repo_that_is_not_registered() {
    let (_dir, pool) = fresh_pool().await;

    assert!(
        start_adoption(&pool, 404, "spring-otter", "mvp")
            .await
            .unwrap()
            .is_none()
    );
    assert!(conversations(&pool).await.unwrap().is_empty());
}

/// The mark is a row like every other, so it is there after a restart — a page
/// drawn for adopting before the reboot is drawn for adopting after it.
#[tokio::test]
async fn the_roadmap_being_adopted_survives_the_database_being_reopened() {
    let (dir, pool) = fresh_pool().await;
    let repo_id = repo(&pool, "verkstead").await;
    let id = start_adoption(&pool, repo_id, "spring-otter", "mvp")
        .await
        .unwrap()
        .unwrap();
    pool.close().await;

    let reopened = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    assert_eq!(
        adopting(&reopened, id).await.unwrap().as_deref(),
        Some("mvp")
    );
}
