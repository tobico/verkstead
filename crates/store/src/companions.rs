//! The other registered Repos a Conversation works alongside: which ones, how
//! it may touch each, and what each one's checkout will be cut from.
//!
//! A companion is a **registered Repo added to a Conversation**, which is the
//! whole of the trust boundary: the registry says what Verkstead may operate
//! inside, and a Conversation may only compose what is already in it. Nothing
//! here reaches a filesystem — what a companion becomes on disk is the grill
//! start's business, and what it is allowed to be is this table's.
//!
//! It hangs off the Conversation as a side table of its own, the way the
//! worktree and the direction and the adoption do: there is no migration
//! machinery here and `conversations` is STRICT and left alone. One row per Repo
//! per Conversation, by the primary key — which is also what makes *added
//! twice* something the insert refuses rather than something a read-then-write
//! has to notice in time.
//!
//! All four of a companion's facts are columns, and all four are the human's:
//! which Repo when it is added, and then the mode, the base and the branch name
//! from the row it draws — every one of them refused the moment the
//! Conversation is past drafting.

use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use sqlx::{Sqlite, SqlitePool, Transaction};

use super::Repo;
use super::conversations::{Edited, branch_made, not_drafting};

/// How far into a companion a session may reach.
///
/// Two, and no third: a repository is there to be read, or it is there to be
/// worked in. What the words decide is the sandbox's binds and whether a branch
/// is cut for it — neither of which has a halfway.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompanionMode {
    /// Read it and nothing else. Its checkout is detached at the commit the
    /// base resolved to, and the sandbox binds it read-only.
    ReadOnly,

    /// Work in it: a branch of its own, cut from the base, and a sandbox that
    /// may write to it.
    ReadWrite,
}

impl CompanionMode {
    /// The word the column holds. Spelled out for the reason a Lifecycle's is:
    /// the table should read as something rather than as a number nobody can
    /// look up.
    pub(crate) fn stored(self) -> &'static str {
        match self {
            Self::ReadOnly => "read-only",
            Self::ReadWrite => "read-write",
        }
    }

    /// The mode a stored word names. A word this does not know is a database
    /// written by a Verkstead this one does not understand, which is worth
    /// saying rather than guessing past — and guessing past it here would be
    /// guessing at whether a session may write to somebody's repository.
    pub(crate) fn read(word: &str) -> Result<Self> {
        Ok(match word {
            "read-only" => Self::ReadOnly,
            "read-write" => Self::ReadWrite,
            other => bail!("a companion repo is in the unknown mode {other:?}"),
        })
    }
}

/// One companion of a Conversation, with the Repo it names read back beside it.
///
/// The Repo whole rather than by id, as a Conversation's own is: everything
/// that reads a companion wants what it is called or where it is, and an id
/// alone can say neither.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Companion {
    pub repo: Repo,

    pub mode: CompanionMode,

    /// The branch this companion's checkout comes off, where the human named
    /// one. `None` is the same rule the Conversation's own base follows: that
    /// repository's default branch, as it stands when grilling starts.
    pub base_ref: Option<String>,

    /// What a read-write companion's branch is to be called, or empty for
    /// *mirroring* — the Conversation's own branch name, followed as it is
    /// renamed. Empty on a read-only companion too, there being no branch to
    /// name: its checkout is detached.
    pub branch: String,

    /// Where this companion was checked out, once grilling has made its
    /// worktree.
    ///
    /// `None` before grilling starts and again after closing, which is the
    /// Conversation's own worktree's rule and the same fact by it: a companion
    /// has a directory for exactly as long as the work does.
    pub worktree: Option<PathBuf>,
}

impl Companion {
    /// The branch this companion's work is done on, given the Conversation's
    /// own — or `None` where it holds none, a read-only companion's checkout
    /// being detached.
    ///
    /// Where *mirroring* is resolved, and the only place: an empty name is not
    /// a branch called nothing, it is the record saying this companion follows
    /// the Conversation's branch. Which it can be asked about at any time,
    /// because the Conversation's own branch is settled before either of them
    /// is cut.
    pub fn branch_for(&self, conversation: &str) -> Option<String> {
        if self.mode == CompanionMode::ReadOnly {
            return None;
        }

        Some(match self.branch.is_empty() {
            true => conversation.to_owned(),
            false => self.branch.clone(),
        })
    }
}

