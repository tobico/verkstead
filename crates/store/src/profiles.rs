//! The Agent Profiles a session can be run under: which account, and which
//! model.
//!
//! A Profile is four facts and a discriminator, because that is the whole of
//! what launching a session needs. The claude directory and config file are the
//! pair bind-mounted over `~/.claude` and `~/.claude.json` inside the sandbox,
//! which is what keeps one account's sessions out of another's; the model is
//! what the session runs on unless something says otherwise.
//!
//! The paths are stored resolved, as a Repo's is and for the same reason:
//! whoever saved the Profile had `..` and every symlink taken out of them before
//! they arrived, so what is recorded is what the filesystem means rather than
//! what somebody typed. Whether they are inside a Watched Path is decided above
//! the store, where the boundary lives.
//!
//! The agent type is a column with one value in it. Nothing branches on it and
//! nothing this stage does would read differently without it — the point is that
//! a second backend slots in beside `claude` rather than having to be migrated
//! in underneath it.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use sqlx::SqlitePool;

/// Which coding agent a Profile runs.
///
/// One value, spelled out in the column so the table reads as something. A word
/// this does not know is a database written by a Verkstead that has a backend
/// this one does not, which is worth saying rather than guessing past.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentType {
    Claude,
}

impl AgentType {
    fn stored(self) -> &'static str {
        match self {
            Self::Claude => "claude",
        }
    }

    fn read(word: &str) -> Result<Self> {
        Ok(match word {
            "claude" => Self::Claude,
            other => bail!("a Profile names the unknown agent type {other:?}"),
        })
    }
}

/// An Agent Profile as the store holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    pub id: i64,

    /// What the human calls this account. Unique: a picker with two `work` rows
    /// in it is a picker nobody can use.
    pub name: String,

    /// The directory bind-mounted over `~/.claude`, resolved.
    pub claude_dir: PathBuf,

    /// The file bind-mounted over `~/.claude.json`, resolved.
    pub config_file: PathBuf,

    /// What a session runs on unless it is told otherwise.
    pub model: String,

    pub agent_type: AgentType,
}

/// What a Profile is being saved as — everything but the id, which is the
/// store's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileFacts {
    pub name: String,
    pub claude_dir: PathBuf,
    pub config_file: PathBuf,
    pub model: String,
    pub agent_type: AgentType,
}

/// What became of writing a Profile down.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Saving {
    /// Recorded.
    Saved,

    /// There is no Profile with that id to change.
    NoSuchProfile,

    /// Another Profile is called that already.
    NameTaken,
}

/// What became of removing one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Deleting {
    Deleted,
    NoSuchProfile,

    /// A Conversation has chosen it. Removing it would leave that Conversation
    /// pointing at nothing, which is a session that fails to start later rather
    /// than a refusal now.
    InUse,
}

/// The table the Profiles live in.
///
/// `name` is unique, and the insert lets the index refuse a repeat rather than
/// looking first: two tabs saving the same name would otherwise both get past
/// the look.
pub(crate) async fn apply_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS profiles (
             id          INTEGER PRIMARY KEY AUTOINCREMENT,
             name        TEXT NOT NULL UNIQUE,
             claude_dir  TEXT NOT NULL,
             config_file TEXT NOT NULL,
             model       TEXT NOT NULL,
             agent_type  TEXT NOT NULL
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the profiles table")?;

    Ok(())
}

/// Record a Profile, which is expected to have been checked already: that its
/// pair exists and sits inside the Watched Paths is decided above the store.
///
/// `None` means another Profile is called that.
pub async fn create_profile(pool: &SqlitePool, facts: &ProfileFacts) -> Result<Option<Profile>> {
    let row: Option<(i64,)> = sqlx::query_as(
        "INSERT INTO profiles (name, claude_dir, config_file, model, agent_type)
         VALUES (?, ?, ?, ?, ?)
         ON CONFLICT (name) DO NOTHING
         RETURNING id",
    )
    .bind(&facts.name)
    .bind(text(&facts.claude_dir)?)
    .bind(text(&facts.config_file)?)
    .bind(&facts.model)
    .bind(facts.agent_type.stored())
    .fetch_optional(pool)
    .await
    .with_context(|| format!("saving the Profile {:?}", facts.name))?;

    Ok(row.map(|(id,)| Profile {
        id,
        name: facts.name.clone(),
        claude_dir: facts.claude_dir.clone(),
        config_file: facts.config_file.clone(),
        model: facts.model.clone(),
        agent_type: facts.agent_type,
    }))
}

