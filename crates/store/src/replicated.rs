//! The **Replicated records**: a record that is the cluster's rather than a
//! device's, held by every device of it (ADR-0028).
//!
//! **One thing with record kinds on it.** Authors are the first kind; Agent
//! Profiles and their bindings, the Instructions text and the declared MCP
//! servers follow onto the same table, so what a record *is* is said here
//! once. Every one of them is:
//!
//! ```text
//! kind        the kind it is, "authors" and so on
//! uuid        the record's, the same on every device
//! updated_at  the stamp of the last write, in milliseconds since the epoch
//! deleted_at  the deletion stamp, or nothing — a removal is final
//! written_by  the Device Id that wrote it last, which breaks a tie
//! body        the kind's own fields, as JSON
//! ```
//!
//! The kind's fields are a JSON body rather than columns of their own for the
//! reason a Set's are: what a kind holds is the kind's to say, and the record's
//! shape around it is the same whatever that is. See [`Kind`].
//!
//! **Two rules, each one function.** [`arriving_wins`] is the merge: per
//! record, the newest stamp wins with the writer's Device Id breaking a tie,
//! and a record carrying a deletion stamp is never overwritten by an edit.
//! [`stamped`] is how a write here is stamped: past what it was made on top
//! of, whatever this device's clock says.
//!
//! **Removed records are kept**, never pruned: a deletion stamp is what tells a
//! device that was off when the removal happened that its edit is to stay
//! removed. The tables are tiny.
//!
//! Ordered by when each record was first written down here, which is what a
//! kind with a first — the one Author the wizard is about — reads its first
//! as: a rewrite keeps a record's place.

use anyhow::{Context, Result};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

/// The tables the records live in: the records of every kind, and which kinds
/// have been taken up out of wherever they were kept before — see
/// [`take_up`].
///
/// Hanging off nothing, for the reason the mirrors never did: a record is
/// nobody's to cascade from.
pub(crate) async fn apply_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS replicated (
             kind       TEXT NOT NULL,
             uuid       TEXT NOT NULL,
             updated_at INTEGER NOT NULL,
             deleted_at INTEGER,
             written_by TEXT NOT NULL,
             body       TEXT NOT NULL,
             PRIMARY KEY (kind, uuid)
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the replicated table")?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS replicated_taken_up (
             kind TEXT PRIMARY KEY
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the replicated_taken_up table")?;

    Ok(())
}

/// A kind of record: what it is called in the table, and what it holds.
///
/// Declared by whoever owns the kind — the server, for Authors — so that this
/// crate says what a record is and nothing about what an Author is.
pub trait Kind {
    /// What its records are filed under, which is the same on every device.
    const NAME: &'static str;

    /// The kind's own fields, written down as the record's body.
    ///
    /// Compared to tell a write that changes nothing from one that does: a
    /// save of every section of a page rewrites nothing it did not change.
    type Fields: Serialize + DeserializeOwned + PartialEq + Clone;
}

/// One record of a kind, as this device holds it — and as it crosses to a
/// member, which is the same record: every device serves its whole table of a
/// kind, removed records included, and merges what its members serve.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Record<F> {
    pub uuid: String,
    pub updated_at: i64,
    pub deleted_at: Option<i64>,
    pub written_by: String,
    pub fields: F,
}

impl<F> Record<F> {
    /// Whether it has been removed.
    pub fn removed(&self) -> bool {
        self.deleted_at.is_some()
    }

    /// What the merge rule reads of it.
    pub fn stamp(&self) -> Stamp<'_> {
        Stamp {
            updated_at: self.updated_at,
            deleted_at: self.deleted_at,
            written_by: &self.written_by,
        }
    }
}

/// What the merge rule reads of a record: when it was last written, whether it
/// has been removed, and by which device.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stamp<'a> {
    pub updated_at: i64,
    pub deleted_at: Option<i64>,
    pub written_by: &'a str,
}

impl Stamp<'_> {
    /// The order two writes neither of which had seen the other are decided
    /// in: the later stamp, and the greater Device Id between equal ones.
    fn newer_than(&self, other: &Stamp<'_>) -> bool {
        (self.updated_at, self.written_by) > (other.updated_at, other.written_by)
    }
}

