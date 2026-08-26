//! Which Conversations the human has put away: the ones a Closed Conversation
//! becomes when there is nothing left to read on it.
//!
//! Archiving is a fact about the sidebar rather than about the work, so it is
//! beside the Conversations rather than a column on them — the reason a
//! placement is, said again: there is no migration machinery here, and
//! `conversations` is STRICT and left alone. The row being there is the whole
//! of the flag, and taking it away is what unarchiving will be.
//!
//! Nothing leaves a Timeline. An archived Conversation is one the list stops
//! drawing and nothing else: its record is where it was, its branch is where it
//! was, and opening it by its own URL shows all of it.
//!
//! `archived_conversations` rather than `archivings`, which is taken: that is
//! where a locked Question Set is stored, under the name locking went by before
//! it was called locking — see [`super::apply_schema`].

use anyhow::{Context, Result};
use sqlx::SqlitePool;

use super::conversations::Lifecycle;

/// What became of archiving one.
///
/// Named the way [`Closing`](super::Closing)'s outcomes are, and refused for the
/// one reason closing is not: a Conversation still being worked on is not
/// something to hide from the list, so it is closed first and archived after.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Archiving {
    /// Archived: the row is written, and the sidebar stops drawing it.
    Archived,

    /// It was archived already. Nothing to record and nothing wrong — what the
    /// human asked for holds either way.
    AlreadyArchived,

    /// It has not been closed, so there is nothing to put away yet.
    NotClosed,

    /// There is no Conversation with that id.
    NoSuchConversation,
}

/// The table an archived Conversation's row lives in.
pub(crate) async fn apply_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS archived_conversations (
             conversation_id INTEGER PRIMARY KEY REFERENCES conversations(id),
             archived_at     TEXT NOT NULL
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the archived_conversations table")?;

    Ok(())
}

/// Put a Closed Conversation away, stamping it with the time.
///
/// The state is read and the row written in one transaction, so a Conversation
/// steered back into the work from another device between the two cannot end up
/// hidden from the list it is being worked in.
pub async fn archive_conversation(pool: &SqlitePool, id: i64) -> Result<Archiving> {
    let mut tx = pool.begin().await.context("archiving a Conversation")?;

    let row: Option<(String,)> = sqlx::query_as("SELECT state FROM conversations WHERE id = ?")
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .with_context(|| format!("reading the state of Conversation {id}"))?;

    let Some((state,)) = row else {
        return Ok(Archiving::NoSuchConversation);
    };

    if Lifecycle::read(&state)? != Lifecycle::Closed {
        return Ok(Archiving::NotClosed);
    }

    let written = sqlx::query(
        "INSERT INTO archived_conversations (conversation_id, archived_at)
         VALUES (?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
         ON CONFLICT (conversation_id) DO NOTHING",
    )
    .bind(id)
    .execute(&mut *tx)
    .await
    .with_context(|| format!("archiving Conversation {id}"))?;

    tx.commit().await.context("archiving a Conversation")?;

    Ok(if written.rows_affected() == 0 {
        Archiving::AlreadyArchived
    } else {
        Archiving::Archived
    })
}
