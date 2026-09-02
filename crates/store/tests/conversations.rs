//! Conversations: what starting one records, what a drafting one is still the
//! human's to change, and that all of it is still there after a restart.

use std::path::{Path, PathBuf};

use sqlx::SqlitePool;
use verkstead_store::{
    Account, Archiving, Closing, Edited, Event, Grilling, Lifecycle, Picked, ProfileFacts,
    RowState, Switched, Unarchiving, add_companion, adopting, archive_conversation, archived,
    close_conversation, conversation_branch, conversations, create_profile, follow_branch,
    load_conversation, open_database, register_repo, reinvent_branch, rename_branch, save_brief,
    set_base_commit, set_grilling_pairing, set_state, settle_naming, show_archived,
    showing_archived, start_adoption, start_building, start_conversation, start_grilling,
    start_unnamed_conversation, switch_repo, timeline, unarchive_conversation,
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

/// The model every made-up Profile here lists.
const MODEL: &str = "claude-opus-5";

/// A Profile to pair a role with, which is all these tests want one for: what a
/// switch must leave alone.
fn profile_facts(name: &str) -> ProfileFacts {
    ProfileFacts {
        name: name.to_owned(),
        account: Account::Claude {
            claude_dir: PathBuf::from(format!("/watched/accounts/{name}/.claude")),
            config_file: PathBuf::from(format!("/watched/accounts/{name}/.claude.json")),
        },
        models: vec![MODEL.to_owned()],
    }
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
        rename_branch(&pool, id, Some("rate-limiting"))
            .await
            .unwrap(),
        Edited::Saved
    );
    assert_eq!(
        set_base_commit(&pool, id, Some("6f32b11")).await.unwrap(),
        Edited::Saved
    );

    let conversation = load_conversation(&pool, id).await.unwrap().unwrap();
    assert_eq!(conversation.branch, "rate-limiting");
    assert!(conversation.branch_named);
    assert_eq!(conversation.base_commit.as_deref(), Some("6f32b11"));
}

/// A Conversation started on a name nobody settled on carries it all the same —
/// there is a branch to cut — and says whose it is.
#[tokio::test]
async fn a_conversation_can_be_started_on_a_name_nobody_has_settled_on() {
    let (_dir, pool) = fresh_pool().await;
    let repo_id = repo(&pool, "verkstead").await;
    let id = start_unnamed_conversation(&pool, repo_id, "amber-kestrel")
        .await
        .unwrap()
        .unwrap();

    let conversation = load_conversation(&pool, id).await.unwrap().unwrap();
    assert_eq!(conversation.branch, "amber-kestrel");
    assert!(!conversation.branch_named);
}

/// Starting the work on a name nobody settled on leaves the naming of it to the
/// first session, and starting it on a name the human typed leaves nothing to
/// anybody.
#[tokio::test]
async fn starting_the_work_leaves_an_invented_branch_name_to_be_replaced() {
    let (_dir, pool) = fresh_pool().await;
    let repo_id = repo(&pool, "verkstead").await;

    let invented = start_unnamed_conversation(&pool, repo_id, "amber-kestrel")
        .await
        .unwrap()
        .unwrap();
    let typed = start_conversation(&pool, repo_id, "rate-limiting")
        .await
        .unwrap()
        .unwrap();

    // Before the press there is nothing to name: the field is the human's until
    // the branch is cut.
    assert!(
        !load_conversation(&pool, invented)
            .await
            .unwrap()
            .unwrap()
            .naming
    );

    for (id, worktree) in [(invented, "amber-kestrel"), (typed, "rate-limiting")] {
        start_grilling(
            &pool,
            id,
            "c0ffee",
            &Path::new("/data/worktrees").join(worktree),
            &[],
        )
        .await
        .unwrap();
    }

    assert!(
        load_conversation(&pool, invented)
            .await
            .unwrap()
            .unwrap()
            .naming,
        "the first session is the one told to pick a name",
    );
    assert!(
        !load_conversation(&pool, typed)
            .await
            .unwrap()
            .unwrap()
            .naming,
        "a name the human typed has nothing to wait for",
    );
}

