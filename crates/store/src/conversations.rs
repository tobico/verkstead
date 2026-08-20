//! The Conversations a grilling session is run against: a Repo, a branch, a base
//! commit, and the Timeline everything about the work lands on.
//!
//! Nothing here executes anything. The branch and the worktree are made by the
//! server, against git and the filesystem; what this records is that they were —
//! which commit was branched from, where the worktree was put, and that the
//! Conversation has moved. A store that shelled out to git would be a store with
//! a second way to fail.
//!
//! The Timeline is its own table from the start rather than a Brief column on
//! the Conversation. The Brief is the first Event and, for now, the only kind of
//! Event there is; agent output, Question Sets and commits are the same list
//! with more in it. A Brief kept beside the Timeline rather than in it would
//! have to be moved into it later, and a reopened round adds a second Brief
//! Event rather than editing the first — which a column could not hold at all.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use sqlx::SqlitePool;

/// Where a Conversation has got to.
///
/// The ladder is the domain's rather than any one stage's invention, so the
/// states beyond the two this one reaches are here too: the rules written
/// against them — a Brief and a branch name are the human's to change only while
/// the Conversation is still drafting — need the states they refuse on behalf of
/// to exist before the stage that reaches them does.
///
/// [`Lifecycle::Aborted`] is off the ladder rather than on it. Every other state
/// is somewhere the work has got to, and aborting is the work stopping wherever
/// it was — which is why it is reachable from all of them and leads nowhere.
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

    /// Stopped, from wherever it had got to. The worktree is gone; the branch is
    /// not, because a branch is cheap and may hold work worth reading.
    Aborted,
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
            Self::Aborted => "aborted",
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
            "aborted" => Self::Aborted,
            other => bail!("a Conversation is in the unknown state {other:?}"),
        })
    }
}

/// A Conversation as the store holds it, with the Repo it is attached to read
/// back beside it — there is no Conversation without one, and everything done
/// about a Conversation is done inside that repository.
///
/// The two Profiles are read back the same way, because whether a Conversation
/// is ready to grill turns on what they are rather than on which ids they hold:
/// a Profile whose pair has gone is not something to launch a session under, and
/// the id alone cannot say so.
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

    /// The Agent Profile the grilling session runs under, once one is chosen.
    pub grilling_profile: Option<super::Profile>,

    /// And the one the implementation runs under. A separate choice because it
    /// is genuinely a separate account and model — and because the
    /// implementation session cannot simply carry the grilling one on.
    pub implementation_profile: Option<super::Profile>,

    /// Where the Conversation's worktree was put, once grilling has made one.
    ///
    /// `None` before grilling starts and again after aborting — the two ways a
    /// Conversation has no worktree, which are the same fact about it whatever
    /// put it there. Whether the directory is still on disk is not something the
    /// store can say; see [`abort_conversation`] for who does.
    pub worktree: Option<PathBuf>,
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

/// What an Event is. Three kinds so far — the rest of the table in the design
/// arrives with the stages that produce them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// The Brief: the markdown the Conversation starts from, as the human last
    /// wrote it.
    Brief(String),

    /// The Conversation moved, and this is the state it moved to.
    ///
    /// One kind for every move rather than one per destination: what the
    /// Timeline is recording is that the work changed hands, and the state it
    /// changed to is the only thing that differs between one move and the next.
    /// Starting to grill and aborting both land here.
    Moved(Lifecycle),

    /// A session's output, summarised. The whole of it is the transcript beside
    /// it — see [`super::transcripts`] — which is what the details pane shows
    /// and what this is a line of.
    ///
    /// The only Event whose body is not in the `body` column: a transcript is
    /// written a chunk at a time for as long as a session runs, and a column
    /// that was rewritten whole on every chunk would cost more the longer the
    /// session went on.
    AgentOutput(super::Summary),
}

