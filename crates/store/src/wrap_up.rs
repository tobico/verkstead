//! What wrap-up is still waiting on, and how many goes the machine has had at a
//! red check.
//!
//! Two small tables and no Timeline Events, which is the whole shape of this
//! module. Everything a human reads about wrap-up is already an Event — the pull
//! request, the commits a fix session lands, the Interruption where it stops
//! asking the machine. What is kept here is the bookkeeping underneath: facts
//! that decide what Verkstead does next and that nobody would want a row on a
//! Timeline for.
//!
//! Both survive a restart, and both have to. A server that came back up having
//! forgotten how many fix sessions a check had already had would dispatch them
//! again for ever, which is exactly the failure *two attempts, then ask the
//! human* exists to prevent.
//!
//! What is *settled* is written down and what is red is not: the checks are
//! asked of GitHub on every poll, so a red suite needs no memory. The row is
//! deleted again the moment they stop being green — see [`unsettle_wrap_up`] —
//! which is what makes a commit pushed to the pull request put its checks back
//! to waiting rather than leaving yesterday's green standing.

use anyhow::{Context, Result, bail};
use sqlx::SqlitePool;

/// One of the things a Conversation has to have settled before wrap-up is over.
///
/// The review's Question Set being answered and the pull request's comments
/// being addressed are the other two, and they arrive with the stages that
/// produce them — there is nothing yet that could settle either, and a variant
/// nothing ever writes would be a wrap-up that could never finish.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitingOn {
    /// The pull request's checks are green.
    Checks,
}

impl WaitingOn {
    /// The word the column holds. Lowercase and spelled out, so a database
    /// opened by hand says something.
    fn stored(self) -> &'static str {
        match self {
            Self::Checks => "checks",
        }
    }

    /// The one a stored word names. An unknown word is a database written by a
    /// Verkstead this one does not understand, exactly as an unknown lifecycle
    /// state is.
    fn read(word: &str) -> Result<Self> {
        Ok(match word {
            "checks" => Self::Checks,
            other => bail!("a wrap-up is waiting on the unknown thing {other:?}"),
        })
    }
}

/// The two tables wrap-up keeps its bookkeeping in.
///
/// Both hang off a Conversation rather than off a Timeline Event, unlike nearly
/// everything else here, and that is the point: neither is something that
/// happened, so neither is something to draw. They are what Verkstead knows
/// about a Conversation it is wrapping up.
pub(crate) async fn apply_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS wrap_up_settled (
             conversation_id INTEGER NOT NULL REFERENCES conversations(id),
             waiting_on      TEXT NOT NULL,
             at              TEXT NOT NULL,
             PRIMARY KEY (conversation_id, waiting_on)
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the wrap-up settlements table")?;

    // Keyed by the check's name rather than by anything of GitHub's, because the
    // name is what survives what this is counting: a fix session pushes a commit,
    // GitHub starts a whole new run with new ids, and *the same check* has to
    // mean the same thing across both.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS check_fix_attempts (
             conversation_id INTEGER NOT NULL REFERENCES conversations(id),
             check_name      TEXT NOT NULL,
             attempts        INTEGER NOT NULL,
             PRIMARY KEY (conversation_id, check_name)
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the check fix attempts table")?;

    Ok(())
}

/// Record that one of the things wrap-up waits on is settled.
///
/// Written again where it was settled already, which is every poll of a green
/// suite: settling is a statement about how things are now rather than an event,
/// so saying it twice is saying the same thing twice.
pub async fn settle_wrap_up(
    pool: &SqlitePool,
    conversation_id: i64,
    waiting_on: WaitingOn,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO wrap_up_settled (conversation_id, waiting_on, at)
         VALUES (?, ?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
         ON CONFLICT (conversation_id, waiting_on) DO NOTHING",
    )
    .bind(conversation_id)
    .bind(waiting_on.stored())
    .execute(pool)
    .await
    .with_context(|| {
        format!("settling {waiting_on:?} for the wrap-up of Conversation {conversation_id}")
    })?;

    Ok(())
}

