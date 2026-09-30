//! The session a launch on this device **carries on from**: the conversation a
//! transferred agent was having when the work moved (ADR-0020, *Transfer*).
//!
//! What is worth a test here is the part a reader could not check for itself.
//! The row is written once, by the arrival, and read once, by the launch — and
//! the reading **spends** it, which is the whole of what makes the session after
//! an arrival the one that carries a conversation on and the session after *that*
//! an ordinary session of the work. A second reader that got the same answer
//! would be a task session primed with a note about a move it had already been
//! told about.
//!
//! And what it is written *from*: the newest session on the record, with the
//! harness that ran it. The newest is what the agent was having, so a lookup that
//! answered with an older one would resume a conversation somebody had already
//! left — and the harness is what says a Conversation whose Profile was changed
//! under it has a log no other backend could read.
//!
//! What the launch then does with the answer is the server's — see its
//! `tests/transfer.rs`, where a grilling really carries on across two machines.

use std::path::{Path, PathBuf};
use std::time::Duration;

use sqlx::SqlitePool;
use verkstead_store::{
    Account, AgentType, Continued, Pairing, ProfileFacts, carrying_a_conversation,
    continue_on_arrival, create_profile, end_session, open_database, register_repo, start_capture,
    start_conversation, take_up_the_conversation, the_last_session,
};

/// The device every Conversation started here is ranked by, named the way a
/// cluster names one.
const THIS_DEVICE: &str = "aa00bb11cc22dd33ee44ff5566778899";

/// A pool over a fresh database, plus the directory keeping it alive.
async fn fresh_pool() -> (tempfile::TempDir, SqlitePool) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();
    (dir, pool)
}

/// A Conversation to hang sessions off.
async fn conversation(pool: &SqlitePool) -> i64 {
    let repo = register_repo(pool, Path::new("/watched/verkstead"), "verkstead", "main")
        .await
        .unwrap()
        .expect("nothing was registered at that path yet")
        .id;

    start_conversation(pool, repo, "rate-limiting", THIS_DEVICE)
        .await
        .unwrap()
        .expect("the Repo was just registered")
}

/// A session of that Conversation, named and paired the way a launch names and
/// pairs one — which is what the lookup below reads.
async fn ran(pool: &SqlitePool, id: i64, session: &str, agent_type: AgentType) {
    let profile = create_profile(
        pool,
        &ProfileFacts {
            name: Some(session.to_owned()),
            account: match agent_type {
                AgentType::Claude => Account::Claude {
                    claude_dir: PathBuf::from("/watched/accounts/work/.claude"),
                    config_file: PathBuf::from("/watched/accounts/work/.claude.json"),
                },
                _ => Account::Codex {
                    home: PathBuf::from("/watched/accounts/work/.codex"),
                },
            },
            models: vec!["claude-opus-5".to_owned()],
            memory: true,
        },
    )
    .await
    .unwrap()
    .expect("nothing is called that yet");

    let pairing = Pairing {
        profile,
        model: Some("claude-opus-5".to_owned()),
    };

    let event = start_capture(pool, id, Some(session), Some(&pairing))
        .await
        .unwrap();

    end_session(pool, event, Some(0), Duration::from_millis(400), true)
        .await
        .unwrap();
}

/// The **newest** session is the one to carry on from, with the harness that ran
/// it — which is the conversation the agent was having when the work moved.
#[tokio::test]
async fn the_last_session_is_the_one_to_carry_on_from() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool).await;

    ran(
        &pool,
        id,
        "11111111-1111-4111-8111-111111111111",
        AgentType::Claude,
    )
    .await;
    ran(
        &pool,
        id,
        "22222222-2222-4222-8222-222222222222",
        AgentType::Claude,
    )
    .await;

    assert_eq!(
        the_last_session(&pool, id).await.unwrap(),
        Some(Continued {
            session_id: "22222222-2222-4222-8222-222222222222".to_owned(),
            agent_type: AgentType::Claude,
        }),
        "the newest of them, rather than the first: an older one is a conversation \
         somebody had already left",
    );
}

