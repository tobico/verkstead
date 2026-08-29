//! The Agent Profiles a session can be run under: which account, and which
//! models.
//!
//! A Profile is a name, an account and a list of models, because that is the
//! whole of what launching a session needs. The account is what keeps one
//! account's sessions out of another's, and its shape is its agent type's — see
//! [`Account`]; the models are what a session may be run on.
//!
//! The models are a list and not one model, and the list is the Profile's own:
//! different Profiles reach different accounts, so each names what it can
//! actually launch rather than sharing one list nobody's account really has.
//! There is no default and no preferred entry — the list only says what is
//! available, and every pick made from it is explicit.
//!
//! They live in a table of their own, `profile_models`, hung off `profiles` the
//! way the directions are hung off the conversations: there is no migration
//! machinery here and `profiles` is STRICT, so a new fact arrives as a new table
//! rather than as a column added to an old one. The old `model` column stays
//! where it is, and a Profile written before the list existed is read as the one
//! entry that column holds — which is what carries every saved Profile over with
//! nothing for the human to re-enter.
//!
//! An account's paths are stored resolved, as a Repo's is and for the same
//! reason: whoever saved the Profile had `..` and every symlink taken out of
//! them before they arrived, so what is recorded is what the filesystem means
//! rather than what somebody typed. Whether they are inside a Watched Path is
//! decided above the store, where the boundary lives.
//!
//! The agent type is a column with one value in it, and it is what says which
//! shape a row's account is written in — the launch line's flags are keyed on
//! it already. The point is that a second backend slots in beside `claude`
//! rather than having to be migrated in underneath it: it adds an arm to
//! [`Account`] and keeps its home in `profile_homes`, which is made here and
//! stays empty until there is a type with one.

use std::collections::HashMap;
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

/// The account a Profile runs as, in the shape the agent type running it keeps
/// one.
///
/// Claude Code's account is a pair — a directory and a file beside it, bound
/// over `~/.claude` and `~/.claude.json` — because that is how Claude Code keeps
/// one. Every backend after it keeps its whole account under a single
/// relocatable home, which is a shape of its own and a table of its own: see
/// `profile_homes` in [`apply_schema`].
///
/// The shape *is* the discriminator, rather than sitting beside one: a Profile
/// holding a pair runs Claude, and there is no second field for it to disagree
/// with. Which agent type that comes to is [`Account::agent_type`], and the
/// column is what it is written down as.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Account {
    /// Claude Code's pair.
    Claude {
        /// The directory bind-mounted over `~/.claude`, resolved.
        claude_dir: PathBuf,

        /// The file bind-mounted over `~/.claude.json`, resolved.
        config_file: PathBuf,
    },
}

impl Account {
    /// Which agent runs an account of this shape.
    pub fn agent_type(&self) -> AgentType {
        match self {
            Self::Claude { .. } => AgentType::Claude,
        }
    }
}

/// An Agent Profile as the store holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    pub id: i64,

    /// What the human calls this account. Unique: a picker with two `work` rows
    /// in it is a picker nobody can use.
    pub name: String,

    /// The account a session under this Profile is run as, in its type's shape.
    pub account: Account,

    /// The models this account can run a session on, in the order they were
    /// written. None of them is the default: the order is the human's typing
    /// kept intact so that editing the list reads back as they left it.
    pub models: Vec<String>,
}

impl Profile {
    /// Which agent this Profile runs, which is what its account's shape says.
    pub fn agent_type(&self) -> AgentType {
        self.account.agent_type()
    }

    /// The model to run on where nothing paired one with it.
    ///
    /// The first of the list, which is a Profile's only model in the ordinary
    /// case. Nothing picks this any more — a session runs on the model its
    /// Pairing names — so what is left for it is the Conversation that chose a
    /// Profile before there was a model to choose beside it: see
    /// [`Pairing::runs_on`]. `None` is a Profile with no models at all —
    /// refused above the store, so what it means here is a row somebody edited
    /// by hand.
    pub fn model(&self) -> Option<&str> {
        self.models.first().map(String::as_str)
    }
}

/// What a Conversation has settled about one of its roles: a Profile, and the
/// one of that Profile's models its sessions run on.
///
/// The pair rather than the Profile alone, because a Profile's list says what
/// its account *can* launch and a session runs one thing. Both halves are
/// chosen together, in one press, and both are fixed when grilling starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pairing {
    pub profile: Profile,

    /// The model paired with it, where one was paired.
    ///
    /// `None` is a choice made before pairings existed: the Profile was picked
    /// alone and the model was whatever that Profile carried. Left as a state
    /// to be in rather than filled in on the way out, because the two are
    /// different things to a Conversation still drafting — an unpaired choice
    /// is one to make again — and see [`Pairing::runs_on`] for what a
    /// Conversation past drafting runs on instead.
    pub model: Option<String>,
}

