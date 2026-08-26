//! The Pauses a Verkstead of before put on a Timeline when an Agent Profile's
//! account ran out of window.
//!
//! Nothing writes one any more. An exhausted window stops a run the way
//! everything else does — one stop on the Conversation, one Notice, one Resume
//! — and what is left here is the reading half: a Pause Event written before
//! that is the record of what happened, and ADR-0006's rule is that the record
//! is kept and read rather than rewritten.
//!
//! So a stored Pause still says which account ran out and the line the session
//! printed, and a Timeline holding one still draws it — as the sentence every
//! other stop is said in, there being one kind of stopped thing to read. What it
//! no longer says is anything about *now*: whether a Conversation is stopped is
//! the one stop's to answer, and an open Pause found in a database written
//! before this stage was read onto its Conversation as one, reset words and all
//! — see [`super::stops::apply_schema`].
//!
//! Which is what leaves three of the columns unread. When the window came back
//! is on the stop now, and it is drawn there, beside the press; and whether the
//! wait ended, and what ended it, was a card's to say — that card has gone, and
//! a wait that is over is over. Not one of them is dropped: rewriting the record
//! is the one thing this module does not do.

use anyhow::{Context, Result};
use sqlx::SqlitePool;

/// A Pause as a Timeline draws one: which account ran out, and the line that
/// said so.
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
}

/// The pauses table. It hangs off a Timeline Event as a pull request does: a
/// Pause is one Event's full self, and the Event is what a Timeline holds.
///
/// Declared still, though nothing writes one: a database made this morning has
/// no Pause on any Timeline and the Timeline's own read joins this table all
/// the same, so the shape has to be there for the join to find nothing in.
///
/// `resets_at`, `resumed_by` and `resumed_at` with it, though nothing reads any
/// of the three. They are columns of rows written before this stage, and
/// dropping them would be rewriting the record.
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

/// Every Pause on a Conversation's Timeline, by the Event each one is.
///
/// A read of its own rather than a join, for the arithmetic: the Timeline's
/// query is already at the sixteen positions a tuple can be read back as. It costs little where the join would not have —
/// most Conversations answer this with nothing.
pub(crate) async fn on_timeline(
    pool: &SqlitePool,
    conversation_id: i64,
) -> Result<std::collections::HashMap<i64, Pause>> {
    type Row = (i64, String, String);

    let rows: Vec<Row> = sqlx::query_as(
        "SELECT event_id, profile, said
         FROM pauses
         WHERE conversation_id = ?",
    )
    .bind(conversation_id)
    .fetch_all(pool)
    .await
    .with_context(|| format!("reading the Pauses of Conversation {conversation_id}"))?;

    Ok(rows
        .into_iter()
        .map(|(event_id, profile, said)| (event_id, Pause { profile, said }))
        .collect())
}

/// The word the `kind` column holds for a Pause.
///
/// A constant beside the Event's own spelling, because a Timeline read has to
/// know the word without an Event in hand to ask.
pub(crate) const PAUSE: &str = "pause";
