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
//!
//! **And beside that row, what the Question Sets landed as.** A resumed agent
//! remembers asking a Set and remembers the id it asked under, and every id of a
//! transferred record is renumbered as it lands — so the note that primes the
//! resume has to tell it which id each of its questions has here. The landing
//! builds that map as it walks, because everything pointing at a Set has to be
//! renumbered against it, and the record lands in a leg of its own, one before
//! the arrival: there is no handing it on in memory, so it is written down. See
//! [`sets_landed`] and [`sets_as_they_landed`].
//!
//! **Which is not spent, unlike the row above.** Whether a launch continues
//! something is a moment; what a Set landed as is a fact, and it stays true for as
//! long as the record that landed is the record here. The next landing writes its
//! own map over it — see [`super::slices::land`] — and a Conversation deleted
//! takes it along.

use std::collections::HashMap;

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

/// The two tables: the conversation a launch continues, one at most per
/// Conversation because a launch continues one conversation or none — and what
/// each Question Set of the last record to land came out as, one row per Set.
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

    // Keyed by the Conversation and the id the Set was asked under, because that
    // is what the reading is by: the note answers *what is this question of mine
    // called here*, and the id it had over there is the only name the agent has
    // for it.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS landed_sets (
             conversation_id INTEGER NOT NULL REFERENCES conversations(id),
             was             INTEGER NOT NULL,
             landed_as       INTEGER NOT NULL,
             PRIMARY KEY (conversation_id, was)
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the landed_sets table")?;

    Ok(())
}

/// Write down what each Question Set of an arriving record landed as, inside the
/// transaction the record lands in.
///
/// `sets` is the landing's own map, old id against new. Written over whatever an
/// earlier landing left: a Conversation that has come and gone and come back again
/// landed under ids of its own each time, and the map of two moves ago names Sets
/// this device has renumbered since.
///
/// Takes the connection rather than the pool, unlike everything else here. This is
/// part of the landing rather than a write beside it — a map that outlived a
/// record which did not land would say a Set had an id nothing ever issued. See
/// [`super::slices::land`], which is the one caller.
pub(crate) async fn sets_landed(
    tx: &mut sqlx::SqliteConnection,
    conversation_id: i64,
    sets: &HashMap<i64, i64>,
) -> Result<()> {
    sqlx::query("DELETE FROM landed_sets WHERE conversation_id = ?")
        .bind(conversation_id)
        .execute(&mut *tx)
        .await
        .with_context(|| {
            format!(
                "letting go of what an earlier record landed Conversation \
                 {conversation_id}'s Question Sets as"
            )
        })?;

    for (was, landed_as) in sets {
        sqlx::query("INSERT INTO landed_sets (conversation_id, was, landed_as) VALUES (?, ?, ?)")
            .bind(conversation_id)
            .bind(was)
            .bind(landed_as)
            .execute(&mut *tx)
            .await
            .with_context(|| {
                format!(
                    "recording that Question Set {was} of an arriving Conversation landed as \
                     {landed_as}"
                )
            })?;
    }

    Ok(())
}

/// What the Question Sets of the record that landed against `conversation_id` came
/// out as: the id each was asked under, against the id it has here.
///
/// Read by the launch that carries a conversation on, which is the whole of what
/// the map is for: the agent it resumes knows its questions by the ids it asked
/// them under, and nothing else on this device could tell it which of them is
/// which.
///
/// Empty for every Conversation that was never moved here, and for one whose
/// record landed with no Set asked from it.
pub async fn sets_as_they_landed(
    pool: &SqlitePool,
    conversation_id: i64,
) -> Result<HashMap<i64, i64>> {
    let rows: Vec<(i64, i64)> =
        sqlx::query_as("SELECT was, landed_as FROM landed_sets WHERE conversation_id = ?")
            .bind(conversation_id)
            .fetch_all(pool)
            .await
            .with_context(|| {
                format!(
                    "reading what the Question Sets of Conversation {conversation_id} landed as"
                )
            })?;

    Ok(rows.into_iter().collect())
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

/// Whether there is a conversation standing to be carried on for
/// `conversation_id` — **read without spending it**.
///
/// The one question that has to be asked before the launch rather than by it: a
/// relaunch locks the Question Sets the dead session was idling on, and a resume
/// brings that reader back on another machine, so whether the Sets are held or
/// locked turns on this. See `server::grillings`, the one caller, where a peek
/// that came out wrong costs a Set held open for a session that then opened one of
/// its own — which the launch locks on its way through instead.
///
/// False for nearly every launch there is, this being a row written only by an
/// arrival.
pub async fn carrying_a_conversation(pool: &SqlitePool, conversation_id: i64) -> Result<bool> {
    let row: Option<(i64,)> =
        sqlx::query_as("SELECT conversation_id FROM continued_sessions WHERE conversation_id = ?")
            .bind(conversation_id)
            .fetch_optional(pool)
            .await
            .with_context(|| {
                format!(
                    "reading whether Conversation {conversation_id} has a conversation to carry on"
                )
            })?;

    Ok(row.is_some())
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
