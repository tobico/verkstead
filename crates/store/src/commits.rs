//! The commits a session lands on a Conversation's branch, as Timeline Events.
//!
//! A commit is the only visible product of unattended execution: the sessions
//! run without being driven, and what they leave behind is history on a branch.
//! So each one becomes an Event — what it changed, in a line — and the details
//! pane shows its diff.
//!
//! What the Timeline draws — the line about a commit, and the Commit Summary the
//! agent wrote under its subject — is held here rather than read back out of git
//! every time a page looks, for the reason a Capture's is: every open page reads
//! the whole Timeline, and a repository asked once per commit per read would be a
//! git process per row of it. The diff itself is *not* held, and that is the
//! other half of the same judgement — it is megabytes the Timeline never shows,
//! and the repository already has it.
//!
//! Nothing here talks to git. What is recorded is what the server read off a
//! repository — see the server's `commits` module — exactly as a worktree is
//! recorded rather than made here.
//!
//! One commit per Conversation per sha, by the unique index. That is what makes
//! *exactly once* a fact about the database rather than a promise made by
//! whatever is watching the branch: a sweep that runs twice over the same
//! commit records it once, whether the second sweep is a poll that overlapped,
//! a session restarting, or a server that came back up.

use std::collections::HashMap;

use anyhow::{Context, Result};
use sqlx::SqlitePool;

use super::conversations::Event;

/// A commit as its Timeline Event holds it: which commit, what it was called,
/// how much of the repository it moved, and what it said about itself.
///
/// The message and not the diff. The diff the details pane fetches arrives
/// headerless, so this is the only thing that can say what the commit was called
/// — and the only thing that can say what the agent wrote under it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Commit {
    /// The full hash. Full rather than shortened, because a short hash is a
    /// display of one: what it takes to be unambiguous grows with the
    /// repository, and this is what git is asked with later.
    pub sha: String,

    /// The first line of the commit message.
    pub subject: String,

    /// How many files it touched.
    pub files: i64,

    pub insertions: i64,
    pub deletions: i64,

    /// What the commit says about itself: the body of its message with the
    /// trailing trailer block taken off, or `None` where nothing was left of it.
    ///
    /// The agent writes it, so it is markdown — a Diagram of the delta, then
    /// prose — and the details pane renders it above the diff. A bookkeeping
    /// commit carries none, and neither does any commit recorded before this was
    /// kept: `None` is the ordinary case rather than the damaged one.
    pub summary: Option<String>,
}

/// The commits table. It hangs off a Timeline Event, as a Capture does: a
/// commit is one Event's full self, and the Event is what a Timeline holds.
///
/// The Conversation is on the row as well as on the Event above it, which is
/// the one thing here that is written twice. It is what the unique index needs:
/// *this Conversation has this commit once* is the rule, and SQLite cannot
/// index a column that lives in another table.
pub(crate) async fn apply_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS commits (
             event_id        INTEGER PRIMARY KEY REFERENCES timeline_events(id),
             conversation_id INTEGER NOT NULL REFERENCES conversations(id),
             sha             TEXT NOT NULL,
             subject         TEXT NOT NULL,
             files           INTEGER NOT NULL,
             insertions      INTEGER NOT NULL,
             deletions       INTEGER NOT NULL,
             UNIQUE (conversation_id, sha)
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the commits table")?;

    // The Commit Summary, beside the commit rather than in it. A column on the
    // table above would need every row that is already there to grow one, and
    // there is no migration machinery here to do that with — where a table of its
    // own is simply absent for the commits recorded before it existed, which is
    // exactly what "that commit carries no summary" means.
    //
    // Keyed by the Event and not by the Conversation and sha: the commit row
    // above is what owns the identity, and this hangs off the same Event it does.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS commit_summaries (
             event_id INTEGER PRIMARY KEY REFERENCES timeline_events(id),
             summary  TEXT NOT NULL
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the commit summaries table")?;

    Ok(())
}