impl Event {
    /// The word the `kind` column holds. `'static`, so the one statement that
    /// wants the word without an Event to hand can ask for it and let the Event
    /// go.
    pub(crate) fn kind(&self) -> &'static str {
        match self {
            Self::Brief(_) => "brief",
            Self::Moved(_) => "moved",
            Self::AgentOutput(_) => "agent-output",
        }
    }

    /// What goes in the `body` column beside the kind.
    fn body(&self) -> &str {
        match self {
            Self::Brief(markdown) => markdown,
            Self::Moved(state) => state.stored(),
            // Nothing: what a session printed is in the transcript tables, and
            // what the Timeline shows of it is read back from there too.
            Self::AgentOutput(_) => "",
        }
    }

    /// The Event a row holds, with the summary read alongside it where the row
    /// is an agent-output one.
    ///
    /// A transcript's summary row is written in the same transaction as its
    /// Event, so one without the other is a database somebody has been in by
    /// hand — worth saying rather than reading as a session that printed
    /// nothing.
    fn read(kind: &str, body: String, summary: Option<super::Summary>) -> Result<Self> {
        Ok(match kind {
            "brief" => Self::Brief(body),
            "moved" => Self::Moved(Lifecycle::read(&body)?),
            "agent-output" => Self::AgentOutput(
                summary.ok_or_else(|| anyhow!("a session's output has no transcript beside it"))?,
            ),
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

/// What became of choosing one of a Conversation's two Agent Profiles.
///
/// No drafting refusal among them, unlike the Brief and the branch name: a
/// Profile is a setting rather than a document something has been built from,
/// and the implementation one is used after the grilling is over — freezing it
/// when grilling starts would take it away exactly when the human has just
/// learned what they want it to be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Chosen {
    /// Recorded.
    Chosen,

    /// There is no Conversation with that id.
    NoSuchConversation,

    /// There is no Profile with that id to choose.
    NoSuchProfile,
}

/// What became of starting a Conversation grilling.
///
/// Only the two refusals the store is in a position to make. Everything else
/// starting is refused for — an unchosen Profile, an empty Brief, a base commit
/// nothing answers to — is decided above it, against the Profiles and against
/// git, and is settled by the time this is called.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grilling {
    /// Recorded: the base commit, the worktree, the state and the Event.
    Started,

    /// There is no Conversation with that id.
    NoSuchConversation,

    /// It is past drafting, so it has been started once already — or aborted.
    NotDrafting,
}

/// What became of aborting one.
///
/// Aborting twice is not an error, which is what [`Aborting::AlreadyAborted`] is
/// for: it is a distinct outcome rather than a failure, because the thing the
/// human asked for holds either way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Aborting {
    /// Stopped: the worktree is forgotten, the state is [`Lifecycle::Aborted`],
    /// and the move is on the Timeline.
    Aborted,

    /// It was aborted already. Nothing to record and nothing wrong.
    AlreadyAborted,

    /// There is no Conversation with that id.
    NoSuchConversation,
}

/// The tables a Conversation and its Timeline live in.
///
/// The Timeline is indexed by the Conversation it belongs to, because that is
/// the only way it is ever read: a Timeline is one Conversation's, whole and in
/// order.
pub(crate) async fn apply_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS conversations (
             id                        INTEGER PRIMARY KEY AUTOINCREMENT,
             repo_id                   INTEGER NOT NULL REFERENCES repos(id),
             created_at                TEXT NOT NULL,
             branch                    TEXT NOT NULL,
             base_commit               TEXT,
             state                     TEXT NOT NULL,
             grilling_profile_id       INTEGER REFERENCES profiles(id),
             implementation_profile_id INTEGER REFERENCES profiles(id)
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

    // The worktree hangs off a Conversation rather than being a column on it,
    // as an archiving hangs off a Set: there is no migration machinery here and
    // `conversations` is STRICT and left alone. One worktree per Conversation,
    // by the primary key — and a Conversation that has none has no row, which is
    // both the state before grilling and the state after aborting.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS worktrees (
             conversation_id INTEGER PRIMARY KEY REFERENCES conversations(id),
             path            TEXT NOT NULL
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the worktrees table")?;

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

/// One Conversation with its Repo and whichever Profiles it has chosen, or
/// `None` if there is no such Conversation.
///
/// The Profiles are fetched beside the row rather than joined into it: they are
/// each optional, they are read back whole, and two more `LEFT JOIN`s' worth of
/// columns to unpack would say nothing the two small reads do not.
pub async fn load_conversation(pool: &SqlitePool, id: i64) -> Result<Option<Conversation>> {
    /// The columns in the order the query below selects them.
    type Row = (
        i64,
        String,
        String,
        Option<String>,
        String,
        Option<i64>,
        Option<i64>,
        i64,
        String,
        String,
        String,
    );

    let row: Option<Row> = sqlx::query_as(
        "SELECT c.id, c.created_at, c.branch, c.base_commit, c.state,
                c.grilling_profile_id, c.implementation_profile_id,
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
        grilling_profile_id,
        implementation_profile_id,
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
        grilling_profile: chosen_profile(pool, grilling_profile_id).await?,
        implementation_profile: chosen_profile(pool, implementation_profile_id).await?,
        worktree: worktree(pool, id).await?,
    }))
}

