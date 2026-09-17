//! A session Verkstead told the human about: one gone **Idle** that the Rescue
//! spoke to three times in a row without an answer.
//!
//! Not a stop. The session is still running and holds its Worktree, nothing
//! about driving has changed, and Resume is not what the human is offered —
//! nothing stopped. What is kept here is the one fact the Notice cannot carry:
//! that the session is *still* sitting there, which is what the *blocked on
//! you* badge is drawn from beside an open Set and a stop.
//!
//! A column on the Conversation, for the reason a stop is one: the badge is
//! folded per row of the sidebar in one query, and one escalation standing per
//! Conversation is a fact about the record rather than a rule to keep.
//!
//! Cleared when the session is seen working again or is ended — see
//! [`settle_escalation`]. What an escalation *means* is the server's, in its
//! `rescues` module.

use anyhow::{Context, Result};
use sqlx::SqlitePool;

use super::conversations::Event;

/// The column, added to `conversations` rather than declared with it — see
/// [`super::stops::apply_schema`], which the same reasoning is written beside.
pub(crate) async fn apply_schema(pool: &SqlitePool) -> Result<()> {
    let there: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM pragma_table_info('conversations') WHERE name = 'escalated_notice'",
    )
    .fetch_optional(pool)
    .await
    .context("looking for the escalated_notice column of a Conversation")?;

    if there.is_none() {
        sqlx::query(
            "ALTER TABLE conversations
             ADD COLUMN escalated_notice INTEGER REFERENCES timeline_events(id)",
        )
        .execute(pool)
        .await
        .context("adding the escalated_notice column to the Conversations")?;
    }

    Ok(())
}

/// Put the Notice on the Timeline and say the Conversation is waiting on the
/// human over it.
///
/// The Notice it became, or `None` where nothing was written: one escalation
/// standing already, which is the same silence noticed twice, or a
/// Conversation that is not there any more.
pub async fn escalate(
    pool: &SqlitePool,
    conversation_id: i64,
    markdown: &str,
) -> Result<Option<i64>> {
    let mut tx = super::writing(pool, "escalating over a Conversation").await?;

    let standing: Option<(Option<i64>,)> =
        sqlx::query_as("SELECT escalated_notice FROM conversations WHERE id = ?")
            .bind(conversation_id)
            .fetch_optional(&mut *tx)
            .await
            .with_context(|| {
                format!("asking whether Conversation {conversation_id} has an escalation standing")
            })?;

    let Some((None,)) = standing else {
        return Ok(None);
    };

    let event = Event::Notice(markdown.to_owned());

    let (notice,): (i64,) = sqlx::query_as(
        "INSERT INTO timeline_events (conversation_id, at, kind, body)
         VALUES (?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), ?, ?)
         RETURNING id",
    )
    .bind(conversation_id)
    .bind(event.kind())
    .bind(markdown)
    .fetch_one(&mut *tx)
    .await
    .with_context(|| {
        format!(
            "putting the Notice of an escalation on the Timeline of Conversation {conversation_id}"
        )
    })?;

    sqlx::query("UPDATE conversations SET escalated_notice = ? WHERE id = ?")
        .bind(notice)
        .bind(conversation_id)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("recording the escalation over Conversation {conversation_id}"))?;

    tx.commit()
        .await
        .context("escalating over a Conversation")?;

    Ok(Some(notice))
}

/// Take the escalation that `notice` wrote away, and say whether there was one.
///
/// By the Notice rather than by the Conversation, so that the session an
/// escalation was about can only ever take its own away: a later session's,
/// written in the moment between one ending and this landing, stands.
pub async fn settle_escalation(
    pool: &SqlitePool,
    conversation_id: i64,
    notice: i64,
) -> Result<bool> {
    let settled = sqlx::query(
        "UPDATE conversations SET escalated_notice = NULL
          WHERE id = ? AND escalated_notice = ?",
    )
    .bind(conversation_id)
    .bind(notice)
    .execute(pool)
    .await
    .with_context(|| format!("settling the escalation over Conversation {conversation_id}"))?;

    Ok(settled.rows_affected() > 0)
}

/// The Notice of the escalation standing over a Conversation, or `None` where
/// there is none.
pub async fn escalated(pool: &SqlitePool, conversation_id: i64) -> Result<Option<i64>> {
    let row: Option<(Option<i64>,)> =
        sqlx::query_as("SELECT escalated_notice FROM conversations WHERE id = ?")
            .bind(conversation_id)
            .fetch_optional(pool)
            .await
            .with_context(|| {
                format!("reading whether Conversation {conversation_id} has an escalation standing")
            })?;

    Ok(row.and_then(|(notice,)| notice))
}

/// The same rule as [`escalated`] said as SQL about a Conversation row aliased
/// `c`, for the fold of what waits on the human.
pub(crate) fn waited_on() -> &'static str {
    "c.escalated_notice IS NOT NULL"
}
