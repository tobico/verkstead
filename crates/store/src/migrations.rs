//! The one-time rewrites a database written by an older Verkstead needs, run as
//! it is opened.
//!
//! Every other schema statement here is a `CREATE TABLE IF NOT EXISTS`: the
//! shape is declared, and a database that already has it is left alone. What
//! lives in this module is the other kind — a rewrite of rows that are already
//! there, run once because the thing they described has been replaced by
//! something else.
//!
//! And three of them are about a Conversation ending on more than one pull
//! request: what a fix session was counted against, what a settled suite was
//! about, and which pull request a comment somebody was sent to deal with was
//! left on, are all a pull request's rather than a Conversation's now.
//!
//! Four of them are a column arriving rather than rows moving between tables —
//! the Review role's Profile, the branch name somebody settled on, whether a
//! branch is still waiting to be named, and whether a session is idling on a
//! stored ask — which is the same kind of one-time rewrite: the rows already
//! there are given the value that says what was true of them before the column
//! existed.
//!
//! Each is written to be safe against a database that has already had it, and
//! what says whether there is anything to do is the presence of what it
//! rewrites rather than a version number kept somewhere. So a database opened
//! twice is rewritten once, and one made from scratch this morning has nothing
//! here to do at all.

use anyhow::{Context, Result};
use sqlx::SqlitePool;

use super::{Lifecycle, stops::Decision};

/// Run whatever this database still needs, in the order it needs them.
pub(crate) async fn apply(pool: &SqlitePool) -> Result<()> {
    stops_recorded_the_old_way(pool).await?;
    conversations_that_were_aborted(pool).await?;
    commits_that_named_no_repo(pool).await?;
    pull_requests_that_named_no_repo(pool).await?;
    fix_attempts_that_named_no_repo(pool).await?;
    settlements_that_named_no_pull_request(pool).await?;
    addressed_comments_that_named_no_pull_request(pool).await?;
    conversations_that_had_no_review_profile(pool).await?;
    conversations_whose_branch_name_had_no_owner(pool).await?;
    conversations_whose_branch_nobody_was_naming(pool).await?;
    stored_asks_nobody_was_idling_on(pool).await
}

/// Give every stored ask written before there were two kinds of one the column
/// that says whether a session is idling on it.
///
/// Nobody is: every row this reaches is a Deferred Ask, because store-and-nudge
/// asking arrives with the column. So the column's own default is the whole of
/// the rewrite, and there is no `UPDATE` under it — see [`super::deferrals`].
///
/// Safe to run twice: what says whether there is anything to do is the column
/// being absent, and after the first run it is there.
async fn stored_asks_nobody_was_idling_on(pool: &SqlitePool) -> Result<()> {
    let there: Option<(String,)> =
        sqlx::query_as("SELECT name FROM pragma_table_info('deferrals') WHERE name = ?")
            .bind("idled")
            .fetch_optional(pool)
            .await
            .context("looking for which stored asks a session is idling on")?;

    if there.is_some() {
        return Ok(());
    }

    sqlx::query("ALTER TABLE deferrals ADD COLUMN idled INTEGER NOT NULL DEFAULT 0")
        .execute(pool)
        .await
        .context("settling the stored asks written before there were two kinds")?;

    Ok(())
}

/// Give every Conversation written before a branch could be left to its first
/// session to name the column that says whether one is.
///
/// Nobody is: the naming instruction goes out with the start press, and every
/// row this reaches was started before there was one to go out — see
/// [`super::Conversation::naming`]. So the column's own default is the whole of
/// the rewrite, and there is no `UPDATE` under it.
///
/// Safe to run twice: what says whether there is anything to do is the column
/// being absent, and after the first run it is there.
async fn conversations_whose_branch_nobody_was_naming(pool: &SqlitePool) -> Result<()> {
    let there: Option<(String,)> =
        sqlx::query_as("SELECT name FROM pragma_table_info('conversations') WHERE name = ?")
            .bind("naming")
            .fetch_optional(pool)
            .await
            .context("looking for the branch a Conversation is still having named")?;

    if there.is_some() {
        return Ok(());
    }

    sqlx::query("ALTER TABLE conversations ADD COLUMN naming INTEGER NOT NULL DEFAULT 0")
        .execute(pool)
        .await
        .context("settling the branch names the Conversations written before this carry")?;

    Ok(())
}

