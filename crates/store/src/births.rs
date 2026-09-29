//! The **birth key** every Conversation carries, and the mark that says a copy
//! of one has been transferred away (ADR-0020, *Transfer*).
//!
//! A Conversation can be moved from the device it was drafted on to another one,
//! and what crosses is a copy: the record's slice is written into the far end's
//! own tables under ids that device issued, and the source keeps what it had. So
//! a piece of work is one thing with several rows in several databases, and
//! nothing about a row says which of them the others are — ids are each
//! device's own and collide by construction.
//!
//! **The birth key is what says it.** The Device Id of the machine the
//! Conversation was drafted on, and the id it was given there: stamped once, at
//! creation, and never touched again, so a Conversation that has moved twice
//! still answers to the key it was born under. Every copy of one piece of work
//! carries the same key, which is the whole of what makes the **Merged List**
//! able to draw the work once however many machines hold a copy of it — see
//! `server::merging`.
//!
//! **And the mark beside it says which copy is not the live one.** A device that
//! has handed its Conversation on keeps its copy as a tombstone: the mark names
//! the device that has the live record and the id it goes by over there, so old
//! links still land on the work and a transfer back has somewhere to put itself.
//! Nothing writes to a Conversation wearing one.
//!
//! **Two tables beside `conversations` rather than columns on it**, which is
//! what every fact about a Conversation that is not one string has been: that
//! table is STRICT and there is no migration machinery below it, so the
//! worktree, the direction, the branch a stage stands on and the rest all arrive
//! this way — see `conversations::apply_schema`. A birth key is a device and an
//! id and the mark is a device and an id, so neither is one string, and neither
//! is a column.
//!
//! **A row with no birth key is a database written before there was one.** The
//! key names a device and the identity is read out of the very pool the open
//! runs on, so nothing an open can do will fill it — exactly as a Rank cannot be
//! minted there. What fills it is [`super::stamp_the_births`], at the first
//! start that has this device's id in hand and before any route is answered.

use anyhow::{Context, Result};
use sqlx::SqlitePool;

/// The key a Conversation was born under: the device it was drafted on, and the
/// id that device gave it.
///
/// Cluster-wide, unlike everything else that names a Conversation: the pair is
/// unique across every database in a cluster, because a device numbers its own
/// rows and no two devices share an id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Birth {
    /// The **Device Id** of the machine it was drafted on.
    pub device: String,

    /// And the id it was given there, which is its local id on that machine and
    /// on no other.
    pub id: i64,
}

impl Birth {
    /// The pair as one string, which is what a merged list tells two rows apart
    /// by — see [`super::ConversationRow::born`].
    ///
    /// Opaque wherever it is compared, exactly as a Rank is: the two halves are
    /// read off the columns, and this is only ever looked at for equality. The
    /// separator is the one `reaching.ts` writes a row key with, there being no
    /// reason for the viewer and this to spell the same pair two ways.
    pub fn key(&self) -> String {
        format!("{}/{}", self.device, self.id)
    }
}

/// Where the live record of a transferred Conversation is: the device that has
/// it, and the id it goes by there.
///
/// The same two facts a [`Birth`] holds and a different sentence: that one says
/// where the work started, and this one says where it is now. A Conversation
/// with no mark is one whose own row is the live one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transferred {
    /// The **Device Id** of the machine holding the live record.
    pub device: String,

    /// And the id it numbered its copy, which is what the redirect off this
    /// copy's URL is built from.
    pub id: i64,
}

/// The two tables: the key every Conversation is born with, and the mark a copy
/// that has been handed on wears.
///
/// Both keyed by the Conversation, one row apiece at most. A Conversation with
/// no birth key is one written before there were any — see this module's own
/// docs — and one with no mark is a live record, which is nearly all of them.
pub(crate) async fn apply_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS births (
             conversation_id INTEGER PRIMARY KEY REFERENCES conversations(id),
             device          TEXT NOT NULL,
             born_as         INTEGER NOT NULL
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the births table")?;

    // Indexed by the key itself, because that is the other way it is read: a
    // copy arriving asks whether this database already holds the Conversation it
    // is a copy of, which is a lookup by the pair rather than by the row.
    sqlx::query("CREATE INDEX IF NOT EXISTS births_key ON births (device, born_as)")
        .execute(pool)
        .await
        .context("indexing the Conversations by the key they were born under")?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS transferred (
             conversation_id INTEGER PRIMARY KEY REFERENCES conversations(id),
             device          TEXT NOT NULL,
             live_as         INTEGER NOT NULL
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the transferred table")?;

    Ok(())
}

/// Stamp a Conversation with the key it was born under, inside the transaction
/// that made it.
///
/// Called by [`super::conversations`]'s one create, with this device's id and
/// the id the insert just issued — which is what a Conversation that has never
/// moved has and what every backfilled row is given too.
pub(crate) async fn stamp(
    tx: &mut sqlx::SqliteConnection,
    conversation_id: i64,
    born: &Birth,
) -> Result<()> {
    sqlx::query("INSERT INTO births (conversation_id, device, born_as) VALUES (?, ?, ?)")
        .bind(conversation_id)
        .bind(&born.device)
        .bind(born.id)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("stamping Conversation {conversation_id} with its birth key"))?;

    Ok(())
}

