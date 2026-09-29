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
//! One row per pull request, by the unique index: the Conversation, the Repo and
//! the number. A Conversation working alongside read-write companions ends on one
//! per repository, and a Conversation wrapping up a **stack** holds several in
//! the one repository — a stack being a chain of pull requests each of whose
//! bases is the one below's head, all in the same place. So recording a second is
//! the same wrap-up learning about another pull request rather than a second
//! move.
//!
//! The Repo is part of that identity for the commits table's reason: two
//! repositories are two sets of numbers, and `#41` says nothing across them. The
//! number is part of it because one repository holds as many as a stack is deep.
//!
//! Which is why a PR recorded twice records nothing new. It is what makes a
//! second attempt at the same ending safe, and it is what a *second wrap* lands
//! on: a Conversation whose review split its findings out into a backlog leaves
//! Wrapping to build them and finishes again, and what its finish step opens is
//! the pull requests it already had. So each record is reused rather than written
//! twice, and the lifecycle moves either side of it are what tell the re-entry's
//! story on the Timeline.

use std::collections::HashMap;

use anyhow::{Context, Result, bail};
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

    /// The branch the work is on, unqualified — `tobi/steer` rather than
    /// `origin/tobi/steer`.
    ///
    /// The one thing here that is neither a label nor a link, and the reason it
    /// is written down at all: a green suite is held against what origin holds on
    /// *this* pull request's branch rather than on whatever the Worktree has
    /// checked out, and a session sent at a stack has to be told which branches
    /// it is working. Both are questions about a pull request that several in one
    /// Worktree make impossible to answer off the checkout.
    ///
    /// `None` on a row written before Verkstead wrote it down — which for the
    /// Conversation's own repository is that Conversation's own branch, the one
    /// thing it was possible for it to be, and for a companion's is nothing
    /// anybody can recover.
    pub head: Option<String>,

    /// And the branch it goes into, unqualified the same way.
    ///
    /// Written down for the one thing a head cannot say by itself: which pull
    /// request of a repository is *below* which. A **stack** is a chain of
    /// branches each based on the one under it, so a base that is another
    /// recorded pull request's head is the link between the two — and with both
    /// on the row the chain is a fact about the record rather than something to
    /// go back to GitHub for. See [`stack`], which is that reading.
    ///
    /// `None` on a row written before Verkstead wrote it down. Where a chain
    /// cannot be read off the rows, [`stack`] falls back on the order they were
    /// recorded in, which is what such a row was implicitly ordered by before.
    pub base: Option<String>,

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

/// Whether a pull request merges into its base, as the last look at GitHub found
/// it.
///
/// Two words rather than GitHub's three, and the missing one is the point: a
/// GitHub that has not worked the answer out yet — which is what it says for a
/// while after every push — is *not known*, and not knowing is the absence of a
/// row here rather than a word in it. The same spirit the rollup beside it is
/// written in, and the same one the watcher reads a `gh` that will not answer
/// in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Merging {
    /// GitHub says it merges.
    Cleanly,

    /// GitHub says it does not: the branch and its base have both changed the
    /// same lines since they parted, and nothing lands until somebody resolves
    /// it.
    Conflicting,
}

impl Merging {
    /// The word the column holds. Lowercase and spelled out, so a database
    /// opened by hand says something.
    fn stored(self) -> &'static str {
        match self {
            Self::Cleanly => "cleanly",
            Self::Conflicting => "conflicting",
        }
    }

    /// The one a stored word names. An unknown word is a database written by a
    /// Verkstead this one does not understand, exactly as an unknown lifecycle
    /// state is.
    fn read(word: &str) -> Result<Self> {
        Ok(match word {
            "cleanly" => Self::Cleanly,
            "conflicting" => Self::Conflicting,
            other => bail!("a pull request merges the unknown {other:?}"),
        })
    }
}

/// Where a pull request has got to, as the last look at GitHub found it.
///
/// Three words rather than the two [`Merging`] has, and no *not known* among
/// them for the same reason: nothing having asked is the absence of a row here.
/// What matters is the difference between the first and the other two — an open
/// pull request is one still worth asking about, and a merged or closed one is a
/// question that has been answered for good. See [`unfinished_pull_requests`],
/// where that is a sweep's whole end condition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Standing {
    /// It is open, and whether it merges is still a live question.
    Open,

    /// Somebody merged it, which is the ending this pipeline is built around and
    /// the one act Verkstead never makes itself.
    Merged,

    /// Somebody closed it without merging it.
    Closed,
}

impl Standing {
    /// The word the column holds. Lowercase and spelled out, so a database
    /// opened by hand says something.
    fn stored(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Merged => "merged",
            Self::Closed => "closed",
        }
    }

    /// The one a stored word names. An unknown word is a database written by a
    /// Verkstead this one does not understand, exactly as an unknown lifecycle
    /// state is.
    fn read(word: &str) -> Result<Self> {
        Ok(match word {
            "open" => Self::Open,
            "merged" => Self::Merged,
            "closed" => Self::Closed,
            other => bail!("a pull request stands the unknown {other:?}"),
        })
    }
}