/// Give every Conversation written before a branch name had an owner the column
/// that says whose it is, and record the name each of them is carrying as one
/// somebody settled on.
///
/// A branch name used to be one thing — prefilled randomly, typed over or not,
/// and drawn either way. It is two now: the name Verkstead invented at creation
/// and the name somebody settled on, and only the second is ever shown — see
/// [`super::Conversation::branch_named`]. So every row written before this takes
/// the name it has as settled, whichever of the two it really was: it is the
/// name the human has been reading in the sidebar all along, and a Conversation
/// that started reading *Draft* on account of an upgrade would be one they could
/// no longer find.
///
/// Safe to run twice: what says whether there is anything to do is the column
/// being absent, and after the first run it is there.
async fn conversations_whose_branch_name_had_no_owner(pool: &SqlitePool) -> Result<()> {
    let there: Option<(String,)> =
        sqlx::query_as("SELECT name FROM pragma_table_info('conversations') WHERE name = ?")
            .bind("named_branch")
            .fetch_optional(pool)
            .await
            .context("looking for the settled branch name of a Conversation")?;

    if there.is_some() {
        return Ok(());
    }

    sqlx::query("ALTER TABLE conversations ADD COLUMN named_branch TEXT")
        .execute(pool)
        .await
        .context("giving the Conversations written before this a settled branch name")?;

    sqlx::query("UPDATE conversations SET named_branch = branch")
        .execute(pool)
        .await
        .context("settling the branch names the Conversations written before this carry")?;

    Ok(())
}

/// Give every Conversation written before there was a Review role the column
/// that holds it.
///
/// A Conversation used to fix two Pairings and now fixes three — one for the
/// grilling, one for what builds, one for the wrap-up's review — and the third
/// goes in a column beside the other two rather than in a table of its own,
/// because [`super::conversations::Role`] names a column per role and a role
/// whose Profile half lived somewhere else would be two ways of storing the one
/// choice.
///
/// Empty for every row it adds, which is what an unchosen picker is: what was
/// grilled before this existed was reviewed under the implementation Pairing,
/// and writing that in would be inventing a choice nobody made.
///
/// Which is still where such a Conversation's review runs. The column is empty
/// and there is no way left to fill it — the pickers froze when its work
/// started — so a review with none picked is launched under the Implementation
/// Pairing, above the store. That is what keeps an empty column a choice nobody
/// made rather than a wrap-up nothing can ever finish.
///
/// Safe to run twice: what says whether there is anything to do is the column
/// being absent, and after the first run it is there.
async fn conversations_that_had_no_review_profile(pool: &SqlitePool) -> Result<()> {
    let there: Option<(String,)> =
        sqlx::query_as("SELECT name FROM pragma_table_info('conversations') WHERE name = ?")
            .bind("review_profile_id")
            .fetch_optional(pool)
            .await
            .context("looking for the review Profile of a Conversation")?;

    if there.is_some() {
        return Ok(());
    }

    sqlx::query(
        "ALTER TABLE conversations
         ADD COLUMN review_profile_id INTEGER REFERENCES profiles(id)",
    )
    .execute(pool)
    .await
    .context("giving the Conversations written before this a review Profile")?;

    Ok(())
}

