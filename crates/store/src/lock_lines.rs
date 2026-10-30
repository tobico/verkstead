//! The line a lock carries, where whoever locked a Set said why (ADR-0029,
//! *Verkstead asks with a Question Set of its own*).
//!
//! A human's lock says nothing beyond itself: the human pressed it, with the
//! badge in front of them, and the sheet's own sentence about a Set locked
//! unanswered is the whole of what there is to say. One of Verkstead's own
//! Sets is locked by something happening instead — the device it was waiting
//! on answered, and the work was moved there — and *nobody answered it, and the
//! agent was told* would be wrong on both counts. So that lock carries a line,
//! drawn on the sheet and the Timeline in its place.
//!
//! A table of its own beside the lock, for the reason the asker is kept beside
//! the Set — see [`super::own_sets`]: a lock hangs off a Set rather than being a
//! column on it, and a fact about a lock hangs off it in turn. One row per lock
//! that said something, and none for one that did not.

use std::collections::HashMap;

use anyhow::{Context, Result};
use sqlx::{SqliteConnection, SqlitePool};

pub(crate) async fn apply_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS lock_lines (
             set_id INTEGER PRIMARY KEY REFERENCES archivings(set_id),
             line   TEXT NOT NULL
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the lock_lines table")?;

    Ok(())
}

/// Write down `line` as what Set `set_id`'s lock says, in the transaction that
/// is taking the lock.
pub(crate) async fn record(
    connection: &mut SqliteConnection,
    set_id: i64,
    line: &str,
) -> Result<()> {
    sqlx::query("INSERT INTO lock_lines (set_id, line) VALUES (?, ?)")
        .bind(set_id)
        .bind(line)
        .execute(&mut *connection)
        .await
        .with_context(|| format!("writing what the lock on Question Set {set_id} says"))?;

    Ok(())
}

/// What Set `set_id`'s lock says, where it was locked with a line.
pub async fn lock_line(pool: &SqlitePool, set_id: i64) -> Result<Option<String>> {
    let found: Option<(String,)> = sqlx::query_as("SELECT line FROM lock_lines WHERE set_id = ?")
        .bind(set_id)
        .fetch_optional(pool)
        .await
        .with_context(|| format!("reading what the lock on Question Set {set_id} says"))?;

    Ok(found.map(|(line,)| line))
}

/// What the locks on one Conversation's Sets say, by Set, for those locked
/// with a line.
pub async fn lock_lines(pool: &SqlitePool, conversation_id: i64) -> Result<HashMap<i64, String>> {
    let rows: Vec<(i64, String)> = sqlx::query_as(
        "SELECT l.set_id, l.line
         FROM lock_lines l
         JOIN set_events s ON s.set_id = l.set_id
         JOIN timeline_events e ON e.id = s.event_id
         WHERE e.conversation_id = ?",
    )
    .bind(conversation_id)
    .fetch_all(pool)
    .await
    .with_context(|| {
        format!("reading what the locks on Conversation {conversation_id}'s Sets say")
    })?;

    Ok(rows.into_iter().collect())
}
