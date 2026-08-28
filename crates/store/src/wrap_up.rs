//! What wrap-up is still waiting on, how many goes the machine has had at a red
//! check, which comments it has already dispatched about — and the move to Done
//! that having settled all of it is.
//!
//! Everything here but the review is kept per pull request rather than per
//! Conversation: the checks that have settled, the goes one of them has had,
//! whether anything said is left unaddressed, and which comments a session has
//! been dispatched about. A Conversation ends on one pull request per repository
//! it was worked in, each with a suite of its own and a conversation of its own —
//! `Rust` red on two of them is two failures rather than one, and a human writing
//! on one of them is not writing on the other — so the Repo the pull request was
//! opened in is part of what each row is about. The review is one review across
//! the whole of it and is written against no pull request at all.
//!
//! Three small tables and one Timeline Event, which is the whole shape of this
//! module. Everything else a human reads about wrap-up is already an Event — the
//! pull request, the commits a fix session lands, the Notice of the stop where
//! it stops asking the machine. What is kept here is the bookkeeping underneath:
//! facts that decide what Verkstead does next and that nobody would want a row
//! on a Timeline for.
//!
//! All three survive a restart, and all three have to. A server that came back
//! up having forgotten how many fix sessions a check had already had would
//! dispatch them again for ever, which is exactly the failure *two attempts,
//! then ask the human* exists to prevent; one that had forgotten which comments
//! it had read would dispatch a session about feedback that was addressed
//! yesterday.
//!
//! What is *settled* is written down and what is outstanding is not: the checks
//! and the comments are asked of GitHub on every poll, so a red suite needs no
//! memory. The row is deleted again the moment one of them stops being true —
//! see [`unsettle_wrap_up`] — which is what makes a commit pushed to the pull
//! request put its checks back to waiting rather than leaving yesterday's green
//! standing, and a comment landing after a quiet spell something to deal with.

use anyhow::{Context, Result, bail};
use sqlx::SqlitePool;

use super::conversations::{Lifecycle, moved};

/// One of the things a Conversation has to have settled before wrap-up is over.
///
/// Three kinds of thing and nothing else, though not three settlements: the
/// checks and what has been said are one each per pull request, and a
/// Conversation ends on one pull request per repository it was worked in, so a
/// wrap-up with a companion is waiting on five things rather than three. What is
/// *not* here is the merge: stages stack on unmerged predecessors, so a
/// Conversation that stayed in Wrapping until its pull request landed would hold
/// up every stage behind it — and merging is the human act this pipeline is
/// built around rather than a step in it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitingOn {
    /// The checks are green on the pull request opened in this Repo.
    ///
    /// One per pull request rather than one per Conversation, because a suite
    /// is a fact about a pull request: a Conversation ends on one per
    /// repository it was worked in, each with its own checks running against
    /// its own branch, and a wrap-up that settled *the* checks would be one
    /// where a green companion stood for a red one.
    Checks(i64),

    /// The self-review has been answered — or found nothing to ask about.
    ///
    /// Unlike the checks, this is settled once and stays settled — within one
    /// wrap. A review is something that happened rather than a state of the
    /// branch: the human has read what it found and said which of it to fix, and
    /// a commit landing afterwards does not un-say that.
    ///
    /// Across re-entry it does not hold, and could not: a wrap-up that split its
    /// findings out into a backlog leaves Wrapping to build them, and what comes
    /// back is a branch nobody has read. So the move out takes this settle with
    /// it — see [`super::implement_again`] — and the second wrap reviews afresh.
    Review,

    /// Nothing has been said on the pull request opened in this Repo that has
    /// not had a session dispatched about it.
    ///
    /// Like the checks and unlike the review: a comment landing after this
    /// settled unsettles it again, because a wrap-up that stopped reading its
    /// pull request the first time it went quiet would be one a human could not
    /// reach.
    ///
    /// And per pull request for the checks' reason. A human writes on the pull
    /// request they are reading, and a Conversation ends on one per repository
    /// it was worked in: a wrap-up that settled *the* comments would be one
    /// where a quiet companion stood for a busy one, and the pull request that
    /// went quiet is the one that has nothing outstanding on it.
    Comments(i64),
}

