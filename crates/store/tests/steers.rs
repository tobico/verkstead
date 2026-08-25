//! Steers: the human moving a Conversation into the state they say it belongs
//! in, from wherever it had got to.
//!
//! Two Events per steer, and the pair is the point. The Steer is the human's own
//! — *I moved this* — and the Moved line beside it is the machine's plain record
//! of the transition, the same line every other move leaves. A Timeline with
//! only the second could never be read back for who decided.
//!
//! And the Pairing where the human picked one, written in the same transaction:
//! steering re-settles what runs the work, and a Conversation moved into a state
//! something runs in without the Pairing those sessions run under would be a
//! move only half made.
//!
//! And what the human wrote to steer with: the instruction a steer into
//! Implementing sends its session off with, which is the Steer Event's own body
//! rather than a document beside it.
//!
//! And the round a steer into Grilling opens: the Brief the human wrote for it,
//! frozen where it lands, the wrap-up bookkeeping of the round before it
//! forgotten, and the Worktree and base commit the steer had to make written
//! beside the move.
//!
//! Nothing here is about what runs afterwards, and nothing here *makes*
//! anything. Checking a branch out, clearing a stop and launching a session are
//! the server's, and are asked of it there; what this is about is that the
//! record of one steer is written whole or not at all.

use std::path::{Path, PathBuf};

use sqlx::SqlitePool;
use verkstead_schema::Direction;
use verkstead_store::{
    AgentType, Directing, Edited, Event, Lifecycle, ProfileFacts, Role, Settling, Steer, Steering,
    WaitingOn, create_profile, fix_attempts, load_conversation, open_database, pick_direction,
    record_fix_attempt, register_repo, save_brief, settle_wrap_up, start_conversation,
    start_grilling, steer_conversation, timeline, wrap_up_settled,
};

/// The plainest steer there is: the move and nothing beside it.
///
/// What every one here starts from, with whatever it is about written over the
/// top of it. A steer settles a Pairing, opens a round with a Brief and records
/// the Worktree it had to make only where the human's press said so, and the
/// ordinary press says none of it.
fn into(target: Lifecycle) -> Steer<'static> {
    Steer {
        target,
        pairing: None,
        brief: None,
        instruction: None,
        direction: None,
        worktree: None,
        base_commit: None,
    }
}

/// A pool over a fresh database, plus the directory keeping it alive.
async fn fresh_pool() -> (tempfile::TempDir, SqlitePool) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();
    (dir, pool)
}

/// A Conversation still drafting: the state every other one here is grilled out
/// of, and a source for a steer like any other.
async fn drafting(pool: &SqlitePool) -> i64 {
    let repo = register_repo(pool, Path::new("/srv/verkstead"), "verkstead", "main")
        .await
        .unwrap()
        .expect("nothing is registered at that path yet");

    let id = start_conversation(pool, repo.id, "rate-limiting")
        .await
        .unwrap()
        .expect("the Repo was just registered");

    save_brief(pool, id, "# Rate limiting\n").await.unwrap();

    id
}

/// And one being grilled, which is a Conversation with a branch and a Worktree
/// behind it.
async fn grilling(pool: &SqlitePool) -> i64 {
    let id = drafting(pool).await;

    start_grilling(
        pool,
        id,
        "c0ffee",
        Path::new("/state/worktrees/rate-limiting"),
    )
    .await
    .unwrap();

    id
}

/// An Agent Profile to pair with, by name and with the models it lists.
async fn profile(pool: &SqlitePool, name: &str, models: &[&str]) -> i64 {
    create_profile(
        pool,
        &ProfileFacts {
            name: name.to_owned(),
            claude_dir: PathBuf::from(format!("/state/profiles/{name}")),
            config_file: PathBuf::from(format!("/state/profiles/{name}.json")),
            models: models.iter().map(|model| (*model).to_owned()).collect(),
            agent_type: AgentType::Claude,
        },
    )
    .await
    .unwrap()
    .expect("nothing is called that yet")
    .id
}

/// Where a Conversation says it has got to.
async fn state(pool: &SqlitePool, id: i64) -> Lifecycle {
    load_conversation(pool, id)
        .await
        .unwrap()
        .expect("the Conversation is there")
        .state
}