/// A start with no grilling in it leaves the same job to the session it starts,
/// there being nothing different about it but which state it lands in.
#[tokio::test]
async fn a_start_with_no_grilling_leaves_the_branch_to_be_named_too() {
    let (_dir, pool) = fresh_pool().await;
    let repo_id = repo(&pool, "verkstead").await;
    let id = start_unnamed_conversation(&pool, repo_id, "amber-kestrel")
        .await
        .unwrap()
        .unwrap();

    start_building(
        &pool,
        id,
        "c0ffee",
        Path::new("/data/worktrees/amber-kestrel"),
        &[],
    )
    .await
    .unwrap();

    let conversation = load_conversation(&pool, id).await.unwrap().unwrap();
    assert_eq!(conversation.state, Lifecycle::Implementing);
    assert!(conversation.naming);
}

/// The rename the instruction asked for is the end of the waiting, and so is a
/// session that ended without making one.
#[tokio::test]
async fn a_branch_stops_waiting_to_be_named_by_being_renamed_or_by_being_settled_for() {
    let (_dir, pool) = fresh_pool().await;
    let repo_id = repo(&pool, "verkstead").await;

    let renamed = start_unnamed_conversation(&pool, repo_id, "amber-kestrel")
        .await
        .unwrap()
        .unwrap();
    let left = start_unnamed_conversation(&pool, repo_id, "brave-otter")
        .await
        .unwrap()
        .unwrap();

    for (id, worktree) in [(renamed, "amber-kestrel"), (left, "brave-otter")] {
        start_grilling(
            &pool,
            id,
            "c0ffee",
            &Path::new("/data/worktrees").join(worktree),
            &[],
        )
        .await
        .unwrap();
    }

    follow_branch(&pool, renamed, "rate-limiting")
        .await
        .unwrap();
    settle_naming(&pool, left).await.unwrap();

    let renamed = load_conversation(&pool, renamed).await.unwrap().unwrap();
    assert_eq!(renamed.branch, "rate-limiting");
    assert!(!renamed.naming);

    let left = load_conversation(&pool, left).await.unwrap().unwrap();
    assert_eq!(
        left.branch, "brave-otter",
        "settling for a name is not changing it",
    );
    assert!(!left.naming);
    assert!(
        !left.branch_named,
        "settling for a name is not somebody having chosen it either",
    );
}

/// And the sidebar row says the same thing, being what draws the title.
#[tokio::test]
async fn a_row_says_whether_its_branch_is_still_to_be_named() {
    let (_dir, pool) = fresh_pool().await;
    let repo_id = repo(&pool, "verkstead").await;
    let id = start_unnamed_conversation(&pool, repo_id, "amber-kestrel")
        .await
        .unwrap()
        .unwrap();

    start_grilling(
        &pool,
        id,
        "c0ffee",
        Path::new("/data/worktrees/amber-kestrel"),
        &[],
    )
    .await
    .unwrap();

    let rows = conversations(&pool).await.unwrap();
    let row = rows.iter().find(|row| row.id == id).unwrap();
    assert!(row.naming);
    assert!(!row.branch_named);

    settle_naming(&pool, id).await.unwrap();

    let rows = conversations(&pool).await.unwrap();
    assert!(!rows.iter().find(|row| row.id == id).unwrap().naming);
}