impl Pairing {
    /// What a session under this Pairing is launched on.
    ///
    /// The paired model, and the Profile's own where nothing was paired — which
    /// is the model that Profile would have been run on at the time the choice
    /// was made, so a Conversation that chose before pairings existed goes on
    /// exactly as it did.
    pub fn runs_on(&self) -> Option<&str> {
        self.model.as_deref().or_else(|| self.profile.model())
    }
}

/// What a Conversation has settled about one of its roles: the Pairing that
/// role's sessions run under, that the role runs no session at all, or nothing
/// yet.
///
/// Three states rather than an `Option`, because *no review* is a choice the
/// human made and an empty picker is one they have not. Both leave the role
/// without a Pairing and only one of them lets the work start, so a record that
/// could not tell them apart would either refuse a settled Conversation or
/// start an unsettled one.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Picked {
    /// The picker is empty: nothing has been chosen for this role.
    ///
    /// Which includes a Profile chosen before pairings existed — see
    /// [`Pairing::model`] — because half a choice is a choice to make again.
    #[default]
    Nothing,

    /// The role runs no session at all, picked from the same flat list the
    /// Pairings are picked from and stored apart from having picked nothing.
    Skipped,

    /// The Profile and model this role's sessions are launched under.
    Under(Pairing),
}

impl Picked {
    /// The Pairing where one was picked, for everything that reads a role as
    /// something to launch a session under.
    pub fn pairing(&self) -> Option<&Pairing> {
        match self {
            Self::Under(pairing) => Some(pairing),
            _ => None,
        }
    }

    /// Whether the human picked the row that runs no session.
    pub fn skipped(&self) -> bool {
        matches!(self, Self::Skipped)
    }

    /// Whether anything has been picked at all — a Pairing or the row that says
    /// there is to be none.
    ///
    /// Says nothing about whether a Pairing that was picked is still something
    /// to run: whether its Profile's pair is where it was left is read against
    /// the Watched Paths, which is above the store.
    pub fn picked(&self) -> bool {
        !matches!(self, Self::Nothing)
    }
}

/// What a Profile is being saved as — everything but the id, which is the
/// store's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileFacts {
    pub name: String,
    pub account: Account,
    pub models: Vec<String>,
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

/// The tables the Profiles live in.
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

    // The models each Profile can run, one row apiece. A table of its own for
    // the reason the directions are one: `profiles` is STRICT and there is no
    // migration machinery to alter it with, so what is new hangs off what is
    // there.
    //
    // `position` is the order they were written in and nothing more — no entry
    // is preferred — kept so that a list read back into the form is the list the
    // human typed.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS profile_models (
             profile_id INTEGER NOT NULL REFERENCES profiles(id),
             position   INTEGER NOT NULL,
             model      TEXT NOT NULL,
             PRIMARY KEY (profile_id, position)
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the profile_models table")?;

    // And where a Profile whose account is one relocatable home keeps it —
    // every backend but Claude, none of which has landed yet, so this table is
    // made and left empty. It is made now rather than with the first of them
    // for the reason `profile_models` is a table at all: `profiles` is STRICT,
    // there is no migration machinery, and a new fact about a Profile is a new
    // table hung off it by id. One home per Profile, so the id is the key.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS profile_homes (
             profile_id INTEGER PRIMARY KEY REFERENCES profiles(id),
             home       TEXT NOT NULL
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the profile_homes table")?;

    Ok(())
}

/// Record a Profile, which is expected to have been checked already: that its
/// pair exists and sits inside the Watched Paths is decided above the store.
///
/// `None` means another Profile is called that.
pub async fn create_profile(pool: &SqlitePool, facts: &ProfileFacts) -> Result<Option<Profile>> {
    let mut tx = super::writing(pool, "saving a Profile").await?;

    let (claude_dir, config_file) = pair(&facts.account)?;

    let row: Option<(i64,)> = sqlx::query_as(
        "INSERT INTO profiles (name, claude_dir, config_file, model, agent_type)
         VALUES (?, ?, ?, ?, ?)
         ON CONFLICT (name) DO NOTHING
         RETURNING id",
    )
    .bind(&facts.name)
    .bind(claude_dir)
    .bind(config_file)
    .bind(legacy_model(facts))
    .bind(facts.account.agent_type().stored())
    .fetch_optional(&mut *tx)
    .await
    .with_context(|| format!("saving the Profile {:?}", facts.name))?;

    let Some((id,)) = row else {
        return Ok(None);
    };

    write_models(&mut tx, id, &facts.models).await?;

    tx.commit()
        .await
        .with_context(|| format!("saving the Profile {:?}", facts.name))?;

    Ok(Some(Profile {
        id,
        name: facts.name.clone(),
        account: facts.account.clone(),
        models: facts.models.clone(),
    }))
}