/// **The merge rule**: whether a record arriving from another device replaces
/// the one held here under its uuid.
///
/// - **A removal is final.** A held record carrying a deletion stamp is never
///   overwritten by an edit, however late that edit is stamped: an edit made
///   on a device that was apart when the removal happened stays removed.
/// - **A deletion stamp arriving beats any edit**, for the same reason.
/// - Otherwise **the newest write wins**, the writing device's id breaking a
///   tie between equal stamps — and so between two removals, so that every
///   device ends up holding the same one.
///
/// What is held already wins anything equal to it, which is the same record
/// pulled twice: a pull is idempotent.
pub fn arriving_wins(held: Stamp<'_>, arriving: Stamp<'_>) -> bool {
    match (held.deleted_at, arriving.deleted_at) {
        (Some(_), None) => false,
        (None, Some(_)) => true,
        _ => arriving.newer_than(&held),
    }
}

/// **The stamping rule**: what a write made here at `now` is stamped with, on
/// top of a record this device holds stamped `held` — or none.
///
/// The later of this device's now and one past what it was made on top of.
/// The stamps are wall clocks on different machines, and a VM or a laptop that
/// slept can run behind: stamped with its own clock alone, an edit made there
/// shortly after an edit elsewhere would be the older of the two, and the next
/// pull would quietly revert it on the very device it was made on.
pub fn stamped(now: i64, held: Option<i64>) -> i64 {
    held.map_or(now, |held| now.max(held + 1))
}

/// This device's clock, in the unit the stamps are written in.
pub(crate) fn now() -> i64 {
    let nanos = time::OffsetDateTime::now_utc().unix_timestamp_nanos();

    i64::try_from(nanos / 1_000_000).unwrap_or(i64::MAX)
}

type Row = (String, i64, Option<i64>, String, String);

fn record<F: DeserializeOwned>(
    kind: &str,
    (uuid, updated_at, deleted_at, written_by, body): Row,
) -> Result<Record<F>> {
    let fields =
        serde_json::from_str(&body).with_context(|| format!("reading the {kind} record {uuid}"))?;

    Ok(Record {
        uuid,
        updated_at,
        deleted_at,
        written_by,
        fields,
    })
}

/// Every record of kind `K` this device holds, the removed ones included, in
/// the order each was first written down here.
pub async fn records<K: Kind>(pool: &SqlitePool) -> Result<Vec<Record<K::Fields>>> {
    let mut connection = pool
        .acquire()
        .await
        .with_context(|| format!("reading the {} records", K::NAME))?;

    records_on::<K>(&mut connection).await
}

/// And the same off a connection already held, which is how a read or a write
/// that has to see one moment of the table — a Profile's uniqueness checked
/// against the whole of it, then written — reads it inside its own
/// transaction.
pub(crate) async fn records_on<K: Kind>(
    connection: &mut sqlx::SqliteConnection,
) -> Result<Vec<Record<K::Fields>>> {
    let rows: Vec<Row> = sqlx::query_as(
        "SELECT uuid, updated_at, deleted_at, written_by, body
         FROM replicated
         WHERE kind = ?
         ORDER BY rowid",
    )
    .bind(K::NAME)
    .fetch_all(&mut *connection)
    .await
    .with_context(|| format!("reading the {} records", K::NAME))?;

    rows.into_iter().map(|row| record(K::NAME, row)).collect()
}

/// One record of kind `K`, removed or not, off a connection already held.
pub(crate) async fn record_on<K: Kind>(
    connection: &mut sqlx::SqliteConnection,
    uuid: &str,
) -> Result<Option<Record<K::Fields>>> {
    let row: Option<Row> = sqlx::query_as(
        "SELECT uuid, updated_at, deleted_at, written_by, body
         FROM replicated
         WHERE kind = ? AND uuid = ?",
    )
    .bind(K::NAME)
    .bind(uuid)
    .fetch_optional(&mut *connection)
    .await
    .with_context(|| format!("reading the {} record {uuid}", K::NAME))?;

    row.map(|row| record(K::NAME, row)).transpose()
}