/// Every Brief on its Timeline, oldest first.
///
/// All of them rather than the newest: a round steered into gets a Brief of its
/// own, and what says it is a second one beside the first is that the first is
/// still there.
async fn briefs(pool: &SqlitePool, id: i64) -> Vec<String> {
    timeline(pool, id)
        .await
        .unwrap()
        .into_iter()
        .filter_map(|event| match event.event {
            Event::Brief(markdown) => Some(markdown),
            _ => None,
        })
        .collect()
}

/// Its Timeline as the kinds that say where the work went: the states it was
/// steered into and the states it moved to, in the order they landed.
async fn ladder(pool: &SqlitePool, id: i64) -> Vec<(&'static str, Lifecycle)> {
    timeline(pool, id)
        .await
        .unwrap()
        .into_iter()
        .filter_map(|event| match event.event {
            Event::Steer(target, _) => Some(("steer", target)),
            Event::Moved(state) => Some(("moved", state)),
            _ => None,
        })
        .collect()
}

#[tokio::test]
async fn a_steer_moves_the_conversation_and_leaves_the_two_events_of_one() {
    let (_dir, pool) = fresh_pool().await;
    let id = grilling(&pool).await;

    assert_eq!(
        steer_conversation(&pool, id, into(Lifecycle::Done))
            .await
            .unwrap(),
        Steering::Steered,
    );

    assert_eq!(state(&pool, id).await, Lifecycle::Done);

    assert_eq!(
        ladder(&pool, id).await,
        [
            ("moved", Lifecycle::Grilling),
            ("steer", Lifecycle::Done),
            ("moved", Lifecycle::Done),
        ],
        "the human's own line first and the machine's move under it: the act, \
         and then what came of it",
    );
}

#[tokio::test]
async fn every_state_is_somewhere_to_be_steered_from() {
    let (_dir, pool) = fresh_pool().await;

    // A draft, which nothing has ever run in, and a Conversation Verkstead has
    // already finished with. Neither is a rung the pipeline would move from, and
    // both are the human's to move.
    let draft = drafting(&pool).await;

    assert_eq!(
        steer_conversation(&pool, draft, into(Lifecycle::Done))
            .await
            .unwrap(),
        Steering::Steered,
    );
    assert_eq!(state(&pool, draft).await, Lifecycle::Done);

    assert_eq!(
        steer_conversation(&pool, draft, into(Lifecycle::Done))
            .await
            .unwrap(),
        Steering::Steered,
        "a Conversation steered where it already is is steered there again: \
         the human said so, and there is no state here to be wrong about",
    );

    assert_eq!(
        ladder(&pool, draft).await,
        [
            ("steer", Lifecycle::Done),
            ("moved", Lifecycle::Done),
            ("steer", Lifecycle::Done),
            ("moved", Lifecycle::Done),
        ],
    );
}

/// A Pairing picked in the modal is recorded as the Conversation's own — both
/// halves of it — and it is recorded long past drafting, which is the whole of
/// why a steer does not go through the drafting pickers' own call.
#[tokio::test]
async fn a_steer_settles_the_pairing_the_human_picked() {
    let (_dir, pool) = fresh_pool().await;
    let id = grilling(&pool).await;
    let profile = profile(&pool, "opus", &["opus-5", "opus-4.8"]).await;

    assert_eq!(
        steer_conversation(
            &pool,
            id,
            Steer {
                pairing: Some(Settling {
                    role: Role::Implementation,
                    profile_id: profile,
                    model: "opus-4.8",
                }),
                ..into(Lifecycle::Wrapping)
            },
        )
        .await
        .unwrap(),
        Steering::Steered,
    );

    let conversation = load_conversation(&pool, id)
        .await
        .unwrap()
        .expect("the Conversation is there");

    assert_eq!(conversation.state, Lifecycle::Wrapping);

    let paired = conversation
        .implementation_pairing
        .expect("the steer settled one");

    assert_eq!(paired.profile.id, profile);
    assert_eq!(
        paired.model.as_deref(),
        Some("opus-4.8"),
        "both halves of it: either alone is not something to launch a session \
         with",
    );
    assert!(
        conversation.grilling_pairing.is_none(),
        "and only the role steered into: the other is nobody's to re-settle here",
    );
}