/// Write the key a copy that came from another device was born under, over the
/// one this device's own create stamped on the row it made for it.
///
/// **The key travels with the work**, which is the whole of why it is
/// cluster-wide: every copy of one piece of work says the same thing about where
/// it started, so a Conversation that has moved twice still answers to the key it
/// was born under. The row is the arriving device's own and the id on it is that
/// device's own; the key is neither of those and is not this device's to invent.
///
/// So it replaces rather than refuses. Making a Conversation stamps it — see
/// [`stamp`], which is the one thing a create does about this — and a copy is
/// made by the same create as anything else. What *cannot* change is the key of a
/// piece of work, and nothing here does: what this writes is the key that came
/// with it.
pub async fn record_birth(pool: &SqlitePool, conversation_id: i64, born: &Birth) -> Result<()> {
    sqlx::query(
        "INSERT INTO births (conversation_id, device, born_as)
         VALUES (?, ?, ?)
         ON CONFLICT (conversation_id) DO UPDATE SET device = excluded.device,
                                                     born_as = excluded.born_as",
    )
    .bind(conversation_id)
    .bind(&born.device)
    .bind(born.id)
    .execute(pool)
    .await
    .with_context(|| {
        format!("writing the key Conversation {conversation_id} came here having been born under")
    })?;

    Ok(())
}

/// The key a Conversation was born under, or nothing where it has none — a
/// database the backfill has not reached.
pub async fn birth(pool: &SqlitePool, conversation_id: i64) -> Result<Option<Birth>> {
    let row: Option<(String, i64)> =
        sqlx::query_as("SELECT device, born_as FROM births WHERE conversation_id = ?")
            .bind(conversation_id)
            .fetch_optional(pool)
            .await
            .with_context(|| {
                format!("reading the key Conversation {conversation_id} was born under")
            })?;

    Ok(row.map(|(device, id)| Birth { device, id }))
}

/// Mark a Conversation transferred away: the live record is on `to`'s device,
/// under the id it numbered its copy.
///
/// **The copy stays**, which is what the mark is for. It holds the id so that
/// every link anybody kept still leads to the work, and so that a transfer back
/// has a row to replace rather than a new one to invent — see the stage brief.
///
/// Written again where a mark is already there, because a Conversation can be
/// handed on more than once: what it says is where the live record is *now*, and
/// the copy that answered a moment ago may have passed it on since.
pub async fn transfer_away(
    pool: &SqlitePool,
    conversation_id: i64,
    to: &Transferred,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO transferred (conversation_id, device, live_as)
         VALUES (?, ?, ?)
         ON CONFLICT (conversation_id) DO UPDATE SET device = excluded.device,
                                                     live_as = excluded.live_as",
    )
    .bind(conversation_id)
    .bind(&to.device)
    .bind(to.id)
    .execute(pool)
    .await
    .with_context(|| format!("marking Conversation {conversation_id} transferred away"))?;

    Ok(())
}

/// Where the live record of a Conversation is, where this copy is not it.
///
/// `None` is the ordinary Conversation: this database's row is the record, and
/// there is nothing to redirect anybody to.
pub async fn transferred(pool: &SqlitePool, conversation_id: i64) -> Result<Option<Transferred>> {
    let row: Option<(String, i64)> =
        sqlx::query_as("SELECT device, live_as FROM transferred WHERE conversation_id = ?")
            .bind(conversation_id)
            .fetch_optional(pool)
            .await
            .with_context(|| {
                format!("reading where the live record of Conversation {conversation_id} is")
            })?;

    Ok(row.map(|(device, id)| Transferred { device, id }))
}
