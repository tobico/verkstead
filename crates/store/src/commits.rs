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
//! One commit per Conversation per Repo per sha, by the unique index. That is
//! what makes *exactly once* a fact about the database rather than a promise
//! made by whatever is watching the branch: a sweep that runs twice over the
//! same commit records it once, whether the second sweep is a poll that
//! overlapped, a session restarting, or a server that came back up.
//!
//! The Repo is part of that identity because one Conversation is swept in more
//! than one repository: its own, and a branch per read-write companion. Two
//! repositories are two histories, and a sha says nothing across them.

use std::collections::HashMap;

use anyhow::{Context, Result};
use sqlx::SqlitePool;

use super::Repo;
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

    /// What the Repo this commit landed in is called, where that is not the
    /// Conversation's own — the label the Timeline card and the details pane
    /// draw.
    ///
    /// Read back rather than written: the row holds the Repo's id, which
    /// [`record_commit`] is told separately, and this is the name a reader
    /// wants. `None` is the work's own repository, and it draws unlabeled — an
    /// unlabeled card means the work's own repo, and the label earns its place
    /// when repos mix.
    pub repo: Option<String>,

    /// Whether it is a merge: the commit a resolution session leaves behind
    /// where it brought the base branch in and settled the conflicts.
    ///
    /// Kept beside the commit rather than asked of git whenever a page looks,
    /// for the reason the subject and the counts are kept: a Timeline is read
    /// every time an open page hears the world moved, and a repository asked
    /// once per row would be a git process per commit of it.
    ///
    /// A merge reads as an ordinary small commit otherwise — what it carries is
    /// the hunks the agent resolved, which is a handful of lines — so the card
    /// says which it is. `false` is that ordinary commit, and it is what every
    /// commit recorded before this was kept comes back as: the column's own
    /// default, and the only thing that can be said of a commit nobody asked
    /// git about.
    pub merge: bool,
}