/// Following a session's rename moves the name and leaves whose it is where it
/// was — in either of the two columns a Conversation's branch lives in.
///
/// The name Verkstead invented is still Verkstead's after a session picked a
/// better one; the name the human typed is still theirs. What moves is the
/// branch, because the branch has moved.
#[tokio::test]
async fn following_a_rename_moves_the_name_and_not_whose_it_is() {
    let (_dir, pool) = fresh_pool().await;
    let repo_id = repo(&pool, "verkstead").await;

    let verksteads = start_unnamed_conversation(&pool, repo_id, "amber-kestrel")
        .await
        .unwrap()
        .unwrap();
    let theirs = start_conversation(&pool, repo_id, "throttling")
        .await
        .unwrap()
        .unwrap();

    follow_branch(&pool, verksteads, "rate-limiting")
        .await
        .unwrap();
    follow_branch(&pool, theirs, "rate-limiting-too")
        .await
        .unwrap();

    let conversation = load_conversation(&pool, verksteads).await.unwrap().unwrap();
    assert_eq!(conversation.branch, "rate-limiting");
    assert!(!conversation.branch_named);

    let conversation = load_conversation(&pool, theirs).await.unwrap().unwrap();
    assert_eq!(conversation.branch, "rate-limiting-too");
    assert!(conversation.branch_named);

    assert_eq!(
        conversation_branch(&pool, theirs).await.unwrap().as_deref(),
        Some("rate-limiting-too"),
        "which is the reading everything that only wants the name takes",
    );
    assert_eq!(
        conversation_branch(&pool, theirs + 1000).await.unwrap(),
        None,
        "and no such Conversation is no such branch",
    );
}

/// Handing a followed name back is still the name the Conversation started on,
/// rather than the one the session renamed the branch to.
///
/// The prefill is what stands when the field is cleared, and following a rename
/// is not the human typing in it — but it does move the prefill, because the
/// branch it named is not there any more. So what stands is where the branch
/// actually is.
#[tokio::test]
async fn handing_back_a_name_after_a_rename_leaves_the_branch_that_exists() {
    let (_dir, pool) = fresh_pool().await;
    let repo_id = repo(&pool, "verkstead").await;
    let id = start_unnamed_conversation(&pool, repo_id, "amber-kestrel")
        .await
        .unwrap()
        .unwrap();

    rename_branch(&pool, id, Some("rate-limiting"))
        .await
        .unwrap();
    follow_branch(&pool, id, "throttling").await.unwrap();
    rename_branch(&pool, id, None).await.unwrap();

    let conversation = load_conversation(&pool, id).await.unwrap().unwrap();
    assert_eq!(conversation.branch, "throttling");
    assert!(!conversation.branch_named);
}

/// Handing the name back leaves the one the Conversation was started on, rather
/// than a branch called nothing or another name invented on the spot.
#[tokio::test]
async fn handing_the_branch_name_back_leaves_the_one_it_started_on() {
    let (_dir, pool) = fresh_pool().await;
    let repo_id = repo(&pool, "verkstead").await;
    let id = start_unnamed_conversation(&pool, repo_id, "amber-kestrel")
        .await
        .unwrap()
        .unwrap();

    rename_branch(&pool, id, Some("rate-limiting"))
        .await
        .unwrap();
    assert_eq!(rename_branch(&pool, id, None).await.unwrap(), Edited::Saved);

    let conversation = load_conversation(&pool, id).await.unwrap().unwrap();
    assert_eq!(conversation.branch, "amber-kestrel");
    assert!(!conversation.branch_named);
}

/// Inventing another name replaces the prefill and leaves whose the name is
/// alone: what went in is another name Verkstead invented, so the Conversation
/// is still on one of its own and its first session still has one to pick.
#[tokio::test]
async fn inventing_another_name_replaces_the_one_the_conversation_started_on() {
    let (_dir, pool) = fresh_pool().await;
    let repo_id = repo(&pool, "verkstead").await;
    let id = start_unnamed_conversation(&pool, repo_id, "amber-kestrel")
        .await
        .unwrap()
        .unwrap();

    reinvent_branch(&pool, id, "hushed-otter").await.unwrap();

    let conversation = load_conversation(&pool, id).await.unwrap().unwrap();
    assert_eq!(conversation.branch, "hushed-otter");
    assert!(!conversation.branch_named);
}