/// And that it is not settled after all, which is a check that has gone red
/// again or a run that has started over.
///
/// Nothing to do where it was never settled, which is the ordinary case for as
/// long as a suite is running.
pub async fn unsettle_wrap_up(
    pool: &SqlitePool,
    conversation_id: i64,
    waiting_on: WaitingOn,
) -> Result<()> {
    sqlx::query("DELETE FROM wrap_up_settled WHERE conversation_id = ? AND waiting_on = ?")
        .bind(conversation_id)
        .bind(waiting_on.stored())
        .execute(pool)
        .await
        .with_context(|| {
            format!(
                "putting {waiting_on:?} back to waiting for the wrap-up of \
                 Conversation {conversation_id}"
            )
        })?;

    Ok(())
}

/// What a Conversation's wrap-up has settled so far.
///
/// The whole set rather than one asked about at a time, because what it is for
/// is the question *is wrap-up over* — which is about all of them together.
pub async fn wrap_up_settled(pool: &SqlitePool, conversation_id: i64) -> Result<Vec<WaitingOn>> {
    let rows: Vec<(String,)> =
        sqlx::query_as("SELECT waiting_on FROM wrap_up_settled WHERE conversation_id = ?")
            .bind(conversation_id)
            .fetch_all(pool)
            .await
            .with_context(|| {
                format!("reading what the wrap-up of Conversation {conversation_id} has settled")
            })?;

    rows.into_iter()
        .map(|(waiting_on,)| WaitingOn::read(&waiting_on))
        .collect()
}

/// How many fix sessions this check has already had.
///
/// Zero for a check nothing has been dispatched for, which is every check the
/// first time it goes red.
pub async fn fix_attempts(pool: &SqlitePool, conversation_id: i64, check: &str) -> Result<i64> {
    let row: Option<(i64,)> = sqlx::query_as(
        "SELECT attempts FROM check_fix_attempts
         WHERE conversation_id = ? AND check_name = ?",
    )
    .bind(conversation_id)
    .bind(check)
    .fetch_optional(pool)
    .await
    .with_context(|| {
        format!("reading what has been tried about {check:?} on Conversation {conversation_id}")
    })?;

    Ok(row.map(|(attempts,)| attempts).unwrap_or(0))
}

/// Count one more, and say how many that makes.
///
/// Counted as the session is dispatched rather than as it ends, which is the way
/// round that holds when a server is restarted mid-fix: an attempt that was
/// spent and not written down would be one the next server spends again.
pub async fn record_fix_attempt(
    pool: &SqlitePool,
    conversation_id: i64,
    check: &str,
) -> Result<i64> {
    let (attempts,): (i64,) = sqlx::query_as(
        "INSERT INTO check_fix_attempts (conversation_id, check_name, attempts)
         VALUES (?, ?, 1)
         ON CONFLICT (conversation_id, check_name)
             DO UPDATE SET attempts = attempts + 1
         RETURNING attempts",
    )
    .bind(conversation_id)
    .bind(check)
    .fetch_one(pool)
    .await
    .with_context(|| {
        format!("counting a fix session for {check:?} on Conversation {conversation_id}")
    })?;

    Ok(attempts)
}

/// Forget what a Conversation's checks have already been given, so they start
/// again from nothing.
///
/// What a retried Interruption does. The human has read the evidence and asked
/// for another go, and a count left standing would be a retry that raised the
/// same Interruption on its next poll without dispatching anything.
pub async fn forget_fix_attempts(pool: &SqlitePool, conversation_id: i64) -> Result<()> {
    sqlx::query("DELETE FROM check_fix_attempts WHERE conversation_id = ?")
        .bind(conversation_id)
        .execute(pool)
        .await
        .with_context(|| {
            format!("forgetting what has been tried about Conversation {conversation_id}'s checks")
        })?;

    Ok(())
}
