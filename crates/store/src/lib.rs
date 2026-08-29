//! SQLite persistence for Question Sets, their Responses, and the locking of
//! the ones nobody will ever answer.
//!
//! A Set and a Response are each kept as one JSON body — JSON rather than YAML
//! because it preserves a Preface's whitespace exactly, and the Preface is
//! markdown the human reads back verbatim. `title`, `project` and `branch` are
//! lifted into columns beside the body, so that what a Set is *about* can be
//! read without deserializing it.
//!
//! A stored body this build's schema will not take is read back as
//! [`Asked::Unreadable`] rather than as a failure — see there for why one
//! unreadable record has to cost its own row and nothing beside it.
//!
//! Every Set is asked from a Conversation and lands on its Timeline — see
//! [`ask`], which is the one way one is stored. What answering it does is here
//! all the same: a Response reaches the waiting agent the same way whether it
//! came from the workbench, from a phone or from `curl`.
//!
//! The store sits below both the agent API and the web UI: the UI's server
//! functions live in the shared `verkstead-app` crate, which cannot reach back
//! into the server binary that links it.

use std::path::Path;

use anyhow::{Context, Result, anyhow};
use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode};
use tokio::sync::broadcast;
use verkstead_schema::{QuestionSet, Response, ResponseAccepted, ValidationError};

mod archives;
mod captures;
mod commits;
mod companions;
mod conversations;
mod deferrals;
mod endings;
mod migrations;
mod pairings;
mod pauses;
mod placements;
mod profiles;
mod pull_requests;
mod push;
mod repos;
mod session_names;
mod stops;
mod transcripts;
mod unseen;
mod waits;
mod wrap_up;

pub use archives::{
    Archiving, Unarchiving, archive_conversation, archived, show_archived, showing_archived,
    unarchive_conversation,
};
pub use captures::{Summary, append_capture, capture, start_capture, summarise_capture};
pub use commits::{Commit, commit, commit_repo, commits_landed, record_commit, recorded_commits};
pub use companions::{
    Adding, Change, Companion, CompanionMode, CompanionWorktree, Configured, Joining, Opening,
    Removing, add_companion, companions, configure_companion, remove_companion,
};
pub use conversations::{
    Chosen, Closing, Conversation, ConversationRow, Directing, Edited, Ending, Event, Grilling,
    Implementing, Landed, Lifecycle, Rebuilding, Role, RowState, SetOnTimeline, Settling, Staged,
    Steer, Steering, TimelineEvent, adopting, ask, asked_from, close_conversation, conversations,
    follow_up_over, implement_again, last_batch_proposal, last_proposal, load_conversation, note,
    open_set, pick_direction, record_backlog, record_handoff, record_roadmap, rename_branch,
    save_brief, set_asked_from, set_base_commit, set_grilling_pairing, set_implementation_pairing,
    set_state, stacks_on, start_adoption, start_conversation, start_grilling, start_implementing,
    start_stage, state, steer_conversation, timeline, unanswered_set_since,
};
pub use deferrals::{Ask, Unfolded, deferred, deferred_on_timeline, record_folded, unfolded};
pub use endings::{ended_on, nothing_else};
pub use pairings::{RepoPairings, remembered_pairings};
pub use pauses::Pause;
pub use placements::place_conversations;
pub use profiles::{
    AgentType, Deleting, Pairing, Profile, ProfileFacts, Saving, create_profile, delete_profile,
    load_profile, profiles, update_profile,
};
pub use pull_requests::{
    PullRequest, Rollup, Wrapping, check_rollup, pull_request, pull_request_repo, pull_requests,
    record_another_pull_request, record_check_rollup, record_pull_request,
};
pub use push::{
    PushSubscription, Subscribing, VapidKeys, forget_subscription, push_subscriptions,
    store_subscription, vapid_keys,
};
pub use repos::{Repo, load_repo, register_repo, registered_repos};
pub use session_names::session_id;
pub use stops::{
    Decision, Stopped, ask_to_stop, asked_to_stop, clear_stop, forget_stop, stop, stopped,
};
pub use transcripts::{append_transcript, transcript, transcript_after};
pub use unseen::{see_conversation, stamp_unseen};
pub use waits::{WaitHeld, Waits};
pub use wrap_up::{
    Finished, Narrowing, WAITED_ON, WaitingOn, addressed_comments, finish_wrap_up, fix_attempts,
    forget_addressed_comments, forget_every_addressed_comment, forget_fix_attempts,
    forget_narrowing, narrowed_to_checks, narrowing, record_addressed_comments, record_fix_attempt,
    settle_wrap_up, unsettle_wrap_up, wrap_up_settled,
};