/// And the ones of them that have not been removed, which is every list and
/// picker's.
pub async fn live_records<K: Kind>(pool: &SqlitePool) -> Result<Vec<Record<K::Fields>>> {
    Ok(records::<K>(pool)
        .await?
        .into_iter()
        .filter(|record| !record.removed())
        .collect())
}

/// What a write here came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Written {
    /// Written down, stamped by [`stamped`].
    Written,

    /// The record already held exactly these fields, and nothing was written:
    /// a write that changes nothing is no news to anybody.
    Unchanged,

    /// The record has been removed, which is final: nothing was written.
    Removed,

    /// There is no such record, and the write was one that edits rather than
    /// makes — see [`rewrite`].
    Absent,
}

/// Write `fields` as the record of kind `K` under `uuid`, made here by
/// `device`: a new record where there is none, and an edit of the one held
/// where there is.
pub async fn write<K: Kind>(
    pool: &SqlitePool,
    device: &str,
    uuid: &str,
    fields: &K::Fields,
) -> Result<Written> {
    writing::<K>(pool, device, uuid, fields, true).await
}

/// And the same where only an edit is meant: `Absent` where no record is held
/// under `uuid`, and nothing written.
pub async fn rewrite<K: Kind>(
    pool: &SqlitePool,
    device: &str,
    uuid: &str,
    fields: &K::Fields,
) -> Result<Written> {
    writing::<K>(pool, device, uuid, fields, false).await
}

async fn writing<K: Kind>(
    pool: &SqlitePool,
    device: &str,
    uuid: &str,
    fields: &K::Fields,
    making: bool,
) -> Result<Written> {
    let mut tx = super::writing(pool, "writing a record").await?;

    let written = written_on::<K>(&mut tx, device, uuid, fields, making).await?;

    tx.commit()
        .await
        .with_context(|| format!("writing the {} record {uuid}", K::NAME))?;

    Ok(written)
}

/// [`write`] or [`rewrite`], inside a transaction somebody else opened — so
/// that what decides whether to write, and the write, are one moment.
pub(crate) async fn written_on<K: Kind>(
    connection: &mut sqlx::SqliteConnection,
    device: &str,
    uuid: &str,
    fields: &K::Fields,
    making: bool,
) -> Result<Written> {
    let held = record_on::<K>(&mut *connection, uuid).await?;

    match &held {
        None if !making => return Ok(Written::Absent),
        Some(held) if held.removed() => return Ok(Written::Removed),
        Some(held) if held.fields == *fields => return Ok(Written::Unchanged),
        _ => {}
    }

    let body = serde_json::to_string(fields)
        .with_context(|| format!("writing down the {} record {uuid}", K::NAME))?;

    sqlx::query(
        "INSERT INTO replicated (kind, uuid, updated_at, deleted_at, written_by, body)
         VALUES (?, ?, ?, NULL, ?, ?)
         ON CONFLICT (kind, uuid) DO UPDATE SET
             updated_at = excluded.updated_at,
             written_by = excluded.written_by,
             body = excluded.body",
    )
    .bind(K::NAME)
    .bind(uuid)
    .bind(stamped(now(), held.map(|held| held.updated_at)))
    .bind(device)
    .bind(body)
    .execute(&mut *connection)
    .await
    .with_context(|| format!("writing the {} record {uuid}", K::NAME))?;

    Ok(Written::Written)
}

/// Remove the record of kind `K` under `uuid`, by `device`: a deletion stamp
/// written on it, stamped the way a write is. `false` where there is no live
/// record under it and nothing was written.
///
/// The record is kept, its fields with it: the deletion stamp is what every
/// other device is to learn from it.
pub async fn remove<K: Kind>(pool: &SqlitePool, device: &str, uuid: &str) -> Result<bool> {
    let mut tx = super::writing(pool, "removing a record").await?;

    let removed = removed_on::<K>(&mut tx, device, uuid).await?;

    tx.commit()
        .await
        .with_context(|| format!("removing the {} record {uuid}", K::NAME))?;

    Ok(removed)
}

