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
//! And the state the steer found it in, beside the Steer Event: where the work
//! went is the Event's own, and where it came from is nowhere at all the moment
//! the state column is written over. An Investigating steered into goes back to
//! it when the human says there is nothing else, so it is a fact somebody
//! recorded rather than one read back off the Timeline afterwards.
//!
//! And the review a steer into Wrapping puts back to waiting, from whatever
//! state it was steered: a steer is the human saying look at this again, so the
//! wrap-up it lands in reads the branch rather than inheriting what the last one
//! made of it.
//!
//! Nothing here is about what runs afterwards, and nothing here *makes*
//! anything. Checking a branch out, clearing a stop and launching a session are
//! the server's, and are asked of it there; what this is about is that the
//! record of one steer is written whole or not at all.

use std::path::{Path, PathBuf};

use sqlx::SqlitePool;
use verkstead_schema::Direction;
use verkstead_store::{
    Account, Adding, Base, CompanionMode, Deleting, Directing, Edited, Event, Joining, Lifecycle,
    Opening, PickedPairing, ProfileFacts, Recorded, RecordedPairing, Role, Settling, Steer,
    SteerRecord, Steering, WaitingOn, add_companion, create_profile, delete_profile, fix_attempts,
    load_conversation, open_database, pick_direction, record_fix_attempt, register_repo,
    save_brief, settle_naming, settle_wrap_up, start_conversation, start_grilling,
    start_unnamed_conversation, steer_conversation, timeline, wrap_up_settled,
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
        &[],
    )
    .await
    .unwrap();

    id
}

/// A Draft nobody has named, which is the only source the instruction is ever
/// about.
async fn unnamed(pool: &SqlitePool) -> i64 {
    let repo = register_repo(pool, Path::new("/srv/verkstead"), "verkstead", "main")
        .await
        .unwrap()
        .expect("nothing is registered at that path yet");

    let id = start_unnamed_conversation(pool, repo.id, "amber-kestrel")
        .await
        .unwrap()
        .expect("the Repo was just registered");

    save_brief(pool, id, "# Rate limiting\n").await.unwrap();

    id
}

/// Whether the branch is still waiting to be named.
async fn naming(pool: &SqlitePool, id: i64) -> bool {
    load_conversation(pool, id)
        .await
        .unwrap()
        .expect("the Conversation is there")
        .naming
}