/// Put a commit on a Conversation's Timeline, and say which Event it became.
///
/// `None` is a commit that is already there, or a Conversation that is not:
/// neither is a failure, and both are what a sweep of a branch runs into every
/// time it looks. A branch is swept as a whole rather than as a list of what
/// has arrived since — see the server's `commits` module — so nearly every
/// commit this is offered has been recorded already.
///
/// One transaction, because an Event without its commit row is a Timeline
/// holding a Commit that cannot say what it changed — and [`Event::read`]
/// refuses one rather than drawing it as a commit of nothing.
pub async fn record_commit(
    pool: &SqlitePool,
    conversation_id: i64,
    commit: &Commit,
) -> Result<Option<i64>> {
    let mut tx = super::writing(pool, "recording a commit").await?;

    // Asked inside the transaction, so that the answer still holds when the
    // insert below acts on it. The unique index is what settles it either way:
    // this is how a commit already recorded comes back as `None` rather than as
    // a constraint violation nobody asked about.
    let recorded: Option<(i64,)> =
        sqlx::query_as("SELECT event_id FROM commits WHERE conversation_id = ? AND sha = ?")
            .bind(conversation_id)
            .bind(&commit.sha)
            .fetch_optional(&mut *tx)
            .await
            .with_context(|| {
                format!(
                    "looking for commit {} on Conversation {conversation_id}",
                    commit.sha
                )
            })?;

    if recorded.is_some() {
        return Ok(None);
    }

    // Selected from `conversations` rather than trusting the id, as every other
    // Event is written: SQLite enforces a foreign key only when asked to, and a
    // commit attributed to a Conversation that is not there would be on nobody's
    // Timeline.
    let event: Option<(i64,)> = sqlx::query_as(
        "INSERT INTO timeline_events (conversation_id, at, kind, body)
         SELECT id, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), ?, ''
         FROM conversations WHERE id = ?
         RETURNING id",
    )
    .bind(Event::Commit(commit.clone()).kind())
    .bind(conversation_id)
    .fetch_optional(&mut *tx)
    .await
    .with_context(|| {
        format!("putting a commit on the Timeline of Conversation {conversation_id}")
    })?;

    let Some((event_id,)) = event else {
        return Ok(None);
    };

    sqlx::query(
        "INSERT INTO commits
             (event_id, conversation_id, sha, subject, files, insertions, deletions)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(event_id)
    .bind(conversation_id)
    .bind(&commit.sha)
    .bind(&commit.subject)
    .bind(commit.files)
    .bind(commit.insertions)
    .bind(commit.deletions)
    .execute(&mut *tx)
    .await
    .with_context(|| format!("recording commit {} of Event {event_id}", commit.sha))?;

    // In the same transaction as the commit it belongs to, so that *exactly once*
    // covers the summary too: a commit is either on the Timeline with everything
    // it came with, or it is not there at all and the next sweep offers it again.
    if let Some(summary) = &commit.summary {
        sqlx::query("INSERT INTO commit_summaries (event_id, summary) VALUES (?, ?)")
            .bind(event_id)
            .bind(summary)
            .execute(&mut *tx)
            .await
            .with_context(|| format!("recording the summary of Event {event_id}"))?;
    }

    tx.commit().await.context("recording a commit")?;

    Ok(Some(event_id))
}

/// Which commits a Conversation already has on its Timeline.
///
/// What a sweep of the branch asks before it goes reading git: the shas it comes
/// back with are the ones there is nothing left to do about, and everything else
/// on the branch is a commit to describe and record. Asked as a set rather than
/// as "the last one recorded", because the last one is not a place in a branch —
/// a branch that was amended or reset has commits before its tip that this has
/// never seen.
pub async fn recorded_commits(pool: &SqlitePool, conversation_id: i64) -> Result<Vec<String>> {
    let rows: Vec<(String,)> = sqlx::query_as("SELECT sha FROM commits WHERE conversation_id = ?")
        .bind(conversation_id)
        .fetch_all(pool)
        .await
        .with_context(|| format!("reading the commits of Conversation {conversation_id}"))?;

    Ok(rows.into_iter().map(|(sha,)| sha).collect())
}

/// The commit one of a Conversation's Events is, or `None` where that
/// Conversation has no such Event.
///
/// The Conversation is part of the question rather than trusted from the path,
/// exactly as a Capture's is: a commit is reached through the Timeline it is
/// on, and an Event id belonging to another Conversation names nothing here.
pub async fn commit(
    pool: &SqlitePool,
    conversation_id: i64,
    event_id: i64,
) -> Result<Option<Commit>> {
    // Left-joined, because a commit with no summary is the ordinary commit
    // rather than a row that has gone missing: every commit recorded before
    // summaries were kept is one, and so is every bookkeeping commit since.
    let row: Option<(String, String, i64, i64, i64, Option<String>)> = sqlx::query_as(
        "SELECT c.sha, c.subject, c.files, c.insertions, c.deletions, s.summary
         FROM commits c
         LEFT JOIN commit_summaries s ON s.event_id = c.event_id
         WHERE c.event_id = ? AND c.conversation_id = ?",
    )
    .bind(event_id)
    .bind(conversation_id)
    .fetch_optional(pool)
    .await
    .with_context(|| format!("reading the commit of Event {event_id}"))?;

    Ok(row.map(
        |(sha, subject, files, insertions, deletions, summary)| Commit {
            sha,
            subject,
            files,
            insertions,
            deletions,
            summary,
        },
    ))
}

/// The Commit Summaries on a Conversation's Timeline, by the Event each one
/// belongs to.
///
/// Its own read rather than a column on the Timeline's own query, for the reason
/// a Capture's summaries are: that query is at the number of columns a tuple can
/// be read back as, and there is no position left to put one in. One more read
/// for the whole Timeline, and most Timelines answer it with nothing.
pub(crate) async fn summaries_on_timeline(
    pool: &SqlitePool,
    conversation_id: i64,
) -> Result<HashMap<i64, String>> {
    let rows: Vec<(i64, String)> = sqlx::query_as(
        "SELECT s.event_id, s.summary
         FROM commit_summaries s
         JOIN commits c ON c.event_id = s.event_id
         WHERE c.conversation_id = ?",
    )
    .bind(conversation_id)
    .fetch_all(pool)
    .await
    .with_context(|| format!("reading the commit summaries of Conversation {conversation_id}"))?;

    Ok(rows.into_iter().collect())
}