/// Where a companion's checkout was put, for the record that follows the work.
///
/// The path alone, because the path is the whole of what the grill start
/// decided that the configuration does not already say: which Repo, which mode
/// and which branch are all on the companion's own row, and the branch resolves
/// through [`Companion::branch_for`] wherever it is wanted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompanionWorktree {
    pub repo_id: i64,

    pub path: PathBuf,
}

/// What became of adding one.
///
/// Every refusal is named rather than collapsed into one, because each is
/// something different for the human to make of it: a Repo that is gone, a
/// Conversation that has moved on, and the two that are about what a companion
/// *is* — a Conversation is not a companion of itself, and a Repo added twice
/// would be one repository with two checkouts in one sandbox.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Adding {
    /// Added, with the defaults: read-only, the default-branch rule, and no
    /// branch name.
    Added,

    /// There is no Conversation with that id.
    NoSuchConversation,

    /// It is past drafting, so its configuration froze when grilling started.
    NotDrafting,

    /// There is no Repo with that id — taken off the registry between the menu
    /// reading it and the press that picked one.
    NoSuchRepo,

    /// The Repo picked is the Conversation's own. It is already the work's
    /// repository; adding it beside itself would be a second checkout of it in
    /// the same sandbox.
    OwnRepo,

    /// That Repo is a companion of this Conversation already.
    AlreadyAdded,
}

/// And of taking one away.
///
/// No *no such companion*: taking away one that is not there leaves the
/// Conversation exactly as the press asked for, which is what
/// [`Removing::Removed`] says. What is refused is the two things that are wrong
/// with the asking rather than with the state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Removing {
    /// Gone, or never there.
    Removed,

    /// There is no Conversation with that id.
    NoSuchConversation,

    /// It is past drafting, so its configuration froze when grilling started.
    NotDrafting,
}

/// The table the companions live in.
///
/// One row per Repo per Conversation, by the primary key: a Repo added twice is
/// refused by the insert rather than by a look that two tabs could both get
/// past.
pub(crate) async fn apply_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS companions (
             conversation_id INTEGER NOT NULL REFERENCES conversations(id),
             repo_id         INTEGER NOT NULL REFERENCES repos(id),
             mode            TEXT NOT NULL,
             base_ref        TEXT,
             branch          TEXT NOT NULL,
             PRIMARY KEY (conversation_id, repo_id)
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the companions table")?;

    // And where each of them was checked out, once a grill start has checked it
    // out. Room of its own rather than a column on `companions`, for the reason
    // the Conversation's own worktree is not a column on `conversations`: there
    // is no migration machinery here and both tables are STRICT and left alone.
    //
    // Not the `worktrees` table either, which is one row per Conversation by its
    // primary key and stays that way — a Conversation with three companions has
    // four checkouts, and widening that table would be widening the shape every
    // other reader of it is written against.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS companion_worktrees (
             conversation_id INTEGER NOT NULL REFERENCES conversations(id),
             repo_id         INTEGER NOT NULL REFERENCES repos(id),
             path            TEXT NOT NULL,
             PRIMARY KEY (conversation_id, repo_id)
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the companion worktrees table")?;

    Ok(())
}

/// Record where a Conversation's companions were checked out.
///
/// Written over whatever is there rather than inserted, which is the rule the
/// Conversation's own worktree row follows: a record that somehow holds a
/// directory already is corrected to the one just made.
///
/// Takes the transaction the Conversation is moving in, because it is the same
/// moment: a Conversation that said it was grilling without saying where its
/// companions went would be one nothing could bind into a sandbox and nothing
/// would come back and remove.
pub(crate) async fn record_worktrees(
    tx: &mut Transaction<'_, Sqlite>,
    id: i64,
    worktrees: &[CompanionWorktree],
) -> Result<()> {
    for worktree in worktrees {
        sqlx::query(
            "INSERT INTO companion_worktrees (conversation_id, repo_id, path) VALUES (?, ?, ?)
             ON CONFLICT (conversation_id, repo_id) DO UPDATE SET path = excluded.path",
        )
        .bind(id)
        .bind(worktree.repo_id)
        .bind(super::repos::text(&worktree.path)?)
        .execute(&mut **tx)
        .await
        .with_context(|| {
            format!(
                "recording the worktree of Repo {} on Conversation {id}",
                worktree.repo_id
            )
        })?;
    }

    Ok(())
}

