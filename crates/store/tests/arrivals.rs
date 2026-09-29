//! A Conversation **arriving from another device**: the row this database
//! writes for work that was being done somewhere else, and the taking back of
//! one whose move never finished (ADR-0020, *Transfer*).
//!
//! What is worth a test here is what the row has to carry that no create
//! invents. An id is this database's own and a Repo is this machine's, but the
//! **Rank** and the **birth key** came with the work: the rank carries the
//! device that issued it and is what keeps the copy in its place in the merged
//! order, and the key is what says two copies in two databases are one piece of
//! work. A create that quietly stamped either with this device's own would draw
//! the work twice and lose its place at once — and neither could be noticed by
//! reading the row back on the machine that wrote it.
//!
//! And the Pairings, which arrive as three ids of this database's own: what a
//! role runs under is a Profile row here, a mirror being one like any other, and
//! a role picked away is a choice rather than an empty picker.
//!
//! The sweep is the other half of a failure being safe. A copy that reached here
//! and whose move then fell over was never the human's to look at, so the device
//! that sent it takes it back — which the archive does not authorise and does
//! not need to. What the server makes of both is its `tests/transfer.rs`, where
//! two devices really move work between them; what is here is the database.

use std::path::Path;

use sqlx::SqlitePool;
use verkstead_store::{
    Account, Arrival, ArrivingPicked, Birth, Deletion, Lifecycle, Picked, ProfileFacts, arrive,
    birth, conversation_rank, conversations, create_profile, delete_conversation,
    load_conversation, open_database, register_repo, start_conversation, sweep_arrival,
};

/// The device this database belongs to.
const THIS_DEVICE: &str = "0011223344556677889900aabbccddee";

/// And the one the work was drafted and worked on, which is where every arriving
/// Conversation here comes from.
const THE_OTHER_DEVICE: &str = "aa00bb11cc22dd33ee44ff5566778899";

/// The Rank the work sits at, as the device that issued it wrote it: a key, a
/// separator and that machine's own id.
const RANK: &str = "a0-aa00bb11cc22dd33ee44ff5566778899";

/// A pool over a fresh database, plus the directory keeping it alive.
async fn fresh_pool() -> (tempfile::TempDir, SqlitePool) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();
    (dir, pool)
}

/// A Repo for the arriving work to land in — which on a real move is the one the
/// sending device's matching settled.
async fn repo(pool: &SqlitePool) -> i64 {
    register_repo(pool, Path::new("/here/verkstead"), "verkstead", "main")
        .await
        .unwrap()
        .expect("nothing was registered at that path yet")
        .id
}

/// An Agent Profile of this device's own for a Pairing to name.
async fn profile(pool: &SqlitePool, name: &str) -> i64 {
    create_profile(
        pool,
        &ProfileFacts {
            name: Some(name.to_owned()),
            account: Account::Codex {
                home: Path::new("/here/accounts").join(name),
            },
            models: vec!["gpt-5".to_owned()],
            memory: true,
        },
    )
    .await
    .unwrap()
    .expect("no other Profile is called that")
    .id
}

/// The Conversation as a transfer hands it over: grilling, on a branch somebody
/// settled on, at the rank it already sat at and under the key it was born.
fn arriving(repo_id: i64, profile_id: i64) -> Arrival {
    Arrival {
        born: Birth {
            device: THE_OTHER_DEVICE.to_owned(),
            id: 7,
        },
        repo_id,
        branch: "rate-limiting".to_owned(),
        branch_named: true,
        naming: false,
        state: Lifecycle::Grilling,
        rank: RANK.to_owned(),
        grilling: ArrivingPicked::Under {
            profile_id,
            model: Some("gpt-5".to_owned()),
        },
        implementation: ArrivingPicked::Under {
            profile_id,
            model: Some("gpt-5".to_owned()),
        },
        review: ArrivingPicked::Skipped,
    }
}