/// And it does nothing at all where the human has settled a name. Theirs is not
/// a name to go picking again behind them — a repository already holding it is
/// something to tell them about instead.
#[tokio::test]
async fn inventing_another_name_leaves_a_name_the_human_settled() {
    let (_dir, pool) = fresh_pool().await;
    let repo_id = repo(&pool, "verkstead").await;
    let theirs = start_conversation(&pool, repo_id, "throttling")
        .await
        .unwrap()
        .unwrap();
    let typed = start_unnamed_conversation(&pool, repo_id, "amber-kestrel")
        .await
        .unwrap()
        .unwrap();

    rename_branch(&pool, typed, Some("rate-limiting"))
        .await
        .unwrap();

    for id in [theirs, typed] {
        reinvent_branch(&pool, id, "hushed-otter").await.unwrap();
    }

    let conversation = load_conversation(&pool, theirs).await.unwrap().unwrap();
    assert_eq!(conversation.branch, "throttling");

    let conversation = load_conversation(&pool, typed).await.unwrap().unwrap();
    assert_eq!(conversation.branch, "rate-limiting");
    assert!(conversation.branch_named);

    // Including the prefill underneath it, which is what stands if that name is
    // ever handed back.
    rename_branch(&pool, typed, None).await.unwrap();
    let conversation = load_conversation(&pool, typed).await.unwrap().unwrap();
    assert_eq!(conversation.branch, "amber-kestrel");
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
        rename_branch(&pool, id, Some("something-else"))
            .await
            .unwrap(),
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
        rename_branch(&pool, 404, Some("nothing")).await.unwrap(),
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
    rename_branch(&pool, id, Some("rate-limiting"))
        .await
        .unwrap();
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

/// Moving a draft onto another Repo, and the three things that follow from it:
/// the base back on the rule, every companion kept but the one that has just
/// become the Conversation's own, and the branch name and the Pairings exactly
/// where the human left them.
#[tokio::test]
async fn switching_a_drafts_repo_resets_its_base_and_drops_only_the_companion_it_became() {
    let (_dir, pool) = fresh_pool().await;
    let verkstead = repo(&pool, "verkstead").await;
    let askance = repo(&pool, "askance").await;
    let notes = repo(&pool, "notes").await;

    let id = start_conversation(&pool, verkstead, "amber-kestrel")
        .await
        .unwrap()
        .unwrap();

    // Everything the switch must not touch, said first: a branch the human
    // typed, an account to grill under, and two repos to work alongside — one
    // of which is where the work is about to move.
    rename_branch(&pool, id, Some("rate-limiting"))
        .await
        .unwrap();
    let profile = create_profile(&pool, &profile_facts("desk"))
        .await
        .unwrap()
        .expect("nothing is called that yet");
    set_grilling_pairing(&pool, id, profile.id, Some(MODEL))
        .await
        .unwrap();
    set_base_commit(&pool, id, Some("main")).await.unwrap();
    add_companion(&pool, id, askance).await.unwrap();
    add_companion(&pool, id, notes).await.unwrap();

    assert_eq!(
        switch_repo(&pool, id, askance).await.unwrap(),
        Switched::Switched
    );

    let conversation = load_conversation(&pool, id).await.unwrap().unwrap();
    assert_eq!(conversation.repo.name, "askance");
    assert_eq!(
        conversation.base_commit, None,
        "the override named a branch of the repo being left, so the new repo's \
         default-branch rule stands again"
    );
    assert_eq!(
        conversation
            .companions
            .iter()
            .map(|companion| companion.repo.name.as_str())
            .collect::<Vec<_>>(),
        ["notes"],
        "the repo it moved onto is its own now, and a Conversation is no \
         companion of itself — the other one has nothing to do with the move"
    );
    assert_eq!(conversation.branch, "rate-limiting");
    assert!(conversation.branch_named);
    assert!(matches!(conversation.grilling_pairing, Picked::Under(_)));
}

/// The freeze: a checkout is of one repository, so from the moment there is one
/// the Repo is settled — asked off the worktree rather than off the state, which
/// is what a second round steered back into Draft is still holding.
#[tokio::test]
async fn a_repo_switch_is_refused_once_the_branch_has_been_cut() {
    let (_dir, pool) = fresh_pool().await;
    let id = drafted(&pool).await;
    let elsewhere = repo(&pool, "askance").await;

    start_grilling(&pool, id, "deadbeef", Path::new("/state/worktrees/x"), &[])
        .await
        .unwrap();

    assert_eq!(
        switch_repo(&pool, id, elsewhere).await.unwrap(),
        Switched::NotDrafting
    );

    // And still refused where the state has come back to Draft, the worktree
    // having stayed: that is a round steered onto work that is already built.
    set_state(&pool, id, Lifecycle::Draft).await.unwrap();

    assert_eq!(
        switch_repo(&pool, id, elsewhere).await.unwrap(),
        Switched::NotDrafting
    );
    assert_eq!(
        load_conversation(&pool, id)
            .await
            .unwrap()
            .unwrap()
            .repo
            .name,
        "verkstead"
    );
}

/// And the other freeze, which has nothing to do with a checkout: a
/// Conversation adopting a roadmap is in the repository that roadmap is written
/// in, and only the roadmap's name is kept — so moving the work would leave it
/// adopting a name rather than a roadmap, and finding either nothing or a
/// different roadmap of the same name.
#[tokio::test]
async fn a_repo_switch_is_refused_while_a_roadmap_is_being_adopted() {
    let (_dir, pool) = fresh_pool().await;
    let verkstead = repo(&pool, "verkstead").await;
    let askance = repo(&pool, "askance").await;

    let id = start_adoption(&pool, verkstead, "amber-kestrel", "mvp")
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        switch_repo(&pool, id, askance).await.unwrap(),
        Switched::Adopting
    );

    // And nothing moved: the roadmap is still being read off the repository it
    // is written in.
    let conversation = load_conversation(&pool, id).await.unwrap().unwrap();
    assert_eq!(conversation.repo.name, "verkstead");
    assert_eq!(adopting(&pool, id).await.unwrap().as_deref(), Some("mvp"));
}

