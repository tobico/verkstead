//! What a Cleanup does to an archived Conversation: the bulk taken out of one
//! the human put away days ago, the mark saying it has been, and — later, and
//! only where they have asked for it — the whole of it deleted.
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
//!
//! **A delete runs on that clock too, and takes the whole of it.** The one
//! thing in Verkstead that forgets, and off until the human turns it on: every
//! row the Conversation owns, walked child before parent, and the Conversation
//! itself last of all — see [`delete_conversation`]. There is no mark for it and
//! nothing for a second pass to compare against, because what a delete leaves
//! behind is no Conversation at all.
//!
//! **And what a delete never touches is what is not the store's**: the branch
//! the work is on, which belongs to the repository and which closing already
//! chose to keep, and a published Share, which is a file somebody put somewhere
//! deliberately. No git operation belongs on this path, and there is none.

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

/// And what became of deleting one.
///
/// [`Trimming`]'s outcomes less the one about doing it twice: a delete leaves
/// nothing behind to ask again about, so the second time a Conversation is
/// deleted there is no Conversation of that id — which is a sentence this
/// already had.
///
/// `Deletion` rather than `Deleting`, which every other outcome here would be
/// called: [`Deleting`](super::Deleting) is what became of deleting a Profile,
/// and one word for two questions in one crate is a word that answers neither.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Deletion {
    /// Deleted: every row it owned is gone, and so is the Conversation.
    Deleted,

    /// It has not been archived, so no clock has started on it.
    NotArchived,

    /// There is no Conversation with that id.
    NoSuchConversation,
}

/// The tables whose rows hang off this Conversation's Timeline Events, in the
/// order a delete empties them: a child before whatever it points at, every
/// time.
///
/// Emptied by the Events rather than by the Conversation because that is how
/// they are keyed — a Capture, a Transcript and a session's name are each one
/// session's, and one session is one Event on one Timeline.
const EVENT_KEYED: &[&str] = &[
    // The turn count points at the Capture, and the Capture and its chunks at
    // the Event, so the three go in that order.
    "capture_turns",
    "capture_chunks",
    "captures",
    // What the session's own backend kept of it, what Verkstead called it, and
    // what it ran under.
    "transcript_lines",
    "session_names",
    "session_pairings",
    "session_agents",
    // And the summary a commit card is drawn from. The commit itself names the
    // Conversation as well and goes with the rest of those.
    "commit_summaries",
];

/// The tables one Question Set's own rows live in, emptied a Set at a time
/// before the Set and the pairing that put it on a Timeline.
///
/// A Set is only ever asked from a Conversation — `set_events` is the one way
/// one is stored — so every Set reachable through this Conversation's Events is
/// this Conversation's, and there is nobody else it could be left holding.
const SET_KEYED: &[&str] = &["responses", "archivings", "deferrals", "endings"];

/// And the tables keyed on the Conversation itself: the sidecars, and the three
/// that name both it and an Event.
///
/// Everything the record keeps beside a Conversation rather than on it —
/// `conversations` being STRICT and left alone, which is why there are this
/// many. Enumerated from the schema rather than from memory when this was
/// written, and held against the schema by a test ever since — see
/// [`deleted_tables`].
const CONVERSATION_KEYED: &[&str] = &[
    // The Events these hang off go last of all, so they are emptied here rather
    // than with the event-keyed ones above.
    "commits",
    "pull_requests",
    "pauses",
    // What GitHub last said about the work.
    "pull_request_checks",
    "pull_request_merges",
    "pull_request_standings",
    // And what the wrap-up got through.
    "wrap_up_settled",
    "check_fix_attempts",
    "conflict_fix_attempts",
    "addressed_comments",
    "wrap_up_narrowings",
    // The other repositories it was worked in.
    "companions",
    "companion_worktrees",
    // What was shared of it, which is the record of a share rather than the
    // share: the file itself was put somewhere on purpose and stays there.
    "shares",
    "share_comments",
    // And where it sat, what it was doing, and how it was set up to do it.
    "placements",
    "unseen_conversations",
    "worktrees",
    "directions",
    "stage_branches",
    "pairing_models",
    "skipped_roles",
    "adoptions",
    // The archiving that authorised all of this, and the trim mark under it.
    "archived_conversations",
    "trimmed_conversations",
];

