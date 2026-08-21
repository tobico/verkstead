//! SQLite persistence for Question Sets, their Responses, and the archiving of
//! the ones nobody will ever answer.
//!
//! A Set and a Response are each kept as one JSON body — JSON rather than YAML
//! because it preserves a Preface's whitespace exactly, and the Preface is
//! markdown the human reads back verbatim. `title`, `project` and `branch` are
//! lifted into columns beside the body, so that what a Set is *about* can be
//! read without deserializing it.
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

use anyhow::{Context, Result};
use sqlx::SqlitePool;
use sqlx::sqlite::SqliteConnectOptions;
use tokio::sync::broadcast;
use verkstead_schema::{QuestionSet, Response, ResponseAccepted, ValidationError};

mod commits;
mod conversations;
mod profiles;
mod push;
mod repos;
mod transcripts;
mod waits;

pub use commits::{Commit, commit, record_commit, recorded_commits};
pub use conversations::{
    Aborting, Chosen, Conversation, ConversationRow, Directed, Directing, Edited, Event, Grilling,
    Implementing, Lifecycle, SetOnTimeline, TimelineEvent, abort_conversation, ask, asked_from,
    choose_direction, conversations, load_conversation, move_to_direction, record_handoff,
    rename_branch, save_brief, set_asked_from, set_base_commit, set_grilling_profile,
    set_implementation_profile, set_state, start_conversation, start_grilling, start_implementing,
    timeline,
};
pub use profiles::{
    AgentType, Deleting, Profile, ProfileFacts, Saving, create_profile, delete_profile,
    load_profile, profiles, update_profile,
};
pub use push::{
    PushSubscription, Subscribing, VapidKeys, forget_subscription, push_subscriptions,
    store_subscription, vapid_keys,
};
pub use repos::{Repo, register_repo, registered_repos};
pub use transcripts::{Summary, append_transcript, start_transcript, transcript};
pub use waits::{WaitHeld, Waits};

/// A Set as the store holds it: the agent's Set plus the identity the server
/// stamped on it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredSet {
    pub id: i64,
    pub created_at: String,
    pub set: QuestionSet,
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

    /// Archived unanswered by the human. Nothing is coming.
    ArchivedUnanswered(SetArchived),
}

/// A Set the human closed unanswered: which Set, and when they closed it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetArchived {
    pub set_id: i64,

    /// When the server took the archiving, RFC 3339.
    pub archived_at: String,
}

/// Word that a Set has just been settled — answered, or archived unanswered —
/// so a wait held on it can end without going back to the store to look.
///
/// It lives beside the store for the same reason the store is its own crate:
/// the agent API's long-poll, the web UI's submit and the web UI's archiving are
/// in different crates, and they have to meet at the same channel or a browser
/// would leave a waiting agent hanging until its hold window closed.
#[derive(Debug, Clone)]
pub struct Settlements(broadcast::Sender<i64>);

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
    pub fn subscribe(&self) -> broadcast::Receiver<i64> {
        self.0.subscribe()
    }

    /// Tell whoever is listening that this Set is settled. A send error means
    /// nobody is, which is the ordinary case — the agent may well be between
    /// polls.
    fn announce(&self, set_id: i64) {
        let _ = self.0.send(set_id);
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
    /// The human did not pick the Option that means go ahead, so the grilling
    /// carries on.
    ///
    /// Nothing is recorded and nothing is wrong: this is how a human disagrees.
    /// The session that proposed is still holding the thread and has their
    /// Response to read, and what it does with it — keep grilling, or propose
    /// again — is the agent's own to decide.
    SentBack,

    /// Accepted, and this is what became of moving the Conversation on.
    ///
    /// [`Directing::NotGrilling`] is not a failure here either: a grilling that
    /// put two proposals has the first acceptance move the Conversation, and the
    /// second finds the move already made.
    Accepted(Directing),
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

    /// The Set was archived unanswered, which closed it for good: an archived
    /// Set cannot also become an answered one.
    Archived,
}

/// Answer a Set: check the Response resolves it, store it, and wake whoever is
/// waiting.
///
/// This is the one path a Response takes, whether it came from the human's
/// browser or from `curl`, so a wait ends the same way either way — and so an
/// archived Set is refused the same way either way.
///
/// It is also where a grilling ends. A Set carrying a `proposal` is the grilling
/// agent's closing move, and answering one moves its Conversation out of Grilling
/// and into Direction — here, rather than in either endpoint, for the reason
/// everything else about answering is here: the browser and `curl` must not be
/// able to leave a Conversation in different states for the same Answer.
pub async fn submit_response(
    pool: &SqlitePool,
    settlements: &Settlements,
    set_id: i64,
    response: &Response,
) -> Result<Submission> {
    let Some(stored) = load_set(pool, set_id).await? else {
        return Ok(Submission::NoSuchSet);
    };

    if let Err(invalid) = response.validate(&stored.set) {
        return Ok(Submission::Invalid(invalid));
    }

    let Some(accepted) = insert_response(pool, set_id, response).await? else {
        // The insert refuses both ways a Set can already be settled, so which of
        // the two it was is read back rather than assumed.
        return Ok(match load_archiving(pool, set_id).await? {
            Some(_) => Submission::Archived,
            None => Submission::AlreadyAnswered,
        });
    };

    // After the Response is stored, and only for the Set that carries a
    // proposal. The insert is what makes a Set answered once, so a proposal is
    // settled once too: a second Response to the same Set never gets this far.
    let proposed = match &stored.set.proposal {
        Some(proposal) if proposal.accepted(response) => {
            Some(Proposed::Accepted(accept_proposal(pool, set_id).await?))
        }
        // Answered some other way, which is how a human disagrees: the grilling
        // carries on, and the agent has their Response.
        Some(_) => Some(Proposed::SentBack),
        None => None,
    };

    settlements.announce(set_id);

    Ok(Submission::Accepted(Taken { accepted, proposed }))
}