/// A Set as the store holds it: what was asked plus the identity the server
/// stamped on it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredSet {
    pub id: i64,
    pub created_at: String,
    pub set: Asked,

    /// Whether it was a Deferred Ask: stored and returned to, with no session
    /// idling on the Answer — see [`deferrals`].
    pub deferred: bool,
}

/// What a stored Question Set is, as far as this build can read it.
///
/// ADR-0006's rule for Transcript lines, applied to the Sets themselves: keep
/// what was written, and defer rendering it rather than lose the record. There
/// is no migration machinery here by design, so every field that leaves the
/// schema leaves stored bodies this build's `deny_unknown_fields` will not
/// take — and one of those must cost its own row and nothing beside it, where
/// a failure propagated out of a read would cost the whole Timeline it is on.
///
/// Nothing here ever rewrites a stored body. It is the record of what was
/// asked, and a Verkstead that can read it again later should still find it as
/// it was written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Asked {
    /// The Set, as the agent sent it.
    Set(QuestionSet),

    /// A stored body this build cannot read, kept as it stands.
    Unreadable(Unreadable),
}

/// A stored Question Set nothing here can deserialize: the body, and what
/// reading it came to.
///
/// Both, because between them they are the whole of what is left to say about
/// one — the body is what was asked, and the reason is why this build cannot
/// say what that was.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unreadable {
    /// The stored JSON, byte for byte.
    pub body: String,

    /// What deserializing it said.
    pub why: String,
}

impl Asked {
    /// What a stored body holds, kept as it stands where this build cannot read
    /// it.
    ///
    /// The one place a stored Set is deserialized, which is what makes the
    /// fallback the same fallback everywhere rather than something each caller
    /// has to remember.
    fn read(body: String) -> Self {
        match serde_json::from_str(&body) {
            Ok(set) => Self::Set(set),
            Err(error) => Self::Unreadable(Unreadable {
                body,
                // Serde's own sentence, kept as it is. It names the field that
                // is no longer in the schema and where in the body it sits,
                // which is exactly what somebody looking at the row wants to
                // know, and nothing here could word it better.
                why: error.to_string(),
            }),
        }
    }

    /// The Set where this build can read it, and nothing where it cannot.
    ///
    /// What everything walking a Timeline for the Sets on it asks: an
    /// unreadable one carries no proposal to settle and no exchange to put in a
    /// prompt, so passing over it is the whole of what there is to do about one.
    pub fn set(&self) -> Option<&QuestionSet> {
        match self {
            Self::Set(set) => Some(set),
            Self::Unreadable(_) => None,
        }
    }

    /// The Set, or an error naming why there is none to be had.
    ///
    /// For the callers that cannot go on without it — answering above all,
    /// which is checked against the Questions it resolves. What they fail is
    /// the unreadable Set's own action, which is as far as one is allowed to
    /// reach.
    pub fn readable(&self, set_id: i64) -> Result<&QuestionSet> {
        match self {
            Self::Set(set) => Ok(set),
            Self::Unreadable(unreadable) => Err(anyhow!(
                "Question Set {set_id} cannot be read: {}",
                unreadable.why
            )),
        }
    }
}

/// A Response as the store holds it: the human's reply plus when it landed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredResponse {
    pub set_id: i64,
    pub submitted_at: String,
    pub response: Response,
}

/// How a Set was settled, for whoever is waiting to hear that it was.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Settlement {
    /// Answered: this is the Response, and the wait is over.
    Answered(StoredResponse),

    /// Locked unanswered by the human. Nothing is coming.
    LockedUnanswered(SetLocked),
}

