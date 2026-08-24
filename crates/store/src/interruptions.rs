//! The Interruptions an unattended run stops at: what Verkstead noticed, the
//! evidence it gathered, and how the human settled it.
//!
//! An Interruption is something Verkstead detected and cannot resolve itself — a
//! session that exited badly, or one that ended having landed nothing. It is a
//! Timeline Event like any other, and the thing that makes it different is that
//! it is *open*: the run does not advance past one, and the Conversation carries
//! *blocked on you* until the human picks a remedy.
//!
//! At most one open per Conversation, by the unique index. That is what makes
//! *the run stops here* a fact about the database rather than a promise made by
//! whoever raised it: two open Interruptions would be two things the human had to
//! answer about one run that is not running.
//!
//! The evidence is held here rather than fetched when the human looks, and for
//! the reason a commit's summary is: it is a reading of a Worktree and a session
//! at the moment they went wrong, and both move on. A git status read an hour
//! later would be a status of whatever happened next.
//!
//! Nothing here does anything about an Interruption. Launching a session again,
//! standing back, or ending the run is the server's — see its `interruptions`
//! module. What this records is which of the three the human chose and what they
//! wrote alongside it.

use anyhow::{Context, Result, bail};
use sqlx::SqlitePool;

/// Which step a session was launched for, as the record holds it.
///
/// What retry needs in order to launch the same step again — and no more than
/// that: which *task* it was is the backlog's to say, not this. A retry of a task
/// runs the same fork that reads `.tasks/` and takes the lowest number left,
/// which is the same answer the run was already working from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// The breakdown that writes the backlog, which is a task-list run's first
    /// step.
    Planning,

    /// One task of that backlog.
    Task,

    /// The finish that follows the last of them.
    Finish,

    /// An inline implementation, which is the whole of the work in one session.
    Inline,

    /// Writing the roadmap, which is the whole of a roadmap Conversation's own
    /// work in one session: the stages are Conversations of their own.
    Roadmap,

    /// Planning a roadmap stage: the session that re-grounds the stage's brief
    /// and writes the backlog the runner then works.
    ///
    /// A backlog's first step like [`Self::Planning`], and a different one
    /// because a different fork runs it — the brief has to be re-grounded and
    /// the roadmap's own score moved, neither of which a breakdown knows about.
    Stage,

    /// Getting a wrapping Conversation's pull request green again.
    ///
    /// Not a step of a backlog at all — the backlog is finished by the time one
    /// of these is raised. What a retry means here is *have another go at the
    /// red checks*, which is the fix sessions starting over from no attempts
    /// spent.
    Checks,

    /// The self-review of a wrapping Conversation's branch.
    ///
    /// Not a step of a backlog either, and raised for a review session that
    /// ended without putting anything to the human — which is not the same thing
    /// as a review that found nothing. A retry is the review over again, in a
    /// session as fresh as the first one was.
    Review,

    /// Answering a batch of comments left on a wrapping Conversation's pull
    /// request.
    ///
    /// Not a step of a backlog either, and the review's twin one turn later: a
    /// batch session proposes what it would do about what was said and lands
    /// what the human accepted, so this is raised for one that did not finish,
    /// or that went with accepted work never landed. A retry is whichever half
    /// is left — the fixes alone where they were decided and never done, and
    /// nothing but the watching where the batch is settled and the wrap-up
    /// simply carries on.
    Comments,

    /// A Manual Task: the instruction the human typed at the end of the
    /// Timeline, run outside the pipeline altogether.
    ///
    /// Not a step of anything, and the only one whose *what* is written down
    /// nowhere but the Timeline — a bare word has no room for the paragraph the
    /// human typed. Which is what the word is for: a retry that finds it reads
    /// the instruction back off the newest Manual Task Event, rather than
    /// asking `.tasks/` what to run next.
    Manual,

    /// A Conversation that is **Stalled**: in a driven state with nothing
    /// driving it, which is a run that stopped without anything noticing it
    /// at the time.
    ///
    /// Not a step of anything either, and the one word here that names no
    /// session at all — nothing failed, because nothing was ever launched.
    /// What a retry means is decided by the state the Conversation is in
    /// rather than by anything a backlog says: the state is the whole of what
    /// says what ought to have been driving it.
    Stalled,
}

