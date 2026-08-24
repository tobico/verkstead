//! The Pauses a run waits out: an Agent Profile's account exhausting its window,
//! and how the wait ended.
//!
//! A Pause is a run that has stopped without anything having gone wrong. The
//! account it was spending is out of window, so nothing it launched would get
//! anywhere — and the agent itself is waiting for the same reset. What Verkstead
//! adds is that the wait is *said*: which Profile ran out, when the window comes
//! back where that could be read, and a press to start again.
//!
//! Shaped like a halt and deliberately, because it stops a run the same way: the
//! run does not advance past one, and the Conversation carries *blocked on you*
//! until it is closed. At most one open per Conversation, by the partial unique
//! index — two Pauses would be two things to answer about one wait.
//!
//! Where it differs is how it closes. A halt waits on the human alone; a Pause is
//! closed either by their press or by the reset time passing, and the row says
//! which — see [`Resumed`]. Neither reverts anything: the repository is left
//! exactly as the session left it, as it is after a halt.
//!
//! Nothing here recognises a limit, and nothing here acts on one. What the
//! wording is, and what starting the work again means, is the server's — see its
//! `limits` module.

use anyhow::{Context, Result, bail};
use sqlx::SqlitePool;

/// A Pause, whole: which account ran out, when it comes back, and what became of
/// the wait.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pause {
    /// What the Agent Profile whose account ran out is called, as it was called
    /// then.
    ///
    /// The name and not the id, for the reason a halt's evidence is gathered
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
    /// The half that makes a Pause end itself. Without it the wait is the
    /// human's to end, which is a whole answer rather than a lesser one: the
    /// Timeline says an account ran out, and one press starts the work again.
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

/// One Conversation's open Pause, for whatever is waiting the reset out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Waiting {
    pub conversation_id: i64,
    pub event_id: i64,

    /// When the window resets, RFC 3339, or `None` where none could be read —
    /// which is a wait only the human ends.
    pub resets_at: Option<String>,
}

/// The pauses table. It hangs off a Timeline Event as a pull request does: a
/// Pause is one Event's full self, and the Event is what a Timeline holds.
///
/// The Conversation is on the row as well as on the Event above it, because the
/// partial unique index needs it and SQLite cannot index a column that lives in
/// another table.
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

    // Partial, so that it constrains only the open ones: a long run against a
    // busy account collects a Pause a day, and exactly one of them is stopping
    // it.
    sqlx::query(
        "CREATE UNIQUE INDEX IF NOT EXISTS pauses_open
             ON pauses (conversation_id) WHERE resumed_at IS NULL",
    )
    .execute(pool)
    .await
    .context("indexing the open Pauses by their Conversation")?;

    Ok(())
}

/// Put a Pause on a Conversation's Timeline, and say which Event it became.
///
/// `None` is a Conversation that is paused already, or one that is not there:
/// neither is a failure. A session redraws the line that says its account ran out
/// for as long as it waits, so the second reading of it is the ordinary case
/// rather than the strange one — and the first Pause is the one the human is
/// being told about.
///
/// One transaction, because an Event without its row is a Timeline holding a
/// Pause that cannot say what it is waiting for.
pub async fn record_pause(
    pool: &SqlitePool,
    conversation_id: i64,
    profile: &str,
    said: &str,
    resets_at: Option<&str>,
) -> Result<Option<i64>> {
    let mut tx = pool.begin().await.context("pausing a run")?;

    // Asked inside the transaction, so the answer still holds when the insert
    // below acts on it. The partial unique index is what settles it either way.
    let open: Option<(i64,)> = sqlx::query_as(
        "SELECT event_id FROM pauses WHERE conversation_id = ? AND resumed_at IS NULL",
    )
    .bind(conversation_id)
    .fetch_optional(&mut *tx)
    .await
    .with_context(|| format!("looking for an open Pause on Conversation {conversation_id}"))?;

    if open.is_some() {
        return Ok(None);
    }

    // Selected from `conversations` rather than trusting the id, as every other
    // Event is written: a Pause attributed to a Conversation that is not there
    // would be one nobody could ever end.
    let event: Option<(i64,)> = sqlx::query_as(
        "INSERT INTO timeline_events (conversation_id, at, kind, body)
         SELECT id, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), ?, ''
         FROM conversations WHERE id = ?
         RETURNING id",
    )
    .bind(PAUSE)
    .bind(conversation_id)
    .fetch_optional(&mut *tx)
    .await
    .with_context(|| {
        format!("putting a Pause on the Timeline of Conversation {conversation_id}")
    })?;

    let Some((event_id,)) = event else {
        return Ok(None);
    };

    sqlx::query(
        "INSERT INTO pauses (event_id, conversation_id, profile, said, resets_at)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(event_id)
    .bind(conversation_id)
    .bind(profile)
    .bind(said)
    .bind(resets_at)
    .execute(&mut *tx)
    .await
    .with_context(|| format!("recording the Pause of Event {event_id}"))?;

    tx.commit().await.context("pausing a run")?;

    Ok(Some(event_id))
}

/// Which Event a Conversation's open Pause is, or `None` where nothing is waiting.
///
/// What the runner asks before it launches anything, beside the same question
/// about a halt: a run does not advance while an account is out of window, and
/// the one place that is decided is here.
pub async fn open_pause(pool: &SqlitePool, conversation_id: i64) -> Result<Option<i64>> {
    let row: Option<(i64,)> = sqlx::query_as(
        "SELECT event_id FROM pauses WHERE conversation_id = ? AND resumed_at IS NULL",
    )
    .bind(conversation_id)
    .fetch_optional(pool)
    .await
    .with_context(|| format!("reading the open Pause of Conversation {conversation_id}"))?;

    Ok(row.map(|(event_id,)| event_id))
}

/// Every Pause that is still waiting, across every Conversation.
///
/// What the sweep that ends a wait on its reset time reads. Everything at once
/// rather than a question per Conversation, and for the reason the stall sweep
/// reads the sessions register once: this is a look over the whole server, and
/// nearly always comes back with nothing.
///
/// Which is also what makes the reset survive a restart. Nothing holds a timer
/// across the process, and a Pause whose window came back while the server was
/// down is one the next sweep finds already due.
pub async fn waiting_pauses(pool: &SqlitePool) -> Result<Vec<Waiting>> {
    let rows: Vec<(i64, i64, Option<String>)> = sqlx::query_as(
        "SELECT conversation_id, event_id, resets_at
         FROM pauses WHERE resumed_at IS NULL
         ORDER BY event_id",
    )
    .fetch_all(pool)
    .await
    .context("reading the Pauses that are still waiting")?;

    Ok(rows
        .into_iter()
        .map(|(conversation_id, event_id, resets_at)| Waiting {
            conversation_id,
            event_id,
            resets_at,
        })
        .collect())
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
/// Recorded before anything acts on it, for the reason a halt is cleared before
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