/// A Set the human closed unanswered: which Set, and when they closed it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetLocked {
    pub set_id: i64,

    /// When the server took the lock, RFC 3339.
    pub locked_at: String,
}

/// A Set that has just been settled: which Set, and the Conversation it was
/// asked from.
///
/// The Conversation rides along because the other listener is the viewer's Nudge
/// stream, which has to say where the change happened (ADR-0009) and would
/// otherwise ask the store the question the store had just answered for itself.
/// It is optional for the one case that should never happen — a Set with no
/// Timeline Event behind it — rather than because a Set can be asked from
/// nowhere.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SettledSet {
    pub set_id: i64,
    pub conversation_id: Option<i64>,
}

/// Word that a Set has just been settled — answered, or locked unanswered —
/// so a wait held on it can end without going back to the store to look.
///
/// It lives beside the store for the same reason the store is its own crate:
/// the agent API's long-poll, the web UI's submit and the web UI's locking are
/// in different crates, and they have to meet at the same channel or a browser
/// would leave a waiting agent hanging until its hold window closed.
#[derive(Debug, Clone)]
pub struct Settlements(broadcast::Sender<SettledSet>);

impl Settlements {
    /// A channel that will hold `backlog` announcements for a listener that
    /// falls behind.
    pub fn new(backlog: usize) -> Self {
        let (announcements, _) = broadcast::channel(backlog);
        Self(announcements)
    }

    /// Hear about Sets settled from now on. Subscribe before reading the store,
    /// so a Response landing between the two wakes the wait instead of slipping
    /// past it.
    pub fn subscribe(&self) -> broadcast::Receiver<SettledSet> {
        self.0.subscribe()
    }

    /// Tell whoever is listening that this Set is settled. A send error means
    /// nobody is, which is the ordinary case — the agent may well be between
    /// polls.
    fn announce(&self, settled: SettledSet) {
        let _ = self.0.send(settled);
    }
}

/// A Response that was taken: what the Set now says, and what answering it moved.
///
/// What it moved travels back with the acceptance rather than being logged here,
/// because the store has no voice of its own: what it does is hand back what
/// happened, and whichever half of the server took the Response is what says so.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Taken {
    pub accepted: ResponseAccepted,

    /// What became of the wrap-up proposal this Set carried, or `None` where it
    /// carried none — which is every ordinary Set.
    pub proposed: Option<Proposed>,
}

/// What became of a wrap-up proposal the human has just answered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Proposed {
    /// The human picked no direction, so the grilling carries on.
    ///
    /// Nothing is recorded and nothing is wrong: this is how a human disagrees.
    /// The session that proposed is still holding the thread and has their
    /// Response to read, and what it does with it — keep grilling, or propose
    /// again — is the agent's own to decide.
    SentBack,

    /// Accepted: the human picked a direction, and this is what became of
    /// moving the Conversation on.
    ///
    /// The pick travels with it because it is the whole of what the server does
    /// next — the direction is already settled, so nobody has to be asked a
    /// second time.
    ///
    /// Accepted is not the same as moved, and no pick moves anything:
    /// [`Directing::Writing`] is the grilling session carrying on to write the
    /// picked Direction's artifact, which is what moves the Conversation when it
    /// lands. [`Directing::NotGrilling`] is not a failure either — it is a pick
    /// answered after the grilling it was put from had already ended.
    Accepted {
        direction: verkstead_schema::Direction,
        directing: Directing,
    },
}

/// What became of a submitted Response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Submission {
    /// Stored as the Set's answer, and everyone waiting on it has been told.
    Accepted(Taken),

    /// There is no Set with that id to answer.
    NoSuchSet,

    /// The Response does not resolve the Set, so it is not an answer to it.
    Invalid(ValidationError),

    /// The Set was answered before this Response arrived. A Set is answered
    /// once, and the first Response stands.
    AlreadyAnswered,

    /// The Set was locked unanswered, which closed it for good: a locked
    /// Set cannot also become an answered one.
    Locked,
}

