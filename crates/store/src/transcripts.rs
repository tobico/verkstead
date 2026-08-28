//! What a session said, as its own backend wrote it down: the Transcript, kept
//! line for line.
//!
//! A session's agent keeps a log of the conversation it is having — the prose it
//! wrote, the tools it called, what was put to it — and that log is the ground
//! truth of what happened, where the Capture beside it is only how the session
//! looked. Verkstead follows the log while the session runs (see the server's
//! `transcript` module) and keeps every line of it here.
//!
//! Lines are stored **verbatim** and parsed by whoever reads them, which is what
//! holds the coupling to a file format somebody else owns down to one place. A
//! change to that format can leave a line nothing knows how to render; it can
//! never lose what was said (ADR 0006).
//!
//! Rows keyed by the Event and a sequence number, appended and never rewritten —
//! the same shape as the Capture's chunks and for the same reason: a Transcript
//! grows for as long as its session runs, and rewriting one column would cost
//! more the longer the session had been talking.
//!
//! A session that leaves no log has no rows here, which is the ordinary case
//! rather than a fault: it is every stub agent the test suite runs, every
//! backend that keeps no such log, and every session Verkstead could not name.
//! What those sessions said is read back off the Capture.

use anyhow::{Context, Result};
use sqlx::SqlitePool;

/// The table the lines live in. Hangs off a Timeline Event, because one Event is
/// one session and a Transcript is one session's own record of itself.
pub(crate) async fn apply_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS transcript_lines (
             event_id INTEGER NOT NULL REFERENCES timeline_events(id),
             seq      INTEGER NOT NULL,
             line     TEXT NOT NULL,
             PRIMARY KEY (event_id, seq)
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the transcript_lines table")?;

    Ok(())
}

/// Add the lines a session has written since last time, in the order it wrote
/// them.
///
/// All of them in one transaction, so that a poll which caught several lines
/// puts them all on the Transcript or none of them: what the caller keeps on a
/// failure is the whole batch to try again, and a half-written batch would have
/// it write some of them twice.
///
/// The sequence numbers carry on from what is already there rather than being
/// counted by the caller, for the reason the Capture's are: what orders a
/// Transcript is the order its lines arrived.
pub async fn append_transcript(pool: &SqlitePool, event_id: i64, lines: &[String]) -> Result<()> {
    if lines.is_empty() {
        return Ok(());
    }

    let mut tx = super::writing(pool, "adding to a Transcript").await?;

    let (last,): (i64,) =
        sqlx::query_as("SELECT COALESCE(MAX(seq), 0) FROM transcript_lines WHERE event_id = ?")
            .bind(event_id)
            .fetch_one(&mut *tx)
            .await
            .with_context(|| format!("finding the end of the Transcript of Event {event_id}"))?;

    for (at, line) in lines.iter().enumerate() {
        sqlx::query("INSERT INTO transcript_lines (event_id, seq, line) VALUES (?, ?, ?)")
            .bind(event_id)
            .bind(last + at as i64 + 1)
            .bind(line)
            .execute(&mut *tx)
            .await
            .with_context(|| format!("adding to the Transcript of Event {event_id}"))?;
    }

    tx.commit().await.context("adding to a Transcript")?;

    Ok(())
}

/// One session's Transcript, every line of it in the order it was written.
///
/// `None` where that Conversation has no such Event, which is not the same
/// question as whether the session said anything: a session that kept no record
/// of itself and one that has not started talking yet both come back with no
/// lines at all, and what tells them apart is the Capture.
pub async fn transcript(
    pool: &SqlitePool,
    conversation_id: i64,
    event_id: i64,
) -> Result<Option<Vec<String>>> {
    transcript_after(pool, conversation_id, event_id, 0).await
}

/// The same Transcript from `after` onwards: the lines whose sequence number is
/// past it, which is everything written since a reader that had got that far
/// last looked.
///
/// What a running session's open pane reads on every batch, rather than the
/// whole record again — an hour of talking, twice a second, over the network
/// this tool is used across (ADR 0009).
///
/// `None` for the Event that is not this Conversation's, and for an `after`
/// this Transcript has never reached. The second is a reader holding a cursor
/// this record cannot answer from, which is a thing to say rather than an empty
/// answer to give: what the caller does about it is read the record whole,
/// which is always correct.
pub async fn transcript_after(
    pool: &SqlitePool,
    conversation_id: i64,
    event_id: i64,
    after: i64,
) -> Result<Option<Vec<String>>> {
    // Against the Timeline rather than against the lines, because there is no
    // row here until a session has said something — a Transcript with nothing on
    // it is an ordinary answer, and an Event on somebody else's Conversation is
    // not.
    let found: Option<(i64,)> =
        sqlx::query_as("SELECT id FROM timeline_events WHERE id = ? AND conversation_id = ?")
            .bind(event_id)
            .bind(conversation_id)
            .fetch_optional(pool)
            .await
            .with_context(|| format!("looking for the Transcript of Event {event_id}"))?;

    if found.is_none() {
        return Ok(None);
    }

    // The sequence numbers run from 1 without gaps — they are counted on from
    // whatever was there — so the last of them is how far this record goes, and
    // a cursor past it is one no reading of this record ever handed out.
    let (end,): (i64,) =
        sqlx::query_as("SELECT COALESCE(MAX(seq), 0) FROM transcript_lines WHERE event_id = ?")
            .bind(event_id)
            .fetch_one(pool)
            .await
            .with_context(|| format!("finding the end of the Transcript of Event {event_id}"))?;

    if after > end {
        return Ok(None);
    }

    let lines: Vec<(String,)> = sqlx::query_as(
        "SELECT line FROM transcript_lines WHERE event_id = ? AND seq > ? ORDER BY seq",
    )
    .bind(event_id)
    .bind(after)
    .fetch_all(pool)
    .await
    .with_context(|| format!("reading the Transcript of Event {event_id}"))?;

    Ok(Some(lines.into_iter().map(|(line,)| line).collect()))
}