/// A Profile that went between the list the modal read and the pick it made from
/// it takes the whole steer with it.
///
/// The move and the Pairing are one act, so a pick that cannot be written is a
/// move that does not happen: a Conversation wrapping under a Profile that is
/// not there would be work nothing could start.
#[tokio::test]
async fn a_steer_naming_a_profile_that_has_gone_moves_nothing() {
    let (_dir, pool) = fresh_pool().await;
    let id = grilling(&pool).await;

    assert_eq!(
        steer_conversation(
            &pool,
            id,
            Steer {
                pairing: Some(Settling {
                    role: Role::Implementation,
                    profile_id: 404,
                    model: "opus-5",
                }),
                ..into(Lifecycle::Wrapping)
            },
        )
        .await
        .unwrap(),
        Steering::NoSuchProfile,
    );

    assert_eq!(
        state(&pool, id).await,
        Lifecycle::Grilling,
        "it is where it was",
    );
    assert_eq!(
        ladder(&pool, id).await,
        [("moved", Lifecycle::Grilling)],
        "and nothing on the Timeline says otherwise",
    );
}

/// A steer into Grilling opens a round, and what the human wrote in the modal is
/// that round's Brief: a second Brief Event beside the first rather than an edit
/// of it, frozen the moment it lands.
///
/// Frozen because the round it opens is past drafting, which is the only state a
/// Brief can be edited in — so the same [`save_brief`] that would have written
/// over a draft's own is refused on this one.
#[tokio::test]
async fn a_steer_into_grilling_with_a_brief_opens_a_round_with_it() {
    let (_dir, pool) = fresh_pool().await;
    let id = grilling(&pool).await;

    assert_eq!(
        steer_conversation(
            &pool,
            id,
            Steer {
                brief: Some("# Retries\n\nThe backoff is wrong.\n"),
                ..into(Lifecycle::Grilling)
            },
        )
        .await
        .unwrap(),
        Steering::Steered,
    );

    assert_eq!(
        briefs(&pool, id).await,
        [
            "# Rate limiting\n".to_owned(),
            "# Retries\n\nThe backoff is wrong.\n".to_owned(),
        ],
        "the round before it was built from what it was built from, and that \
         stays on the record beside the new one",
    );

    assert_eq!(
        ladder(&pool, id).await,
        [
            ("moved", Lifecycle::Grilling),
            ("steer", Lifecycle::Grilling),
            ("moved", Lifecycle::Grilling),
        ],
    );

    assert_eq!(
        save_brief(&pool, id, "# Something else\n").await.unwrap(),
        Edited::NotDrafting,
        "and it is frozen: the round it opened has no Draft to leave",
    );
}

/// And one without leaves the Steer Event alone: the round starts on the Brief
/// that is already there.
#[tokio::test]
async fn a_steer_into_grilling_without_one_writes_no_brief() {
    let (_dir, pool) = fresh_pool().await;
    let id = grilling(&pool).await;

    assert_eq!(
        steer_conversation(&pool, id, into(Lifecycle::Grilling))
            .await
            .unwrap(),
        Steering::Steered,
    );

    assert_eq!(
        briefs(&pool, id).await,
        ["# Rate limiting\n".to_owned()],
        "the one the round is grilled on, and nothing written over it",
    );
}

/// The round before a steered-into grilling is over, so its wrap-up bookkeeping
/// is forgotten — the same forgetting a reopened Conversation does.
///
/// A round that inherited the one before it would reach Wrapping with everything
/// wrap-up waits on already settled and would be over the moment it arrived. The
/// steers that open no round leave all of it exactly where it is: a wrap-up
/// steered back into wrapping up is the *same* round, looked at again.
#[tokio::test]
async fn a_steer_into_grilling_forgets_the_round_before_it() {
    let (_dir, pool) = fresh_pool().await;
    let id = grilling(&pool).await;

    settle_wrap_up(&pool, id, WaitingOn::Review).await.unwrap();
    record_fix_attempt(&pool, id, "Rust").await.unwrap();

    steer_conversation(&pool, id, into(Lifecycle::Wrapping))
        .await
        .unwrap();

    assert_eq!(
        wrap_up_settled(&pool, id).await.unwrap(),
        [WaitingOn::Review],
        "a wrap-up steered into wrapping up is the same round, looked at again",
    );
    assert_eq!(fix_attempts(&pool, id, "Rust").await.unwrap(), 1);

    steer_conversation(&pool, id, into(Lifecycle::Grilling))
        .await
        .unwrap();

    assert!(
        wrap_up_settled(&pool, id).await.unwrap().is_empty(),
        "and the round that starts here waits on all of it from nothing",
    );
    assert_eq!(fix_attempts(&pool, id, "Rust").await.unwrap(), 0);
}

