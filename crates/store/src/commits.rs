//! The commits a session lands on a Conversation's branch, as Timeline Events.
//!
//! A commit is the only visible product of unattended execution: the sessions
//! run without being driven, and what they leave behind is history on a branch.
//! So each one becomes an Event — what it changed, in a line — and the details
//! pane shows its diff.
//!
//! The summary is held here rather than read back out of git every time a page
//! looks, for the reason a transcript's is: every open page reads the whole
//! Timeline, and a repository asked once per commit per read would be a git
//! process per row of it. The diff itself is *not* held, and that is the other
//! half of the same judgement — it is megabytes the Timeline never shows, and
//! the repository already has it.
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

use anyhow::{Context, Result};
use sqlx::SqlitePool;

use super::conversations::Event;

/// A commit as its Timeline Event holds it: which commit, what it was called,
/// and how much of the repository it moved.
///
/// The subject alone and not the whole message. The Timeline gives a commit one
/// line, and the rest of the message is in the diff the details pane fetches —
/// which is also why the message has to come from here at all: the diff arrives
/// headerless, so the Event is the only thing that can say what the commit was
/// called.
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
}

/// The commits table. It hangs off a Timeline Event, as a transcript does: a
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
    let mut tx = pool.begin().await.context("recording a commit")?;

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
/// exactly as a transcript's is: a commit is reached through the Timeline it is
/// on, and an Event id belonging to another Conversation names nothing here.
pub async fn commit(
    pool: &SqlitePool,
    conversation_id: i64,
    event_id: i64,
) -> Result<Option<Commit>> {
    let row: Option<(String, String, i64, i64, i64)> = sqlx::query_as(
        "SELECT sha, subject, files, insertions, deletions
         FROM commits
         WHERE event_id = ? AND conversation_id = ?",
    )
    .bind(event_id)
    .bind(conversation_id)
    .fetch_optional(pool)
    .await
    .with_context(|| format!("reading the commit of Event {event_id}"))?;

    Ok(
        row.map(|(sha, subject, files, insertions, deletions)| Commit {
            sha,
            subject,
            files,
            insertions,
            deletions,
        }),
    )
}
