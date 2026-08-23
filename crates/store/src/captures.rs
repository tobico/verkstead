//! What a session printed: the Capture kept whole, and the summary its Timeline
//! Event is read by.
//!
//! Two tables rather than one more Event body, and the reason is the shape of
//! the writing. A Capture grows for as long as its session runs, a chunk at a
//! time — and `UPDATE … SET body = body || ?` rewrites every byte already there
//! on each of those, so an hour-long session would cost more the longer it had
//! been talking. Chunks are inserted instead, and read back in the order they
//! arrived.
//!
//! The summary sits beside them because of the shape of the reading. Every open
//! page reads the whole Timeline, and working out a line count and a last
//! statement would mean reading every Capture and every Transcript of every
//! session to draw two lines of text. What writes it is the relay that writes
//! the chunks — see the server's `capture` module — which is following both the
//! output and the session's own log anyway, and can say either a flush at a
//! time.
//!
//! Nothing here says whether a session is still running. A running session is a
//! process, and a table cannot hold one: what a restarted server has is the
//! Capture of a session that is over, which is exactly what these rows say.

use std::collections::HashMap;

use anyhow::{Context, Result};
use sqlx::SqlitePool;

use super::conversations::Event;

/// A Capture as the Timeline shows it: how much of it there is, and the last
/// thing the session said.
///
/// The design's summary for an agent-output Event, held rather than derived —
/// see the module's own documentation for why.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Summary {
    /// How many lines the session has printed.
    pub lines: i64,

    /// How many turns its conversation has taken, counted as the Transcript
    /// pane draws them: the agent's prose, its thinking, a tool it called, what
    /// the tool said back, a turn put to it. The backend's own bookkeeping is
    /// not one of them.
    ///
    /// `None` where there is no log to count off — every stub agent, every
    /// backend that keeps none, and every session Verkstead could not name.
    /// Which is not the same as none counted yet: a session with no Transcript
    /// says nothing here rather than saying zero, and the row shows no metric
    /// at all.
    pub turns: Option<i64>,

    /// The last thing the agent said, off the log it keeps of its own
    /// conversation — or, where it keeps none, the last line it printed with
    /// the terminal's control sequences taken out. Empty where the session has
    /// said nothing yet.
    pub latest: String,
}

/// The Capture tables. Each hangs off a Timeline Event: a Capture is one Event's
/// full self, and the Event is what a Conversation's Timeline holds.
pub(crate) async fn apply_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS captures (
             event_id INTEGER PRIMARY KEY REFERENCES timeline_events(id),
             lines    INTEGER NOT NULL,
             latest   TEXT NOT NULL
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the captures table")?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS capture_chunks (
             event_id INTEGER NOT NULL REFERENCES timeline_events(id),
             seq      INTEGER NOT NULL,
             text     TEXT NOT NULL,
             PRIMARY KEY (event_id, seq)
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the capture_chunks table")?;

    // How many turns the session's conversation has taken, in a table of its
    // own rather than a column on `captures`: there is no migration machinery
    // here and that table is STRICT and left alone.
    //
    // Which turns out to be the shape the count wants anyway. A session that
    // keeps no log has no turns to count, and no row here — where a column
    // would have had to hold a zero and be told apart from a real one.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS capture_turns (
             event_id INTEGER PRIMARY KEY REFERENCES captures(event_id),
             turns    INTEGER NOT NULL
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the capture_turns table")?;

    Ok(())
}

/// Put an empty agent-output Event on a Conversation's Timeline, and say which
/// Event it is so the session can write into it.
///
/// Empty because nothing has been printed yet, and on the Timeline from the
/// start regardless: a session's output is a thing that is happening, and an
/// Event that appeared only once it was over would be a Timeline nobody could
/// watch.
///
/// `session_id` is what the session about to print here was named — see
/// [`crate::session_id`] — and it is written in the same transaction, so an
/// Event and the name of the session writing into it arrive together. `None`
/// where the session has no name to record.
pub async fn start_capture(
    pool: &SqlitePool,
    conversation_id: i64,
    session_id: Option<&str>,
) -> Result<i64> {
    let mut tx = pool.begin().await.context("starting a Capture")?;

    let (event_id,): (i64,) = sqlx::query_as(
        "INSERT INTO timeline_events (conversation_id, at, kind, body)
         VALUES (?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), ?, '')
         RETURNING id",
    )
    .bind(conversation_id)
    .bind(Event::AgentOutput(Summary::default()).kind())
    .fetch_one(&mut *tx)
    .await
    .with_context(|| format!("putting a session's output on the Timeline of {conversation_id}"))?;

    sqlx::query("INSERT INTO captures (event_id, lines, latest) VALUES (?, 0, '')")
        .bind(event_id)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("opening the Capture of Event {event_id}"))?;

    if let Some(session_id) = session_id {
        super::session_names::name_session(&mut tx, event_id, session_id).await?;
    }

    tx.commit().await.context("starting a Capture")?;

    Ok(event_id)
}