/// [`remove`], inside a transaction somebody else opened.
pub(crate) async fn removed_on<K: Kind>(
    connection: &mut sqlx::SqliteConnection,
    device: &str,
    uuid: &str,
) -> Result<bool> {
    let held: Option<(i64, Option<i64>)> =
        sqlx::query_as("SELECT updated_at, deleted_at FROM replicated WHERE kind = ? AND uuid = ?")
            .bind(K::NAME)
            .bind(uuid)
            .fetch_optional(&mut *connection)
            .await
            .with_context(|| format!("reading the {} record {uuid}", K::NAME))?;

    let Some((updated_at, None)) = held else {
        return Ok(false);
    };

    let stamp = stamped(now(), Some(updated_at));

    sqlx::query(
        "UPDATE replicated SET updated_at = ?, deleted_at = ?, written_by = ?
         WHERE kind = ? AND uuid = ?",
    )
    .bind(stamp)
    .bind(stamp)
    .bind(device)
    .bind(K::NAME)
    .bind(uuid)
    .execute(&mut *connection)
    .await
    .with_context(|| format!("removing the {} record {uuid}", K::NAME))?;

    Ok(true)
}

/// Merge what a member served of kind `K` into what this device holds, by
/// [`arriving_wins`] record by record, in one transaction — and say whether
/// anything here moved.
///
/// **Idempotent**: the same records merged twice move nothing the second time,
/// so a pull made on a Nudge that was not needed costs a read and nothing
/// else. A record this device has never held is written down whatever it
/// carries, a removed one included: the deletion stamp is what tells an edit
/// that arrives later to stay removed.
pub async fn merge<K: Kind>(pool: &SqlitePool, arriving: &[Record<K::Fields>]) -> Result<bool> {
    let mut tx = super::writing(pool, "merging records").await?;
    let mut moved = false;

    for record in arriving {
        let held: Option<(i64, Option<i64>, String)> = sqlx::query_as(
            "SELECT updated_at, deleted_at, written_by FROM replicated
             WHERE kind = ? AND uuid = ?",
        )
        .bind(K::NAME)
        .bind(&record.uuid)
        .fetch_optional(&mut *tx)
        .await
        .with_context(|| format!("reading the {} record {}", K::NAME, record.uuid))?;

        let wins = held
            .as_ref()
            .is_none_or(|(updated_at, deleted_at, written_by)| {
                arriving_wins(
                    Stamp {
                        updated_at: *updated_at,
                        deleted_at: *deleted_at,
                        written_by,
                    },
                    record.stamp(),
                )
            });

        if !wins {
            continue;
        }

        let body = serde_json::to_string(&record.fields)
            .with_context(|| format!("writing down the {} record {}", K::NAME, record.uuid))?;

        sqlx::query(
            "INSERT INTO replicated (kind, uuid, updated_at, deleted_at, written_by, body)
             VALUES (?, ?, ?, ?, ?, ?)
             ON CONFLICT (kind, uuid) DO UPDATE SET
                 updated_at = excluded.updated_at,
                 deleted_at = excluded.deleted_at,
                 written_by = excluded.written_by,
                 body = excluded.body",
        )
        .bind(K::NAME)
        .bind(&record.uuid)
        .bind(record.updated_at)
        .bind(record.deleted_at)
        .bind(&record.written_by)
        .bind(body)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("merging the {} record {}", K::NAME, record.uuid))?;

        moved = true;
    }

    tx.commit()
        .await
        .with_context(|| format!("merging the {} records", K::NAME))?;

    Ok(moved)
}

/// Whether kind `K` has been taken up already — see [`take_up`].
pub async fn taken_up<K: Kind>(pool: &SqlitePool) -> Result<bool> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT kind FROM replicated_taken_up WHERE kind = ?")
            .bind(K::NAME)
            .fetch_optional(pool)
            .await
            .with_context(|| format!("reading whether the {} have been taken up", K::NAME))?;

    Ok(row.is_some())
}