/// Attribute every commit recorded before Verkstead swept more than one
/// repository, and rebuild the index that keeps one commit per Conversation.
///
/// A commit used to be the Conversation's and the sha's — one repository per
/// Conversation, so naming it would have been naming the only thing it could
/// be. A Conversation now works alongside companion repos and a read-write one
/// is swept like the work's own, so a commit is the Conversation's, the Repo's
/// and the sha's. Every row already there belongs to the Conversation's own
/// repository, which is what it was possible for it to be.
///
/// The table is rebuilt rather than altered because the rule is declared inline
/// as a `UNIQUE`, and SQLite gives that its own index that no `DROP INDEX`
/// reaches: a column can be added to a table, but the constraint on it is part
/// of the table's own text. So the shape is written out again here, filled from
/// the old rows, and swapped in — which is also what carries the rule forward,
/// the rename taking the index with it.
///
/// The shape is written out rather than borrowed from [`super::commits`], for
/// the reason the stop prose below is: this is a shape rows are put into once
/// and never again, and a rewrite that moved with the declaration would make a
/// database opened after the next column is added come out a different shape
/// from one opened today.
///
/// Safe to run twice: what says whether there is anything to do is the column
/// being absent, and after the first run it is there.
async fn commits_that_named_no_repo(pool: &SqlitePool) -> Result<()> {
    let there: Option<(String,)> =
        sqlx::query_as("SELECT name FROM pragma_table_info('commits') WHERE name = ?")
            .bind("repo_id")
            .fetch_optional(pool)
            .await
            .context("looking for the Repo of a recorded commit")?;

    if there.is_some() {
        return Ok(());
    }

    let mut tx = super::writing(
        pool,
        "attributing the commits recorded before this to a repository",
    )
    .await?;

    sqlx::query(
        "CREATE TABLE commits_by_repo (
             event_id        INTEGER PRIMARY KEY REFERENCES timeline_events(id),
             conversation_id INTEGER NOT NULL REFERENCES conversations(id),
             repo_id         INTEGER NOT NULL REFERENCES repos(id),
             sha             TEXT NOT NULL,
             subject         TEXT NOT NULL,
             files           INTEGER NOT NULL,
             insertions      INTEGER NOT NULL,
             deletions       INTEGER NOT NULL,
             UNIQUE (conversation_id, repo_id, sha)
         ) STRICT",
    )
    .execute(&mut *tx)
    .await
    .context("making the commits table over with a repository on it")?;

    // Joined to the Conversations rather than sub-selected per row, which is
    // also what leaves behind a commit whose Conversation has gone: there is no
    // repository such a row could be attributed to, and it is on nobody's
    // Timeline to be read off.
    sqlx::query(
        "INSERT INTO commits_by_repo
             (event_id, conversation_id, repo_id, sha, subject, files, insertions, deletions)
         SELECT c.event_id, c.conversation_id, v.repo_id, c.sha, c.subject,
                c.files, c.insertions, c.deletions
         FROM commits c
         JOIN conversations v ON v.id = c.conversation_id",
    )
    .execute(&mut *tx)
    .await
    .context("attributing the commits recorded before this to a repository")?;

    sqlx::query("DROP TABLE commits")
        .execute(&mut *tx)
        .await
        .context("taking away the commits table as it was")?;

    // Which takes the unique index with it, that being the table's own: the rule
    // the rebuilt table carries is the Conversation, the Repo and the sha.
    sqlx::query("ALTER TABLE commits_by_repo RENAME TO commits")
        .execute(&mut *tx)
        .await
        .context("putting the rebuilt commits table where the old one was")?;

    tx.commit()
        .await
        .context("attributing the commits recorded before this to a repository")
}

/// Attribute every pull request recorded before a Conversation could end on more
/// than one, and rebuild the index that keeps one of them per Conversation.
///
/// A pull request used to be the Conversation's alone — one branch per
/// Conversation and one pull request per branch, so naming the repository would
/// have been naming the only thing it could be. A Conversation now ends on one
/// per repository it was worked in: its own, and one per touched read-write
/// companion. Every row already there belongs to the Conversation's own
/// repository, which is what it was possible for it to be.
///
/// The table is rebuilt rather than altered for the reason the commits table
/// above is: `UNIQUE (conversation_id)` is declared inline, SQLite gives it its
/// own index that no `DROP INDEX` reaches, and a `CREATE TABLE IF NOT EXISTS`
/// does nothing at all to a table that is already there. So the shape is written
/// out again here, filled from the old rows, and swapped in — the rename taking
/// the rebuilt rule with it.
///
/// Written out rather than borrowed from [`super::pull_requests`], for that same
/// migration's reason: this is a shape rows are put into once and never again.
///
/// Safe to run twice: what says whether there is anything to do is the column
/// being absent, and after the first run it is there.
async fn pull_requests_that_named_no_repo(pool: &SqlitePool) -> Result<()> {
    let there: Option<(String,)> =
        sqlx::query_as("SELECT name FROM pragma_table_info('pull_requests') WHERE name = ?")
            .bind("repo_id")
            .fetch_optional(pool)
            .await
            .context("looking for the Repo of a recorded pull request")?;

    if there.is_some() {
        return Ok(());
    }

    let mut tx = super::writing(
        pool,
        "attributing the pull requests recorded before this to a repository",
    )
    .await?;

    sqlx::query(
        "CREATE TABLE pull_requests_by_repo (
             event_id        INTEGER PRIMARY KEY REFERENCES timeline_events(id),
             conversation_id INTEGER NOT NULL REFERENCES conversations(id),
             repo_id         INTEGER NOT NULL REFERENCES repos(id),
             number          INTEGER NOT NULL,
             title           TEXT NOT NULL,
             url             TEXT NOT NULL,
             UNIQUE (conversation_id, repo_id)
         ) STRICT",
    )
    .execute(&mut *tx)
    .await
    .context("making the pull requests table over with a repository on it")?;

    // Joined to the Conversations rather than sub-selected per row, which is
    // also what leaves behind a pull request whose Conversation has gone: there
    // is no repository such a row could be attributed to, and it is on nobody's
    // Timeline to be read off.
    sqlx::query(
        "INSERT INTO pull_requests_by_repo
             (event_id, conversation_id, repo_id, number, title, url)
         SELECT p.event_id, p.conversation_id, v.repo_id, p.number, p.title, p.url
         FROM pull_requests p
         JOIN conversations v ON v.id = p.conversation_id",
    )
    .execute(&mut *tx)
    .await
    .context("attributing the pull requests recorded before this to a repository")?;

    sqlx::query("DROP TABLE pull_requests")
        .execute(&mut *tx)
        .await
        .context("taking away the pull requests table as it was")?;

    // Which takes the unique index with it, that being the table's own: the rule
    // the rebuilt table carries is the Conversation and the Repo.
    sqlx::query("ALTER TABLE pull_requests_by_repo RENAME TO pull_requests")
        .execute(&mut *tx)
        .await
        .context("putting the rebuilt pull requests table where the old one was")?;

    tx.commit()
        .await
        .context("attributing the pull requests recorded before this to a repository")
}

