//! The **Replicated records**, as one device writes them down: a write stamped
//! by the stamping rule, a removal that is final, and a kind taken up once out
//! of wherever it was kept before (ADR-0028).
//!
//! The two rules themselves are the module's own unit tests; what is asked here
//! is that the store writes by them.

use serde::{Deserialize, Serialize};
use verkstead_store::open_database;
use verkstead_store::replicated::{
    Kind, Record, Written, live_records, merge, records, remove, rewrite, take_up, taken_up, write,
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

fn arriving(
    uuid: &str,
    updated_at: i64,
    deleted_at: Option<i64>,
    by: &str,
    text: &str,
) -> Record<Note> {
    Record {
        uuid: uuid.to_owned(),
        updated_at,
        deleted_at,
        written_by: by.to_owned(),
        fields: note(text),
    }
}

/// A member's table merged here: a record never held is written down as it
/// arrives, removed or not, and the same table merged twice moves nothing.
#[tokio::test]
async fn a_members_records_are_merged_and_a_second_merge_moves_nothing() {
    let (_dir, pool) = fresh_pool().await;

    let served = [
        arriving("one", 10, None, "b", "first"),
        arriving("two", 12, Some(12), "b", "gone"),
    ];

    assert!(merge::<Notes>(&pool, &served).await.unwrap());
    assert!(!merge::<Notes>(&pool, &served).await.unwrap());

    assert_eq!(records::<Notes>(&pool).await.unwrap(), served);
    let live: Vec<_> = live_records::<Notes>(&pool)
        .await
        .unwrap()
        .into_iter()
        .map(|record| record.uuid)
        .collect();
    assert_eq!(live, ["one"]);
}

/// Merged by the rule: a newer edit replaces what is held, an older one is
/// dropped, and an edit arriving after a removal stays removed.
#[tokio::test]
async fn a_merge_keeps_the_newest_and_a_removal_final() {
    let (_dir, pool) = fresh_pool().await;

    merge::<Notes>(
        &pool,
        &[
            arriving("one", 10, None, "a", "first"),
            arriving("two", 10, Some(10), "a", "removed"),
        ],
    )
    .await
    .unwrap();

    assert!(
        !merge::<Notes>(&pool, &[arriving("one", 9, None, "b", "older")])
            .await
            .unwrap()
    );
    assert!(
        merge::<Notes>(&pool, &[arriving("one", 11, None, "b", "newer")])
            .await
            .unwrap()
    );
    assert!(
        !merge::<Notes>(&pool, &[arriving("two", 20, None, "b", "edited after")])
            .await
            .unwrap()
    );

    let held = records::<Notes>(&pool).await.unwrap();
    assert_eq!(held[0], arriving("one", 11, None, "b", "newer"));
    assert_eq!(held[1], arriving("two", 10, Some(10), "a", "removed"));
}

/// An Author mirror an older Verkstead wrote down is a record of the same uuid
/// stamped older than anything, and the `author_mirrors` table is gone.
#[tokio::test]
async fn the_author_mirrors_are_records_stamped_oldest() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("verkstead.db");

    let pool = open_database(&path).await.unwrap();
    sqlx::query(
        "CREATE TABLE author_mirrors (
             id           TEXT PRIMARY KEY,
             home_device  TEXT NOT NULL,
             display_name TEXT NOT NULL,
             name         TEXT NOT NULL,
             email        TEXT NOT NULL,
             holds_token  INTEGER NOT NULL
         ) STRICT",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO author_mirrors VALUES
             ('work', 'device-a', '', 'Ada', 'ada@work.example', 1),
             ('home', 'device-a', 'Home', 'Ada L.', '', 0)",
    )
    .execute(&pool)
    .await
    .unwrap();
    pool.close().await;

    let pool = open_database(&path).await.unwrap();

    let held: Vec<(String, i64, Option<i64>, String, serde_json::Value)> = sqlx::query_as(
        "SELECT uuid, updated_at, deleted_at, written_by, body FROM replicated
         WHERE kind = 'authors' ORDER BY uuid",
    )
    .fetch_all(&pool)
    .await
    .unwrap()
    .into_iter()
    .map(
        |(uuid, at, deleted, by, body): (String, i64, Option<i64>, String, String)| {
            (uuid, at, deleted, by, serde_json::from_str(&body).unwrap())
        },
    )
    .collect();

    assert_eq!(
        held,
        [
            (
                "home".to_owned(),
                0,
                None,
                "device-a".to_owned(),
                serde_json::json!({
                    "display_name": "Home",
                    "name": "Ada L.",
                    "email": null,
                    "holds_token": false,
                }),
            ),
            (
                "work".to_owned(),
                0,
                None,
                "device-a".to_owned(),
                serde_json::json!({
                    "display_name": null,
                    "name": "Ada",
                    "email": "ada@work.example",
                    "holds_token": true,
                }),
            ),
        ]
    );

    let mirrors: Option<(String,)> =
        sqlx::query_as("SELECT name FROM sqlite_master WHERE name = 'author_mirrors'")
            .fetch_optional(&pool)
            .await
            .unwrap();
    assert_eq!(mirrors, None);
}

/// And a kind taken up over a record stamped oldest replaces it, which is this
/// device's own Author over a mirror of it.
#[tokio::test]
async fn a_take_up_replaces_a_record_stamped_oldest() {
    let (_dir, pool) = fresh_pool().await;

    merge::<Notes>(&pool, &[arriving("one", 0, None, "b", "mirrored")])
        .await
        .unwrap();
    take_up::<Notes>(&pool, "a", &[("one".to_owned(), note("own"))])
        .await
        .unwrap();

    let held = records::<Notes>(&pool).await.unwrap();
    assert_eq!(held[0].fields, note("own"));
    assert_eq!(held[0].written_by, "a");
}
