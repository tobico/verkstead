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
//! way the directions are hung off the conversations: there was no migration
//! machinery here when the list arrived, so it came as a new table rather than as
//! a column added to an old one. There is machinery now — see
//! [`super::migrations`] — and it is how [`Profile::memory`] became a column
//! instead. The old `model` column stays where it is, and a Profile written
//! before the list existed is read as the one entry that column holds — which is
//! what carries every saved Profile over with nothing for the human to re-enter.
//!
//! An account's paths are stored resolved, as a Repo's is and for the same
//! reason: whoever saved the Profile had `..` and every symlink taken out of
//! them before they arrived, so what is recorded is what the filesystem means
//! rather than what somebody typed. Whether they are of the shape their harness
//! wants is decided above the store, where the reading lives.
//!
//! A Profile's name is optional. A name is what tells two accounts of one
//! harness apart, and a harness with one account has nothing to tell apart — so
//! the column is nullable and uniqueness is two rules rather than one: at most
//! one unnamed Profile per harness, and no two named alike. Both are indexes,
//! so both refuse a write rather than being looked up in front of one.
//!
//! **A row may be a member's rather than this device's own**, and that is the
//! whole of what a **mirror** is: the same columns, marked with the device the
//! Profile is at home on and the id it has there (ADR-0020, *Shared Profiles*).
//! It is a row of this table rather than a table beside it because the point of
//! a mirror is the local id — a pairing, a Repo's memory of what it was last
//! grilled with and every Conversation go on holding one, and nothing that reads
//! a Profile id changes. What it costs is that the two uniqueness rules become
//! this device's own rows', which is what the partial indexes in
//! [`apply_schema`] are; what writes one is [`record_mirror`], and the account
//! paths on it are the home machine's and belong to no filesystem here.
//!
//! The agent type is a column, and it is what says which shape a row's account
//! is written in — the launch line's flags and the asking channel are keyed on
//! it. A second backend slots in beside `claude` rather than having to be
//! migrated in underneath it: it adds an arm to [`Account`] and keeps its home
//! in `profile_homes`, which is a table hung off `profiles` for the reason
//! `profile_models` is one.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use sqlx::SqlitePool;

/// Which coding agent a Profile runs.
///
/// One word apiece, spelled out in the column so the table reads as something.
/// A word this does not know is a database written by a Verkstead that has a
/// backend this one does not, which is worth saying rather than guessing past.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AgentType {
    Claude,
    Codex,
    Grok,
    OpenCode,
}

impl AgentType {
    /// The word this type is written down as.
    ///
    /// The `agent_type` column's, and the same word a sandbox carries in its
    /// environment so that `verkstead guide` printed inside one knows which
    /// backend it is being printed for. One spelling for both, because a second
    /// vocabulary would be a second thing to keep in step.
    pub fn word(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
            Self::Grok => "grok",
            Self::OpenCode => "opencode",
        }
    }

    /// And back again, which is the only way one is ever read.
    pub fn read(word: &str) -> Result<Self> {
        Ok(match word {
            "claude" => Self::Claude,
            "codex" => Self::Codex,
            "grok" => Self::Grok,
            "opencode" => Self::OpenCode,
            other => bail!("a Profile names the unknown agent type {other:?}"),
        })
    }

    /// How a session of this type puts a Question Set to the human.
    ///
    /// A fact about the backend rather than about the Set (ADR-0011): the CLI
    /// asks the same way everywhere, and what differs is whether the backend can
    /// afford to hold a shell command open for hours. Kept beside the type
    /// itself so that the Guide a session reads and the server that takes its
    /// Set answer the question the same way.
    pub fn channel(self) -> Channel {
        match self {
            Self::Claude | Self::OpenCode => Channel::Blocking,
            Self::Codex | Self::Grok => Channel::StoreAndNudge,
        }
    }
}

/// How a backend's sessions ask, which is one of two things.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    /// `verkstead ask` idles until the Response comes back, so the Answers are
    /// in front of the session when it goes on. What a backend that can hold a
    /// shell command open for hours does.
    Blocking,

    /// `verkstead ask` stores the Set and returns at once, the session ends its
    /// turn, and Verkstead types a line into its terminal when the Response
    /// lands — which the session answers by fetching with `verkstead answers`.
    /// What a backend whose shell tool yields after seconds does: an ask held
    /// open there is a paid model turn spent on every poll of it.
    StoreAndNudge,
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
        /// The directory a session's `~/.claude` is built from, resolved.
        claude_dir: PathBuf,

        /// The file a session's `~/.claude.json` is copied from, resolved.
        config_file: PathBuf,
    },

    /// Codex's one relocatable home, bind-mounted over `~/.codex`, resolved.
    ///
    /// The whole account: the credentials and the configuration are files inside
    /// it, so there is nothing beside it to travel with.
    Codex { home: PathBuf },

    /// Grok Build's one relocatable home, bind-mounted over `~/.grok`,
    /// resolved.
    ///
    /// Codex's shape, for the reason Codex's is that shape: a subscription
    /// login writes its `auth.json` inside the home and an API key arrives by
    /// environment, so the directory is the whole of what a Profile names
    /// either way.
    Grok { home: PathBuf },

    /// OpenCode's one relocatable home, resolved — the same shape again, but
    /// mounted as more than one directory.
    ///
    /// opencode keeps no single dot-directory: it reads the four XDG base
    /// directories and appends `opencode` to each, so what a Profile names here
    /// is a *home* those paths resolve inside, holding
    /// `.config/opencode` and `.local/share/opencode` — the account being the
    /// second of them, where `auth.json` and the session store are written. It
    /// is the shape one `HOME=<the directory> opencode` leaves behind, which is
    /// how such an account is made in the first place. Which of them are bound
    /// where the sandbox says, as every other account's mounting is.
    OpenCode { home: PathBuf },
}

