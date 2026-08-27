//! The pull requests a Conversation's work ended up on, and the move into
//! Wrapping that recording one is.
//!
//! The finish step pushes the branches and opens the pull requests — the session
//! does that through its own `gh`, following each target repository's review
//! process — and Verkstead then asks the *host's* `gh` what was opened. What it
//! records is three short facts per pull request: the number, the title and the
//! URL. That is what the Timeline pins, and it is all that is worth keeping: the
//! commit list and the comments move for as long as a PR is open, so they are
//! fetched when somebody looks rather than written down here.
//!
//! Recording one is the move. A Conversation with a PR is a Conversation whose
//! work is being wrapped up, so the row, the state and the two Events are one
//! transaction: a Wrapping with no PR under it would be a Conversation waiting
//! on a review of nothing.
//!
//! One PR per Conversation per Repo, by the unique index. A Conversation is one
//! branch per repository and a branch is one pull request — so a Conversation
//! working alongside read-write companions ends on several, one each, and
//! recording the second is the same wrap-up learning about another pull request
//! rather than a second move.
//!
//! The Repo is part of that identity for the commits table's reason: two
//! repositories are two sets of numbers, and `#41` says nothing across them.
//!
//! Which is also why a PR recorded against a repository that already has one
//! records nothing new. It is what makes a second attempt at the same ending
//! safe, and it is what a *second wrap* lands on: a Conversation whose review
//! split its findings out into a backlog leaves Wrapping to build them and
//! finishes again, and what its finish step opens is the pull requests it
//! already had. So each record is reused rather than written twice, and the
//! lifecycle moves either side of it are what tell the re-entry's story on the
//! Timeline.

use std::collections::HashMap;

use anyhow::{Context, Result};
use sqlx::{Sqlite, SqlitePool, Transaction};

use super::Repo;
use super::conversations::{Event, Lifecycle, moved};

/// A pull request as its Timeline Event holds it: which one, what it is called,
/// where it is, and which repository it was opened in.
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

    /// What the Repo it was opened in is called, where that is not the
    /// Conversation's own — the label the pinned card draws.
    ///
    /// Read back rather than written, exactly as [`super::Commit::repo`] is: the
    /// row holds the Repo's id, which [`record_pull_request`] is told
    /// separately, and this is the name a reader wants. `None` is the work's own
    /// repository, and it draws unlabeled — an unlabeled card means the work's
    /// own repo, and the label earns its place when repos mix.
    pub repo: Option<String>,
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
/// commits table's reason: *one Conversation has one pull request per Repo* is
/// the rule, and SQLite cannot index a column that lives in another table.
///
/// The Repo is the second column of that index, and a database written before a
/// Conversation could end on more than one pull request has neither it nor the
/// column — it has `UNIQUE (conversation_id)` instead, which is the old rule.
/// Which is [`super::migrations`]'s to put right as the database opens rather
/// than this function's: the constraint is declared inline, so it is the table
/// itself that has to be rebuilt, and that is not something a `CREATE TABLE IF
/// NOT EXISTS` can reach.
pub(crate) async fn apply_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS pull_requests (
             event_id        INTEGER PRIMARY KEY REFERENCES timeline_events(id),
             conversation_id INTEGER NOT NULL REFERENCES conversations(id),
             repo_id         INTEGER NOT NULL REFERENCES repos(id),
             number          INTEGER NOT NULL,
             title           TEXT NOT NULL,
             url             TEXT NOT NULL,
             UNIQUE (conversation_id, repo_id)
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the pull requests table")?;

    Ok(())
}