/// Answer a Set: check the Response resolves it, store it, and wake whoever is
/// waiting.
///
/// This is the one path a Response takes, whether it came from the human's
/// browser or from `curl`, so a wait ends the same way either way — and so an
/// locked Set is refused the same way either way.
///
/// It is also where a direction is settled. A Set carrying a `proposal` is the
/// grilling agent's closing move, and a Response that picks a direction on one
/// records the pick and moves whatever the pick moves — here, rather than in
/// either endpoint, for the reason everything else about answering is here: the
/// browser and `curl` must not be able to leave a Conversation in different
/// states for the same Answer.
pub async fn submit_response(
    pool: &SqlitePool,
    settlements: &Settlements,
    set_id: i64,
    response: &Response,
) -> Result<Submission> {
    let Some(stored) = load_set(pool, set_id).await? else {
        return Ok(Submission::NoSuchSet);
    };

    // A Response is checked against the Questions it resolves, so a Set this
    // build cannot read cannot be answered: there is nothing to check it
    // against. The failure is this Set's own — nothing else on its Timeline is
    // touched by it — and the workbench offers no way to get here, since it
    // draws an unreadable Set as a record rather than as a sheet.
    let set = stored.set.readable(set_id)?;

    if let Err(invalid) = response.validate(set) {
        return Ok(Submission::Invalid(invalid));
    }

    let Some(accepted) = insert_response(pool, set_id, response).await? else {
        // The insert refuses both ways a Set can already be settled, so which of
        // the two it was is read back rather than assumed.
        return Ok(match load_lock(pool, set_id).await? {
            Some(_) => Submission::Locked,
            None => Submission::AlreadyAnswered,
        });
    };

    // After the Response is stored, and only for the Set that carries a
    // proposal. The insert is what makes a Set answered once, so a proposal is
    // settled once too: a second Response to the same Set never gets this far.
    //
    // The pick is the whole of accepting — see [`Response::direction`] — so
    // there is nothing here to read off the Answers.
    let proposed = match (&set.proposal, response.direction) {
        (Some(_), Some(direction)) => Some(Proposed::Accepted {
            direction,
            directing: accept_proposal(pool, set_id, direction).await?,
        }),
        // Answered without a pick, which is how a human disagrees: the grilling
        // carries on, and the agent has their Response.
        (Some(_), None) => Some(Proposed::SentBack),
        (None, _) => None,
    };

    // A wrap-up's Set carries its Answers now too, and answering it moves nothing
    // at all. What wrap-up waits on is the session that asked, which is still
    // running: the Response goes back to it, it does what was accepted, and its
    // ending cleanly is what settles the review — see [`last_proposal`], which is
    // how the Set is found again afterwards.
    settlements.announce(settled_set(pool, set_id).await?);

    Ok(Submission::Accepted(Taken { accepted, proposed }))
}

/// Act on the direction picked on an accepted proposal, for the Conversation the
/// Set was asked from.
///
/// A Set is on exactly one Timeline, so which Conversation to move is read off
/// the Set rather than passed in: the agent-facing endpoint knows which
/// Conversation it is answering for and the viewer's does not, and this is the
/// one path both of them take.
///
/// [`Directing::NoSuchConversation`] therefore covers a Set on no Timeline as
/// well, which cannot happen for a stored Set — [`ask`] writes the Set, its Event
/// and the row joining them in one transaction. It is handed back rather than
/// raised because the Response is stored by the time this runs, and failing the
/// submission now would leave a waiting agent holding a Set that has in fact been
/// answered.
async fn accept_proposal(
    pool: &SqlitePool,
    set_id: i64,
    direction: verkstead_schema::Direction,
) -> Result<Directing> {
    let Some(conversation_id) = asked_from(pool, set_id).await? else {
        return Ok(Directing::NoSuchConversation);
    };

    pick_direction(pool, conversation_id, direction).await
}