/// And the harness it names is the harness that really ran it, whatever the
/// Conversation is paired with now.
#[tokio::test]
async fn the_harness_is_the_one_that_ran_that_session() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool).await;

    ran(
        &pool,
        id,
        "33333333-3333-4333-8333-333333333333",
        AgentType::Codex,
    )
    .await;

    assert_eq!(
        the_last_session(&pool, id)
            .await
            .unwrap()
            .map(|continued| continued.agent_type),
        Some(AgentType::Codex),
    );
}

/// A Conversation whose work never started has nothing to carry on from.
#[tokio::test]
async fn a_conversation_with_no_session_has_nothing_to_carry_on_from() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool).await;

    assert_eq!(the_last_session(&pool, id).await.unwrap(), None);
}

/// **Taking the conversation up spends it**, so the session after the one that
/// carried it on is an ordinary session of the work.
#[tokio::test]
async fn taking_the_conversation_up_spends_it() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool).await;

    let continued = Continued {
        session_id: "44444444-4444-4444-8444-444444444444".to_owned(),
        agent_type: AgentType::Claude,
    };

    continue_on_arrival(&pool, id, &continued).await.unwrap();

    assert_eq!(
        take_up_the_conversation(&pool, id).await.unwrap(),
        Some(continued),
        "the first launch after the arrival takes it up",
    );

    assert_eq!(
        take_up_the_conversation(&pool, id).await.unwrap(),
        None,
        "and the launch after that has nothing to take up: what it would have \
         been primed with is a note about a move the session before it was \
         already told about",
    );
}

/// **And asking whether there is one to take up does not spend it**, which is
/// what lets the question be asked in front of the launch as well as by it.
///
/// A relaunch holds the Question Sets it would otherwise lock where a conversation
/// is standing to be carried on, and that reading comes before the launch that
/// spends the row — so a peek that spent it would be the relaunch resuming nothing
/// and locking nothing.
#[tokio::test]
async fn asking_whether_there_is_a_conversation_to_carry_on_does_not_spend_it() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool).await;

    assert!(
        !carrying_a_conversation(&pool, id).await.unwrap(),
        "nothing has arrived on it, which is every ordinary launch",
    );

    let continued = Continued {
        session_id: "77777777-7777-4777-8777-777777777777".to_owned(),
        agent_type: AgentType::Claude,
    };

    continue_on_arrival(&pool, id, &continued).await.unwrap();

    assert!(
        carrying_a_conversation(&pool, id).await.unwrap(),
        "and there is one now",
    );
    assert!(
        carrying_a_conversation(&pool, id).await.unwrap(),
        "asked twice over, the looking having taken nothing",
    );

    assert_eq!(
        take_up_the_conversation(&pool, id).await.unwrap(),
        Some(continued),
        "and the launch after the peeking is still the one that takes it up",
    );
}

/// And a Conversation nothing has arrived on has nothing to take up, which is
/// every ordinary launch.
#[tokio::test]
async fn an_ordinary_launch_has_nothing_to_take_up() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool).await;

    assert_eq!(take_up_the_conversation(&pool, id).await.unwrap(), None);
}

/// **A Conversation that comes home again carries on from the newest session it
/// had**, rather than from the one an earlier arrival wrote down.
///
/// Work moved four times has been away twice, and the conversation to pick up the
/// second time is the one the other machine was having then — two moves ago is a
/// session nothing is part way through.
#[tokio::test]
async fn coming_back_again_carries_on_from_the_newer_session() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool).await;

    for session in [
        "55555555-5555-4555-8555-555555555555",
        "66666666-6666-4666-8666-666666666666",
    ] {
        continue_on_arrival(
            &pool,
            id,
            &Continued {
                session_id: session.to_owned(),
                agent_type: AgentType::Claude,
            },
        )
        .await
        .unwrap();
    }

    assert_eq!(
        take_up_the_conversation(&pool, id)
            .await
            .unwrap()
            .map(|continued| continued.session_id),
        Some("66666666-6666-4666-8666-666666666666".to_owned()),
    );
}