impl Account {
    /// Which agent runs an account of this shape.
    pub fn agent_type(&self) -> AgentType {
        match self {
            Self::Claude { .. } => AgentType::Claude,
            Self::Codex { .. } => AgentType::Codex,
            Self::Grok { .. } => AgentType::Grok,
            Self::OpenCode { .. } => AgentType::OpenCode,
        }
    }

    /// The one directory this account is kept under, where its type keeps one.
    ///
    /// `None` for Claude, whose account is the pair in the row itself — see
    /// `profile_homes` in [`apply_schema`] for what the rest keep there.
    fn home(&self) -> Option<&Path> {
        match self {
            Self::Claude { .. } => None,
            Self::Codex { home } | Self::Grok { home } | Self::OpenCode { home } => Some(home),
        }
    }
}

/// An Agent Profile as the store holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    pub id: i64,

    /// What the human calls this account, where they have called it anything.
    ///
    /// Unique among the named: a picker with two `work` rows in it is a picker
    /// nobody can use. `None` is a Profile nobody typed a word for — the one
    /// account on its harness, where the harness and the model say the whole of
    /// what it is — and a harness has at most one of those, for the reason a
    /// name is unique.
    pub name: Option<String>,

    /// The account a session under this Profile is run as, in its type's shape.
    pub account: Account,

    /// The models this account can run a session on, in the order they were
    /// written. None of them is the default: the order is the human's typing
    /// kept intact so that editing the list reads back as they left it.
    pub models: Vec<String>,

    /// Whether a session under this Profile is given the account's memory store,
    /// or starts with an empty one of its own.
    ///
    /// On by default, and on for every Profile saved before there was a switch:
    /// the shared store is what the human has always been getting, so the switch
    /// is a way to stop rather than a way to start. Off, the session's store is
    /// its own and empty — fresh memory, and none of the human's transcripts
    /// reachable from inside it.
    pub memory: bool,

    /// Where this Profile is at home, where that is not this device.
    ///
    /// `None` is one of this device's own, which is every Profile there was
    /// before a cluster could share them. `Some` is a **mirror**: a row written
    /// down from what a member said, carrying what that member's row is drawn
    /// and picked by and nothing of this device's own. See [`Mirror`].
    pub mirror: Option<Mirror>,
}

/// Where a mirror is at home: the device it belongs to, and the id it has
/// there.
///
/// **Both halves, because neither is enough on its own.** The device is what a
/// row is drawn with and what an edit or a launch is put to; the id is what that
/// device calls this Profile, which is the only name for it that survives a
/// rename. The row's own [`Profile::id`] stays this device's, so every pairing,
/// every Repo's memory of what it was last grilled with and every Conversation
/// goes on holding a local id and nothing that reads one changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mirror {
    /// The **Device Id** of the machine this Profile is at home on.
    pub device: String,

    /// And what it is numbered there, which is what a refresh finds it by.
    pub id: i64,

    /// And whether the account holds a **login file** over there, as that
    /// device last said.
    ///
    /// The one fact about a member's account that this device cannot look at
    /// and cannot do without: a login kept somewhere that is not a file leaves
    /// nothing to mirror, so the Profile cannot be used away from home at all —
    /// see `Broken::NoLoginAtHome`. It travels on the row rather than being
    /// asked for at the launch, because the row has to *say* so wherever it is
    /// drawn, long before anybody presses anything.
    ///
    /// `true` for a mirror written down before this was carried, which is what
    /// was assumed of every mirror until now; the next refresh says what is
    /// really true.
    pub login: bool,
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
    /// to run: whether its Profile's pair is where it was left is read off the
    /// filesystem, which is above the store.
    pub fn picked(&self) -> bool {
        !matches!(self, Self::Nothing)
    }
}

/// What a Profile is being saved as — everything but the id, which is the
/// store's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileFacts {
    pub name: Option<String>,
    pub account: Account,
    pub models: Vec<String>,
    pub memory: bool,
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

    /// That harness already has a Profile nobody named.
    DefaultTaken,
}

