//! The Conversations a grilling session will later be run against: a Repo, a
//! branch, a base commit, and the Timeline everything about the work lands on.
//!
//! Nothing here executes anything. A Conversation is a record — no branch is
//! created, no worktree, no session — and it stays in [`Lifecycle::Draft`] until
//! the stage that starts grilling moves it on. What the record is for is that
//! the human can write the brief and settle the branch name before any of that
//! happens.
//!
//! The Timeline is its own table from the start rather than a Brief column on
//! the Conversation. The Brief is the first Event and, for now, the only kind of
//! Event there is; agent output, Question Sets and commits are the same list
//! with more in it. A Brief kept beside the Timeline rather than in it would
//! have to be moved into it later, and a reopened round adds a second Brief
//! Event rather than editing the first — which a column could not hold at all.

use anyhow::{Context, Result, anyhow, bail};
use sqlx::SqlitePool;

/// Where a Conversation has got to.
///
/// The whole ladder is here though only [`Lifecycle::Draft`] is ever written
/// yet: it is the domain's, not this stage's invention, and the rules written
/// against it — a Brief and a branch name are the human's to change only while
/// the Conversation is still drafting — need the states they are refusing on
/// behalf of to exist before the stage that reaches them does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lifecycle {
    /// The brief is being written. Everything about the Conversation is still
    /// the human's to change.
    Draft,

    /// A grilling session is running against it.
    Grilling,

    /// The grilling is over and how to implement the work is being chosen.
    Direction,

    /// The work is being done.
    Implementing,

    /// The work is on a PR and the wrap-up loop has it.
    Wrapping,

    /// Finished. It can be reopened with a new round.
    Done,
}

impl Lifecycle {
    /// The word the column holds. Lowercase and spelled out, so the table reads
    /// as something rather than as a number nobody can look up.
    fn stored(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Grilling => "grilling",
            Self::Direction => "direction",
            Self::Implementing => "implementing",
            Self::Wrapping => "wrapping",
            Self::Done => "done",
        }
    }

    /// The state a stored word names. A word this does not know is a database
    /// written by a Verkstead this one does not understand, which is worth
    /// saying rather than guessing past.
    fn read(word: &str) -> Result<Self> {
        Ok(match word {
            "draft" => Self::Draft,
            "grilling" => Self::Grilling,
            "direction" => Self::Direction,
            "implementing" => Self::Implementing,
            "wrapping" => Self::Wrapping,
            "done" => Self::Done,
            other => bail!("a Conversation is in the unknown state {other:?}"),
        })
    }
}

/// A Conversation as the store holds it, with the Repo it is attached to read
/// back beside it — there is no Conversation without one, and everything done
/// about a Conversation is done inside that repository.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Conversation {
    pub id: i64,
    pub created_at: String,
    pub repo: super::Repo,

    /// The branch the work will be done on. Prefilled with a random name at
    /// creation and the human's to change while the Conversation is drafting.
    pub branch: String,

    /// The commit to branch from, where the human named one. `None` is not a
    /// missing value: it is the rule that the default branch's tip at grill
    /// start is what gets used, which is a thing to resolve then rather than a
    /// commit to record now.
    pub base_commit: Option<String>,

    pub state: Lifecycle,
}

/// One row of the conversations sidebar, drawn without reading a Timeline.
///
/// The branch is the row's name. A Conversation has no title of its own — the
/// domain gives it a Repo, a Brief, a branch and a base commit and nothing else
/// — and of those the branch is the one short line the human chose, which is
/// what a list is read by.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversationRow {
    pub id: i64,
    pub branch: String,

    /// What the Repo is called, which is the only thing about it a row shows.
    pub repo: String,

    pub state: Lifecycle,
}

/// One entry in a Conversation's Timeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineEvent {
    pub id: i64,

    /// When it landed, RFC 3339.
    pub at: String,

    pub event: Event,
}

/// What an Event is. One kind so far — the rest of the table in the design
/// arrives with the stages that produce them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// The Brief: the markdown the Conversation starts from, as the human last
    /// wrote it.
    Brief(String),
}

impl Event {
    /// The word the `kind` column holds. `'static`, so the one statement that
    /// wants the word without an Event to hand can ask for it and let the Event
    /// go.
    fn kind(&self) -> &'static str {
        match self {
            Self::Brief(_) => "brief",
        }
    }

    /// What goes in the `body` column beside the kind.
    fn body(&self) -> &str {
        match self {
            Self::Brief(markdown) => markdown,
        }
    }

    fn read(kind: &str, body: String) -> Result<Self> {
        Ok(match kind {
            "brief" => Self::Brief(body),
            other => bail!("a Timeline holds an Event of the unknown kind {other:?}"),
        })
    }
}

/// What became of an edit to a drafting Conversation.
///
/// One outcome type for the three of them — the Brief, the branch name and the
/// base commit — because they are refused for the same two reasons, and a
/// caller telling them apart would be telling apart the same sentence three
/// times.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edited {
    /// Recorded.
    Saved,

    /// There is no Conversation with that id.
    NoSuchConversation,

    /// It is past drafting, so this is not the human's to change any more.
    NotDrafting,
}