#[tokio::test]
async fn a_conversation_arrives_under_this_devices_own_id_with_everything_else_as_it_was() {
    let (_dir, pool) = fresh_pool().await;
    let repo = repo(&pool).await;
    let profile = profile(&pool, "work").await;

    let landed = arrive(&pool, &arriving(repo, profile))
        .await
        .unwrap()
        .expect("the Repo is registered");

    let held = load_conversation(&pool, landed)
        .await
        .unwrap()
        .expect("the Conversation that just landed");

    assert_eq!(held.state, Lifecycle::Grilling, "the state it was in");
    assert_eq!(held.branch, "rate-limiting", "the branch the work is on");
    assert!(held.branch_named, "which is a name somebody settled on");
    assert_eq!(held.repo.id, repo, "in the Repo the matching settled");

    assert_eq!(
        held.grilling_pairing.as_ref().map(|under| under.profile.id),
        Some(profile),
        "and the grilling runs under a Profile of this device's own",
    );
    assert_eq!(
        held.implementation_pairing
            .as_ref()
            .map(|under| under.profile.id),
        Some(profile),
        "and so does the implementation",
    );
    assert_eq!(
        held.review_pairing,
        Picked::Skipped,
        "and the role that was picked away arrives picked away, which is a \
         choice rather than an empty picker",
    );
}

#[tokio::test]
async fn the_rank_and_the_birth_key_arrive_as_they_were_rather_than_being_invented_here() {
    let (_dir, pool) = fresh_pool().await;
    let repo = repo(&pool).await;
    let profile = profile(&pool, "work").await;

    // One of this device's own first, so the arriving row is not the only thing
    // in the list and its rank is not the only rank there is.
    start_conversation(&pool, repo, "something-else", THIS_DEVICE)
        .await
        .unwrap()
        .expect("the Repo is registered");

    let landed = arrive(&pool, &arriving(repo, profile))
        .await
        .unwrap()
        .expect("the Repo is registered");

    assert_eq!(
        conversation_rank(&pool, landed).await.unwrap(),
        Some(RANK.to_owned()),
        "the Rank crossed verbatim: it carries the device that issued it, so \
         the copy sits where the work sat",
    );

    assert_eq!(
        birth(&pool, landed).await.unwrap(),
        Some(Birth {
            device: THE_OTHER_DEVICE.to_owned(),
            id: 7,
        }),
        "and the key it was born under, which is not this device's to invent — \
         a copy stamped here would be a second piece of work",
    );

    let row = conversations(&pool, false)
        .await
        .unwrap()
        .into_iter()
        .find(|row| row.id == landed)
        .expect("the arriving Conversation is on the list");

    assert_eq!(
        row.born,
        format!("{THE_OTHER_DEVICE}/7"),
        "and the sidebar carries the pair as the one string the merge reads",
    );
    assert!(
        !row.transferred,
        "and this copy is the record: nothing has been handed on from here",
    );
}

#[tokio::test]
async fn nothing_arrives_against_a_repo_this_device_no_longer_holds() {
    let (_dir, pool) = fresh_pool().await;
    let profile = profile(&pool, "work").await;

    assert_eq!(
        arrive(&pool, &arriving(404, profile)).await.unwrap(),
        None,
        "the id was matched against this registry a moment ago, and a Repo that \
         has gone since is no repository to put work in",
    );

    assert!(
        conversations(&pool, true).await.unwrap().is_empty(),
        "and nothing half-written is left behind",
    );
}

#[tokio::test]
async fn a_copy_whose_move_never_finished_is_swept_whatever_state_it_is_in() {
    let (_dir, pool) = fresh_pool().await;
    let repo = repo(&pool).await;
    let profile = profile(&pool, "work").await;

    let landed = arrive(&pool, &arriving(repo, profile))
        .await
        .unwrap()
        .expect("the Repo is registered");

    // The ordinary delete refuses it, and rightly: what authorises forgetting
    // the human's own work is their archiving of it, and nothing here has been
    // archived.
    assert_eq!(
        delete_conversation(&pool, landed).await.unwrap(),
        Deletion::NotArchived,
    );

    // The sweep does not need it. The copy was never on anybody's sidebar, and
    // the device asking for it back is the one that wrote it a moment ago.
    assert_eq!(
        sweep_arrival(&pool, landed).await.unwrap(),
        Deletion::Deleted,
    );

    assert_eq!(
        load_conversation(&pool, landed).await.unwrap(),
        None,
        "and what the move left here is gone",
    );
    assert_eq!(
        birth(&pool, landed).await.unwrap(),
        None,
        "the key it arrived under with it",
    );
}

#[tokio::test]
async fn sweeping_something_that_is_not_there_is_not_a_failure() {
    let (_dir, pool) = fresh_pool().await;

    assert_eq!(
        sweep_arrival(&pool, 404).await.unwrap(),
        Deletion::NoSuchConversation,
        "what the sender is asking for is that it not be here, and it is not",
    );
}
