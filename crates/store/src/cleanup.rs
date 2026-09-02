//! Trimming an archived Conversation: the bulk taken out of one the human put
//! away days ago, and the mark saying it has been.
//!
//! Verkstead keeps everything, which is most of its case for being trusted with
//! a record — and everything is megabytes a session at a time. What a Cleanup
//! lets go of is the part of that nobody reads twice: the full agent output, the
//! verbatim Transcripts, and the names the sessions ran under.
//!
//! **The rule is the card.** What a Timeline card draws survives a trim, and
//! what only a drill-down shows does not. So a Capture's summary stays and its
//! chunks go — the turn count with the summary, the row and the details pane
//! both drawing it — the Transcript's lines go and the Events they hung off stay
//! where they were, and every card the Timeline is really made of is untouched:
//! the Brief, the Question Sets and their Responses, the commits and their
//! summaries, the pull requests, and what each session ran under. A Share is
//! untouched by the same rule from the other side, a Share never having carried
//! any of what goes — see `verkstead_render::sharing`, which is where what
//! boards one is decided.
//!
//! **The mark is a sidecar**, beside the archivings rather than a column on
//! them, for the reason an archiving is one itself: there is no migration
//! machinery here, and `conversations` is STRICT and left alone.
//!
//! **And the clock runs from the archiving.** One is trimmable when it has been
//! archived for longer than the Cleanup's days *and* has not been trimmed since
//! it was last archived — so a Conversation steered back to life and put away
//! again has its new bulk taken too, the mark left from last time being older
//! than the archiving that counts now. Unarchiving takes the archive row away
//! and thereby stops the clock; the mark stays where it is, because what was
//! taken is gone whatever happens next.

use anyhow::{Context, Result};
use sqlx::SqlitePool;

/// What became of trimming one.
///
/// Named the way [`Archiving`](super::Archiving)'s outcomes are, and refused for
/// the one reason archiving is not: a trim is the loss the archiving authorised,
/// so there is nothing to authorise it on a Conversation that was never put
/// away.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trimming {
    /// Trimmed: the bulk is gone and the mark is written.
    Trimmed,

    /// It has been trimmed since it was last archived. Nothing left to take and
    /// nothing wrong — what the sweep asked for holds either way.
    AlreadyTrimmed,

    /// It has not been archived, so no clock has started on it.
    NotArchived,

    /// There is no Conversation with that id.
    NoSuchConversation,
}

/// The table a trimmed Conversation's row lives in.
pub(crate) async fn apply_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS trimmed_conversations (
             conversation_id INTEGER PRIMARY KEY REFERENCES conversations(id),
             trimmed_at      TEXT NOT NULL
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the trimmed_conversations table")?;

    Ok(())
}

/// Every Conversation there is a trim to do on: archived longer than `days` ago,
/// and not trimmed since it was archived.
///
/// What the server's cleanup sweep walks. The threshold is asked of SQLite
/// rather than worked out here, so that the stamps being compared and the *now*
/// they are compared against come from the one clock — every stamp in the store
/// is SQLite's own.
///
/// The stamps sort as text: `%Y-%m-%dT%H:%M:%fZ` is fixed-width and most
/// significant first, so *before* and *less than* are the same question. That is
/// what lets the second half of the rule be a comparison rather than two parses
/// — a trim mark older than the archiving beside it is one from a life this
/// Conversation has been steered back out of and put away again since.
pub async fn trimmable(pool: &SqlitePool, days: u32) -> Result<Vec<i64>> {
    let rows: Vec<(i64,)> = sqlx::query_as(
        "SELECT a.conversation_id
         FROM archived_conversations a
         LEFT JOIN trimmed_conversations t ON t.conversation_id = a.conversation_id
         WHERE a.archived_at < strftime('%Y-%m-%dT%H:%M:%fZ', 'now', ?)
           AND (t.trimmed_at IS NULL OR t.trimmed_at < a.archived_at)
         ORDER BY a.conversation_id",
    )
    .bind(format!("-{days} days"))
    .fetch_all(pool)
    .await
    .with_context(|| format!("listing what has been archived for {days} days"))?;

    Ok(rows.into_iter().map(|(id,)| id).collect())
}