/// The tables a Conversation and its Timeline live in.
///
/// The Timeline is indexed by the Conversation it belongs to, because that is
/// the only way it is ever read: a Timeline is one Conversation's, whole and in
/// order.
pub(crate) async fn apply_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS conversations (
             id          INTEGER PRIMARY KEY AUTOINCREMENT,
             repo_id     INTEGER NOT NULL REFERENCES repos(id),
             created_at  TEXT NOT NULL,
             branch      TEXT NOT NULL,
             base_commit TEXT,
             state       TEXT NOT NULL
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the conversations table")?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS timeline_events (
             id              INTEGER PRIMARY KEY AUTOINCREMENT,
             conversation_id INTEGER NOT NULL REFERENCES conversations(id),
             at              TEXT NOT NULL,
             kind            TEXT NOT NULL,
             body            TEXT NOT NULL
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the timeline_events table")?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS timeline_events_conversation
             ON timeline_events (conversation_id, id)",
    )
    .execute(pool)
    .await
    .context("indexing the Timeline by its Conversation")?;

    Ok(())
}

/// Start a Conversation against a registered Repo, on `branch`, with an empty
/// Brief already in its Timeline.
///
/// `None` means there is no such Repo. The insert selects from `repos` rather
/// than trusting the id, so a Conversation cannot come to hang off a repository
/// that was never registered — SQLite does not enforce a foreign key unless it
/// is asked to, and a row that named nothing would be a Conversation with
/// nowhere to work.
///
/// The Brief goes in with it, in the same transaction: the Brief is the first
/// Event, and a Conversation whose Timeline was empty because the second insert
/// failed would be one the human could not write anything into.
pub async fn start_conversation(
    pool: &SqlitePool,
    repo_id: i64,
    branch: &str,
) -> Result<Option<i64>> {
    let mut tx = pool.begin().await.context("starting a Conversation")?;

    let row: Option<(i64,)> = sqlx::query_as(
        "INSERT INTO conversations (repo_id, created_at, branch, base_commit, state)
         SELECT id, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), ?, NULL, ?
         FROM repos WHERE id = ?
         RETURNING id",
    )
    .bind(branch)
    .bind(Lifecycle::Draft.stored())
    .bind(repo_id)
    .fetch_optional(&mut *tx)
    .await
    .with_context(|| format!("starting a Conversation on Repo {repo_id}"))?;

    let Some((id,)) = row else {
        return Ok(None);
    };

    // Empty, because nothing has been written yet. It is an Event all the same:
    // the Brief is the first thing on the Timeline whether or not it says
    // anything, and the Timeline is where the human writes it.
    let brief = Event::Brief(String::new());
    sqlx::query(
        "INSERT INTO timeline_events (conversation_id, at, kind, body)
         VALUES (?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), ?, ?)",
    )
    .bind(id)
    .bind(brief.kind())
    .bind(brief.body())
    .execute(&mut *tx)
    .await
    .with_context(|| format!("writing the Brief of Conversation {id}"))?;

    tx.commit().await.context("starting a Conversation")?;

    Ok(Some(id))
}

/// Every Conversation, newest first.
///
/// Newest first like the Set lists, and for the same reason: what was started
/// last is what is being worked on. The design gives the sidebar a manual order
/// eventually; until there is one, the order a Conversation was started in is
/// the one order that means anything.
pub async fn conversations(pool: &SqlitePool) -> Result<Vec<ConversationRow>> {
    let rows: Vec<(i64, String, String, String)> = sqlx::query_as(
        "SELECT c.id, c.branch, r.name, c.state
         FROM conversations c
         JOIN repos r ON r.id = c.repo_id
         ORDER BY c.id DESC",
    )
    .fetch_all(pool)
    .await
    .context("listing the Conversations")?;

    rows.into_iter()
        .map(|(id, branch, repo, state)| {
            Ok(ConversationRow {
                id,
                branch,
                repo,
                state: Lifecycle::read(&state)?,
            })
        })
        .collect()
}

