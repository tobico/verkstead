//! What each session ran under: the name of the Agent Profile it was launched
//! from, the id of the model it was launched on and the agent that ran it, by
//! the Timeline Event that session printed into.
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
//! Which agent ran it is a second table beside that one, for the same reason
//! once more: it is a fact that arrived after the pairing did. Written in the
//! same transaction, so an Event carries the whole of what its session ran
//! under or none of it.
//!
//! An Event with no row is a session started before any of this was written
//! down, and it is not an error anywhere: what it means is a session whose
//! pairing was never recorded, which every reader shows as nothing rather than
//! as a guess. An Event with a pairing and no agent type is that one table
//! later — a session from after the pairing was written down and before the
//! agent was — and it reads the same way.

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

    /// And which agent ran it: the harness rather than the model, which is what
    /// says whose mark goes beside the reading.
    ///
    /// A copy for the name's reason. What a Profile runs is the shape of the
    /// account it holds, so a Profile retyped or deleted since would take this
    /// answer with it too.
    ///
    /// `None` for a session started before this was written down.
    pub agent_type: Option<super::AgentType>,
}

/// The table the pairings live in, and the one the agents do.
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

    // Beside that one rather than a column in it: the pairing was written down
    // first and there is no migration machinery here to widen its row with. The
    // word is [`super::AgentType::word`]'s, which is the one spelling a
    // Profile's own column is written in.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS session_agents (
             event_id   INTEGER PRIMARY KEY REFERENCES timeline_events(id),
             agent_type TEXT NOT NULL
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the session_agents table")?;

    Ok(())
}

/// Write down what the session printing into `event_id` was launched under.
///
/// Takes a connection rather than the pool, for [`super::session_names`]'s
/// reason: this happens inside the transaction that opens the Capture, so the
/// Event and what its session runs under arrive together or not at all. Both
/// rows in the one call for the same reason again — a Pairing and the agent
/// running it are one fact written in two tables.
pub(crate) async fn pair_session(
    conn: &mut sqlx::SqliteConnection,
    event_id: i64,
    ran_under: &super::Pairing,
) -> Result<()> {
    sqlx::query("INSERT INTO session_pairings (event_id, profile, model) VALUES (?, ?, ?)")
        .bind(event_id)
        .bind(&ran_under.profile.name)
        .bind(ran_under.runs_on())
        .execute(&mut *conn)
        .await
        .with_context(|| format!("recording what the session of Event {event_id} ran under"))?;

    sqlx::query("INSERT INTO session_agents (event_id, agent_type) VALUES (?, ?)")
        .bind(event_id)
        .bind(ran_under.profile.agent_type().word())
        .execute(conn)
        .await
        .with_context(|| format!("recording which agent ran the session of Event {event_id}"))?;

    Ok(())
}

/// What every session on a Conversation's Timeline ran under, by the Event each
/// one printed into.
///
/// Read for the whole Timeline rather than per Event, for the Capture
/// summaries' reason — see [`super::captures::on_timeline`]. An Event with no
/// row is simply absent from the map, and one whose agent was never written down
/// is in it without one.
pub(crate) async fn on_timeline(
    pool: &SqlitePool,
    conversation_id: i64,
) -> Result<HashMap<i64, RanUnder>> {
    let rows: Vec<(i64, String, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT p.event_id, p.profile, p.model, a.agent_type
         FROM session_pairings p
         JOIN timeline_events e ON e.id = p.event_id
         LEFT JOIN session_agents a ON a.event_id = p.event_id
         WHERE e.conversation_id = ?",
    )
    .bind(conversation_id)
    .fetch_all(pool)
    .await
    .with_context(|| {
        format!("reading what the sessions of Conversation {conversation_id} ran under")
    })?;

    rows.into_iter()
        .map(|(event_id, profile, model, agent_type)| {
            // Read back into the type rather than passed along as the word, so a
            // word nothing here wrote is heard about where it is read instead of
            // reaching the wire as something no reader has a case for.
            let agent_type = agent_type
                .as_deref()
                .map(super::AgentType::read)
                .transpose()
                .with_context(|| {
                    format!("reading which agent ran the session of Event {event_id}")
                })?;

            Ok((
                event_id,
                RanUnder {
                    profile,
                    model,
                    agent_type,
                },
            ))
        })
        .collect()
}