impl WaitingOn {
    /// The word the column holds. Lowercase and spelled out, so a database
    /// opened by hand says something.
    fn stored(self) -> &'static str {
        match self {
            Self::Checks(_) => "checks",
            Self::Review => "review",
            Self::Comments(_) => "comments",
        }
    }

    /// And which pull request it is about, by the Repo that one was opened
    /// in.
    ///
    /// [`NO_PULL_REQUEST`] for the ones that are about the wrap-up as a
    /// whole, which is what the review is: one review across every pull
    /// request the Conversation ended on.
    fn repo(self) -> i64 {
        match self {
            Self::Checks(repo_id) | Self::Comments(repo_id) => repo_id,
            Self::Review => NO_PULL_REQUEST,
        }
    }

    /// The one a stored word and a Repo name between them. An unknown word is
    /// a database written by a Verkstead this one does not understand,
    /// exactly as an unknown lifecycle state is.
    fn read(word: &str, repo_id: i64) -> Result<Self> {
        Ok(match word {
            "checks" => Self::Checks(repo_id),
            "review" => Self::Review,
            "comments" => Self::Comments(repo_id),
            other => bail!("a wrap-up is waiting on the unknown thing {other:?}"),
        })
    }
}

/// What stands in the Repo column of a settlement about no pull request.
///
/// Zero, which is no repository: SQLite hands rowids out from one, so nothing
/// registered can collide with it. Which is also why that column is not a
/// foreign key — a reference to `repos` would refuse the review's own row.
const NO_PULL_REQUEST: i64 = 0;

/// Everything wrap-up waits on that there is one of per Conversation.
///
/// The review, and nothing beside it: one review reads the whole of the work
/// however many repositories it was worked in, so it is the one thing here that
/// is not a fact about a pull request.
///
/// Written out rather than derived, because what it is for is the one question
/// [`finish_wrap_up`] asks — and a list that grew a variant without anybody
/// deciding it belonged here would be a wrap-up quietly waiting on something new.
///
/// The checks and what has been said are not here, and could not be: there is a
/// suite and a conversation per pull request, and which pull requests a
/// Conversation ended on is a fact about the record rather than a constant. So
/// the rule reads them off it and waits on every one — see [`finish_wrap_up`].
pub const WAITED_ON: [WaitingOn; 1] = [WaitingOn::Review];

/// What became of asking whether a wrap-up is over.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Finished {
    /// It is: the Conversation is Done, and the move is on its Timeline.
    Done,

    /// Something is still outstanding, so it stays where it is.
    StillWaiting,

    /// It is not wrapping up any more — closed out from under the watchers, or
    /// finished by the poll before this one.
    NotWrapping,

    /// There is no Conversation with that id.
    NoSuchConversation,
}

/// The three tables wrap-up keeps its bookkeeping in.
///
/// All of them hang off a Conversation rather than off a Timeline Event, unlike
/// nearly everything else here, and that is the point: none of them is something
/// that happened, so none of them is something to draw. They are what Verkstead
/// knows about a Conversation it is wrapping up.
pub(crate) async fn apply_schema(pool: &SqlitePool) -> Result<()> {
    // The Repo is which pull request a settlement is about, and part of what it
    // is rather than a note beside it: the checks settle per pull request.
    // [`NO_PULL_REQUEST`] stands there for the ones about the whole wrap-up,
    // which is why the column references nothing.
    //
    // A database written before a Conversation could end on more than one pull
    // request has neither the column nor the rule. Which is
    // [`super::migrations`]'s to put right as the database opens rather than this
    // function's: the rule is declared inline as the primary key, so it is the
    // table itself that has to be rebuilt, and that is not something a `CREATE
    // TABLE IF NOT EXISTS` can reach.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS wrap_up_settled (
             conversation_id INTEGER NOT NULL REFERENCES conversations(id),
             repo_id         INTEGER NOT NULL,
             waiting_on      TEXT NOT NULL,
             at              TEXT NOT NULL,
             PRIMARY KEY (conversation_id, repo_id, waiting_on)
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the wrap-up settlements table")?;

    // Keyed by the check's name rather than by anything of GitHub's, because the
    // name is what survives what this is counting: a fix session pushes a commit,
    // GitHub starts a whole new run with new ids, and *the same check* has to
    // mean the same thing across both.
    //
    // And by the Repo whose pull request it went red on, for the reason the
    // settlements above carry one: the same check name red on two pull requests
    // is two different failures, and one spending the other's attempts would stop
    // a run that still had somewhere to go.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS check_fix_attempts (
             conversation_id INTEGER NOT NULL REFERENCES conversations(id),
             repo_id         INTEGER NOT NULL REFERENCES repos(id),
             check_name      TEXT NOT NULL,
             attempts        INTEGER NOT NULL,
             PRIMARY KEY (conversation_id, repo_id, check_name)
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the check fix attempts table")?;

    // Keyed by what GitHub calls the comment, which is what survives a restart:
    // a server that came back up and read every comment as new would dispatch a
    // session about feedback that was addressed yesterday.
    //
    // And by the Repo whose pull request it was left on, for the reason the rows
    // above carry one: what settles is *this pull request has nothing
    // outstanding*, which is what lets one go quiet while another is still being
    // answered. GitHub's ids are unique across repositories, so the Repo is not
    // what keeps two comments apart — it is what says which pull request a row
    // is an answer about.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS addressed_comments (
             conversation_id INTEGER NOT NULL REFERENCES conversations(id),
             repo_id         INTEGER NOT NULL REFERENCES repos(id),
             comment_id      TEXT NOT NULL,
             at              TEXT NOT NULL,
             PRIMARY KEY (conversation_id, repo_id, comment_id)
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the addressed comments table")?;

    Ok(())
}

