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
//! **And it says who asked**: the human's press, or the session's own call to
//! `verkstead transfer` (ADR-0020, *The agent's call*). Whichever asked last is
//! the request that stands, and it is what the Timeline names once the move is
//! made — which is why the mark hands it back, read in the same transaction.
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

    // And who asked, through `ALTER TABLE` rather than in the declaration above,
    // so that a database made before there was any other way to ask takes the
    // same path. A request written then was a press, so that is the default.
    let there: Option<(String,)> =
        sqlx::query_as("SELECT name FROM pragma_table_info('transfers') WHERE name = ?")
            .bind("asked_by")
            .fetch_optional(pool)
            .await
            .context("looking for the column saying who asked for a transfer")?;

    if there.is_none() {
        sqlx::query("ALTER TABLE transfers ADD COLUMN asked_by TEXT NOT NULL DEFAULT 'human'")
            .execute(pool)
            .await
            .context("adding the column saying who asked for a transfer")?;
    }

    Ok(())
}

/// Who asked for a Conversation to be moved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AskedBy {
    /// The human, pressing *Transfer to…*.
    Human,

    /// The session running in the Conversation, through `verkstead transfer` —
    /// to a device the human ticked, or the one the work was drafted on.
    Session,
}

impl AskedBy {
    fn column(self) -> &'static str {
        match self {
            AskedBy::Human => "human",
            AskedBy::Session => "session",
        }
    }

    fn read(column: &str) -> AskedBy {
        match column {
            "session" => AskedBy::Session,
            _ => AskedBy::Human,
        }
    }
}

/// Ask for this Conversation to be moved onto `device` once whatever is running
/// has reached its end.
///
/// Nothing is moved and nothing is put on the Timeline. What the record holds
/// from here is that the work is going somewhere, which is what the Timeline
/// reads as *Transferring to* that machine and what keeps anything else from
/// being launched in the meantime.
///
/// Hands back the device the request it replaced named, where there was one: a
/// request that already named this device already has a mover behind it.
pub async fn ask_to_transfer(
    pool: &SqlitePool,
    conversation_id: i64,
    device: &str,
    by: AskedBy,
) -> Result<Option<String>> {
    let mut tx = super::writing(pool, "asking for a Conversation to be transferred").await?;

    let before: Option<(String,)> =
        sqlx::query_as("SELECT device FROM transfers WHERE conversation_id = ?")
            .bind(conversation_id)
            .fetch_optional(&mut *tx)
            .await
            .with_context(|| {
                format!("reading which device Conversation {conversation_id} was going to")
            })?;

    sqlx::query(
        "INSERT INTO transfers (conversation_id, device, asked_at, asked_by)
         VALUES (?, ?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), ?)
         ON CONFLICT (conversation_id) DO UPDATE SET device = excluded.device,
                                                     asked_at = excluded.asked_at,
                                                     asked_by = excluded.asked_by",
    )
    .bind(conversation_id)
    .bind(device)
    .bind(by.column())
    .execute(&mut *tx)
    .await
    .with_context(|| format!("asking for Conversation {conversation_id} to be transferred"))?;

    tx.commit()
        .await
        .context("asking for a Conversation to be transferred")?;

    Ok(before.map(|(device,)| device))
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

/// Who asked for the move this Conversation is on its way to, where one is.
///
/// What tells the mover whether the session it is seeing out is to be ended at
/// its turn's end — the session's own call — or left to end by itself, which is
/// what the human's press promises (ADR-0020, *The agent's call*). Read with the
/// device, because a request that has moved on to another machine is not the one
/// the asker was reading about.
pub async fn transfer_asked_by(
    pool: &SqlitePool,
    conversation_id: i64,
) -> Result<Option<(String, AskedBy)>> {
    let row: Option<(String, String)> =
        sqlx::query_as("SELECT device, asked_by FROM transfers WHERE conversation_id = ?")
            .bind(conversation_id)
            .fetch_optional(pool)
            .await
            .with_context(|| {
                format!("reading who asked for Conversation {conversation_id} to be transferred")
            })?;

    Ok(row.map(|(device, by)| (device, AskedBy::read(&by))))
}

/// Write the mark saying where the live record now is, and spend the request in
/// the same breath — or say that the request has moved on and write nothing
/// (ADR-0020, *Transfer*).
///
/// **A move's commit point, as one step.** Until the mark is written nothing has
/// changed on this device and the work is still being done here; once it is, the
/// far end is told the move is over and presses Resume for itself. So the mark is
/// the one write a mover may not make late.
///
/// **And late is what a second press makes it.** Pressing again is the human
/// changing their mind about where the work should go, and the request is
/// replaced rather than refused — see [`ask_to_transfer`]. The mover behind the
/// first press has minutes of packing and pushing in front of it, so by the time
/// it reaches this it may be carrying a copy to a machine the human has since
/// changed their mind about: reading the request and writing the mark as two
/// statements would let both movers write one, and two devices would each hold a
/// live copy of one piece of work with an agent starting in it.
///
/// So the read and the two writes are one transaction, and [`Marked::Superseded`]
/// is what a mover that has been overtaken is told. What it does about it is
/// sweep the copy it had just landed and stand down, leaving the request and the
/// Conversation to the mover that holds them — see the server's `transfers`.
pub async fn transfer_made(
    pool: &SqlitePool,
    conversation_id: i64,
    to: &super::Transferred,
) -> Result<Marked> {
    let mut tx = super::writing(pool, "recording where a Conversation's work went").await?;

    let asked: Option<(String, String)> =
        sqlx::query_as("SELECT device, asked_by FROM transfers WHERE conversation_id = ?")
            .bind(conversation_id)
            .fetch_optional(&mut *tx)
            .await
            .with_context(|| {
                format!("reading the transfer Conversation {conversation_id} is being moved by")
            })?;

    // A request naming another device, or none at all: either way the move this
    // mark would be about is not the move the record is asking for any more.
    let by = match asked {
        Some((device, by)) if device == to.device => AskedBy::read(&by),
        asked => {
            return Ok(Marked::Superseded {
                asked: asked.map(|(device, _)| device),
            });
        }
    };

    super::births::mark_away(&mut tx, conversation_id, to).await?;

    // And the request is spent, in the transaction that made the mark rather
    // than after it: what is left behind by one that is not is a second move
    // made at the next moment nothing is running.
    sqlx::query("DELETE FROM transfers WHERE conversation_id = ?")
        .bind(conversation_id)
        .execute(&mut *tx)
        .await
        .with_context(|| {
            format!("spending the transfer that moved Conversation {conversation_id}")
        })?;

    tx.commit()
        .await
        .context("recording where a Conversation's work went")?;

    Ok(Marked::Marked { by })
}

/// What became of writing a move's mark at its commit point.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Marked {
    /// The mark is written and the request is spent: the work is over there, and
    /// this copy is the tombstone. With who asked for the move, as the request
    /// stood when it was spent — which is what the Timeline names.
    Marked { by: AskedBy },

    /// The request names somewhere else now, so nothing was written: a second
    /// press landed while this move was in flight, and the mover that holds it
    /// is the one making the move.
    Superseded {
        /// The device the request names instead, or `None` where it has been
        /// taken away altogether — by the human, or by a mover that finished
        /// first.
        asked: Option<String>,
    },
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