/// The Profile an id names, where there is an id at all.
async fn chosen_profile(pool: &SqlitePool, id: Option<i64>) -> Result<Option<super::Profile>> {
    match id {
        None => Ok(None),
        Some(id) => super::load_profile(pool, id).await,
    }
}

/// Where a Conversation's worktree was put, if it has one.
async fn worktree(pool: &SqlitePool, id: i64) -> Result<Option<std::path::PathBuf>> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT path FROM worktrees WHERE conversation_id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
            .with_context(|| format!("reading the worktree of Conversation {id}"))?;

    Ok(row.map(|(path,)| std::path::PathBuf::from(path)))
}

/// Choose the Agent Profile the grilling session will run under.
pub async fn set_grilling_profile(pool: &SqlitePool, id: i64, profile_id: i64) -> Result<Chosen> {
    choose(pool, id, profile_id, "grilling_profile_id").await
}

/// Choose the Agent Profile the implementation will run under.
pub async fn set_implementation_profile(
    pool: &SqlitePool,
    id: i64,
    profile_id: i64,
) -> Result<Chosen> {
    choose(pool, id, profile_id, "implementation_profile_id").await
}

/// Record one of the two choices.
///
/// The Profile is selected from `profiles` inside the statement rather than
/// checked first, as a Conversation's Repo is: SQLite enforces a foreign key
/// only when asked to, and a column naming a Profile that is not there is a
/// session that fails to start with nobody watching.
///
/// `column` is one of two literals this module passes, never anything a request
/// reached.
async fn choose(pool: &SqlitePool, id: i64, profile_id: i64, column: &str) -> Result<Chosen> {
    if !conversation_exists(pool, id).await? {
        return Ok(Chosen::NoSuchConversation);
    }

    let changed = sqlx::query(&format!(
        "UPDATE conversations
         SET {column} = (SELECT id FROM profiles WHERE id = ?)
         WHERE id = ? AND EXISTS (SELECT 1 FROM profiles WHERE id = ?)"
    ))
    .bind(profile_id)
    .bind(id)
    .bind(profile_id)
    .execute(pool)
    .await
    .with_context(|| format!("choosing Profile {profile_id} for Conversation {id}"))?
    .rows_affected();

    Ok(match changed {
        0 => Chosen::NoSuchProfile,
        _ => Chosen::Chosen,
    })
}