/// One Conversation with its Repo, or `None` if there is no such Conversation.
pub async fn load_conversation(pool: &SqlitePool, id: i64) -> Result<Option<Conversation>> {
    /// The columns in the order the query below selects them.
    type Row = (
        i64,
        String,
        String,
        Option<String>,
        String,
        i64,
        String,
        String,
        String,
    );

    let row: Option<Row> = sqlx::query_as(
        "SELECT c.id, c.created_at, c.branch, c.base_commit, c.state,
                r.id, r.path, r.name, r.default_branch
         FROM conversations c
         JOIN repos r ON r.id = c.repo_id
         WHERE c.id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .with_context(|| format!("loading Conversation {id}"))?;

    let Some((
        id,
        created_at,
        branch,
        base_commit,
        state,
        repo_id,
        repo_path,
        repo_name,
        default_branch,
    )) = row
    else {
        return Ok(None);
    };

    Ok(Some(Conversation {
        id,
        created_at,
        repo: super::Repo {
            id: repo_id,
            path: std::path::PathBuf::from(repo_path),
            name: repo_name,
            default_branch,
        },
        branch,
        base_commit: base_commit.filter(|commit| !commit.is_empty()),
        state: Lifecycle::read(&state)?,
    }))
}

/// A Conversation's Timeline, oldest first — which is reading order, and which
/// puts the Brief at the top where it was written.
///
/// Ordered by id rather than by `at`: the id is handed out in the order things
/// happened, and two Events stamped in the same millisecond must not come back
/// in an arbitrary one.
pub async fn timeline(pool: &SqlitePool, conversation_id: i64) -> Result<Vec<TimelineEvent>> {
    let rows: Vec<(i64, String, String, String)> = sqlx::query_as(
        "SELECT id, at, kind, body
         FROM timeline_events
         WHERE conversation_id = ?
         ORDER BY id",
    )
    .bind(conversation_id)
    .fetch_all(pool)
    .await
    .with_context(|| format!("reading the Timeline of Conversation {conversation_id}"))?;

    rows.into_iter()
        .map(|(id, at, kind, body)| {
            Ok(TimelineEvent {
                id,
                at,
                event: Event::read(&kind, body)?,
            })
        })
        .collect()
}

/// Rewrite a drafting Conversation's Brief.
///
/// The Brief Event is edited in place rather than added to: while a Conversation
/// is drafting there is one Brief and this is it. The frozen-Brief rule the
/// design states — a reopened round adds a new Brief rather than editing the old
/// one — is the drafting guard here, keeping its half of the bargain from the
/// start.
pub async fn save_brief(pool: &SqlitePool, id: i64, markdown: &str) -> Result<Edited> {
    if let Some(refusal) = not_drafting(pool, id).await? {
        return Ok(refusal);
    }

    sqlx::query(
        "UPDATE timeline_events SET body = ?
         WHERE conversation_id = ? AND kind = ?",
    )
    .bind(markdown)
    .bind(id)
    .bind(Event::Brief(String::new()).kind())
    .execute(pool)
    .await
    .with_context(|| format!("saving the Brief of Conversation {id}"))?;

    Ok(Edited::Saved)
}

/// Name the branch a drafting Conversation's work will be done on.
///
/// Whether the name is one git would take is decided above the store, where git
/// itself is asked — this records what it is given.
pub async fn rename_branch(pool: &SqlitePool, id: i64, branch: &str) -> Result<Edited> {
    if let Some(refusal) = not_drafting(pool, id).await? {
        return Ok(refusal);
    }

    sqlx::query("UPDATE conversations SET branch = ? WHERE id = ?")
        .bind(branch)
        .bind(id)
        .execute(pool)
        .await
        .with_context(|| format!("renaming the branch of Conversation {id}"))?;

    Ok(Edited::Saved)
}

/// Record the commit a drafting Conversation branches from, or `None` to put it
/// back on the default-branch rule.
///
/// `None` is the ordinary case and not a cleared field: the design says the base
/// commit is the default branch's tip *at grill start*, so while drafting there
/// is no value to hold — only whether the human has overridden the rule.
pub async fn set_base_commit(pool: &SqlitePool, id: i64, commit: Option<&str>) -> Result<Edited> {
    if let Some(refusal) = not_drafting(pool, id).await? {
        return Ok(refusal);
    }

    sqlx::query("UPDATE conversations SET base_commit = ? WHERE id = ?")
        .bind(commit)
        .bind(id)
        .execute(pool)
        .await
        .with_context(|| format!("recording the base commit of Conversation {id}"))?;

    Ok(Edited::Saved)
}

/// The reason this Conversation is not the human's to edit, or `None` where it
/// is.
///
/// Read before the write rather than guarded inside it, unlike the Set tables:
/// there is one human at the workbench, and what would be raced for here is
/// their own two tabs editing one Brief. What matters is that a Conversation
/// past drafting refuses, and that a Conversation that is not there says so.
async fn not_drafting(pool: &SqlitePool, id: i64) -> Result<Option<Edited>> {
    let row: Option<(String,)> = sqlx::query_as("SELECT state FROM conversations WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .with_context(|| format!("reading the state of Conversation {id}"))?;

    let Some((state,)) = row else {
        return Ok(Some(Edited::NoSuchConversation));
    };

    Ok(match Lifecycle::read(&state)? {
        Lifecycle::Draft => None,
        _ => Some(Edited::NotDrafting),
    })
}

/// Move a Conversation on to another state.
///
/// Nothing in this stage calls it — no Conversation leaves [`Lifecycle::Draft`]
/// until grilling can start — but the states are what the drafting guard refuses
/// on behalf of, and a guard nothing can reach is a guard nothing can test.
pub async fn set_state(pool: &SqlitePool, id: i64, state: Lifecycle) -> Result<()> {
    let changed = sqlx::query("UPDATE conversations SET state = ? WHERE id = ?")
        .bind(state.stored())
        .bind(id)
        .execute(pool)
        .await
        .with_context(|| format!("moving Conversation {id} to {}", state.stored()))?
        .rows_affected();

    if changed == 0 {
        return Err(anyhow!("there is no Conversation {id} to move"));
    }

    Ok(())
}
