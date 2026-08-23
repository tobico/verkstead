//! The pull request a Conversation's work ended up on, and the move into
//! Wrapping that recording one is.
//!
//! The finish step pushes the branch and opens the PR — the session does that
//! through its own `gh`, following the target repository's review process — and
//! Verkstead then asks the *host's* `gh` what was opened. What it records is
//! three short facts: the number, the title and the URL. That is what the
//! Timeline pins, and it is all that is worth keeping: the commit list and the
//! comments move for as long as the PR is open, so they are fetched when
//! somebody looks rather than written down here.
//!
//! Recording one is the move. A Conversation with a PR is a Conversation whose
//! work is being wrapped up, so the row, the state and the two Events are one
//! transaction: a Wrapping with no PR under it would be a Conversation waiting
//! on a review of nothing.
//!
//! One PR per Conversation, by the unique index. A Conversation is one branch
//! and a branch is one pull request — and the state check above the insert means
//! a second attempt at the same finish finds the move already made rather than
//! recording a second PR against it.

use std::collections::HashMap;

use anyhow::{Context, Result};
use sqlx::SqlitePool;

use super::conversations::{Event, Lifecycle, moved};

/// A pull request as its Timeline Event holds it: which one, what it is called,
/// and where it is.
///
/// Nothing about its state — draft or ready, open or merged, green or red. All
/// of that moves while the PR is open, and what moves is asked of `gh` when the
/// human looks rather than remembered here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullRequest {
    /// The number GitHub gave it, which is what everybody calls it by.
    pub number: i64,

    /// Its title, which is the feature name the finish step gave it.
    pub title: String,

    /// The whole URL, so the workbench can link out to it without building one
    /// out of a repository name it would have to guess at.
    pub url: String,
}

/// What became of recording one.
///
/// The mirror of [`super::Implementing`] one state along, and refused for the
/// same kind of reason: a Conversation that is neither implementing nor grilling
/// has nothing behind it that opens a pull request, so there is nothing here for
/// a PR to be the end of.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wrapping {
    /// Recorded: the PR, the state, and both Events on the Timeline.
    Started,

    /// It is neither implementing nor grilling, so this is not a Conversation
    /// with work to wrap up — it was aborted out from under the run, or it is
    /// wrapping already.
    NothingToWrap,

    /// There is no Conversation with that id.
    NoSuchConversation,
}

/// The pull requests table. It hangs off a Timeline Event, as a commit does: a
/// PR is one Event's full self, and the Event is what a Timeline holds.
///
/// The Conversation is on the row as well as on the Event above it, for the
/// commits table's reason: *one Conversation has one pull request* is the rule,
/// and SQLite cannot index a column that lives in another table.
pub(crate) async fn apply_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS pull_requests (
             event_id        INTEGER PRIMARY KEY REFERENCES timeline_events(id),
             conversation_id INTEGER NOT NULL REFERENCES conversations(id),
             number          INTEGER NOT NULL,
             title           TEXT NOT NULL,
             url             TEXT NOT NULL,
             UNIQUE (conversation_id)
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the pull requests table")?;

    Ok(())
}