/// Attribute every fix session counted before a Conversation could end on more
/// than one pull request, and rebuild the key that keeps the count per check.
///
/// The count used to be the Conversation's and the check's — one pull request per
/// Conversation, so naming the repository would have been naming the only thing
/// it could be. A Conversation now has a suite per pull request, and the same
/// check name red on two of them is two different failures: one spending the
/// other's attempts would stop a run that still had somewhere to go. Every row
/// already there was counted against the Conversation's own repository, which is
/// what it was possible for it to be.
///
/// The table is rebuilt rather than altered for the reason the two above it are:
/// the rule is the primary key, declared inline, and a `CREATE TABLE IF NOT
/// EXISTS` does nothing at all to a table that is already there.
///
/// Safe to run twice: what says whether there is anything to do is the column
/// being absent, and after the first run it is there.
async fn fix_attempts_that_named_no_repo(pool: &SqlitePool) -> Result<()> {
    let there: Option<(String,)> =
        sqlx::query_as("SELECT name FROM pragma_table_info('check_fix_attempts') WHERE name = ?")
            .bind("repo_id")
            .fetch_optional(pool)
            .await
            .context("looking for the Repo of a counted fix session")?;

    if there.is_some() {
        return Ok(());
    }

    let mut tx = super::writing(
        pool,
        "attributing the fix sessions counted before this to a repository",
    )
    .await?;

    sqlx::query(
        "CREATE TABLE check_fix_attempts_by_repo (
             conversation_id INTEGER NOT NULL REFERENCES conversations(id),
             repo_id         INTEGER NOT NULL REFERENCES repos(id),
             check_name      TEXT NOT NULL,
             attempts        INTEGER NOT NULL,
             PRIMARY KEY (conversation_id, repo_id, check_name)
         ) STRICT",
    )
    .execute(&mut *tx)
    .await
    .context("making the check fix attempts table over with a repository on it")?;

    // Joined to the Conversations rather than sub-selected per row, which is
    // also what leaves behind a count whose Conversation has gone: there is no
    // repository such a row could be attributed to, and no wrap-up left to read
    // it.
    sqlx::query(
        "INSERT INTO check_fix_attempts_by_repo
             (conversation_id, repo_id, check_name, attempts)
         SELECT a.conversation_id, v.repo_id, a.check_name, a.attempts
         FROM check_fix_attempts a
         JOIN conversations v ON v.id = a.conversation_id",
    )
    .execute(&mut *tx)
    .await
    .context("attributing the fix sessions counted before this to a repository")?;

    sqlx::query("DROP TABLE check_fix_attempts")
        .execute(&mut *tx)
        .await
        .context("taking away the check fix attempts table as it was")?;

    sqlx::query("ALTER TABLE check_fix_attempts_by_repo RENAME TO check_fix_attempts")
        .execute(&mut *tx)
        .await
        .context("putting the rebuilt check fix attempts table where the old one was")?;

    tx.commit()
        .await
        .context("attributing the fix sessions counted before this to a repository")
}

