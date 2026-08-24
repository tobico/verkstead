//! Where driving stopped: when it stopped, whether anybody chose it, and the
//! Notice on the Timeline that says what happened.
//!
//! A halt is durable state on a Conversation rather than something that
//! happened — the thing that happened is the Notice, which is an ordinary
//! Timeline Event and stays on the record for ever. What is kept here is the
//! one fact the Notice cannot carry: that the Conversation is stopped *now*,
//! which is what the *blocked on you* badge is drawn from and what says whether
//! anything ought to be driving it.
//!
//! At most one per Conversation, by the primary key. A Conversation that is
//! stopped is stopped once: a second halt raised against one already halted is
//! the same stop noticed twice — a sweep looking again, or two watchers finding
//! the same dead session — and the first Notice is the one that explains it.
//!
//! Cleared when driving starts again — see [`clear_halt`], which is what Resume
//! presses. Nothing here starts anything: what a halt *means* is the server's,
//! and what this holds is only that there is one.
//!
//! Beside it, the stop that has not landed yet: the human pressed **Stop** while
//! a session was still running, so the run halts once that session has reached
//! its own end rather than now — see [`ask_to_stop`]. Durable for the reason the
//! halt is. A Conversation the human asked to stop is one that stays stopped,
//! and a server restarted in the gap that read nothing here would take it up
//! again as though nobody had asked.

use anyhow::{Context, Result, bail};
use sqlx::SqlitePool;

use super::conversations::Event;

/// Whether anybody chose to stop.
///
/// The one thing a restart has to know about a halt. What Verkstead pulled the
/// brake on, or the human asked to stop, stays stopped until somebody says
/// otherwise; what a crash took away is a Conversation nobody decided anything
/// about, and starting it again is putting things back rather than overriding a
/// decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Halt {
    /// Verkstead pulled the brake, or the human asked it to stop.
    Deliberate,

    /// Nothing chose anything: a restart or a crash took the driver away.
    Circumstance,
}

impl Halt {
    /// The word the column holds. Lowercase and spelled out, so a database
    /// opened by hand says something.
    fn stored(self) -> &'static str {
        match self {
            Self::Deliberate => "deliberate",
            Self::Circumstance => "circumstance",
        }
    }

    /// The one a stored word names. An unknown word is a database written by a
    /// Verkstead this one does not understand, exactly as an unknown lifecycle
    /// state is.
    fn read(word: &str) -> Result<Self> {
        Ok(match word {
            "deliberate" => Self::Deliberate,
            "circumstance" => Self::Circumstance,
            other => bail!("a Conversation is halted for the unknown reason {other:?}"),
        })
    }
}

/// A halt, whole: what kind of stop it is, when it happened, and which Event
/// explains it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Halted {
    pub halt: Halt,

    /// The Notice saying what stopped, why, and what the evidence was. Where
    /// the *blocked on you* badge points, a badge with nowhere to go being no
    /// use to anybody.
    pub event_id: i64,

    /// When it stopped, RFC 3339.
    pub at: String,
}

/// The halts table, and the asked-for stops that have yet to become one. Both
/// hang off a Conversation rather than off the Notice below it, unlike nearly
/// every other row here, and that is the point: a Notice is something that
/// happened and a halt is how things *are*, so one Conversation has any number
/// of the first and at most one of the second.
pub(crate) async fn apply_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS halts (
             conversation_id INTEGER PRIMARY KEY REFERENCES conversations(id),
             at              TEXT NOT NULL,
             halt            TEXT NOT NULL,
             event_id        INTEGER NOT NULL REFERENCES timeline_events(id)
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the halts table")?;

    // One asked-for stop per Conversation, by the primary key, for the reason
    // there is one halt: a second press is the first one arriving again, and
    // what it asks for has not changed.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS stops_asked (
             conversation_id INTEGER PRIMARY KEY REFERENCES conversations(id),
             at              TEXT NOT NULL
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the table of stops asked for")?;

    Ok(())
}