/// Record the pull request the work was carried to, and move the Conversation
/// into Wrapping.
///
/// Two states get here, because two kinds of work open a pull request. A backlog
/// worked to empty is Implementing, and its finish step opened one. A roadmap is
/// still Grilling — the session that settled the work wrote the roadmap and
/// carried the branch on without ever leaving the grilling — so Wrapping is the
/// rung straight after it, and Implementing never happens on a Conversation whose
/// building is its Stages'.
///
/// One transaction, as every move is — and this one carries more than a move:
/// the PR Event, the row it hangs off, the state, and the move itself. What the
/// Timeline must never hold is one of them without the others.
///
/// The state is read inside the transaction so that the answer still holds when
/// the insert acts on it. That is what makes a second attempt at the same ending
/// safe: the first made the move, and the second finds a Conversation that has
/// nothing left to wrap.
pub async fn record_pull_request(
    pool: &SqlitePool,
    conversation_id: i64,
    pull_request: &PullRequest,
) -> Result<Wrapping> {
    let mut tx = pool.begin().await.context("recording a pull request")?;

    let row: Option<(String,)> = sqlx::query_as("SELECT state FROM conversations WHERE id = ?")
        .bind(conversation_id)
        .fetch_optional(&mut *tx)
        .await
        .with_context(|| format!("reading the state of Conversation {conversation_id}"))?;

    let Some((state,)) = row else {
        return Ok(Wrapping::NoSuchConversation);
    };

    if !matches!(
        Lifecycle::read(&state)?,
        Lifecycle::Implementing | Lifecycle::Grilling
    ) {
        return Ok(Wrapping::NothingToWrap);
    }

    let (event_id,): (i64,) = sqlx::query_as(
        "INSERT INTO timeline_events (conversation_id, at, kind, body)
         VALUES (?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), ?, '')
         RETURNING id",
    )
    .bind(conversation_id)
    .bind(Event::PullRequest(pull_request.clone()).kind())
    .fetch_one(&mut *tx)
    .await
    .with_context(|| {
        format!("putting a pull request on the Timeline of Conversation {conversation_id}")
    })?;

    sqlx::query(
        "INSERT INTO pull_requests (event_id, conversation_id, number, title, url)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(event_id)
    .bind(conversation_id)
    .bind(pull_request.number)
    .bind(&pull_request.title)
    .bind(&pull_request.url)
    .execute(&mut *tx)
    .await
    .with_context(|| {
        format!(
            "recording pull request {} of Event {event_id}",
            pull_request.number
        )
    })?;

    sqlx::query("UPDATE conversations SET state = ? WHERE id = ?")
        .bind(Lifecycle::Wrapping.stored())
        .bind(conversation_id)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("moving Conversation {conversation_id} to wrapping"))?;

    moved(&mut tx, conversation_id, Lifecycle::Wrapping).await?;

    tx.commit().await.context("recording a pull request")?;

    Ok(Wrapping::Started)
}

/// The pull request a Conversation's work is on, or `None` where it has none
/// yet.
///
/// What the wrap-up watchers ask before they ask GitHub anything: the number is
/// how a pull request is named on a command line, and a Conversation that is
/// wrapping up has exactly one — see [`record_pull_request`], which is what
/// makes both of those true at once.
pub async fn pull_request(pool: &SqlitePool, conversation_id: i64) -> Result<Option<PullRequest>> {
    let row: Option<(i64, String, String)> =
        sqlx::query_as("SELECT number, title, url FROM pull_requests WHERE conversation_id = ?")
            .bind(conversation_id)
            .fetch_optional(pool)
            .await
            .with_context(|| {
                format!("reading the pull request of Conversation {conversation_id}")
            })?;

    Ok(row.map(|(number, title, url)| PullRequest { number, title, url }))
}

/// The pull request on a Conversation's Timeline, against the Event it is.
///
/// A map of at most one, read on its own rather than joined into the Timeline
/// query, for the reason an Interruption's is: that query is already at the
/// sixteen columns a tuple can be read back as. This one is cheaper still —
/// there is one PR per Conversation and there is usually none.
pub(crate) async fn on_timeline(
    pool: &SqlitePool,
    conversation_id: i64,
) -> Result<HashMap<i64, PullRequest>> {
    let rows: Vec<(i64, i64, String, String)> = sqlx::query_as(
        "SELECT event_id, number, title, url FROM pull_requests WHERE conversation_id = ?",
    )
    .bind(conversation_id)
    .fetch_all(pool)
    .await
    .with_context(|| format!("reading the pull request of Conversation {conversation_id}"))?;

    Ok(rows
        .into_iter()
        .map(|(event_id, number, title, url)| (event_id, PullRequest { number, title, url }))
        .collect())
}