/// Rewrite a Profile, whole: everything about one is the human's to change, and
/// nothing about it is an artifact that could have been built from it yet.
pub async fn update_profile(pool: &SqlitePool, id: i64, facts: &ProfileFacts) -> Result<Saving> {
    let mut tx = super::writing(pool, "rewriting a Profile").await?;

    // The name it is being given may be another Profile's. Asked as its own
    // statement rather than caught off the update, because an update that
    // changed nothing and an update that hit the index are two different
    // sentences and `rows_affected` cannot tell them apart.
    let clash: Option<(i64,)> =
        sqlx::query_as("SELECT id FROM profiles WHERE name = ? AND id <> ?")
            .bind(&facts.name)
            .bind(id)
            .fetch_optional(&mut *tx)
            .await
            .with_context(|| format!("looking for another Profile called {:?}", facts.name))?;

    if clash.is_some() {
        return Ok(Saving::NameTaken);
    }

    let (claude_dir, config_file) = pair(&facts.account)?;

    let changed = sqlx::query(
        "UPDATE profiles
         SET name = ?, claude_dir = ?, config_file = ?, model = ?, agent_type = ?
         WHERE id = ?",
    )
    .bind(&facts.name)
    .bind(claude_dir)
    .bind(config_file)
    .bind(legacy_model(facts))
    .bind(facts.account.agent_type().stored())
    .bind(id)
    .execute(&mut *tx)
    .await
    .with_context(|| format!("rewriting Profile {id}"))?
    .rows_affected();

    if changed == 0 {
        return Ok(Saving::NoSuchProfile);
    }

    // The list is replaced rather than reconciled: it is a handful of lines the
    // human retyped, and which of them happen to be the same lines as before is
    // not a fact anything holds on to.
    forget_models(&mut tx, id).await?;
    write_models(&mut tx, id, &facts.models).await?;

    // And the account with them: what a Profile's account is, is its type's
    // shape, so a rewrite that changed the type would otherwise leave the old
    // type's home sitting behind it.
    forget_home(&mut tx, id).await?;

    tx.commit()
        .await
        .with_context(|| format!("rewriting Profile {id}"))?;

    Ok(Saving::Saved)
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
         WHERE grilling_profile_id = ?
            OR implementation_profile_id = ?
            OR review_profile_id = ?",
    )
    .bind(id)
    .bind(id)
    .bind(id)
    .fetch_optional(pool)
    .await
    .with_context(|| format!("looking for a Conversation using Profile {id}"))?;

    if chosen.is_some() {
        return Ok(Deleting::InUse);
    }

    let mut tx = super::writing(pool, "removing a Profile").await?;

    forget_models(&mut tx, id).await?;
    forget_home(&mut tx, id).await?;

    let removed = sqlx::query("DELETE FROM profiles WHERE id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("removing Profile {id}"))?
        .rows_affected();

    tx.commit()
        .await
        .with_context(|| format!("removing Profile {id}"))?;

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
    let rows: Vec<Row> = sqlx::query_as(
        "SELECT id, name, claude_dir, config_file, model, agent_type
         FROM profiles
         ORDER BY name, id",
    )
    .fetch_all(pool)
    .await
    .context("listing the Agent Profiles")?;

    // The whole of the little table at once rather than a query per Profile: the
    // list is a handful of accounts, and reading it in one hop is the same shape
    // as the one look at the filesystem the server takes over the lot of them.
    let listed: Vec<(i64, String)> = sqlx::query_as(
        "SELECT profile_id, model FROM profile_models ORDER BY profile_id, position",
    )
    .fetch_all(pool)
    .await
    .context("listing the models the Agent Profiles run")?;

    let mut models: HashMap<i64, Vec<String>> = HashMap::new();
    for (profile_id, model) in listed {
        models.entry(profile_id).or_default().push(model);
    }

    rows.into_iter()
        .map(|row| {
            let listed = models.remove(&row.0).unwrap_or_default();
            read_row(row, listed)
        })
        .collect()
}