/// Every table a delete empties, in no particular order: what the walk covers,
/// said as a value.
///
/// Public for one reason, and it is a good one. The store's tables are declared
/// a module at a time and a Conversation is named from two dozen of them, so
/// the thing most likely to go wrong here is not this walk being wrong today but
/// a table added next year that nobody joins it to. The test holds this against
/// the schema — every table SQLite says references a Conversation has to be in
/// here — and that is a test which fails when the walk falls behind rather than
/// one that agrees with a list because both were copied from the same place.
pub fn deleted_tables() -> Vec<&'static str> {
    EVENT_KEYED
        .iter()
        .chain(SET_KEYED)
        .chain(CONVERSATION_KEYED)
        .chain(super::stops::CARRIED)
        .copied()
        .chain([
            "set_events",
            "question_sets",
            "timeline_events",
            "conversations",
        ])
        .collect()
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

/// Every Conversation there is a delete to do on: archived longer than `days`
/// ago.
///
/// [`trimmable`]'s query without its second half. A trim is a thing that can be
/// owed twice, so it asks after a mark; a delete is owed once, and the row the
/// mark would go in is one of the rows it takes.
pub async fn deletable(pool: &SqlitePool, days: u32) -> Result<Vec<i64>> {
    let rows: Vec<(i64,)> = sqlx::query_as(
        "SELECT conversation_id
         FROM archived_conversations
         WHERE archived_at < strftime('%Y-%m-%dT%H:%M:%fZ', 'now', ?)
         ORDER BY conversation_id",
    )
    .bind(format!("-{days} days"))
    .fetch_all(pool)
    .await
    .with_context(|| format!("listing what has been archived for {days} days"))?;

    Ok(rows.into_iter().map(|(id,)| id).collect())
}

/// Delete one archived Conversation and everything the store holds of it.
///
/// Refused for [`trim_conversation`]'s reason, and it is a stronger one here:
/// the archiving is the human saying they are finished looking, and it is the
/// only thing in the record that authorises forgetting any of this. One taken
/// back off the archive between the sweep listing it and this reaching it is one
/// this refuses, which is what the outcome is read for.
///
/// **One transaction, and child before parent all the way down.** Foreign keys
/// are on, so the order is not a tidiness: a parent deleted while something
/// still points at it is a failure rather than a mess left behind, and the walk
/// is written to be one SQLite would refuse if it were wrong. The Events go
/// second to last and the Conversation last, everything else hanging off one or
/// the other.
///
/// **The one column pointing the other way is nulled first.** A stop names the
/// Notice it wrote on the Timeline, so `conversations` references
/// `timeline_events` and `timeline_events` references `conversations`; there is
/// no order that deletes both, and the way through it is to let go of the Notice
/// before deleting the Event it names.
///
/// **And nothing outside the store is touched.** No branch, no worktree, no
/// published Share — see this module's header, where what a delete is not is
/// what most of the case for it rests on.
pub async fn delete_conversation(pool: &SqlitePool, id: i64) -> Result<Deletion> {
    let mut tx = super::writing(pool, "deleting a Conversation").await?;

    let known: Option<(i64,)> = sqlx::query_as("SELECT id FROM conversations WHERE id = ?")
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .with_context(|| format!("looking for Conversation {id}"))?;

    if known.is_none() {
        return Ok(Deletion::NoSuchConversation);
    }

    let archived: Option<(String,)> =
        sqlx::query_as("SELECT archived_at FROM archived_conversations WHERE conversation_id = ?")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await
            .with_context(|| format!("reading when Conversation {id} was archived"))?;

    if archived.is_none() {
        return Ok(Deletion::NotArchived);
    }

    for table in EVENT_KEYED {
        erase(
            &mut tx,
            id,
            table,
            "event_id IN (SELECT id FROM timeline_events WHERE conversation_id = ?)",
        )
        .await?;
    }

    // Read before anything is taken, because the pairing that says which Sets
    // are this Conversation's is itself one of the rows going: a Set found after
    // `set_events` had been emptied would be a Set nothing could find at all.
    let sets: Vec<(i64,)> = sqlx::query_as(
        "SELECT set_id FROM set_events
         WHERE event_id IN (SELECT id FROM timeline_events WHERE conversation_id = ?)",
    )
    .bind(id)
    .fetch_all(&mut *tx)
    .await
    .with_context(|| format!("listing the Question Sets asked from Conversation {id}"))?;

    for (set,) in sets {
        for table in SET_KEYED {
            forget(&mut tx, set, table, "set_id = ?").await?;
        }

        forget(&mut tx, set, "set_events", "set_id = ?").await?;
        forget(&mut tx, set, "question_sets", "id = ?").await?;
    }

    for table in CONVERSATION_KEYED {
        erase(&mut tx, id, table, "conversation_id = ?").await?;
    }

    // And the two tables a Verkstead of before kept a stopped Conversation in,
    // where this database is old enough to have them. Nothing writes to either
    // any more — see [`super::stops`], which reads them once on the way past —
    // but a row left in one is still a row naming this Conversation, and on a
    // database that enforces its keys it is a row nothing could delete around.
    for table in super::stops::CARRIED {
        if there(&mut tx, table).await? {
            erase(&mut tx, id, table, "conversation_id = ?").await?;
        }
    }

    sqlx::query("UPDATE conversations SET stopped_notice = NULL WHERE id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("letting go of the stop Notice of Conversation {id}"))?;

    erase(&mut tx, id, "timeline_events", "conversation_id = ?").await?;
    erase(&mut tx, id, "conversations", "id = ?").await?;

    tx.commit().await.context("deleting a Conversation")?;

    Ok(Deletion::Deleted)
}

/// One table emptied of the rows naming Conversation `id`, `by` saying how they
/// name it.
///
/// [`take`]'s shape without its verb: a delete says the same two sentences over
/// two dozen tables, and the difference between them is one `WHERE` clause. The
/// table name and the clause are both this module's own text and never a
/// caller's, there being no caller outside it.
async fn erase(tx: &mut sqlx::SqliteConnection, id: i64, table: &str, by: &str) -> Result<()> {
    sqlx::query(&format!("DELETE FROM {table} WHERE {by}"))
        .bind(id)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("deleting the {table} of Conversation {id}"))?;

    Ok(())
}