/// What became of locking a Set unanswered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Locking {
    /// Closed: the Set has stopped waiting on the human, and anyone holding a
    /// wait on it has been told.
    Locked(SetLocked),

    /// There is no Set with that id to lock.
    NoSuchSet,

    /// The Set has a Response, so it is a decision already. Locking
    /// unanswered is for a Set nobody will ever answer, and it does not touch
    /// what was decided.
    AlreadyAnswered,

    /// The Set was locked before this arrived — from another device, or
    /// another tab.
    AlreadyLocked,
}

/// Close a Set the human is never going to be able to answer: settle it
/// unanswered, and tell whoever is waiting on it.
///
/// Only a human may do this (ADR-0001): a disconnected agent is never enough,
/// because the CLI reconnects through transient drops. Nothing here consults
/// Liveness — the decision is the human's, taken with the badge in front of
/// them.
pub async fn lock_set(
    pool: &SqlitePool,
    settlements: &Settlements,
    set_id: i64,
) -> Result<Locking> {
    if !set_exists(pool, set_id).await? {
        return Ok(Locking::NoSuchSet);
    }

    let Some(locked) = insert_lock(pool, set_id).await? else {
        return Ok(match load_response(pool, set_id).await? {
            Some(_) => Locking::AlreadyAnswered,
            None => Locking::AlreadyLocked,
        });
    };

    settlements.announce(settled_set(pool, set_id).await?);

    Ok(Locking::Locked(locked))
}

/// What to announce about a Set that has just settled: the Set, and where it was
/// asked from.
///
/// The Conversation is looked up rather than carried down from the caller
/// because both settling paths are reached with a Set id and nothing else — the
/// browser's locking has only the id in its route, and the agent's Response
/// only the id it was told to answer.
async fn settled_set(pool: &SqlitePool, set_id: i64) -> Result<SettledSet> {
    Ok(SettledSet {
        set_id,
        conversation_id: asked_from(pool, set_id).await?,
    })
}

/// Open the SQLite database at `path`, creating the file if it is absent and
/// bringing its schema up to date.
///
/// **Write-ahead logging**, which sqlx will not turn on by itself — it leaves
/// `journal_mode` alone because switching a database into or out of WAL takes an
/// exclusive lock no busy timeout can wait on, and it will not do that behind an
/// application's back. Here it is this application's decision to make, and the
/// moment to make it is this one: a server that has just opened its database has
/// nothing else running against it.
///
/// It is worth making because of what the default costs. Under a rollback
/// journal a reader and a writer cannot hold the file at once, so every poll of
/// a Timeline is something a session's Capture write has to queue behind. Verkstead
/// writes continuously while a session runs and reads on every open page. WAL is
/// the mode that shape of use is for.
///
/// It is not, on its own, what makes a write safe from a concurrent one — see
/// [`writing`], which is the other half.
pub async fn open_database(path: &Path) -> Result<SqlitePool> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating database directory {}", parent.display()))?;
    }

    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal);

    let pool = SqlitePool::connect_with(options)
        .await
        .with_context(|| format!("opening database {}", path.display()))?;

    apply_schema(&pool).await?;

    Ok(pool)
}

/// Open a transaction that is going to write, with `doing` as the words any
/// failure of it is reported under.
///
/// `BEGIN IMMEDIATE` rather than the plain `BEGIN` sqlx opens, and every
/// transaction in the store goes through here, because every transaction in the
/// store writes. What each of them does is read the record, decide on it, and
/// write — a state read before the move it authorises, a count read before the
/// row that changes it — and that shape is the one SQLite handles worst.
///
/// A deferred `BEGIN` takes no lock. The first read takes a shared one, and the
/// first write then has to promote it. **SQLite will not wait for that
/// promotion**: where another connection is holding its own read of the same
/// database, promoting would deadlock the pair of them, so rather than call the
/// busy handler it fails the statement at once with *database is locked*. The
/// five-second busy timeout never comes into it, and no amount of raising it
/// would. Under a rollback journal that is a shared lock in the way; under WAL
/// it is `SQLITE_BUSY_SNAPSHOT`, another connection having committed since the
/// snapshot this transaction is reading. Both are the same bug to a caller.
///
/// `BEGIN IMMEDIATE` takes the write lock up front, before the first read, so
/// there is no promotion to fail — and *waiting for a lock that is already
/// held* is exactly the case the busy timeout does cover. The cost is that
/// writers queue against each other from the first statement rather than the
/// first write, which is the right trade for a store whose transactions are all
/// short and all end in a write.
///
/// This was not a theoretical failure. A finish step recorded its pull request
/// through [`record_pull_request`] while the session that opened it was still
/// writing its Capture, the promotion failed, and the Conversation was left
/// implementing with the work on a pull request nothing knew about.
pub(crate) async fn writing(
    pool: &SqlitePool,
    doing: &'static str,
) -> Result<sqlx::Transaction<'static, sqlx::Sqlite>> {
    pool.begin_with("BEGIN IMMEDIATE").await.context(doing)
}

