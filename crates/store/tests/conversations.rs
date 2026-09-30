//! Conversations: what starting one records, what a drafting one is still the
//! human's to change, and that all of it is still there after a restart.

use std::path::{Path, PathBuf};

use sqlx::SqlitePool;
use verkstead_schema::Direction;
use verkstead_store::{
    Account, AdoptedPullRequest, Archiving, Closing, Decision, Driven, Edited, Event, Grilling,
    Joined, Lifecycle, Planned, ProfileFacts, PullRequest, Queued, Recorded, RoadmapStage,
    RowState, StageOf, StageStanding, Staged, Steer, Switched, Unarchiving, add_companion,
    adopted_pull_request, adopting, any_archived, archive_conversation, archived, clear_stop,
    close_conversation, conversation_branch, conversations, create_profile, driven_roadmaps,
    follow_branch, hold_pull_request, join_queue, load_conversation, open_database, pick_direction,
    queue_to_join, record_another_pull_request, record_pull_request, record_roadmap, register_repo,
    reinvent_branch, rename_branch, roadmap_planner, save_brief, set_base_commit,
    set_grilling_pairing, set_state, set_target, settle_naming, show_archived, showing_archived,
    stacks_on, stage_chain, stage_roadmap, stage_standings, start_adoption, start_conversation,
    start_grilling, start_stage, start_tinkering, start_unnamed_conversation, state,
    steer_conversation, stop, switch_repo, target, timeline, unarchive_conversation,
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
        name: Some(name.to_owned()),
        account: Account::Claude {
            claude_dir: PathBuf::from(format!("/watched/accounts/{name}/.claude")),
            config_file: PathBuf::from(format!("/watched/accounts/{name}/.claude.json")),
        },
        models: vec![MODEL.to_owned()],
        memory: true,
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
    assert!(conversation.grilling_pairing.is_some());
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

/// And *not* the same freeze over a Draft from before there were Processes,
/// unlike a roadmap: a Draft holding a pull-request adoption moves like any other.
///
/// What it is pointed at is the Target field, and a Target is a string whose
/// meaning follows the Repo — a bare `#41` is the number of whichever repository
/// it is read in, and a URL naming somewhere else is refused at Start by name.
/// Which is what such a Draft's target is, its adoption row being read as that
/// pull request's URL: moved onto another Repo it still names the repository it
/// always did, and the press says so. Refusing here would be a refusal nothing
/// could draw — nothing on the wire tells this Draft apart from a Review somebody
/// typed the same URL into.
#[tokio::test]
async fn a_repo_switch_goes_through_over_a_held_pull_request() {
    let (_dir, pool) = fresh_pool().await;
    let verkstead = repo(&pool, "verkstead").await;
    let askance = repo(&pool, "askance").await;

    let id = held_by(&pool, verkstead).await;

    assert_eq!(
        switch_repo(&pool, id, askance).await.unwrap(),
        Switched::Switched
    );

    // Moved, and still pointed at the pull request it always was — which is a
    // URL, so what the press will say about it is that it is another
    // repository's.
    assert_eq!(
        load_conversation(&pool, id)
            .await
            .unwrap()
            .unwrap()
            .repo
            .name,
        "askance",
    );
    assert_eq!(
        target(&pool, id).await.unwrap().as_deref(),
        Some("https://github.com/tobico/verkstead/pull/41"),
    );
}

/// A Conversation holding a pull-request adoption is pointed at it: its Target
/// reads as that pull request's own URL, which is what makes a Draft from before
/// there were Processes a **Review** that Start can take up.
///
/// The record is the weakest of the three ways a Target is named, so a field
/// somebody has typed in wins over it — and clearing that field falls back to the
/// adoption again, an emptied field keeping no row.
#[tokio::test]
async fn a_held_pull_request_is_the_target_where_nothing_else_names_one() {
    let (_dir, pool) = fresh_pool().await;
    let verkstead = repo(&pool, "verkstead").await;

    let id = held_by(&pool, verkstead).await;

    assert_eq!(
        target(&pool, id).await.unwrap().as_deref(),
        Some("https://github.com/tobico/verkstead/pull/41"),
    );
    assert_eq!(
        load_conversation(&pool, id)
            .await
            .unwrap()
            .unwrap()
            .target
            .as_deref(),
        Some("https://github.com/tobico/verkstead/pull/41"),
        "which is what the page draws the field from",
    );

    assert_eq!(
        set_target(&pool, id, Some("rate-limiting")).await.unwrap(),
        Edited::Saved,
    );
    assert_eq!(
        target(&pool, id).await.unwrap().as_deref(),
        Some("rate-limiting"),
        "the human's own typing names it instead",
    );

    assert_eq!(set_target(&pool, id, None).await.unwrap(), Edited::Saved);
    assert_eq!(
        target(&pool, id).await.unwrap().as_deref(),
        Some("https://github.com/tobico/verkstead/pull/41"),
    );
}

/// A Draft as the retired *Wrap up a pull request* level left one: started the
/// ordinary way, with the pull request written beside it.
///
/// Which is the only way there is to one now — the start that made these is
/// gone, and the row is written at the press. See [`hold_pull_request`].
async fn held_by(pool: &sqlx::SqlitePool, repo_id: i64) -> i64 {
    let id = start_unnamed_conversation(pool, repo_id, "amber-kestrel")
        .await
        .unwrap()
        .unwrap();

    hold_pull_request(pool, id, &rate_limiting()).await.unwrap();

    id
}

/// An ordinary Conversation, and one adopting a roadmap, are both holding no
/// pull request — which is what keeps their pages on the shapes they have.
#[tokio::test]
async fn a_conversation_started_any_other_way_is_holding_no_pull_request() {
    let (_dir, pool) = fresh_pool().await;
    let verkstead = repo(&pool, "verkstead").await;

    let ordinary = start_unnamed_conversation(&pool, verkstead, "amber-kestrel")
        .await
        .unwrap()
        .unwrap();
    let roadmap = start_adoption(&pool, verkstead, "quiet-heron", "mvp")
        .await
        .unwrap()
        .unwrap();

    for id in [ordinary, roadmap] {
        assert_eq!(adopted_pull_request(&pool, id).await.unwrap(), None);
    }
}

/// The pull request the tests above are about, as `gh` answered about it.
fn rate_limiting() -> AdoptedPullRequest {
    AdoptedPullRequest {
        number: 41,
        title: "Rate limiting for the public API".to_owned(),
        url: "https://github.com/tobico/verkstead/pull/41".to_owned(),
        head: "rate-limiting".to_owned(),
        base: "main".to_owned(),
    }
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

/// And the **Tinker** landing writes the same three things and leaves the
/// Conversation in Follow-up, which is the whole of what separates the two.
#[tokio::test]
async fn starting_a_tinker_records_the_same_things_and_lands_in_follow_up() {
    let (_dir, pool) = fresh_pool().await;
    let id = drafted(&pool).await;

    assert_eq!(
        start_tinkering(
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
    assert_eq!(
        conversation.state,
        Lifecycle::FollowUp,
        "a Tinker is never interviewed, so there is no grilling to land in",
    );
    assert_eq!(conversation.base_commit.as_deref(), Some("deadbeef"));
    assert_eq!(
        conversation.worktree.as_deref(),
        Some(Path::new("/state/worktrees/verkstead-rate-limiting"))
    );
    assert_eq!(moves(&pool, id).await, [Lifecycle::FollowUp]);

    assert_eq!(
        start_tinkering(&pool, id, "cafe", Path::new("/state/worktrees/y"), &[])
            .await
            .unwrap(),
        Grilling::NotDrafting,
        "and it cannot be started twice, for the reason no start can",
    );
}

/// Investigating is a state of its own, with a word of its own in the column.
///
/// Written here directly, because nothing in this task reaches it: what is
/// being asked is that the column round-trips it, which is how a Conversation
/// left in Investigating is read back after a restart.
#[tokio::test]
async fn a_conversation_set_investigating_reads_back_investigating() {
    let (_dir, pool) = fresh_pool().await;
    let id = drafted(&pool).await;

    set_state(&pool, id, Lifecycle::Investigating)
        .await
        .unwrap();

    assert_eq!(
        load_conversation(&pool, id).await.unwrap().unwrap().state,
        Lifecycle::Investigating,
        "the word the column holds is one this Verkstead reads",
    );
    assert_eq!(
        state(&pool, id).await.unwrap(),
        Some(Lifecycle::Investigating),
        "and the cheap reading of it says the same thing",
    );
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

/// And whether there is anything behind that switch at all, which is what the
/// filtered list cannot say: an empty list is the same empty list either way,
/// and the page that has no sidebar to hang the switch under has to know which
/// of the two it is looking at.
///
/// It is about the archiving rather than about the switch, so it answers the
/// same whichever position the switch is in.
#[tokio::test]
async fn whether_anything_is_archived_is_read_apart_from_the_switch() {
    let (_dir, pool) = fresh_pool().await;
    let id = drafted(&pool).await;

    assert!(!any_archived(&pool).await.unwrap());

    close_conversation(&pool, id).await.unwrap();
    archive_conversation(&pool, id).await.unwrap();

    assert!(any_archived(&pool).await.unwrap());
    assert!(conversations(&pool).await.unwrap().is_empty());

    show_archived(&pool, true).await.unwrap();
    assert!(any_archived(&pool).await.unwrap());

    unarchive_conversation(&pool, id).await.unwrap();
    assert!(!any_archived(&pool).await.unwrap());
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

/// Starting a stage records which roadmap it is a stage of **and which stage of
/// it**, in the one transaction that makes it a stage.
///
/// Both halves of one fact, on the one row — the label as the roadmap's own line
/// labels it, zero-padding kept, because that is the form everything else about a
/// roadmap names a stage in. Nothing derives it from the branch, which is why the
/// branch here is named for a stage the label does not match: what is stored is
/// what the caller read off the roadmap.
#[tokio::test]
async fn starting_a_stage_records_which_stage_of_which_roadmap_it_is() {
    let (_dir, pool) = fresh_pool().await;
    let id = drafted(&pool).await;

    assert_eq!(
        start_stage(
            &pool,
            id,
            "c0ffee",
            Path::new("/data/worktrees/stage"),
            Some("roadmaps/mvp/04-wrap-up"),
            RoadmapStage {
                roadmap: "mvp",
                label: "05",
            },
            &[],
        )
        .await
        .unwrap(),
        Staged::Started,
    );

    assert_eq!(
        stage_roadmap(&pool, id).await.unwrap(),
        Some(StageOf {
            roadmap: "mvp".to_owned(),
            stage: Some("05".to_owned()),
        }),
    );
}

/// A row holding a roadmap and no stage is not the same answer as no row at all,
/// and the read says which.
///
/// Three shapes and all three are told apart here: a stage Verkstead recorded
/// the label for, a stage from between the two changes whose row holds the
/// roadmap alone, and a Conversation nothing ever recorded a roadmap against.
/// Which of the middle one's two meanings it carries — a stage from before, or
/// the Conversation that wrote the roadmap — is read from what is stored beside
/// it, and [`stacks_on`] is that reading's first half.
#[tokio::test]
async fn a_stage_with_no_label_is_told_from_a_conversation_with_no_row() {
    let (_dir, pool) = fresh_pool().await;
    let repo_id = repo(&pool, "verkstead").await;

    let before = start_conversation(&pool, repo_id, "rate-limiting")
        .await
        .unwrap()
        .unwrap();

    start_stage(
        &pool,
        before,
        "c0ffee",
        Path::new("/data/worktrees/before"),
        None,
        RoadmapStage {
            roadmap: "mvp",
            label: "03",
        },
        &[],
    )
    .await
    .unwrap();

    // Which is what a stage started between ADR-0017 and the label looks like:
    // the roadmap on the record and nothing said about which stage.
    sqlx::query("UPDATE stage_roadmaps SET stage = NULL WHERE conversation_id = ?")
        .bind(before)
        .execute(&pool)
        .await
        .unwrap();

    assert_eq!(
        stage_roadmap(&pool, before).await.unwrap(),
        Some(StageOf {
            roadmap: "mvp".to_owned(),
            stage: None,
        }),
        "a roadmap and no label, which is a row rather than nothing",
    );
    assert_eq!(
        stacks_on(&pool, before).await.unwrap(),
        Some(None),
        "and a stage all the same, which is what says the empty label is a \
         stage's rather than a roadmap-writer's",
    );

    let never = start_conversation(&pool, repo_id, "amber-kestrel")
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        stage_roadmap(&pool, never).await.unwrap(),
        None,
        "and a Conversation nothing recorded a roadmap against has no row at all",
    );
    assert_eq!(stacks_on(&pool, never).await.unwrap(), None);
}

/// A Conversation that has picked a direction, which is the one thing a pick
/// needs: [`pick_direction`] is refused for anything but a grilling.
async fn directed(pool: &SqlitePool, repo_id: i64, branch: &str, direction: Direction) -> i64 {
    let id = start_conversation(pool, repo_id, branch)
        .await
        .unwrap()
        .unwrap();

    start_grilling(
        pool,
        id,
        "c0ffee",
        &Path::new("/data/worktrees").join(branch.replace('/', "-")),
        &[],
    )
    .await
    .unwrap();

    pick_direction(pool, id, direction).await.unwrap();

    id
}

/// A stage Conversation of `roadmap`, started as a stage and left implementing.
///
/// The branch is named after the label rather than read off it, which is what the
/// record is for: nothing here or anywhere else works a label out from a branch.
async fn stage(pool: &SqlitePool, repo_id: i64, roadmap: &str, label: &str) -> i64 {
    let id = start_conversation(pool, repo_id, &format!("roadmaps/{roadmap}/{label}"))
        .await
        .unwrap()
        .unwrap();

    start_stage(
        pool,
        id,
        "c0ffee",
        &PathBuf::from("/data/worktrees").join(format!("{roadmap}-{label}")),
        None,
        RoadmapStage { roadmap, label },
        &[],
    )
    .await
    .unwrap();

    id
}

/// A steer into `target` and nothing else with it, which is how a test walks a
/// stage up the ladder: the move is written the way every move is, so the
/// Timeline says where it has been.
fn steer(target: Lifecycle) -> Steer<'static> {
    Steer {
        target,
        pairings: &[],
        brief: None,
        instruction: None,
        direction: None,
        worktree: None,
        base: None,
        companions: &[],
        opened: &[],
        checkouts: &[],
        said: None,
        recorded: Recorded::default(),
        scratch: &[],
    }
}

/// What the record says about each stage of a roadmap, which is the reading both
/// of Verkstead's own readings of a roadmap are handed.
///
/// Every one of the three answers, and each of them from a Conversation that got
/// where it is the way a real one does: still implementing, finished, closed from
/// a wrap-up, and closed before there was one. And settled read as *ever in Done*
/// rather than *in Done now*, which is what the two halves at the foot of this
/// are.
#[tokio::test]
async fn the_record_says_which_stages_settled_which_are_in_flight_and_which_were_abandoned() {
    let (_dir, pool) = fresh_pool().await;
    let repo_id = repo(&pool, "verkstead").await;

    let done = stage(&pool, repo_id, "mvp", "01").await;
    steer_conversation(&pool, done, steer(Lifecycle::Done))
        .await
        .unwrap();

    let wrapped_up_then_closed = stage(&pool, repo_id, "mvp", "02").await;
    steer_conversation(&pool, wrapped_up_then_closed, steer(Lifecycle::Wrapping))
        .await
        .unwrap();
    close_conversation(&pool, wrapped_up_then_closed)
        .await
        .unwrap();

    let closed_part_way = stage(&pool, repo_id, "mvp", "03").await;
    close_conversation(&pool, closed_part_way).await.unwrap();

    // And one nothing has finished with, left implementing, which is where every
    // stage starts.
    let implementing = stage(&pool, repo_id, "mvp", "04").await;

    let standings = stage_standings(&pool, repo_id).await.unwrap();

    assert_eq!(
        standings.of("mvp", "01"),
        Some(StageStanding::Settled),
        "a stage in Done has settled, whether or not its pull request has merged",
    );
    assert_eq!(
        standings.of("mvp", "02"),
        Some(StageStanding::Settled),
        "and so has one closed from a wrap-up: the human saying they are finished with it",
    );
    assert_eq!(
        standings.of("mvp", "03"),
        Some(StageStanding::Abandoned),
        "one closed before it ever wrapped up is neither settled nor in flight",
    );
    assert_eq!(
        standings.of("mvp", "04"),
        Some(StageStanding::InFlight),
        "and one still being implemented is in flight",
    );
    assert_eq!(
        standings.of("mvp", "05"),
        None,
        "and a stage the record holds nothing about is nothing at all, which is \
         what leaves its box to speak for it",
    );

    // And settled once is settled: a stage steered out of Done — put back to work,
    // or followed up on months after it merged — has finished its work all the
    // same. Whether its branch is moving again is the chain's question rather than
    // this reading's, and reading it as in flight is what stopped the adoption
    // dead at a stage that was done.
    steer_conversation(&pool, done, steer(Lifecycle::Implementing))
        .await
        .unwrap();

    assert_eq!(
        stage_standings(&pool, repo_id)
            .await
            .unwrap()
            .of("mvp", "01"),
        Some(StageStanding::Settled),
    );

    // Where *never in Done* is the whole of what in flight means: the stage still
    // being implemented has been nowhere else, and steering it into Wrapping — one
    // rung short — leaves it there.
    steer_conversation(&pool, implementing, steer(Lifecycle::Wrapping))
        .await
        .unwrap();

    assert_eq!(
        stage_standings(&pool, repo_id)
            .await
            .unwrap()
            .of("mvp", "04"),
        Some(StageStanding::InFlight),
    );
}

/// And whether each of them has **stopped**, which is the reading beside the
/// standing rather than a fourth answer inside it: a stage that stopped is in
/// flight — it holds its place, its branch and its worktree — and what the stop
/// changes is what a human reading the roadmap is told about it.
#[tokio::test]
async fn the_record_says_which_stages_have_stopped_beside_where_each_of_them_got_to() {
    let (_dir, pool) = fresh_pool().await;
    let repo_id = repo(&pool, "verkstead").await;

    // One going and one stopped, both in flight.
    stage(&pool, repo_id, "mvp", "01").await;
    let halted = stage(&pool, repo_id, "mvp", "02").await;

    stop(
        &pool,
        halted,
        Decision::Verkstead,
        "The session fell over.",
        None,
    )
    .await
    .unwrap();

    let standings = stage_standings(&pool, repo_id).await.unwrap();

    assert_eq!(
        standings.of("mvp", "02"),
        Some(StageStanding::InFlight),
        "a stopped stage is still in flight: nothing about the stop settles it or \
         gives up its place",
    );
    assert!(standings.stopped("mvp", "02"));
    assert!(
        !standings.stopped("mvp", "01"),
        "and the one nothing has stopped is going on",
    );
    assert!(
        !standings.stopped("mvp", "03"),
        "as is a stage the record holds nothing about, there being nothing there \
         that could have stopped",
    );

    // And Resume takes it back, the stop being the standing state of a run rather
    // than something that happened to it once.
    clear_stop(&pool, halted).await.unwrap();

    assert!(
        !stage_standings(&pool, repo_id)
            .await
            .unwrap()
            .stopped("mvp", "02"),
    );
}

/// Two Conversations answer to one label where a stage was attempted twice, and
/// the roadmap has one line for it either way: settled counts over in flight, and
/// in flight over abandoned.
#[tokio::test]
async fn the_furthest_of_two_attempts_at_one_stage_is_what_the_record_says() {
    let (_dir, pool) = fresh_pool().await;
    let repo_id = repo(&pool, "verkstead").await;

    // Abandoned, and taken up again: somebody is on it now.
    let abandoned = stage(&pool, repo_id, "mvp", "01").await;
    close_conversation(&pool, abandoned).await.unwrap();
    let second = stage(&pool, repo_id, "mvp", "01").await;

    assert_eq!(
        stage_standings(&pool, repo_id)
            .await
            .unwrap()
            .of("mvp", "01"),
        Some(StageStanding::InFlight),
        "in flight over abandoned, the second attempt being the live one",
    );

    // And once that one finishes, the stage has settled — whichever order the
    // rows come back in.
    steer_conversation(&pool, second, steer(Lifecycle::Done))
        .await
        .unwrap();

    assert_eq!(
        stage_standings(&pool, repo_id)
            .await
            .unwrap()
            .of("mvp", "01"),
        Some(StageStanding::Settled),
    );
}

/// And the stop travels with the standing that is believed: the attempt whose
/// standing wins is the one whose stop is reported, because a stop belonging to
/// an attempt nobody is reading is nothing to say about the stage.
#[tokio::test]
async fn the_stop_reported_is_the_one_on_the_attempt_the_record_believes() {
    let (_dir, pool) = fresh_pool().await;
    let repo_id = repo(&pool, "verkstead").await;

    // The first attempt stopped and was closed part-way through; the second is
    // somebody's now, and going.
    let first = stage(&pool, repo_id, "mvp", "01").await;
    stop(
        &pool,
        first,
        Decision::Verkstead,
        "The session fell over.",
        None,
    )
    .await
    .unwrap();
    close_conversation(&pool, first).await.unwrap();

    stage(&pool, repo_id, "mvp", "01").await;

    let standings = stage_standings(&pool, repo_id).await.unwrap();

    assert_eq!(standings.of("mvp", "01"), Some(StageStanding::InFlight));
    assert!(
        !standings.stopped("mvp", "01"),
        "the live attempt is the one being read, and nothing has stopped it",
    );
}

/// The reading is one Repo's. A Conversation belongs to one Repo and two Repos may
/// hold roadmaps of the same name, so a stage of `mvp` over there answers for
/// nothing here.
#[tokio::test]
async fn one_repos_stages_do_not_answer_for_anothers() {
    let (_dir, pool) = fresh_pool().await;
    let ours = repo(&pool, "verkstead").await;
    let theirs = repo(&pool, "askance").await;

    let elsewhere = stage(&pool, theirs, "mvp", "01").await;
    steer_conversation(&pool, elsewhere, steer(Lifecycle::Done))
        .await
        .unwrap();

    assert_eq!(
        stage_standings(&pool, ours).await.unwrap().of("mvp", "01"),
        None,
        "the other Repo's finished stage 01 says nothing about this Repo's",
    );
    assert_eq!(
        stage_standings(&pool, theirs)
            .await
            .unwrap()
            .of("mvp", "01"),
        Some(StageStanding::Settled),
    );
}

/// And a row holding a roadmap and no label is no stage's standing: it is a stage
/// from before the label was written down, or the Conversation that wrote the
/// roadmap, and the boxes go on speaking for whatever it was.
#[tokio::test]
async fn a_row_with_no_label_stands_for_no_stage() {
    let (_dir, pool) = fresh_pool().await;
    let repo_id = repo(&pool, "verkstead").await;

    let before = stage(&pool, repo_id, "mvp", "01").await;
    steer_conversation(&pool, before, steer(Lifecycle::Done))
        .await
        .unwrap();

    sqlx::query("UPDATE stage_roadmaps SET stage = NULL WHERE conversation_id = ?")
        .bind(before)
        .execute(&pool)
        .await
        .unwrap();

    assert_eq!(
        stage_standings(&pool, repo_id).await.unwrap(),
        verkstead_store::StageStandings::default(),
        "a roadmap with no label against it puts nothing in the reading",
    );
}

/// One pull request opened on a stage's branch, which is what joins it to the
/// chain.
///
/// The number is the caller's, so two pull requests in one test cannot be
/// confused for each other, and the branch is the Conversation's: what a chain
/// holds is branches, and this is the one the finish pushed.
async fn opened(pool: &SqlitePool, repo_id: i64, id: i64, number: i64) {
    let branch = conversation_branch(pool, id)
        .await
        .unwrap()
        .expect("a Conversation is on a branch from the moment it exists");

    record_pull_request(
        pool,
        id,
        repo_id,
        &PullRequest {
            number,
            title: format!("Stage {number}"),
            url: format!("https://github.com/tobico/verkstead/pull/{number}"),
            head: Some(branch),
            base: None,
            repo: None,
        },
    )
    .await
    .unwrap();
}

/// The chain is the stages that opened a pull request, bottom to top, in the
/// order they opened one — and nothing else.
///
/// Which is the whole of what a join is: a stage joins its roadmap's chain when
/// its finish pushes its branch and opens a pull request on it, so the pull
/// request is the record of the join and the Timeline Event it hangs off is the
/// record of when. A stage still working its backlog has pushed nothing and is
/// no link of anything.
///
/// The order is the order they *finished* rather than the order the roadmap
/// numbers them in, which is the case worth writing down: stage 02 finishing
/// first puts stage 02 at the foot of the chain, and stage 01 joins on top of
/// it.
#[tokio::test]
async fn a_roadmaps_chain_is_the_stages_that_opened_a_pull_request_in_that_order() {
    let (_dir, pool) = fresh_pool().await;
    let repo_id = repo(&pool, "verkstead").await;

    let first = stage(&pool, repo_id, "mvp", "02").await;
    let second = stage(&pool, repo_id, "mvp", "01").await;

    // And one still working its backlog, which has pushed nothing and opened
    // nothing: a stage nothing can rebase onto is no link of the chain.
    stage(&pool, repo_id, "mvp", "03").await;

    assert_eq!(
        stage_chain(&pool, repo_id, "mvp").await.unwrap(),
        Vec::new(),
        "a roadmap whose stages have opened nothing has no chain yet",
    );

    opened(&pool, repo_id, first, 42).await;
    opened(&pool, repo_id, second, 43).await;

    assert_eq!(
        stage_chain(&pool, repo_id, "mvp").await.unwrap(),
        vec![
            Joined {
                conversation_id: first,
                stage: "02".to_owned(),
                branch: "roadmaps/mvp/02".to_owned(),
            },
            Joined {
                conversation_id: second,
                stage: "01".to_owned(),
                branch: "roadmaps/mvp/01".to_owned(),
            },
        ],
        "the order they joined in, which is the order they finished in",
    );
}

/// A stage that opens a second pull request keeps the place its first one gave
/// it.
///
/// A stack is what a finished roadmap leaves behind, so a Conversation may have
/// more than one pull request recorded against it — the rest of a stack, a
/// companion repository's. What says when this stage joined is the first of
/// them, and a later row must not float it back to the top of the chain above
/// stages that joined while it was wrapping up.
#[tokio::test]
async fn a_stage_joins_the_chain_where_its_first_pull_request_put_it() {
    let (_dir, pool) = fresh_pool().await;
    let repo_id = repo(&pool, "verkstead").await;

    let below = stage(&pool, repo_id, "mvp", "01").await;
    let above = stage(&pool, repo_id, "mvp", "02").await;

    opened(&pool, repo_id, below, 42).await;
    opened(&pool, repo_id, above, 43).await;

    record_another_pull_request(
        &pool,
        below,
        repo_id,
        &PullRequest {
            number: 44,
            title: "The rest of the stack".to_owned(),
            url: "https://github.com/tobico/verkstead/pull/44".to_owned(),
            head: Some("roadmaps/mvp/01".to_owned()),
            base: None,
            repo: None,
        },
    )
    .await
    .unwrap();

    assert_eq!(
        stage_chain(&pool, repo_id, "mvp")
            .await
            .unwrap()
            .into_iter()
            .map(|link| link.stage)
            .collect::<Vec<_>>(),
        vec!["01".to_owned(), "02".to_owned()],
        "the first pull request is what says when a stage joined",
    );
}

/// The chain is one Repo's and one roadmap's. Two Repos may hold roadmaps of
/// the same name, and one Repo holds every roadmap it has ever run — so a stage
/// of `mvp` over there, and a stage of `processes` over here, are links of
/// neither chain.
#[tokio::test]
async fn a_chain_holds_no_other_repos_stages_and_no_other_roadmaps() {
    let (_dir, pool) = fresh_pool().await;
    let ours = repo(&pool, "verkstead").await;
    let theirs = repo(&pool, "askance").await;

    let mine = stage(&pool, ours, "mvp", "01").await;
    opened(&pool, ours, mine, 42).await;

    let beside = stage(&pool, ours, "processes", "01").await;
    opened(&pool, ours, beside, 43).await;

    let elsewhere = stage(&pool, theirs, "mvp", "01").await;
    opened(&pool, theirs, elsewhere, 44).await;

    assert_eq!(
        stage_chain(&pool, ours, "mvp")
            .await
            .unwrap()
            .into_iter()
            .map(|link| link.conversation_id)
            .collect::<Vec<_>>(),
        vec![mine],
        "one roadmap of one Repo, and neither of the other two",
    );
}

/// The queue to join is the stages whose tasks have finished, in the order they
/// finished them — which is the order they take their places in.
///
/// One stage at a time joins a roadmap's chain, and which of two whose boxes were
/// both ticked goes first cannot be the order a server happened to notice them
/// in. So the place is a row, and it is the row's own order that answers.
///
/// The numbers the roadmap gives the stages say nothing about it: stage 03
/// finishing its tasks first is stage 03 joining first.
#[tokio::test]
async fn a_roadmaps_queue_to_join_is_the_order_the_stages_tasks_finished_in() {
    let (_dir, pool) = fresh_pool().await;
    let repo_id = repo(&pool, "verkstead").await;

    let first = stage(&pool, repo_id, "mvp", "03").await;
    let second = stage(&pool, repo_id, "mvp", "01").await;

    // And one still working its backlog, which has taken no place at all.
    stage(&pool, repo_id, "mvp", "02").await;

    assert_eq!(
        join_queue(&pool, repo_id, "mvp").await.unwrap(),
        Vec::new(),
        "a roadmap none of whose stages has finished its tasks has nobody queued",
    );

    queue_to_join(&pool, first).await.unwrap();
    queue_to_join(&pool, second).await.unwrap();

    assert_eq!(
        join_queue(&pool, repo_id, "mvp").await.unwrap(),
        vec![
            Queued {
                conversation_id: first,
                stage: "03".to_owned(),
            },
            Queued {
                conversation_id: second,
                stage: "01".to_owned(),
            },
        ],
        "the order their tasks finished in, whatever the roadmap numbers them",
    );
}

/// And a stage taken up again keeps the place it already had.
///
/// The wait may be long, and a restart takes every stage left mid-run up from
/// where it stands — so a stage held before its finish is asked the same question
/// a second time and a third. A place written afresh each time would be a queue a
/// restart reordered, which is the whole reason it is a stored fact.
#[tokio::test]
async fn a_stage_taken_up_again_keeps_the_place_the_queue_gave_it() {
    let (_dir, pool) = fresh_pool().await;
    let repo_id = repo(&pool, "verkstead").await;

    let first = stage(&pool, repo_id, "mvp", "01").await;
    let second = stage(&pool, repo_id, "mvp", "02").await;

    queue_to_join(&pool, first).await.unwrap();
    queue_to_join(&pool, second).await.unwrap();

    // Which is what a restart does: the stage in front is taken up again, looks
    // at the queue again, and asks for a place it already has.
    queue_to_join(&pool, first).await.unwrap();

    assert_eq!(
        join_queue(&pool, repo_id, "mvp")
            .await
            .unwrap()
            .into_iter()
            .map(|queued| queued.conversation_id)
            .collect::<Vec<_>>(),
        vec![first, second],
        "the same order it would have been without the restart",
    );
}

/// The queue is one Repo's and one roadmap's, and holds only the rows that say
/// which stage they are.
///
/// [`stage_chain`]'s reasons, and [`stage_standings`]'s: two Repos may hold
/// roadmaps of the same name, one Repo holds every roadmap it has ever run, and a
/// row with no label stands for no stage — the Conversation that *wrote* a
/// roadmap has one, and it joins nothing.
#[tokio::test]
async fn a_queue_holds_no_other_repos_stages_no_other_roadmaps_and_no_unlabelled_row() {
    let (_dir, pool) = fresh_pool().await;
    let ours = repo(&pool, "verkstead").await;
    let theirs = repo(&pool, "askance").await;

    let mine = stage(&pool, ours, "mvp", "01").await;
    queue_to_join(&pool, mine).await.unwrap();

    let beside = stage(&pool, ours, "processes", "01").await;
    queue_to_join(&pool, beside).await.unwrap();

    let elsewhere = stage(&pool, theirs, "mvp", "01").await;
    queue_to_join(&pool, elsewhere).await.unwrap();

    // The Conversation that wrote the roadmap: a `stage_roadmaps` row with the
    // roadmap and no label.
    let wrote = start_conversation(&pool, ours, "roadmaps/mvp")
        .await
        .unwrap()
        .unwrap();
    record_roadmap(&pool, wrote, Some("mvp")).await.unwrap();
    queue_to_join(&pool, wrote).await.unwrap();

    assert_eq!(
        join_queue(&pool, ours, "mvp")
            .await
            .unwrap()
            .into_iter()
            .map(|queued| queued.conversation_id)
            .collect::<Vec<_>>(),
        vec![mine],
        "one roadmap of one Repo, and no row that is not a stage of it",
    );
}

/// The Conversation that *wrote* a roadmap comes back by its branch, which is the
/// foot of that roadmap's chain while its pull request is unmerged.
///
/// What says a row is that Conversation's is the roadmap with no stage label
/// beside it *and* the Roadmap direction — so a stage of the same roadmap is not
/// it, and neither is an unlabelled row left by a Conversation headed somewhere
/// else, which is what a stage started before the label was written down is.
#[tokio::test]
async fn the_branch_a_roadmap_was_planned_on_is_its_own_conversations() {
    let (_dir, pool) = fresh_pool().await;
    let ours = repo(&pool, "verkstead").await;
    let theirs = repo(&pool, "askance").await;

    // A stage of it, which holds a label and is nobody's planning branch.
    stage(&pool, ours, "mvp", "01").await;

    // A row with no label and another direction: a stage Verkstead started
    // before it wrote the label down.
    let before = directed(&pool, ours, "mvp/02-grilling", Direction::TaskList).await;
    record_roadmap(&pool, before, Some("mvp")).await.unwrap();

    // Somebody else's repository, planning a roadmap of the same name.
    let elsewhere = directed(&pool, theirs, "the-mvp-roadmap", Direction::Roadmap).await;
    record_roadmap(&pool, elsewhere, Some("mvp")).await.unwrap();

    assert_eq!(
        roadmap_planner(&pool, ours, "mvp").await.unwrap(),
        None,
        "nothing in this Repo has planned it: a label is not it, and neither is a direction \
         of something else",
    );

    let wrote = directed(&pool, ours, "roadmaps/mvp", Direction::Roadmap).await;
    record_roadmap(&pool, wrote, Some("mvp")).await.unwrap();

    assert_eq!(
        roadmap_planner(&pool, ours, "mvp").await.unwrap(),
        Some(Planned {
            conversation_id: wrote,
            branch: "roadmaps/mvp".to_owned(),
        }),
        "both halves of the foot: the Conversation that planned it, and the branch it \
         planned it on",
    );

    assert_eq!(
        roadmap_planner(&pool, ours, "public-release")
            .await
            .unwrap(),
        None,
        "and one roadmap's planning branch says nothing about another's",
    );

    // And the name a session renamed the branch to is the name that comes back,
    // for the reason the chain's links come back under theirs: what the record
    // holds now is what git holds now.
    follow_branch(&pool, wrote, "roadmaps/the-mvp")
        .await
        .unwrap();

    assert_eq!(
        roadmap_planner(&pool, ours, "mvp")
            .await
            .unwrap()
            .map(|planned| planned.branch)
            .as_deref(),
        Some("roadmaps/the-mvp"),
    );
}

/// The roadmaps being driven come back **oldest first**, which is the order the
/// free places on the server are handed out in.
///
/// Oldest is the roadmap's own age — the Conversation that wrote it — rather
/// than its name, the Repo it is in or how long any stage of it has been
/// waiting. So the order here is deliberately none of the three the rows would
/// otherwise fall into: the younger roadmap is in the Repo registered first and
/// is named first alphabetically, and it still comes second.
#[tokio::test]
async fn the_roadmaps_being_driven_come_back_oldest_first() {
    let (_dir, pool) = fresh_pool().await;
    let ours = repo(&pool, "verkstead").await;
    let theirs = repo(&pool, "askance").await;

    // The older of the two, and the one that would sort last on every other
    // reading: its Repo was registered second and its name begins with an `r`.
    let older = directed(&pool, theirs, "roadmaps/rate-limiting", Direction::Roadmap).await;
    record_roadmap(&pool, older, Some("rate-limiting"))
        .await
        .unwrap();

    // The younger, planned after it. Two stages of it as well, which say nothing
    // about its age: what a roadmap is aged off is the Conversation that wrote
    // it, and a stage started this morning does not make the roadmap young.
    let younger = directed(&pool, ours, "roadmaps/brain-chat", Direction::Roadmap).await;
    record_roadmap(&pool, younger, Some("brain-chat"))
        .await
        .unwrap();

    stage(&pool, ours, "brain-chat", "01").await;
    stage(&pool, ours, "brain-chat", "02").await;

    assert_eq!(
        driven_roadmaps(&pool).await.unwrap(),
        vec![
            Driven {
                repo_id: theirs,
                roadmap: "rate-limiting".to_owned(),
            },
            Driven {
                repo_id: ours,
                roadmap: "brain-chat".to_owned(),
            },
        ],
        "the roadmap written first is served first, whatever the names and the Repos are",
    );

    // And a roadmap nobody planned ages off the first stage of it anybody
    // adopted, there being no planning Conversation to ask. Adopted now, so it
    // is the youngest of the three.
    stage(&pool, ours, "public-release", "07").await;

    assert_eq!(
        driven_roadmaps(&pool)
            .await
            .unwrap()
            .into_iter()
            .map(|driven| driven.roadmap)
            .collect::<Vec<_>>(),
        ["rate-limiting", "brain-chat", "public-release"],
        "a roadmap adopted stage by stage is as old as the stage that adopted it",
    );
}