async fn conversation_exists(pool: &SqlitePool, id: i64) -> Result<bool> {
    let found: Option<(i64,)> = sqlx::query_as("SELECT 1 FROM conversations WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .with_context(|| format!("looking for Conversation {id}"))?;

    Ok(found.is_some())
}

/// A Conversation's Timeline, oldest first — which is reading order, and which
/// puts the Brief at the top where it was written.
///
/// Ordered by id rather than by `at`: the id is handed out in the order things
/// happened, and two Events stamped in the same millisecond must not come back
/// in an arbitrary one.
///
/// A transcript's summary is joined in rather than fetched per Event, and no
/// transcript itself is: a Timeline is read every time an open page looks again,
/// and what a session printed is megabytes the middle pane never shows.
pub async fn timeline(pool: &SqlitePool, conversation_id: i64) -> Result<Vec<TimelineEvent>> {
    /// The columns in the order the query below selects them: the Event, and
    /// the transcript summary that is there for one kind of Event and no other.
    type Row = (i64, String, String, String, Option<i64>, Option<String>);

    let rows: Vec<Row> = sqlx::query_as(
        "SELECT e.id, e.at, e.kind, e.body, t.lines, t.latest
         FROM timeline_events e
         LEFT JOIN transcripts t ON t.event_id = e.id
         WHERE e.conversation_id = ?
         ORDER BY e.id",
    )
    .bind(conversation_id)
    .fetch_all(pool)
    .await
    .with_context(|| format!("reading the Timeline of Conversation {conversation_id}"))?;

    rows.into_iter()
        .map(|(id, at, kind, body, lines, latest)| {
            let summary = lines
                .zip(latest)
                .map(|(lines, latest)| super::Summary { lines, latest });

            Ok(TimelineEvent {
                id,
                at,
                event: Event::read(&kind, body, summary)?,
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

/// Record that a Conversation has started grilling: what it branched from, where
/// its worktree is, and that it has moved.
///
/// The branch and the worktree are already made by the time this is called —
/// the server does that, against git — so what is written here is the record of
/// work that has happened, not an instruction to do any. Which is also why it is
/// one transaction: a Conversation left saying `draft` with a worktree on disk
/// would be one nothing could start again and nothing would clean up.
///
/// `base_commit` is written whether or not the human overrode one. Where they
/// did not, the rule was the default branch's tip *at grill start* — so this is
/// the moment that rule resolves to a commit, and after it there is a fact about
/// what the work branched from rather than a rule about what it would have.
pub async fn start_grilling(
    pool: &SqlitePool,
    id: i64,
    base_commit: &str,
    worktree: &Path,
) -> Result<Grilling> {
    let worktree = super::repos::text(worktree)?;

    let mut tx = pool.begin().await.context("starting a grilling")?;

    let row: Option<(String,)> = sqlx::query_as("SELECT state FROM conversations WHERE id = ?")
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .with_context(|| format!("reading the state of Conversation {id}"))?;

    let Some((state,)) = row else {
        return Ok(Grilling::NoSuchConversation);
    };

    if Lifecycle::read(&state)? != Lifecycle::Draft {
        return Ok(Grilling::NotDrafting);
    }

    sqlx::query(
        "UPDATE conversations
         SET base_commit = ?, state = ?
         WHERE id = ?",
    )
    .bind(base_commit)
    .bind(Lifecycle::Grilling.stored())
    .bind(id)
    .execute(&mut *tx)
    .await
    .with_context(|| format!("moving Conversation {id} to grilling"))?;

    sqlx::query("INSERT INTO worktrees (conversation_id, path) VALUES (?, ?)")
        .bind(id)
        .bind(worktree)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("recording the worktree of Conversation {id}"))?;

    moved(&mut tx, id, Lifecycle::Grilling).await?;

    tx.commit().await.context("starting a grilling")?;

    Ok(Grilling::Started)
}

/// Record that a Conversation has been aborted: its worktree is gone, and it has
/// stopped wherever it had got to.
///
/// The worktree is forgotten rather than remembered as removed, because there is
/// nothing left to point at — the branch it was checked out on is still there,
/// and that is the thing worth keeping. The directory itself is removed by the
/// server before this is called, for the reason the branch is created before
/// [`start_grilling`] is: the record follows the work rather than promising it.
///
/// Aborting one that is already aborted records nothing and is not an error. The
/// human asked for it to be stopped, and it is.
pub async fn abort_conversation(pool: &SqlitePool, id: i64) -> Result<Aborting> {
    let mut tx = pool.begin().await.context("aborting a Conversation")?;

    let row: Option<(String,)> = sqlx::query_as("SELECT state FROM conversations WHERE id = ?")
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .with_context(|| format!("reading the state of Conversation {id}"))?;

    let Some((state,)) = row else {
        return Ok(Aborting::NoSuchConversation);
    };

    if Lifecycle::read(&state)? == Lifecycle::Aborted {
        return Ok(Aborting::AlreadyAborted);
    }

    sqlx::query("UPDATE conversations SET state = ? WHERE id = ?")
        .bind(Lifecycle::Aborted.stored())
        .bind(id)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("aborting Conversation {id}"))?;

    sqlx::query("DELETE FROM worktrees WHERE conversation_id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("forgetting the worktree of Conversation {id}"))?;

    moved(&mut tx, id, Lifecycle::Aborted).await?;

    tx.commit().await.context("aborting a Conversation")?;

    Ok(Aborting::Aborted)
}

/// Put a move on a Conversation's Timeline.
async fn moved(tx: &mut sqlx::SqliteConnection, id: i64, state: Lifecycle) -> Result<()> {
    let event = Event::Moved(state);

    sqlx::query(
        "INSERT INTO timeline_events (conversation_id, at, kind, body)
         VALUES (?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), ?, ?)",
    )
    .bind(id)
    .bind(event.kind())
    .bind(event.body())
    .execute(&mut *tx)
    .await
    .with_context(|| {
        format!(
            "recording that Conversation {id} moved to {}",
            state.stored()
        )
    })?;

    Ok(())
}

/// Move a Conversation on to another state.
///
/// The blunt instrument, for the states no stage has arrived at yet. Starting to
/// grill and aborting have their own calls, because each of them is a move plus
/// everything else that has to be true at the same moment.
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