/// The commits table. It hangs off a Timeline Event, as a Capture does: a
/// commit is one Event's full self, and the Event is what a Timeline holds.
///
/// The Conversation is on the row as well as on the Event above it, which is
/// the one thing here that is written twice. It is what the unique index needs:
/// *this Conversation has this commit once in this Repo* is the rule, and
/// SQLite cannot index a column that lives in another table.
///
/// The Repo is the third column of that index, and a database written before
/// Verkstead swept more than one repository has neither it nor the column. Which
/// is [`super::migrations`]'s to put right as the database opens rather than
/// this function's: the constraint is declared inline, so it is the table itself
/// that has to be rebuilt, and that is not something a `CREATE TABLE IF NOT
/// EXISTS` can reach.
pub(crate) async fn apply_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS commits (
             event_id        INTEGER PRIMARY KEY REFERENCES timeline_events(id),
             conversation_id INTEGER NOT NULL REFERENCES conversations(id),
             repo_id         INTEGER NOT NULL REFERENCES repos(id),
             sha             TEXT NOT NULL,
             subject         TEXT NOT NULL,
             files           INTEGER NOT NULL,
             insertions      INTEGER NOT NULL,
             deletions       INTEGER NOT NULL,
             merge           INTEGER NOT NULL DEFAULT 0,
             UNIQUE (conversation_id, repo_id, sha)
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the commits table")?;

    // The Commit Summary, beside the commit rather than in it. A column on the
    // table above would have needed every row already there to grow one, and
    // there was no migration machinery here to do that with when this arrived —
    // where a table of its own is simply absent for the commits recorded before
    // it existed, which is exactly what "that commit carries no summary" means.
    //
    // There is machinery now, and it is how the merge flag above became a
    // column — see [`super::migrations`]. Moving the summary into the table
    // would be a rewrite of every row to say what an absent one already says.
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
/// `repo_id` is the registered Repo the sweep read it out of: the Conversation's
/// own, or one of its read-write companions'. It is part of the commit's
/// identity rather than a note about it — see [`apply_schema`] — so it is asked
/// for rather than taken off [`Commit::repo`], which is a name for reading.
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
///
/// And an *immediate* one, alone among the writes here, because this is the one
/// several writers reach at the same moment: a session ending is every one of
/// its branches being swept a last time at once, one watcher per repository. A
/// transaction that opens deferred takes a read lock for the look below and then
/// asks to upgrade it, and SQLite refuses an upgrade it cannot wait on rather
/// than waiting — where a transaction that says up front that it is going to
/// write is one the other writer's busy handler simply waits for. The last
/// commit of a session is exactly what would be lost otherwise: there is no next
/// sweep to try again.
pub async fn record_commit(
    pool: &SqlitePool,
    conversation_id: i64,
    repo_id: i64,
    commit: &Commit,
) -> Result<Option<i64>> {
    let mut tx = super::writing(pool, "recording a commit").await?;

    // Asked inside the transaction, so that the answer still holds when the
    // insert below acts on it. The unique index is what settles it either way:
    // this is how a commit already recorded comes back as `None` rather than as
    // a constraint violation nobody asked about.
    let recorded: Option<(i64,)> = sqlx::query_as(
        "SELECT event_id FROM commits
         WHERE conversation_id = ? AND repo_id = ? AND sha = ?",
    )
    .bind(conversation_id)
    .bind(repo_id)
    .bind(&commit.sha)
    .fetch_optional(&mut *tx)
    .await
    .with_context(|| {
        format!(
            "looking for commit {} of Repo {repo_id} on Conversation {conversation_id}",
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
             (event_id, conversation_id, repo_id, sha, subject, files, insertions,
              deletions, merge)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(event_id)
    .bind(conversation_id)
    .bind(repo_id)
    .bind(&commit.sha)
    .bind(&commit.subject)
    .bind(commit.files)
    .bind(commit.insertions)
    .bind(commit.deletions)
    .bind(commit.merge)
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

/// Take a commit off a Conversation's Timeline, and say which Event it was.
///
/// The other half of a sweep. A branch whose commits have been rewritten —
/// rebased to settle a conflict, or amended — carries the same work under new
/// shas, and a Timeline that only ever gained would hold both: the work twice
/// over, half of it under shas the repository no longer has. So a sweep that
/// finds a recorded commit the branch has stopped carrying forgets it, and
/// records the rewritten one in its place — see the server's `commits` module
/// for what decides which those are.
///
/// `None` is a commit that is not there, which is the same answer
/// [`record_commit`] gives for one that already is: neither is a failure, and
/// both are what a sweep run twice over the same branch runs into.
///
/// One transaction, for the reason recording is one: the Event, the commit row
/// and the Commit Summary are one thing on the Timeline, and an Event left
/// behind with no commit row under it is one [`Event::read`] refuses rather
/// than draws. And an immediate one, the same way and for the same reason —
/// this runs in the sweep that records, against a database a session ending is
/// having every one of its branches swept in at once.
pub async fn forget_commit(
    pool: &SqlitePool,
    conversation_id: i64,
    repo_id: i64,
    sha: &str,
) -> Result<Option<i64>> {
    let mut tx = super::writing(pool, "forgetting a commit").await?;

    // Asked inside the transaction, so that the Event it finds is the Event the
    // deletes below act on. The Conversation and the Repo are asked along with
    // the sha because the three of them together are the commit's identity: a
    // sha on its own names a commit on somebody else's Timeline just as well.
    let recorded: Option<(i64,)> = sqlx::query_as(
        "SELECT event_id FROM commits
         WHERE conversation_id = ? AND repo_id = ? AND sha = ?",
    )
    .bind(conversation_id)
    .bind(repo_id)
    .bind(sha)
    .fetch_optional(&mut *tx)
    .await
    .with_context(|| {
        format!("looking for commit {sha} of Repo {repo_id} on Conversation {conversation_id}")
    })?;

    let Some((event_id,)) = recorded else {
        return Ok(None);
    };

    // What hangs off the Event first, then the commit row, then the Event
    // itself: each of the three points at the one above it, so this is the order
    // that never leaves a row naming something that has gone.
    sqlx::query("DELETE FROM commit_summaries WHERE event_id = ?")
        .bind(event_id)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("forgetting the summary of Event {event_id}"))?;

    sqlx::query("DELETE FROM commits WHERE event_id = ?")
        .bind(event_id)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("forgetting commit {sha} of Event {event_id}"))?;

    sqlx::query("DELETE FROM timeline_events WHERE id = ?")
        .bind(event_id)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("taking Event {event_id} off the Timeline"))?;

    tx.commit().await.context("forgetting a commit")?;

    Ok(Some(event_id))
}

/// Which commits a Conversation already has on its Timeline out of one Repo.
///
/// What a sweep of a branch asks before it goes reading git, and it is both of
/// the sweep's questions: everything on the branch that is not among these is a
/// commit to describe and record, and everything among these the branch no
/// longer carries is a commit to forget — see [`forget_commit`]. Asked as a set
/// rather than as "the last one recorded", because the last one is not a place
/// in a branch: one that was amended or rebased carries neither the commits this
/// has seen nor them in the order it saw them.
///
/// Per Repo and not per Conversation, because that is what a sweep is: one
/// watcher reads one branch of one repository, and the commits on another repo's
/// branch are nothing it could do anything about.
pub async fn recorded_commits(
    pool: &SqlitePool,
    conversation_id: i64,
    repo_id: i64,
) -> Result<Vec<String>> {
    let rows: Vec<(String,)> =
        sqlx::query_as("SELECT sha FROM commits WHERE conversation_id = ? AND repo_id = ?")
            .bind(conversation_id)
            .bind(repo_id)
            .fetch_all(pool)
            .await
            .with_context(|| {
                format!("reading the commits of Repo {repo_id} on Conversation {conversation_id}")
            })?;

    Ok(rows.into_iter().map(|(sha,)| sha).collect())
}

/// Where a Conversation's commits stand: the newest commit Event on its
/// Timeline, in every repository it is being worked in, and `0` where it has
/// none.
///
/// What the runner reads before a session and again after it, to say whether the
/// session committed anything at all. Across the repositories rather than per
/// Repo, because that is the question: a session that committed only in a
/// read-write companion has done work, and a run told otherwise would call it a
/// session that came to nothing.
///
/// **The newest Event rather than how many there are**, because a sweep
/// subtracts as well as adds — see [`forget_commit`]. A branch that was rebased
/// or amended carries the same work under new shas, so the sweep forgets as many
/// commits as it records and a count comes back to where it started: a
/// resolution session on a Repo that rebases would read as one that committed
/// nothing, and the runner would go on waiting for a number that never moves.
///
/// An Event id moves under that, because `timeline_events` autoincrements and so
/// never hands an id out twice: whatever a sweep records is numbered above
/// everything the Conversation was already holding, and a rewrite records. It is
/// a marker rather than a number to do arithmetic on — every reader holds one
/// from before the session and compares it with one from after, and nothing
/// counts anything.
pub async fn commits_landed(pool: &SqlitePool, conversation_id: i64) -> Result<i64> {
    let (landed,): (Option<i64>,) =
        sqlx::query_as("SELECT max(event_id) FROM commits WHERE conversation_id = ?")
            .bind(conversation_id)
            .fetch_one(pool)
            .await
            .with_context(|| {
                format!("reading where the commits of Conversation {conversation_id} stand")
            })?;

    Ok(landed.unwrap_or(0))
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
    /// The columns in the order the query below selects them.
    type Row = (
        String,
        String,
        i64,
        i64,
        i64,
        Option<String>,
        Option<String>,
        bool,
    );

    // The summary left-joined, because a commit with no summary is the ordinary
    // commit rather than a row that has gone missing: every commit recorded
    // before summaries were kept is one, and so is every bookkeeping commit
    // since.
    //
    // And the Repo left-joined too, on the condition that says what the label is
    // for: it is joined only where the commit's Repo is not the Conversation's
    // own, so the name comes back for a companion's commit and nothing comes
    // back for the work's own. A Repo taken off the registry is nothing to draw
    // either, which is the same unlabeled card.
    let row: Option<Row> = sqlx::query_as(
        "SELECT c.sha, c.subject, c.files, c.insertions, c.deletions, s.summary, r.name,
                c.merge
         FROM commits c
         JOIN conversations v ON v.id = c.conversation_id
         LEFT JOIN commit_summaries s ON s.event_id = c.event_id
         LEFT JOIN repos r ON r.id = c.repo_id AND r.id <> v.repo_id
         WHERE c.event_id = ? AND c.conversation_id = ?",
    )
    .bind(event_id)
    .bind(conversation_id)
    .fetch_optional(pool)
    .await
    .with_context(|| format!("reading the commit of Event {event_id}"))?;

    Ok(row.map(
        |(sha, subject, files, insertions, deletions, summary, repo, merge)| Commit {
            sha,
            subject,
            files,
            insertions,
            deletions,
            summary,
            repo,
            merge,
        },
    ))
}

/// Which registered Repo one of a Conversation's commits is in.
///
/// What the details pane reads its diff out of. The Conversation's own
/// repository is the answer for most commits and the wrong answer for a
/// companion's, so it is the commit that is asked rather than the Conversation:
/// the row says which Repo the sweep read it out of, and that is where the
/// patch is.
///
/// `None` where the Conversation has no such Event, and where the Repo it names
/// is no longer registered. Both are the same thing to whoever asked — there is
/// nothing left that can say what this commit changed.
pub async fn commit_repo(
    pool: &SqlitePool,
    conversation_id: i64,
    event_id: i64,
) -> Result<Option<Repo>> {
    let row: Option<(i64, String, String, String)> = sqlx::query_as(
        "SELECT r.id, r.path, r.name, r.default_branch
         FROM commits c
         JOIN repos r ON r.id = c.repo_id
         WHERE c.event_id = ? AND c.conversation_id = ?",
    )
    .bind(event_id)
    .bind(conversation_id)
    .fetch_optional(pool)
    .await
    .with_context(|| format!("reading the repository of the commit of Event {event_id}"))?;

    Ok(row.map(|(id, path, name, default_branch)| Repo {
        id,
        path: std::path::PathBuf::from(path),
        name,
        default_branch,
    }))
}

/// What a commit on a Conversation's Timeline carries that the Timeline's own
/// query has no column left to hold.
///
/// Two things rather than a summary alone, and one read rather than two: that
/// query is at the number of columns a tuple can be read back as, so whatever
/// arrives after it is a read of its own — and a second read for a boolean
/// beside a table this one is already joining would be a query per Timeline for
/// nothing.
#[derive(Debug, Clone, Default)]
pub(crate) struct BesideCommit {
    /// The Commit Summary, or `None` where the commit carried none — which is
    /// every bookkeeping commit and every commit recorded before summaries were
    /// kept.
    pub summary: Option<String>,

    /// Whether the commit is a merge — see [`Commit::merge`].
    pub merge: bool,
}

/// What each commit on a Conversation's Timeline carries beside its row, by the
/// Event it belongs to.
///
/// Its own read rather than columns on the Timeline's own query, for the reason
/// a Capture's summaries are: that query is at the number of columns a tuple can
/// be read back as, and there is no position left to put one in. One more read
/// for the whole Timeline, and a Timeline with no commit on it answers it with
/// nothing.
///
/// Driven off the commits rather than off the summaries, because the merge flag
/// is a column of the commit and every commit has one: the summary is what is
/// left-joined, and most of them come back without.
pub(crate) async fn beside_commits_on_timeline(
    pool: &SqlitePool,
    conversation_id: i64,
) -> Result<HashMap<i64, BesideCommit>> {
    let rows: Vec<(i64, Option<String>, bool)> = sqlx::query_as(
        "SELECT c.event_id, s.summary, c.merge
         FROM commits c
         LEFT JOIN commit_summaries s ON s.event_id = c.event_id
         WHERE c.conversation_id = ?",
    )
    .bind(conversation_id)
    .fetch_all(pool)
    .await
    .with_context(|| {
        format!("reading the commits on the Timeline of Conversation {conversation_id}")
    })?;

    Ok(rows
        .into_iter()
        .map(|(event_id, summary, merge)| (event_id, BesideCommit { summary, merge }))
        .collect())
}