/// Stop driving a Conversation, and put the Notice saying why on its Timeline.
///
/// The Event the Notice became, or `None` where nothing was written: a
/// Conversation already halted, or one that is not there. Neither is a failure.
/// A run stops once, so a second halt against a stopped Conversation is the
/// same stop arriving twice — the sweep looking again a minute later is exactly
/// that — and the first Notice is the one the human reads.
///
/// One transaction, because a halt without its Notice is a badge pointing at
/// nothing, and a Notice without its halt is a Conversation that says it
/// stopped and does not know it.
pub async fn halt(
    pool: &SqlitePool,
    conversation_id: i64,
    halt: Halt,
    markdown: &str,
) -> Result<Option<i64>> {
    let mut tx = pool.begin().await.context("halting a Conversation")?;

    // Asked inside the transaction, so the answer still holds when the insert
    // below acts on it. The primary key is what settles it either way.
    let already: Option<(i64,)> =
        sqlx::query_as("SELECT event_id FROM halts WHERE conversation_id = ?")
            .bind(conversation_id)
            .fetch_optional(&mut *tx)
            .await
            .with_context(|| {
                format!("asking whether Conversation {conversation_id} had stopped")
            })?;

    if already.is_some() {
        return Ok(None);
    }

    let event = Event::Notice(markdown.to_owned());

    // Selected from `conversations` rather than trusting the id, as every other
    // Event is written: SQLite enforces a foreign key only when asked to, and a
    // halt attributed to a Conversation that is not there would be one nobody
    // could ever start again.
    let written: Option<(i64,)> = sqlx::query_as(
        "INSERT INTO timeline_events (conversation_id, at, kind, body)
         SELECT id, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), ?, ?
         FROM conversations WHERE id = ?
         RETURNING id",
    )
    .bind(event.kind())
    .bind(markdown)
    .bind(conversation_id)
    .fetch_optional(&mut *tx)
    .await
    .with_context(|| {
        format!("putting the Notice of a halt on the Timeline of Conversation {conversation_id}")
    })?;

    let Some((event_id,)) = written else {
        return Ok(None);
    };

    sqlx::query(
        "INSERT INTO halts (conversation_id, at, halt, event_id)
         VALUES (?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), ?, ?)",
    )
    .bind(conversation_id)
    .bind(halt.stored())
    .bind(event_id)
    .execute(&mut *tx)
    .await
    .with_context(|| format!("recording that Conversation {conversation_id} has stopped"))?;

    tx.commit().await.context("halting a Conversation")?;

    Ok(Some(event_id))
}

/// Whether a Conversation is halted, and what the halt is.
///
/// `None` is a Conversation nothing has stopped — which is every one that is
/// being driven, and every one nothing is supposed to be driving.
pub async fn halted(pool: &SqlitePool, conversation_id: i64) -> Result<Option<Halted>> {
    let row: Option<(String, String, i64)> =
        sqlx::query_as("SELECT halt, at, event_id FROM halts WHERE conversation_id = ?")
            .bind(conversation_id)
            .fetch_optional(pool)
            .await
            .with_context(|| format!("reading whether Conversation {conversation_id} is halted"))?;

    let Some((halt, at, event_id)) = row else {
        return Ok(None);
    };

    Ok(Some(Halted {
        halt: Halt::read(&halt)?,
        event_id,
        at,
    }))
}

/// Ask for the run to stop once whatever is running now has reached its end.
///
/// What **Stop** records where a session is still going. Nothing is ended and
/// nothing is put on the Timeline: the halt and its Notice come later, as the
/// run is about to launch the next thing — see the server's `stops` module.
///
/// Nothing happens twice. A second press is the first one arriving again, and
/// the Conversation is stopping either way.
pub async fn ask_to_stop(pool: &SqlitePool, conversation_id: i64) -> Result<()> {
    // Selected from `conversations` rather than trusting the id, as every halt
    // is: a stop asked for on a Conversation that is not there is one nothing
    // would ever act on.
    sqlx::query(
        "INSERT OR IGNORE INTO stops_asked (conversation_id, at)
         SELECT id, strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
         FROM conversations WHERE id = ?",
    )
    .bind(conversation_id)
    .execute(pool)
    .await
    .with_context(|| format!("asking Conversation {conversation_id} to stop"))?;

    Ok(())
}

/// Whether the human has asked this Conversation to stop and it has not stopped
/// yet.
///
/// Asked in front of every launch a run makes, which is where a stop asked for
/// becomes a halt.
pub async fn asked_to_stop(pool: &SqlitePool, conversation_id: i64) -> Result<bool> {
    let row: Option<(i64,)> =
        sqlx::query_as("SELECT conversation_id FROM stops_asked WHERE conversation_id = ?")
            .bind(conversation_id)
            .fetch_optional(pool)
            .await
            .with_context(|| {
                format!("reading whether Conversation {conversation_id} was asked to stop")
            })?;

    Ok(row.is_some())
}

/// Take an asked-for stop away: it has become a halt, or Resume has overtaken
/// it.
///
/// Nothing to do where none was asked for, which is every Conversation nobody
/// has pressed Stop on.
pub async fn forget_stop(pool: &SqlitePool, conversation_id: i64) -> Result<()> {
    sqlx::query("DELETE FROM stops_asked WHERE conversation_id = ?")
        .bind(conversation_id)
        .execute(pool)
        .await
        .with_context(|| {
            format!("forgetting the stop asked for on Conversation {conversation_id}")
        })?;

    Ok(())
}

/// Take the halt away, which is what starting to drive again does.
///
/// The Notice stays where it is: it is a record of a stop that really happened,
/// and a Timeline that took yesterday's back would be one nobody could read.
/// What goes is only the state — the badge, and the reason a restart would
/// leave the Conversation alone.
///
/// Nothing to do where there was no halt, which is the ordinary case for a
/// Conversation being driven perfectly well.
pub async fn clear_halt(pool: &SqlitePool, conversation_id: i64) -> Result<()> {
    sqlx::query("DELETE FROM halts WHERE conversation_id = ?")
        .bind(conversation_id)
        .execute(pool)
        .await
        .with_context(|| format!("starting to drive Conversation {conversation_id} again"))?;

    Ok(())
}