/// What the steer had to make before anything could run in it: the Worktree it
/// checked out, and — for a Draft, which has never had a branch — the commit
/// that branch was cut from.
///
/// Recorded here rather than made here. Git and the filesystem are the server's
/// to reach, and after this there is a fact about what the work branched from
/// rather than a rule about what it would have.
#[tokio::test]
async fn a_steer_records_the_worktree_and_the_commit_it_branched_from() {
    let (_dir, pool) = fresh_pool().await;
    let draft = drafting(&pool).await;

    assert_eq!(
        steer_conversation(
            &pool,
            draft,
            Steer {
                worktree: Some(Path::new("/state/worktrees/rate-limiting")),
                base_commit: Some("c0ffee"),
                ..into(Lifecycle::Grilling)
            },
        )
        .await
        .unwrap(),
        Steering::Steered,
    );

    let conversation = load_conversation(&pool, draft)
        .await
        .unwrap()
        .expect("the Conversation is there");

    assert_eq!(conversation.state, Lifecycle::Grilling);
    assert_eq!(
        conversation.worktree.as_deref(),
        Some(Path::new("/state/worktrees/rate-limiting")),
    );
    assert_eq!(
        conversation.base_commit.as_deref(),
        Some("c0ffee"),
        "the column held the branch the human picked while drafting, and now \
         holds what that resolved to",
    );
}

/// A steer into Implementing leaves the direction as it found it.
///
/// What says how the work is being built is the Conversation's own pick, and a
/// steer that carries on what already stands changes nothing about that: the
/// backlog it picks up is the backlog that pick led to. Nothing here writes the
/// column, and this is what says so.
#[tokio::test]
async fn a_steer_into_implementing_leaves_the_direction_it_found() {
    let (_dir, pool) = fresh_pool().await;
    let id = grilling(&pool).await;

    assert_eq!(
        pick_direction(&pool, id, Direction::TaskList)
            .await
            .unwrap(),
        Directing::Writing,
    );

    assert_eq!(
        steer_conversation(&pool, id, into(Lifecycle::Implementing))
            .await
            .unwrap(),
        Steering::Steered,
    );

    let conversation = load_conversation(&pool, id)
        .await
        .unwrap()
        .expect("the Conversation is there");

    assert_eq!(conversation.state, Lifecycle::Implementing);
    assert_eq!(conversation.direction, Some(Direction::TaskList));
}

/// Every instruction on its Timeline, in the order it was written.
///
/// The Steer Event's own body rather than an Event beside it, which is the whole
/// of what says the record holds what was asked for: a steer read back is the
/// state it named *and* the job it set.
async fn instructions(pool: &SqlitePool, id: i64) -> Vec<Option<String>> {
    timeline(pool, id)
        .await
        .unwrap()
        .into_iter()
        .filter_map(|event| match event.event {
            Event::Steer(_, instruction) => Some(instruction),
            _ => None,
        })
        .collect()
}

/// A steer carrying an instruction leaves it on the Steer Event, word for word.
///
/// Kept as the human wrote it, markdown and all: what they typed is the whole of
/// what the session it starts was asked to do, so a record that had tidied it
/// would be a record of something slightly else.
#[tokio::test]
async fn a_steer_with_an_instruction_keeps_it_as_the_steers_own_body() {
    let (_dir, pool) = fresh_pool().await;
    let id = grilling(&pool).await;

    let instruction = "Rebase this onto `main`.\n\nThen run the tests.\n";

    assert_eq!(
        steer_conversation(
            &pool,
            id,
            Steer {
                instruction: Some(instruction),
                direction: Some(Direction::Inline),
                ..into(Lifecycle::Implementing)
            },
        )
        .await
        .unwrap(),
        Steering::Steered,
    );

    assert_eq!(
        instructions(&pool, id).await,
        [Some(instruction.to_owned())],
        "the document the human wrote, on the Event that says they wrote it",
    );

    assert_eq!(
        ladder(&pool, id).await,
        [
            ("moved", Lifecycle::Grilling),
            ("steer", Lifecycle::Implementing),
            ("moved", Lifecycle::Implementing),
        ],
        "and the pair around it unchanged: the instruction rides on the human's \
         own line rather than beside it",
    );

    assert!(
        briefs(&pool, id).await.len() == 1,
        "and nothing opened a round: an instruction is one session's job rather \
         than what a round is grilled about",
    );
}

