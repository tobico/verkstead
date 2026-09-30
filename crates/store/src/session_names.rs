//! What each session is called, by the Timeline Event that session printed into
//! — the id its agent was told to run under, or the one it turned out to have
//! chosen for itself.
//!
//! A session's own backend keeps a log of the conversation it had, in a file
//! named after the session's id — and reading that log back is what the
//! Transcript is made of. Where the backend takes an id at launch, Verkstead
//! decides it rather than discovering it, so finding the log is a lookup; the
//! alternative is working out the name the backend would have chosen, which means
//! reimplementing a private algorithm belonging to somebody else's program.
//!
//! **Where it takes none, the name is read back instead** — see [`found_as`].
//! Codex chooses its own and writes it into the log it opens, so the search that
//! finds that log is where its name is learned, and the row here is written over
//! the one the Capture opened with. Read rather than reproduced, which is the same
//! bargain the other way about: what the name is *for* is no longer only finding
//! the log but telling another device what to resume.
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

/// Write down that the session printing into `event_id` is the one Verkstead
/// named `session_id` after all: a session resuming an earlier conversation runs
/// under the name that conversation already had.
///
/// **Written over the name the Capture was opened under**, because the two facts
/// are known at two different moments. A Capture is opened before the slow part of
/// a launch — a boundary written, an account and a memory store fetched over a
/// link — so that the slow part has somewhere to say what it is doing; and whether
/// this session can continue an earlier one turns on that very store having come
/// across, which is not settled until the fetching is done. So the row the opening
/// writes is the name a fresh session would have run under, and a launch that comes
/// out a resume writes the name it is continuing over it.
///
/// Still before there is a process, which is what the row has to be true of: what
/// reads it is the lookup that finds the log this Event's Transcript is read from,
/// and the log a resumed session writes into is the one it resumed.
///
/// Takes the pool rather than a connection, unlike [`name_session`]: this is its
/// own write, after the transaction that opened the Capture and before the session
/// is spawned.
pub async fn continued_as(pool: &SqlitePool, event_id: i64, session_id: &str) -> Result<()> {
    renamed(pool, event_id, session_id).await.with_context(|| {
        format!("recording that the session of Event {event_id} continues an earlier one")
    })
}

/// And write down that the session printing into `event_id` turned out to call
/// itself `session_id` — the name a backend Verkstead could not name gave itself,
/// read out of the log as the search for it found one.
///
/// **The other half of the bargain at the top of this module.** Verkstead decides
/// the id where the backend takes one, so the log is a lookup; where the backend
/// takes none — codex — nothing known before the session started names its log,
/// and what identifies it is what the session wrote in it about itself. The
/// rollout names its own session id in the same opening line the search reads the
/// Worktree off, and that id is the one `codex resume` takes. So it goes here, in
/// the one place a session's name is kept, and a device that never ran the session
/// can carry its conversation on.
///
/// **Written over the name the Capture was opened under**, which for such a
/// backend is a name nothing ever used: the opening cannot know a name the session
/// has not chosen yet, so it writes the one a launch would have given a backend
/// that took one. What is written here is the id the log really is of, and until
/// the log is found the row says an id no harness answers to — which is a resume
/// that finds no log and falls through to Verkstead's own.
///
/// Takes the pool rather than a connection, for [`continued_as`]'s reason: this is
/// its own write, made as the Transcript is polled rather than inside anything.
pub async fn found_as(pool: &SqlitePool, event_id: i64, session_id: &str) -> Result<()> {
    renamed(pool, event_id, session_id)
        .await
        .with_context(|| format!("recording what the session of Event {event_id} calls itself"))
}

/// The write both of the two above are: the name of a session, over whatever the
/// Capture's opening put there.
///
/// One statement rather than two spellings of it, because they are one fact —
/// which session this Event's log is of — learned at two different moments and for
/// two different reasons. What differs is the story, which is the caller's.
async fn renamed(pool: &SqlitePool, event_id: i64, session_id: &str) -> Result<()> {
    sqlx::query(
        "INSERT INTO session_names (event_id, session_id) VALUES (?, ?)
         ON CONFLICT(event_id) DO UPDATE SET session_id = excluded.session_id",
    )
    .bind(event_id)
    .bind(session_id)
    .execute(pool)
    .await?;

    Ok(())
}