/// Rewrite a Profile, whole: everything about one is the human's to change, and
/// nothing about it is an artifact that could have been built from it yet.
pub async fn update_profile(pool: &SqlitePool, id: i64, facts: &ProfileFacts) -> Result<Saving> {
    // The name it is being given may be another Profile's. Asked as its own
    // statement rather than caught off the update, because an update that
    // changed nothing and an update that hit the index are two different
    // sentences and `rows_affected` cannot tell them apart.
    let clash: Option<(i64,)> =
        sqlx::query_as("SELECT id FROM profiles WHERE name = ? AND id <> ?")
            .bind(&facts.name)
            .bind(id)
            .fetch_optional(pool)
            .await
            .with_context(|| format!("looking for another Profile called {:?}", facts.name))?;

    if clash.is_some() {
        return Ok(Saving::NameTaken);
    }

    let changed = sqlx::query(
        "UPDATE profiles
         SET name = ?, claude_dir = ?, config_file = ?, model = ?, agent_type = ?
         WHERE id = ?",
    )
    .bind(&facts.name)
    .bind(text(&facts.claude_dir)?)
    .bind(text(&facts.config_file)?)
    .bind(&facts.model)
    .bind(facts.agent_type.stored())
    .bind(id)
    .execute(pool)
    .await
    .with_context(|| format!("rewriting Profile {id}"))?
    .rows_affected();

    Ok(match changed {
        0 => Saving::NoSuchProfile,
        _ => Saving::Saved,
    })
}

/// Remove a Profile nobody is running under.
///
/// A Profile a Conversation has chosen is refused rather than taken away from
/// it: a Conversation pointing at a Profile that is not there is a session that
/// fails to start with nobody watching, which is the failure this whole stage is
/// arranged to move forward in time.
pub async fn delete_profile(pool: &SqlitePool, id: i64) -> Result<Deleting> {
    let chosen: Option<(i64,)> = sqlx::query_as(
        "SELECT id FROM conversations
         WHERE grilling_profile_id = ? OR implementation_profile_id = ?",
    )
    .bind(id)
    .bind(id)
    .fetch_optional(pool)
    .await
    .with_context(|| format!("looking for a Conversation using Profile {id}"))?;

    if chosen.is_some() {
        return Ok(Deleting::InUse);
    }

    let removed = sqlx::query("DELETE FROM profiles WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .with_context(|| format!("removing Profile {id}"))?
        .rows_affected();

    Ok(match removed {
        0 => Deleting::NoSuchProfile,
        _ => Deleting::Deleted,
    })
}

/// Every Profile, by name.
///
/// Alphabetical like the Repos, and for the same reason: a Profile is not news,
/// it is something to pick out of a short list, and the name is what it is
/// looked for by.
pub async fn profiles(pool: &SqlitePool) -> Result<Vec<Profile>> {
    let rows: Vec<(i64, String, String, String, String, String)> = sqlx::query_as(
        "SELECT id, name, claude_dir, config_file, model, agent_type
         FROM profiles
         ORDER BY name, id",
    )
    .fetch_all(pool)
    .await
    .context("listing the Agent Profiles")?;

    rows.into_iter().map(read_row).collect()
}

/// One Profile, or `None` if there is no such Profile.
pub async fn load_profile(pool: &SqlitePool, id: i64) -> Result<Option<Profile>> {
    let row: Option<(i64, String, String, String, String, String)> = sqlx::query_as(
        "SELECT id, name, claude_dir, config_file, model, agent_type
         FROM profiles
         WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .with_context(|| format!("loading Profile {id}"))?;

    row.map(read_row).transpose()
}

/// A row of the profiles table as a [`Profile`].
type Row = (i64, String, String, String, String, String);

fn read_row(row: Row) -> Result<Profile> {
    let (id, name, claude_dir, config_file, model, agent_type) = row;

    Ok(Profile {
        id,
        name,
        claude_dir: PathBuf::from(claude_dir),
        config_file: PathBuf::from(config_file),
        model,
        agent_type: AgentType::read(&agent_type)?,
    })
}

/// A path as SQLite can hold it, which is UTF-8 or nothing.
///
/// Refused outright rather than written lossily, as a Repo's path is: a stored
/// path that is not the one on disk is a boundary check that will pass for the
/// wrong directory later.
fn text(path: &Path) -> Result<&str> {
    path.to_str()
        .ok_or_else(|| anyhow!("the path {} is not valid UTF-8", path.display()))
}
