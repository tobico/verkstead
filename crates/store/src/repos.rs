//! The Repos registered with Verkstead: the git repositories it has been told
//! about, from inside a Watched Path.
//!
//! A registration is three facts and nothing else — where the repository is,
//! what to call it, and which branch it works from — because that is the whole
//! of what a Conversation needs to find it again. The repository's files stay
//! the source of truth for everything else; nothing here is a copy of them.
//!
//! The path is stored resolved: whoever registered it had `..` and every
//! symlink taken out of it before it arrived, so the row holds the path the
//! filesystem actually means rather than the one somebody typed. That is also
//! what makes the uniqueness real — two spellings of one directory are one
//! Repo.
//!
//! Taking a Repo away is an **unregistering** rather than a delete: every
//! Conversation ever started on one names it by id, and a row deleted out from
//! under them would be a Timeline that could no longer say which repository its
//! work was done in. So the row stays and is flagged, in a table of its own
//! beside the registrations — the reason an archiving is a row rather than a
//! column, said again: there is no migration machinery here and `repos` is
//! STRICT and left alone. Every read that offers Repos for *new* work goes
//! through [`registered_repos`], which does not show a flagged one; every read
//! that resolves a Repo something is already on goes by id, and finds it where
//! it was.
//!
//! Which is also why registering a path a flagged row already holds revives
//! that row rather than being refused as registered already: the path is still
//! unique and the Repo is still the same Repo, so a second registration of it
//! is the human asking for it back.
//!
//! One more fact about a Repo lives beside the registration in the same shape:
//! how a merge conflict on its pull requests is resolved, where the human has
//! overridden what `config.yaml` says for every Repo at once. A row is an
//! override and no row is the global answer — see [`ConflictResolution`].

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use sqlx::SqlitePool;

/// A Repo as the store holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Repo {
    pub id: i64,

    /// Where the repository is, resolved: absolute, with no symlink and no
    /// `..` left in it.
    pub path: PathBuf,

    /// What to call it in a list. The directory's own name, which is what the
    /// human calls it too.
    pub name: String,

    /// The branch a Conversation branches from unless it says otherwise.
    pub default_branch: String,
}

/// The tables the registrations live in.
///
/// `path` is unique, which is what makes registering the same repository twice
/// something the insert refuses rather than something a read-then-write has to
/// notice in time.
pub(crate) async fn apply_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS repos (
             id             INTEGER PRIMARY KEY AUTOINCREMENT,
             path           TEXT NOT NULL UNIQUE,
             name           TEXT NOT NULL,
             default_branch TEXT NOT NULL
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the repos table")?;

    // And which of them have been taken away, one row apiece. The row being
    // there is the whole of the flag, and taking it away again is what reviving
    // a Repo is — the shape an archived Conversation is kept in, and for the
    // same reason: `repos` is STRICT and there is no migration machinery here to
    // add a column with.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS unregistered_repos (
             repo_id         INTEGER PRIMARY KEY REFERENCES repos(id),
             unregistered_at TEXT NOT NULL
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the unregistered_repos table")?;

    // And how each of them resolves a merge conflict, where the human has said
    // something other than what the settings file says for every Repo at once.
    //
    // A table of its own beside the registrations for the reason the flag above
    // is one: `repos` is STRICT and there is no migration machinery here to add
    // a column with. A row is an override and no row is the global answer, so
    // there is nothing here to mean *unset* and nothing to write for a Repo
    // nobody has been to.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS repo_resolutions (
             repo_id  INTEGER PRIMARY KEY REFERENCES repos(id),
             strategy TEXT NOT NULL
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the repo resolutions table")?;

    Ok(())
}

