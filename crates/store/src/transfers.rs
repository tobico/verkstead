//! The request that a Conversation is to be moved onto another device, written
//! down until whatever is driving it acts on it (ADR-0020, *Transfer*).
//!
//! **A request rather than a move.** The press does not carry the work
//! anywhere: the session running for this Conversation is part way through a
//! turn, and a move that cut across it would leave the work wherever the agent
//! had got to. So what is written here is *this Conversation is going to that
//! device*, and the move runs once the turn has ended — which is Stop's shape
//! exactly, and for Stop's reason. See [`super::ask_to_stop`], the same sentence
//! about the same moment.
//!
//! **A table beside `conversations` rather than columns on it**, unlike the stop
//! it is shaped after. A stop asked for is a time and nothing else, and a column
//! held it; a transfer asked for is a time and the **Device Id** it is going to,
//! which is two things — the same reason the birth key and the mark beside it
//! are tables. See [`super::births`].
//!
//! **The last press is the one that stands.** Pressing again on another machine
//! is the human changing their mind about where the work should go, in the
//! minutes before it goes: the request is replaced rather than refused, and the
//! device named here is the one the move will be made to. Nothing has left this
//! machine until then, so there is nothing to take back.
//!
//! **And it is forgotten by whatever acts on it** — by the move that landed, or
//! by the move that failed and stopped the Conversation instead. A request left
//! behind would be a second move made at the next moment nothing was running.

use anyhow::{Context, Result};
use sqlx::SqlitePool;

/// The table: one request per Conversation at most, naming the device the work
/// is going to and when the press was made.
///
/// Keyed by the Conversation, so a second press replaces the first rather than
/// standing beside it.
pub(crate) async fn apply_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS transfers (
             conversation_id INTEGER PRIMARY KEY REFERENCES conversations(id),
             device          TEXT NOT NULL,
             asked_at        TEXT NOT NULL
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the transfers table")?;

    Ok(())
}

/// Ask for this Conversation to be moved onto `device` once whatever is running
/// has reached its end.
///
/// Nothing is moved and nothing is put on the Timeline. What the record holds
/// from here is that the work is going somewhere, which is what the Timeline
/// reads as *Transferring to* that machine and what keeps anything else from
/// being launched in the meantime.
pub async fn ask_to_transfer(pool: &SqlitePool, conversation_id: i64, device: &str) -> Result<()> {
    sqlx::query(
        "INSERT INTO transfers (conversation_id, device, asked_at)
         VALUES (?, ?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
         ON CONFLICT (conversation_id) DO UPDATE SET device = excluded.device,
                                                     asked_at = excluded.asked_at",
    )
    .bind(conversation_id)
    .bind(device)
    .execute(pool)
    .await
    .with_context(|| format!("asking for Conversation {conversation_id} to be transferred"))?;

    Ok(())
}

/// Which device this Conversation is on its way to, where a press has asked for
/// one.
///
/// `None` is every Conversation staying where it is, which is nearly all of
/// them.
pub async fn transfer_asked(pool: &SqlitePool, conversation_id: i64) -> Result<Option<String>> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT device FROM transfers WHERE conversation_id = ?")
            .bind(conversation_id)
            .fetch_optional(pool)
            .await
            .with_context(|| {
                format!("reading which device Conversation {conversation_id} is going to")
            })?;

    Ok(row.map(|(device,)| device))
}

/// Take the request away: the move has been made, or it failed and the
/// Conversation was stopped instead.
///
/// Nothing to do where none was asked for, which is every Conversation nobody
/// has pressed *Transfer to…* on.
pub async fn forget_transfer(pool: &SqlitePool, conversation_id: i64) -> Result<()> {
    sqlx::query("DELETE FROM transfers WHERE conversation_id = ?")
        .bind(conversation_id)
        .execute(pool)
        .await
        .with_context(|| {
            format!("forgetting the transfer asked for on Conversation {conversation_id}")
        })?;

    Ok(())
}