/// One pull request a Done Conversation is still waiting to see land: which
/// Conversation, which repository to ask GitHub in, and which number to ask
/// about.
///
/// What the sweep after Done walks — see [`unfinished_pull_requests`]. The Repo
/// rather than its id alone for [`pull_requests`]'s reason: `gh` reads its
/// repository from wherever it is run, and `#7` names something else in another
/// one or nothing at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unfinished {
    /// The Done Conversation whose work the pull request carries.
    pub conversation_id: i64,

    /// The Repo it was opened in, which is where `gh` is run to ask about it.
    pub repo: Repo,

    /// And the number GitHub gave it, which means that in that repository alone.
    pub number: i64,
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
/// commits table's reason: *one Conversation has one row per pull request* is the
/// rule, and SQLite cannot index a column that lives in another table.
///
/// The Repo and the number are the rest of that index — together they are what a
/// pull request *is* to Verkstead, `#41` naming something else in the next
/// repository along or nothing at all. A database written before a Conversation
/// could hold several in one repository has the number in the table and not in
/// the rule, and one written before a Conversation could end on more than one
/// pull request has neither the Repo nor the column. Both are
/// [`super::migrations`]'s to put right as the database opens rather than this
/// function's: the constraint is declared inline, so it is the table itself that
/// has to be rebuilt, and that is not something a `CREATE TABLE IF NOT EXISTS`
/// can reach.
///
/// `head_branch` and `base_branch` are nullable for the same migration's sake:
/// every row written before Verkstead wrote the two branches down has nothing to
/// put there. See [`PullRequest::head`] and [`PullRequest::base`], and
/// [`super::migrations`] for the column arriving on a database that has rows
/// already.
pub(crate) async fn apply_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS pull_requests (
             event_id        INTEGER PRIMARY KEY REFERENCES timeline_events(id),
             conversation_id INTEGER NOT NULL REFERENCES conversations(id),
             repo_id         INTEGER NOT NULL REFERENCES repos(id),
             number          INTEGER NOT NULL,
             title           TEXT NOT NULL,
             url             TEXT NOT NULL,
             head_branch     TEXT,
             base_branch     TEXT,
             UNIQUE (conversation_id, repo_id, number)
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
    // One row or none per pull request, a suite being a fact about one branch:
    // two watchers writing one row would each be reading the other's suite, and
    // a Conversation with a read-write companion or a stack has as many suites
    // as it has pull requests. A database written while this was keyed by the
    // Conversation alone is [`super::migrations`]'s to rekey.
    //
    // And it survives a restart, which is the whole reason it is written down
    // rather than held in the watcher: the watcher stops when the wrap-up is
    // over, and a Done Conversation would otherwise lose its icon the next time
    // the server came up.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS pull_request_checks (
             conversation_id INTEGER NOT NULL REFERENCES conversations(id),
             repo_id         INTEGER NOT NULL REFERENCES repos(id),
             number          INTEGER NOT NULL,
             rollup          TEXT NOT NULL,
             at              TEXT NOT NULL,
             PRIMARY KEY (conversation_id, repo_id, number)
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the pull request checks table")?;

    // And whether GitHub can merge it, which is the other thing about a pull
    // request that is written down here and moves — asked of the same `gh` call
    // the rollup above comes from.
    //
    // A table of its own rather than a column beside the rollup, because the two
    // are not read at the same moments: a sweep after Done asks whether a branch
    // still merges and never asks how its checks were. Keyed the same way, one
    // row per pull request — a Conversation with a read-write companion has one
    // clean and one conflicted as easily as two of either, and so has a stack
    // whose base moved under its bottom branch alone.
    //
    // Written down for the rollup's reason as well: the watching stops when the
    // wrap-up is over, and what a card draws about a Done Conversation is the
    // last thing anybody asked GitHub.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS pull_request_merges (
             conversation_id INTEGER NOT NULL REFERENCES conversations(id),
             repo_id         INTEGER NOT NULL REFERENCES repos(id),
             number          INTEGER NOT NULL,
             merging         TEXT NOT NULL,
             at              TEXT NOT NULL,
             PRIMARY KEY (conversation_id, repo_id, number)
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the pull request merges table")?;

    // And where it has got to — open, merged or closed — which is the fact that
    // ends the asking. A pull request nobody has merged or closed is one whose
    // conflicts are still worth watching for long after the Conversation is
    // Done; one that has been is a question with a final answer, and nothing
    // asks GitHub about it again. See [`unfinished_pull_requests`].
    //
    // Beside the merges above rather than a column in them, because a pull
    // request that has been merged still merged cleanly: the two are separate
    // readings of one `gh` answer, and neither is the other's qualifier. A row
    // in one and no row in the other is the ordinary shape of it — the wrap-up's
    // own watcher writes the merge down and never asks where the pull request
    // has got to, that being no business of a Conversation still wrapping up.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS pull_request_standings (
             conversation_id INTEGER NOT NULL REFERENCES conversations(id),
             repo_id         INTEGER NOT NULL REFERENCES repos(id),
             number          INTEGER NOT NULL,
             standing        TEXT NOT NULL,
             at              TEXT NOT NULL,
             PRIMARY KEY (conversation_id, repo_id, number)
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the pull request standings table")?;

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
/// And a third that is not an ending at all: a Draft holding a pull request
/// somebody opened elsewhere, which the take-up has just put on that pull
/// request's branch — see [`super::take_up`]. There the work was built before
/// Verkstead saw it, so the wrap-up is the whole of what there is to do, and
/// this record is what starts it. Which Drafts those are is a row rather than a
/// state, so it is asked as one — see [`taking_one_up`].
///
/// And a fourth, which is a **Tinker** finishing: a Conversation in Follow-up
/// whose branch holds commits and no pull request is one whose ending sent a
/// `submitting` session to open one, and this is the pull request it opened. The
/// door is here rather than a move written beside the ending, so a Tinker carries
/// into Wrapping through the entry every other ending uses — with the PR Event,
/// the row and the move in one transaction, and the wrap-up's watchers started
/// over what was opened. See `crate::runner`, which is where the branch is asked.
/// A follow-up steered into never reaches this: it is on a pull request already,
/// and its ending is [`super::follow_up_over`].
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
/// Which is why neither a companion's pull request nor the rest of a stack comes
/// through here. A Conversation that is already Wrapping has nothing left to
/// wrap, so a second PR arriving a moment behind the work's own would be refused
/// — see [`record_another_pull_request`], which is this same row without the move
/// over the top of it.
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
    let mut tx = super::writing(pool, "recording a pull request").await?;

    let row: Option<(String,)> = sqlx::query_as("SELECT state FROM conversations WHERE id = ?")
        .bind(conversation_id)
        .fetch_optional(&mut *tx)
        .await
        .with_context(|| format!("reading the state of Conversation {conversation_id}"))?;

    let Some((state,)) = row else {
        return Ok(Wrapping::NoSuchConversation);
    };

    // Four states reach here, and two of them are doors rather than endings: a
    // Draft holding a pull request is one the human is taking up, its worktree
    // is already on that pull request's head branch, and this record is the same
    // move the finish step makes. Asked of the record rather than of the state
    // alone, so that nothing else can carry a Draft into Wrapping. A Follow-up
    // is the Tinker whose ending sent for the pull request its branch was on
    // none of — nothing else opens one over a Conversation in that state.
    let wrappable = match Lifecycle::read(&state)? {
        Lifecycle::Implementing | Lifecycle::Grilling | Lifecycle::FollowUp => true,
        Lifecycle::Draft => taking_one_up(&mut tx, conversation_id).await?,
        _ => false,
    };

    if !wrappable {
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
/// another pull request rather than a second ending. Which is the whole
/// difference between the two — the pull request a Conversation was pointed at is
/// what moves it, and the companions' and the rest of its stack are what the move
/// then covers.
///
/// `false` where there is no Conversation with that id. Nothing else is refused
/// for: a pull request already on the record reuses the row it has, which is what
/// makes a discovery run twice do nothing the second time.
pub async fn record_another_pull_request(
    pool: &SqlitePool,
    conversation_id: i64,
    repo_id: i64,
    pull_request: &PullRequest,
) -> Result<bool> {
    let mut tx = super::writing(pool, "recording another pull request").await?;

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

/// Whether this Draft is one somebody is taking a pull request up on.
///
/// The one thing that lets a Draft move into Wrapping, and it is a row rather
/// than a guess: a **Review** whose press has resolved its Target has the pull
/// request written beside it — see [`super::hold_pull_request`] — and every other
/// Draft in the database has nothing there and nothing to wrap.
///
/// Read inside the caller's transaction, so that the answer still holds when the
/// move acts on it.
async fn taking_one_up(
    tx: &mut Transaction<'static, Sqlite>,
    conversation_id: i64,
) -> Result<bool> {
    let held: Option<(i64,)> =
        sqlx::query_as("SELECT number FROM pull_request_adoptions WHERE conversation_id = ?")
            .bind(conversation_id)
            .fetch_optional(&mut **tx)
            .await
            .with_context(|| {
                format!("reading which pull request Conversation {conversation_id} is holding")
            })?;

    Ok(held.is_some())
}

/// The Event and the row under it, or nothing at all where this Conversation is
/// on that pull request already.
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
        "SELECT event_id FROM pull_requests
         WHERE conversation_id = ? AND repo_id = ? AND number = ?",
    )
    .bind(conversation_id)
    .bind(repo_id)
    .bind(pull_request.number)
    .fetch_optional(&mut **tx)
    .await
    .with_context(|| {
        format!(
            "looking for pull request {} of Repo {repo_id} on Conversation {conversation_id}",
            pull_request.number
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
        "INSERT INTO pull_requests
             (event_id, conversation_id, repo_id, number, title, url, head_branch, base_branch)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(event_id)
    .bind(conversation_id)
    .bind(repo_id)
    .bind(pull_request.number)
    .bind(&pull_request.title)
    .bind(&pull_request.url)
    .bind(pull_request.head.as_deref())
    .bind(pull_request.base.as_deref())
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

/// The first pull request a Conversation's work was recorded on in one Repo, or
/// `None` where that repository has none yet.
///
/// Per Repo and not per Conversation, because a number is a fact about a
/// repository: a Conversation working alongside read-write companions ends on
/// one pull request each, and `#41` in one of them is a different pull request
/// from `#41` in another.
///
/// **The first**, in the order they were recorded, which is the one the
/// Conversation was pointed at: a repository holds as many as a stack is deep,
/// and what is asked for here is the pull request a wrap-up is *defined* by
/// rather than the whole of what it covers. Which is what the steer into Wrapping
/// asks about the Conversation's own repository, and what says whether a
/// repository has a pull request at all. [`pull_requests`] is the list.
pub async fn pull_request(
    pool: &SqlitePool,
    conversation_id: i64,
    repo_id: i64,
) -> Result<Option<PullRequest>> {
    /// The columns in the order the query below selects them: the pull request,
    /// the branches its work is on and goes into, and the Repo's name where it is
    /// not the Conversation's own.
    type Row = (
        i64,
        String,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
    );

    let row: Option<Row> = sqlx::query_as(
        "SELECT p.number, p.title, p.url, p.head_branch, p.base_branch, r.name
         FROM pull_requests p
         JOIN conversations v ON v.id = p.conversation_id
         LEFT JOIN repos r ON r.id = p.repo_id AND r.id <> v.repo_id
         WHERE p.conversation_id = ? AND p.repo_id = ?
         ORDER BY p.event_id
         LIMIT 1",
    )
    .bind(conversation_id)
    .bind(repo_id)
    .fetch_optional(pool)
    .await
    .with_context(|| {
        format!("reading the pull request of Repo {repo_id} on Conversation {conversation_id}")
    })?;

    Ok(
        row.map(|(number, title, url, head, base, repo)| PullRequest {
            number,
            title,
            url,
            head,
            base,
            repo,
        }),
    )
}

/// One pull request by name: `number` of Repo `repo_id`, or `None` where the
/// Conversation is not on it.
///
/// What a watcher asks every poll. One is started for each pull request on the
/// record and told which — the Repo and the number together, that being what a
/// pull request is — so what it needs back is *is this one still on the record*
/// and the branch its work is on. [`pull_request`] beside it is the other
/// question, *what has this repository got*, which a stack gives several answers
/// to; this one is asked about one of them.
///
/// `None` is a record that has been got at rather than a wrap-up to carry on
/// with: a watcher is started where a pull request is recorded and never before
/// it, and nothing takes one off the record while a Conversation is wrapping up.
pub async fn pull_request_numbered(
    pool: &SqlitePool,
    conversation_id: i64,
    repo_id: i64,
    number: i64,
) -> Result<Option<PullRequest>> {
    /// The columns in the order the query below selects them: the pull request,
    /// the branches its work is on and goes into, and the Repo's name where it is
    /// not the Conversation's own.
    type Row = (
        i64,
        String,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
    );

    let row: Option<Row> = sqlx::query_as(
        "SELECT p.number, p.title, p.url, p.head_branch, p.base_branch, r.name
         FROM pull_requests p
         JOIN conversations v ON v.id = p.conversation_id
         LEFT JOIN repos r ON r.id = p.repo_id AND r.id <> v.repo_id
         WHERE p.conversation_id = ? AND p.repo_id = ? AND p.number = ?",
    )
    .bind(conversation_id)
    .bind(repo_id)
    .bind(number)
    .fetch_optional(pool)
    .await
    .with_context(|| {
        format!(
            "reading pull request #{number} of Repo {repo_id} on Conversation {conversation_id}"
        )
    })?;

    Ok(
        row.map(|(number, title, url, head, base, repo)| PullRequest {
            number,
            title,
            url,
            head,
            base,
            repo,
        }),
    )
}

/// Every pull request a Conversation's work is on, each with the Repo it was
/// opened in.
///
/// What a wrap-up's watchers are started from: there is a suite per pull request
/// and a Conversation ends on one per repository it was worked in and as many in
/// one repository as its stack is deep, so *which pull requests* is a question
/// with a list for an answer. The Repo comes with each of them because that is
/// where `gh` has to be run to ask about it — a number means something else in
/// another repository, or nothing.
///
/// In the order they were recorded, which is the one the Conversation was pointed
/// at first and the rest — the companions', the stack's — as they were found.
///
/// A pull request whose Repo is no longer registered is left out rather than
/// carried without one: there is nowhere left to ask about it.
pub async fn pull_requests(
    pool: &SqlitePool,
    conversation_id: i64,
) -> Result<Vec<(Repo, PullRequest)>> {
    /// The columns in the order the query below selects them: the Repo, whether
    /// it is one beside the Conversation's own, and the pull request.
    type Row = (
        i64,
        String,
        String,
        String,
        i64,
        i64,
        String,
        String,
        Option<String>,
        Option<String>,
    );

    let rows: Vec<Row> = sqlx::query_as(
        "SELECT r.id, r.path, r.name, r.default_branch, r.id <> v.repo_id,
                p.number, p.title, p.url, p.head_branch, p.base_branch
         FROM pull_requests p
         JOIN conversations v ON v.id = p.conversation_id
         JOIN repos r ON r.id = p.repo_id
         WHERE p.conversation_id = ?
         ORDER BY p.event_id",
    )
    .bind(conversation_id)
    .fetch_all(pool)
    .await
    .with_context(|| format!("reading the pull requests of Conversation {conversation_id}"))?;

    Ok(rows
        .into_iter()
        .map(
            |(id, path, name, default_branch, beside, number, title, url, head, base)| {
                let repo = Repo {
                    id,
                    path: std::path::PathBuf::from(path),
                    name,
                    default_branch,
                };

                // The label the pinned card draws, which is the Repo's name only
                // where it is not the Conversation's own — see [`PullRequest::repo`].
                let named = (beside != 0).then(|| repo.name.clone());

                (
                    repo,
                    PullRequest {
                        number,
                        title,
                        url,
                        head,
                        base,
                        repo: named,
                    },
                )
            },
        )
        .collect())
}

/// The pull requests a Conversation has recorded in one Repo, in stack order:
/// the bottom of the chain first, and each one based on the one before it.
///
/// What a wrap-up over a **stack** is read by. A stack is a chain of branches
/// each based on the one under it, and that chain is on the rows themselves —
/// each pull request's head and the branch it goes into — so this is a reading
/// of what is recorded rather than another question for GitHub. Which matters
/// because the reading outlives the walk: the chain was found at Start and the
/// session that syncs it is dispatched whenever a conflict turns up, hours and a
/// restart later.
///
/// One repository at a time, because a stack is one repository's: a companion's
/// pull request is based on a branch of its own repository and has nothing to do
/// with this chain.
///
/// **A set that is not one chain comes back in the order it was recorded**, which
/// is the order [`pull_requests`] gives and the one every reader had before there
/// were stacks: a row from before Verkstead wrote the branches down, a repository
/// holding two unrelated pull requests, a chain that forks in two. None of those
/// has a bottom to start from, and inventing one would be an order nobody could
/// check. A lone pull request is a chain of one and comes back as itself either
/// way.
pub async fn stack(
    pool: &SqlitePool,
    conversation_id: i64,
    repo_id: i64,
) -> Result<Vec<PullRequest>> {
    let recorded: Vec<PullRequest> = pull_requests(pool, conversation_id)
        .await?
        .into_iter()
        .filter(|(repo, _)| repo.id == repo_id)
        .map(|(_, opened)| opened)
        .collect();

    let Some(order) = chained(&recorded) else {
        return Ok(recorded);
    };

    Ok(order.into_iter().map(|at| recorded[at].clone()).collect())
}

/// Which of `recorded` sits on which, as the branches on the rows say — or
/// `None` where they do not say one chain.
///
/// The whole of [`stack`]'s ordering, and a function of its own because it is
/// the part worth reading twice: a chain is *one* row nothing is based on, and
/// then one row based on each head in turn until there are no more. Anything
/// else — a head missing, two rows at the bottom, two rows on one head, a row
/// the walk never reaches — is not a chain, and is refused rather than
/// half-ordered.
fn chained(recorded: &[PullRequest]) -> Option<Vec<usize>> {
    // Every head, and every one of them distinct: two pull requests open on one
    // branch is not something GitHub allows, and a row that never recorded its
    // head is one nothing can be said to sit on.
    let heads: HashMap<&str, usize> = recorded
        .iter()
        .enumerate()
        .filter_map(|(at, opened)| Some((opened.head.as_deref()?, at)))
        .collect();

    if heads.len() != recorded.len() {
        return None;
    }

    // The bottom: the one row whose base is nobody else's head, which is the
    // branch the whole stack eventually merges into.
    let mut bottoms = recorded.iter().enumerate().filter(|(_, opened)| {
        !opened
            .base
            .as_deref()
            .is_some_and(|base| heads.contains_key(base))
    });

    let (bottom, _) = bottoms.next()?;

    if bottoms.next().is_some() {
        return None;
    }

    let mut order = vec![bottom];

    // And up from it, one link at a time: the row based on the last one's head.
    loop {
        let head = recorded[*order.last()?].head.as_deref()?;

        let mut above = recorded
            .iter()
            .enumerate()
            .filter(|(_, opened)| opened.base.as_deref() == Some(head))
            .map(|(at, _)| at);

        let Some(next) = above.next() else {
            break;
        };

        if above.next().is_some() {
            return None;
        }

        order.push(next);
    }

    // And every row reached. A row the chain walked past is a second stack in
    // the one repository, which is no more one chain than a fork is.
    (order.len() == recorded.len()).then_some(order)
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
/// tuple can be read back as. This one is cheap regardless — a Conversation has a
/// handful of pull requests at the very most, and usually none at all.
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
    /// The columns in the order the query below selects them: the Event, the pull
    /// request, the branches its work is on and goes into, and the Repo's name
    /// where it is not the Conversation's own.
    type Row = (
        i64,
        i64,
        String,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
    );

    let rows: Vec<Row> = sqlx::query_as(
        "SELECT p.event_id, p.number, p.title, p.url, p.head_branch, p.base_branch, r.name
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
        .map(|(event_id, number, title, url, head, base, repo)| {
            (
                event_id,
                PullRequest {
                    number,
                    title,
                    url,
                    head,
                    base,
                    repo,
                },
            )
        })
        .collect())
}

/// Which Conversation has this Repo's pull request on its record, where one has —
/// the open one, where one of them is open.
///
/// What a press asks: a take-up names a Repo and a number, and there is one
/// *open* Conversation per pull request — so a number a Conversation still at
/// work has is refused leading there, and a finished one's is taken up over the
/// top of it. Which of those this answer is, is the caller's to decide: what is
/// asked here is who has it.
///
/// By the Repo and the number together, because that pair is what a pull
/// request *is* to Verkstead: `#41` names something else in the next repository
/// along, or nothing at all.
///
/// **Every Conversation**, whatever state it is in — Done and Closed included. A
/// pull request stays on the record it was written to, so a Done holder is a
/// Conversation to close rather than a row to overlook, and a Closed one is what
/// says this pull request has been through here before. Which is also why an
/// Archived one counts: archiving is a Closed Conversation off the sidebar rather
/// than a state of its own.
///
/// **And the open one where there is one**, which is why this is ordered at all.
/// A pull request is on several records from the moment a take-up may close its
/// holder and record it again: the Conversation that had it, and the one that
/// took it over. Whoever asks who has it — the take-up itself, and the stack note
/// that names a neighbour's holder — means the one that is still at work, and the
/// newest where none of them is, that being the one the others are the history
/// of. Unordered, SQLite answers with whichever row it reaches first, which is
/// ordinarily the oldest: the Conversation that was closed to make way, offered
/// as the way on.
///
/// **The pull request a Conversation's work is *on*, rather than every row
/// recorded beside it.** A wrap-up over a stack records the whole chain so that
/// it can watch it — see [`stack`] — and those neighbours usually belong to a
/// Conversation each, that being what a stack in this workbench is. Read
/// straight off the rows, a pull request in a stack of three would be three
/// Conversations' at once and the last two would be refused leading to the first.
/// So what is asked is the *first* row recorded in each repository, which is what
/// [`pull_request`] means by the pull request a wrap-up is defined by: the one it
/// was pointed at, or the one its finish step opened there. Everything after it
/// in that repository was recorded because it is watched.
pub async fn conversation_on_pull_request(
    pool: &SqlitePool,
    repo_id: i64,
    number: i64,
) -> Result<Option<i64>> {
    on_pull_request(pool, repo_id, number, None).await
}

/// The same question with one Conversation left out of the answer: who *else*
/// has this pull request on their record.
///
/// What a Steer asks — see ADR-0020, and the paragraph in it about the way back.
/// A take-up is pressed by a Draft, and a Draft has no pull request on its
/// record, so the question there can only ever be about somebody else; a steer is
/// made by the Conversation whose pull request it is, so the one row that is
/// never the answer is its own.
///
/// **And the leaving out is the query's rather than the caller's**, because the
/// ordering above is what decides between two rows. A Closed Conversation steered
/// back onto its pull request is often the newest row on it — it took the pull
/// request over and was closed afterwards — so an answer that came back as itself
/// and was then thrown away would be an answer that never named the Done holder
/// standing on the branch.
pub async fn other_conversation_on_pull_request(
    pool: &SqlitePool,
    repo_id: i64,
    number: i64,
    besides: i64,
) -> Result<Option<i64>> {
    on_pull_request(pool, repo_id, number, Some(besides)).await
}

/// The one query both of those are, `besides` being the Conversation to leave out
/// of it where there is one to leave out.
async fn on_pull_request(
    pool: &SqlitePool,
    repo_id: i64,
    number: i64,
    besides: Option<i64>,
) -> Result<Option<i64>> {
    let row: Option<(i64,)> = sqlx::query_as(
        "SELECT p.conversation_id
         FROM pull_requests p
         JOIN conversations v ON v.id = p.conversation_id
         WHERE p.repo_id = ? AND p.number = ?
           AND (? IS NULL OR p.conversation_id <> ?)
           AND p.event_id = (SELECT MIN(q.event_id) FROM pull_requests q
                             WHERE q.conversation_id = p.conversation_id
                               AND q.repo_id = p.repo_id)
         ORDER BY v.state NOT IN (?, ?) DESC, v.id DESC
         LIMIT 1",
    )
    .bind(repo_id)
    .bind(number)
    .bind(besides)
    .bind(besides)
    .bind(Lifecycle::Done.stored())
    .bind(Lifecycle::Closed.stored())
    .fetch_optional(pool)
    .await
    .with_context(|| {
        format!("reading which Conversation is already on pull request {number} of Repo {repo_id}")
    })?;

    Ok(row.map(|(conversation_id,)| conversation_id))
}

/// Write down how the checks on the pull request `number` names in `repo_id` are,
/// and say whether that is news.
///
/// Per pull request, a suite being a fact about one branch: a Conversation with a
/// read-write companion has two, and one wrapping up a stack has one per pull
/// request in the chain. One row between them would be each watcher reading the
/// other's suite and the card drawing whichever wrote last.
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
    repo_id: i64,
    number: i64,
    rollup: Rollup,
) -> Result<bool> {
    let mut tx = super::writing(pool, "recording how a pull request's checks are").await?;

    let row: Option<(String,)> = sqlx::query_as(
        "SELECT rollup FROM pull_request_checks
         WHERE conversation_id = ? AND repo_id = ? AND number = ?",
    )
    .bind(conversation_id)
    .bind(repo_id)
    .bind(number)
    .fetch_optional(&mut *tx)
    .await
    .with_context(|| {
        format!(
            "reading how the checks on pull request {number} of Repo {repo_id} on \
             Conversation {conversation_id} were"
        )
    })?;

    let before = row.map(|(word,)| Rollup::read(&word)).transpose()?;

    sqlx::query(
        "INSERT INTO pull_request_checks (conversation_id, repo_id, number, rollup, at)
         VALUES (?, ?, ?, ?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
         ON CONFLICT (conversation_id, repo_id, number)
         DO UPDATE SET rollup = excluded.rollup, at = excluded.at",
    )
    .bind(conversation_id)
    .bind(repo_id)
    .bind(number)
    .bind(rollup.stored())
    .execute(&mut *tx)
    .await
    .with_context(|| {
        format!(
            "recording how the checks on pull request {number} of Repo {repo_id} on \
             Conversation {conversation_id} are"
        )
    })?;

    tx.commit()
        .await
        .context("recording how a pull request's checks are")?;

    Ok(before != Some(rollup))
}

/// And how they were the last time anything asked, or `None` where nothing has.
///
/// It may be stale, and on a Conversation nothing is watching any more it will
/// be: the watching stops when the wrap-up is over, and what is read after that
/// is the last thing anybody asked GitHub — which is a card an hour behind rather
/// than a card that is wrong.
pub async fn check_rollup(
    pool: &SqlitePool,
    conversation_id: i64,
    repo_id: i64,
    number: i64,
) -> Result<Option<Rollup>> {
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT rollup FROM pull_request_checks
         WHERE conversation_id = ? AND repo_id = ? AND number = ?",
    )
    .bind(conversation_id)
    .bind(repo_id)
    .bind(number)
    .fetch_optional(pool)
    .await
    .with_context(|| {
        format!(
            "reading how the checks on pull request {number} of Repo {repo_id} on \
             Conversation {conversation_id} are"
        )
    })?;

    row.map(|(word,)| Rollup::read(&word)).transpose()
}

/// And every one of a Conversation's rollups together, by the Timeline Event each
/// pull request is.
///
/// What the Conversation view draws the check icon off, and the mirror of
/// [`merges`] below in every respect — including why it is keyed by the Event: a
/// pull request's card is drawn twice from one Event, pinned above the record and
/// at the moment it opened, and an Event id is what both copies have to hand.
///
/// Every pull request rather than the Conversation's own. A rollup used to be the
/// Conversation's alone, so the one it belonged to was the one that moved the
/// Conversation into Wrapping and a companion's card drew no icon at all; each
/// now has its own, and draws it.
///
/// A pull request nothing has asked GitHub about has no entry, which is the card
/// that draws no icon — the same honesty a suite with no checks in it is written
/// down with, which is not at all.
pub async fn rollups(pool: &SqlitePool, conversation_id: i64) -> Result<HashMap<i64, Rollup>> {
    let rows: Vec<(i64, String)> = sqlx::query_as(
        "SELECT p.event_id, c.rollup
         FROM pull_requests p
         JOIN pull_request_checks c
           ON c.conversation_id = p.conversation_id
          AND c.repo_id = p.repo_id
          AND c.number = p.number
         WHERE p.conversation_id = ?",
    )
    .bind(conversation_id)
    .fetch_all(pool)
    .await
    .with_context(|| {
        format!("reading how Conversation {conversation_id}'s pull requests' checks are")
    })?;

    rows.into_iter()
        .map(|(event_id, word)| Ok((event_id, Rollup::read(&word)?)))
        .collect()
}

/// Write down whether the pull request `number` names in `repo_id` merges into
/// its base.
///
/// Per pull request, as the rollup above is: whether a branch conflicts with its
/// base is a fact about the branch. One conflicting while another merges is the
/// ordinary shape of it — a base having moved in one repository and not in the
/// other, or under the bottom of a stack and nowhere above it.
///
/// Written over rather than appended to: this is how the pull request merges
/// now, and a conflict that has been resolved is not a conflict.
///
/// Only ever the two definite readings. A GitHub that has not worked the answer
/// out is not written down at all, so what stands is the last thing it did say —
/// see [`Merging`], and [`crate::checks`] in the server, where not knowing is
/// what changes nothing.
///
/// Says whether that was news, exactly as [`record_check_rollup`] does and for
/// the same reason: this is asked on every poll of a wrap-up's watcher and on
/// every sweep after Done, and a pull request that merged cleanly a minute ago
/// merges cleanly now. The caller Nudges the open pages on the strength of it,
/// so a word that did not move is a page not re-reading a Timeline nothing
/// happened on.
pub async fn record_merging(
    pool: &SqlitePool,
    conversation_id: i64,
    repo_id: i64,
    number: i64,
    merging: Merging,
) -> Result<bool> {
    let mut tx = super::writing(pool, "recording whether a pull request merges").await?;

    let row: Option<(String,)> = sqlx::query_as(
        "SELECT merging FROM pull_request_merges
         WHERE conversation_id = ? AND repo_id = ? AND number = ?",
    )
    .bind(conversation_id)
    .bind(repo_id)
    .bind(number)
    .fetch_optional(&mut *tx)
    .await
    .with_context(|| {
        format!(
            "reading whether pull request {number} of Repo {repo_id} on \
             Conversation {conversation_id} merged"
        )
    })?;

    let before = row.map(|(word,)| Merging::read(&word)).transpose()?;

    sqlx::query(
        "INSERT INTO pull_request_merges (conversation_id, repo_id, number, merging, at)
         VALUES (?, ?, ?, ?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
         ON CONFLICT (conversation_id, repo_id, number)
         DO UPDATE SET merging = excluded.merging, at = excluded.at",
    )
    .bind(conversation_id)
    .bind(repo_id)
    .bind(number)
    .bind(merging.stored())
    .execute(&mut *tx)
    .await
    .with_context(|| {
        format!(
            "recording whether pull request {number} of Repo {repo_id} on \
             Conversation {conversation_id} merges"
        )
    })?;

    tx.commit()
        .await
        .context("recording whether a pull request merges")?;

    Ok(before != Some(merging))
}

/// And how it merged the last time anything asked, or `None` where nothing has.
///
/// It may be stale, and on a Conversation nothing is watching any more it will
/// be — the rollup's trade exactly: the watching stops when the wrap-up is over,
/// and what is read after that is the last thing anybody asked GitHub.
pub async fn merging(
    pool: &SqlitePool,
    conversation_id: i64,
    repo_id: i64,
    number: i64,
) -> Result<Option<Merging>> {
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT merging FROM pull_request_merges
         WHERE conversation_id = ? AND repo_id = ? AND number = ?",
    )
    .bind(conversation_id)
    .bind(repo_id)
    .bind(number)
    .fetch_optional(pool)
    .await
    .with_context(|| {
        format!(
            "reading whether pull request {number} of Repo {repo_id} on \
             Conversation {conversation_id} merges"
        )
    })?;

    row.map(|(word,)| Merging::read(&word)).transpose()
}

/// Which of a Conversation's pull requests the last look at GitHub found
/// conflicting with their base, as the Repo and the number of each.
///
/// What the resolve press reads to know there is anything to resolve, and which
/// of the wrap-up's settlements it puts back to waiting — see
/// [`super::resolve_conflicts`], whose transaction this runs in.
///
/// The record rather than GitHub, deliberately. The press is offered off the
/// same written-down fact the card draws its mark from and the details pane says
/// in words, so it is answered off that fact too: a button drawn from one
/// reading and refused by another would be a press nobody could predict. What
/// keeps the fact fresh is the sweep after Done and the pane's own asking as it
/// opens — see [`crate::merges`] in the server.
///
/// A pull request nothing has ever asked GitHub about has no row at all and is
/// not among these, exactly as it draws no mark: not knowing is not a conflict.
///
/// One entry per conflicting pull request, because what the caller does with each
/// is put a settlement back to waiting and that settlement is the pull request's.
/// A stack with three branches conflicting names its repository three times, once
/// per number, and the wrap-up waits on each of those merges — which is three
/// settlements to unsettle.
pub(crate) async fn conflicted(
    tx: &mut sqlx::SqliteConnection,
    conversation_id: i64,
) -> Result<Vec<(i64, i64)>> {
    let rows: Vec<(i64, i64)> = sqlx::query_as(
        "SELECT p.repo_id, p.number
         FROM pull_requests p
         JOIN pull_request_merges m
           ON m.conversation_id = p.conversation_id
          AND m.repo_id = p.repo_id
          AND m.number = p.number
         WHERE p.conversation_id = ? AND m.merging = ?
         ORDER BY p.event_id",
    )
    .bind(conversation_id)
    .bind(Merging::Conflicting.stored())
    .fetch_all(&mut *tx)
    .await
    .with_context(|| {
        format!("reading which of Conversation {conversation_id}'s pull requests conflict")
    })?;

    Ok(rows)
}

/// And every one of them a Conversation has, by the Timeline Event each pull
/// request is.
///
/// What the Conversation view draws the conflict mark off: the card is drawn
/// twice from one Event — pinned above the record and at the moment it opened —
/// and an Event id is what both copies have to hand. Which is why this is keyed
/// by the Event rather than by the Repo, unlike [`merging`] above, whose caller
/// is a watcher that already knows which repository it is asking about.
///
/// Every pull request's, as [`rollups`] beside it is: whether a branch merges is
/// written down per pull request, so each card draws its own reading.
///
/// A pull request nothing has asked GitHub about has no entry at all, which is
/// the card that draws no mark. Stale on a Conversation nothing is watching or
/// sweeping any more, in the spirit everything read back here is.
pub async fn merges(pool: &SqlitePool, conversation_id: i64) -> Result<HashMap<i64, Merging>> {
    let rows: Vec<(i64, String)> = sqlx::query_as(
        "SELECT p.event_id, m.merging
         FROM pull_requests p
         JOIN pull_request_merges m
           ON m.conversation_id = p.conversation_id
          AND m.repo_id = p.repo_id
          AND m.number = p.number
         WHERE p.conversation_id = ?",
    )
    .bind(conversation_id)
    .fetch_all(pool)
    .await
    .with_context(|| {
        format!("reading whether Conversation {conversation_id}'s pull requests merge")
    })?;

    rows.into_iter()
        .map(|(event_id, word)| Ok((event_id, Merging::read(&word)?)))
        .collect()
}

/// Write down where the pull request `number` names in `repo_id` has got to.
///
/// Per pull request for [`record_merging`]'s reason and beside it in the same
/// spirit: this is a reading of GitHub as it stood the last time anything asked,
/// written over rather than appended to.
///
/// The one reading that is final. A pull request recorded merged or closed is
/// one nothing asks about again — see [`unfinished_pull_requests`] — so what is
/// written here is what ends a sweep, and *open* is the only word that leaves it
/// running.
pub async fn record_standing(
    pool: &SqlitePool,
    conversation_id: i64,
    repo_id: i64,
    number: i64,
    standing: Standing,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO pull_request_standings (conversation_id, repo_id, number, standing, at)
         VALUES (?, ?, ?, ?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
         ON CONFLICT (conversation_id, repo_id, number)
         DO UPDATE SET standing = excluded.standing, at = excluded.at",
    )
    .bind(conversation_id)
    .bind(repo_id)
    .bind(number)
    .bind(standing.stored())
    .execute(pool)
    .await
    .with_context(|| {
        format!(
            "recording where pull request {number} of Repo {repo_id} on \
             Conversation {conversation_id} has got to"
        )
    })?;

    Ok(())
}

/// And where it stood the last time anything asked, or `None` where nothing has.
///
/// Stale on a pull request nothing is sweeping any more, exactly as the merge
/// beside it is — and on a merged or closed one it is stale for good, that being
/// the reading nothing asks about twice.
pub async fn standing(
    pool: &SqlitePool,
    conversation_id: i64,
    repo_id: i64,
    number: i64,
) -> Result<Option<Standing>> {
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT standing FROM pull_request_standings
         WHERE conversation_id = ? AND repo_id = ? AND number = ?",
    )
    .bind(conversation_id)
    .bind(repo_id)
    .bind(number)
    .fetch_optional(pool)
    .await
    .with_context(|| {
        format!(
            "reading where pull request {number} of Repo {repo_id} on \
             Conversation {conversation_id} has got to"
        )
    })?;

    row.map(|(word,)| Standing::read(&word)).transpose()
}

/// Every pull request there is still something to ask GitHub about after the
/// work on it is over: a Done Conversation's, that nothing has recorded merged
/// or closed.
///
/// What the sweep after Done walks, and the whole of what decides which pull
/// requests it asks about — see [`crate::checks`] in the server for the watcher
/// that covers a wrap-up, and the sweep for what covers a pull request once the
/// wrap-up is finished with.
///
/// **Done alone.** A Conversation still Wrapping has a watcher of its own asking
/// this every half minute, and one that is Closed is the human finished with the
/// work — which takes an Archived one with it, archiving being a Closed
/// Conversation off the sidebar rather than a state of its own. Everything below
/// Wrapping has no pull request to ask about in the first place.
///
/// **And open alone.** A merged or closed pull request is an answer that will
/// not change, so it drops out of the list the moment one is recorded and never
/// comes back — which is what ends the asking per pull request rather than all
/// at once. A pull request nothing has asked about yet has no row at all, and
/// that is as much a reason to ask as an open one.
///
/// **Per pull request rather than per repository.** Each recorded pull request is
/// asked about on its own number, so a stack of three in one repository is three
/// questions and three rows — the standing that ends the asking belonging to one
/// pull request and saying nothing about the ones beside it.
///
/// In Conversation order and then the order the pull requests were recorded,
/// which is each Conversation's own first and the rest as they were found.
pub async fn unfinished_pull_requests(pool: &SqlitePool) -> Result<Vec<Unfinished>> {
    /// The columns in the order the query below selects them: the Conversation,
    /// the Repo to ask in, and the number to ask about.
    type Row = (i64, i64, String, String, String, i64);

    let rows: Vec<Row> = sqlx::query_as(
        "SELECT p.conversation_id, r.id, r.path, r.name, r.default_branch, p.number
         FROM pull_requests p
         JOIN conversations v ON v.id = p.conversation_id
         JOIN repos r ON r.id = p.repo_id
         LEFT JOIN pull_request_standings s
              ON s.conversation_id = p.conversation_id
             AND s.repo_id = p.repo_id
             AND s.number = p.number
         WHERE v.state = ?
           AND (s.standing IS NULL OR s.standing = ?)
         ORDER BY p.conversation_id, p.event_id",
    )
    .bind(Lifecycle::Done.stored())
    .bind(Standing::Open.stored())
    .fetch_all(pool)
    .await
    .context("reading which pull requests are still waiting to land")?;

    Ok(rows
        .into_iter()
        .map(
            |(conversation_id, id, path, name, default_branch, number)| Unfinished {
                conversation_id,
                repo: Repo {
                    id,
                    path: std::path::PathBuf::from(path),
                    name,
                    default_branch,
                },
                number,
            },
        )
        .collect())
}