impl Step {
    /// The word the column holds. Lowercase and spelled out, so a database
    /// opened by hand says something.
    fn stored(self) -> &'static str {
        match self {
            Self::Planning => "planning",
            Self::Task => "task",
            Self::Finish => "finish",
            Self::Inline => "inline",
            Self::Roadmap => "roadmap",
            Self::Stage => "stage",
            Self::Checks => "checks",
            Self::Review => "review",
            Self::Comments => "comments",
            Self::Manual => "manual",
            Self::Stalled => "stalled",
        }
    }

    /// The step a stored word names. One this does not know is a database
    /// written by a Verkstead this one does not understand, exactly as an
    /// unknown lifecycle state is.
    fn read(word: &str) -> Result<Self> {
        Ok(match word {
            "planning" => Self::Planning,
            "task" => Self::Task,
            "finish" => Self::Finish,
            "inline" => Self::Inline,
            "roadmap" => Self::Roadmap,
            "stage" => Self::Stage,
            "checks" => Self::Checks,
            "review" => Self::Review,
            "comments" => Self::Comments,
            "manual" => Self::Manual,
            "stalled" => Self::Stalled,
            other => bail!("an Interruption names the unknown step {other:?}"),
        })
    }
}

/// One of the three things the human can do about an Interruption.
///
/// Roadrunner's remedies, and roadrunner's meanings: run the step again, stand
/// back and take it on, or end the run. In every case the repository is left as
/// the session left it — none of the three reverts, resets or stashes anything.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Remedy {
    /// Run the step again in a fresh session, told whatever the human wrote
    /// alongside.
    Retry,

    /// Verkstead stops driving, so the human can take the step on themselves.
    TakeOver,

    /// The run ends here.
    Abort,
}

impl Remedy {
    fn stored(self) -> &'static str {
        match self {
            Self::Retry => "retry",
            Self::TakeOver => "take-over",
            Self::Abort => "abort",
        }
    }

    fn read(word: &str) -> Result<Self> {
        Ok(match word {
            "retry" => Self::Retry,
            "take-over" => Self::TakeOver,
            "abort" => Self::Abort,
            other => bail!("an Interruption was settled by the unknown remedy {other:?}"),
        })
    }
}

/// What makes the choice answerable without opening a terminal: which step
/// failed, how it ended, what git makes of the Worktree, and the tail of what
/// the session last said.
///
/// Gathered at the moment the run stopped and kept, because all four of them
/// move on: a Worktree is a directory the human also has, and a session's output
/// belongs to a process that has gone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Evidence {
    pub step: Step,

    /// What the step was, in words — "the breakdown", "a task of the backlog".
    /// The sentence rather than the word above it, because that is what the
    /// Timeline draws and the two are read by different readers.
    pub what: String,

    /// How it ended: the exit status, or that it ended without landing anything.
    pub how: String,

    /// What git makes of the Worktree, as `git status` says it. Empty where the
    /// repository would not answer, or where there was nothing pending.
    pub git_status: String,

    /// The tail of what the session printed, tidied of the terminal's own
    /// control sequences. Empty where it printed nothing at all.
    pub tail: String,
}

/// An Interruption, whole: what happened, and what became of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Interruption {
    pub evidence: Evidence,

    /// How the human settled it, or `None` while it is still open — which is
    /// the state the run is stopped in.
    pub settled: Option<Settled>,
}

/// How an Interruption was settled: which remedy, whatever the human wrote
/// alongside it, and when.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settled {
    pub remedy: Remedy,

    /// What the human wrote alongside the choice — "try again but leave the
    /// migration alone". Empty where they wrote nothing, which is the ordinary
    /// case for the two remedies that launch nothing.
    pub note: String,

    /// When they chose, RFC 3339.
    pub at: String,
}

/// What became of settling one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Settling {
    /// Recorded: the Interruption is closed, and the remedy is what to act on.
    Settled,

    /// This Conversation has no such Interruption. An Event id belonging to
    /// another Conversation names nothing here, exactly as a Capture's does.
    NoSuchInterruption,

    /// It was settled already — from another device, or by a second press. Not
    /// an error, and not something to act on twice: the first choice stands.
    AlreadySettled,
}

/// The interruptions table. It hangs off a Timeline Event, as a commit does: an
/// Interruption is one Event's full self, and the Event is what a Timeline holds.
///
/// The Conversation is on the row as well as on the Event above it, which is the
/// one thing here written twice. It is what the partial unique index needs — *one
/// open Interruption per Conversation* is the rule, and SQLite cannot index a
/// column that lives in another table.
pub(crate) async fn apply_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS interruptions (
             event_id        INTEGER PRIMARY KEY REFERENCES timeline_events(id),
             conversation_id INTEGER NOT NULL REFERENCES conversations(id),
             step            TEXT NOT NULL,
             what            TEXT NOT NULL,
             how             TEXT NOT NULL,
             git_status      TEXT NOT NULL,
             tail            TEXT NOT NULL,
             remedy          TEXT,
             note            TEXT,
             settled_at      TEXT
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the interruptions table")?;

    // Partial, so that it constrains only the open ones: a Conversation may
    // collect any number of settled Interruptions over a long run, and exactly
    // one that is stopping it.
    sqlx::query(
        "CREATE UNIQUE INDEX IF NOT EXISTS interruptions_open
             ON interruptions (conversation_id) WHERE remedy IS NULL",
    )
    .execute(pool)
    .await
    .context("indexing the open Interruptions by their Conversation")?;

    Ok(())
}