/// Attribute every settled checks row to the pull request it was about, and
/// rebuild the key that keeps one settlement per thing waited on.
///
/// A wrap-up used to wait on three things, all of them the Conversation's. The
/// checks and what has been said are now one per pull request each — a
/// Conversation ends on one per repository it was worked in, each with its own
/// suite and its own conversation — so a settlement is the Conversation's, the
/// pull request's and the thing's. Every settled `checks` and `comments` row
/// already there is the Conversation's own repository's, which is the only pull
/// request it was possible for it to be about; the review stays what it is, one
/// review across the whole of it, and is written against no pull request at all.
///
/// The table is rebuilt for the reason the three above it are, and the column it
/// gains references nothing: the rows about no pull request carry a Repo of zero,
/// which is no repository — SQLite hands rowids out from one — and a reference
/// would refuse them. See [`super::wrap_up`].
///
/// Safe to run twice: what says whether there is anything to do is the column
/// being absent, and after the first run it is there.
async fn settlements_that_named_no_pull_request(pool: &SqlitePool) -> Result<()> {
    let there: Option<(String,)> =
        sqlx::query_as("SELECT name FROM pragma_table_info('wrap_up_settled') WHERE name = ?")
            .bind("repo_id")
            .fetch_optional(pool)
            .await
            .context("looking for the pull request a wrap-up's settlement is about")?;

    if there.is_some() {
        return Ok(());
    }

    let mut tx = super::writing(
        pool,
        "attributing the settled checks of before to a pull request",
    )
    .await?;

    sqlx::query(
        "CREATE TABLE wrap_up_settled_by_repo (
             conversation_id INTEGER NOT NULL REFERENCES conversations(id),
             repo_id         INTEGER NOT NULL,
             waiting_on      TEXT NOT NULL,
             at              TEXT NOT NULL,
             PRIMARY KEY (conversation_id, repo_id, waiting_on)
         ) STRICT",
    )
    .execute(&mut *tx)
    .await
    .context("making the wrap-up settlements table over with a pull request on it")?;

    // Everything but the review against the Conversation's own repository, which
    // is the whole of what this rewrite decides. Joined to the Conversations for
    // the reason the rewrites above are joined: a settlement whose Conversation
    // has gone has no repository to be attributed to.
    sqlx::query(
        "INSERT INTO wrap_up_settled_by_repo
             (conversation_id, repo_id, waiting_on, at)
         SELECT s.conversation_id,
                CASE s.waiting_on WHEN 'review' THEN 0 ELSE v.repo_id END,
                s.waiting_on, s.at
         FROM wrap_up_settled s
         JOIN conversations v ON v.id = s.conversation_id",
    )
    .execute(&mut *tx)
    .await
    .context("attributing the settled checks of before to a pull request")?;

    sqlx::query("DROP TABLE wrap_up_settled")
        .execute(&mut *tx)
        .await
        .context("taking away the wrap-up settlements table as it was")?;

    sqlx::query("ALTER TABLE wrap_up_settled_by_repo RENAME TO wrap_up_settled")
        .execute(&mut *tx)
        .await
        .context("putting the rebuilt wrap-up settlements table where the old one was")?;

    tx.commit()
        .await
        .context("attributing the settled checks of before to a pull request")
}

