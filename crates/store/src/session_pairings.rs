//! What each session ran under: the name of the Agent Profile it was launched
//! from and the id of the model it was launched on, by the Timeline Event that
//! session printed into.
//!
//! Written down rather than looked up, because it is history rather than
//! status. The server has the Pairing in hand at the moment it starts a session
//! and nowhere else: a Conversation's Pairing is a thing the human can change,
//! a Profile can be renamed or deleted, and a Verkstead that has been restarted
//! has no sessions at all — so a record that asked the Conversation what its
//! sessions ran under would answer for the next one rather than for the one it
//! was asked about.
//!
//! A table of its own rather than a column on the Capture, for the reason
//! [`super::session_names`] is one: there is no migration machinery here, and a
//! fact that arrived after the Capture did can be added beside it without one.
//! One row per Event, because one Event is one session.
//!
//! An Event with no row is a session started before any of this was written
//! down, and it is not an error anywhere: what it means is a session whose
//! pairing was never recorded, which every reader shows as nothing rather than
//! as a guess.

use std::collections::HashMap;

use anyhow::{Context, Result};
use sqlx::SqlitePool;

/// What one session was launched under.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RanUnder {
    /// The Agent Profile's name as it read when the session started. A copy
    /// rather than the Profile's id: what the record is for is saying what ran,
    /// and a Profile that has since been renamed or deleted would take the
    /// answer with it.
    pub profile: String,

    /// The model id it was launched on, raw — `claude-opus-5` rather than
    /// "Opus 5". Prettifying is the viewer's, so an id nothing here has heard
    /// of still reaches the human as the id.
    ///
    /// `None` where the Pairing named no model at all, which is a Profile
    /// somebody left without one: the session was launched on whatever its
    /// agent defaults to, and there is nothing true to write down.
    pub model: Option<String>,
}

/// The table the pairings live in.
pub(crate) async fn apply_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS session_pairings (
             event_id INTEGER PRIMARY KEY REFERENCES timeline_events(id),
             profile  TEXT NOT NULL,
             model    TEXT
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the session_pairings table")?;

    Ok(())
}

/// Write down what the session printing into `event_id` was launched under.
///
/// Takes a connection rather than the pool, for [`super::session_names`]'s
/// reason: this happens inside the transaction that opens the Capture, so the
/// Event and what its session runs under arrive together or not at all.
pub(crate) async fn pair_session(
    conn: &mut sqlx::SqliteConnection,
    event_id: i64,
    ran_under: &super::Pairing,
) -> Result<()> {
    sqlx::query("INSERT INTO session_pairings (event_id, profile, model) VALUES (?, ?, ?)")
        .bind(event_id)
        .bind(&ran_under.profile.name)
        .bind(ran_under.runs_on())
        .execute(conn)
        .await
        .with_context(|| format!("recording what the session of Event {event_id} ran under"))?;

    Ok(())
}

/// What every session on a Conversation's Timeline ran under, by the Event each
/// one printed into.
///
/// Read for the whole Timeline rather than per Event, for the Capture
/// summaries' reason — see [`super::captures::on_timeline`]. An Event with no
/// row is simply absent from the map.
pub(crate) async fn on_timeline(
    pool: &SqlitePool,
    conversation_id: i64,
) -> Result<HashMap<i64, RanUnder>> {
    let rows: Vec<(i64, String, Option<String>)> = sqlx::query_as(
        "SELECT p.event_id, p.profile, p.model
         FROM session_pairings p
         JOIN timeline_events e ON e.id = p.event_id
         WHERE e.conversation_id = ?",
    )
    .bind(conversation_id)
    .fetch_all(pool)
    .await
    .with_context(|| {
        format!("reading what the sessions of Conversation {conversation_id} ran under")
    })?;

    Ok(rows
        .into_iter()
        .map(|(event_id, profile, model)| (event_id, RanUnder { profile, model }))
        .collect())
}
