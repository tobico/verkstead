//! The Nothing-else mark: the human saying, on a follow-up's Set, that there is
//! nothing else to follow up.
//!
//! A table of its own, for the reason a lock and a deferral each have one:
//! `responses` is STRICT, there is no migration machinery here, and a fact about
//! a Response that is not part of what was answered hangs off it rather than
//! becoming a column on it.
//!
//! Beside the stored body rather than in it, which is the whole point of the
//! arrangement here. The body is *what the agent is handed* — the Answers, the
//! comment, the direction — and the mark is none of that: it is the human
//! telling Verkstead the follow-up is over. Kept here, the Response a waiting
//! session reads is byte for byte the Response it would have read without one,
//! so the agent has nothing to know about how a follow-up ends and no way to
//! act on it. [`super::insert_response`] is where the two are split apart, which
//! is the one place a Response is stored.
//!
//! One row per marked Response and none for an ordinary one, so the row being
//! there is the whole of it.

use anyhow::{Context, Result};
use sqlx::{SqliteConnection, SqlitePool};

pub(crate) async fn apply_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS endings (
             set_id INTEGER PRIMARY KEY REFERENCES question_sets(id)
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the endings table")?;

    Ok(())
}

/// Record that this Set's Response carried the mark, in the transaction that is
/// storing the Response.
///
/// Taken as a connection rather than a pool because that is the only way it is
/// ever written: a mark recorded outside the transaction that stored the
/// Response would be a moment where an ended follow-up read as one still going,
/// and a moment the other way if the Response were the one that failed.
pub(crate) async fn mark(conn: &mut SqliteConnection, set_id: i64) -> Result<()> {
    sqlx::query("INSERT INTO endings (set_id) VALUES (?)")
        .bind(set_id)
        .execute(conn)
        .await
        .with_context(|| format!("marking the Response to Question Set {set_id} as the last"))?;

    Ok(())
}

/// Whether this Set's Response carried the mark.
///
/// Per Set, because that is what decides it: the mark is never sticky, and what
/// a rule about a whole follow-up asks is whether the *latest* settled Set of
/// the Conversation carries one.
pub async fn ended_on(pool: &SqlitePool, set_id: i64) -> Result<bool> {
    let found: Option<(i64,)> = sqlx::query_as("SELECT set_id FROM endings WHERE set_id = ?")
        .bind(set_id)
        .fetch_optional(pool)
        .await
        .with_context(|| format!("reading the Nothing-else mark on Question Set {set_id}"))?;

    Ok(found.is_some())
}
