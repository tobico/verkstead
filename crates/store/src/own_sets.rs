//! The Question Sets Verkstead asks itself, rather than an agent in a session
//! (ADR-0029, *Verkstead asks with a Question Set of its own*): which Sets they
//! are, and what each one is asking about.
//!
//! A table of its own, for the reason a lock and a deferral each have one:
//! `question_sets` is STRICT, there is no migration machinery here, and a fact
//! learned about a Set hangs off it rather than becoming a column on it. And
//! beside the body rather than in it, for the reason how a Set was asked is
//! kept beside it — see [`super::deferrals`]: the body is what the human reads,
//! and what Verkstead does with their answer is neither.
//!
//! **What it is asking about is kept as Verkstead wrote it down**, rather than
//! read back out of the words on the Set. A Set of Verkstead's is settled by
//! Verkstead acting on the Option picked — moving the work, or launching it
//! where it is — and which device *Move to nuc-win* means is a Device Id, which
//! no sentence on the Set says.
//!
//! One row per Set of Verkstead's and none for an agent's, so the row being
//! there is the whole of *Verkstead asked this*.

use std::collections::HashSet;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sqlx::{SqliteConnection, SqlitePool};
use verkstead_schema::Response;

/// What a Set of Verkstead's is asking about, as Verkstead will act on it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "asking", rename_all = "snake_case")]
pub enum OwnAsking {
    /// Where work that wants a platform is to run, there being devices of that
    /// platform linked and none it can be moved to yet: none this Conversation
    /// may be moved to, or the ones it may be all asleep.
    ///
    /// Its Options are one *Move to* per device of `moves`, in that order, and
    /// then *Run it here*.
    Placing {
        work: PlacedWork,

        /// The platform it wants, as the human writes it — *Windows*.
        wants: String,

        /// The Device Ids the *Move to* Options name, in the order they are
        /// offered.
        moves: Vec<String>,

        /// The Device Ids of the permitted devices that were asleep, in the
        /// order they are tried: the first of them to answer cleanly while the
        /// Set is open takes the work, and the Set is locked saying so. Empty
        /// where none was, and on a Set recorded before there were any.
        #[serde(default)]
        waiting_on: Vec<String>,
    },
}

/// The work a placing Set is about.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlacedWork {
    /// A task of the backlog, by its number and its label as the list writes
    /// it.
    Task { number: u32, label: String },

    /// The inline run, its implementation session about to start.
    Inline,

    /// A roadmap stage, by its label as the roadmap writes it, its planning
    /// session about to start.
    Stage { label: String },

    /// The session's own call, `verkstead transfer` naming a device that is
    /// linked and not permitted, or permitted and asleep: a request rather
    /// than a refusal (ADR-0029). Its `wants` is that device's OS word, and its
    /// last Option is *Keep it here*, which carries the session on where it is.
    Called,
}

/// One of a Conversation's Sets of Verkstead's own, and how it stands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnSet {
    pub set_id: i64,

    /// The Timeline Event it hangs on, which says when it was asked against
    /// the rest of the Timeline — see [`building_since`].
    pub event_id: i64,

    pub asking: OwnAsking,

    /// What the human said to it, where they have answered.
    pub response: Option<Response>,

    /// Whether it was locked unanswered.
    pub locked: bool,
}

pub(crate) async fn apply_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS own_sets (
             set_id INTEGER PRIMARY KEY REFERENCES question_sets(id),
             asking TEXT NOT NULL
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the own_sets table")?;

    Ok(())
}

/// Record that Set `set_id` is Verkstead's, asking about `asking`, in the
/// transaction that is storing it.
pub(crate) async fn record(
    connection: &mut SqliteConnection,
    set_id: i64,
    asking: &OwnAsking,
) -> Result<()> {
    let asking = serde_json::to_string(asking).context("serialising what a Set asks about")?;

    sqlx::query("INSERT INTO own_sets (set_id, asking) VALUES (?, ?)")
        .bind(set_id)
        .bind(asking)
        .execute(&mut *connection)
        .await
        .with_context(|| format!("recording Question Set {set_id} as Verkstead's own"))?;

    Ok(())
}