/// Record that one of the things wrap-up waits on is settled.
///
/// Written again where it was settled already, which is every poll of a green
/// suite: settling is a statement about how things are now rather than an event,
/// so saying it twice is saying the same thing twice.
pub async fn settle_wrap_up(
    pool: &SqlitePool,
    conversation_id: i64,
    waiting_on: WaitingOn,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO wrap_up_settled (conversation_id, repo_id, waiting_on, at)
         VALUES (?, ?, ?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
         ON CONFLICT (conversation_id, repo_id, waiting_on) DO NOTHING",
    )
    .bind(conversation_id)
    .bind(waiting_on.repo())
    .bind(waiting_on.stored())
    .execute(pool)
    .await
    .with_context(|| {
        format!("settling {waiting_on:?} for the wrap-up of Conversation {conversation_id}")
    })?;

    Ok(())
}

/// And that it is not settled after all, which is a check that has gone red
/// again or a run that has started over.
///
/// Nothing to do where it was never settled, which is the ordinary case for as
/// long as a suite is running.
pub async fn unsettle_wrap_up(
    pool: &SqlitePool,
    conversation_id: i64,
    waiting_on: WaitingOn,
) -> Result<()> {
    let mut connection = pool
        .acquire()
        .await
        .context("putting something a wrap-up waits on back to waiting")?;

    unsettle(&mut connection, conversation_id, waiting_on).await
}

/// The same, inside a transaction that is doing something else as well.
///
/// Which is the move out of Wrapping: leaving takes the review's settle with it,
/// in the same breath as the state changes, so a Conversation being built again
/// is never one carrying a settled review of work that has not been done yet.
/// See [`super::implement_again`].
pub(crate) async fn unsettle(
    tx: &mut sqlx::SqliteConnection,
    conversation_id: i64,
    waiting_on: WaitingOn,
) -> Result<()> {
    sqlx::query(
        "DELETE FROM wrap_up_settled
         WHERE conversation_id = ? AND repo_id = ? AND waiting_on = ?",
    )
    .bind(conversation_id)
    .bind(waiting_on.repo())
    .bind(waiting_on.stored())
    .execute(&mut *tx)
    .await
    .with_context(|| {
        format!(
            "putting {waiting_on:?} back to waiting for the wrap-up of \
             Conversation {conversation_id}"
        )
    })?;

    Ok(())
}

/// What a Conversation's wrap-up has settled so far.
///
/// The whole set rather than one asked about at a time, because what it is for
/// is the question *is wrap-up over* — which is about all of them together.
pub async fn wrap_up_settled(pool: &SqlitePool, conversation_id: i64) -> Result<Vec<WaitingOn>> {
    let rows: Vec<(String, i64)> =
        sqlx::query_as("SELECT waiting_on, repo_id FROM wrap_up_settled WHERE conversation_id = ?")
            .bind(conversation_id)
            .fetch_all(pool)
            .await
            .with_context(|| {
                format!("reading what the wrap-up of Conversation {conversation_id} has settled")
            })?;

    rows.into_iter()
        .map(|(waiting_on, repo_id)| WaitingOn::read(&waiting_on, repo_id))
        .collect()
}

