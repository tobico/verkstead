//! What a session printed: the transcript kept whole, and the summary its
//! Timeline Event is read by.
//!
//! Two tables rather than one more Event body, and the reason is the shape of
//! the writing. A transcript grows for as long as its session runs, a chunk at a
//! time — and `UPDATE … SET body = body || ?` rewrites every byte already there
//! on each of those, so an hour-long session would cost more the longer it had
//! been talking. Chunks are inserted instead, and read back in the order they
//! arrived.
//!
//! The summary sits beside them because of the shape of the reading. Every open
//! page reads the whole Timeline, and working out a line count and a last
//! statement from the chunks would mean reading every transcript of every
//! session to draw two lines of text. What writes it is the relay that writes
//! the chunks — see the server's `transcript` module — which is following the
//! output anyway and can say both a chunk at a time.
//!
//! Nothing here says whether a session is still running. A running session is a
//! process, and a table cannot hold one: what a restarted server has is the
//! transcript of a session that is over, which is exactly what these rows say.

use anyhow::{Context, Result};
use sqlx::SqlitePool;

use super::conversations::Event;

/// A transcript as the Timeline shows it: how much of it there is, and the last
/// thing the session said.
///
/// The design's summary for an agent-output Event, held rather than derived —
/// see the module's own documentation for why.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Summary {
    /// How many lines the session has printed.
    pub lines: i64,

    /// The last of them that said anything, with the terminal's own control
    /// sequences taken out. Empty where the session has printed nothing yet.
    pub latest: String,
}

/// The transcript tables. Both hang off a Timeline Event: a transcript is one
/// Event's full self, and the Event is what a Conversation's Timeline holds.
pub(crate) async fn apply_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS transcripts (
             event_id INTEGER PRIMARY KEY REFERENCES timeline_events(id),
             lines    INTEGER NOT NULL,
             latest   TEXT NOT NULL
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the transcripts table")?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS transcript_chunks (
             event_id INTEGER NOT NULL REFERENCES timeline_events(id),
             seq      INTEGER NOT NULL,
             text     TEXT NOT NULL,
             PRIMARY KEY (event_id, seq)
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the transcript_chunks table")?;

    Ok(())
}

/// Put an empty agent-output Event on a Conversation's Timeline, and say which
/// Event it is so the session can write into it.
///
/// Empty because nothing has been printed yet, and on the Timeline from the
/// start regardless: a session's output is a thing that is happening, and an
/// Event that appeared only once it was over would be a Timeline nobody could
/// watch.
pub async fn start_transcript(pool: &SqlitePool, conversation_id: i64) -> Result<i64> {
    let mut tx = pool.begin().await.context("starting a transcript")?;

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

    sqlx::query("INSERT INTO transcripts (event_id, lines, latest) VALUES (?, 0, '')")
        .bind(event_id)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("opening the transcript of Event {event_id}"))?;

    tx.commit().await.context("starting a transcript")?;

    Ok(event_id)
}

/// Add what the session has said since last time, and what the Timeline now
/// reads.
///
/// The chunk and the summary go in together, because a transcript whose summary
/// was one flush behind would be a Timeline saying something the details pane
/// disagreed with. The sequence number is taken from what is already there
/// rather than counted by the caller: what orders a transcript is the order its
/// chunks were written.
pub async fn append_transcript(
    pool: &SqlitePool,
    event_id: i64,
    text: &str,
    summary: &Summary,
) -> Result<()> {
    let mut tx = pool.begin().await.context("adding to a transcript")?;

    sqlx::query(
        "INSERT INTO transcript_chunks (event_id, seq, text)
         VALUES (
             ?,
             (SELECT COALESCE(MAX(seq), 0) + 1 FROM transcript_chunks WHERE event_id = ?),
             ?
         )",
    )
    .bind(event_id)
    .bind(event_id)
    .bind(text)
    .execute(&mut *tx)
    .await
    .with_context(|| format!("adding to the transcript of Event {event_id}"))?;

    sqlx::query("UPDATE transcripts SET lines = ?, latest = ? WHERE event_id = ?")
        .bind(summary.lines)
        .bind(&summary.latest)
        .bind(event_id)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("summarising the transcript of Event {event_id}"))?;

    tx.commit().await.context("adding to a transcript")?;

    Ok(())
}

/// One Conversation's transcript, whole and in order, or `None` where that
/// Conversation has no such Event.
///
/// The Conversation is part of the question rather than trusted from the path: a
/// transcript is reached through the Conversation whose Timeline it is on, and
/// an Event id that belongs to another one names nothing here.
pub async fn transcript(
    pool: &SqlitePool,
    conversation_id: i64,
    event_id: i64,
) -> Result<Option<String>> {
    let found: Option<(i64,)> = sqlx::query_as(
        "SELECT t.event_id
         FROM transcripts t
         JOIN timeline_events e ON e.id = t.event_id
         WHERE t.event_id = ? AND e.conversation_id = ?",
    )
    .bind(event_id)
    .bind(conversation_id)
    .fetch_optional(pool)
    .await
    .with_context(|| format!("looking for the transcript of Event {event_id}"))?;

    if found.is_none() {
        return Ok(None);
    }

    let chunks: Vec<(String,)> =
        sqlx::query_as("SELECT text FROM transcript_chunks WHERE event_id = ? ORDER BY seq")
            .bind(event_id)
            .fetch_all(pool)
            .await
            .with_context(|| format!("reading the transcript of Event {event_id}"))?;

    Ok(Some(chunks.into_iter().map(|(text,)| text).collect()))
}