/// Take kind `K` up out of wherever it was kept before records were — every
/// one of `kept`, under the uuid it already carries, written by `device` —
/// and say that it has been, in one transaction.
///
/// **Once.** What is said here is what says the old place is never read for
/// the kind again: a record written into it afterwards by hand is somebody
/// editing a file nothing reads, which the caller is to say. A uuid already
/// held is left as it is, unless what is held is a live record stamped older
/// than anything, at the epoch — an Author mirror written down as a record,
/// see [`super::migrations`] — which any real write is to replace.
pub async fn take_up<K: Kind>(
    pool: &SqlitePool,
    device: &str,
    kept: &[(String, K::Fields)],
) -> Result<()> {
    let mut tx = super::writing(pool, "taking records up").await?;
    let stamp = now();

    for (uuid, fields) in kept {
        let body = serde_json::to_string(fields)
            .with_context(|| format!("writing down the {} record {uuid}", K::NAME))?;

        sqlx::query(
            "INSERT INTO replicated (kind, uuid, updated_at, deleted_at, written_by, body)
             VALUES (?, ?, ?, NULL, ?, ?)
             ON CONFLICT (kind, uuid) DO UPDATE SET
                 updated_at = excluded.updated_at,
                 written_by = excluded.written_by,
                 body = excluded.body
             WHERE replicated.deleted_at IS NULL AND replicated.updated_at = 0",
        )
        .bind(K::NAME)
        .bind(uuid)
        .bind(stamp)
        .bind(device)
        .bind(body)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("taking up the {} record {uuid}", K::NAME))?;
    }

    sqlx::query("INSERT INTO replicated_taken_up (kind) VALUES (?) ON CONFLICT (kind) DO NOTHING")
        .bind(K::NAME)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("saying the {} have been taken up", K::NAME))?;

    tx.commit()
        .await
        .with_context(|| format!("taking up the {}", K::NAME))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stamp(updated_at: i64, deleted_at: Option<i64>, written_by: &str) -> Stamp<'_> {
        Stamp {
            updated_at,
            deleted_at,
            written_by,
        }
    }

    #[test]
    fn the_newest_write_wins() {
        assert!(arriving_wins(stamp(10, None, "a"), stamp(11, None, "a")));
        assert!(!arriving_wins(stamp(11, None, "a"), stamp(10, None, "b")));
    }

    #[test]
    fn equal_stamps_are_decided_by_the_device_id() {
        assert!(arriving_wins(stamp(10, None, "a"), stamp(10, None, "b")));
        assert!(!arriving_wins(stamp(10, None, "b"), stamp(10, None, "a")));
    }

    #[test]
    fn the_same_record_pulled_twice_changes_nothing() {
        assert!(!arriving_wins(stamp(10, None, "a"), stamp(10, None, "a")));
        assert!(!arriving_wins(
            stamp(10, Some(10), "a"),
            stamp(10, Some(10), "a")
        ));
    }

    #[test]
    fn an_edit_stamped_after_a_deletion_stamp_is_dropped() {
        assert!(!arriving_wins(
            stamp(10, Some(10), "a"),
            stamp(20, None, "b")
        ));
    }

    #[test]
    fn a_deletion_stamp_arriving_beats_any_edit() {
        assert!(arriving_wins(
            stamp(20, None, "a"),
            stamp(10, Some(10), "b")
        ));
    }

    #[test]
    fn an_edit_older_than_what_is_held_is_dropped() {
        assert!(!arriving_wins(stamp(20, None, "a"), stamp(19, None, "z")));
    }

    #[test]
    fn of_two_removals_every_device_keeps_the_same_one() {
        let one = stamp(10, Some(10), "a");
        let other = stamp(12, Some(12), "b");

        assert!(arriving_wins(one, other));
        assert!(!arriving_wins(other, one));
    }

    #[test]
    fn a_write_is_stamped_with_this_devices_now() {
        assert_eq!(stamped(100, None), 100);
        assert_eq!(stamped(100, Some(40)), 100);
    }

    #[test]
    fn a_write_on_a_clock_behind_the_held_stamp_is_stamped_one_past_it() {
        assert_eq!(stamped(100, Some(250)), 251);
        assert_eq!(stamped(250, Some(250)), 251);
    }
}
