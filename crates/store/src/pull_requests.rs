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
//!
//! Which is why a *second wrap* records nothing new. A Conversation whose review
//! split its findings out into a backlog leaves Wrapping to build them and
//! finishes again, and what its finish step opens is the pull request it already
//! had. So the record is reused rather than written twice: the move is made, and
//! the lifecycle moves either side of it are what tell the re-entry's story on
//! the Timeline.

use std::collections::HashMap;

use anyhow::{Context, Result, bail};
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

/// How a pull request's checks are getting on, taken all together.
///
/// One word for a whole suite, which is what a card has room to draw: any
/// check failed reads as failed, else anything still running reads as
/// running, else they passed. That order because it is the order a human
/// wants it in — a red check is the thing to go and look at, and a suite half
/// way through is not green yet.
///
/// There is no variant for *nobody has asked*, and there is no room for one:
/// not knowing is the absence of a row, in the same spirit the checks watcher
/// reads a `gh` that could not answer as neither green nor red.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rollup {
    /// Every check finished and none of them is red.
    Passed,

    /// Nothing is red, and something has not finished.
    Running,

    /// Something is red, whatever else is still going on.
    Failed,
}

impl Rollup {
    /// The word the column holds. Lowercase and spelled out, so a database
    /// opened by hand says something.
    fn stored(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Running => "running",
            Self::Failed => "failed",
        }
    }

    /// The one a stored word names. An unknown word is a database written by a
    /// Verkstead this one does not understand, exactly as an unknown lifecycle
    /// state is.
    fn read(word: &str) -> Result<Self> {
        Ok(match word {
            "passed" => Self::Passed,
            "running" => Self::Running,
            "failed" => Self::Failed,
            other => bail!("a pull request's checks are the unknown {other:?}"),
        })
    }
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
    /// with work to wrap up — it was closed out from under the run, or it is
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

    // How the checks on it are getting on, which is the one thing about a pull
    // request that is written down here and moves. The three facts above are
    // what was opened and never change; this is a reading of GitHub as it
    // stood the last time anything asked, kept because the card draws it and
    // the card is read long after anything is watching.
    //
    // Beside the pull request rather than on its row, which is where a fact
    // that moves belongs: the row hangs off a Timeline Event, and a Timeline
    // Event is a thing that happened.
    //
    // One row or none per Conversation, there being one pull request per
    // Conversation — and it survives a restart, which is the whole reason it
    // is written down rather than held in the watcher: the watcher stops when
    // the wrap-up is over, and a Done Conversation would otherwise lose its
    // icon the next time the server came up.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS pull_request_checks (
             conversation_id INTEGER PRIMARY KEY REFERENCES conversations(id),
             rollup          TEXT NOT NULL,
             at              TEXT NOT NULL
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the pull request checks table")?;

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
///
/// A *second wrap* is the other thing that gets here, and it is not that. The
/// Conversation left Wrapping to build a backlog its review split out — see
/// [`super::implement_again`] — so it is Implementing again and this is an ending
/// like any other, except that the branch is already on a pull request. There the
/// record is reused: one row, one Event, and the move made over the top of them.
pub async fn record_pull_request(
    pool: &SqlitePool,
    conversation_id: i64,
    pull_request: &PullRequest,
) -> Result<Wrapping> {
    let mut tx = super::writing(pool, "recording a pull request").await?;

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

    let recorded: Option<(i64,)> =
        sqlx::query_as("SELECT event_id FROM pull_requests WHERE conversation_id = ?")
            .bind(conversation_id)
            .fetch_optional(&mut *tx)
            .await
            .with_context(|| {
                format!("looking for the pull request Conversation {conversation_id} is already on")
            })?;

    if recorded.is_none() {
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
    }

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
/// query, for the reason a Capture summary's is: that query is already at the
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

/// Write down how the pull request's checks are, and say whether that is news.
///
/// Called on every poll of the checks watcher, which is every half minute for as
/// long as a Conversation is wrapping up — so what it answers is *did this
/// change anything*, and the caller Nudges the open pages on the strength of it.
/// A suite that is still running is the same word half an hour running, and a
/// page told about it every thirty seconds would be a page re-reading a Timeline
/// nothing had happened on.
///
/// Written over rather than appended to: this is how the checks are now, and
/// what they were an hour ago is what the runs on GitHub are for.
pub async fn record_check_rollup(
    pool: &SqlitePool,
    conversation_id: i64,
    rollup: Rollup,
) -> Result<bool> {
    let mut tx = super::writing(pool, "recording how a pull request's checks are").await?;

    let row: Option<(String,)> =
        sqlx::query_as("SELECT rollup FROM pull_request_checks WHERE conversation_id = ?")
            .bind(conversation_id)
            .fetch_optional(&mut *tx)
            .await
            .with_context(|| format!("reading how Conversation {conversation_id}'s checks were"))?;

    let before = row.map(|(word,)| Rollup::read(&word)).transpose()?;

    sqlx::query(
        "INSERT INTO pull_request_checks (conversation_id, rollup, at)
         VALUES (?, ?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
         ON CONFLICT (conversation_id)
         DO UPDATE SET rollup = excluded.rollup, at = excluded.at",
    )
    .bind(conversation_id)
    .bind(rollup.stored())
    .execute(&mut *tx)
    .await
    .with_context(|| format!("recording how Conversation {conversation_id}'s checks are"))?;

    tx.commit()
        .await
        .context("recording how a pull request's checks are")?;

    Ok(before != Some(rollup))
}

/// And how they were the last time anything asked, or `None` where nothing has.
///
/// What the Conversation view carries to the card. It may be stale, and on a
/// Conversation nothing is watching any more it will be: the watching stops when
/// the wrap-up is over, and what is drawn after that is the last thing anybody
/// asked GitHub — which is a card an hour behind rather than a card that is
/// wrong.
pub async fn check_rollup(pool: &SqlitePool, conversation_id: i64) -> Result<Option<Rollup>> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT rollup FROM pull_request_checks WHERE conversation_id = ?")
            .bind(conversation_id)
            .fetch_optional(pool)
            .await
            .with_context(|| format!("reading how Conversation {conversation_id}'s checks are"))?;

    row.map(|(word,)| Rollup::read(&word)).transpose()
}
