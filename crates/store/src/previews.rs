//! The Previews a building session suggests for trying its work by hand: a
//! title and the one line typed into a terminal to try it (ADR-0024).
//!
//! **A Preview is not an Event.** It is a side table of the Conversation, the
//! way a pending steer is: no card when one is suggested or replaced, and so no
//! place on a Share. What a Preview is *for* is a press in the Code pane rather
//! than a reading of the record, and a card for every replacement of *Build and
//! test* would be the Timeline recording the agent's housekeeping.
//!
//! **Keyed on its title.** The same title suggested again replaces the row
//! whole in place — command, description and stamp, so one suggested without a
//! description clears the one that stood — keeping the row where it was in the list; retiring
//! one deletes the row, there being nothing on the Timeline pointing at it to
//! keep.

use anyhow::{Context, Result};
use sqlx::SqlitePool;

/// One Preview standing on a Conversation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preview {
    /// What it goes by, and what it is replaced and retired by.
    pub title: String,

    /// The one line typed into a terminal to try the work.
    pub command: String,

    /// What it is for, as markdown its card shows above the command, where the
    /// session that last suggested it wrote any.
    pub description: Option<String>,

    /// The words of the session that last suggested it, on the Memo's pattern
    /// — *the session that worked task 03*.
    pub left_by: String,

    /// When it was last suggested, RFC 3339.
    pub at: String,
}

/// The previews table, one row per standing Preview.
///
/// Ordered by `rowid`, which an upsert keeps: a Preview replaced stays where
/// it was in the list, and the Transfer copies rows in that order too.
pub(crate) async fn apply_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS previews (
             conversation_id INTEGER NOT NULL REFERENCES conversations(id),
             title           TEXT NOT NULL,
             command         TEXT NOT NULL,
             description     TEXT,
             left_by         TEXT NOT NULL,
             at              TEXT NOT NULL,
             PRIMARY KEY (conversation_id, title)
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the previews table")?;

    Ok(())
}

/// Suggest a Preview on a Conversation, or replace the command, the
/// description and the stamp of the one standing under the same title — a
/// `None` description clearing the one that stood.
///
/// `false` means there is no such Conversation. Whether the session suggesting
/// it may, and whether the suggestion is well formed, are the caller's to have
/// settled.
pub async fn suggest_preview(
    pool: &SqlitePool,
    conversation_id: i64,
    title: &str,
    command: &str,
    description: Option<&str>,
    left_by: &str,
) -> Result<bool> {
    let suggested = sqlx::query(
        "INSERT INTO previews (conversation_id, title, command, description, left_by, at)
         SELECT id, ?, ?, ?, ?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
         FROM conversations WHERE id = ?
         ON CONFLICT (conversation_id, title) DO UPDATE
         SET command = excluded.command, description = excluded.description,
             left_by = excluded.left_by, at = excluded.at",
    )
    .bind(title)
    .bind(command)
    .bind(description)
    .bind(left_by)
    .bind(conversation_id)
    .execute(pool)
    .await
    .with_context(|| format!("suggesting a Preview on Conversation {conversation_id}"))?;

    Ok(suggested.rows_affected() > 0)
}

/// Take the Preview standing under `title` on a Conversation back.
///
/// `false` where none stands under that title, which the caller says by name.
pub async fn retire_preview(pool: &SqlitePool, conversation_id: i64, title: &str) -> Result<bool> {
    let retired = sqlx::query("DELETE FROM previews WHERE conversation_id = ? AND title = ?")
        .bind(conversation_id)
        .bind(title)
        .execute(pool)
        .await
        .with_context(|| format!("retiring a Preview of Conversation {conversation_id}"))?;

    Ok(retired.rows_affected() > 0)
}

/// Every Preview standing on a Conversation, oldest first: in the order each
/// title was first suggested.
pub async fn standing_previews(pool: &SqlitePool, conversation_id: i64) -> Result<Vec<Preview>> {
    let rows: Vec<(String, String, Option<String>, String, String)> = sqlx::query_as(
        "SELECT title, command, description, left_by, at FROM previews
         WHERE conversation_id = ? ORDER BY rowid",
    )
    .bind(conversation_id)
    .fetch_all(pool)
    .await
    .with_context(|| format!("reading the Previews of Conversation {conversation_id}"))?;

    Ok(rows
        .into_iter()
        .map(|(title, command, description, left_by, at)| Preview {
            title,
            command,
            description,
            left_by,
            at,
        })
        .collect())
}