/// Take the bulk out of one archived Conversation, and write down that it has
/// been.
///
/// Read and written in one transaction, the archiving's own reason one along: a
/// Conversation unarchived from another device between the reading and the
/// deleting would otherwise be one put back on the list with its output taken
/// out from under it.
///
/// The rows go by the Events they hang off, which is what makes this one
/// Conversation's — a Capture, a Transcript and a session's name are each one
/// session's, and one session is one Event on one Timeline.
pub async fn trim_conversation(pool: &SqlitePool, id: i64) -> Result<Trimming> {
    let mut tx = super::writing(pool, "trimming a Conversation").await?;

    let known: Option<(i64,)> = sqlx::query_as("SELECT id FROM conversations WHERE id = ?")
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .with_context(|| format!("looking for Conversation {id}"))?;

    if known.is_none() {
        return Ok(Trimming::NoSuchConversation);
    }

    let archived: Option<(String,)> =
        sqlx::query_as("SELECT archived_at FROM archived_conversations WHERE conversation_id = ?")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await
            .with_context(|| format!("reading when Conversation {id} was archived"))?;

    let Some((archived_at,)) = archived else {
        return Ok(Trimming::NotArchived);
    };

    let trimmed: Option<(String,)> =
        sqlx::query_as("SELECT trimmed_at FROM trimmed_conversations WHERE conversation_id = ?")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await
            .with_context(|| format!("reading when Conversation {id} was trimmed"))?;

    // Compared as text, which is [`trimmable`]'s comparison said again — and it
    // has to be the same one: a sweep that listed a Conversation here and was
    // told it had already been trimmed would be a log line every hour about
    // work nobody can do.
    if let Some((trimmed_at,)) = trimmed
        && trimmed_at >= archived_at
    {
        return Ok(Trimming::AlreadyTrimmed);
    }

    // What a session printed, whole. The summary beside it is what the card
    // reads, and it stays.
    take(&mut tx, id, "capture_chunks", "the Captures").await?;

    // And the record the session's own backend kept of the conversation it had,
    // which is the same words again in somebody else's file format.
    take(&mut tx, id, "transcript_lines", "the Transcripts").await?;

    // And what Verkstead called each of those sessions, which is only ever a
    // name to go and look a log up by — and the logs are outside the store, in
    // worktrees a close has already swept away.
    take(&mut tx, id, "session_names", "the session names").await?;

    sqlx::query(
        "INSERT INTO trimmed_conversations (conversation_id, trimmed_at)
         VALUES (?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
         ON CONFLICT (conversation_id) DO UPDATE SET trimmed_at = excluded.trimmed_at",
    )
    .bind(id)
    .execute(&mut *tx)
    .await
    .with_context(|| format!("marking Conversation {id} trimmed"))?;

    tx.commit().await.context("trimming a Conversation")?;

    Ok(Trimming::Trimmed)
}

/// Whether one Conversation has had its bulk taken.
///
/// What the Conversation's own page asks, so that the record can say **Trimmed**
/// and a card whose drill-down is gone can say why — the reading beside
/// [`archived`](super::archived), which is the fact this one sits under.
///
/// The mark's presence and nothing else: it says the bulk of some life of this
/// Conversation is gone, which is true from the trim onwards whatever happens
/// next. That is deliberately not [`trimmable`]'s comparison read backwards. One
/// unarchived has no clock and is still missing what was taken; one archived a
/// second time is trimmable again and is *also* still missing it, and a page
/// that called that untrimmed would be a page explaining the first life's
/// missing output as breakage.
pub async fn trimmed(pool: &SqlitePool, id: i64) -> Result<bool> {
    let row: Option<(i64,)> = sqlx::query_as(
        "SELECT conversation_id FROM trimmed_conversations WHERE conversation_id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .with_context(|| format!("reading whether Conversation {id} has been trimmed"))?;

    Ok(row.is_some())
}

/// One event-keyed table emptied of one Conversation's rows, reported under
/// `what` where it fails.
///
/// The three tables a trim takes are the same shape — rows hanging off the
/// Timeline Events of one Conversation — so they are taken the same way rather
/// than written out three times. The table name is this module's own text and
/// never a caller's, there being no caller outside it.
async fn take(tx: &mut sqlx::SqliteConnection, id: i64, table: &str, what: &str) -> Result<()> {
    sqlx::query(&format!(
        "DELETE FROM {table}
         WHERE event_id IN (SELECT id FROM timeline_events WHERE conversation_id = ?)"
    ))
    .bind(id)
    .execute(&mut *tx)
    .await
    .with_context(|| format!("trimming {what} of Conversation {id}"))?;

    Ok(())
}