/// When one of the things a wrap-up waits on was settled, where it has been.
///
/// The moment rather than the fact, which is what tells one half of a wrap-up's
/// proposals from the other: the review is the session a wrap-up starts with and
/// no batch is dispatched until it has settled, so a proposal put up before this
/// is the review's own and one put up after it is a batch's. See
/// [`super::last_batch_proposal`].
pub async fn settled_when(
    pool: &SqlitePool,
    conversation_id: i64,
    waiting_on: WaitingOn,
) -> Result<Option<String>> {
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT at FROM wrap_up_settled
         WHERE conversation_id = ? AND repo_id = ? AND waiting_on = ?",
    )
    .bind(conversation_id)
    .bind(waiting_on.repo())
    .bind(waiting_on.stored())
    .fetch_optional(pool)
    .await
    .with_context(|| {
        format!(
            "reading when {waiting_on:?} was settled for the wrap-up of \
             Conversation {conversation_id}"
        )
    })?;

    Ok(row.map(|(at,)| at))
}

/// How many fix sessions this check has already had on the pull request opened
/// in `repo_id`.
///
/// Zero for a check nothing has been dispatched for, which is every check the
/// first time it goes red.
///
/// The Repo is part of the question rather than a filter on it: the same check
/// name red on two of a Conversation's pull requests is two different failures,
/// and one spending the other's attempts would stop a run that still had
/// somewhere to go.
pub async fn fix_attempts(
    pool: &SqlitePool,
    conversation_id: i64,
    repo_id: i64,
    check: &str,
) -> Result<i64> {
    let row: Option<(i64,)> = sqlx::query_as(
        "SELECT attempts FROM check_fix_attempts
         WHERE conversation_id = ? AND repo_id = ? AND check_name = ?",
    )
    .bind(conversation_id)
    .bind(repo_id)
    .bind(check)
    .fetch_optional(pool)
    .await
    .with_context(|| {
        format!(
            "reading what has been tried about {check:?} in Repo {repo_id} on Conversation \
             {conversation_id}"
        )
    })?;

    Ok(row.map(|(attempts,)| attempts).unwrap_or(0))
}

/// Count one more, and say how many that makes.
///
/// Counted as the session is dispatched rather than as it ends, which is the way
/// round that holds when a server is restarted mid-fix: an attempt that was
/// spent and not written down would be one the next server spends again.
pub async fn record_fix_attempt(
    pool: &SqlitePool,
    conversation_id: i64,
    repo_id: i64,
    check: &str,
) -> Result<i64> {
    let (attempts,): (i64,) = sqlx::query_as(
        "INSERT INTO check_fix_attempts (conversation_id, repo_id, check_name, attempts)
         VALUES (?, ?, ?, 1)
         ON CONFLICT (conversation_id, repo_id, check_name)
             DO UPDATE SET attempts = attempts + 1
         RETURNING attempts",
    )
    .bind(conversation_id)
    .bind(repo_id)
    .bind(check)
    .fetch_one(pool)
    .await
    .with_context(|| {
        format!(
            "counting a fix session for {check:?} in Repo {repo_id} on Conversation \
             {conversation_id}"
        )
    })?;

    Ok(attempts)
}

/// Which of the comments on the pull request opened in `repo_id` have already
/// had a session dispatched about them.
///
/// The whole set rather than one asked about at a time, because what it is for
/// is the question *which of these are new* — which is about all of them at once,
/// and the comments arrive from GitHub as a list.
///
/// One pull request's rather than the Conversation's, because that is the
/// question a watcher asks: it read one pull request's comments and is deciding
/// which of *those* to dispatch about.
pub async fn addressed_comments(
    pool: &SqlitePool,
    conversation_id: i64,
    repo_id: i64,
) -> Result<Vec<String>> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT comment_id FROM addressed_comments
         WHERE conversation_id = ? AND repo_id = ?",
    )
    .bind(conversation_id)
    .bind(repo_id)
    .fetch_all(pool)
    .await
    .with_context(|| {
        format!(
            "reading which of Conversation {conversation_id}'s comments in Repo {repo_id} \
             have been dispatched for"
        )
    })?;

    Ok(rows.into_iter().map(|(comment_id,)| comment_id).collect())
}