/// Record a repository, which is expected to have been checked already: that it
/// is inside a Watched Path, and that it is a git repository, is decided above
/// the store.
///
/// `None` means this path is registered already. Refused by the unique index
/// rather than by looking first, so two tabs cannot both get past the look.
///
/// A path a Repo that was taken away still holds is the one registration that
/// is neither of those: the row is revived rather than refused, and it comes
/// back with the name and the default branch this registration read off the
/// repository just now — it is the same Repo, and what it has been called since
/// somebody took it away is a fact about the repository rather than about the
/// row. The upsert's `WHERE` is what tells the two apart: a row nobody flagged
/// falls through to doing nothing, which is the refusal above.
pub async fn register_repo(
    pool: &SqlitePool,
    path: &Path,
    name: &str,
    default_branch: &str,
) -> Result<Option<Repo>> {
    let stored = text(path)?;

    let mut tx = super::writing(pool, "registering a Repo").await?;

    let row: Option<(i64,)> = sqlx::query_as(
        "INSERT INTO repos (path, name, default_branch)
         VALUES (?, ?, ?)
         ON CONFLICT (path) DO UPDATE
             SET name = excluded.name, default_branch = excluded.default_branch
             WHERE repos.id IN (SELECT repo_id FROM unregistered_repos)
         RETURNING id",
    )
    .bind(stored)
    .bind(name)
    .bind(default_branch)
    .fetch_optional(&mut *tx)
    .await
    .with_context(|| format!("registering the Repo at {}", path.display()))?;

    let Some((id,)) = row else {
        return Ok(None);
    };

    // Whichever of the two it was, this Repo is registered now — and for a fresh
    // one there is no flag to take away, so the delete is what says "registered"
    // in both cases rather than a branch on which of them happened.
    sqlx::query("DELETE FROM unregistered_repos WHERE repo_id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("reviving the Repo at {}", path.display()))?;

    tx.commit()
        .await
        .with_context(|| format!("registering the Repo at {}", path.display()))?;

    Ok(Some(Repo {
        id,
        path: path.to_owned(),
        name: name.to_owned(),
        default_branch: default_branch.to_owned(),
    }))
}

/// What became of taking one away.
///
/// Named the way [`super::Deleting`] is, because it is the same sentence about
/// the other thing the settings page configures — and refused for the same kind
/// of reason: what is being worked on now is not something to take out from
/// under the work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unregistering {
    /// Taken off the registry. Every list stops offering it, and every
    /// Conversation on it goes on saying which repository it was worked in.
    Unregistered,

    /// There is no registered Repo with that id — including one somebody has
    /// already taken away, which is a link followed twice rather than a Repo.
    NoSuchRepo,

    /// A Conversation that is neither Done nor Closed is on it. Work still going
    /// on in a repository is the reason to keep it registered, so the removal is
    /// refused rather than the work being left on a Repo nothing offers.
    InUse,
}

