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
//! And a copy **coming home**, which is the ordinary shape of the second move:
//! the birth key says the arriving work and the copy this device kept are one
//! thing, so the copy is written over under the id it already has rather than a
//! second Conversation being made beside it. What the row keeps through it is
//! this device's own — the Worktree it cut, and the mark saying where the live
//! record is until the move is over.
//!
//! And the **checkout**, which lands in a leg of its own after the row and is
//! the other place the two devices' worlds have to be kept apart: the Worktree
//! is a path of *this* machine's and is in nothing that crossed, while the base
//! the branch was cut from is a fact about the work and came with it.
//!
//! The sweep is the other half of a failure being safe. A copy that reached here
//! and whose move then fell over was never the human's to look at, so the device
//! that sent it takes it back — which the archive does not authorise and does
//! not need to. What the server makes of both is its `tests/transfer.rs`, where
//! two devices really move work between them; what is here is the database.

use std::path::Path;

use sqlx::SqlitePool;
use verkstead_store::{
    Account, Arrival, ArrivingPicked, Birth, CompanionWorktree, Deletion, Lifecycle, Picked,
    ProfileFacts, Replacing, Transferred, arrive, birth, born_as, conversation_rank, conversations,
    create_profile, delete_conversation, load_conversation, open_database, register_repo, replace,
    start_conversation, sweep_arrival, transfer_away, transferred,
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

/// **The checkout is written down after the row**, which is the order the two
/// legs of a move go in: the row is what the sending device numbers, and the
/// branch is what it numbers *against*.
///
/// The path is this device's own and is never in what arrived — a Worktree is a
/// directory on one machine, so it is in no slice and on no wire. The base is
/// the other way round: what the branch was cut from is a fact about the work,
/// and a copy that landed without it would report the history under the branch
/// as this Conversation's own.
#[tokio::test]
async fn the_checkout_lands_against_the_row_that_arrived_before_it() {
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

    assert_eq!(
        held.worktree, None,
        "the row arrives with nowhere to work, the checkout being a leg of its own",
    );
    assert_eq!(held.base_commit, None, "and with no base under it yet");

    let here = Path::new("/here/data/worktrees/verkstead-rate-limiting");

    verkstead_store::arrived_checkout(&pool, landed, here, Some(BASE), Some("main"), &[])
        .await
        .unwrap();

    let held = load_conversation(&pool, landed)
        .await
        .unwrap()
        .expect("the Conversation the checkout landed against");

    assert_eq!(
        held.worktree.as_deref(),
        Some(here),
        "the work is checked out where this device put it",
    );
    assert_eq!(
        held.base_commit.as_deref(),
        Some(BASE),
        "and the branch says what it was cut from",
    );
    assert_eq!(held.base_ref.as_deref(), Some("main"), "and off what");
}

/// **And the Companions' checkouts land with it**, on the side table a start
/// writes them to and in the same transaction: a Conversation that said where its
/// own work was checked out and not where the repositories beside it went would
/// be one nothing could bind into a sandbox.
///
/// Each carries what it was cut from as well as where it went, which for a
/// read-only Companion is the commit it is detached at — a Companion's base is a
/// *name* on its row and a name moves, so this is the only thing that ever
/// records which commit that name came to.
#[tokio::test]
async fn the_companions_checkouts_land_with_the_conversations_own() {
    let (_dir, pool) = fresh_pool().await;
    let repo = repo(&pool).await;
    let profile = profile(&pool, "work").await;

    let beside = register_repo(&pool, Path::new("/here/askance"), "askance", "main")
        .await
        .unwrap()
        .expect("nothing was registered at that path yet")
        .id;

    let landed = arrive(&pool, &arriving(repo, profile))
        .await
        .unwrap()
        .expect("the Repo is registered");

    let alongside = Path::new("/here/data/worktrees/askance-rate-limiting");

    verkstead_store::arrived_checkout(
        &pool,
        landed,
        Path::new("/here/data/worktrees/verkstead-rate-limiting"),
        Some(BASE),
        Some("main"),
        &[CompanionWorktree {
            repo_id: beside,
            path: alongside.to_owned(),
            base_commit: Some(BASE.to_owned()),
        }],
    )
    .await
    .unwrap();

    let written: Vec<(i64, String, Option<String>)> = sqlx::query_as(
        "SELECT repo_id, path, base_commit FROM companion_worktrees WHERE conversation_id = ?",
    )
    .bind(landed)
    .fetch_all(&pool)
    .await
    .unwrap();

    assert_eq!(
        written,
        vec![(
            beside,
            alongside.display().to_string(),
            Some(BASE.to_owned()),
        )],
        "the companion is checked out where this device put it, off what it came from",
    );
}

/// **A Conversation coming home lands on the row it left**, under the id that row
/// already has rather than as a second Conversation beside it.
///
/// Which is what the **birth key** is for from this side: the work was drafted
/// here, handed on, and is coming back, and the key is the one thing every copy
/// of it says the same. So the lookup finds the copy this device kept and the
/// arriving Conversation is written over it — every link anybody saved naming an
/// id that is still the work's.
///
/// What is written over is everything the row says about the work. What is
/// **not** is the Worktree, which is this device's own directory and the one the
/// branch is going back into; and the mark saying where the live record is, which
/// stays on until the sending device says the move is over — see
/// `births::live_here`.
#[tokio::test]
async fn a_conversation_coming_home_lands_on_the_row_it_left() {
    let (_dir, pool) = fresh_pool().await;
    let repo = repo(&pool).await;
    let profile = profile(&pool, "work").await;

    // Drafted here and handed on: the row this device kept, the directory it cut
    // for the work, and the mark saying the record is on the other machine.
    let ours = start_conversation(&pool, repo, "rate-limiting", THIS_DEVICE)
        .await
        .unwrap()
        .expect("the Repo is registered");

    let cut = Path::new("/here/data/worktrees/verkstead-rate-limiting");

    verkstead_store::arrived_checkout(&pool, ours, cut, Some(BASE), Some("main"), &[])
        .await
        .unwrap();

    transfer_away(
        &pool,
        ours,
        &Transferred {
            device: THE_OTHER_DEVICE.to_owned(),
            id: 12,
        },
    )
    .await
    .unwrap();

    // And coming back, under the key it was born under — which is this device's
    // own id for it, the work having been drafted here.
    let born = Birth {
        device: THIS_DEVICE.to_owned(),
        id: ours,
    };

    let coming_home = Arrival {
        born: born.clone(),
        state: Lifecycle::Implementing,
        branch: "rate-limiting-again".to_owned(),
        ..arriving(repo, profile)
    };

    assert_eq!(
        born_as(&pool, &born).await.unwrap(),
        Some(ours),
        "the key says the arriving work and the copy held here are one thing",
    );

    assert_eq!(
        replace(&pool, ours, &coming_home).await.unwrap(),
        Replacing::Replaced,
    );

    let held = load_conversation(&pool, ours)
        .await
        .unwrap()
        .expect("the copy that was here is the Conversation that came back");

    assert_eq!(
        conversations(&pool, true).await.unwrap().len(),
        1,
        "one Conversation, which is the one that was already here",
    );

    assert_eq!(
        held.state,
        Lifecycle::Implementing,
        "in the state the work is in now rather than the one it left in",
    );
    assert_eq!(
        held.branch, "rate-limiting-again",
        "on the branch it is on now",
    );
    assert_eq!(
        held.grilling_pairing.as_ref().map(|under| under.profile.id),
        Some(profile),
        "under the Pairings that came with it",
    );
    assert_eq!(
        held.review_pairing,
        Picked::Skipped,
        "the role picked away among them",
    );

    assert_eq!(
        conversation_rank(&pool, ours).await.unwrap(),
        Some(RANK.to_owned()),
        "at the Rank the work sits at, which is the cluster's rather than \
         either machine's",
    );
    assert_eq!(
        birth(&pool, ours).await.unwrap(),
        Some(born),
        "still answering to the key it was born under",
    );

    assert_eq!(
        held.worktree.as_deref(),
        Some(cut),
        "and the directory this device cut for the work the first time is still \
         where the work goes",
    );
    assert_eq!(
        transferred(&pool, ours).await.unwrap().map(|live| live.id),
        Some(12),
        "and the mark stays on until the sending device says the move is over: \
         a return that fell over here is the tombstone it started as",
    );
}

/// And a return refuses what an arrival refuses: a Repo unregistered since the
/// match settled it is no repository to put work in.
#[tokio::test]
async fn a_conversation_coming_home_to_a_repo_that_has_gone_is_refused() {
    let (_dir, pool) = fresh_pool().await;
    let repo = repo(&pool).await;
    let profile = profile(&pool, "work").await;

    let ours = start_conversation(&pool, repo, "rate-limiting", THIS_DEVICE)
        .await
        .unwrap()
        .expect("the Repo is registered");

    let coming_home = Arrival {
        born: Birth {
            device: THIS_DEVICE.to_owned(),
            id: ours,
        },
        ..arriving(404, profile)
    };

    assert_eq!(
        replace(&pool, ours, &coming_home).await.unwrap(),
        Replacing::NoSuchRepo,
    );

    let held = load_conversation(&pool, ours)
        .await
        .unwrap()
        .expect("the copy that was here is still here");

    assert_eq!(
        held.repo.id, repo,
        "and nothing of the row was written: a move that is refused is a move \
         that did not happen",
    );
    assert_eq!(held.branch, "rate-limiting");
}

/// **Nothing that never left is found by a key of somebody else's.** Two devices
/// number their own rows and collide by construction, so the lookup is by the
/// pair rather than by the id — a Conversation of this device's own that happened
/// to be numbered 7 is not the one another machine drafted as 7.
#[tokio::test]
async fn a_key_of_another_devices_finds_nothing_here() {
    let (_dir, pool) = fresh_pool().await;
    let repo = repo(&pool).await;

    let ours = start_conversation(&pool, repo, "rate-limiting", THIS_DEVICE)
        .await
        .unwrap()
        .expect("the Repo is registered");

    assert_eq!(
        born_as(
            &pool,
            &Birth {
                device: THE_OTHER_DEVICE.to_owned(),
                id: ours,
            },
        )
        .await
        .unwrap(),
        None,
        "same id, another device: two pieces of work",
    );

    assert_eq!(
        born_as(
            &pool,
            &Birth {
                device: THIS_DEVICE.to_owned(),
                id: ours,
            },
        )
        .await
        .unwrap(),
        Some(ours),
        "and this device's own id for it is the key it was born under",
    );
}

/// What the branch was cut from on the machine the work came off, which is the
/// same commit in every copy of a repository.
const BASE: &str = "1c1ca65983a10208a9fec4326867f434ffeefdb5";