/// Record that a session has been dispatched about these comments, so the next
/// poll does not dispatch another one.
///
/// The whole batch in one transaction, because one batch is what one session is
/// dispatched for: half a batch written down would be a restart that dispatched
/// a second session about the other half.
///
/// Written as the session is dispatched rather than as it ends, for the reason a
/// fix attempt is counted that way — see [`record_fix_attempt`]: a comment a
/// server had dispatched for and not written down is one the next server
/// dispatches for again.
pub async fn record_addressed_comments(
    pool: &SqlitePool,
    conversation_id: i64,
    repo_id: i64,
    comments: &[String],
) -> Result<()> {
    let mut tx = super::begin_writing(pool)
        .await
        .context("recording which comments have been dispatched for")?;

    for comment in comments {
        sqlx::query(
            "INSERT INTO addressed_comments (conversation_id, repo_id, comment_id, at)
             VALUES (?, ?, ?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
             ON CONFLICT (conversation_id, repo_id, comment_id) DO NOTHING",
        )
        .bind(conversation_id)
        .bind(repo_id)
        .bind(comment)
        .execute(&mut *tx)
        .await
        .with_context(|| {
            format!(
                "recording that {comment:?} has been dispatched for in Repo {repo_id} on \
                 Conversation {conversation_id}"
            )
        })?;
    }

    tx.commit()
        .await
        .context("recording which comments have been dispatched for")?;

    Ok(())
}

/// Forget that a session was dispatched about these comments, so the next poll
/// dispatches another one.
///
/// The other half of [`record_addressed_comments`], and what a batch session
/// that did not finish leaves behind: the comments were recorded as addressed as
/// it was dispatched, and a session that fell over before it put anything to the
/// human addressed none of them. Forgetting them is what makes Resume the batch
/// over again, in a session as fresh as the first.
///
/// Only ever called for a batch nothing is left running about — the session is
/// gone and the run has stopped with a Notice saying so — so there is nothing
/// racing this to dispatch about them in the meantime.
pub async fn forget_addressed_comments(
    pool: &SqlitePool,
    conversation_id: i64,
    repo_id: i64,
    comments: &[String],
) -> Result<()> {
    let mut tx = super::begin_writing(pool)
        .await
        .context("forgetting which comments have been dispatched for")?;

    for comment in comments {
        sqlx::query(
            "DELETE FROM addressed_comments
             WHERE conversation_id = ? AND repo_id = ? AND comment_id = ?",
        )
        .bind(conversation_id)
        .bind(repo_id)
        .bind(comment)
        .execute(&mut *tx)
        .await
        .with_context(|| {
            format!(
                "forgetting that {comment:?} was dispatched for in Repo {repo_id} on \
                 Conversation {conversation_id}"
            )
        })?;
    }

    tx.commit()
        .await
        .context("forgetting which comments have been dispatched for")?;

    Ok(())
}

/// The same for every comment on every one of a Conversation's pull requests.
///
/// What a batch session nobody can identify leaves behind: a server that came
/// back up over one has no way of knowing which pull request it was answering,
/// let alone which of that pull request's comments were its batch and which the
/// review folded in before it. So every one of them is read again — a comment
/// read twice costs a session's work and one dropped costs the human theirs.
///
/// Called for the reason [`forget_addressed_comments`] is, and under the same
/// guarantee: the run has stopped with a Notice saying so, and nothing is racing
/// this to dispatch about them.
pub async fn forget_every_addressed_comment(pool: &SqlitePool, conversation_id: i64) -> Result<()> {
    sqlx::query("DELETE FROM addressed_comments WHERE conversation_id = ?")
        .bind(conversation_id)
        .execute(pool)
        .await
        .with_context(|| {
            format!(
                "forgetting which of Conversation {conversation_id}'s comments have been \
                 dispatched for"
            )
        })?;

    Ok(())
}

/// Move the Conversation to Done, where its wrap-up has settled everything it
/// waits on.
///
/// The rule that ends a wrap-up, and Verkstead's own to apply: there is nobody at
/// the workbench to press anything, which is the whole of what running unattended
/// means. Any one of [`WAITED_ON`] still outstanding leaves it where it is.
///
/// And every pull request's checks and comments beside it, which is the part
/// that is read off the record rather than written out: a Conversation ends on
/// one pull request per repository it was worked in, each with a suite of its
/// own and a conversation of its own, and one still red or one with something
/// said on it that nobody has been sent to answer is a wrap-up still going. Read
/// inside the transaction with the settlements, so that a companion's pull
/// request recorded while this was deciding is one the decision waits for.
///
/// One transaction, as every move is, and the settlements are read inside it so
/// that the answer still holds when the update acts on it — which is what makes
/// two watchers asking at once safe: the first makes the move and the second
/// finds a Conversation that is not wrapping up any more.
///
/// What it does *not* wait for is the merge. Done means Verkstead has finished
/// with the work, not that it is on `main`.
pub async fn finish_wrap_up(pool: &SqlitePool, conversation_id: i64) -> Result<Finished> {
    let mut tx = super::begin_writing(pool)
        .await
        .context("finishing a wrap-up")?;

    let row: Option<(String,)> = sqlx::query_as("SELECT state FROM conversations WHERE id = ?")
        .bind(conversation_id)
        .fetch_optional(&mut *tx)
        .await
        .with_context(|| format!("reading the state of Conversation {conversation_id}"))?;

    let Some((state,)) = row else {
        return Ok(Finished::NoSuchConversation);
    };

    if Lifecycle::read(&state)? != Lifecycle::Wrapping {
        return Ok(Finished::NotWrapping);
    }

    let settled: Vec<(String, i64)> =
        sqlx::query_as("SELECT waiting_on, repo_id FROM wrap_up_settled WHERE conversation_id = ?")
            .bind(conversation_id)
            .fetch_all(&mut *tx)
            .await
            .with_context(|| {
                format!("reading what the wrap-up of Conversation {conversation_id} has settled")
            })?;

    let settled: Vec<WaitingOn> = settled
        .into_iter()
        .map(|(waiting_on, repo_id)| WaitingOn::read(&waiting_on, repo_id))
        .collect::<Result<_>>()?;

    let opened: Vec<(i64,)> =
        sqlx::query_as("SELECT repo_id FROM pull_requests WHERE conversation_id = ?")
            .bind(conversation_id)
            .fetch_all(&mut *tx)
            .await
            .with_context(|| {
                format!("reading which pull requests Conversation {conversation_id} is on")
            })?;

    let waiting_on = WAITED_ON.into_iter().chain(
        opened
            .into_iter()
            .flat_map(|(repo_id,)| [WaitingOn::Checks(repo_id), WaitingOn::Comments(repo_id)]),
    );

    if !waiting_on.into_iter().all(|one| settled.contains(&one)) {
        return Ok(Finished::StillWaiting);
    }

    sqlx::query("UPDATE conversations SET state = ? WHERE id = ?")
        .bind(Lifecycle::Done.stored())
        .bind(conversation_id)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("moving Conversation {conversation_id} to done"))?;

    moved(&mut tx, conversation_id, Lifecycle::Done).await?;

    tx.commit().await.context("finishing a wrap-up")?;

    Ok(Finished::Done)
}