/// Forget them again, the directories themselves having been removed.
///
/// [`super::close_conversation`]'s doing, beside the row for the Conversation's
/// own: a closed Conversation has no checkouts, and the branches its read-write
/// companions were worked on stay where they are.
pub(crate) async fn forget_worktrees(tx: &mut Transaction<'_, Sqlite>, id: i64) -> Result<()> {
    sqlx::query("DELETE FROM companion_worktrees WHERE conversation_id = ?")
        .bind(id)
        .execute(&mut **tx)
        .await
        .with_context(|| format!("forgetting the companion worktrees of Conversation {id}"))?;

    Ok(())
}

/// Add a registered Repo to a drafting Conversation, with the least the human
/// has to say: read-only, the default-branch rule, and no branch name.
///
/// Refused past drafting the way the branch name and the base commit are, and
/// off the same two questions: where the Conversation has got to, and whether
/// its branch has been made. A configuration frozen at grill start is one no
/// press of the setup card rewrites.
pub async fn add_companion(pool: &SqlitePool, id: i64, repo_id: i64) -> Result<Adding> {
    if let Some(refusal) = editable(pool, id).await? {
        return Ok(refusal);
    }

    // Its own Repo before the registry, because it is the cheaper question and
    // the more specific answer: a Conversation's own repository is registered
    // by definition, so asking the other way round would only ever say the
    // same thing less usefully.
    let own: Option<(i64,)> = sqlx::query_as("SELECT repo_id FROM conversations WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .with_context(|| format!("reading which Repo Conversation {id} is in"))?;

    if own.map(|(repo,)| repo) == Some(repo_id) {
        return Ok(Adding::OwnRepo);
    }

    if super::load_repo(pool, repo_id).await?.is_none() {
        return Ok(Adding::NoSuchRepo);
    }

    let added: Option<(i64,)> = sqlx::query_as(
        "INSERT INTO companions (conversation_id, repo_id, mode, base_ref, branch)
         VALUES (?, ?, ?, NULL, '')
         ON CONFLICT (conversation_id, repo_id) DO NOTHING
         RETURNING repo_id",
    )
    .bind(id)
    .bind(repo_id)
    .bind(CompanionMode::ReadOnly.stored())
    .fetch_optional(pool)
    .await
    .with_context(|| format!("adding Repo {repo_id} to Conversation {id}"))?;

    Ok(match added {
        Some(_) => Adding::Added,
        None => Adding::AlreadyAdded,
    })
}

/// Take one away again, for as long as the Conversation is still drafting.
pub async fn remove_companion(pool: &SqlitePool, id: i64, repo_id: i64) -> Result<Removing> {
    if let Some(refusal) = editable(pool, id).await? {
        return Ok(match refusal {
            Adding::NoSuchConversation => Removing::NoSuchConversation,
            _ => Removing::NotDrafting,
        });
    }

    sqlx::query("DELETE FROM companions WHERE conversation_id = ? AND repo_id = ?")
        .bind(id)
        .bind(repo_id)
        .execute(pool)
        .await
        .with_context(|| format!("taking Repo {repo_id} off Conversation {id}"))?;

    Ok(Removing::Removed)
}

/// What became of configuring one.
///
/// One outcome for all three changes, the way [`Edited`] is one for the Brief
/// and the branch and the base: they are refused for the same reasons, and a
/// caller telling them apart would be telling apart the same sentence three
/// times. What is different between them — a branch git will not take, a branch
/// the companion's repository does not have — is decided above the store, where
/// git is asked.
///
/// [`Edited`]: super::Edited
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Configured {
    /// Recorded.
    Saved,

    /// There is no Conversation with that id.
    NoSuchConversation,

    /// It is past drafting, so its configuration froze when grilling started.
    NotDrafting,

    /// That Repo is not a companion of this Conversation — taken off between
    /// the card drawing the row and the press that configured it. Said rather
    /// than reported as done: the press asked a row to hold something, and
    /// there is no row to hold it.
    NoSuchCompanion,
}

/// What a press on a companion's row changes about it.
///
/// The three of them as one type because they are one write with one guard in
/// front of it: which column moves is the whole of the difference, and three
/// functions would be three copies of the same two questions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change<'a> {
    /// The mode switch. Flipping to read-only takes the branch name with it:
    /// a read-only companion is checked out detached and has no branch, so a
    /// name left behind would be a name for a branch nobody will cut.
    Mode(CompanionMode),

    /// The base picker. `None` is the default-branch rule, which is what a
    /// companion starts on.
    Base(Option<&'a str>),

    /// The branch field, where empty is *mirroring* — the Conversation's own
    /// branch name, followed as it is renamed.
    Branch(&'a str),
}

/// Configure a companion of a drafting Conversation: its mode, its base, or the
/// name of the branch its work is done on.
///
/// Refused for what adding one is refused for, and off the same two questions:
/// a configuration frozen at grill start is one no press of the setup card
/// rewrites.
pub async fn configure_companion(
    pool: &SqlitePool,
    id: i64,
    repo_id: i64,
    change: Change<'_>,
) -> Result<Configured> {
    if let Some(refusal) = editable(pool, id).await? {
        return Ok(match refusal {
            Adding::NoSuchConversation => Configured::NoSuchConversation,
            _ => Configured::NotDrafting,
        });
    }

    let update = match change {
        // Two columns rather than one, for [`Change::Mode`]'s reason — and the
        // branch is left alone going the other way, there being nothing to
        // clear and mirroring to go back to.
        Change::Mode(CompanionMode::ReadOnly) => sqlx::query(
            "UPDATE companions SET mode = ?, branch = '' WHERE conversation_id = ? AND repo_id = ?",
        )
        .bind(CompanionMode::ReadOnly.stored()),
        Change::Mode(mode) => {
            sqlx::query("UPDATE companions SET mode = ? WHERE conversation_id = ? AND repo_id = ?")
                .bind(mode.stored())
        }
        Change::Base(base) => sqlx::query(
            "UPDATE companions SET base_ref = ? WHERE conversation_id = ? AND repo_id = ?",
        )
        .bind(base),
        Change::Branch(branch) => sqlx::query(
            "UPDATE companions SET branch = ? WHERE conversation_id = ? AND repo_id = ?",
        )
        .bind(branch),
    };

    let done = update
        .bind(id)
        .bind(repo_id)
        .execute(pool)
        .await
        .with_context(|| format!("configuring Repo {repo_id} on Conversation {id}"))?;

    Ok(match done.rows_affected() {
        0 => Configured::NoSuchCompanion,
        _ => Configured::Saved,
    })
}

/// Every companion of a Conversation, by the Repo's name.
///
/// Alphabetical for the reason the Repo list is: the rows are something to read
/// and pick out of rather than news, and the order they were added in is a fact
/// about nothing. The id breaks a tie between two directories of one name.
pub async fn companions(pool: &SqlitePool, id: i64) -> Result<Vec<Companion>> {
    /// The columns in the order the query below selects them.
    type Row = (
        i64,
        String,
        String,
        String,
        String,
        Option<String>,
        String,
        Option<String>,
    );

    // The checkout by an outer join, because a companion has one only once
    // grilling has made it: a drafting Conversation's companions are rows with
    // nowhere on disk yet, and that is the ordinary state rather than a missing
    // one.
    let rows: Vec<Row> = sqlx::query_as(
        "SELECT r.id, r.path, r.name, r.default_branch, c.mode, c.base_ref, c.branch, w.path
         FROM companions c
         JOIN repos r ON r.id = c.repo_id
         LEFT JOIN companion_worktrees w
                ON w.conversation_id = c.conversation_id AND w.repo_id = c.repo_id
         WHERE c.conversation_id = ?
         ORDER BY r.name, r.id",
    )
    .bind(id)
    .fetch_all(pool)
    .await
    .with_context(|| format!("reading the companion repos of Conversation {id}"))?;

    rows.into_iter()
        .map(
            |(repo_id, path, name, default_branch, mode, base_ref, branch, worktree)| {
                Ok(Companion {
                    repo: Repo {
                        id: repo_id,
                        path: std::path::PathBuf::from(path),
                        name,
                        default_branch,
                    },
                    mode: CompanionMode::read(&mode)?,
                    base_ref: base_ref.filter(|base| !base.is_empty()),
                    branch,
                    worktree: worktree.map(PathBuf::from),
                })
            },
        )
        .collect()
}

/// The reason this Conversation's companions are not the human's to change, or
/// `None` where they are.
///
/// The same two questions the branch name and the base commit are refused off,
/// asked here as one because the answers are the same sentence: the
/// configuration froze when grilling started.
async fn editable(pool: &SqlitePool, id: i64) -> Result<Option<Adding>> {
    if let Some(refusal) = not_drafting(pool, id).await? {
        return Ok(Some(match refusal {
            Edited::NoSuchConversation => Adding::NoSuchConversation,
            _ => Adding::NotDrafting,
        }));
    }

    if branch_made(pool, id).await? {
        return Ok(Some(Adding::NotDrafting));
    }

    Ok(None)
}
