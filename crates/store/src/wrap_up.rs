//! What wrap-up is still waiting on, how many goes the machine has had at a red
//! check, which comments it has already dispatched about — and the move to Done
//! that having settled all of it is.
//!
//! Everything here but the review is kept per pull request rather than per
//! Conversation: the checks that have settled, the goes one of them has had,
//! whether anything said is left unaddressed, and which comments a session has
//! been dispatched about. A Conversation ends on one pull request per repository
//! it was worked in and as many in one repository as its stack is deep, each with
//! a suite of its own and a conversation of its own — `Rust` red on two of them is
//! two failures rather than one, and a human writing on one of them is not writing
//! on the other — so which pull request a row is about is part of what it is. The
//! review is one review across the whole of it and is written against no pull
//! request at all.
//!
//! **Which pull request is the Repo and the number together**, that being what a
//! pull request *is* to Verkstead: `#41` names something else in the next
//! repository along, or nothing, and one repository holds as many numbers as its
//! stack is deep. The one count that stays keyed by the Repo alone is the goes
//! spent on a conflict, and it stays there on purpose — see
//! [`conflict_fix_attempts`].
//!
//! Four small tables and one Timeline Event, which is the whole shape of this
//! module. Everything else a human reads about wrap-up is already an Event — the
//! pull request, the commits a fix session lands, the Notice of the stop where
//! it stops asking the machine. What is kept here is the bookkeeping underneath:
//! facts that decide what Verkstead does next and that nobody would want a row
//! on a Timeline for.
//!
//! All four survive a restart, and all four have to. A server that came back
//! up having forgotten how many fix sessions a check had already had would
//! dispatch them again for ever, which is exactly the failure *two attempts,
//! then ask the human* exists to prevent; one that had forgotten which comments
//! it had read would dispatch a session about feedback that was addressed
//! yesterday; one that had forgotten it had already said a wrap-up was down to
//! its checks would say it a second time on the same Timeline.
//!
//! What is *settled* is written down and what is outstanding is not: the checks,
//! the comments and whether the branch merges are asked of GitHub on every poll,
//! so a red suite needs no memory. The row is deleted again the moment one of
//! them stops being true — see [`unsettle_wrap_up`] — which is what makes a
//! commit pushed to the pull request put its checks back to waiting rather than
//! leaving yesterday's green standing, a comment landing after a quiet spell
//! something to deal with, and a base moving under the branch a conflict to
//! resolve.

use anyhow::{Context, Result, bail};
use sqlx::SqlitePool;

use super::conversations::{Lifecycle, moved};

/// One of the things a Conversation has to have settled before wrap-up is over.
///
/// Four kinds of thing and nothing else, though not four settlements: the
/// checks, what has been said and whether the branch merges are one each per
/// pull request, and a Conversation ends on one pull request per repository it
/// was worked in and as many in one repository as its stack is deep, so a
/// wrap-up with a companion is waiting on seven things rather than four, and one
/// over a stack of three on ten. What is *not* here is the merge itself: stages
/// stack on unmerged predecessors, so a Conversation that stayed in Wrapping
/// until its pull request landed would hold up every stage behind it — and
/// merging is the human act this pipeline is built around rather than a step in
/// it. *Can be merged* and *has been merged* are different facts, and only the
/// first is something Verkstead waits for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitingOn {
    /// The checks are green on pull request `number` of this Repo.
    ///
    /// One per pull request rather than one per Conversation, because a suite
    /// is a fact about a pull request: a Conversation ends on one per
    /// repository it was worked in and as many in one repository as its stack
    /// is deep, each with its own checks running against its own branch, and a
    /// wrap-up that settled *the* checks would be one where a green companion
    /// stood for a red one — or the top of a stack for the bottom of it.
    ///
    /// The Repo *and* the number, because that is what a pull request is: the
    /// number alone means something else in the next repository along, and the
    /// Repo alone is one settlement over a chain of branches.
    Checks { repo_id: i64, number: i64 },

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
    ///
    /// And a steer into Wrapping takes it too, from whatever state the human
    /// steered from: a steer is them saying *look at this again*, so the wrap-up
    /// it lands in reads the branch rather than inheriting what the last one made
    /// of it. See [`super::steer_conversation`], and [`super::resolve_conflicts`]
    /// for the one move into Wrapping that leaves it standing.
    Review,

    /// Nothing has been said on pull request `number` of this Repo that has not
    /// had a session dispatched about it.
    ///
    /// Like the checks and unlike the review: a comment landing after this
    /// settled unsettles it again, because a wrap-up that stopped reading its
    /// pull request the first time it went quiet would be one a human could not
    /// reach.
    ///
    /// And per pull request for the checks' reason. A human writes on the pull
    /// request they are reading, and a Conversation ends on one per repository
    /// it was worked in and as many in one repository as its stack is deep: a
    /// wrap-up that settled *the* comments would be one where a quiet companion
    /// stood for a busy one, and the pull request that went quiet is the one
    /// that has nothing outstanding on it.
    Comments { repo_id: i64, number: i64 },

    /// GitHub can merge pull request `number` of this Repo into its base.
    ///
    /// Like the checks in every way that matters: per pull request, because a
    /// conflict is a fact about one branch and its base; settled by GitHub
    /// saying so and unsettled by GitHub saying otherwise, because a base that
    /// moves under a branch puts a pull request in conflict long after anybody
    /// touched it.
    ///
    /// Which is also why it is the pull request's rather than the repository's
    /// in a stack: the branches of one are each other's bases, so a conflict low
    /// in the chain is a fact about that branch and everything above it merges
    /// or does not on its own account.
    ///
    /// What it is here for is Done. A Conversation that finished over a
    /// conflicted pull request would be one Verkstead had called done and
    /// nobody could land — and waiting on it is also what closes the race where
    /// a conflict appears just as the last suite goes green.
    Mergeable { repo_id: i64, number: i64 },
}

