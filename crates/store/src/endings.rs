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

use super::conversations::{Event, Lifecycle};

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

/// Whether the follow-up on this Conversation has been marked as over: its
/// latest answered Set carries the mark.
///
/// **The latest one decides, and the mark is never sticky.** A Set asked after
/// an end-marked Response is the follow-up going round again — the human may
/// pick Nothing else and write *one more thing* in the comment beside it — so
/// what this reads is the newest Response of the round and never the newest mark.
/// An answer without one puts the follow-up back to running.
///
/// **This follow-up's own**, which is what the window is for. A Conversation can
/// be steered into Follow-up more than once, and a mark left by the round before
/// it would end the next one before it had asked anything. So the window opens
/// at the newest move into Follow-up, exactly as a wrap-up's proposals are
/// counted from the newest move into Wrapping — see
/// [`super::conversations::last_batch_proposal`].
///
/// **Answered rather than settled**, which is the one place the two part
/// company: a Set locked unanswered carries no Response and so carries no mark,
/// and reading one as the latest word would end a follow-up on a question the
/// human never answered. It reads as *not marked*, which leaves the follow-up
/// running.
///
/// **And never a Deferred Ask**, as nothing else here counts one: its Answers
/// are for a later session by design, so a deferred Set answered in passing is
/// not the round's own last word. A store-and-nudge round is the round's own
/// last word all the same — the session that asked it is idling on the Answer
/// with its turn ended, which is exactly the session this mark ends.
pub async fn nothing_else(pool: &SqlitePool, conversation_id: i64) -> Result<bool> {
    let found: Option<(i64,)> = sqlx::query_as(
        "SELECT q.id
         FROM question_sets q
         JOIN set_events s ON s.set_id = q.id
         JOIN timeline_events e ON e.id = s.event_id
         JOIN responses r ON r.set_id = q.id
         LEFT JOIN deferrals d ON d.set_id = q.id
         WHERE e.conversation_id = ?
           AND (d.set_id IS NULL OR d.idled)
           AND e.id > COALESCE(
                   (SELECT MAX(w.id) FROM timeline_events w
                    WHERE w.conversation_id = ? AND w.kind = ? AND w.body = ?),
                   0)
         ORDER BY q.id DESC
         LIMIT 1",
    )
    .bind(conversation_id)
    .bind(conversation_id)
    .bind(Event::Moved(Lifecycle::FollowUp).kind())
    .bind(Lifecycle::FollowUp.stored())
    .fetch_optional(pool)
    .await
    .with_context(|| {
        format!("looking for the last round Conversation {conversation_id} answered")
    })?;

    let Some((set_id,)) = found else {
        return Ok(false);
    };

    ended_on(pool, set_id).await
}