/// Record a pull request the work was carried to, and move the Conversation
/// into Wrapping.
///
/// `repo_id` is the registered Repo the pull request was opened in: the
/// Conversation's own, or one of its read-write companions'. It is part of the
/// pull request's identity rather than a note about it — see [`apply_schema`] —
/// so it is asked for rather than taken off [`PullRequest::repo`], which is a
/// name for reading.
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
/// Which is why a companion's pull request does not come through here. A
/// Conversation that is already Wrapping has nothing left to wrap, so a second
/// PR arriving a moment behind the work's own would be refused — see
/// [`record_another_pull_request`], which is this same row without the move over
/// the top of it.
///
/// A *second wrap* is the other thing that gets here, and it is not that. The
/// Conversation left Wrapping to build a backlog its review split out — see
/// [`super::implement_again`] — so it is Implementing again and this is an ending
/// like any other, except that the branch is already on a pull request. There the
/// record is reused: the row it has, one Event, and the move made over the top
/// of them.
pub async fn record_pull_request(
    pool: &SqlitePool,
    conversation_id: i64,
    repo_id: i64,
    pull_request: &PullRequest,
) -> Result<Wrapping> {
    let mut tx = super::begin_writing(pool)
        .await
        .context("recording a pull request")?;

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

    record(&mut tx, conversation_id, repo_id, pull_request).await?;

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

/// Record another pull request of a wrap-up that is already under way, and say
/// whether there was a Conversation to record it against.
///
/// The same row [`record_pull_request`] writes, without the move: the
/// Conversation is Wrapping already, and this is that wrap-up learning about
/// another repository's pull request rather than a second ending. Which is the
/// whole difference between the two — a Conversation's own repository is what
/// moves it, and its companions are what the move then covers.
///
/// `false` where there is no Conversation with that id. Nothing else is refused
/// for: a pull request recorded against a repository that already has one reuses
/// the row it has, which is what makes a discovery run twice do nothing the
/// second time.
pub async fn record_another_pull_request(
    pool: &SqlitePool,
    conversation_id: i64,
    repo_id: i64,
    pull_request: &PullRequest,
) -> Result<bool> {
    let mut tx = super::begin_writing(pool)
        .await
        .context("recording another pull request")?;

    let there: Option<(i64,)> = sqlx::query_as("SELECT id FROM conversations WHERE id = ?")
        .bind(conversation_id)
        .fetch_optional(&mut *tx)
        .await
        .with_context(|| format!("looking for Conversation {conversation_id}"))?;

    if there.is_none() {
        return Ok(false);
    }

    record(&mut tx, conversation_id, repo_id, pull_request).await?;

    tx.commit()
        .await
        .context("recording another pull request")?;

    Ok(true)
}

/// The Event and the row under it, or nothing at all where that repository
/// already has a pull request on this Conversation.
///
/// Shared by the two above, because the row is the same row: what differs is
/// only whether the Conversation moves over the top of it.
///
/// The look is inside the caller's transaction, so that the answer still holds
/// when the insert acts on it. The unique index is what settles it either way.
async fn record(
    tx: &mut Transaction<'static, Sqlite>,
    conversation_id: i64,
    repo_id: i64,
    pull_request: &PullRequest,
) -> Result<()> {
    let recorded: Option<(i64,)> = sqlx::query_as(
        "SELECT event_id FROM pull_requests WHERE conversation_id = ? AND repo_id = ?",
    )
    .bind(conversation_id)
    .bind(repo_id)
    .fetch_optional(&mut **tx)
    .await
    .with_context(|| {
        format!(
            "looking for the pull request Conversation {conversation_id} is already on in Repo \
             {repo_id}"
        )
    })?;

    if recorded.is_some() {
        return Ok(());
    }

    let (event_id,): (i64,) = sqlx::query_as(
        "INSERT INTO timeline_events (conversation_id, at, kind, body)
         VALUES (?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), ?, '')
         RETURNING id",
    )
    .bind(conversation_id)
    .bind(Event::PullRequest(pull_request.clone()).kind())
    .fetch_one(&mut **tx)
    .await
    .with_context(|| {
        format!("putting a pull request on the Timeline of Conversation {conversation_id}")
    })?;

    sqlx::query(
        "INSERT INTO pull_requests (event_id, conversation_id, repo_id, number, title, url)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(event_id)
    .bind(conversation_id)
    .bind(repo_id)
    .bind(pull_request.number)
    .bind(&pull_request.title)
    .bind(&pull_request.url)
    .execute(&mut **tx)
    .await
    .with_context(|| {
        format!(
            "recording pull request {} of Event {event_id}",
            pull_request.number
        )
    })?;

    Ok(())
}

/// The pull request a Conversation's work is on in one Repo, or `None` where
/// that repository has none yet.
///
/// Per Repo and not per Conversation, because a number is a fact about a
/// repository: a Conversation working alongside read-write companions ends on
/// one pull request each, and `#41` in one of them is a different pull request
/// from `#41` in another.
///
/// What a wrap-up's watchers ask before they ask GitHub anything — the number is
/// how a pull request is named on a command line — and what the steer into
/// Wrapping asks about the Conversation's own repository, that being the one a
/// wrap-up is defined by.
pub async fn pull_request(
    pool: &SqlitePool,
    conversation_id: i64,
    repo_id: i64,
) -> Result<Option<PullRequest>> {
    let row: Option<(i64, String, String, Option<String>)> = sqlx::query_as(
        "SELECT p.number, p.title, p.url, r.name
         FROM pull_requests p
         JOIN conversations v ON v.id = p.conversation_id
         LEFT JOIN repos r ON r.id = p.repo_id AND r.id <> v.repo_id
         WHERE p.conversation_id = ? AND p.repo_id = ?",
    )
    .bind(conversation_id)
    .bind(repo_id)
    .fetch_optional(pool)
    .await
    .with_context(|| {
        format!("reading the pull request of Repo {repo_id} on Conversation {conversation_id}")
    })?;

    Ok(row.map(|(number, title, url, repo)| PullRequest {
        number,
        title,
        url,
        repo,
    }))
}

/// Which registered Repo one of a Conversation's pull requests was opened in.
///
/// What the details pane asks GitHub in. The Conversation's own repository is
/// the answer for most pull requests and the wrong answer for a companion's —
/// a number means something else there, or nothing — so it is the pull request
/// that is asked rather than the Conversation.
///
/// The Conversation is part of the question rather than trusted from the path,
/// exactly as [`super::commit_repo`]'s is: a pull request is reached through the
/// Timeline it is on, and an Event id belonging to another Conversation names
/// nothing here.
///
/// `None` where the Conversation has no such Event, and where the Repo it names
/// is no longer registered. Both are the same thing to whoever asked — there is
/// nothing left that can say where this pull request is.
pub async fn pull_request_repo(
    pool: &SqlitePool,
    conversation_id: i64,
    event_id: i64,
) -> Result<Option<Repo>> {
    let row: Option<(i64, String, String, String)> = sqlx::query_as(
        "SELECT r.id, r.path, r.name, r.default_branch
         FROM pull_requests p
         JOIN repos r ON r.id = p.repo_id
         WHERE p.event_id = ? AND p.conversation_id = ?",
    )
    .bind(event_id)
    .bind(conversation_id)
    .fetch_optional(pool)
    .await
    .with_context(|| format!("reading the repository of the pull request of Event {event_id}"))?;

    Ok(row.map(|(id, path, name, default_branch)| Repo {
        id,
        path: std::path::PathBuf::from(path),
        name,
        default_branch,
    }))
}

/// The pull requests on a Conversation's Timeline, against the Events they are.
///
/// A map read on its own rather than joined into the Timeline query, for the
/// reason a Capture summary's is: that query is already at the sixteen columns a
/// tuple can be read back as. This one is cheap regardless — a Conversation has
/// one pull request per repository it was worked in, and usually none at all.
///
/// The Repo is left-joined on the condition that says what the label is for: it
/// is joined only where the pull request's Repo is not the Conversation's own,
/// so the name comes back for a companion's and nothing comes back for the work's
/// own. A Repo taken off the registry is nothing to draw either, which is the
/// same unlabeled card.
pub(crate) async fn on_timeline(
    pool: &SqlitePool,
    conversation_id: i64,
) -> Result<HashMap<i64, PullRequest>> {
    let rows: Vec<(i64, i64, String, String, Option<String>)> = sqlx::query_as(
        "SELECT p.event_id, p.number, p.title, p.url, r.name
         FROM pull_requests p
         JOIN conversations v ON v.id = p.conversation_id
         LEFT JOIN repos r ON r.id = p.repo_id AND r.id <> v.repo_id
         WHERE p.conversation_id = ?",
    )
    .bind(conversation_id)
    .fetch_all(pool)
    .await
    .with_context(|| format!("reading the pull requests of Conversation {conversation_id}"))?;

    Ok(rows
        .into_iter()
        .map(|(event_id, number, title, url, repo)| {
            (
                event_id,
                PullRequest {
                    number,
                    title,
                    url,
                    repo,
                },
            )
        })
        .collect())
}
