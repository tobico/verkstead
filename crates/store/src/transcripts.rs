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

    let mut tx = pool.begin().await.context("adding to a Transcript")?;

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
/// Empty where the session left no log, which is not the same question as
/// whether the Event exists: a session that kept no record of itself and one
/// that has not started talking yet both say nothing here, and what tells them
/// apart is the Capture.
pub async fn transcript(pool: &SqlitePool, event_id: i64) -> Result<Vec<String>> {
    let lines: Vec<(String,)> =
        sqlx::query_as("SELECT line FROM transcript_lines WHERE event_id = ? ORDER BY seq")
            .bind(event_id)
            .fetch_all(pool)
            .await
            .with_context(|| format!("reading the Transcript of Event {event_id}"))?;

    Ok(lines.into_iter().map(|(line,)| line).collect())
}