/// Take a Repo off the registry, if nothing live is being worked in it.
///
/// The live count is the one the Repo's own pane shows — [`super::work_on_repo`]
/// — so what refuses the removal is the same reading the human is looking at
/// when they press it, rather than a second opinion about what "finished" means.
pub async fn unregister_repo(pool: &SqlitePool, id: i64) -> Result<Unregistering> {
    // Through the same read the pane behind the press is drawn from, so that
    // what counts as being on the registry is said in one place rather than
    // spelled again here.
    if registered_repo(pool, id).await?.is_none() {
        return Ok(Unregistering::NoSuchRepo);
    }

    if super::work_on_repo(pool, id).await?.live > 0 {
        return Ok(Unregistering::InUse);
    }

    sqlx::query(
        "INSERT INTO unregistered_repos (repo_id, unregistered_at)
         VALUES (?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
         ON CONFLICT (repo_id) DO NOTHING",
    )
    .bind(id)
    .execute(pool)
    .await
    .with_context(|| format!("unregistering the Repo {id}"))?;

    Ok(Unregistering::Unregistered)
}

/// Every registered Repo, by name.
///
/// Alphabetical rather than newest first, unlike the Set lists: a Repo is not
/// news, it is something to pick out of a list that barely changes, and the
/// name is what it is looked for by. The id breaks a tie between two
/// directories of the same name in different places.
///
/// One that has been taken away is not on it — this is the read everything
/// offering Repos for new work goes through, and what was taken away is not on
/// offer. What is already on one resolves it by id and finds it; see
/// [`load_repo`].
pub async fn registered_repos(pool: &SqlitePool) -> Result<Vec<Repo>> {
    let rows: Vec<(i64, String, String, String)> = sqlx::query_as(
        "SELECT id, path, name, default_branch
         FROM repos
         WHERE id NOT IN (SELECT repo_id FROM unregistered_repos)
         ORDER BY name, id",
    )
    .fetch_all(pool)
    .await
    .context("listing the registered Repos")?;

    Ok(rows
        .into_iter()
        .map(|(id, path, name, default_branch)| Repo {
            id,
            path: PathBuf::from(path),
            name,
            default_branch,
        })
        .collect())
}

/// One Repo, by id, whether or not it is still on the registry.
///
/// For the reads that are about a Repo rather than about the list of them —
/// which branches it has, say — where the id came off a row the page was
/// already holding. `None` is a Repo that was never registered, which is a link
/// followed for one that never existed.
///
/// A Repo somebody took away is still found here, because this is how everything
/// already on one resolves it: a Conversation's Timeline goes on saying which
/// repository its work was done in, whatever the settings list is offering now.
/// Which is why this is the wrong read for anything that is about to *use* a
/// Repo — that question is [`registered_repo`]'s, and the two stand apart so
/// that neither has to guess which of them a caller meant.
pub async fn load_repo(pool: &SqlitePool, id: i64) -> Result<Option<Repo>> {
    let row: Option<(i64, String, String, String)> = sqlx::query_as(
        "SELECT id, path, name, default_branch
         FROM repos
         WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .with_context(|| format!("reading the Repo {id}"))?;

    Ok(row.map(|(id, path, name, default_branch)| Repo {
        id,
        path: PathBuf::from(path),
        name,
        default_branch,
    }))
}

/// One Repo that is on the registry, by id.
///
/// The read for everything about to *use* a Repo rather than to say which one
/// some work was already done in: opening its pane, and the questions asked in
/// front of the writes that put new work on it. `None` is a Repo nothing is
/// registered under — one that never was, and one somebody has taken away,
/// which are one answer here because neither is on offer.
///
/// Beside [`load_repo`] rather than a flag on it. The two are asked for opposite
/// reasons, and a caller made to pass a boolean would sooner or later pass the
/// wrong one — which is exactly how a Repo that had been taken away came to open
/// its own pane and be started on.
pub async fn registered_repo(pool: &SqlitePool, id: i64) -> Result<Option<Repo>> {
    let row: Option<(i64, String, String, String)> = sqlx::query_as(
        "SELECT id, path, name, default_branch
         FROM repos
         WHERE id = ? AND id NOT IN (SELECT repo_id FROM unregistered_repos)",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .with_context(|| format!("reading the registered Repo {id}"))?;

    Ok(row.map(|(id, path, name, default_branch)| Repo {
        id,
        path: PathBuf::from(path),
        name,
        default_branch,
    }))
}

/// A path as SQLite can hold it, which is UTF-8 or nothing.
///
/// A path the filesystem holds as bytes that are not UTF-8 cannot be stored
/// without being changed, and a stored path that is not the one on disk is a
/// boundary check that will pass for the wrong directory later. So it is
/// refused outright rather than written lossily.
///
/// Shared with [`crate::conversations`], which stores a worktree's path under
/// the same rule and for the same reason.
pub(crate) fn text(path: &Path) -> Result<&str> {
    path.to_str()
        .ok_or_else(|| anyhow!("the path {} is not valid UTF-8", path.display()))
}

/// Where every Repo on record is, taken away or not.
///
/// Not [`registered_repos`], which is the read everything *offering* Repos for
/// new work goes through — and a Repo the human took away is not on offer. This
/// is the other question, asked by the sweep of orphaned worktrees: where might
/// git still be holding a registration for a directory that has gone? An
/// unregistering leaves the repository exactly where it was, so its
/// registrations go stale like anybody else's, and skipping it would leave one
/// nothing ever prunes — git refusing to check that branch out anywhere later,
/// and the same path registering again bringing the same Repo back to be
/// refused in.
///
/// The paths alone, because pruning is all that is done with them and a name is
/// nothing git is asked.
pub async fn recorded_repos(pool: &SqlitePool) -> Result<Vec<PathBuf>> {
    let rows: Vec<(String,)> = sqlx::query_as("SELECT path FROM repos ORDER BY id")
        .fetch_all(pool)
        .await
        .context("listing where every Repo on record is")?;

    Ok(rows
        .into_iter()
        .map(|(path,)| PathBuf::from(path))
        .collect())
}

/// How a merge conflict on a Repo's pull request is to be resolved.
///
/// Two ways of putting the base branch's work on a branch that has diverged
/// from it, and the whole of the difference is what happens to the commits that
/// are already there: a merge leaves every one of them where it is, and a rebase
/// writes them again on top of the base and has to be force-pushed.
///
/// One enum for the two places the choice is written down — the word in
/// `config.yaml`, which is the global setting, and the word in the column below,
/// which is one Repo's override of it. The file and the column would otherwise
/// be two spellings of one idea, and a spelling that drifted would be a
/// Verkstead resolving a conflict the way nobody asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictResolution {
    /// Merge the base branch into the work branch. Nothing is rewritten and
    /// nothing is force-pushed, so whatever has been read or stacked on the
    /// branch goes on standing — which is why this is what nobody choosing
    /// anything gets.
    Merge,

    /// Rebase the work branch onto the base branch, and force-push what comes
    /// out. A tidier history, at the cost of rewriting what reviewers have
    /// already read and breaking anything stacked on the branch.
    Rebase,
}

impl ConflictResolution {
    /// The word the column holds, which is the word `config.yaml` holds too —
    /// lowercase and spelled out, so a database opened by hand says the same
    /// thing the settings file does.
    fn stored(self) -> &'static str {
        match self {
            Self::Merge => "merge",
            Self::Rebase => "rebase",
        }
    }

    /// The one a stored word names. An unknown word is a database written by a
    /// Verkstead this one does not understand, exactly as an unknown lifecycle
    /// state is.
    fn read(word: &str) -> Result<Self> {
        Ok(match word {
            "merge" => Self::Merge,
            "rebase" => Self::Rebase,
            other => bail!("a conflict is to be resolved by the unknown {other:?}"),
        })
    }
}

/// What one Repo overrides the global resolution strategy with, or `None` where
/// it overrides nothing — which is every Repo until somebody says otherwise.
///
/// `None` is *use whatever is configured globally* rather than *merge*: the two
/// are the same answer today and would stop being the same the moment the global
/// setting is changed, and a Repo that had quietly frozen the old global would
/// be a setting the human cannot see they have made.
///
/// Found for a Repo somebody has taken off the registry as well, the way
/// [`load_repo`] finds one: nothing new is worked in it, and a Conversation
/// already wrapping in it is still a Conversation whose conflicts have to be
/// resolved somehow.
pub async fn repo_resolution(
    pool: &SqlitePool,
    repo_id: i64,
) -> Result<Option<ConflictResolution>> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT strategy FROM repo_resolutions WHERE repo_id = ?")
            .bind(repo_id)
            .fetch_optional(pool)
            .await
            .with_context(|| format!("reading how the Repo {repo_id} resolves a conflict"))?;

    row.map(|(word,)| ConflictResolution::read(&word))
        .transpose()
}

/// Say how this Repo resolves a conflict, or take back what was said.
///
/// `None` removes the row rather than writing a word for the global answer:
/// what *use the global setting* means is that there is nothing here, and a row
/// holding today's global would go on holding it after the global moved.
pub async fn set_repo_resolution(
    pool: &SqlitePool,
    repo_id: i64,
    resolution: Option<ConflictResolution>,
) -> Result<()> {
    match resolution {
        Some(resolution) => sqlx::query(
            "INSERT INTO repo_resolutions (repo_id, strategy)
             VALUES (?, ?)
             ON CONFLICT (repo_id) DO UPDATE SET strategy = excluded.strategy",
        )
        .bind(repo_id)
        .bind(resolution.stored())
        .execute(pool)
        .await
        .with_context(|| format!("saying how the Repo {repo_id} resolves a conflict"))?,

        None => sqlx::query("DELETE FROM repo_resolutions WHERE repo_id = ?")
            .bind(repo_id)
            .execute(pool)
            .await
            .with_context(|| {
                format!("taking back how the Repo {repo_id} was told to resolve a conflict")
            })?,
    };

    Ok(())
}