/// One Profile, or `None` if there is no such Profile.
pub async fn load_profile(pool: &SqlitePool, id: i64) -> Result<Option<Profile>> {
    let row: Option<Row> = sqlx::query_as(
        "SELECT id, name, claude_dir, config_file, model, agent_type
         FROM profiles
         WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .with_context(|| format!("loading Profile {id}"))?;

    let Some(row) = row else {
        return Ok(None);
    };

    let listed: Vec<(String,)> =
        sqlx::query_as("SELECT model FROM profile_models WHERE profile_id = ? ORDER BY position")
            .bind(id)
            .fetch_all(pool)
            .await
            .with_context(|| format!("loading the models Profile {id} runs"))?;

    read_row(row, listed.into_iter().map(|(model,)| model).collect()).map(Some)
}

/// A row of the profiles table as a [`Profile`].
type Row = (i64, String, String, String, String, String);

/// One row and whatever `profile_models` holds for it.
///
/// An empty list is a Profile written before the list existed: what it holds is
/// the one model in the old column, which becomes the sole entry of its list
/// without anybody having to retype it. A Profile saved since always has its
/// rows, so the old column is never read for one.
fn read_row(row: Row, listed: Vec<String>) -> Result<Profile> {
    let (id, name, claude_dir, config_file, model, agent_type) = row;

    let models = match (listed.is_empty(), model.is_empty()) {
        (true, false) => vec![model],
        _ => listed,
    };

    let account = match AgentType::read(&agent_type)? {
        AgentType::Claude => Account::Claude {
            claude_dir: PathBuf::from(claude_dir),
            config_file: PathBuf::from(config_file),
        },
    };

    Ok(Profile {
        id,
        name,
        account,
        models,
    })
}

/// What an account puts in the row's own two path columns.
///
/// Claude's pair, which is what those columns were made for. A type whose
/// account is a single home has nothing to say in them and keeps its home in
/// `profile_homes` instead — they are NOT NULL and cannot be dropped from a
/// STRICT table, so what it writes there is the empty string.
fn pair(account: &Account) -> Result<(&str, &str)> {
    Ok(match account {
        Account::Claude {
            claude_dir,
            config_file,
        } => (text(claude_dir)?, text(config_file)?),
    })
}

/// Put a Profile's list down, in the order it was given.
async fn write_models(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    id: i64,
    models: &[String],
) -> Result<()> {
    for (position, model) in models.iter().enumerate() {
        sqlx::query("INSERT INTO profile_models (profile_id, position, model) VALUES (?, ?, ?)")
            .bind(id)
            .bind(position as i64)
            .bind(model)
            .execute(&mut **tx)
            .await
            .with_context(|| format!("saving the models Profile {id} runs"))?;
    }

    Ok(())
}

/// And take it away again.
async fn forget_models(tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>, id: i64) -> Result<()> {
    sqlx::query("DELETE FROM profile_models WHERE profile_id = ?")
        .bind(id)
        .execute(&mut **tx)
        .await
        .with_context(|| format!("clearing the models Profile {id} runs"))?;

    Ok(())
}

/// And the same for the home a Profile of a type that keeps one has.
///
/// `profile_homes` references `profiles(id)` and foreign keys are enforced, so
/// a Profile removed with its home left behind it is a Profile that cannot be
/// removed at all. Called wherever the models are and for the same reasons: a
/// removal takes the whole of a Profile with it, and a rewrite replaces the
/// account rather than reconciling it.
///
/// Nothing writes a home yet — Claude's account is the pair in the row itself —
/// so today this clears nothing. The stage that lands a type with a home writes
/// it beside this, exactly as `write_models` sits beside `forget_models`.
async fn forget_home(tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>, id: i64) -> Result<()> {
    sqlx::query("DELETE FROM profile_homes WHERE profile_id = ?")
        .bind(id)
        .execute(&mut **tx)
        .await
        .with_context(|| format!("clearing the home Profile {id} keeps its account under"))?;

    Ok(())
}

/// What goes in the old `model` column, which is NOT NULL and cannot be dropped
/// from a STRICT table.
///
/// The first of the list, so that a database read by a Verkstead from before
/// this change still says something true. Nothing here reads it back for a
/// Profile that has its rows.
fn legacy_model(facts: &ProfileFacts) -> String {
    facts.models.first().cloned().unwrap_or_default()
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
