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

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
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

/// The table the registrations live in.
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

    Ok(())
}

/// Record a repository, which is expected to have been checked already: that it
/// is inside a Watched Path, and that it is a git repository, is decided above
/// the store.
///
/// `None` means this path is registered already. Refused by the unique index
/// rather than by looking first, so two tabs cannot both get past the look.
pub async fn register_repo(
    pool: &SqlitePool,
    path: &Path,
    name: &str,
    default_branch: &str,
) -> Result<Option<Repo>> {
    let stored = text(path)?;

    let row: Option<(i64,)> = sqlx::query_as(
        "INSERT INTO repos (path, name, default_branch)
         VALUES (?, ?, ?)
         ON CONFLICT (path) DO NOTHING
         RETURNING id",
    )
    .bind(stored)
    .bind(name)
    .bind(default_branch)
    .fetch_optional(pool)
    .await
    .with_context(|| format!("registering the Repo at {}", path.display()))?;

    Ok(row.map(|(id,)| Repo {
        id,
        path: path.to_owned(),
        name: name.to_owned(),
        default_branch: default_branch.to_owned(),
    }))
}

/// Every registered Repo, by name.
///
/// Alphabetical rather than newest first, unlike the Set lists: a Repo is not
/// news, it is something to pick out of a list that barely changes, and the
/// name is what it is looked for by. The id breaks a tie between two
/// directories of the same name in different places.
pub async fn registered_repos(pool: &SqlitePool) -> Result<Vec<Repo>> {
    let rows: Vec<(i64, String, String, String)> = sqlx::query_as(
        "SELECT id, path, name, default_branch
         FROM repos
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

/// A path as SQLite can hold it, which is UTF-8 or nothing.
///
/// A path the filesystem holds as bytes that are not UTF-8 cannot be stored
/// without being changed, and a stored path that is not the one on disk is a
/// boundary check that will pass for the wrong directory later. So it is
/// refused outright rather than written lossily.
fn text(path: &Path) -> Result<&str> {
    path.to_str()
        .ok_or_else(|| anyhow!("the path {} is not valid UTF-8", path.display()))
}
