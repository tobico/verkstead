//! What Verkstead called each session: the id its agent was told to run under,
//! by the Timeline Event that session printed into.
//!
//! A session's own backend keeps a log of the conversation it had, in a file
//! named after the session's id — and reading that log back is what the
//! Transcript is made of. Verkstead decides the id rather than discovering it,
//! so finding the log is a lookup; the alternative is working out the name the
//! backend would have chosen, which means reimplementing a private algorithm
//! belonging to somebody else's program.
//!
//! A table of its own rather than a column on the Capture, for the reason the
//! Sets' locks are a table of their own: there is no migration machinery here,
//! and a fact that arrived after the Capture did can be added beside it without
//! one. One row per Event, because one Event is one session.
//!
//! A session with no row is one Verkstead could not name — see the server's
//! `sessions` module — and it is not an error anywhere: what it means is a
//! session whose log cannot be looked up, which is also every session that
//! leaves no log at all. Both fall back to the Capture, which is a complete
//! record either way.

use anyhow::{Context, Result};
use sqlx::SqlitePool;

/// The table the names live in.
pub(crate) async fn apply_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS session_names (
             event_id   INTEGER PRIMARY KEY REFERENCES timeline_events(id),
             session_id TEXT NOT NULL
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the session_names table")?;

    Ok(())
}

/// Write down that the session printing into `event_id` was named `session_id`.
///
/// Takes a connection rather than the pool, because this happens inside the
/// transaction that opens the Capture: an Event that carried a name for a
/// session it had not been given would be a lookup that found somebody else's
/// log.
pub(crate) async fn name_session(
    conn: &mut sqlx::SqliteConnection,
    event_id: i64,
    session_id: &str,
) -> Result<()> {
    sqlx::query("INSERT INTO session_names (event_id, session_id) VALUES (?, ?)")
        .bind(event_id)
        .bind(session_id)
        .execute(conn)
        .await
        .with_context(|| format!("naming the session of Event {event_id}"))?;

    Ok(())
}

/// What the session that printed into `event_id` was called, or `None` where it
/// was never named.
pub async fn session_id(pool: &SqlitePool, event_id: i64) -> Result<Option<String>> {
    let named: Option<(String,)> =
        sqlx::query_as("SELECT session_id FROM session_names WHERE event_id = ?")
            .bind(event_id)
            .fetch_optional(pool)
            .await
            .with_context(|| format!("looking up the name of the session of Event {event_id}"))?;

    Ok(named.map(|(session_id,)| session_id))
}

/// And every name a session of `conversation` was given, oldest first.
///
/// What one harness's memory store is asked by across a cluster: grok files a
/// session's directory under the id it was run with, so the ids this
/// Conversation has had are what say which of a shared account's sessions are
/// this Conversation's — see the server's `mirroring::memory`. The one this
/// session is about to run under is on the list too, its Capture having been
/// opened before the launch that reads this.
///
/// Empty for a Conversation whose sessions were never named, which is every
/// backend that takes no session id.
pub async fn session_ids(pool: &SqlitePool, conversation: i64) -> Result<Vec<String>> {
    let named: Vec<(String,)> = sqlx::query_as(
        "SELECT session_names.session_id
         FROM session_names
         JOIN timeline_events ON timeline_events.id = session_names.event_id
         WHERE timeline_events.conversation_id = ?
         ORDER BY session_names.event_id",
    )
    .bind(conversation)
    .fetch_all(pool)
    .await
    .with_context(|| {
        format!("looking up the names of the sessions of Conversation {conversation}")
    })?;

    Ok(named.into_iter().map(|(session_id,)| session_id).collect())
}
