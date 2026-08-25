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
//! Nothing else here is about what runs afterwards. The state, the two Events
//! and the Pairing are the whole of what the store has to say about a steer;
//! recreating a Worktree, clearing a stop and launching are the server's, and
//! are asked of it there.

use std::path::{Path, PathBuf};

use sqlx::SqlitePool;
use verkstead_store::{
    AgentType, Event, Lifecycle, ProfileFacts, Role, Settling, Steering, create_profile,
    load_conversation, open_database, register_repo, save_brief, start_conversation,
    start_grilling, steer_conversation, timeline,
};

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

/// Its Timeline as the kinds that say where the work went: the states it was
/// steered into and the states it moved to, in the order they landed.
async fn ladder(pool: &SqlitePool, id: i64) -> Vec<(&'static str, Lifecycle)> {
    timeline(pool, id)
        .await
        .unwrap()
        .into_iter()
        .filter_map(|event| match event.event {
            Event::Steer(target) => Some(("steer", target)),
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
        steer_conversation(&pool, id, Lifecycle::Done, None)
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
        steer_conversation(&pool, draft, Lifecycle::Done, None)
            .await
            .unwrap(),
        Steering::Steered,
    );
    assert_eq!(state(&pool, draft).await, Lifecycle::Done);

    assert_eq!(
        steer_conversation(&pool, draft, Lifecycle::Done, None)
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
            Lifecycle::Wrapping,
            Some(Settling {
                role: Role::Implementation,
                profile_id: profile,
                model: "opus-4.8",
            }),
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
            Lifecycle::Wrapping,
            Some(Settling {
                role: Role::Implementation,
                profile_id: 404,
                model: "opus-5",
            }),
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

#[tokio::test]
async fn there_is_no_conversation_to_steer() {
    let (_dir, pool) = fresh_pool().await;

    assert_eq!(
        steer_conversation(&pool, 404, Lifecycle::Done, None)
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
        steer_conversation(&pool, id, Lifecycle::Done, None)
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