/// Bring an opened database up to the shape the server expects. Safe to run
/// against a database that already has it.
async fn apply_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS question_sets (
             id         INTEGER PRIMARY KEY AUTOINCREMENT,
             created_at TEXT NOT NULL,
             title      TEXT NOT NULL,
             project    TEXT,
             branch     TEXT,
             body       TEXT NOT NULL
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the question_sets table")?;

    // One Response per Set, enforced by the primary key rather than by a
    // read-then-write the second submitter could slip through.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS responses (
             set_id       INTEGER PRIMARY KEY REFERENCES question_sets(id),
             submitted_at TEXT NOT NULL,
             body         TEXT NOT NULL
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the responses table")?;

    // A lock hangs off a Set exactly as its Response does, rather than being a
    // column on it: there is no migration machinery here, and `question_sets`
    // is STRICT and left alone. One lock per Set, by the same primary key, for
    // the same reason.
    //
    // `archivings` is the name a lock was stored under when locking was called
    // archiving, and it stays that for the missing migration machinery: a
    // database written before the rename keeps its locks in it, and a renamed
    // table would leave them behind.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS archivings (
             set_id      INTEGER PRIMARY KEY REFERENCES question_sets(id),
             archived_at TEXT NOT NULL
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the locks table")?;

    // Which Sets were asked deferred, and which of those have been folded into a
    // prompt. It hangs off a Set for a lock's reason, said again there.
    deferrals::apply_schema(pool).await?;

    // And which Responses said there was nothing else, which hangs off a Set for
    // that reason too — and for one of its own: what is kept here is deliberately
    // not in the body the agent is handed. See [`endings`].
    endings::apply_schema(pool).await?;

    // The push identity and the devices subscribed to it, which also generates
    // the keypair when this is the database's first run.
    push::apply_schema(pool).await?;

    // The Repos registered from inside the Watched Paths.
    repos::apply_schema(pool).await?;

    // The Agent Profiles a session can be run under.
    profiles::apply_schema(pool).await?;

    // The Conversations attached to them, and their Timelines. After the Repos
    // and the Profiles, because a Conversation's row references all three.
    conversations::apply_schema(pool).await?;

    // And the other registered Repos each Conversation works alongside. After
    // the Conversations and the Repos both, because a companion's row
    // references one of each.
    companions::apply_schema(pool).await?;

    // And what each Repo was last grilled with, so a Conversation started on
    // it arrives with both pickers filled. After the Conversations only for
    // reading order — what it references is the Repos and the Profiles.
    pairings::apply_schema(pool).await?;

    // What the sessions run against them printed. After the Timelines, because a
    // Capture hangs off the Event it is the full self of.
    captures::apply_schema(pool).await?;

    // And what Verkstead called each of those sessions, which hangs off the
    // same Event for the same reason.
    session_names::apply_schema(pool).await?;

    // And the record those sessions kept of themselves, which hangs off the
    // same Event again — one session is one Event, and one Event is one
    // Transcript.
    transcripts::apply_schema(pool).await?;

    // And what they committed, which hangs off the Timelines for the same
    // reason — and off the Conversations too, which is what makes one commit
    // per Conversation a rule the database keeps.
    commits::apply_schema(pool).await?;

    // The Pauses a Verkstead of before put on a Timeline when an account ran
    // out of window. Nothing writes one any more — an exhausted window stops a
    // run the way everything else does — and the table stays because those
    // Events are the record of what happened and still have to read back.
    pauses::apply_schema(pool).await?;

    // And that driving has stopped, which is columns on the Conversation
    // itself: a stop is how things are rather than something that happened,
    // and what did happen is the Notice it points at. After the Pauses,
    // because a database written before this carries its open ones onto the
    // Conversations as the columns arrive — see [`stops::apply_schema`].
    stops::apply_schema(pool).await?;

    // And what the work ended up on, which hangs off the Timelines the same way
    // — and off the Conversations, which is what makes *one pull request per
    // Conversation* a rule the database keeps.
    pull_requests::apply_schema(pool).await?;

    // And what wrapping that pull request up is still waiting on, which hangs
    // off the Conversations alone: none of it is something that happened, so
    // none of it is an Event.
    wrap_up::apply_schema(pool).await?;

    // And where the human put each Conversation in the sidebar, which hangs off
    // the Conversations alone for that reason too — an order is a fact about the
    // list rather than a thing that happened to the work.
    placements::apply_schema(pool).await?;

    // And which of them the human has put away, which the sidebar reads the
    // same way and for the same reason: what a list draws is not a fact about
    // the work either.
    archives::apply_schema(pool).await?;

    // And which of them Verkstead has told the human about and they have not
    // looked at yet, which sits beside the Conversations for that reason again —
    // and is the one fact here about the person reading the list rather than
    // about the work on it. See [`unseen`].
    unseen::apply_schema(pool).await?;

    // And last of all, whatever a database written by an older Verkstead
    // still needs done to it. After every table above, because what a rewrite
    // moves rows into is one of them — see [`migrations`], where each rewrite
    // says for itself how it knows whether it has already run.
    migrations::apply(pool).await?;

    Ok(())
}