/// What Set `set_id` asks about, or `None` where an agent asked it.
pub async fn own_asking(pool: &SqlitePool, set_id: i64) -> Result<Option<OwnAsking>> {
    let found: Option<(String,)> = sqlx::query_as("SELECT asking FROM own_sets WHERE set_id = ?")
        .bind(set_id)
        .fetch_optional(pool)
        .await
        .with_context(|| format!("reading whether Question Set {set_id} is Verkstead's own"))?;

    found
        .map(|(asking,)| {
            serde_json::from_str(&asking)
                .with_context(|| format!("reading what Question Set {set_id} asks about"))
        })
        .transpose()
}

/// Which of one Conversation's Sets are Verkstead's own, which the Timeline
/// draws with their asker named.
pub async fn own_set_ids(pool: &SqlitePool, conversation_id: i64) -> Result<HashSet<i64>> {
    let rows: Vec<(i64,)> = sqlx::query_as(
        "SELECT o.set_id
         FROM own_sets o
         JOIN set_events s ON s.set_id = o.set_id
         JOIN timeline_events e ON e.id = s.event_id
         WHERE e.conversation_id = ?",
    )
    .bind(conversation_id)
    .fetch_all(pool)
    .await
    .with_context(|| format!("reading Conversation {conversation_id}'s Sets of Verkstead's own"))?;

    Ok(rows.into_iter().map(|(set_id,)| set_id).collect())
}

/// One Conversation's Sets of Verkstead's own, oldest first, each with how it
/// stands.
pub async fn own_sets(pool: &SqlitePool, conversation_id: i64) -> Result<Vec<OwnSet>> {
    let rows: Vec<(i64, i64, String, Option<String>, bool)> = sqlx::query_as(
        "SELECT o.set_id, s.event_id, o.asking, r.body, a.set_id IS NOT NULL
         FROM own_sets o
         JOIN set_events s ON s.set_id = o.set_id
         JOIN timeline_events e ON e.id = s.event_id
         LEFT JOIN responses r ON r.set_id = o.set_id
         LEFT JOIN archivings a ON a.set_id = o.set_id
         WHERE e.conversation_id = ?
         ORDER BY o.set_id",
    )
    .bind(conversation_id)
    .fetch_all(pool)
    .await
    .with_context(|| format!("reading Conversation {conversation_id}'s Sets of Verkstead's own"))?;

    rows.into_iter()
        .map(|(set_id, event_id, asking, response, locked)| {
            Ok(OwnSet {
                set_id,
                event_id,
                asking: serde_json::from_str(&asking)
                    .with_context(|| format!("reading what Question Set {set_id} asks about"))?,
                response: response
                    .map(|response| serde_json::from_str(&response))
                    .transpose()
                    .with_context(|| {
                        format!("deserialising the stored Response to Question Set {set_id}")
                    })?,
                locked,
            })
        })
        .collect()
}

/// The Timeline Event the round of building now under way began at: the newest
/// move into Implementing, or `None` where the Conversation has never made one.
///
/// What a placing Set's answer about a task is good for. A backlog's tasks are
/// numbered from `01`, and a Conversation sent back to be built works a fresh
/// backlog numbered from `01` again — see [`super::implement_again`] — so a
/// *Run it here* answered about task 03 of the last one says nothing about task
/// 03 of this one.
pub async fn building_since(pool: &SqlitePool, conversation_id: i64) -> Result<Option<i64>> {
    let moved = super::Event::Moved(super::Lifecycle::Implementing);

    let found: Option<(i64,)> = sqlx::query_as(
        "SELECT MAX(id) FROM timeline_events
         WHERE conversation_id = ? AND kind = ? AND body = ?
         HAVING MAX(id) IS NOT NULL",
    )
    .bind(conversation_id)
    .bind(moved.kind())
    .bind(super::Lifecycle::Implementing.stored())
    .fetch_optional(pool)
    .await
    .with_context(|| {
        format!("reading when Conversation {conversation_id}'s round of building began")
    })?;

    Ok(found.map(|(id,)| id))
}
