//! The Pauses a Verkstead of before put on a Timeline when an Agent Profile's
//! account ran out of window, and how each of those waits ended.
//!
//! Nothing writes one any more. An exhausted window stops a run the way
//! everything else does — one stop on the Conversation, one Notice, one Resume
//! — and what is left here is the reading half: a Pause Event written before
//! that is the record of what happened, and ADR-0006's rule is that the record
//! is kept and read rather than rewritten.
//!
//! So a stored Pause still says which account ran out, the line the session
//! printed, when the window was read to come back, and what ended the wait —
//! and a Timeline holding one still draws it. What it no longer says is
//! anything about *now*: whether a Conversation is stopped is the one stop's to
//! answer, and an open Pause found in a database written before this stage was
//! read onto its Conversation as one — see [`super::stops::apply_schema`].

use anyhow::{Context, Result, bail};
use sqlx::SqlitePool;

/// A Pause, whole: which account ran out, when it comes back, and what became of
/// the wait.
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
    /// printed carried no time this build could read as one.
    ///
    /// The half a Pause used to end itself on. What it says now is what it
    /// always read as: when the account comes back, for somebody looking at
    /// the record.
    pub resets_at: Option<String>,

    /// How the wait ended, or `None` while it is still on — which is the state
    /// the run is stopped in.
    pub resumed: Option<Resumed>,
}

/// How a Pause ended: what started the work again, and when.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resumed {
    pub by: By,

    /// When it ended, RFC 3339.
    pub at: String,
}

/// The two things that end a wait.
///
/// They meet in the same place — the Pause closes and the run goes on from where
/// it stopped — and the record keeps them apart, because *the window came back*
/// and *somebody decided not to wait for it* are different things to read on a
/// Timeline afterwards.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum By {
    /// The human said so, from the workbench or from their phone.
    Human,

    /// The reset time passed.
    Reset,
}

impl By {
    /// The word the column holds. Lowercase and spelled out, so a database
    /// opened by hand says something.
    fn stored(self) -> &'static str {
        match self {
            Self::Human => "human",
            Self::Reset => "reset",
        }
    }

    /// What a stored word names. One this does not know is a database written by
    /// a Verkstead this one does not understand, exactly as an unknown lifecycle
    /// state is.
    fn read(word: &str) -> Result<Self> {
        Ok(match word {
            "human" => Self::Human,
            "reset" => Self::Reset,
            other => bail!("a Pause says it was resumed by the unknown {other:?}"),
        })
    }
}

/// What became of ending one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resuming {
    /// Recorded: the run is free to go on.
    Resumed,

    /// This Conversation has no such Pause. An Event id belonging to another
    /// Conversation names nothing here.
    NoSuchPause,

    /// It had ended already — the human pressed twice, or the reset arrived
    /// while they were pressing. Not an error and not something to act on twice:
    /// the first ending stands.
    AlreadyResumed,
}

/// The pauses table. It hangs off a Timeline Event as a pull request does: a
/// Pause is one Event's full self, and the Event is what a Timeline holds.
///
/// Declared still, though nothing writes one: a database made this morning has
/// no Pause on any Timeline and the Timeline's own read joins this table all
/// the same, so the shape has to be there for the join to find nothing in.
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
    type Row = (
        String,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
    );

    let row: Option<Row> = sqlx::query_as(
        "SELECT profile, said, resets_at, resumed_by, resumed_at
         FROM pauses
         WHERE event_id = ? AND conversation_id = ?",
    )
    .bind(event_id)
    .bind(conversation_id)
    .fetch_optional(pool)
    .await
    .with_context(|| format!("reading the Pause of Event {event_id}"))?;

    let Some((profile, said, resets_at, resumed_by, resumed_at)) = row else {
        return Ok(None);
    };

    Ok(Some(read(
        profile, said, resets_at, resumed_by, resumed_at,
    )?))
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
    type Row = (
        i64,
        String,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
    );

    let rows: Vec<Row> = sqlx::query_as(
        "SELECT event_id, profile, said, resets_at, resumed_by, resumed_at
         FROM pauses
         WHERE conversation_id = ?",
    )
    .bind(conversation_id)
    .fetch_all(pool)
    .await
    .with_context(|| format!("reading the Pauses of Conversation {conversation_id}"))?;

    rows.into_iter()
        .map(
            |(event_id, profile, said, resets_at, resumed_by, resumed_at)| {
                Ok((
                    event_id,
                    read(profile, said, resets_at, resumed_by, resumed_at)?,
                ))
            },
        )
        .collect()
}

/// End one: the wait is over, and this is what ended it.
///
/// Recorded before anything acts on it, for the reason a stop is cleared before
/// anything is launched over it: a Pause acted on without being closed is one the
/// run could be started again from twice, and two launches is two agents in one
/// Worktree.
///
/// [`Resuming::AlreadyResumed`] is an ordinary outcome. The human presses from
/// whichever device is to hand, and the sweep may have got there first — a wait
/// that is over is over, whichever of the two ended it.
pub async fn resume_pause(
    pool: &SqlitePool,
    conversation_id: i64,
    event_id: i64,
    by: By,
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
         SET resumed_by = ?, resumed_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
         WHERE event_id = ?",
    )
    .bind(by.stored())
    .bind(event_id)
    .execute(&mut *tx)
    .await
    .with_context(|| format!("ending the Pause of Event {event_id}"))?;

    tx.commit().await.context("ending a Pause")?;

    Ok(Resuming::Resumed)
}

/// The word the `kind` column holds for a Pause.
///
/// A constant beside the Event's own spelling, because the row is written before
/// there is an Event to ask: [`record_pause`] inserts both in one transaction.
pub(crate) const PAUSE: &str = "pause";

/// A Pause out of the columns a row holds, or the row being a Verkstead this one
/// does not understand.
///
/// Shared by [`pause`] and the Timeline's own read, so that one place knows how
/// the two ending columns go together: they are written in one statement, and a
/// row with a reason and no time on it is a database somebody has been in by
/// hand.
fn read(
    profile: String,
    said: String,
    resets_at: Option<String>,
    resumed_by: Option<String>,
    resumed_at: Option<String>,
) -> Result<Pause> {
    let resumed = match resumed_by.zip(resumed_at) {
        Some((by, at)) => Some(Resumed {
            by: By::read(&by)?,
            at,
        }),
        None => None,
    };

    Ok(Pause {
        profile,
        said,
        resets_at,
        resumed,
    })
}