/// The two refusals about the asking rather than about the state.
#[tokio::test]
async fn switching_onto_a_repo_that_is_not_registered_says_so() {
    let (_dir, pool) = fresh_pool().await;
    let id = drafted(&pool).await;

    assert_eq!(
        switch_repo(&pool, id, 404).await.unwrap(),
        Switched::NoSuchRepo
    );
    assert_eq!(
        switch_repo(&pool, 404, 1).await.unwrap(),
        Switched::NoSuchConversation
    );
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
            Path::new("/state/worktrees/verkstead-rate-limiting"),
            &[],
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

    start_grilling(&pool, id, "0123456", Path::new("/state/worktrees/x"), &[])
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

    start_grilling(&pool, id, "deadbeef", Path::new("/state/worktrees/x"), &[])
        .await
        .unwrap();

    assert_eq!(
        start_grilling(&pool, id, "cafe", Path::new("/state/worktrees/y"), &[])
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
        start_grilling(&pool, 404, "deadbeef", Path::new("/state/worktrees/x"), &[])
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

    start_grilling(&pool, id, "deadbeef", Path::new("/state/worktrees/x"), &[])
        .await
        .unwrap();

    assert_eq!(
        save_brief(&pool, id, "# Something else\n").await.unwrap(),
        Edited::NotDrafting
    );
    assert_eq!(
        rename_branch(&pool, id, Some("something-else"))
            .await
            .unwrap(),
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
    start_grilling(&pool, id, "deadbeef", Path::new("/state/worktrees/x"), &[])
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
    start_grilling(&pool, id, "deadbeef", Path::new("/state/worktrees/x"), &[])
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
        start_grilling(&pool, id, "deadbeef", Path::new("/state/worktrees/x"), &[])
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

/// Write a word into a Conversation's state column that no Verkstead knows.
///
/// A database restored from before a migration ran, one written by a Verkstead
/// from ahead of this one, or a row somebody edited by hand: however it got
/// there, the human is left with a Conversation whose every reader refuses it.
/// Which is the state the three below are about.
async fn corrupt_the_state(pool: &SqlitePool, id: i64) {
    sqlx::query("UPDATE conversations SET state = ? WHERE id = ?")
        .bind("meandering")
        .bind(id)
        .execute(pool)
        .await
        .unwrap();
}

/// And what the state column really holds, read past every parse.
async fn stored_state(pool: &SqlitePool, id: i64) -> String {
    let (state,): (String,) = sqlx::query_as("SELECT state FROM conversations WHERE id = ?")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap();

    state
}

/// The close is the way out of a state word nothing can read — and the way the
/// word itself is repaired.
///
/// A word this Verkstead does not know is not Closed, so the close goes ahead;
/// the write it makes is unconditional, so what the row holds afterwards is
/// `closed`. The Conversation comes back readable, which is the whole point:
/// everything else about it was locked behind that one column.
#[tokio::test]
async fn closing_a_conversation_whose_state_word_is_unreadable_closes_and_heals_it() {
    let (_dir, pool) = fresh_pool().await;
    let id = drafted(&pool).await;
    start_grilling(&pool, id, "deadbeef", Path::new("/state/worktrees/x"), &[])
        .await
        .unwrap();
    corrupt_the_state(&pool, id).await;

    assert!(
        load_conversation(&pool, id).await.is_err(),
        "the ordinary read refuses it, which is what the close has to work past"
    );

    assert_eq!(
        close_conversation(&pool, id).await.unwrap(),
        Closing::Closed
    );

    assert_eq!(stored_state(&pool, id).await, "closed");

    let conversation = load_conversation(&pool, id).await.unwrap().unwrap();
    assert_eq!(conversation.state, Lifecycle::Closed);
    assert_eq!(conversation.worktree, None);
}

/// Archiving one is refused rather than failing, and refused the safe way
/// round: a word nobody can read is not Closed, and hiding a Conversation whose
/// worktree may still be live would put the work out of sight without ending
/// it. Closing and archiving is the press that gets there, because the close
/// heals the word first.
#[tokio::test]
async fn archiving_a_conversation_whose_state_word_is_unreadable_says_it_is_not_closed() {
    let (_dir, pool) = fresh_pool().await;
    let id = drafted(&pool).await;
    corrupt_the_state(&pool, id).await;

    assert_eq!(
        archive_conversation(&pool, id).await.unwrap(),
        Archiving::NotClosed
    );

    close_conversation(&pool, id).await.unwrap();

    assert_eq!(
        archive_conversation(&pool, id).await.unwrap(),
        Archiving::Archived
    );
    assert!(conversations(&pool).await.unwrap().is_empty());
}

/// And the sidebar still draws it, carrying the word it could not read.
///
/// The list is the only route to a Conversation's own page, so one row nobody
/// can parse used to take every other row off the page with it — leaving a
/// human with no way to reach the very Conversation they were trying to end.
#[tokio::test]
async fn the_list_carries_a_row_whose_state_word_is_unreadable() {
    let (_dir, pool) = fresh_pool().await;
    let readable = drafted(&pool).await;
    let repo_id = repo(&pool, "askance").await;
    let broken = start_conversation(&pool, repo_id, "amber-kestrel")
        .await
        .unwrap()
        .unwrap();
    corrupt_the_state(&pool, broken).await;

    let rows = conversations(&pool).await.unwrap();

    assert_eq!(
        rows.iter().map(|row| row.id).collect::<Vec<_>>(),
        [broken, readable],
        "both rows, newest first"
    );
    assert_eq!(
        rows[0].state,
        RowState::Unknown("meandering".to_owned()),
        "with the word it could not read carried, for whoever draws the row to say"
    );
    assert_eq!(rows[0].state.known(), None);
    assert_eq!(rows[1].state, RowState::Known(Lifecycle::Draft));
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

    start_grilling(&pool, id, "deadbeef", Path::new("/state/worktrees/x"), &[])
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

/// The way back: unarchiving puts a Conversation on the list again, and the
/// list is the only thing about it that moves.
#[tokio::test]
async fn unarchiving_puts_a_conversation_back_on_the_list() {
    let (_dir, pool) = fresh_pool().await;
    let id = drafted(&pool).await;
    close_conversation(&pool, id).await.unwrap();
    archive_conversation(&pool, id).await.unwrap();

    assert_eq!(
        unarchive_conversation(&pool, id).await.unwrap(),
        Unarchiving::Unarchived
    );

    assert!(!archived(&pool, id).await.unwrap());

    let list = conversations(&pool).await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].state, RowState::Known(Lifecycle::Closed));
}

/// And it holds: what was taken back out stays out, so a reload does not put
/// away something the human asked for back.
#[tokio::test]
async fn what_was_unarchived_is_still_unarchived_after_a_restart() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("verkstead.db");

    let id = {
        let pool = open_database(&path).await.unwrap();
        let id = drafted(&pool).await;
        close_conversation(&pool, id).await.unwrap();
        archive_conversation(&pool, id).await.unwrap();
        unarchive_conversation(&pool, id).await.unwrap();
        pool.close().await;
        id
    };

    let pool = open_database(&path).await.unwrap();

    assert_eq!(conversations(&pool).await.unwrap().len(), 1);
    assert_eq!(
        unarchive_conversation(&pool, id).await.unwrap(),
        Unarchiving::NotArchived
    );
}

/// Unarchiving one that was never put away is not an error — what the human
/// asked for holds either way.
#[tokio::test]
async fn unarchiving_one_that_is_not_archived_is_not_an_error() {
    let (_dir, pool) = fresh_pool().await;
    let id = drafted(&pool).await;

    assert_eq!(
        unarchive_conversation(&pool, id).await.unwrap(),
        Unarchiving::NotArchived
    );
    assert_eq!(conversations(&pool).await.unwrap().len(), 1);
}

#[tokio::test]
async fn unarchiving_a_conversation_that_is_not_there_says_so() {
    let (_dir, pool) = fresh_pool().await;

    assert_eq!(
        unarchive_conversation(&pool, 404).await.unwrap(),
        Unarchiving::NoSuchConversation
    );
}

/// The human's standing choice to be shown what they have put away: with it
/// on, an archived Conversation is on the list in its ordinary place.
#[tokio::test]
async fn showing_the_archived_puts_them_back_in_the_list() {
    let (_dir, pool) = fresh_pool().await;
    let id = drafted(&pool).await;
    close_conversation(&pool, id).await.unwrap();
    archive_conversation(&pool, id).await.unwrap();

    assert!(!showing_archived(&pool).await.unwrap());
    assert!(conversations(&pool).await.unwrap().is_empty());

    show_archived(&pool, true).await.unwrap();

    assert!(showing_archived(&pool).await.unwrap());
    let list = conversations(&pool).await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].id, id);
    assert!(archived(&pool, id).await.unwrap());

    show_archived(&pool, false).await.unwrap();

    assert!(!showing_archived(&pool).await.unwrap());
    assert!(conversations(&pool).await.unwrap().is_empty());
}

/// A switch rather than a press: asking for the position it is already in is
/// not something to refuse, in either direction.
#[tokio::test]
async fn saying_it_twice_says_the_same_thing() {
    let (_dir, pool) = fresh_pool().await;

    show_archived(&pool, true).await.unwrap();
    show_archived(&pool, true).await.unwrap();
    assert!(showing_archived(&pool).await.unwrap());

    show_archived(&pool, false).await.unwrap();
    show_archived(&pool, false).await.unwrap();
    assert!(!showing_archived(&pool).await.unwrap());
}

/// And the choice outlives the process, which is the whole reason it is here
/// rather than on the device that made it.
#[tokio::test]
async fn the_choice_to_show_them_survives_a_restart() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("verkstead.db");

    {
        let pool = open_database(&path).await.unwrap();
        let id = drafted(&pool).await;
        close_conversation(&pool, id).await.unwrap();
        archive_conversation(&pool, id).await.unwrap();
        show_archived(&pool, true).await.unwrap();
        pool.close().await;
    }

    let pool = open_database(&path).await.unwrap();

    assert!(showing_archived(&pool).await.unwrap());
    assert_eq!(conversations(&pool).await.unwrap().len(), 1);
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
        &[],
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