/// Attribute every addressed comment to the pull request it was left on, and
/// rebuild the key that keeps one row per comment.
///
/// Which comments have been dispatched about used to be the Conversation's, one
/// pull request per Conversation making the pull request nothing worth naming.
/// A Conversation now ends on one per repository it was worked in, each read on
/// its own interval and each settling on its own, so a row is the
/// Conversation's, the pull request's and the comment's. Every row already there
/// is the Conversation's own repository's, which is the only pull request it was
/// possible for the comment to have been left on.
///
/// The table is rebuilt for the reason the ones above it are: the rule is
/// declared inline as the primary key, so it is the table itself that has to be
/// written out again.
///
/// Safe to run twice: what says whether there is anything to do is the column
/// being absent, and after the first run it is there.
async fn addressed_comments_that_named_no_pull_request(pool: &SqlitePool) -> Result<()> {
    let there: Option<(String,)> =
        sqlx::query_as("SELECT name FROM pragma_table_info('addressed_comments') WHERE name = ?")
            .bind("repo_id")
            .fetch_optional(pool)
            .await
            .context("looking for the pull request an addressed comment was left on")?;

    if there.is_some() {
        return Ok(());
    }

    let mut tx = super::writing(
        pool,
        "attributing the comments dispatched for before this to a pull request",
    )
    .await?;

    sqlx::query(
        "CREATE TABLE addressed_comments_by_repo (
             conversation_id INTEGER NOT NULL REFERENCES conversations(id),
             repo_id         INTEGER NOT NULL REFERENCES repos(id),
             comment_id      TEXT NOT NULL,
             at              TEXT NOT NULL,
             PRIMARY KEY (conversation_id, repo_id, comment_id)
         ) STRICT",
    )
    .execute(&mut *tx)
    .await
    .context("making the addressed comments table over with a pull request on it")?;

    // Joined to the Conversations rather than sub-selected per row, which is
    // also what leaves behind a comment whose Conversation has gone: there is no
    // pull request such a row could be attributed to, and no wrap-up left to
    // read it.
    sqlx::query(
        "INSERT INTO addressed_comments_by_repo
             (conversation_id, repo_id, comment_id, at)
         SELECT c.conversation_id, v.repo_id, c.comment_id, c.at
         FROM addressed_comments c
         JOIN conversations v ON v.id = c.conversation_id",
    )
    .execute(&mut *tx)
    .await
    .context("attributing the comments dispatched for before this to a pull request")?;

    sqlx::query("DROP TABLE addressed_comments")
        .execute(&mut *tx)
        .await
        .context("taking away the addressed comments table as it was")?;

    sqlx::query("ALTER TABLE addressed_comments_by_repo RENAME TO addressed_comments")
        .execute(&mut *tx)
        .await
        .context("putting the rebuilt addressed comments table where the old one was")?;

    tx.commit()
        .await
        .context("attributing the comments dispatched for before this to a pull request")
}

/// The table the stops of before are kept in, named once: it is gone by the end
/// of this module, and nothing else in the codebase knows the word.
const OLD_TABLE: &str = "interruptions";

/// The word the state off the ladder was stored as while the press was called
/// Abort. Named once here, because this is the module that takes it away.
const OLD_STATE: &str = "aborted";

/// Rewrite the state of every Conversation that was aborted, and the move Event
/// that says it got there, into the word the press is called now.
///
/// The lifecycle is stored as text and a `moved` Event carries the same text as
/// its body, so the old name is written in two places per Conversation. Both
/// move together: a Timeline whose card said Closed above a move that still said
/// `aborted` would be one Conversation described in two vocabularies.
///
/// The Events are not history being rewritten. A move Event records *which state
/// the work moved to*, and that state is the one this codebase calls Closed —
/// the word in the column is the state's name rather than a note somebody typed,
/// and a name is the kind of thing that gets corrected. What ADR-0006 keeps as
/// written is what a session said, which nothing here touches.
///
/// Safe to run twice: after the first there are no rows left saying `aborted`.
/// [`super::Lifecycle::read`] still knows the word regardless, for a database
/// that never came through here.
async fn conversations_that_were_aborted(pool: &SqlitePool) -> Result<()> {
    let mut tx = super::writing(
        pool,
        "renaming the state of every Conversation that was aborted",
    )
    .await?;

    sqlx::query("UPDATE conversations SET state = ? WHERE state = ?")
        .bind(Lifecycle::Closed.stored())
        .bind(OLD_STATE)
        .execute(&mut *tx)
        .await
        .context("renaming the state of every Conversation that was aborted")?;

    // `kind = 'moved'` and nothing wider: `aborted` was a lifecycle word, and a
    // Brief or a handoff that happens to hold it is prose somebody wrote.
    sqlx::query("UPDATE timeline_events SET body = ? WHERE kind = 'moved' AND body = ?")
        .bind(Lifecycle::Closed.stored())
        .bind(OLD_STATE)
        .execute(&mut *tx)
        .await
        .context("renaming the move Event of every Conversation that was aborted")?;

    tx.commit()
        .await
        .context("renaming the state of every Conversation that was aborted")
}