impl WaitingOn {
    /// The word the column holds. Lowercase and spelled out, so a database
    /// opened by hand says something.
    fn stored(self) -> &'static str {
        match self {
            Self::Checks { .. } => "checks",
            Self::Review => "review",
            Self::Comments { .. } => "comments",
            Self::Mergeable { .. } => "mergeable",
        }
    }

    /// Which repository the pull request it is about was opened in.
    ///
    /// [`NO_PULL_REQUEST`] for the ones that are about the wrap-up as a
    /// whole, which is what the review is: one review across every pull
    /// request the Conversation ended on.
    fn repo(self) -> i64 {
        match self {
            Self::Checks { repo_id, .. }
            | Self::Comments { repo_id, .. }
            | Self::Mergeable { repo_id, .. } => repo_id,
            Self::Review => NO_PULL_REQUEST,
        }
    }

    /// And which pull request of that repository, which is the other half of
    /// the same answer: a stack is several numbers in one Repo.
    ///
    /// [`NO_PULL_REQUEST`] again for the review, whose row names neither.
    fn number(self) -> i64 {
        match self {
            Self::Checks { number, .. }
            | Self::Comments { number, .. }
            | Self::Mergeable { number, .. } => number,
            Self::Review => NO_PULL_REQUEST,
        }
    }

    /// The one a stored word and a pull request between them. An unknown word is
    /// a database written by a Verkstead this one does not understand,
    /// exactly as an unknown lifecycle state is.
    fn read(word: &str, repo_id: i64, number: i64) -> Result<Self> {
        Ok(match word {
            "checks" => Self::Checks { repo_id, number },
            "review" => Self::Review,
            "comments" => Self::Comments { repo_id, number },
            "mergeable" => Self::Mergeable { repo_id, number },
            other => bail!("a wrap-up is waiting on the unknown thing {other:?}"),
        })
    }
}

/// What stands in the Repo and the number of a settlement about no pull request.
///
/// Zero in both, which is no repository and no pull request: SQLite hands rowids
/// out from one and GitHub numbers pull requests from one, so nothing real can
/// collide with it. Which is also why the Repo column is not a foreign key — a
/// reference to `repos` would refuse the review's own row.
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

/// What a wrap-up has settled, inside whatever transaction is asking.
///
/// [`wrap_up_settled`]'s own reading, shared with the two rules that read it
/// beside the record of what a Conversation is on — see [`narrowed`] and
/// [`finish_wrap_up`]: both have to read the two together or they would be
/// deciding off half a moment.
async fn settled(tx: &mut sqlx::SqliteConnection, conversation_id: i64) -> Result<Vec<WaitingOn>> {
    let rows: Vec<(String, i64, i64)> = sqlx::query_as(
        "SELECT waiting_on, repo_id, number FROM wrap_up_settled WHERE conversation_id = ?",
    )
    .bind(conversation_id)
    .fetch_all(&mut *tx)
    .await
    .with_context(|| {
        format!("reading what the wrap-up of Conversation {conversation_id} has settled")
    })?;

    rows.into_iter()
        .map(|(waiting_on, repo_id, number)| WaitingOn::read(&waiting_on, repo_id, number))
        .collect()
}

/// Which pull requests a Conversation is on, as the Repo and the number of each.
///
/// The rest of what a wrap-up waits on, and read off the record rather than
/// written out for the reason [`WAITED_ON`] gives: a Conversation ends on one per
/// repository it was worked in and as many in one repository as its stack is
/// deep, and which those are is a fact about the record.
///
/// Read inside whatever transaction is asking, so that a pull request recorded
/// while the caller was deciding is one the decision waits for — see
/// [`finish_wrap_up`], which is the decision.
///
/// The Repo is off the pull request's own row rather than off the Conversation:
/// a companion's pull request is in another repository, and a stack's are all in
/// one.
async fn opened(tx: &mut sqlx::SqliteConnection, conversation_id: i64) -> Result<Vec<(i64, i64)>> {
    let rows: Vec<(i64, i64)> = sqlx::query_as(
        "SELECT repo_id, number FROM pull_requests
         WHERE conversation_id = ?
         ORDER BY event_id",
    )
    .bind(conversation_id)
    .fetch_all(&mut *tx)
    .await
    .with_context(|| format!("reading which pull requests Conversation {conversation_id} is on"))?;

    Ok(rows)
}

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

