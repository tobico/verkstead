//! Which Conversations Verkstead has told the human about and they have not
//! looked at yet.
//!
//! The first thing here that records what the *human* has done rather than what
//! the work has: every other fact in this database is about a Conversation, and
//! this one is about the person reading the list of them. It lives on the server
//! all the same, and for the reason the archivings do — a mark kept in a browser
//! would be one their phone had never heard of, and a notification read on the
//! phone would leave the laptop's sidebar still calling for attention.
//!
//! One row or none per Conversation, beside them rather than a column on them:
//! there is no migration machinery here and `conversations` is STRICT, which is
//! the placements' reason and the archivings' said a third time. The row being
//! there is the whole of the mark, and two things take it away: the human
//! looking at the Conversation, and the Conversation closing, which is them
//! saying the work is over wherever it had got to. Both are them being done with
//! it, and neither leaves news to go back for.
//!
//! What writes one is narrow on purpose: the wrap-up that carries a Conversation
//! to Done and pushes the news to the devices, in the same breath as the push —
//! see the server's `settling`. A milestone nobody was watching happen is
//! exactly what a mark saying *look here* is for, and a move the human made
//! themselves is exactly what it is not.

use anyhow::{Context, Result};
use sqlx::SqlitePool;

/// The table an unseen Conversation's row lives in.
pub(crate) async fn apply_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS unseen_conversations (
             conversation_id INTEGER PRIMARY KEY REFERENCES conversations(id),
             at              TEXT NOT NULL
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the unseen_conversations table")?;

    Ok(())
}

/// Say that there is something on this Conversation the human has not seen.
///
/// Written again where it was written already, which says the same thing: the
/// mark is *there is news here* rather than a count of how much, and a
/// Conversation cannot be twice unlooked-at.
pub async fn stamp_unseen(pool: &SqlitePool, conversation_id: i64) -> Result<()> {
    sqlx::query(
        "INSERT INTO unseen_conversations (conversation_id, at)
         VALUES (?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
         ON CONFLICT (conversation_id) DO NOTHING",
    )
    .bind(conversation_id)
    .execute(pool)
    .await
    .with_context(|| format!("stamping Conversation {conversation_id} unseen"))?;

    Ok(())
}

/// And that they are done with it, which takes the mark away — they have looked
/// at the Conversation, or they have closed it.
///
/// Answers whether there was one to take, which is what tells a first opening
/// from the many that follow it: the caller announces the list has moved on the
/// strength of it, and a Conversation opened twice should not have every other
/// device read its sidebar again for nothing. Closing has its own reason to
/// announce and ignores the answer.
///
/// Refused for nothing, and silent about a Conversation that has gone: opening
/// something is not a claim that it is still there, and there is no mark left on
/// one that is not.
pub async fn see_conversation(pool: &SqlitePool, conversation_id: i64) -> Result<bool> {
    let cleared = sqlx::query("DELETE FROM unseen_conversations WHERE conversation_id = ?")
        .bind(conversation_id)
        .execute(pool)
        .await
        .with_context(|| format!("clearing the unseen mark on Conversation {conversation_id}"))?;

    Ok(cleared.rows_affected() > 0)
}