/// Rewrite the stops a Verkstead of before kept in a table of their own into the
/// Notices and stops they are now, and take the table away.
///
/// A stop used to be a row of separate facts joined onto its Timeline Event —
/// which step failed, how it ended, what git made of the Worktree, the tail of
/// what the session said, and whichever of three remedies the human chose. A
/// stop is now four columns on the Conversation and an ordinary **Notice** on
/// its Timeline: prose somebody reads, and one durable fact saying the
/// Conversation is stopped *now*. So the columns become the markdown they would
/// have been written as, and each Event becomes the Notice it would have been.
///
/// The ones still open become stops as well, and Verkstead's own: an open stop
/// of before was a step that failed and a run left waiting on the human, which
/// is exactly the stop a restart leaves alone and exactly the stop the marks are
/// for. A Conversation that is stopped already keeps the stop it has — there is
/// one per Conversation, and the first Notice is the one that explains it.
///
/// One transaction, and the table is dropped inside it: a Timeline holding rows
/// that have been rewritten beside a table that still says otherwise would be
/// two accounts of the same stop.
async fn stops_recorded_the_old_way(pool: &SqlitePool) -> Result<()> {
    let table: Option<(String,)> =
        sqlx::query_as("SELECT name FROM sqlite_master WHERE type = 'table' AND name = ?")
            .bind(OLD_TABLE)
            .fetch_optional(pool)
            .await
            .context("looking for the table the stops of before are kept in")?;

    if table.is_none() {
        return Ok(());
    }

    /// The columns in the order the query below selects them.
    type Row = (
        i64,
        i64,
        String,
        String,
        String,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        String,
    );

    let mut tx = super::writing(pool, "rewriting the stops of before").await?;

    let rows: Vec<Row> = sqlx::query_as(
        "SELECT i.event_id, i.conversation_id, i.what, i.how, i.git_status, i.tail,
                i.remedy, i.note, i.settled_at, e.at
         FROM interruptions i
         JOIN timeline_events e ON e.id = i.event_id
         ORDER BY i.event_id",
    )
    .fetch_all(&mut *tx)
    .await
    .context("reading the stops of before")?;

    for (event_id, conversation_id, what, how, git_status, tail, remedy, note, settled_at, at) in
        &rows
    {
        let said = said(
            what,
            how,
            git_status,
            tail,
            remedy.as_deref(),
            note.as_deref(),
            settled_at.as_deref(),
        );

        sqlx::query("UPDATE timeline_events SET kind = 'notice', body = ? WHERE id = ?")
            .bind(&said)
            .bind(event_id)
            .execute(&mut *tx)
            .await
            .with_context(|| format!("rewriting the stop of Event {event_id} as a Notice"))?;

        // Only the open ones. A settled stop is a run that got going again, or
        // one the human ended, and nothing is waiting on either now.
        //
        // The Event's own time rather than this morning's: the stop happened
        // whenever it happened, and a stop stamped with the migration would say
        // the work stopped when the server was upgraded.
        if remedy.is_none() {
            // Only where the Conversation is not stopped already, which is the
            // one stop per Conversation said as a `WHERE`: a Conversation that
            // has one keeps it, and the first Notice is the one that explains it.
            sqlx::query(
                "UPDATE conversations
                    SET stopped_at = ?, stopped_by = ?, stopped_notice = ?
                  WHERE id = ? AND stopped_at IS NULL",
            )
            .bind(at)
            .bind(Decision::Verkstead.stored())
            .bind(event_id)
            .bind(conversation_id)
            .execute(&mut *tx)
            .await
            .with_context(|| format!("recording that Conversation {conversation_id} is stopped"))?;
        }
    }

    sqlx::query(&format!("DROP TABLE {OLD_TABLE}"))
        .execute(&mut *tx)
        .await
        .context("taking away the table the stops of before are kept in")?;

    tx.commit().await.context("rewriting the stops of before")
}

