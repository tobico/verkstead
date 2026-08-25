//! The Pauses a Verkstead of before put on a Timeline when an Agent Profile's
//! account ran out of window, and when each of those waits ended.
//!
//! Nothing writes one any more. An exhausted window stops a run the way
//! everything else does — one stop on the Conversation, one Notice, one Resume
//! — and what is left here is the reading half: a Pause Event written before
//! that is the record of what happened, and ADR-0006's rule is that the record
//! is kept and read rather than rewritten.
//!
//! So a stored Pause still says which account ran out, the line the session
//! printed, when the window was read to come back, and whether the wait is over
//! — and a Timeline holding one still draws it. What it no longer says is
//! anything about *now*: whether a Conversation is stopped is the one stop's to
//! answer, and an open Pause found in a database written before this stage was
//! read onto its Conversation as one — see [`super::stops::apply_schema`].
//!
//! And no longer *what* ended the wait. A Verkstead of before had two answers —
//! the human pressing, and the reset time passing — and there is one left: no
//! stop resumes itself, so every wait that ends ends by a press. A row written
//! before this stage keeps the word it was written with; nothing reads it.

use anyhow::{Context, Result};
use sqlx::SqlitePool;

/// A Pause, whole: which account ran out, when it comes back, and whether the
/// wait is over.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pause {
    /// What the Agent Profile whose account ran out is called, as it was called
    /// then.
    ///
    /// The name and not the id, for the reason a stop's evidence is gathered
    /// rather than fetched: a Profile can be renamed or deleted, and a
    /// Pause that could no longer say which account it was would be a wait
    /// nobody could account for.
    pub profile: String,

    /// The line that said so, as the session printed it.
    ///
    /// Kept because the wording is the backend's and will move: a Pause raised
    /// by a sentence this build recognised is answerable a year later by
    /// somebody reading the sentence itself.
    pub said: String,

    /// When the window resets, RFC 3339 — or `None` where what the session
    /// printed carried no time the Verkstead that wrote this could read as one.
    ///
    /// The half a Pause used to end itself on. What it says now is what it
    /// always read as: when the account comes back, for somebody looking at
    /// the record.
    pub resets_at: Option<String>,

    /// When the wait ended, RFC 3339 — or `None` while it is still on, which is
    /// the state the run is stopped in.
    pub resumed: Option<String>,
}

/// What became of ending one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resuming {
    /// Recorded: the run is free to go on.
    Resumed,

    /// This Conversation has no such Pause. An Event id belonging to another
    /// Conversation names nothing here.
    NoSuchPause,

    /// It had ended already — the human pressed twice, from two devices. Not an
    /// error and not something to act on twice: the first ending stands.
    AlreadyResumed,
}

/// The pauses table. It hangs off a Timeline Event as a pull request does: a
/// Pause is one Event's full self, and the Event is what a Timeline holds.
///
/// Declared still, though nothing writes one: a database made this morning has
/// no Pause on any Timeline and the Timeline's own read joins this table all
/// the same, so the shape has to be there for the join to find nothing in.
///
/// `resumed_by` with it, though nothing reads it either. It is a column of rows
/// written before this stage, and dropping it would be rewriting the record.
pub(crate) async fn apply_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS pauses (
             event_id        INTEGER PRIMARY KEY REFERENCES timeline_events(id),
             conversation_id INTEGER NOT NULL REFERENCES conversations(id),
             profile         TEXT NOT NULL,
             said            TEXT NOT NULL,
             resets_at       TEXT,
             resumed_by      TEXT,
             resumed_at      TEXT
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the pauses table")?;

    Ok(())
}

/// The Pause one of a Conversation's Events is, or `None` where that Conversation
/// has no such Event.
///
/// The Conversation is part of the question rather than trusted from the path: a
/// Pause is reached through the Timeline it is on.
pub async fn pause(
    pool: &SqlitePool,
    conversation_id: i64,
    event_id: i64,
) -> Result<Option<Pause>> {
    type Row = (String, String, Option<String>, Option<String>);

    let row: Option<Row> = sqlx::query_as(
        "SELECT profile, said, resets_at, resumed_at
         FROM pauses
         WHERE event_id = ? AND conversation_id = ?",
    )
    .bind(event_id)
    .bind(conversation_id)
    .fetch_optional(pool)
    .await
    .with_context(|| format!("reading the Pause of Event {event_id}"))?;

    Ok(row.map(|(profile, said, resets_at, resumed)| Pause {
        profile,
        said,
        resets_at,
        resumed,
    }))
}

/// Every Pause on a Conversation's Timeline, by the Event each one is.
///
/// A read of its own rather than a join, for the arithmetic: the Timeline's
/// query is already at the sixteen positions a tuple can be read back as. It costs little where the join would not have —
/// most Conversations answer this with nothing.
pub(crate) async fn on_timeline(
    pool: &SqlitePool,
    conversation_id: i64,
) -> Result<std::collections::HashMap<i64, Pause>> {
    type Row = (i64, String, String, Option<String>, Option<String>);

    let rows: Vec<Row> = sqlx::query_as(
        "SELECT event_id, profile, said, resets_at, resumed_at
         FROM pauses
         WHERE conversation_id = ?",
    )
    .bind(conversation_id)
    .fetch_all(pool)
    .await
    .with_context(|| format!("reading the Pauses of Conversation {conversation_id}"))?;

    Ok(rows
        .into_iter()
        .map(|(event_id, profile, said, resets_at, resumed)| {
            (
                event_id,
                Pause {
                    profile,
                    said,
                    resets_at,
                    resumed,
                },
            )
        })
        .collect())
}

/// End one: the wait is over, which now has one way of happening.
///
/// Recorded before anything acts on it, for the reason a stop is cleared before
/// anything is launched over it: a Pause acted on without being closed is one the
/// run could be started again from twice, and two launches is two agents in one
/// Worktree.
///
/// [`Resuming::AlreadyResumed`] is an ordinary outcome. The human presses from
/// whichever device is to hand, and a press that arrives second is the same
/// press: a wait that is over is over.
pub async fn resume_pause(
    pool: &SqlitePool,
    conversation_id: i64,
    event_id: i64,
) -> Result<Resuming> {
    let mut tx = pool.begin().await.context("ending a Pause")?;

    let found: Option<(Option<String>,)> =
        sqlx::query_as("SELECT resumed_at FROM pauses WHERE event_id = ? AND conversation_id = ?")
            .bind(event_id)
            .bind(conversation_id)
            .fetch_optional(&mut *tx)
            .await
            .with_context(|| format!("looking for the Pause of Event {event_id}"))?;

    let Some((resumed,)) = found else {
        return Ok(Resuming::NoSuchPause);
    };

    if resumed.is_some() {
        return Ok(Resuming::AlreadyResumed);
    }

    sqlx::query(
        "UPDATE pauses
         SET resumed_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
         WHERE event_id = ?",
    )
    .bind(event_id)
    .execute(&mut *tx)
    .await
    .with_context(|| format!("ending the Pause of Event {event_id}"))?;

    tx.commit().await.context("ending a Pause")?;

    Ok(Resuming::Resumed)
}

/// The word the `kind` column holds for a Pause.
///
/// A constant beside the Event's own spelling, because a Timeline read has to
/// know the word without an Event in hand to ask.
pub(crate) const PAUSE: &str = "pause";