/// Add what the session has said since last time, and what the Timeline now
/// reads.
///
/// The chunk and the summary go in together, because a Capture whose summary
/// was one flush behind would be a Timeline saying something the details pane
/// disagreed with. The sequence number is taken from what is already there
/// rather than counted by the caller: what orders a Capture is the order its
/// chunks were written.
pub async fn append_capture(
    pool: &SqlitePool,
    event_id: i64,
    text: &str,
    summary: &Summary,
) -> Result<()> {
    let mut tx = pool.begin().await.context("adding to a Capture")?;

    sqlx::query(
        "INSERT INTO capture_chunks (event_id, seq, text)
         VALUES (
             ?,
             (SELECT COALESCE(MAX(seq), 0) + 1 FROM capture_chunks WHERE event_id = ?),
             ?
         )",
    )
    .bind(event_id)
    .bind(event_id)
    .bind(text)
    .execute(&mut *tx)
    .await
    .with_context(|| format!("adding to the Capture of Event {event_id}"))?;

    write_summary(&mut tx, event_id, summary).await?;

    tx.commit().await.context("adding to a Capture")?;

    Ok(())
}

/// Say again what a Capture's Timeline row reads, without adding to the
/// Capture.
///
/// For the one moment the two move apart. A session's summary is the last thing
/// its agent said, which is on the Transcript rather than here — and an agent
/// writes its closing words on the way out, after the terminal it was printing
/// to has already closed. So the last of the Capture arrives before the last of
/// what the session said, and this is what carries the difference.
///
/// Everything else writes the chunk and the summary together through
/// [`append_capture`], which is what keeps a Timeline from saying something the
/// details pane disagrees with.
pub async fn summarise_capture(pool: &SqlitePool, event_id: i64, summary: &Summary) -> Result<()> {
    let mut tx = pool.begin().await.context("summarising a Capture")?;

    write_summary(&mut tx, event_id, summary).await?;

    tx.commit().await.context("summarising a Capture")?;

    Ok(())
}

/// Write what the Timeline row reads: the summary's own row, and the turn count
/// beside it.
///
/// One place rather than two, because the two writers are the same write — see
/// [`summarise_capture`] for why there are two of them at all.
///
/// A summary with no count writes none and takes none away. `None` is a session
/// with no Transcript to count off, and the row that is not in `capture_turns`
/// is exactly how the Timeline knows to show no metric.
async fn write_summary(
    tx: &mut sqlx::SqliteConnection,
    event_id: i64,
    summary: &Summary,
) -> Result<()> {
    sqlx::query("UPDATE captures SET lines = ?, latest = ? WHERE event_id = ?")
        .bind(summary.lines)
        .bind(&summary.latest)
        .bind(event_id)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("summarising the Capture of Event {event_id}"))?;

    if let Some(turns) = summary.turns {
        sqlx::query(
            "INSERT INTO capture_turns (event_id, turns) VALUES (?, ?)
             ON CONFLICT (event_id) DO UPDATE SET turns = excluded.turns",
        )
        .bind(event_id)
        .bind(turns)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("counting the turns on the Transcript of Event {event_id}"))?;
    }

    Ok(())
}

/// Every Capture summary on a Conversation's Timeline, by the Event each one
/// belongs to.
///
/// Read on its own rather than joined into the Timeline's own query, for the
/// reason an Interruption's is: that query is at the sixteen columns a tuple
/// can be read back as, and a summary is now three of them. One read for the
/// Timeline either way — what is not done here is a read per Event, which is
/// the whole reason the summary is held rather than derived.
pub(crate) async fn on_timeline(
    pool: &SqlitePool,
    conversation_id: i64,
) -> Result<HashMap<i64, Summary>> {
    let rows: Vec<(i64, i64, Option<i64>, String)> = sqlx::query_as(
        "SELECT c.event_id, c.lines, t.turns, c.latest
         FROM captures c
         JOIN timeline_events e ON e.id = c.event_id
         LEFT JOIN capture_turns t ON t.event_id = c.event_id
         WHERE e.conversation_id = ?",
    )
    .bind(conversation_id)
    .fetch_all(pool)
    .await
    .with_context(|| format!("reading the Captures of Conversation {conversation_id}"))?;

    Ok(rows
        .into_iter()
        .map(|(event_id, lines, turns, latest)| {
            (
                event_id,
                Summary {
                    lines,
                    turns,
                    latest,
                },
            )
        })
        .collect())
}

/// One Conversation's Capture, whole and in order, or `None` where that
/// Conversation has no such Event.
///
/// The Conversation is part of the question rather than trusted from the path: a
/// Capture is reached through the Conversation whose Timeline it is on, and an
/// Event id that belongs to another one names nothing here.
pub async fn capture(
    pool: &SqlitePool,
    conversation_id: i64,
    event_id: i64,
) -> Result<Option<String>> {
    let found: Option<(i64,)> = sqlx::query_as(
        "SELECT c.event_id
         FROM captures c
         JOIN timeline_events e ON e.id = c.event_id
         WHERE c.event_id = ? AND e.conversation_id = ?",
    )
    .bind(event_id)
    .bind(conversation_id)
    .fetch_optional(pool)
    .await
    .with_context(|| format!("looking for the Capture of Event {event_id}"))?;

    if found.is_none() {
        return Ok(None);
    }

    let chunks: Vec<(String,)> =
        sqlx::query_as("SELECT text FROM capture_chunks WHERE event_id = ? ORDER BY seq")
            .bind(event_id)
            .fetch_all(pool)
            .await
            .with_context(|| format!("reading the Capture of Event {event_id}"))?;

    Ok(Some(chunks.into_iter().map(|(text,)| text).collect()))
}