/// Put an Interruption on a Conversation's Timeline, and say which Event it
/// became.
///
/// `None` is a Conversation that already has one open, or one that is not there:
/// neither is a failure. A run stops at its first Interruption, so a second
/// raised against the same Conversation is something that got past the guard —
/// two watchers noticing the same dead session — and the first one is the one
/// the human is being asked about.
///
/// One transaction, because an Event without its row is a Timeline holding an
/// Interruption that cannot say what happened — and [`Event::read`] refuses one
/// rather than drawing it as an interruption of nothing.
pub async fn record_interruption(
    pool: &SqlitePool,
    conversation_id: i64,
    evidence: &Evidence,
) -> Result<Option<i64>> {
    let mut tx = pool.begin().await.context("raising an Interruption")?;

    // Asked inside the transaction, so the answer still holds when the insert
    // below acts on it. The partial unique index is what settles it either way.
    let open: Option<(i64,)> = sqlx::query_as(
        "SELECT event_id FROM interruptions
         WHERE conversation_id = ? AND remedy IS NULL",
    )
    .bind(conversation_id)
    .fetch_optional(&mut *tx)
    .await
    .with_context(|| {
        format!("looking for an open Interruption on Conversation {conversation_id}")
    })?;

    if open.is_some() {
        return Ok(None);
    }

    // Selected from `conversations` rather than trusting the id, as every other
    // Event is written: SQLite enforces a foreign key only when asked to, and an
    // Interruption attributed to a Conversation that is not there would be one
    // nobody could ever answer.
    let event: Option<(i64,)> = sqlx::query_as(
        "INSERT INTO timeline_events (conversation_id, at, kind, body)
         SELECT id, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), ?, ''
         FROM conversations WHERE id = ?
         RETURNING id",
    )
    .bind(INTERRUPTION)
    .bind(conversation_id)
    .fetch_optional(&mut *tx)
    .await
    .with_context(|| {
        format!("putting an Interruption on the Timeline of Conversation {conversation_id}")
    })?;

    let Some((event_id,)) = event else {
        return Ok(None);
    };

    sqlx::query(
        "INSERT INTO interruptions
             (event_id, conversation_id, step, what, how, git_status, tail)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(event_id)
    .bind(conversation_id)
    .bind(evidence.step.stored())
    .bind(&evidence.what)
    .bind(&evidence.how)
    .bind(&evidence.git_status)
    .bind(&evidence.tail)
    .execute(&mut *tx)
    .await
    .with_context(|| format!("recording the Interruption of Event {event_id}"))?;

    tx.commit().await.context("raising an Interruption")?;

    Ok(Some(event_id))
}

/// Which Event a Conversation's open Interruption is, or `None` where nothing is
/// stopping it.
///
/// What the *blocked on you* badge is drawn from, and what the runner asks before
/// it launches anything: a run does not advance past an Interruption, and the one
/// place that is decided is here.
pub async fn open_interruption(pool: &SqlitePool, conversation_id: i64) -> Result<Option<i64>> {
    let row: Option<(i64,)> = sqlx::query_as(
        "SELECT event_id FROM interruptions
         WHERE conversation_id = ? AND remedy IS NULL",
    )
    .bind(conversation_id)
    .fetch_optional(pool)
    .await
    .with_context(|| format!("reading the open Interruption of Conversation {conversation_id}"))?;

    Ok(row.map(|(event_id,)| event_id))
}

/// The Interruption one of a Conversation's Events is, or `None` where that
/// Conversation has no such Event.
///
/// The Conversation is part of the question rather than trusted from the path,
/// exactly as a Capture's and a commit's are: an Interruption is reached
/// through the Timeline it is on.
pub async fn interruption(
    pool: &SqlitePool,
    conversation_id: i64,
    event_id: i64,
) -> Result<Option<Interruption>> {
    type Row = (
        String,
        String,
        String,
        String,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
    );

    let row: Option<Row> = sqlx::query_as(
        "SELECT step, what, how, git_status, tail, remedy, note, settled_at
         FROM interruptions
         WHERE event_id = ? AND conversation_id = ?",
    )
    .bind(event_id)
    .bind(conversation_id)
    .fetch_optional(pool)
    .await
    .with_context(|| format!("reading the Interruption of Event {event_id}"))?;

    let Some((step, what, how, git_status, tail, remedy, note, settled_at)) = row else {
        return Ok(None);
    };

    Ok(Some(read(
        step, what, how, git_status, tail, remedy, note, settled_at,
    )?))
}