/// An Agent Profile to pair with, by name and with the models it lists.
async fn profile(pool: &SqlitePool, name: &str, models: &[&str]) -> i64 {
    create_profile(
        pool,
        &ProfileFacts {
            name: Some(name.to_owned()),
            account: Account::Claude {
                claude_dir: PathBuf::from(format!("/state/profiles/{name}")),
                config_file: PathBuf::from(format!("/state/profiles/{name}.json")),
            },
            models: models.iter().map(|model| (*model).to_owned()).collect(),
            memory: true,
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
            Event::Steer(target, ..) => Some(("steer", target)),
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

/// A Pairing picked on the form is recorded as the Conversation's own — both
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
                pairings: &[Settling {
                    role: Role::Implementation,
                    profile_id: profile,
                    model: "opus-4.8",
                }],
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

/// A Profile that went between the list the form read and the pick it made from
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
                pairings: &[Settling {
                    role: Role::Implementation,
                    profile_id: 404,
                    model: "opus-5",
                }],
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

/// A steer into Grilling opens a round, and what the human wrote on the form is
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
/// is forgotten.
///
/// A round that inherited the one before it would reach Wrapping with everything
/// wrap-up waits on already settled and would be over the moment it arrived. The
/// steers that open no round leave the round's bookkeeping where it is: a
/// wrap-up steered back into wrapping up is the *same* round, looked at again —
/// with the review the one thing it does put back, that being what looking again
/// means. See [`a_steer_into_wrapping_reads_the_branch_afresh`].
#[tokio::test]
async fn a_steer_into_grilling_forgets_the_round_before_it() {
    let (_dir, pool) = fresh_pool().await;
    let id = grilling(&pool).await;

    let repo = load_conversation(&pool, id).await.unwrap().unwrap().repo.id;

    settle_wrap_up(&pool, id, WaitingOn::Checks(repo))
        .await
        .unwrap();
    record_fix_attempt(&pool, id, repo, "Rust").await.unwrap();

    steer_conversation(&pool, id, into(Lifecycle::Wrapping))
        .await
        .unwrap();

    assert_eq!(
        wrap_up_settled(&pool, id).await.unwrap(),
        [WaitingOn::Checks(repo)],
        "a wrap-up steered into wrapping up is the same round, looked at again",
    );
    assert_eq!(fix_attempts(&pool, id, repo, "Rust").await.unwrap(), 1);

    steer_conversation(&pool, id, into(Lifecycle::Grilling))
        .await
        .unwrap();

    assert!(
        wrap_up_settled(&pool, id).await.unwrap().is_empty(),
        "and the round that starts here waits on all of it from nothing",
    );
    assert_eq!(fix_attempts(&pool, id, repo, "Rust").await.unwrap(), 0);
}

/// A steer into Wrapping puts the review back to waiting, whatever state it was
/// steered from and whether or not one had settled already.
///
/// Which is what *a steer reads the branch afresh* comes to on the record. A
/// review is one look at the branch and its settle says the look happened, so a
/// wrap-up that arrived carrying somebody else's would be one the watchers found
/// nothing left to do in — the Conversation the human just asked to be looked at
/// again reviewed by nobody. The resolve press is the one move into Wrapping
/// that leaves the settle standing, and is a press of its own for exactly this
/// reason — see `wrapping::resolving_a_conflict_sends_a_done_conversation_back`.
///
/// Nothing else the round settled is touched. The checks, what has been said and
/// the merge are asked of GitHub on every poll, so this round's own watchers
/// settle them from the answers they get; putting them back would be Verkstead
/// forgetting something it is about to be told again.
#[tokio::test]
async fn a_steer_into_wrapping_reads_the_branch_afresh() {
    let (_dir, pool) = fresh_pool().await;
    let id = grilling(&pool).await;

    let repo = load_conversation(&pool, id).await.unwrap().unwrap().repo.id;

    for waiting_on in [
        WaitingOn::Review,
        WaitingOn::Checks(repo),
        WaitingOn::Comments(repo),
        WaitingOn::Mergeable(repo),
    ] {
        settle_wrap_up(&pool, id, waiting_on).await.unwrap();
    }

    // Where the whole of that settling carried it, which is the state the steer
    // this is about starts from: work Verkstead has finished with.
    steer_conversation(&pool, id, into(Lifecycle::Done))
        .await
        .unwrap();

    steer_conversation(&pool, id, into(Lifecycle::Wrapping))
        .await
        .unwrap();

    let settled = wrap_up_settled(&pool, id).await.unwrap();

    assert!(
        !settled.contains(&WaitingOn::Review),
        "the review it was carried to Done on is not this wrap-up's: {settled:?}",
    );
    assert!(
        settled.contains(&WaitingOn::Checks(repo))
            && settled.contains(&WaitingOn::Comments(repo))
            && settled.contains(&WaitingOn::Mergeable(repo)),
        "and everything GitHub is asked about on every poll is left where it \
         was: {settled:?}",
    );

    // And again over a wrap-up that has already reviewed this round, which is
    // the halted one the human steers to get moving: the settle a moment old
    // goes the same way a settle from a finished round does.
    settle_wrap_up(&pool, id, WaitingOn::Review).await.unwrap();

    steer_conversation(&pool, id, into(Lifecycle::Wrapping))
        .await
        .unwrap();

    let settled = wrap_up_settled(&pool, id).await.unwrap();

    assert!(
        !settled.contains(&WaitingOn::Review),
        "a steer says look at this again whatever was looked at before it: \
         {settled:?}",
    );
}

/// What the steer had to make before anything could run in it: the Worktree it
/// checked out, and — for a Draft, which has never had a branch — the commit
/// that branch was cut from and the branch it was resolved through.
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
                base: Some(Base {
                    commit: "c0ffee",
                    named: Some("origin/main"),
                }),
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
    assert_eq!(
        conversation.base_ref.as_deref(),
        Some("origin/main"),
        "and the branch it resolved through is beside it, which is what the \
         commit sweep leaves the base's own commits out by",
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
            Event::Steer(_, instruction, _) => Some(instruction),
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

/// And a steer into Follow-up leaves its brief the same way, in a state that
/// reads back as the one it was written as.
///
/// The brief is what one session was set going on rather than what a round is
/// grilled about, so it rides on the human's own line exactly as an instruction
/// does and opens no round. And the state is the newest word the column holds,
/// which is worth a round trip of its own: a state written one way and read
/// another is a Conversation nothing could load.
#[tokio::test]
async fn a_steer_into_follow_up_keeps_its_brief_and_reads_back_as_follow_up() {
    let (_dir, pool) = fresh_pool().await;
    let id = grilling(&pool).await;

    let brief = "Does it count the `429`s it sends?\n";

    assert_eq!(
        steer_conversation(
            &pool,
            id,
            Steer {
                instruction: Some(brief),
                ..into(Lifecycle::FollowUp)
            },
        )
        .await
        .unwrap(),
        Steering::Steered,
    );

    assert_eq!(state(&pool, id).await, Lifecycle::FollowUp);
    assert_eq!(instructions(&pool, id).await, [Some(brief.to_owned())]);
    assert_eq!(
        ladder(&pool, id).await,
        [
            ("moved", Lifecycle::Grilling),
            ("steer", Lifecycle::FollowUp),
            ("moved", Lifecycle::FollowUp),
        ],
    );
    assert_eq!(
        briefs(&pool, id).await.len(),
        1,
        "and nothing opened a round: a follow-up is asked about the work rather \
         than grilled about a Brief",
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

/// A Draft steered into a state something runs in has its branch left to the
/// session the steer starts, exactly as a grill start would leave it.
///
/// It is the same moment for the same reason: nobody has named this branch, and
/// the session about to read the Brief is the first thing in the system that
/// knows what the work is about.
#[tokio::test]
async fn steering_a_draft_into_the_work_leaves_its_branch_to_be_named() {
    let (_dir, pool) = fresh_pool().await;
    let id = unnamed(&pool).await;

    assert_eq!(
        steer_conversation(&pool, id, into(Lifecycle::Implementing))
            .await
            .unwrap(),
        Steering::Steered,
    );

    assert!(naming(&pool, id).await);
}

/// A Draft steered into Done is not: nothing is started there, so nothing would
/// ever be along to answer the instruction.
#[tokio::test]
async fn steering_a_draft_into_done_settles_for_the_name_it_has() {
    let (_dir, pool) = fresh_pool().await;
    let id = unnamed(&pool).await;

    steer_conversation(&pool, id, into(Lifecycle::Done))
        .await
        .unwrap();

    assert!(!naming(&pool, id).await);
}

/// And a Conversation steered from anywhere else has had its first session
/// already, whatever its branch ended up being called.
#[tokio::test]
async fn steering_work_that_has_already_run_leaves_its_branch_alone() {
    let (_dir, pool) = fresh_pool().await;
    let id = unnamed(&pool).await;

    start_grilling(
        &pool,
        id,
        "c0ffee",
        Path::new("/state/worktrees/amber-kestrel"),
        &[],
    )
    .await
    .unwrap();

    // What the session that ran here left behind: it read the instruction and
    // settled for the name, which is one of the two ways the waiting ends.
    settle_naming(&pool, id).await.unwrap();

    steer_conversation(&pool, id, into(Lifecycle::Implementing))
        .await
        .unwrap();

    assert!(!naming(&pool, id).await);
}

/// The record beside the Steer Event: everything the form settled that its own
/// body cannot hold.
///
/// `None` is the ordinary answer for a steer written before any of it was kept,
/// and for a Timeline with no steer on it at all — which is why the helper
/// hands back the last one rather than asserting there is one.
async fn recorded(pool: &SqlitePool, id: i64) -> Option<SteerRecord> {
    timeline(pool, id)
        .await
        .unwrap()
        .into_iter()
        .filter_map(|event| match event.event {
            Event::Steer(_, _, recorded) => Some(recorded.map(|record| *record)),
            _ => None,
        })
        .next_back()
        .expect("there is a steer on the Timeline")
}

/// A repository to work alongside, registered and handed back by id.
async fn alongside(pool: &SqlitePool, name: &str) -> i64 {
    register_repo(pool, &PathBuf::from(format!("/srv/{name}")), name, "main")
        .await
        .unwrap()
        .expect("nothing is registered at that path yet")
        .id
}

/// The whole form, frozen into the record the submit writes: the ticks, the
/// Pairing the picker was on, and the companion rows the human asked for.
///
/// One transaction with the move, which is what the read back proves: the Steer
/// Event and the row beside it are read as one thing, so a record that moved
/// the work without saying what was picked to run it could not come back from
/// here at all.
#[tokio::test]
async fn a_steer_records_the_whole_form_beside_its_event() {
    let (_dir, pool) = fresh_pool().await;
    let id = drafting(&pool).await;
    let askance = alongside(&pool, "askance").await;
    let docs = alongside(&pool, "verkstead-docs").await;
    let profile = profile(&pool, "opus", &["opus-5", "opus-4.8"]).await;

    // One companion settled while it drafted, which is the set a steer may open
    // up — the other is one it puts in.
    assert_eq!(add_companion(&pool, id, docs).await.unwrap(), Adding::Added,);

    start_grilling(
        &pool,
        id,
        "c0ffee",
        Path::new("/state/worktrees/rate-limiting"),
        &[],
    )
    .await
    .unwrap();

    assert_eq!(
        steer_conversation(
            &pool,
            id,
            Steer {
                pairings: &[Settling {
                    role: Role::Implementation,
                    profile_id: profile,
                    model: "opus-4.8",
                }],
                companions: &[Joining {
                    repo_id: askance,
                    mode: CompanionMode::ReadWrite,
                    base_ref: Some("main"),
                    branch: "rate-limiting",
                }],
                opened: &[Opening {
                    repo_id: docs,
                    branch: "",
                }],
                recorded: Recorded {
                    digest: true,
                    interrupt: true,
                    pairing: Some(PickedPairing {
                        profile_id: profile,
                        model: "opus-4.8",
                    }),
                },
                ..into(Lifecycle::Implementing)
            },
        )
        .await
        .unwrap(),
        Steering::Steered,
    );

    let record = recorded(&pool, id).await.expect("the form was recorded");

    assert!(record.digest, "the tick the human left on");
    assert!(record.interrupt);

    let RecordedPairing::Under(paired) = record.pairing else {
        panic!("the Pairing the picker was on is on the record");
    };

    assert_eq!(paired.profile.id, profile);
    assert_eq!(
        paired.model.as_deref(),
        Some("opus-4.8"),
        "both halves of it, as the picker offers them",
    );

    assert_eq!(record.added.len(), 1);
    assert_eq!(record.added[0].repo, "askance");
    assert_eq!(record.added[0].mode, CompanionMode::ReadWrite);
    assert_eq!(record.added[0].base_ref.as_deref(), Some("main"));
    assert_eq!(record.added[0].branch, "rate-limiting");

    assert_eq!(record.upgraded.len(), 1);
    assert_eq!(record.upgraded[0].repo, "verkstead-docs");
    assert_eq!(
        record.upgraded[0].branch, "",
        "empty is mirroring, which is what the row was left on",
    );

    assert_eq!(
        record.source,
        Some(Lifecycle::Grilling),
        "and the state the press found it in, which the Event's target cannot say",
    );
}

/// The ordinary steer settles nothing and asks for nothing, and its record says
/// exactly that: no Pairing picked, and no companion row either way.
///
/// Which is not the same thing as a steer with no record — see below.
#[tokio::test]
async fn a_steer_that_picked_nothing_records_that_it_picked_nothing() {
    let (_dir, pool) = fresh_pool().await;
    let id = grilling(&pool).await;

    steer_conversation(&pool, id, into(Lifecycle::Done))
        .await
        .unwrap();

    let record = recorded(&pool, id).await.expect("the form was recorded");

    assert!(!record.digest);
    assert!(!record.interrupt);
    assert_eq!(record.pairing, RecordedPairing::Nothing);
    assert!(record.added.is_empty());
    assert!(record.upgraded.is_empty());
}

/// A Profile the human has finished with since reads back as one that is gone
/// rather than as a steer that picked nothing.
///
/// The pick was theirs and the account is removable — see `delete_profile`,
/// which is why the record names it with no foreign key behind it — so what is
/// left to say is that there was a pick and there is no account left to name.
#[tokio::test]
async fn a_steer_whose_profile_has_been_removed_still_says_one_was_picked() {
    let (_dir, pool) = fresh_pool().await;
    let id = grilling(&pool).await;
    let profile = profile(&pool, "opus", &["opus-5"]).await;

    steer_conversation(
        &pool,
        id,
        Steer {
            pairings: &[Settling {
                role: Role::Implementation,
                profile_id: profile,
                model: "opus-5",
            }],
            recorded: Recorded {
                digest: false,
                interrupt: false,
                pairing: Some(PickedPairing {
                    profile_id: profile,
                    model: "opus-5",
                }),
            },
            ..into(Lifecycle::Implementing)
        },
    )
    .await
    .unwrap();

    assert_eq!(
        delete_profile(&pool, profile).await.unwrap(),
        Deleting::Deleted,
        "an account is the human's to be finished with, whatever named it",
    );

    assert_eq!(
        recorded(&pool, id)
            .await
            .expect("the form was recorded")
            .pairing,
        RecordedPairing::Removed,
    );
}

/// The state each steer found the Conversation in, written down beside its own
/// Event.
///
/// One press has one source, and a Conversation steered a few times has one per
/// press: what the record holds is where each of them came *from*, which the
/// Event's own target says nothing about. Read back with the rest of the form,
/// and read again out of a second pool over the same file — the ending that
/// wants it may be hours and a restart away.
#[tokio::test]
async fn a_steer_records_the_state_it_came_out_of() {
    let (dir, pool) = fresh_pool().await;
    let id = grilling(&pool).await;

    // Out of Grilling and into Investigating: the question asked about work that
    // is still being interviewed, which is the source nothing else would say.
    steer_conversation(&pool, id, into(Lifecycle::Investigating))
        .await
        .unwrap();

    // And out of Investigating again, which is what makes this a fact per press
    // rather than one per Conversation.
    steer_conversation(&pool, id, into(Lifecycle::Implementing))
        .await
        .unwrap();

    assert_eq!(
        sources(&pool, id).await,
        [
            (Lifecycle::Investigating, Some(Lifecycle::Grilling)),
            (Lifecycle::Implementing, Some(Lifecycle::Investigating)),
        ],
        "each steer says where it went and where it came from",
    );

    pool.close().await;

    let reopened = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    assert_eq!(
        sources(&reopened, id).await,
        [
            (Lifecycle::Investigating, Some(Lifecycle::Grilling)),
            (Lifecycle::Implementing, Some(Lifecycle::Investigating)),
        ],
        "and a restart reads back what the press wrote down",
    );
}

/// And a steer from before the source was written down has none, which is a
/// steer recorded by an older Verkstead rather than a steer out of nowhere.
///
/// ADR-0006's rule again, said of the one thing this task adds. The row is taken
/// away by hand, which is the only way to have a Timeline that old in a database
/// this build made.
#[tokio::test]
async fn a_steer_from_before_the_source_was_written_down_reads_back_without_one() {
    let (_dir, pool) = fresh_pool().await;
    let id = grilling(&pool).await;

    steer_conversation(&pool, id, into(Lifecycle::Investigating))
        .await
        .unwrap();

    sqlx::query("DELETE FROM steer_sources")
        .execute(&pool)
        .await
        .unwrap();

    assert_eq!(
        sources(&pool, id).await,
        [(Lifecycle::Investigating, None)],
        "the rest of the record is exactly where it was",
    );
}

/// Where each steer on this Timeline went, and where the record says it came
/// from.
///
/// The pair rather than the source alone, because the source is a fact about one
/// press: a Conversation steered three times has three of them, and a reading
/// that dropped the target could not say which press each belonged to.
async fn sources(pool: &SqlitePool, id: i64) -> Vec<(Lifecycle, Option<Lifecycle>)> {
    timeline(pool, id)
        .await
        .unwrap()
        .into_iter()
        .filter_map(|event| match event.event {
            Event::Steer(target, _, recorded) => {
                Some((target, recorded.and_then(|record| record.source)))
            }
            _ => None,
        })
        .collect()
}

/// And a steer written before any of this was kept reads back as the steer it
/// was: the target and the body, and no record beside them.
///
/// ADR-0006's rule — the record is kept and read as it was written — said of
/// the one thing this task adds. The row is taken away by hand, which is the
/// only way to have a Timeline that old in a database this build made.
#[tokio::test]
async fn a_steer_from_before_the_record_was_kept_reads_back_without_one() {
    let (_dir, pool) = fresh_pool().await;
    let id = grilling(&pool).await;

    steer_conversation(&pool, id, into(Lifecycle::Done))
        .await
        .unwrap();

    sqlx::query("DELETE FROM steers WHERE conversation_id = ?")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();

    assert_eq!(recorded(&pool, id).await, None);

    assert_eq!(
        ladder(&pool, id).await,
        [
            ("moved", Lifecycle::Grilling),
            ("steer", Lifecycle::Done),
            ("moved", Lifecycle::Done),
        ],
        "and the Event itself is untouched: what it always said, it says",
    );
}