/// Read a Set back, or `None` if no Set has that id.
///
/// A body this build cannot deserialize comes back as [`Asked::Unreadable`]
/// rather than as a failure: the row is still there and still says what was
/// asked, and losing the read would be losing the record.
pub async fn load_set(pool: &SqlitePool, id: i64) -> Result<Option<StoredSet>> {
    let row: Option<(i64, String, String, Option<i64>)> = sqlx::query_as(
        "SELECT q.id, q.created_at, q.body, d.set_id
         FROM question_sets q
         LEFT JOIN deferrals d ON d.set_id = q.id
         WHERE q.id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .with_context(|| format!("loading Question Set {id}"))?;

    let Some((id, created_at, body, deferral)) = row else {
        return Ok(None);
    };

    Ok(Some(StoredSet {
        id,
        created_at,
        set: Asked::read(body),
        // The row being there is the whole of it: one is written for a Deferred
        // Ask and none for a blocking one.
        deferred: deferral.is_some(),
    }))
}

/// Whether a Set with this id exists, without paying to deserialize it.
pub async fn set_exists(pool: &SqlitePool, id: i64) -> Result<bool> {
    let found: Option<(i64,)> = sqlx::query_as("SELECT 1 FROM question_sets WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .with_context(|| format!("looking for Question Set {id}"))?;

    Ok(found.is_some())
}

/// Store the Response to a Set, stamping it with a submission time.
///
/// `None` means the Set is settled already: it has a Response — a Set is
/// answered once, and the first one stands — or it was locked unanswered,
/// which closed it for good.
///
/// Both are refused inside the one statement rather than checked first, so two
/// devices cannot leave the same Set both answered and locked by racing.
///
/// The Response is expected to have been validated against its Set already.
///
/// The Nothing-else mark is the one thing that does not go into the body: it is
/// taken off here and recorded beside the row, so the Response a waiting agent
/// is handed is byte for byte the one it would have been handed without a mark
/// — see [`endings`]. The two are written in one transaction, because a mark
/// without its Response, or a Response whose mark did not land, would each be a
/// follow-up nobody could say the state of.
pub async fn insert_response(
    pool: &SqlitePool,
    set_id: i64,
    response: &Response,
) -> Result<Option<ResponseAccepted>> {
    let ended = response.nothing_else;
    let body = serde_json::to_string(&Response {
        nothing_else: false,
        ..response.clone()
    })
    .context("serialising the Response")?;

    let mut tx = pool
        .begin()
        .await
        .with_context(|| format!("storing the Response to Question Set {set_id}"))?;

    let row: Option<(String,)> = sqlx::query_as(
        "INSERT INTO responses (set_id, submitted_at, body)
         SELECT ?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), ?
         WHERE NOT EXISTS (SELECT 1 FROM archivings WHERE set_id = ?)
         ON CONFLICT (set_id) DO NOTHING
         RETURNING submitted_at",
    )
    .bind(set_id)
    .bind(body)
    .bind(set_id)
    .fetch_optional(&mut *tx)
    .await
    .with_context(|| format!("storing the Response to Question Set {set_id}"))?;

    // Only where the Response is the one that landed: a second submitter is
    // refused above, and their mark is refused with it.
    if row.is_some() && ended {
        endings::mark(&mut tx, set_id).await?;
    }

    tx.commit()
        .await
        .with_context(|| format!("storing the Response to Question Set {set_id}"))?;

    Ok(row.map(|(submitted_at,)| ResponseAccepted {
        set_id,
        submitted_at,
    }))
}

/// Record that a Set was locked unanswered, stamping it with the time.
///
/// `None` means it was not locked: the Set has a Response, or it had already
/// been locked. Guarded inside the statement for the reason
/// [`insert_response`] is — an answered Set must not also become a locked
/// one.
async fn insert_lock(pool: &SqlitePool, set_id: i64) -> Result<Option<SetLocked>> {
    let row: Option<(String,)> = sqlx::query_as(
        "INSERT INTO archivings (set_id, archived_at)
         SELECT ?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
         WHERE NOT EXISTS (SELECT 1 FROM responses WHERE set_id = ?)
         ON CONFLICT (set_id) DO NOTHING
         RETURNING archived_at",
    )
    .bind(set_id)
    .bind(set_id)
    .fetch_optional(pool)
    .await
    .with_context(|| format!("locking Question Set {set_id}"))?;

    Ok(row.map(|(locked_at,)| SetLocked { set_id, locked_at }))
}

/// When a Set was locked unanswered, or `None` if it was not.
async fn load_lock(pool: &SqlitePool, set_id: i64) -> Result<Option<SetLocked>> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT archived_at FROM archivings WHERE set_id = ?")
            .bind(set_id)
            .fetch_optional(pool)
            .await
            .with_context(|| format!("looking for the lock on Question Set {set_id}"))?;

    Ok(row.map(|(locked_at,)| SetLocked { set_id, locked_at }))
}