/// And the same again for the rows of one Question Set, which are found by the
/// Set rather than by the Conversation it was asked from.
///
/// Its own function rather than a second argument to [`erase`], so that neither
/// of them can be handed the wrong id: what these two bind is the whole of the
/// difference between them.
async fn forget(tx: &mut sqlx::SqliteConnection, set: i64, table: &str, by: &str) -> Result<()> {
    sqlx::query(&format!("DELETE FROM {table} WHERE {by}"))
        .bind(set)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("deleting the {table} of Question Set {set}"))?;

    Ok(())
}

/// Whether this database has a table by that name at all.
///
/// Asked only of the tables a Verkstead of before wrote, which a database made
/// since has none of — [`super::stops`] asks the same question of the same two
/// for the same reason.
async fn there(tx: &mut sqlx::SqliteConnection, table: &str) -> Result<bool> {
    let found: Option<(String,)> =
        sqlx::query_as("SELECT name FROM sqlite_master WHERE type = 'table' AND name = ?")
            .bind(table)
            .fetch_optional(&mut *tx)
            .await
            .with_context(|| format!("looking for the {table} table"))?;

    Ok(found.is_some())
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

/// Give the space a cleanup freed back to the filesystem.
///
/// The half of a cleanup SQLite will not do on its own. A `DELETE` marks the
/// pages it emptied free *inside* the database file and leaves the file the size
/// it was: what comes back is reused by whatever is written next, so a Verkstead
/// that trims stops growing — but a human who turned the delete on to get their
/// disk back gets none of it back until something rewrites the file without the
/// free pages in it. `VACUUM` is that something.
///
/// **Rather than `auto_vacuum`**, which would do it as the rows go and cost
/// nothing here. It cannot be turned on after the fact: a database made without
/// it has to be vacuumed once to change the setting at all, so the pragma buys a
/// migration and this buys none — and what it would be paying for is continuous
/// bookkeeping on every write, to save an hourly rewrite of a file a single
/// human's record fits in.
///
/// Run only after a pass that actually took something — see
/// [`crate::cleanup::sweep`] in the server, which is the whole of the caller.
/// The rewrite is not free, and a sweep that found nothing has nothing to give
/// back.
///
/// It takes the database exclusively for as long as it runs, so it can fail on
/// one that is busy. That is a pass that did not reclaim rather than a cleanup
/// that went wrong: the rows are gone either way, and the next sweep to take
/// something tries again.
pub async fn reclaim(pool: &SqlitePool) -> Result<()> {
    sqlx::query("VACUUM")
        .execute(pool)
        .await
        .context("giving back the space a cleanup freed")?;

    Ok(())
}