/// Every Interruption on a Conversation's Timeline, by the Event each one is.
///
/// The Timeline's own read asks this rather than joining the row in, and the
/// reason is arithmetic: a row here is eight columns, and the Timeline's query
/// is already at the sixteen positions a tuple can be read back as. It costs
/// little where the join would not have — an Interruption is the rare Event, and
/// nearly every Conversation answers this with nothing.
pub(crate) async fn on_timeline(
    pool: &SqlitePool,
    conversation_id: i64,
) -> Result<std::collections::HashMap<i64, Interruption>> {
    type Row = (
        i64,
        String,
        String,
        String,
        String,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
    );

    let rows: Vec<Row> = sqlx::query_as(
        "SELECT event_id, step, what, how, git_status, tail, remedy, note, settled_at
         FROM interruptions
         WHERE conversation_id = ?",
    )
    .bind(conversation_id)
    .fetch_all(pool)
    .await
    .with_context(|| format!("reading the Interruptions of Conversation {conversation_id}"))?;

    rows.into_iter()
        .map(
            |(event_id, step, what, how, git_status, tail, remedy, note, settled_at)| {
                Ok((
                    event_id,
                    read(step, what, how, git_status, tail, remedy, note, settled_at)?,
                ))
            },
        )
        .collect()
}

/// Settle one: the human has chosen a remedy, and this is what they wrote
/// alongside it.
///
/// Recorded before anything acts on the choice, for the reason a worktree is
/// recorded before a session is launched in it: an Interruption that was acted on
/// without being closed would be one the human could answer twice, and a retry
/// pressed twice is two agents in one Worktree.
///
/// [`Settling::AlreadySettled`] is an ordinary outcome. The human answers from
/// whichever device is to hand, and the second press of a button is the first
/// choice arriving again rather than a new decision.
pub async fn settle_interruption(
    pool: &SqlitePool,
    conversation_id: i64,
    event_id: i64,
    remedy: Remedy,
    note: &str,
) -> Result<Settling> {
    let mut tx = pool.begin().await.context("settling an Interruption")?;

    let found: Option<(Option<String>,)> = sqlx::query_as(
        "SELECT remedy FROM interruptions WHERE event_id = ? AND conversation_id = ?",
    )
    .bind(event_id)
    .bind(conversation_id)
    .fetch_optional(&mut *tx)
    .await
    .with_context(|| format!("looking for the Interruption of Event {event_id}"))?;

    let Some((settled,)) = found else {
        return Ok(Settling::NoSuchInterruption);
    };

    if settled.is_some() {
        return Ok(Settling::AlreadySettled);
    }

    sqlx::query(
        "UPDATE interruptions
         SET remedy = ?, note = ?, settled_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
         WHERE event_id = ?",
    )
    .bind(remedy.stored())
    .bind(note)
    .bind(event_id)
    .execute(&mut *tx)
    .await
    .with_context(|| format!("settling the Interruption of Event {event_id}"))?;

    tx.commit().await.context("settling an Interruption")?;

    Ok(Settling::Settled)
}

/// The word the `kind` column holds for an Interruption.
///
/// A constant beside the Event's own spelling for the reason a Question Set's is:
/// the row is written before there is an Event to ask, [`record_interruption`]
/// inserting both in one transaction.
pub(crate) const INTERRUPTION: &str = "interruption";

/// An Interruption out of the columns a Timeline row joins in, or the row it was
/// read from being a Verkstead this one does not understand.
///
/// Shared by [`interruption`] and the Timeline's own read, so that one place
/// knows how the three settlement columns go together: they are written in one
/// statement, and a row with a remedy and no time on it is a database somebody
/// has been in by hand.
#[allow(clippy::too_many_arguments)]
pub(crate) fn read(
    step: String,
    what: String,
    how: String,
    git_status: String,
    tail: String,
    remedy: Option<String>,
    note: Option<String>,
    settled_at: Option<String>,
) -> Result<Interruption> {
    let settled = match remedy.zip(settled_at) {
        Some((remedy, at)) => Some(Settled {
            remedy: Remedy::read(&remedy)?,
            note: note.unwrap_or_default(),
            at,
        }),
        None => None,
    };

    Ok(Interruption {
        evidence: Evidence {
            step: Step::read(&step)?,
            what,
            how,
            git_status,
            tail,
        },
        settled,
    })
}