/// Which of the two uniqueness rules turned a new Profile away.
///
/// Two words rather than [`Saving`]'s four, because there is no rewriting a
/// Profile that is not there to rewrite: what saving a new one comes to is the
/// row or one of these.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Clash {
    /// Another Profile is called that already.
    NameTaken,

    /// That harness already has a Profile nobody named.
    DefaultTaken,
}

/// What became of removing one.
///
/// Two answers rather than three. A Conversation having chosen the Profile was
/// the third of them and is not a refusal any more: it is nulled out of every
/// Pairing that named it instead — see [`delete_profile`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Deleting {
    Deleted,
    NoSuchProfile,
}

/// The tables the Profiles live in.
///
/// **Both uniqueness rules are about this device's own rows**, which is what
/// lets a mirror sit in this table at all: a member's Profile is written down
/// here as an ordinary row marked with the device it is at home on, and two
/// machines each keeping an account called `work` — or each keeping the one
/// unnamed Claude account, which is what nearly every installation holds — are
/// two rows this table has to take. So each rule is a partial index over the
/// rows with no home device on them, and the mirrors are held apart by a rule of
/// their own: one row per Profile per device it came from.
///
/// Which is why `name` is not `UNIQUE` on the column any more. A column
/// constraint cannot be made partial, and SQLite cannot drop one in place — so
/// the table is made over for it, in [`super::migrations`].
pub(crate) async fn apply_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS profiles (
             id          INTEGER PRIMARY KEY AUTOINCREMENT,
             name        TEXT,
             claude_dir  TEXT NOT NULL,
             config_file TEXT NOT NULL,
             model       TEXT NOT NULL,
             agent_type  TEXT NOT NULL,
             memory      INTEGER NOT NULL DEFAULT 1,
             home_device TEXT,
             home_id     INTEGER,
             home_login  INTEGER
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the profiles table")?;

    mirror_columns(pool).await?;
    home_login_column(pool).await?;

    // The rule the column's own `UNIQUE` carried: no two Profiles of this
    // device's own called the same thing. Over the local rows alone now — what a
    // member calls its accounts is that machine's business, and a picker tells
    // two rows called `work` apart by the device drawn on them.
    sqlx::query(
        "CREATE UNIQUE INDEX IF NOT EXISTS profiles_named_here
         ON profiles (name) WHERE home_device IS NULL",
    )
    .execute(pool)
    .await
    .context("creating the index that keeps this device's own Profiles named apart")?;

    // At most one unnamed Profile per harness. A name is what tells two accounts
    // of one harness apart, so a harness may have one account nobody named — and
    // a second would be two rows a picker draws the same way, which is what the
    // unique name was always for.
    //
    // This device's own again, and under a name of its own for it: the rule
    // moved, so the index that carries it is made afresh rather than left
    // standing as the rule it used to be. The old one is taken away below.
    sqlx::query(
        "CREATE UNIQUE INDEX IF NOT EXISTS profiles_one_unnamed_here_per_agent
         ON profiles (agent_type) WHERE name IS NULL AND home_device IS NULL",
    )
    .execute(pool)
    .await
    .context("creating the index that keeps one unnamed Profile per harness")?;

    // And the mirrors' own rule: one row per Profile per device it is at home
    // on. **This is what keeps a mirror's local id the same across refreshes** —
    // what a member says is written onto the row that is already there rather
    // than beside it, so a Pairing made against a mirror outlives every refresh
    // after it.
    sqlx::query(
        "CREATE UNIQUE INDEX IF NOT EXISTS profiles_one_mirror_per_home
         ON profiles (home_device, home_id) WHERE home_device IS NOT NULL",
    )
    .execute(pool)
    .await
    .context("creating the index that keeps one mirror per member Profile")?;

    // And the rule as it was before it was this device's own. Dropped rather
    // than left beside the two above: it names every row in the table, so a
    // member's unnamed Claude account would be refused for this device having
    // one of its own.
    sqlx::query("DROP INDEX IF EXISTS profiles_one_unnamed_per_agent")
        .execute(pool)
        .await
        .context("taking away the index that kept one unnamed Profile per harness everywhere")?;

    // The models each Profile can run, one row apiece. A table of its own for
    // the reason the directions are one: there was no migration machinery to
    // alter `profiles` with when the list arrived, so what was new hung off what
    // was there.
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
    // every backend but Claude, whose account is the pair in the row itself. A
    // table rather than a column for the reason `profile_models` is one: there
    // was no migration machinery to add a column with when homes arrived, so the
    // fact became a table hung off `profiles` by id. One home per Profile, so the
    // id is the key.
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

/// Where a row that is a mirror is at home, added through `ALTER TABLE` as well
/// as declared above — [`super::members::apply_schema`]'s rule, for its reason:
/// a database made this morning and one written before a Profile could be
/// somebody else's take the same path and end the same shape.
///
/// Both arriving null, which is what was true of every row before them: every
/// Profile a device held was its own.
///
/// Its own function because it is run twice over. [`apply_schema`] runs it
/// before the indexes below, which are written over these two columns — a column
/// that arrived after them would be one they could not name — and the rewrite in
/// [`super::migrations`] that lets a Profile go unnamed runs afterwards and
/// rebuilds this table to the shape it had before either column existed, so the
/// rewrite that follows *it* asks for them again.
pub(crate) async fn mirror_columns(pool: &SqlitePool) -> Result<()> {
    for (column, kind) in [("home_device", "TEXT"), ("home_id", "INTEGER")] {
        added(pool, column, kind).await?;
    }

    Ok(())
}

/// And whether the account a mirror names holds a login file over there, added
/// the same way and asked for in the same two places.
///
/// Its own function rather than a third entry in the loop above only because
/// the rewrite in [`super::migrations`] that lets a Profile go unnamed remakes
/// this table in a shape from before any of the three existed — so the rewrite
/// after it asks for the two above and this one again, and then carries all
/// three across. **Asked for rather than re-added**: a column dropped here and
/// put back afterwards is one the pool can refuse, the pragma that looks for it
/// and the `ALTER` that adds it being handed to whichever connections are free.
///
/// Null is *nobody has said*, which reads as a login being there: it is what was
/// assumed of every mirror before the column existed, and the first refresh
/// after this start says what is really true. See [`Mirror::login`].
pub(crate) async fn home_login_column(pool: &SqlitePool) -> Result<()> {
    added(pool, "home_login", "INTEGER").await
}

/// One column on the profiles table, added where it is not there already.
async fn added(pool: &SqlitePool, column: &str, kind: &str) -> Result<()> {
    let there: Option<(String,)> =
        sqlx::query_as("SELECT name FROM pragma_table_info('profiles') WHERE name = ?")
            .bind(column)
            .fetch_optional(pool)
            .await
            .with_context(|| format!("looking for the profiles table's {column} column"))?;

    if there.is_none() {
        // Interpolated rather than bound, because a column name is not a value
        // — and every one of them is written out by a caller here rather than
        // taken from anywhere.
        sqlx::query(&format!("ALTER TABLE profiles ADD COLUMN {column} {kind}"))
            .execute(pool)
            .await
            .with_context(|| format!("adding the profiles table's {column} column"))?;
    }

    Ok(())
}

/// Record a Profile, which is expected to have been checked already: that its
/// pair exists and is of its harness's shape is decided above the store.
///
/// The [`Clash`] is which of the two uniqueness rules turned it away, and it is
/// read off what was being saved rather than out of the index: a row with a name
/// can only have hit the unique name, and a row without one can only have hit
/// the partial index over the harnesses, which is the only index a null name is
/// in. So the index is still what refuses — nothing is looked up in front of the
/// write — and the sentence to say about it needs no second query.
///
/// Those two are the whole of what an insert here can conflict on. The id is the
/// store's own and comes off `AUTOINCREMENT`, so the primary key is not one, and
/// there is no third index.
pub async fn create_profile(
    pool: &SqlitePool,
    facts: &ProfileFacts,
) -> Result<Result<Profile, Clash>> {
    let mut tx = super::writing(pool, "saving a Profile").await?;

    let (claude_dir, config_file) = pair(&facts.account)?;

    let row: Option<(i64,)> = sqlx::query_as(
        "INSERT INTO profiles (name, claude_dir, config_file, model, agent_type, memory)
         VALUES (?, ?, ?, ?, ?, ?)
         ON CONFLICT DO NOTHING
         RETURNING id",
    )
    .bind(&facts.name)
    .bind(claude_dir)
    .bind(config_file)
    .bind(legacy_model(facts))
    .bind(facts.account.agent_type().word())
    .bind(facts.memory)
    .fetch_optional(&mut *tx)
    .await
    .with_context(|| format!("saving the Profile {:?}", facts.name))?;

    let Some((id,)) = row else {
        return Ok(Err(match facts.name {
            Some(_) => Clash::NameTaken,
            None => Clash::DefaultTaken,
        }));
    };

    write_models(&mut tx, id, &facts.models).await?;
    write_home(&mut tx, id, &facts.account).await?;

    tx.commit()
        .await
        .with_context(|| format!("saving the Profile {:?}", facts.name))?;

    Ok(Ok(Profile {
        id,
        name: facts.name.clone(),
        account: facts.account.clone(),
        models: facts.models.clone(),
        memory: facts.memory,

        // A Profile saved here is this device's own. What writes a mirror is
        // [`record_mirror`], which is the other way a row arrives in this table.
        mirror: None,
    }))
}

/// Rewrite a Profile, whole: everything about one is the human's to change, and
/// nothing about it is an artifact that could have been built from it yet.
pub async fn update_profile(pool: &SqlitePool, id: i64, facts: &ProfileFacts) -> Result<Saving> {
    let mut tx = super::writing(pool, "rewriting a Profile").await?;

    // What it is being made may be another Profile's already — its name, or the
    // one unnamed row its harness is allowed. Asked as its own statement rather
    // than caught off the update, because an update that changed nothing and an
    // update that hit an index are two different sentences and `rows_affected`
    // cannot tell them apart.
    //
    // One query per rule, because only one of them is ever asked: a rewrite with
    // a name is in the unique name and nowhere else, and one without is in the
    // partial index over the harnesses and nowhere else.
    //
    // This device's own rows either way, which is what the two indexes are over:
    // a mirror called `work` is a member's account and says nothing about what
    // this device may call one of its own.
    let clash: Option<(i64,)> = match &facts.name {
        Some(name) => sqlx::query_as(
            "SELECT id FROM profiles WHERE name = ? AND home_device IS NULL AND id <> ?",
        )
        .bind(name)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .with_context(|| format!("looking for another Profile called {name:?}"))?,

        None => sqlx::query_as(
            "SELECT id FROM profiles
             WHERE name IS NULL AND agent_type = ? AND home_device IS NULL AND id <> ?",
        )
        .bind(facts.account.agent_type().word())
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .context("looking for another Profile of that harness that nobody named")?,
    };

    if clash.is_some() {
        return Ok(match facts.name {
            Some(_) => Saving::NameTaken,
            None => Saving::DefaultTaken,
        });
    }

    let (claude_dir, config_file) = pair(&facts.account)?;

    let changed = sqlx::query(
        "UPDATE profiles
         SET name = ?, claude_dir = ?, config_file = ?, model = ?, agent_type = ?,
             memory = ?
         WHERE id = ?",
    )
    .bind(&facts.name)
    .bind(claude_dir)
    .bind(config_file)
    .bind(legacy_model(facts))
    .bind(facts.account.agent_type().word())
    .bind(facts.memory)
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
    write_home(&mut tx, id, &facts.account).await?;

    tx.commit()
        .await
        .with_context(|| format!("rewriting Profile {id}"))?;

    Ok(Saving::Saved)
}

/// Remove a Profile, taking it out of everything that named it.
///
/// **Always possible.** A Conversation that had chosen it used to stand in the
/// way; now its choice is nulled out and the removal goes through. What that
/// costs is a session that starts nothing the next time that Conversation is
/// driven, and a steer to pick another account — which is the human's to spend
/// on a Profile they have just decided they are finished with, rather than a
/// refusal that leaves them unable to finish with it at all.
///
/// It costs a session already running nothing: one that has launched holds the
/// account it launched under, and nothing re-reads this row to keep it going.
///
/// Both halves of every Pairing that named it go, not the Profile half alone: a
/// Pairing is an account and a model chosen together, and a role left holding
/// the model by itself would be half a choice nothing can launch. The Repo's
/// memory of what it was last grilled with goes the same way — `repo_pairings`
/// names the Profile in a column that cannot be null — so the next Conversation
/// started there arrives with that picker empty rather than prefilled with an
/// account that is gone. And so does the picker of a steer somebody is part-way
/// through writing, which is the same choice one press before it is settled.
///
/// What is deliberately left alone is the record of what has already run:
/// [`super::session_pairings`] keeps the Profile's *name* rather than its id, so
/// every session ever launched under it goes on saying so.
pub async fn delete_profile(pool: &SqlitePool, id: i64) -> Result<Deleting> {
    let mut tx = super::writing(pool, "removing a Profile").await?;

    forget_pairings(&mut tx, id).await?;
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

/// Write down what a member said about one of its Profiles: the **mirror** row
/// for it on this device, made or brought up to date.
///
/// **The local id is the point of the row and it never moves.** A mirror is
/// found by the pair it is at home under — the device, and the id it has there —
/// so what a refresh does to a Profile this device has already heard of is
/// rewrite the row that is there. Every Pairing made against a mirror, and every
/// Repo's memory of one, goes on naming a row that is still that Profile.
///
/// What is written is what a row is drawn and picked by and nothing of this
/// device's own: the name, the harness, the account as the far end holds it, the
/// models and the memory switch. The account's paths are the home machine's and
/// belong to no filesystem here — they are kept because they are what that
/// Profile *is*, and what a session away from home is actually given is another
/// stage's.
///
/// No [`Clash`] can come back. The two uniqueness rules are over this device's
/// own rows, and a mirror is in neither — see [`apply_schema`] — so a member's
/// `work` lands beside this device's `work`, and a member's unnamed Claude
/// account beside this device's.
pub async fn record_mirror(pool: &SqlitePool, at: &Mirror, facts: &ProfileFacts) -> Result<i64> {
    let mut tx = super::writing(pool, "writing down a member's Profile").await?;

    let (claude_dir, config_file) = pair(&facts.account)?;

    let there: Option<(i64,)> =
        sqlx::query_as("SELECT id FROM profiles WHERE home_device = ? AND home_id = ?")
            .bind(&at.device)
            .bind(at.id)
            .fetch_optional(&mut *tx)
            .await
            .with_context(|| {
                format!(
                    "looking for the mirror of Profile {} on {}",
                    at.id, at.device
                )
            })?;

    let id = match there {
        Some((id,)) => {
            sqlx::query(
                "UPDATE profiles
                 SET name = ?, claude_dir = ?, config_file = ?, model = ?, agent_type = ?,
                     memory = ?, home_login = ?
                 WHERE id = ?",
            )
            .bind(&facts.name)
            .bind(claude_dir)
            .bind(config_file)
            .bind(legacy_model(facts))
            .bind(facts.account.agent_type().word())
            .bind(facts.memory)
            .bind(at.login)
            .bind(id)
            .execute(&mut *tx)
            .await
            .with_context(|| {
                format!("rewriting the mirror of Profile {} on {}", at.id, at.device)
            })?;

            // Replaced rather than reconciled, exactly as a rewrite of this
            // device's own does it: what came over the link is the whole of what
            // that Profile is now, and which of the models happen to be the ones
            // it listed before is not a fact anything holds on to.
            forget_models(&mut tx, id).await?;
            forget_home(&mut tx, id).await?;

            id
        }

        None => {
            let (id,): (i64,) = sqlx::query_as(
                "INSERT INTO profiles
                     (name, claude_dir, config_file, model, agent_type, memory,
                      home_device, home_id, home_login)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                 RETURNING id",
            )
            .bind(&facts.name)
            .bind(claude_dir)
            .bind(config_file)
            .bind(legacy_model(facts))
            .bind(facts.account.agent_type().word())
            .bind(facts.memory)
            .bind(&at.device)
            .bind(at.id)
            .bind(at.login)
            .fetch_one(&mut *tx)
            .await
            .with_context(|| {
                format!(
                    "writing down Profile {} of {} as a mirror",
                    at.id, at.device
                )
            })?;

            id
        }
    };

    write_models(&mut tx, id, &facts.models).await?;
    write_home(&mut tx, id, &facts.account).await?;

    tx.commit()
        .await
        .with_context(|| format!("writing down Profile {} of {}", at.id, at.device))?;

    Ok(id)
}

/// Take away every mirror of `device` but the ones it still holds, named by the
/// ids they have over there.
///
/// **What a Profile removed at home comes to here.** The next refresh finds it
/// gone from what that device answers, so the mirror goes — and it goes the way
/// a local removal goes, out of both halves of every Pairing that named it and
/// out of the Repos' memory with it. See [`delete_profile`], whose reasons are
/// these.
///
/// What comes back is the local ids that were taken away, which is what the
/// caller says a line about: a Profile leaving every device is worth one.
///
/// Nothing at all where `at_home` names everything this device holds of that
/// member, which is every refresh but the one after a removal.
pub async fn forget_mirrors_except(
    pool: &SqlitePool,
    device: &str,
    at_home: &[i64],
) -> Result<Vec<i64>> {
    let mut tx = super::writing(pool, "forgetting a member's Profiles").await?;

    // Asked as a statement rather than filtered in Rust for the reason the
    // removal is one statement: what is being found is rows to delete inside the
    // transaction that deletes them.
    let kept = placeholders(at_home.len());

    let looking = format!(
        "SELECT id FROM profiles
         WHERE home_device = ? AND home_id NOT IN ({kept})",
    );

    let mut asking = sqlx::query_as::<_, (i64,)>(&looking).bind(device);

    for id in at_home {
        asking = asking.bind(id);
    }

    let gone: Vec<(i64,)> = asking
        .fetch_all(&mut *tx)
        .await
        .with_context(|| format!("looking for the Profiles {device} no longer holds"))?;

    let gone: Vec<i64> = gone.into_iter().map(|(id,)| id).collect();

    for id in &gone {
        forget_mirror(&mut tx, *id).await?;
    }

    tx.commit()
        .await
        .with_context(|| format!("forgetting the Profiles {device} no longer holds"))?;

    Ok(gone)
}

/// And every mirror of a device that is no longer a member of this cluster at
/// all.
///
/// **The membership is what prunes this**, which is the stance the merged list
/// takes about the rows it holds of a member: a device that has been unlinked is
/// not one whose accounts this device has any business offering. A member that
/// is merely not answering is not this — its rows are every bit as much its own
/// as they were yesterday, and they stay.
pub async fn forget_mirrors_of_departed(pool: &SqlitePool, members: &[String]) -> Result<Vec<i64>> {
    let mut tx = super::writing(pool, "forgetting a departed device's Profiles").await?;

    let linked = placeholders(members.len());

    let looking = format!(
        "SELECT id FROM profiles
         WHERE home_device IS NOT NULL AND home_device NOT IN ({linked})",
    );

    let mut asking = sqlx::query_as::<_, (i64,)>(&looking);

    for device in members {
        asking = asking.bind(device);
    }

    let gone: Vec<(i64,)> = asking
        .fetch_all(&mut *tx)
        .await
        .context("looking for the Profiles of devices this one is no longer linked to")?;

    let gone: Vec<i64> = gone.into_iter().map(|(id,)| id).collect();

    for id in &gone {
        forget_mirror(&mut tx, *id).await?;
    }

    tx.commit()
        .await
        .context("forgetting the Profiles of devices this one is no longer linked to")?;

    Ok(gone)
}

/// One mirror taken away, out of everything that named it.
///
/// [`delete_profile`]'s own body, inside a transaction somebody else opened:
/// a mirror leaving is a Profile leaving, and a row nulled out of a Pairing here
/// reads exactly as one nulled out by a removal pressed on this device.
async fn forget_mirror(tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>, id: i64) -> Result<()> {
    forget_pairings(tx, id).await?;
    forget_models(tx, id).await?;
    forget_home(tx, id).await?;

    sqlx::query("DELETE FROM profiles WHERE id = ?")
        .bind(id)
        .execute(&mut **tx)
        .await
        .with_context(|| format!("removing the mirror that Profile {id} is"))?;

    Ok(())
}

/// A comma-separated run of `?` for a list bound one item at a time.
///
/// An empty list comes to a subquery that answers no rows at all, because
/// `NOT IN ()` is not something SQLite will parse and *nothing is kept* is
/// exactly the case that has to work: a member that answered no Profiles, and a
/// cluster that has just lost its last member, are both every mirror going.
fn placeholders(many: usize) -> String {
    match many {
        0 => "SELECT NULL WHERE 0".to_owned(),
        _ => std::iter::repeat_n("?", many)
            .collect::<Vec<_>>()
            .join(", "),
    }
}

/// Every Profile, by name.
///
/// Alphabetical like the Repos, and for the same reason: a Profile is not news,
/// it is something to pick out of a short list, and the name is what it is
/// looked for by. The one nobody named sorts first, SQLite ordering a null
/// before every string — which is where a harness's default belongs on a list
/// its named accounts are the exceptions on.
pub async fn profiles(pool: &SqlitePool) -> Result<Vec<Profile>> {
    let rows: Vec<Row> =
        sqlx::query_as(&format!("SELECT {COLUMNS} FROM profiles ORDER BY name, id",))
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

    // And the homes the same way, for the types whose whole account is one.
    // Empty for a list of Claude Profiles, which is what most installations
    // hold.
    let kept: Vec<(i64, String)> = sqlx::query_as("SELECT profile_id, home FROM profile_homes")
        .fetch_all(pool)
        .await
        .context("listing the homes the Agent Profiles keep their accounts under")?;

    let mut homes: HashMap<i64, String> = kept.into_iter().collect();

    rows.into_iter()
        .map(|row| {
            let listed = models.remove(&row.0).unwrap_or_default();
            let home = homes.remove(&row.0);
            read_row(row, listed, home)
        })
        .collect()
}

/// One Profile, or `None` if there is no such Profile.
pub async fn load_profile(pool: &SqlitePool, id: i64) -> Result<Option<Profile>> {
    let row: Option<Row> = sqlx::query_as(&format!("SELECT {COLUMNS} FROM profiles WHERE id = ?",))
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

    let home: Option<(String,)> =
        sqlx::query_as("SELECT home FROM profile_homes WHERE profile_id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
            .with_context(|| format!("loading the home Profile {id} keeps its account under"))?;

    read_row(
        row,
        listed.into_iter().map(|(model,)| model).collect(),
        home.map(|(home,)| home),
    )
    .map(Some)
}

/// A row of the profiles table as a [`Profile`].
type Row = (
    i64,
    Option<String>,
    String,
    String,
    String,
    String,
    bool,
    Option<String>,
    Option<i64>,
    Option<bool>,
);

/// The columns every read of the table takes, in [`Row`]'s order.
///
/// Written once rather than in each of the two statements that select them: the
/// tuple they are read into is one shape, and two lists a column apart would be
/// a mirror read as somebody else's Profile.
const COLUMNS: &str = "id, name, claude_dir, config_file, model, agent_type, memory, \
                       home_device, home_id, home_login";

/// One row, whatever `profile_models` holds for it, and the home in
/// `profile_homes` where its type keeps one.
///
/// An empty list is a Profile written before the list existed: what it holds is
/// the one model in the old column, which becomes the sole entry of its list
/// without anybody having to retype it. A Profile saved since always has its
/// rows, so the old column is never read for one.
///
/// A type whose account is one home and no home row is a Profile something
/// edited by hand: refused rather than read as a home of the empty string,
/// because an account of nowhere is a bind that would land on `/`.
fn read_row(row: Row, listed: Vec<String>, home: Option<String>) -> Result<Profile> {
    let (id, name, claude_dir, config_file, model, agent_type, memory, device, at_home, login) =
        row;

    let models = match (listed.is_empty(), model.is_empty()) {
        (true, false) => vec![model],
        _ => listed,
    };

    let agent_type = AgentType::read(&agent_type)?;

    let account = match agent_type {
        AgentType::Claude => Account::Claude {
            claude_dir: PathBuf::from(claude_dir),
            config_file: PathBuf::from(config_file),
        },
        AgentType::Codex => Account::Codex {
            home: PathBuf::from(kept(agent_type, id, home)?),
        },
        AgentType::Grok => Account::Grok {
            home: PathBuf::from(kept(agent_type, id, home)?),
        },
        AgentType::OpenCode => Account::OpenCode {
            home: PathBuf::from(kept(agent_type, id, home)?),
        },
    };

    // A mirror is both columns or neither: the index that holds one per member
    // Profile is over the pair, and a row with one of them is a row somebody
    // edited by hand. Refused rather than read as local, because a Profile drawn
    // as this device's own is one the human would be offered a session under.
    //
    // The login is read beside them rather than with them: it arrived after the
    // pair did, so a mirror written before it says nothing, and nothing said is
    // what was assumed of every mirror until then — an account with a login to
    // lend. The next refresh writes what is really true.
    let mirror = match (device, at_home) {
        (Some(device), Some(id)) => Some(Mirror {
            device,
            id,
            login: login.unwrap_or(true),
        }),
        (None, None) => None,
        _ => bail!("Profile {id} is half a mirror: it names a device or an id there and not both"),
    };

    Ok(Profile {
        id,
        name,
        account,
        models,
        memory,
        mirror,
    })
}

/// The home a Profile of a type that keeps one was recorded with.
fn kept(agent_type: AgentType, id: i64, home: Option<String>) -> Result<String> {
    home.ok_or_else(|| {
        anyhow!(
            "Profile {id} runs {} and has no home recorded to run it under",
            agent_type.word()
        )
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
        Account::Codex { .. } | Account::Grok { .. } | Account::OpenCode { .. } => ("", ""),
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

/// Take a Profile out of every Pairing that named it, on its way to being
/// removed.
///
/// Four places: a Conversation names one per role, and a Repo remembers the one
/// it was last grilled with in each of the three. Nothing is refused and nothing
/// is reported — a Profile nobody has ever picked passes through here without
/// touching a row, which is what the ordinary removal is.
///
/// The model rows go before the columns they are found by, because that is what
/// finds them: a role's model is looked up through the Conversation whose column
/// still names the Profile, so nulling first would leave the model half behind
/// with nothing left pointing at it.
///
/// Nobody is put on the row that runs no session by this. A role picked away is
/// something the human chose and *no Profile any more* is not: what the removal
/// leaves is a picker with nothing in it, which is the state a Conversation
/// starts in.
async fn forget_pairings(tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>, id: i64) -> Result<()> {
    for role in super::conversations::Role::ALL {
        sqlx::query(&format!(
            "DELETE FROM pairing_models
             WHERE role = ?
               AND conversation_id IN (SELECT id FROM conversations WHERE {} = ?)",
            role.column(),
        ))
        .bind(role.stored())
        .bind(id)
        .execute(&mut **tx)
        .await
        .with_context(|| {
            format!("clearing the models paired with Profile {id} in the {role:?} role")
        })?;

        sqlx::query(&format!(
            "UPDATE conversations SET {} = NULL WHERE {} = ?",
            role.column(),
            role.column(),
        ))
        .bind(id)
        .execute(&mut **tx)
        .await
        .with_context(|| {
            format!("clearing Profile {id} off the Conversations that chose it to {role:?}")
        })?;
    }

    sqlx::query("DELETE FROM repo_pairings WHERE profile_id = ?")
        .bind(id)
        .execute(&mut **tx)
        .await
        .with_context(|| format!("forgetting that a Repo was last grilled under Profile {id}"))?;

    // And the picker of a steer somebody is part-way through writing, which is
    // a Pairing chosen and not yet settled — see [`super::pending_steers`].
    // Both halves, by the rule above: a form holding the model of an account
    // that has gone is half a choice, and the pane reads a row with one half in
    // it as nothing picked anyway. The rest of the form is left exactly as it
    // was — the Profile going is no reason to throw away an afternoon's
    // writing, and picking another is a control the pane already has.
    sqlx::query("UPDATE pending_steers SET profile_id = NULL, model = NULL WHERE profile_id = ?")
        .bind(id)
        .execute(&mut **tx)
        .await
        .with_context(|| format!("clearing Profile {id} off the steers being written under it"))?;

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

/// Put down the one directory a Profile of a type that keeps one is kept under.
///
/// Nothing at all for Claude, whose account is the pair in the row itself — so
/// an installation with no Profile of a later type has an empty table, exactly
/// as it did before there was a type with a home.
async fn write_home(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    id: i64,
    account: &Account,
) -> Result<()> {
    let Some(home) = account.home() else {
        return Ok(());
    };

    sqlx::query("INSERT INTO profile_homes (profile_id, home) VALUES (?, ?)")
        .bind(id)
        .bind(text(home)?)
        .execute(&mut **tx)
        .await
        .with_context(|| format!("saving the home Profile {id} keeps its account under"))?;

    Ok(())
}

/// And the same for the home a Profile of a type that keeps one has.
///
/// `profile_homes` references `profiles(id)` and foreign keys are enforced, so
/// a Profile removed with its home left behind it is a Profile that cannot be
/// removed at all. Called wherever the models are and for the same reasons: a
/// removal takes the whole of a Profile with it, and a rewrite replaces the
/// account rather than reconciling it.
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