/// One old row as the markdown its Notice holds.
///
/// The same shape a stop's Notice is written in today — the stop, the reason,
/// and both pieces of evidence under headings of their own — because it is the
/// same thing being said, and the human reads the two in one list. Written here
/// rather than borrowed from the half of the server that writes the live ones:
/// this is a shape rows are put into once and never again, and a migration that
/// moved with the prose around it would rewrite yesterday's database differently
/// from today's.
///
/// The remedy is noted at the end where one was chosen, because it is the rest
/// of what happened: a stop somebody answered reads wrong without the answer.
fn said(
    what: &str,
    how: &str,
    git_status: &str,
    tail: &str,
    remedy: Option<&str>,
    note: Option<&str>,
    settled_at: Option<&str>,
) -> String {
    let mut said = format!(
        "**{}** stopped.\n\n{how}\n\n### The worktree\n\n{}\n\n### What the last session said\n\n{}\n",
        opening(what),
        indented(
            git_status,
            "Git had nothing pending, or the repository would not answer.",
        ),
        indented(tail, "It said nothing at all."),
    );

    let Some(remedy) = remedy else {
        return said;
    };

    said.push_str("\n### What was done about it\n\n");
    said.push_str(&chosen(remedy, settled_at));

    if let Some(note) = note.map(str::trim).filter(|note| !note.is_empty()) {
        said.push_str("\n\n");
        said.push_str(note);
    }

    said.push('\n');

    said
}

/// The remedy in a sentence, and when it was chosen.
///
/// A word this Verkstead does not know is written down as it was found rather
/// than refused on, unlike every other stored word here: the row is history by
/// the time this runs, and a database that would not open on account of one
/// would be worse than a Notice naming a remedy nobody remembers.
fn chosen(remedy: &str, settled_at: Option<&str>) -> String {
    let named = match remedy {
        "retry" => "The step was run again",
        "take-over" => "Verkstead stopped driving, and the Worktree was the human's",
        "abort" => "The run was ended here",
        other => return format!("The stop was settled by `{other}`.\n"),
    };

    match settled_at {
        Some(at) => format!("{named}, on {at}."),
        None => format!("{named}."),
    }
}

/// The stop with its first letter up, because it opens the sentence the Notice
/// is.
fn opening(what: &str) -> String {
    let mut letters = what.chars();

    match letters.next() {
        Some(first) => first.to_uppercase().collect::<String>() + letters.as_str(),
        None => String::new(),
    }
}

/// A block of terminal output as markdown holds it: every line indented by
/// four, which is the one form nothing inside it can break out of.
///
/// `empty` is the sentence that stands in its place where there is nothing to
/// show, and it is prose rather than a block: there is nothing to preserve the
/// columns of.
fn indented(said: &str, empty: &str) -> String {
    let block: Vec<String> = said.lines().map(|line| format!("    {line}")).collect();

    if block.is_empty() {
        return empty.to_owned();
    }

    block.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An open stop reads as the Notices written today read: what stopped,
    /// why, and both blocks of evidence set apart from the prose.
    #[test]
    fn an_open_stop_becomes_the_notice_it_would_have_been() {
        let said = said(
            "the task in .tasks/03-commit-events.md",
            "the session exited with status 1",
            "## task-runner\n M commits.rs",
            "error: could not compile",
            None,
            None,
            None,
        );

        assert_eq!(
            said,
            "**The task in .tasks/03-commit-events.md** stopped.\n\n\
             the session exited with status 1\n\n\
             ### The worktree\n\n\
             \x20   ## task-runner\n    \x20M commits.rs\n\n\
             ### What the last session said\n\n\
             \x20   error: could not compile\n",
        );
        assert!(
            !said.contains("What was done about it"),
            "nothing was done about it: {said:?}",
        );
    }

    /// And a settled one carries what the human chose, with whatever they wrote
    /// beside it — the rest of what happened, which a Notice without it reads
    /// wrong without.
    #[test]
    fn a_settled_stop_says_what_was_done_about_it() {
        let said = said(
            "implementing the work",
            "the session exited with status 1",
            "",
            "",
            Some("retry"),
            Some("try again but leave the migration alone"),
            Some("2026-08-01T09:14:22.000Z"),
        );

        assert!(
            said.contains(
                "### What was done about it\n\n\
                 The step was run again, on 2026-08-01T09:14:22.000Z.\n\n\
                 try again but leave the migration alone\n"
            ),
            "{said:?}",
        );
    }

    /// A remedy word this Verkstead does not know is written down as it was
    /// found. The row is history, and a database that would not open on account
    /// of one would be worse than a Notice nobody quite understands.
    #[test]
    fn a_remedy_nobody_remembers_is_still_written_down() {
        let said = said(
            "grilling the work",
            "it went quiet",
            "",
            "",
            Some("sleep"),
            None,
            None,
        );

        assert!(said.contains("settled by `sleep`"), "{said:?}");
    }
}
