//! The **birth key** every Conversation carries — the device it was drafted on
//! and the id it was given there — and the mark saying a copy of one has been
//! transferred away (ADR-0020, *Transfer*).
//!
//! What is worth a test here is what a promise could not keep. A Conversation
//! started on this device is born here, under the id this database just issued,
//! which is exactly what a Conversation that has never moved has to say; and a
//! database written before there were keys at all opens with every row of it
//! stamped the same way, at the first start that can say which device this is.
//! Neither is something a reader could check for itself: the key is written once
//! and read for ever after.
//!
//! And the key a copy arrives with is the key it keeps, which is the whole point
//! of the pair being cluster-wide: a Conversation that has moved twice still
//! answers to what it was born under, so nothing here may quietly re-stamp a row
//! with the machine it happens to be sitting on.
//!
//! The mark beside it is the other half: where the live record went, kept on the
//! copy that is no longer it. What the sidebar does with the pair is the server's
//! — see its `tests/merging.rs`, where two devices really hold two copies of one
//! key — and what a URL does with it is the viewer's. What is here is the
//! database.

use std::path::Path;

use sqlx::SqlitePool;
use verkstead_store::{
    Birth, Transferred, birth, conversations, open_database, record_birth, register_repo,
    stamp_the_births, start_conversation, transfer_away, transferred,
};

/// The device every Conversation started here is drafted on, named the way a
/// cluster names one.
const THIS_DEVICE: &str = "aa00bb11cc22dd33ee44ff5566778899";

/// And the other machine in the cluster: where a copy comes from, and where one
/// is transferred to.
const ANOTHER_DEVICE: &str = "0011223344556677889900aabbccddee";

/// A pool over a fresh database, plus the directory keeping it alive.
async fn fresh_pool() -> (tempfile::TempDir, SqlitePool) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();
    (dir, pool)
}

/// A Repo to hang Conversations off.
async fn repo(pool: &SqlitePool) -> i64 {
    register_repo(pool, Path::new("/watched/verkstead"), "verkstead", "main")
        .await
        .unwrap()
        .expect("nothing was registered at that path yet")
        .id
}

/// A Conversation on that Repo, started by this device.
async fn start(pool: &SqlitePool, repo: i64, branch: &str) -> i64 {
    start_conversation(pool, repo, branch, THIS_DEVICE)
        .await
        .unwrap()
        .expect("the Repo was just registered")
}

/// One sidebar row's birth key and its mark, by the Conversation it is about:
/// what the list carries out for the merge to read.
async fn row(pool: &SqlitePool, conversation: i64) -> (String, bool) {
    let rows = conversations(pool, false).await.unwrap();
    let row = rows
        .into_iter()
        .find(|row| row.id == conversation)
        .expect("the Conversation is on the list");

    (row.born, row.transferred)
}

/// A database written before there were birth keys, which is this one with the
/// table empty: the schema arrives as a `CREATE TABLE IF NOT EXISTS`, so what an
/// older database opens with is every row unstamped rather than a table missing.
async fn as_written_before_keys(pool: &SqlitePool) {
    sqlx::query("DELETE FROM births")
        .execute(pool)
        .await
        .unwrap();
}

#[tokio::test]
async fn a_conversation_started_here_is_born_here_under_the_id_it_was_given() {
    let (_dir, pool) = fresh_pool().await;
    let repo = repo(&pool).await;

    let conversation = start(&pool, repo, "task-runner").await;

    assert_eq!(
        birth(&pool, conversation).await.unwrap(),
        Some(Birth {
            device: THIS_DEVICE.to_owned(),
            id: conversation,
        }),
        "a Conversation drafted here has never moved, and its key says so",
    );

    assert_eq!(
        row(&pool, conversation).await,
        (format!("{THIS_DEVICE}/{conversation}"), false),
        "and the sidebar row carries the pair as the one string the merge reads",
    );
}

#[tokio::test]
async fn every_conversation_written_before_this_is_stamped_at_the_first_start_that_can_say_which_device_it_is()
 {
    let (dir, pool) = fresh_pool().await;
    let database = dir.path().join("verkstead.db");
    let repo = repo(&pool).await;

    let first = start(&pool, repo, "task-runner").await;
    let second = start(&pool, repo, "usage-limits").await;
    as_written_before_keys(&pool).await;

    pool.close().await;
    let pool = open_database(&database).await.unwrap();

    assert_eq!(
        birth(&pool, first).await.unwrap(),
        None,
        "the table arrives empty: the open cannot know which device this is",
    );

    stamp_the_births(&pool, THIS_DEVICE).await.unwrap();

    for conversation in [first, second] {
        assert_eq!(
            birth(&pool, conversation).await.unwrap(),
            Some(Birth {
                device: THIS_DEVICE.to_owned(),
                id: conversation,
            }),
            "a Conversation that has never moved is born on this device under \
             its own id, which is what every row from before is",
        );
    }
}

#[tokio::test]
async fn a_key_that_came_from_another_device_is_left_exactly_as_it_is() {
    let (_dir, pool) = fresh_pool().await;
    let repo = repo(&pool).await;

    // A copy that arrived over the link: this database's own row, numbered here,
    // carrying the key of the machine the work was drafted on — which goes on
    // over the one the create stamped, a copy's key being nothing this device
    // invents.
    let copy = start(&pool, repo, "task-runner").await;

    let born = Birth {
        device: ANOTHER_DEVICE.to_owned(),
        id: 7,
    };
    record_birth(&pool, copy, &born).await.unwrap();

    // And a start that finds it already stamped, which is every start after the
    // one that did the stamping.
    stamp_the_births(&pool, THIS_DEVICE).await.unwrap();

    assert_eq!(
        birth(&pool, copy).await.unwrap(),
        Some(born),
        "a Conversation that has moved still answers to the key it was born \
         under, whatever machine is holding it now",
    );

    assert_eq!(
        row(&pool, copy).await.0,
        format!("{ANOTHER_DEVICE}/7"),
        "and its row says the same, so the copy and the original merge as one",
    );
}

#[tokio::test]
async fn a_copy_that_has_been_handed_on_says_where_the_live_record_is() {
    let (_dir, pool) = fresh_pool().await;
    let repo = repo(&pool).await;
    let conversation = start(&pool, repo, "task-runner").await;

    assert_eq!(
        transferred(&pool, conversation).await.unwrap(),
        None,
        "an ordinary Conversation is the record, and there is nowhere to send \
         anybody",
    );

    let to = Transferred {
        device: ANOTHER_DEVICE.to_owned(),
        id: 3,
    };
    transfer_away(&pool, conversation, &to).await.unwrap();

    assert_eq!(
        transferred(&pool, conversation).await.unwrap(),
        Some(to),
        "the copy left behind names the device holding the record and the id it \
         goes by there",
    );

    assert_eq!(
        row(&pool, conversation).await,
        (format!("{THIS_DEVICE}/{conversation}"), true),
        "and the row says it is a tombstone while keeping the key it was born \
         under: the mark is which copy, and the key is which work",
    );

    // Handed on again, from wherever it had got to: what the mark says is where
    // the record is *now*, and a copy that passed it on a second time has a
    // second thing to say.
    let onwards = Transferred {
        device: "ff11223344556677889900aabbccddee".to_owned(),
        id: 12,
    };
    transfer_away(&pool, conversation, &onwards).await.unwrap();

    assert_eq!(
        transferred(&pool, conversation).await.unwrap(),
        Some(onwards),
        "the mark is replaced rather than added to: one live record at a time",
    );
}