/// Forget everything a Conversation's wrap-up has settled and everything its
/// checks have been given, so a second round wraps up from nothing.
///
/// What a steer into Grilling does — see [`super::steer_conversation`], whose
/// transaction this runs in. A round that inherited the round before it would
/// reach Wrapping with every one of the things wrap-up waits on already settled,
/// and would be over the moment it arrived.
///
/// The comments already addressed are deliberately left: a comment somebody
/// wrote and a session answered stays answered, and forgetting it would
/// dispatch a session about yesterday's feedback.
pub(crate) async fn forget_the_round(
    tx: &mut sqlx::SqliteConnection,
    conversation_id: i64,
) -> Result<()> {
    sqlx::query("DELETE FROM wrap_up_settled WHERE conversation_id = ?")
        .bind(conversation_id)
        .execute(&mut *tx)
        .await
        .with_context(|| {
            format!("forgetting what the wrap-up of Conversation {conversation_id} settled")
        })?;

    sqlx::query("DELETE FROM check_fix_attempts WHERE conversation_id = ?")
        .bind(conversation_id)
        .execute(&mut *tx)
        .await
        .with_context(|| {
            format!("forgetting what has been tried about Conversation {conversation_id}'s checks")
        })?;

    Ok(())
}

/// Forget what a Conversation's checks have already been given, so they start
/// again from nothing.
///
/// What Resume does. The human has read the Notice of what stopped and asked
/// for another go, and a count left standing would be a watcher that stopped all
/// over again on its next poll without dispatching anything.
pub async fn forget_fix_attempts(pool: &SqlitePool, conversation_id: i64) -> Result<()> {
    sqlx::query("DELETE FROM check_fix_attempts WHERE conversation_id = ?")
        .bind(conversation_id)
        .execute(pool)
        .await
        .with_context(|| {
            format!("forgetting what has been tried about Conversation {conversation_id}'s checks")
        })?;

    Ok(())
}
