//! Deferred Asks: which Sets were asked without idling a session, and which of
//! their Answers a session has already been told about.
//!
//! A table of its own, for the reason archiving has one: `question_sets` is
//! STRICT, there is no migration machinery here, and a fact learned about a Set
//! after that table was written hangs off it rather than becoming a column on
//! it.
//!
//! Beside the stored body rather than in it, which is the choice worth saying
//! out loud. The body is *what was asked* — the agent's own words, kept as they
//! were written — and how it was asked is neither the agent's wording nor
//! anything the human reads. Two things fall out of keeping it here: SQL can ask
//! the question, which is what lets a quiet session be reaped while a Deferred
//! Ask of its own is still open; and a Set whose body this build can no longer
//! read still says which kind of ask it was.
//!
//! One row per deferred Set and none for a blocking one, so the row being there
//! is the whole of *this was deferred*. What the row holds is the other half:
//! when its Answers went into a session's prompt, or nothing while they have
//! not — see [`unfolded`], which is what makes the folding a record rather than
//! something recomputed from what happens to be answered.

use std::collections::HashSet;

use anyhow::{Context, Result};
use sqlx::{SqliteConnection, SqlitePool};
use verkstead_schema::Response;

/// Which of the two ways a Set is being asked — see [`super::ask`], which is
/// where one is stored either way.
///
/// A Blocking Ask idles the session that sent it until the Response arrives; a
/// Deferred Ask does not, and its Answers reach a later session of the same
/// Conversation instead. Both land on the Timeline, both leave the Conversation
/// *blocked on you*, and both notify: what differs is who is waiting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ask {
    /// The session is idling on the Response.
    Blocking,

    /// Nothing is waiting on it.
    Deferred,
}

/// An answered Deferred Ask no session has been told about yet: the Set, and
/// what the human said to it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unfolded {
    pub set_id: i64,

    /// What was asked — or the stored body where this build can no longer read
    /// it, which is passed over rather than folded. See [`super::Asked`].
    pub set: super::Asked,

    pub response: Response,
}

pub(crate) async fn apply_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS deferrals (
             set_id    INTEGER PRIMARY KEY REFERENCES question_sets(id),
             folded_at TEXT
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the deferrals table")?;

    Ok(())
}

/// Mark a Set deferred, in the transaction that is storing it.
///
/// Taken as a connection rather than a pool because that is the only way it is
/// ever written: a deferral recorded outside the transaction that stored the Set
/// would be a moment where a Deferred Ask read as a blocking one, and what reads
/// it in that moment is the driver deciding whether a quiet session is still
/// asking.
pub(crate) async fn defer(conn: &mut SqliteConnection, set_id: i64) -> Result<()> {
    sqlx::query("INSERT INTO deferrals (set_id) VALUES (?)")
        .bind(set_id)
        .execute(conn)
        .await
        .with_context(|| format!("recording Question Set {set_id} as a Deferred Ask"))?;

    Ok(())
}

/// Which of a Conversation's Sets were asked deferred.
///
/// A read of its own rather than a join in [`super::timeline`], and the reason is
/// arithmetic rather than judgement: that query is already at the sixteen columns
/// a tuple can be read back as. Cheap on its own — one indexed column, and most
/// Conversations have no deferred Set at all.
pub async fn deferred_on_timeline(pool: &SqlitePool, conversation_id: i64) -> Result<HashSet<i64>> {
    let rows: Vec<(i64,)> = sqlx::query_as(
        "SELECT d.set_id
         FROM deferrals d
         JOIN set_events s ON s.set_id = d.set_id
         JOIN timeline_events e ON e.id = s.event_id
         WHERE e.conversation_id = ?",
    )
    .bind(conversation_id)
    .fetch_all(pool)
    .await
    .with_context(|| format!("reading Conversation {conversation_id}'s Deferred Asks"))?;

    Ok(rows.into_iter().map(|(set_id,)| set_id).collect())
}

/// Whether this Set was asked deferred.
pub async fn deferred(pool: &SqlitePool, set_id: i64) -> Result<bool> {
    let found: Option<(i64,)> = sqlx::query_as("SELECT set_id FROM deferrals WHERE set_id = ?")
        .bind(set_id)
        .fetch_optional(pool)
        .await
        .with_context(|| format!("reading whether Question Set {set_id} was deferred"))?;

    Ok(found.is_some())
}

/// The Conversation's answered Deferred Asks that no session has been told
/// about, oldest first — which is the order they were asked in, and so the order
/// the human decided them in.
///
/// Answered only. An unanswered Deferred Ask is still waiting on the human and
/// one they closed unanswered is a decision they declined to make, and neither
/// has anything to fold.
pub async fn unfolded(pool: &SqlitePool, conversation_id: i64) -> Result<Vec<Unfolded>> {
    let rows: Vec<(i64, String, String)> = sqlx::query_as(
        "SELECT q.id, q.body, r.body
         FROM deferrals d
         JOIN question_sets q ON q.id = d.set_id
         JOIN set_events s ON s.set_id = q.id
         JOIN timeline_events e ON e.id = s.event_id
         JOIN responses r ON r.set_id = q.id
         WHERE e.conversation_id = ? AND d.folded_at IS NULL
         ORDER BY q.id",
    )
    .bind(conversation_id)
    .fetch_all(pool)
    .await
    .with_context(|| format!("reading Conversation {conversation_id}'s unfolded Deferred Asks"))?;

    rows.into_iter()
        .map(|(set_id, body, answer)| {
            Ok(Unfolded {
                set_id,
                // Never a failure, whatever the body turns out to hold: an
                // unreadable Set has no exchange to write into a prompt, and it
                // is passed over where the digest is built rather than costing
                // the Sets around it — see [`super::Asked`].
                set: super::Asked::read(body),
                response: serde_json::from_str(&answer).with_context(|| {
                    format!("deserialising the stored Response to Question Set {set_id}")
                })?,
            })
        })
        .collect()
}

/// Record that these Sets' Answers have gone into a session's prompt.
///
/// Written once a session has actually been started on that prompt, so a launch
/// that came to nothing does not cost the human's Answers the one session they
/// were folded into. Folding is a record rather than something worked out from
/// what is answered: that is what makes *each is folded once and never again*
/// true of a Conversation whose sessions run for days.
pub async fn record_folded(pool: &SqlitePool, sets: &[i64]) -> Result<()> {
    for set_id in sets {
        sqlx::query(
            "UPDATE deferrals SET folded_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE set_id = ? AND folded_at IS NULL",
        )
        .bind(set_id)
        .execute(pool)
        .await
        .with_context(|| format!("recording Question Set {set_id} as folded into a prompt"))?;
    }

    Ok(())
}
