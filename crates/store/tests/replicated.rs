//! The **Replicated records**, as one device writes them down: a write stamped
//! by the stamping rule, a removal that is final, and a kind taken up once out
//! of wherever it was kept before (ADR-0028).
//!
//! The two rules themselves are the module's own unit tests; what is asked here
//! is that the store writes by them.

use serde::{Deserialize, Serialize};
use verkstead_store::open_database;
use verkstead_store::replicated::{
    Kind, Written, live_records, records, remove, rewrite, take_up, taken_up, write,
};

/// A kind of no consequence, standing in for Authors.
struct Notes;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Note {
    text: String,
}

impl Kind for Notes {
    const NAME: &'static str = "notes";
    type Fields = Note;
}

fn note(text: &str) -> Note {
    Note {
        text: text.to_owned(),
    }
}

/// A pool over a fresh database, plus the directory keeping it alive.
async fn fresh_pool() -> (tempfile::TempDir, sqlx::SqlitePool) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();
    (dir, pool)
}

#[tokio::test]
async fn a_write_is_a_record_stamped_by_this_device() {
    let (_dir, pool) = fresh_pool().await;

    assert_eq!(
        write::<Notes>(&pool, "a", "one", &note("first"))
            .await
            .unwrap(),
        Written::Written
    );

    let held = records::<Notes>(&pool).await.unwrap();
    assert_eq!(held.len(), 1);
    assert_eq!(held[0].uuid, "one");
    assert_eq!(held[0].written_by, "a");
    assert_eq!(held[0].deleted_at, None);
    assert_eq!(held[0].fields, note("first"));
}

/// The stamping rule at the store: a record held stamped far in the future —
/// written by a device whose clock runs ahead of this one's — is edited here
/// stamped one past it, never with this device's own earlier now.
#[tokio::test]
async fn an_edit_on_a_clock_behind_the_held_stamp_is_stamped_one_past_it() {
    let (_dir, pool) = fresh_pool().await;
    let ahead = i64::MAX / 2;

    write::<Notes>(&pool, "a", "one", &note("first"))
        .await
        .unwrap();
    sqlx::query("UPDATE replicated SET updated_at = ?, written_by = 'b'")
        .bind(ahead)
        .execute(&pool)
        .await
        .unwrap();

    write::<Notes>(&pool, "a", "one", &note("second"))
        .await
        .unwrap();

    let held = records::<Notes>(&pool).await.unwrap();
    assert_eq!(held[0].updated_at, ahead + 1);
    assert_eq!(held[0].written_by, "a");
    assert_eq!(held[0].fields, note("second"));
}

#[tokio::test]
async fn a_write_that_changes_nothing_writes_nothing() {
    let (_dir, pool) = fresh_pool().await;

    write::<Notes>(&pool, "a", "one", &note("first"))
        .await
        .unwrap();
    let before = records::<Notes>(&pool).await.unwrap();

    assert_eq!(
        write::<Notes>(&pool, "b", "one", &note("first"))
            .await
            .unwrap(),
        Written::Unchanged
    );
    assert_eq!(records::<Notes>(&pool).await.unwrap(), before);
}

#[tokio::test]
async fn an_edit_of_nothing_makes_nothing() {
    let (_dir, pool) = fresh_pool().await;

    assert_eq!(
        rewrite::<Notes>(&pool, "a", "one", &note("first"))
            .await
            .unwrap(),
        Written::Absent
    );
    assert!(records::<Notes>(&pool).await.unwrap().is_empty());
}

/// A removal is a deletion stamp on a record that is kept, and final: neither
/// an edit nor a write under the same uuid brings it back.
#[tokio::test]
async fn a_removal_is_a_deletion_stamp_and_final() {
    let (_dir, pool) = fresh_pool().await;

    write::<Notes>(&pool, "a", "one", &note("first"))
        .await
        .unwrap();
    write::<Notes>(&pool, "a", "two", &note("other"))
        .await
        .unwrap();
    let written = records::<Notes>(&pool).await.unwrap()[0].updated_at;

    assert!(remove::<Notes>(&pool, "b", "one").await.unwrap());
    assert!(
        !remove::<Notes>(&pool, "b", "one").await.unwrap(),
        "already"
    );
    assert!(!remove::<Notes>(&pool, "b", "none").await.unwrap(), "never");

    let held = records::<Notes>(&pool).await.unwrap();
    assert_eq!(held.len(), 2, "kept, never pruned");
    assert!(held[0].deleted_at.is_some_and(|stamp| stamp > written));
    assert_eq!(held[0].deleted_at, Some(held[0].updated_at));
    assert_eq!(held[0].written_by, "b");

    let live: Vec<_> = live_records::<Notes>(&pool)
        .await
        .unwrap()
        .into_iter()
        .map(|record| record.uuid)
        .collect();
    assert_eq!(live, ["two"]);

    assert_eq!(
        write::<Notes>(&pool, "a", "one", &note("again"))
            .await
            .unwrap(),
        Written::Removed
    );
    assert_eq!(
        rewrite::<Notes>(&pool, "a", "one", &note("again"))
            .await
            .unwrap(),
        Written::Removed
    );
}

/// The order a kind's records are read in is the order each was first written
/// down, an edit keeping its place.
#[tokio::test]
async fn records_keep_the_order_they_were_first_written_in() {
    let (_dir, pool) = fresh_pool().await;

    for uuid in ["c", "a", "b"] {
        write::<Notes>(&pool, "a", uuid, &note(uuid)).await.unwrap();
    }
    write::<Notes>(&pool, "a", "c", &note("edited"))
        .await
        .unwrap();

    let order: Vec<_> = records::<Notes>(&pool)
        .await
        .unwrap()
        .into_iter()
        .map(|record| record.uuid)
        .collect();
    assert_eq!(order, ["c", "a", "b"]);
}

#[tokio::test]
async fn a_kind_is_taken_up_once_with_the_uuids_it_already_carried() {
    let (_dir, pool) = fresh_pool().await;

    assert!(!taken_up::<Notes>(&pool).await.unwrap());

    take_up::<Notes>(
        &pool,
        "a",
        &[
            ("one".into(), note("first")),
            ("two".into(), note("second")),
        ],
    )
    .await
    .unwrap();

    assert!(taken_up::<Notes>(&pool).await.unwrap());

    let held = records::<Notes>(&pool).await.unwrap();
    let uuids: Vec<_> = held.iter().map(|record| record.uuid.as_str()).collect();
    assert_eq!(uuids, ["one", "two"]);
    assert!(held.iter().all(|record| record.written_by == "a"));

    // Taken up again, which nothing does — but a uuid already held is left as
    // it is.
    take_up::<Notes>(&pool, "b", &[("one".into(), note("other"))])
        .await
        .unwrap();
    assert_eq!(records::<Notes>(&pool).await.unwrap(), held);
}