/// A steer that carries no instruction leaves the Steer Event saying the state
/// alone.
///
/// Which is every steer written before there was one to write, and how they read
/// back — the target above the document rather than under it, so a body of one
/// line is the steer it always was.
#[tokio::test]
async fn a_steer_with_nothing_written_says_the_state_alone() {
    let (_dir, pool) = fresh_pool().await;
    let id = grilling(&pool).await;

    steer_conversation(&pool, id, into(Lifecycle::Done))
        .await
        .unwrap();

    assert_eq!(instructions(&pool, id).await, [None]);
}

/// A Conversation that has never said how its work is built is recorded as
/// building it inline.
///
/// An instruction session is the whole of the work in one session, which is what
/// inline means — and a state something runs in with nothing saying how is a
/// record a pressed Resume refuses on by name. So the steer settles it as it
/// moves, and the Conversation it leaves behind is one that can be started
/// again.
#[tokio::test]
async fn a_steer_records_how_the_work_is_built_where_nothing_said() {
    let (_dir, pool) = fresh_pool().await;
    let id = grilling(&pool).await;

    assert_eq!(
        steer_conversation(
            &pool,
            id,
            Steer {
                instruction: Some("Fix the flaky test.\n"),
                direction: Some(Direction::Inline),
                ..into(Lifecycle::Implementing)
            },
        )
        .await
        .unwrap(),
        Steering::Steered,
    );

    let conversation = load_conversation(&pool, id)
        .await
        .unwrap()
        .expect("the Conversation is there");

    assert_eq!(conversation.direction, Some(Direction::Inline));
}

/// And one that has said is left exactly as it was.
///
/// The rule is the record's rather than the caller's, which is what this asks:
/// a direction offered over a Conversation that already picked one is not
/// written. What says how the work is built is the human's own pick, and the
/// instruction session that runs beside a backlog does not turn that backlog
/// into an inline run.
#[tokio::test]
async fn a_steer_never_writes_over_how_the_work_is_already_built() {
    let (_dir, pool) = fresh_pool().await;
    let id = grilling(&pool).await;

    assert_eq!(
        pick_direction(&pool, id, Direction::TaskList)
            .await
            .unwrap(),
        Directing::Writing,
    );

    assert_eq!(
        steer_conversation(
            &pool,
            id,
            Steer {
                instruction: Some("Fix the flaky test first.\n"),
                direction: Some(Direction::Inline),
                ..into(Lifecycle::Implementing)
            },
        )
        .await
        .unwrap(),
        Steering::Steered,
    );

    let conversation = load_conversation(&pool, id)
        .await
        .unwrap()
        .expect("the Conversation is there");

    assert_eq!(conversation.direction, Some(Direction::TaskList));
}

#[tokio::test]
async fn there_is_no_conversation_to_steer() {
    let (_dir, pool) = fresh_pool().await;

    assert_eq!(
        steer_conversation(&pool, 404, into(Lifecycle::Done))
            .await
            .unwrap(),
        Steering::NoSuchConversation,
    );
}

#[tokio::test]
async fn a_steer_survives_the_database_being_reopened() {
    let dir = tempfile::tempdir().unwrap();
    let database = dir.path().join("verkstead.db");

    let id = {
        let pool = open_database(&database).await.unwrap();
        let id = grilling(&pool).await;
        steer_conversation(&pool, id, into(Lifecycle::Done))
            .await
            .unwrap();
        pool.close().await;
        id
    };

    // The read is the half that matters: an Event of a kind this build cannot
    // read is an error rather than a row it draws around, so a steer written by
    // one process and read by another is what says the kind is on both sides.
    let pool = open_database(&database).await.unwrap();

    assert_eq!(state(&pool, id).await, Lifecycle::Done);
    assert_eq!(
        ladder(&pool, id).await,
        [
            ("moved", Lifecycle::Grilling),
            ("steer", Lifecycle::Done),
            ("moved", Lifecycle::Done),
        ],
    );
}
