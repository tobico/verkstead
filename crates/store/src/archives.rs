//! Which Conversations the human has put away: the ones a Closed Conversation
//! becomes when there is nothing left to read on it.
//!
//! Archiving is a fact about the sidebar rather than about the work, so it is
//! beside the Conversations rather than a column on them — the reason a
//! placement is, said again: there is no migration machinery here, and
//! `conversations` is STRICT and left alone. The row being there is the whole
//! of the flag, and taking it away is what unarchiving is.
//!
//! Nothing leaves a Timeline. An archived Conversation is one the list stops
//! drawing and nothing else: its record is where it was, its branch is where it
//! was, and opening it by its own URL shows all of it.
//!
//! The way back is here too, both halves of it: unarchiving, which takes the
//! row away again, and the human's standing choice to be shown what they have
//! put away. That choice is a fact about the sidebar the way an archiving is,
//! so it is kept beside them rather than on the device that asked — a toggle
//! the next reload forgot would be one nobody would trust to hide anything.
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

/// And what became of taking one back out.
///
/// Milder than [`Archiving`] by one: there is no state to be in the wrong one
/// of. Whatever a Conversation has become since it was put away, putting it
/// back on the list is a thing the human can ask for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unarchiving {
    /// Unarchived: the row is gone, and the sidebar draws it again.
    Unarchived,

    /// It was not archived. Nothing to take away and nothing wrong — what the
    /// human asked for holds either way.
    NotArchived,

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

    // And whether the sidebar is drawing them, which is one row or none: the
    // presence of it is the whole of the setting, the way an archiving's row
    // is the whole of an archiving. A column holding a `0` would be a second
    // way to say what an empty table already says.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS shown_archives (
             only_row INTEGER PRIMARY KEY CHECK (only_row = 0)
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the shown_archives table")?;

    Ok(())
}

/// Put a Closed Conversation away, stamping it with the time.
///
/// The state is read and the row written in one transaction, so a Conversation
/// steered back into the work from another device between the two cannot end up
/// hidden from the list it is being worked in.
pub async fn archive_conversation(pool: &SqlitePool, id: i64) -> Result<Archiving> {
    let mut tx = super::writing(pool, "archiving a Conversation").await?;

    let row: Option<(String,)> = sqlx::query_as("SELECT state FROM conversations WHERE id = ?")
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .with_context(|| format!("reading the state of Conversation {id}"))?;

    let Some((state,)) = row else {
        return Ok(Archiving::NoSuchConversation);
    };

    // Tolerantly, as the close is: a word this Verkstead cannot parse is not
    // Closed, so this answers `NotClosed`. Which is the safe way round and the
    // deliberate one — archiving is *hide it from the list*, and hiding a
    // Conversation whose worktree may still be live would put the work out of
    // sight without ending it. Close and archive is the press that works end to
    // end on a row like that, because closing heals the word first.
    if !Lifecycle::reads_as(&state, Lifecycle::Closed) {
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

/// Take a Conversation back out, so the sidebar draws it again.
///
/// The Conversation is read before the row is taken away, so that unarchiving
/// something that is gone says so rather than reading as an unarchiving of
/// nothing — the two are told apart by whether there is a Conversation, which
/// is the only place the difference is written down.
pub async fn unarchive_conversation(pool: &SqlitePool, id: i64) -> Result<Unarchiving> {
    let mut tx = super::writing(pool, "unarchiving a Conversation").await?;

    let known: Option<(i64,)> = sqlx::query_as("SELECT id FROM conversations WHERE id = ?")
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .with_context(|| format!("looking for Conversation {id}"))?;

    if known.is_none() {
        return Ok(Unarchiving::NoSuchConversation);
    }

    let taken = sqlx::query("DELETE FROM archived_conversations WHERE conversation_id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("unarchiving Conversation {id}"))?;

    tx.commit().await.context("unarchiving a Conversation")?;

    Ok(if taken.rows_affected() == 0 {
        Unarchiving::NotArchived
    } else {
        Unarchiving::Unarchived
    })
}

/// Whether one Conversation has been put away.
///
/// What the Conversation's own page asks, so that the menu offering Archive can
/// offer Unarchive instead. The sidebar does not ask it per row — its own query
/// settles the whole list at once, see [`super::conversations`].
pub async fn archived(pool: &SqlitePool, id: i64) -> Result<bool> {
    let row: Option<(i64,)> = sqlx::query_as(
        "SELECT conversation_id FROM archived_conversations WHERE conversation_id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .with_context(|| format!("reading whether Conversation {id} is archived"))?;

    Ok(row.is_some())
}

/// Whether the sidebar is drawing what has been archived.
pub async fn showing_archived(pool: &SqlitePool) -> Result<bool> {
    let row: Option<(i64,)> = sqlx::query_as("SELECT only_row FROM shown_archives")
        .fetch_optional(pool)
        .await
        .context("reading whether the archived Conversations are shown")?;

    Ok(row.is_some())
}

/// Say whether it is, which is one row written or taken away.
///
/// Idempotent in both directions, because it is a switch rather than a press:
/// what arrives is the position the human has put it in, and asking for the
/// position it is already in is not a thing to refuse.
pub async fn show_archived(pool: &SqlitePool, showing: bool) -> Result<()> {
    if showing {
        sqlx::query("INSERT INTO shown_archives (only_row) VALUES (0) ON CONFLICT DO NOTHING")
            .execute(pool)
            .await
            .context("showing the archived Conversations")?;
    } else {
        sqlx::query("DELETE FROM shown_archives")
            .execute(pool)
            .await
            .context("hiding the archived Conversations")?;
    }

    Ok(())
}
