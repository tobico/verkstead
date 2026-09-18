//! How each session ended: what its process exited with, how long it lived, and
//! whether it ever printed a byte — by the Timeline Event that session printed
//! into.
//!
//! Three facts the relay has in hand at the moment it reaps a child and nothing
//! else ever has again — the process is gone, and a restarted server has no
//! sessions at all. They are written down here because of where they are read:
//! a stop's Notice draws its evidence from what the session *said*, and a
//! session that said nothing leaves the human a block that reads *It said
//! nothing at all* — true, and pointing nowhere. An instant exit named as an
//! instant exit points at the binary. See the server's `stopping` module, which
//! is what puts it into a sentence.
//!
//! **And `printed` is what says the session is the one with nothing to show**,
//! rather than an empty Capture. A Capture is no longer the session's alone: on
//! the platform whose boundary is written, Verkstead opens one with two lines of
//! its own about the boundary before the agent has a terminal at all — see the
//! server's `sessions::verkstead_says` — so a Windows session that printed
//! nothing leaves a Capture that is not empty and evidence that is not its own.
//! The relay is the one thing that ever knows the difference, because it is what
//! reads the session's own bytes, and this is where it says so.
//!
//! **Beside the Event rather than threaded through the stop.** Every caller
//! that writes a stop already says which Event the last session was printing
//! into, and the evidence is read off that Event; a stop that had to be handed
//! how its session ended would be an argument added to every one of them for
//! the sake of the one case where there is nothing else to show.
//!
//! A table of its own rather than a column on the Capture, for the reason
//! [`super::session_names`] is one: there is no migration machinery here, and a
//! fact that arrived after the Capture did can be added beside it without one.
//! One row per Event, because one Event is one session.
//!
//! **And no row for a session Verkstead ended itself**, which is the one ending
//! that is not a session going wrong: its step had landed, or the human pressed
//! something, and how it exited says nothing about either. An Event with no row
//! is that, or a session from before any of this was written down, or one still
//! running — every reader shows all three the way it always showed a session
//! that said nothing.

use std::time::Duration;

use anyhow::{Context, Result};
use sqlx::SqlitePool;

/// How one session ended, as the relay that reaped it saw.
///
/// `Ended` rather than `Ending`, which is what everything else here calls an
/// outcome: [`Ending`](super::Ending) is what became of ending a follow-up, and
/// one word for two questions in one crate is a word that answers neither.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ended {
    /// What the process exited with.
    ///
    /// `None` where there was no code to read: a process something else killed
    /// exits by a signal, and a relay that could not reap one at all has
    /// nothing to say about how it went.
    pub code: Option<i64>,

    /// And how long it lived, from the moment it was spawned to the moment it
    /// was reaped.
    ///
    /// The whole of the session rather than the part of it that was working:
    /// what the number is for is telling an agent that ran and gave up from a
    /// launcher that was never going to run at all, and tenths of a second is
    /// the answer to that.
    pub lived: Duration,

    /// And whether the session itself ever printed a byte.
    ///
    /// The relay's own reading — see the server's `Idle::said_anything`, where
    /// every byte counts whatever the idle judgement makes of it. What it is for
    /// is telling the session's evidence from Verkstead's: a Capture may hold
    /// lines of Verkstead's own about the launch, so a session that printed
    /// nothing has nothing in its Capture worth showing the human however full
    /// that Capture looks.
    pub printed: bool,
}

/// The table the endings live in.
pub(crate) async fn apply_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS session_endings (
             event_id INTEGER PRIMARY KEY REFERENCES timeline_events(id),
             code     INTEGER,
             lived_ms INTEGER NOT NULL,
             printed  INTEGER NOT NULL
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the session_endings table")?;

    Ok(())
}

/// Write down how the session printing into `event_id` ended.
///
/// Milliseconds on the row and a [`Duration`] either side of it: what is stored
/// is an integer because the column is, and what is passed and read back is the
/// span itself, so nobody outside here has a unit to remember.
///
/// The first ending is the one that counts and a second is ignored rather than
/// refused, for the reason a delivery's is — see [`super::record_delivery`]. A
/// session ends once, and a row already there is this having been said twice
/// about the same ending rather than a second ending to record.
pub async fn end_session(
    pool: &SqlitePool,
    event_id: i64,
    code: Option<i32>,
    lived: Duration,
    printed: bool,
) -> Result<()> {
    sqlx::query(
        "INSERT OR IGNORE INTO session_endings (event_id, code, lived_ms, printed) \
         VALUES (?, ?, ?, ?)",
    )
    .bind(event_id)
    .bind(code.map(i64::from))
    .bind(i64::try_from(lived.as_millis()).unwrap_or(i64::MAX))
    .bind(printed)
    .execute(pool)
    .await
    .with_context(|| format!("recording how the session of Event {event_id} ended"))?;

    Ok(())
}

/// How the session that printed into `event_id` ended, or `None` where nothing
/// was written down about it.
///
/// The Conversation is part of the question rather than trusted from the path,
/// for the reason a Capture's reading takes one — see [`super::capture`]: an
/// Event id that belongs to another Conversation names nothing here.
pub async fn session_ending(
    pool: &SqlitePool,
    conversation_id: i64,
    event_id: i64,
) -> Result<Option<Ended>> {
    let found: Option<(Option<i64>, i64, bool)> = sqlx::query_as(
        "SELECT s.code, s.lived_ms, s.printed
         FROM session_endings s
         JOIN timeline_events e ON e.id = s.event_id
         WHERE s.event_id = ? AND e.conversation_id = ?",
    )
    .bind(event_id)
    .bind(conversation_id)
    .fetch_optional(pool)
    .await
    .with_context(|| format!("reading how the session of Event {event_id} ended"))?;

    Ok(found.map(|(code, lived_ms, printed)| Ended {
        code,
        lived: Duration::from_millis(lived_ms.unsigned_abs()),
        printed,
    }))
}