/// Move the Conversation an accepted proposal was asked from on to choosing a
/// direction.
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
async fn accept_proposal(pool: &SqlitePool, set_id: i64) -> Result<Directing> {
    let Some(conversation_id) = asked_from(pool, set_id).await? else {
        return Ok(Directing::NoSuchConversation);
    };

    move_to_direction(pool, conversation_id).await
}

/// What became of archiving a Set unanswered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Archiving {
    /// Closed: the Set has stopped waiting on the human, and anyone holding a
    /// wait on it has been told.
    Archived(SetArchived),

    /// There is no Set with that id to archive.
    NoSuchSet,

    /// The Set has a Response, so it is a decision already. Archiving
    /// unanswered is for a Set nobody will ever answer, and it does not touch
    /// what was decided.
    AlreadyAnswered,

    /// The Set was archived before this arrived — from another device, or
    /// another tab.
    AlreadyArchived,
}

/// Close a Set the human is never going to be able to answer: settle it
/// unanswered, and tell whoever is waiting on it.
///
/// Only a human may do this (ADR-0001): a disconnected agent is never enough,
/// because the CLI reconnects through transient drops. Nothing here consults
/// Liveness — the decision is the human's, taken with the badge in front of
/// them.
pub async fn archive_set(
    pool: &SqlitePool,
    settlements: &Settlements,
    set_id: i64,
) -> Result<Archiving> {
    if !set_exists(pool, set_id).await? {
        return Ok(Archiving::NoSuchSet);
    }

    let Some(archived) = insert_archiving(pool, set_id).await? else {
        return Ok(match load_response(pool, set_id).await? {
            Some(_) => Archiving::AlreadyAnswered,
            None => Archiving::AlreadyArchived,
        });
    };

    settlements.announce(set_id);

    Ok(Archiving::Archived(archived))
}

/// Open the SQLite database at `path`, creating the file if it is absent and
/// bringing its schema up to date.
pub async fn open_database(path: &Path) -> Result<SqlitePool> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating database directory {}", parent.display()))?;
    }

    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true);

    let pool = SqlitePool::connect_with(options)
        .await
        .with_context(|| format!("opening database {}", path.display()))?;

    apply_schema(&pool).await?;

    Ok(pool)
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

    // Archiving hangs off a Set exactly as its Response does, rather than being
    // a column on it: there is no migration machinery here, and
    // `question_sets` is STRICT and left alone. One archiving per Set, by the
    // same primary key, for the same reason.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS archivings (
             set_id      INTEGER PRIMARY KEY REFERENCES question_sets(id),
             archived_at TEXT NOT NULL
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the archivings table")?;

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

    // What the sessions run against them printed. After the Timelines, because a
    // transcript hangs off the Event it is the full self of.
    transcripts::apply_schema(pool).await?;

    // And what they committed, which hangs off the Timelines for the same
    // reason — and off the Conversations too, which is what makes one commit
    // per Conversation a rule the database keeps.
    commits::apply_schema(pool).await?;

    Ok(())
}

/// Read a Set back, or `None` if no Set has that id.
pub async fn load_set(pool: &SqlitePool, id: i64) -> Result<Option<StoredSet>> {
    let row: Option<(i64, String, String)> =
        sqlx::query_as("SELECT id, created_at, body FROM question_sets WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
            .with_context(|| format!("loading Question Set {id}"))?;

    let Some((id, created_at, body)) = row else {
        return Ok(None);
    };

    let set = serde_json::from_str(&body)
        .with_context(|| format!("deserialising stored Question Set {id}"))?;

    Ok(Some(StoredSet {
        id,
        created_at,
        set,
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
/// answered once, and the first one stands — or it was archived unanswered,
/// which closed it for good.
///
/// Both are refused inside the one statement rather than checked first, so two
/// devices cannot leave the same Set both answered and archived by racing.
///
/// The Response is expected to have been validated against its Set already.
pub async fn insert_response(
    pool: &SqlitePool,
    set_id: i64,
    response: &Response,
) -> Result<Option<ResponseAccepted>> {
    let body = serde_json::to_string(response).context("serialising the Response")?;

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
    .fetch_optional(pool)
    .await
    .with_context(|| format!("storing the Response to Question Set {set_id}"))?;

    Ok(row.map(|(submitted_at,)| ResponseAccepted {
        set_id,
        submitted_at,
    }))
}

/// Record that a Set was archived unanswered, stamping it with the time.
///
/// `None` means it was not archived: the Set has a Response, or it had already
/// been archived. Guarded inside the statement for the reason
/// [`insert_response`] is — an answered Set must not also become an archived
/// one.
async fn insert_archiving(pool: &SqlitePool, set_id: i64) -> Result<Option<SetArchived>> {
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
    .with_context(|| format!("archiving Question Set {set_id}"))?;

    Ok(row.map(|(archived_at,)| SetArchived {
        set_id,
        archived_at,
    }))
}

/// When a Set was archived unanswered, or `None` if it was not.
async fn load_archiving(pool: &SqlitePool, set_id: i64) -> Result<Option<SetArchived>> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT archived_at FROM archivings WHERE set_id = ?")
            .bind(set_id)
            .fetch_optional(pool)
            .await
            .with_context(|| format!("looking for the archiving of Question Set {set_id}"))?;

    Ok(row.map(|(archived_at,)| SetArchived {
        set_id,
        archived_at,
    }))
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

    Ok(load_archiving(pool, set_id)
        .await?
        .map(Settlement::ArchivedUnanswered))
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