/// The five tables wrap-up keeps its bookkeeping in.
///
/// All of them hang off a Conversation rather than off a Timeline Event, unlike
/// nearly everything else here, and that is the point: none of them is something
/// that happened, so none of them is something to draw. They are what Verkstead
/// knows about a Conversation it is wrapping up.
pub(crate) async fn apply_schema(pool: &SqlitePool) -> Result<()> {
    // The Repo and the number are which pull request a settlement is about, and
    // part of what it is rather than a note beside it: the checks settle per pull
    // request. [`NO_PULL_REQUEST`] stands in both for the ones about the whole
    // wrap-up, which is why the Repo column references nothing.
    //
    // A database written before a Conversation could end on more than one pull
    // request has neither of them nor the rule, and one written before a
    // repository could hold more than one has the Repo and not the number. Both
    // are [`super::migrations`]'s to put right as the database opens rather than
    // this function's: the rule is declared inline as the primary key, so it is
    // the table itself that has to be rebuilt, and that is not something a
    // `CREATE TABLE IF NOT EXISTS` can reach.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS wrap_up_settled (
             conversation_id INTEGER NOT NULL REFERENCES conversations(id),
             repo_id         INTEGER NOT NULL,
             number          INTEGER NOT NULL,
             waiting_on      TEXT NOT NULL,
             at              TEXT NOT NULL,
             PRIMARY KEY (conversation_id, repo_id, number, waiting_on)
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
    // And by the pull request it went red on, for the reason the settlements
    // above name one: the same check name red on two pull requests is two
    // different failures, and one spending the other's attempts would stop a run
    // that still had somewhere to go. Two of them in one repository is exactly as
    // much two failures as two repositories' are, which is why the number is part
    // of the key and not merely a fact on the row.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS check_fix_attempts (
             conversation_id INTEGER NOT NULL REFERENCES conversations(id),
             repo_id         INTEGER NOT NULL REFERENCES repos(id),
             number          INTEGER NOT NULL,
             check_name      TEXT NOT NULL,
             attempts        INTEGER NOT NULL,
             PRIMARY KEY (conversation_id, repo_id, number, check_name)
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the check fix attempts table")?;

    // And what has been tried at getting a repository's pull requests to merge,
    // counted per repository rather than per anything on them: a conflict is one
    // fact about one branch and its base, and there is nothing on it to key by the
    // way a suite has check names.
    //
    // **The Repo without the number, which is the one key here that did not move
    // onto the pull request** — and deliberately, rather than the one table
    // nobody got round to. The pull requests of one repository are a stack, whose
    // branches are each other's bases: a fix low in the chain changes everything
    // above it, so what a resolution is dispatched about is the stack rather than
    // one of its branches. Counting by the Repo is counting per stack, which is
    // what the goes over a conflict are meant to be counted by. See
    // [`conflict_fix_attempts`].
    //
    // A table of its own beside the checks' rather than a row among them, for
    // the reason the merges table sits beside the rollup: what a resolution
    // session is dispatched about is not a check, and a check name borrowed to
    // sit here under would be one GitHub could one day report for real.
    //
    // Written down for the checks' attempts' reason as well: an attempt spent by
    // a server that then restarted is one the next server must not spend again.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS conflict_fix_attempts (
             conversation_id INTEGER NOT NULL REFERENCES conversations(id),
             repo_id         INTEGER NOT NULL REFERENCES repos(id),
             attempts        INTEGER NOT NULL,
             PRIMARY KEY (conversation_id, repo_id)
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the conflict fix attempts table")?;

    // Keyed by what GitHub calls the comment, which is what survives a restart:
    // a server that came back up and read every comment as new would dispatch a
    // session about feedback that was addressed yesterday.
    //
    // And by the Repo the pull request it was left on is in, which is as fine as
    // this one needs to be. GitHub's ids are unique across repositories, so what
    // keeps two comments apart is the id: the Repo narrows the read to a
    // repository's own, and a row that came from the pull request below in a stack
    // is a comment the watcher above never read and so never asks about. Which is
    // why this is not among the tables the stack moved onto the pull request — what
    // settles per pull request is the *settlement* above, and it already does.
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

    // And the mark that says the Notice of a wrap-up narrowing to its checks
    // has been written. One row or none per Conversation, because the condition
    // is either on or off — and the row going away again is what makes a second
    // narrowing a second Notice rather than a silence.
    //
    // Written down rather than remembered, for the reason the three above are:
    // a wrap-up sits narrowed for as long as a suite takes, which is longer
    // than a server being restarted stays up, and a watcher that came back
    // having forgotten would say the same thing on the same Timeline twice.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS wrap_up_narrowings (
             conversation_id INTEGER NOT NULL PRIMARY KEY REFERENCES conversations(id),
             at              TEXT NOT NULL
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the wrap-up narrowings table")?;

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
    let mut connection = pool
        .acquire()
        .await
        .context("settling something a wrap-up waits on")?;

    settle(&mut connection, conversation_id, waiting_on).await
}

