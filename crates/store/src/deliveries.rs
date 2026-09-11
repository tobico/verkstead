//! Which Sets' Answers have actually reached a session: the record that tells a
//! wait that ended in delivery from one that ended in nothing.
//!
//! A table of its own, for the reason a lock and a deferral each have one:
//! `question_sets` is STRICT, there is no migration machinery here, and a fact
//! learned about a Set after that table was written hangs off it rather than
//! becoming a column on it.
//!
//! **Every kind of ask, which is what makes it worth keeping at all.** The
//! folding record beside a stored ask already says when its Answers went into a
//! prompt — see [`super::deferrals`] — and says nothing about a Blocking Ask,
//! because a blocking ask's Answers reach its session by definition: the wait is
//! what delivers them. That is true of a wait that ends. It is not true of one
//! that is killed, which is an ordinary thing for a harness to do to a
//! background command, and which leaves a Set answered, a session alive and
//! nothing between them. So the one moment a Response is handed to anybody is
//! written down for every ask, and the nudge has a fact to read rather than a
//! guess to make — see [`crate::nudging`] in the server.
//!
//! **Written where the handing over happens** rather than by anything the CLI
//! says afterwards, so a session that read its Answers and then died still
//! counts as having read them. The row is the delivery; what the session did
//! with it is the session's own.
//!
//! Not to be confused with the liveness registry beside this one — see
//! [`super::waits`]. That says whether anybody is holding a wait *right now*,
//! which cannot tell a CLI that collected its Response and exited from one that
//! was killed before it could. This says which of the two happened.
//!
//! One row per delivered Set and none for a Set nobody has been handed, so the
//! row being there is the whole of the reading.

use anyhow::{Context, Result};
use sqlx::SqlitePool;

pub(crate) async fn apply_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS deliveries (
             set_id       INTEGER PRIMARY KEY REFERENCES question_sets(id),
             delivered_at TEXT NOT NULL
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the deliveries table")?;

    Ok(())
}

/// Record that Set `set_id`'s Response has been handed to a session.
///
/// The first handing over is the one that counts, so a second is ignored rather
/// than refused: a Response may be fetched again — by a session told to come
/// back for it, or by one that was handed it and asked again — and none of those
/// is a new delivery. What the time on the row means is *when its Answers first
/// reached anybody*.
pub async fn record_delivery(pool: &SqlitePool, set_id: i64) -> Result<()> {
    sqlx::query(
        "INSERT OR IGNORE INTO deliveries (set_id, delivered_at)
         VALUES (?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
    )
    .bind(set_id)
    .execute(pool)
    .await
    .with_context(|| format!("recording Question Set {set_id}'s Response as delivered"))?;

    Ok(())
}

/// Whether Set `set_id`'s Response has been handed to a session.
///
/// False for a Set nobody has answered yet, which is the same reading and wants
/// no separate one: there is nothing to have delivered.
pub async fn delivered(pool: &SqlitePool, set_id: i64) -> Result<bool> {
    let found: Option<(i64,)> = sqlx::query_as("SELECT set_id FROM deliveries WHERE set_id = ?")
        .bind(set_id)
        .fetch_optional(pool)
        .await
        .with_context(|| format!("reading whether Question Set {set_id} was delivered"))?;

    Ok(found.is_some())
}
