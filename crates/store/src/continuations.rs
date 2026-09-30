//! The session a launch on this device **continues** rather than replaces: the
//! conversation a transferred agent was having when the work moved (ADR-0020,
//! *Transfer*).
//!
//! A Conversation that has just arrived is one whose agent is part way through a
//! turn on another machine — and where the harness has a resume of its own, the
//! session started here is that resume rather than a fresh one. What says so is
//! this row, written in the arrival leg once the record has landed and the work
//! is here: the name Verkstead gave the session that crossed, and which harness
//! ran it.
//!
//! **A row of its own rather than a reading of the Timeline.** Whether a launch
//! continues something is a fact about *this* device, and it holds while nothing
//! has been launched here yet — which is a moment rather than a shape the record
//! keeps. A table beside the Conversation is what says it, for the reason the
//! session names are a table of their own: `conversations` is STRICT, there is no
//! migration machinery here, and a fact learned about a Conversation after that
//! row was written hangs off it.
//!
//! **Spent by the first launch that reads it**, either way it comes out. The
//! session that lands in the moment after an arrival is the one with a
//! conversation to carry on; the one after that is an ordinary session of the
//! work, and a row left standing would prime it with a note about a move it had
//! already been told about. So [`take_up_the_conversation`] reads and spends in
//! one step.
//!
//! **And the harness is written down beside the name**, rather than looked up
//! through the Event that session printed into. What the launch has to ask is
//! whether the Pairing it is about to run under is the harness whose log this
//! is — a Conversation whose Profile was changed under it has a log no other
//! backend can read — and that is a fact about the session that crossed, the way
//! everything in [`super::session_pairings`] is.

use anyhow::{Context, Result};
use sqlx::SqlitePool;

use super::AgentType;

/// The conversation a launch here carries on: the name Verkstead gave the session
/// that was having it, and the harness that was having it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Continued {
    /// The session id the harness is told to resume — and the name the Capture of
    /// the session that resumes it is written down under, a resumed conversation
    /// keeping the id it already had.
    pub session_id: String,

    /// And which backend's log that name is of. A launch under any other harness
    /// has nothing it could read, so it opens a session of its own instead.
    pub agent_type: AgentType,
}

/// The table: one at most per Conversation, because a launch continues one
/// conversation or none.
pub(crate) async fn apply_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS continued_sessions (
             conversation_id INTEGER PRIMARY KEY REFERENCES conversations(id),
             session_id      TEXT NOT NULL,
             agent_type      TEXT NOT NULL
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the continued_sessions table")?;

    Ok(())
}

/// Write down that the next session launched for `conversation_id` on this device
/// continues `continued` rather than opening one of its own.
///
/// Written over whatever is there, the way an arriving row is: a Conversation
/// that has come and gone and come back again is carrying on from the newest
/// session it had, and what an earlier arrival wrote is a conversation two moves
/// ago.
pub async fn continue_on_arrival(
    pool: &SqlitePool,
    conversation_id: i64,
    continued: &Continued,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO continued_sessions (conversation_id, session_id, agent_type)
         VALUES (?, ?, ?)
         ON CONFLICT(conversation_id) DO UPDATE
             SET session_id = excluded.session_id, agent_type = excluded.agent_type",
    )
    .bind(conversation_id)
    .bind(&continued.session_id)
    .bind(continued.agent_type.word())
    .execute(pool)
    .await
    .with_context(|| {
        format!("recording the session Conversation {conversation_id} carries on from")
    })?;

    Ok(())
}

/// What the next launch for `conversation_id` carries on from, **spent in the
/// same breath**.
///
/// One transaction, for the reason a move's mark and its request are one: two
/// launches racing each other into the same moment would otherwise both read it,
/// and the second of them is a session of the work rather than the one the
/// arrival was about.
///
/// `None` is every ordinary launch, which is nearly all of them: nothing arrived,
/// or what arrived has already been carried on from.
pub async fn take_up_the_conversation(
    pool: &SqlitePool,
    conversation_id: i64,
) -> Result<Option<Continued>> {
    let mut tx = super::writing(pool, "taking up a carried conversation").await?;

    let row: Option<(String, String)> = sqlx::query_as(
        "DELETE FROM continued_sessions WHERE conversation_id = ?
         RETURNING session_id, agent_type",
    )
    .bind(conversation_id)
    .fetch_optional(&mut *tx)
    .await
    .with_context(|| {
        format!("reading the session Conversation {conversation_id} carries on from")
    })?;

    tx.commit()
        .await
        .context("taking up a carried conversation")?;

    let Some((session_id, agent_type)) = row else {
        return Ok(None);
    };

    // Read back into the type rather than handed on as the word, which is the
    // reading [`super::session_pairings`] makes of the same column: a word
    // nothing here wrote is heard about where it is read rather than reaching a
    // launch as a harness no arm has a case for.
    let agent_type = AgentType::read(&agent_type).with_context(|| {
        format!("reading which harness Conversation {conversation_id} carries on from")
    })?;

    Ok(Some(Continued {
        session_id,
        agent_type,
    }))
}

/// The newest session of `conversation_id`, as something a launch could carry on
/// from: the name Verkstead gave it and the harness that ran it.
///
/// **The newest, because that is the conversation the agent was having.** The
/// record holds every name Verkstead has given this Conversation's sessions, in
/// the order of the Events they printed into, and the last of them is the one
/// that was running when the work moved.
///
/// `None` three ways, and each of them is a Conversation there is nothing to
/// carry on from: one whose sessions were never named, which is every backend
/// Verkstead does not name a session for; one whose newest session's harness was
/// never written down, which is a session from before that was recorded; and one
/// with no sessions at all.
pub async fn the_last_session(
    pool: &SqlitePool,
    conversation_id: i64,
) -> Result<Option<Continued>> {
    // Left, and refused on a null below rather than skipped over: a session
    // whose harness nothing wrote down is still the newest session, and carrying
    // on from the one before it would be resuming a conversation that had
    // already been left.
    let row: Option<(String, Option<String>)> = sqlx::query_as(
        "SELECT session_names.session_id, session_agents.agent_type
         FROM session_names
         JOIN timeline_events ON timeline_events.id = session_names.event_id
         LEFT JOIN session_agents ON session_agents.event_id = session_names.event_id
         WHERE timeline_events.conversation_id = ?
         ORDER BY session_names.event_id DESC
         LIMIT 1",
    )
    .bind(conversation_id)
    .fetch_optional(pool)
    .await
    .with_context(|| format!("reading the newest session of Conversation {conversation_id}"))?;

    let Some((session_id, Some(agent_type))) = row else {
        return Ok(None);
    };

    let agent_type = AgentType::read(&agent_type).with_context(|| {
        format!("reading which harness the newest session of Conversation {conversation_id} ran")
    })?;

    Ok(Some(Continued {
        session_id,
        agent_type,
    }))
}