/// The same, inside a transaction that is doing something else as well.
///
/// Which is a Conversation entering Wrapping with something already settled,
/// rather than settling it once the watchers have looked — the shape the resolve
/// press has for the review, widened to be written rather than merely left
/// standing. A **Fix Merge Issues** is what wants it: its wrap-up is narrowed to
/// what GitHub refuses a merge for, so the review and its pull request's comments
/// are settled as it lands, in or before the transaction that makes the move, and
/// no sweep or restart can find it wrapping up without them. See
/// [`super::take_up`], which is the door.
pub(crate) async fn settle(
    tx: &mut sqlx::SqliteConnection,
    conversation_id: i64,
    waiting_on: WaitingOn,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO wrap_up_settled (conversation_id, repo_id, number, waiting_on, at)
         VALUES (?, ?, ?, ?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
         ON CONFLICT (conversation_id, repo_id, number, waiting_on) DO NOTHING",
    )
    .bind(conversation_id)
    .bind(waiting_on.repo())
    .bind(waiting_on.number())
    .bind(waiting_on.stored())
    .execute(&mut *tx)
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
/// Which is the two moves the review's settle does not survive, each taking it in
/// the same breath as the state changes. Leaving Wrapping to build a split-out
/// backlog takes it, so a Conversation being built again is never one carrying a
/// settled review of work that has not been done yet — see
/// [`super::implement_again`]; and a steer into Wrapping takes it, so the wrap-up
/// the human asked for reads the branch rather than inheriting what the last one
/// made of it — see [`super::steer_conversation`].
pub(crate) async fn unsettle(
    tx: &mut sqlx::SqliteConnection,
    conversation_id: i64,
    waiting_on: WaitingOn,
) -> Result<()> {
    sqlx::query(
        "DELETE FROM wrap_up_settled
         WHERE conversation_id = ? AND repo_id = ? AND number = ? AND waiting_on = ?",
    )
    .bind(conversation_id)
    .bind(waiting_on.repo())
    .bind(waiting_on.number())
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

/// Record that the review is over, and put every pull request's checks back to
/// waiting with it.
///
/// **One transaction, because the two are one fact.** A review lands whatever
/// the human accepted and pushes it as it ends, so a suite that was green is
/// green about the commit before that push and every one of them has to run
/// again. A settle written without the unsettle beside it — or a poll of
/// [`finish_wrap_up`] reading between the two — is a Conversation reaching Done
/// on a green nobody re-earned, which is the same failure
/// [`super::follow_up_over`] takes a `pushed` for.
///
/// Every pull request rather than one: a review reads the work whole and pushes
/// into whichever worktree it fixed something in, and none of the suites it may
/// have replaced is this Conversation's to keep.
///
/// A review that pushed nothing — one that found nothing worth raising, or one
/// the human turned off — costs a poll of the checks and nothing else. The next
/// look settles the suite again, and nothing could have finished in the
/// meantime: the review is what the wrap-up was waiting on.
pub async fn review_over(pool: &SqlitePool, conversation_id: i64) -> Result<()> {
    let mut tx = super::writing(pool, "recording that a review is over").await?;

    for (repo_id, number) in opened(&mut tx, conversation_id).await? {
        unsettle(
            &mut tx,
            conversation_id,
            WaitingOn::Checks { repo_id, number },
        )
        .await?;
    }

    sqlx::query(
        "INSERT INTO wrap_up_settled (conversation_id, repo_id, number, waiting_on, at)
         VALUES (?, ?, ?, ?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
         ON CONFLICT (conversation_id, repo_id, number, waiting_on) DO NOTHING",
    )
    .bind(conversation_id)
    .bind(WaitingOn::Review.repo())
    .bind(WaitingOn::Review.number())
    .bind(WaitingOn::Review.stored())
    .execute(&mut *tx)
    .await
    .with_context(|| format!("settling the review of Conversation {conversation_id}"))?;

    tx.commit()
        .await
        .context("recording that a review is over")?;

    Ok(())
}

/// Put every pull request's checks back to waiting, a batch session having just
/// ended over them.
///
/// The third of these, and the same fact each time: a session that lands what
/// the human accepted pushes it, and a suite that was green is green about the
/// commit before that push. [`review_over`] takes it with the review's settle
/// and [`super::follow_up_over`] with the move out of Follow-up; a batch has
/// neither to be carried by, because what settles a batch is the comments
/// watcher noticing on its next poll that nothing is left unaddressed. So this
/// is the whole of the act rather than half of one.
///
/// **Which is why it is written before that poll can happen.** The settle and
/// the unsettle are two different watchers' writes, and the finishing rule is a
/// third reader between them — so the ordering has to be a rule rather than a
/// cadence, exactly as it does for a review. See [`finish_wrap_up`], and
/// `crate::responding` on the server, which is where a batch session ends.
///
/// One transaction across all of them, so that no reading of the table catches
/// half a fact.
///
/// Every pull request rather than the one the batch was dispatched about: what
/// a comment asks for is fixed wherever the thing it is about lives, and a
/// session sent at one pull request's comments may well commit in a companion's
/// worktree beside it — the addressing skill is written for exactly that.
///
/// A batch that pushed nothing costs a poll of the checks and nothing else. The
/// next look settles them again, and nothing could have finished in the
/// meantime: the comments this batch was dispatched about are unaddressed for as
/// long as it runs, and a wrap-up does not finish over those.
pub async fn batch_over(pool: &SqlitePool, conversation_id: i64) -> Result<()> {
    let mut tx = super::writing(pool, "recording that a batch session is over").await?;

    for (repo_id, number) in opened(&mut tx, conversation_id).await? {
        unsettle(
            &mut tx,
            conversation_id,
            WaitingOn::Checks { repo_id, number },
        )
        .await?;
    }

    tx.commit()
        .await
        .context("recording that a batch session is over")?;

    Ok(())
}

/// What a Conversation's wrap-up has settled so far.
///
/// The whole set rather than one asked about at a time, because what it is for
/// is the question *is wrap-up over* — which is about all of them together.
pub async fn wrap_up_settled(pool: &SqlitePool, conversation_id: i64) -> Result<Vec<WaitingOn>> {
    let rows: Vec<(String, i64, i64)> = sqlx::query_as(
        "SELECT waiting_on, repo_id, number FROM wrap_up_settled WHERE conversation_id = ?",
    )
    .bind(conversation_id)
    .fetch_all(pool)
    .await
    .with_context(|| {
        format!("reading what the wrap-up of Conversation {conversation_id} has settled")
    })?;

    rows.into_iter()
        .map(|(waiting_on, repo_id, number)| WaitingOn::read(&waiting_on, repo_id, number))
        .collect()
}

/// Whether a wrap-up has narrowed to its checks: the review answered, the
/// comments dealt with, every pull request merging, the checks alone left
/// outstanding, and the Conversation still Wrapping.
///
/// Half of the condition the human reads as **Waiting on checks**. The other
/// half is that nothing is running in the Worktree, which is a fact about a
/// process rather than about a row and so belongs to the caller — see
/// [`narrowing`], which takes it.
///
/// Derived every time it is asked rather than stored: it is the settle facts
/// read a particular way, and a column saying the same thing would be a second
/// answer to go wrong.
pub async fn narrowed_to_checks(pool: &SqlitePool, conversation_id: i64) -> Result<bool> {
    let mut connection = pool
        .acquire()
        .await
        .context("reading whether a wrap-up is down to its checks")?;

    narrowed(&mut connection, conversation_id).await
}

/// The same question inside a transaction, which is where [`narrowing`] asks it.
async fn narrowed(tx: &mut sqlx::SqliteConnection, conversation_id: i64) -> Result<bool> {
    let row: Option<(String,)> = sqlx::query_as("SELECT state FROM conversations WHERE id = ?")
        .bind(conversation_id)
        .fetch_optional(&mut *tx)
        .await
        .with_context(|| format!("reading the state of Conversation {conversation_id}"))?;

    let Some((state,)) = row else {
        return Ok(false);
    };

    if Lifecycle::read(&state)? != Lifecycle::Wrapping {
        return Ok(false);
    }

    let settled = settled(&mut *tx, conversation_id).await?;
    let opened = opened(&mut *tx, conversation_id).await?;

    // Everything else settled and some suite still running: narrowing is a
    // wrap-up having got down to the one thing nothing here can hurry, which is
    // why it is worth saying out loud rather than leaving as plain Wrapping.
    //
    // Read off the same facts [`finish_wrap_up`] reads, and asked the same way:
    // the review once for the whole of the work, and the comments and the merge
    // once per pull request the Conversation ended on. One suite outstanding is
    // enough — a wrap-up down to a companion's red checks is as narrowed as one
    // down to its own, and both are waiting on the same thing.
    //
    // The merge is one of the things that has to have settled, rather than
    // something to narrow to: a wrap-up whose pull request conflicts is not
    // waiting on GitHub to finish anything, and saying it was waiting on its
    // checks would send the human off to watch a suite that was never the
    // problem.
    let everything_else = WAITED_ON
        .into_iter()
        .chain(opened.iter().flat_map(|&(repo_id, number)| {
            [
                WaitingOn::Comments { repo_id, number },
                WaitingOn::Mergeable { repo_id, number },
            ]
        }))
        .all(|one| settled.contains(&one));

    let a_suite_outstanding = opened
        .iter()
        .any(|&(repo_id, number)| !settled.contains(&WaitingOn::Checks { repo_id, number }));

    Ok(everything_else && a_suite_outstanding)
}

/// What a look at whether a wrap-up has narrowed found, and what the looker owes
/// the Timeline for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Narrowing {
    /// It has narrowed, and this is the first look to say so: the Notice is the
    /// caller's to write.
    Narrowed,

    /// It has narrowed and the Notice is already on the Timeline, which is every
    /// look after the first for as long as the condition holds.
    NoticedAlready,

    /// It has not — or not any more, in which case the mark is now gone and the
    /// next narrowing is a fresh Notice rather than a silence.
    NotNarrowed,
}

/// Ask whether a wrap-up has narrowed to its checks, and keep the mark that says
/// its Notice has been written.
///
/// `working` is whether a session is running in the Conversation's Worktree,
/// which is the half of the condition the store cannot see: a fix session
/// actively working a red check is a wrap-up getting on with it rather than one
/// waiting, and reads here as not narrowed.
///
/// One transaction, so that the answer still holds when the mark acts on it —
/// which is what makes two watchers asking at once safe, a Resume over a stopped
/// wrap-up being how there come to be two. The first is told to write the Notice
/// and the second finds it written.
///
/// The mark going away again is the whole of what makes this *once per
/// narrowing*: a fix session dispatched or a comment landing puts the answer
/// back to no, the row goes with it, and the narrowing after that is a Notice of
/// its own.
pub async fn narrowing(
    pool: &SqlitePool,
    conversation_id: i64,
    working: bool,
) -> Result<Narrowing> {
    let mut tx = pool
        .begin()
        .await
        .context("looking at whether a wrap-up has narrowed to its checks")?;

    let has_narrowed = !working && narrowed(&mut tx, conversation_id).await?;

    let outcome = if has_narrowed {
        let written = sqlx::query(
            "INSERT INTO wrap_up_narrowings (conversation_id, at)
             VALUES (?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
             ON CONFLICT (conversation_id) DO NOTHING",
        )
        .bind(conversation_id)
        .execute(&mut *tx)
        .await
        .with_context(|| {
            format!("marking the wrap-up of Conversation {conversation_id} as down to its checks")
        })?
        .rows_affected();

        if written > 0 {
            Narrowing::Narrowed
        } else {
            Narrowing::NoticedAlready
        }
    } else {
        // Only where there is one to take off, which is what keeps the ordinary
        // poll a read. A wrap-up waiting on its review is asked this on the
        // settling loop's own cadence for as long as the review takes and
        // answers *not narrowed* every time, so a delete run unconditionally
        // would be a write and a commit per poll for a row that was never
        // there — and two watchers asking at once, which this is arranged to be
        // safe under, would be two write locks contending rather than two
        // readers. A deferred transaction that has only read takes no write
        // lock at all.
        if marked(&mut tx, conversation_id).await? {
            unmark(&mut tx, conversation_id).await?;
        }

        Narrowing::NotNarrowed
    };

    tx.commit()
        .await
        .context("looking at whether a wrap-up has narrowed to its checks")?;

    Ok(outcome)
}

/// Take the mark off again without asking anything, so the next look at a
/// wrap-up still down to its checks is told to write the line afresh.
///
/// What a caller does when the Notice it was told to write would not write: the
/// mark says the line is on the Timeline, and one standing over a line that
/// never landed is a narrowing said nowhere at all.
pub async fn forget_narrowing(pool: &SqlitePool, conversation_id: i64) -> Result<()> {
    let mut connection = pool
        .acquire()
        .await
        .context("forgetting that a wrap-up was down to its checks")?;

    unmark(&mut connection, conversation_id).await
}

/// Whether the mark saying a narrowing was said out loud is there.
///
/// What [`narrowing`] asks before it deletes, so that the poll which changes
/// nothing — every poll of a wrap-up that has not narrowed, which is most of
/// one — costs a read rather than a write. Nothing else asks: the condition
/// itself is [`narrowed`]'s to read off the settle facts, and this is only ever
/// about the row.
async fn marked(tx: &mut sqlx::SqliteConnection, conversation_id: i64) -> Result<bool> {
    let found: Option<(i64,)> =
        sqlx::query_as("SELECT conversation_id FROM wrap_up_narrowings WHERE conversation_id = ?")
            .bind(conversation_id)
            .fetch_optional(&mut *tx)
            .await
            .with_context(|| {
                format!(
                    "reading whether the wrap-up of Conversation {conversation_id} had been \
                     said to be down to its checks"
                )
            })?;

    Ok(found.is_some())
}

/// The delete both of them are, so the two cannot come to disagree about which
/// row it is.
async fn unmark(tx: &mut sqlx::SqliteConnection, conversation_id: i64) -> Result<()> {
    sqlx::query("DELETE FROM wrap_up_narrowings WHERE conversation_id = ?")
        .bind(conversation_id)
        .execute(&mut *tx)
        .await
        .with_context(|| {
            format!(
                "forgetting that the wrap-up of Conversation {conversation_id} was down to \
                 its checks"
            )
        })?;

    Ok(())
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
         WHERE conversation_id = ? AND repo_id = ? AND number = ? AND waiting_on = ?",
    )
    .bind(conversation_id)
    .bind(waiting_on.repo())
    .bind(waiting_on.number())
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

/// How many fix sessions this check has already had on pull request `number` of
/// Repo `repo_id`.
///
/// Zero for a check nothing has been dispatched for, which is every check the
/// first time it goes red.
///
/// The pull request is part of the question rather than a filter on it: the same
/// check name red on two of a Conversation's pull requests is two different
/// failures, and one spending the other's attempts would stop a run that still
/// had somewhere to go. Two of them in one repository — a stack — no less than
/// two in two.
pub async fn fix_attempts(
    pool: &SqlitePool,
    conversation_id: i64,
    repo_id: i64,
    number: i64,
    check: &str,
) -> Result<i64> {
    let row: Option<(i64,)> = sqlx::query_as(
        "SELECT attempts FROM check_fix_attempts
         WHERE conversation_id = ? AND repo_id = ? AND number = ? AND check_name = ?",
    )
    .bind(conversation_id)
    .bind(repo_id)
    .bind(number)
    .bind(check)
    .fetch_optional(pool)
    .await
    .with_context(|| {
        format!(
            "reading what has been tried about {check:?} on pull request #{number} of Repo \
             {repo_id} on Conversation {conversation_id}"
        )
    })?;

    Ok(row.map(|(attempts,)| attempts).unwrap_or(0))
}

/// The most any one check on pull request `number` of Repo `repo_id` has had.
///
/// Zero where nothing has been dispatched about that pull request at all, which
/// is a suite that has never been red and one that is still running alike.
///
/// What *has this pull request run out of goes* is asked with, by a watcher
/// deciding whether the run has anywhere left to go — see `checks::owed_elsewhere`
/// in the server. The most rather than the least, because a check that has never
/// failed has no row here at all and would otherwise read as one with both its
/// goes still in hand.
pub async fn most_fix_attempts(
    pool: &SqlitePool,
    conversation_id: i64,
    repo_id: i64,
    number: i64,
) -> Result<i64> {
    let (attempts,): (i64,) = sqlx::query_as(
        "SELECT COALESCE(MAX(attempts), 0) FROM check_fix_attempts
         WHERE conversation_id = ? AND repo_id = ? AND number = ?",
    )
    .bind(conversation_id)
    .bind(repo_id)
    .bind(number)
    .fetch_one(pool)
    .await
    .with_context(|| {
        format!(
            "reading what has been tried about the checks on pull request #{number} of Repo \
             {repo_id} on Conversation {conversation_id}"
        )
    })?;

    Ok(attempts)
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
    number: i64,
    check: &str,
) -> Result<i64> {
    let (attempts,): (i64,) = sqlx::query_as(
        "INSERT INTO check_fix_attempts (conversation_id, repo_id, number, check_name, attempts)
         VALUES (?, ?, ?, ?, 1)
         ON CONFLICT (conversation_id, repo_id, number, check_name)
             DO UPDATE SET attempts = attempts + 1
         RETURNING attempts",
    )
    .bind(conversation_id)
    .bind(repo_id)
    .bind(number)
    .bind(check)
    .fetch_one(pool)
    .await
    .with_context(|| {
        format!(
            "counting a fix session for {check:?} on pull request #{number} of Repo {repo_id} \
             on Conversation {conversation_id}"
        )
    })?;

    Ok(attempts)
}

/// How many resolution sessions the pull requests of Repo `repo_id` have already
/// had between them at their conflicts.
///
/// Zero for a repository nothing has been dispatched about, which is every one of
/// them the first time GitHub says something will not merge.
///
/// Per repository and nothing finer, unlike the checks beside it. Two reasons,
/// and they point the same way: a suite is many checks where a branch has one
/// base, so *this will not merge* is the whole of what there is to count; and the
/// pull requests of one repository are a **stack**, whose branches are each
/// other's bases — so a resolution is one act over the whole chain rather than
/// one per branch, and a count per branch would be the same act charged three
/// times. Counting by the Repo is counting per stack.
pub async fn conflict_fix_attempts(
    pool: &SqlitePool,
    conversation_id: i64,
    repo_id: i64,
) -> Result<i64> {
    let row: Option<(i64,)> = sqlx::query_as(
        "SELECT attempts FROM conflict_fix_attempts
         WHERE conversation_id = ? AND repo_id = ?",
    )
    .bind(conversation_id)
    .bind(repo_id)
    .fetch_optional(pool)
    .await
    .with_context(|| {
        format!(
            "reading what has been tried about the conflict in Repo {repo_id} on Conversation \
             {conversation_id}"
        )
    })?;

    Ok(row.map(|(attempts,)| attempts).unwrap_or(0))
}

/// Count one more, and say how many that makes.
///
/// Counted as the session is dispatched rather than as it ends, for
/// [`record_fix_attempt`]'s reason: an attempt that was spent and not written
/// down would be one the next server spends again.
pub async fn record_conflict_fix_attempt(
    pool: &SqlitePool,
    conversation_id: i64,
    repo_id: i64,
) -> Result<i64> {
    let (attempts,): (i64,) = sqlx::query_as(
        "INSERT INTO conflict_fix_attempts (conversation_id, repo_id, attempts)
         VALUES (?, ?, 1)
         ON CONFLICT (conversation_id, repo_id)
             DO UPDATE SET attempts = attempts + 1
         RETURNING attempts",
    )
    .bind(conversation_id)
    .bind(repo_id)
    .fetch_one(pool)
    .await
    .with_context(|| {
        format!(
            "counting a resolution session for the conflict in Repo {repo_id} on Conversation \
             {conversation_id}"
        )
    })?;

    Ok(attempts)
}

/// Which of the comments left in Repo `repo_id` have already had a session
/// dispatched about them.
///
/// The whole set rather than one asked about at a time, because what it is for
/// is the question *which of these are new* — which is about all of them at once,
/// and the comments arrive from GitHub as a list.
///
/// One repository's rather than the Conversation's, because that is as much as
/// the question a watcher asks needs: it read one pull request's comments and is
/// deciding which of *those* to dispatch about, so what it hands back is a filter
/// rather than a list to work through, and a comment id is unique wherever it was
/// left.
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
    let mut tx = super::writing(pool, "recording which comments have been dispatched for").await?;

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
    let mut tx = super::writing(pool, "forgetting which comments have been dispatched for").await?;

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
/// And every pull request's checks, comments and merge beside it, which is the
/// part that is read off the record rather than written out: a Conversation ends
/// on one pull request per repository it was worked in and as many in one
/// repository as its stack is deep, each with a suite of its own, a conversation
/// of its own and a base of its own to merge into, and one still red, one with
/// something said on it that nobody has been sent to answer, or one GitHub cannot
/// merge is a wrap-up still going. Three in one repository are three suites,
/// three conversations and three merges to wait on. Read inside the transaction
/// with the settlements, so that a pull request recorded while this was deciding —
/// a companion's, or the next one up a chain — is one the decision waits for.
///
/// One transaction, as every move is, and the settlements are read inside it so
/// that the answer still holds when the update acts on it — which is what makes
/// two watchers asking at once safe: the first makes the move and the second
/// finds a Conversation that is not wrapping up any more.
///
/// What it does *not* wait for is the merge. Done means Verkstead has finished
/// with the work, not that it is on `main`.
pub async fn finish_wrap_up(pool: &SqlitePool, conversation_id: i64) -> Result<Finished> {
    let mut tx = super::writing(pool, "finishing a wrap-up").await?;

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

    let settled = settled(&mut tx, conversation_id).await?;

    let waiting_on = WAITED_ON.into_iter().chain(
        opened(&mut tx, conversation_id)
            .await?
            .into_iter()
            .flat_map(|(repo_id, number)| {
                [
                    WaitingOn::Checks { repo_id, number },
                    WaitingOn::Comments { repo_id, number },
                    WaitingOn::Mergeable { repo_id, number },
                ]
            }),
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

    sqlx::query("DELETE FROM conflict_fix_attempts WHERE conversation_id = ?")
        .bind(conversation_id)
        .execute(&mut *tx)
        .await
        .with_context(|| {
            format!(
                "forgetting what has been tried about Conversation {conversation_id}'s conflicts"
            )
        })?;

    // And the mark that says a narrowing was said out loud, so a second round
    // that gets down to its checks says so on its own account. The watcher takes
    // this one off itself the moment the condition ends — see [`narrowing`] —
    // and this is the case it never sees: a round steered away while the server
    // was down.
    unmark(&mut *tx, conversation_id).await?;

    Ok(())
}

/// Forget what a Conversation's checks and conflicts have already been given, so
/// they start again from nothing.
///
/// What Resume does, and a steer into Wrapping with it. The human has read the
/// Notice of what stopped and asked for another go, and a count left standing
/// would be a watcher that stopped all over again on its next poll without
/// dispatching anything.
///
/// Both counts, because either of them is what the Notice they read could have
/// been about: a wrap-up stops on the checks that would not go green or on the
/// pull request that would not merge, and a press that only forgave one of them
/// would be a Resume that stopped again on the other.
pub async fn forget_fix_attempts(pool: &SqlitePool, conversation_id: i64) -> Result<()> {
    sqlx::query("DELETE FROM check_fix_attempts WHERE conversation_id = ?")
        .bind(conversation_id)
        .execute(pool)
        .await
        .with_context(|| {
            format!("forgetting what has been tried about Conversation {conversation_id}'s checks")
        })?;

    sqlx::query("DELETE FROM conflict_fix_attempts WHERE conversation_id = ?")
        .bind(conversation_id)
        .execute(pool)
        .await
        .with_context(|| {
            format!(
                "forgetting what has been tried about Conversation {conversation_id}'s conflicts"
            )
        })?;

    Ok(())
}