/// How a Set was settled, or `None` while it is still waiting on the human.
///
/// The one question a held wait asks, and the one the set view asks: both have
/// to tell an answered Set from one that was closed unanswered, and neither has
/// any use for a third way of finding out.
pub async fn settlement(pool: &SqlitePool, set_id: i64) -> Result<Option<Settlement>> {
    if let Some(stored) = load_response(pool, set_id).await? {
        return Ok(Some(Settlement::Answered(stored)));
    }

    Ok(load_lock(pool, set_id)
        .await?
        .map(Settlement::LockedUnanswered))
}

/// Read a Set's Response back, or `None` if it has not been answered yet.
pub async fn load_response(pool: &SqlitePool, set_id: i64) -> Result<Option<StoredResponse>> {
    let row: Option<(String, String)> =
        sqlx::query_as("SELECT submitted_at, body FROM responses WHERE set_id = ?")
            .bind(set_id)
            .fetch_optional(pool)
            .await
            .with_context(|| format!("loading the Response to Question Set {set_id}"))?;

    let Some((submitted_at, body)) = row else {
        return Ok(None);
    };

    let response = serde_json::from_str(&body)
        .with_context(|| format!("deserialising the stored Response to Question Set {set_id}"))?;

    Ok(Some(StoredResponse {
        set_id,
        submitted_at,
        response,
    }))
}
