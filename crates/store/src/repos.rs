//! The Repos registered with Verkstead: the git repositories it has been told
//! about.
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
//! How a merge conflict on a Repo's pull requests is resolved is not one of
//! those facts. It was: a table of its own beside the registrations, holding
//! what one Repo said instead of what `config.yaml` says for every Repo at
//! once. Nothing offers that override any more, and one no page shows would be
//! a Repo rebasing with nowhere to read why — so the table is dropped on open
//! rather than left standing, and what resolves a conflict is the settings file
//! alone.

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

    /// The uuid of the Author its work is done as, or nothing where none was
    /// ever recorded (ADR-0025). Only the uuid: whether it names an Author
    /// that `config.yaml` still holds is the server's question, since the
    /// store knows nothing of that file.
    pub author: Option<String>,
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

    // And which Author each of them is, by uuid, one row apiece — beside the
    // registrations rather than a column on them, for the reason the flag above
    // is. Unregistering takes nothing out of it: the association is history,
    // and the same path registered again comes back as the Author it was.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS repo_authors (
             repo_id INTEGER PRIMARY KEY REFERENCES repos(id),
             author  TEXT NOT NULL
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the repo_authors table")?;

    // And the table one Repo's own resolution was kept in, which is dropped
    // here rather than created. Beside the creates because this is where the
    // schema is settled: an install that had saved an override would otherwise
    // carry a table nothing reads, and the point of taking the override out is
    // that there is nowhere left for it to be answering from.
    sqlx::query("DROP TABLE IF EXISTS repo_resolutions")
        .execute(pool)
        .await
        .context("dropping the repo resolutions table")?;

    Ok(())
}

/// Record a repository, which is expected to have been checked already: that it
/// is a git repository with a default branch is decided above the store.
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
    register_repo_as(pool, path, name, default_branch, None).await
}

/// [`register_repo`], recording the Repo as the Author with the uuid `author`.
///
/// **An Author already recorded is kept.** A revived Repo comes back as the
/// Author it had rather than the one this registration names, because the
/// association outlives the unregistering as history. `None` records nothing,
/// and leaves one already recorded where it is too.
///
/// One transaction with the registration, so that a Repo is never on the list
/// for a moment without the Author it was registered as.
pub async fn register_repo_as(
    pool: &SqlitePool,
    path: &Path,
    name: &str,
    default_branch: &str,
    author: Option<&str>,
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

    if let Some(author) = author {
        sqlx::query(
            "INSERT INTO repo_authors (repo_id, author) VALUES (?, ?)
             ON CONFLICT (repo_id) DO NOTHING",
        )
        .bind(id)
        .bind(author)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("recording the Author of the Repo at {}", path.display()))?;
    }

    let (author,): (Option<String>,) =
        sqlx::query_as("SELECT (SELECT author FROM repo_authors WHERE repo_id = ?)")
            .bind(id)
            .fetch_one(&mut *tx)
            .await
            .with_context(|| format!("reading the Author of the Repo at {}", path.display()))?;

    tx.commit()
        .await
        .with_context(|| format!("registering the Repo at {}", path.display()))?;

    Ok(Some(Repo {
        id,
        path: path.to_owned(),
        name: name.to_owned(),
        default_branch: default_branch.to_owned(),
        author,
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
/// What counts as live is [`super::work_on_repo`]'s reading — everything that is
/// neither Done nor Closed — so what refuses the removal is the same reading
/// every other count of the work on a Repo is made from, rather than a second
/// opinion about what "finished" means.
pub async fn unregister_repo(pool: &SqlitePool, id: i64) -> Result<Unregistering> {
    // Through the same read the list behind the press is drawn from, so that
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

/// What became of pointing a Repo at another Author.
///
/// Refused for the reason [`Unregistering::InUse`] is, and read the same way:
/// a branch half committed as one Author would carry on as another, so the
/// Author does not change under work that is neither Done nor Closed
/// (ADR-0025). Named rather than counted, because the refusal is put in front
/// of somebody who has to go and finish that work.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Repointing {
    /// Recorded. The next session on it is the new Author's.
    Repointed(Repo),

    /// There is no registered Repo with that id, one taken away included.
    NoSuchRepo,

    /// Work that is neither Done nor Closed has this Repo in it: the branches
    /// of the Conversations on it, and of the ones holding it as a Companion
    /// Repo, each oldest first. One of the two is never empty.
    InUse {
        on: Vec<String>,
        beside: Vec<String>,
    },
}

/// Record the Repo `id` as the Author with the uuid `author`, if no live work
/// has it in it.
///
/// **Live work beside it refuses too**, not only work on it: a Companion Repo
/// is committed in by the Conversation that admitted it, and that admission was
/// made against the Author it had. What counts as live is
/// [`super::work_on_repo`]'s reading, by way of [`super::Lifecycle::read`].
///
/// One transaction, the look and the write together, so a Conversation started
/// on the Repo between the two cannot find itself under an Author it was not
/// started as. Whether `author` is configured is not the store's to know, and
/// is asked above it.
pub async fn repoint_repo(pool: &SqlitePool, id: i64, author: &str) -> Result<Repointing> {
    let mut tx = super::writing(pool, "repointing a Repo").await?;

    let row: Option<RepoRow> = sqlx::query_as(
        "SELECT id, path, name, default_branch, author
         FROM repos LEFT JOIN repo_authors ON repo_authors.repo_id = repos.id
         WHERE id = ? AND id NOT IN (SELECT repo_id FROM unregistered_repos)",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await
    .with_context(|| format!("reading the registered Repo {id}"))?;

    let Some(row) = row else {
        return Ok(Repointing::NoSuchRepo);
    };

    let rows: Vec<(String, String, bool)> = sqlx::query_as(
        "SELECT COALESCE(c.named_branch, c.branch), c.state, c.repo_id = ?
         FROM conversations c
         WHERE c.repo_id = ?
            OR c.id IN (SELECT conversation_id FROM companions WHERE repo_id = ?)
         ORDER BY c.id",
    )
    .bind(id)
    .bind(id)
    .bind(id)
    .fetch_all(&mut *tx)
    .await
    .with_context(|| format!("reading the work the Repo {id} is in"))?;

    let (mut on, mut beside) = (Vec::new(), Vec::new());

    for (branch, state, own) in rows {
        match super::Lifecycle::read(&state)? {
            super::Lifecycle::Done | super::Lifecycle::Closed => {}
            _ if own => on.push(branch),
            _ => beside.push(branch),
        }
    }

    if !on.is_empty() || !beside.is_empty() {
        return Ok(Repointing::InUse { on, beside });
    }

    sqlx::query(
        "INSERT INTO repo_authors (repo_id, author) VALUES (?, ?)
         ON CONFLICT (repo_id) DO UPDATE SET author = excluded.author",
    )
    .bind(id)
    .bind(author)
    .execute(&mut *tx)
    .await
    .with_context(|| format!("recording the Author of the Repo {id}"))?;

    tx.commit()
        .await
        .with_context(|| format!("repointing the Repo {id}"))?;

    Ok(Repointing::Repointed(Repo {
        author: Some(author.to_owned()),
        ..repo_of(row)
    }))
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
    let rows: Vec<RepoRow> = sqlx::query_as(
        "SELECT id, path, name, default_branch, author
         FROM repos LEFT JOIN repo_authors ON repo_authors.repo_id = repos.id
         WHERE id NOT IN (SELECT repo_id FROM unregistered_repos)
         ORDER BY name, id",
    )
    .fetch_all(pool)
    .await
    .context("listing the registered Repos")?;

    Ok(rows.into_iter().map(repo_of).collect())
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
    let row: Option<RepoRow> = sqlx::query_as(
        "SELECT id, path, name, default_branch, author
         FROM repos LEFT JOIN repo_authors ON repo_authors.repo_id = repos.id
         WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .with_context(|| format!("reading the Repo {id}"))?;

    Ok(row.map(repo_of))
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
    let row: Option<RepoRow> = sqlx::query_as(
        "SELECT id, path, name, default_branch, author
         FROM repos LEFT JOIN repo_authors ON repo_authors.repo_id = repos.id
         WHERE id = ? AND id NOT IN (SELECT repo_id FROM unregistered_repos)",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .with_context(|| format!("reading the registered Repo {id}"))?;

    Ok(row.map(repo_of))
}

/// One Repo that is on the registry, by the path it is recorded under.
///
/// The read behind the one registration that changes nothing: a path already
/// registered is refused by the unique index rather than by looking first, and
/// the caller is then holding a refusal about a repository it has no id for.
/// This is that id, and it is asked *after* the write rather than before it, so
/// two tabs registering the same path still cannot both get past a look.
///
/// Registered rather than merely recorded, for [`registered_repo`]'s reason: a
/// path a flagged row holds is not this refusal at all — the upsert revives that
/// row — so a Repo found here is one on offer.
pub async fn registered_repo_at(pool: &SqlitePool, path: &Path) -> Result<Option<Repo>> {
    let stored = text(path)?;

    let row: Option<RepoRow> = sqlx::query_as(
        "SELECT id, path, name, default_branch, author
         FROM repos LEFT JOIN repo_authors ON repo_authors.repo_id = repos.id
         WHERE path = ? AND id NOT IN (SELECT repo_id FROM unregistered_repos)",
    )
    .bind(stored)
    .fetch_optional(pool)
    .await
    .with_context(|| format!("reading the registered Repo at {}", path.display()))?;

    Ok(row.map(repo_of))
}

/// One row of the Repo reads above, as SQLite hands it back.
type RepoRow = (i64, String, String, String, Option<String>);

fn repo_of((id, path, name, default_branch, author): RepoRow) -> Repo {
    Repo {
        id,
        path: PathBuf::from(path),
        name,
        default_branch,
        author,
    }
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
/// Written down in one place, which is the word in `config.yaml`: the setting
/// every Repo shares, and the whole of the answer about any of them. One enum
/// for it rather than one per reader, because a second spelling of the two
/// words would be a chance for a Verkstead to resolve a conflict the way nobody
/// asked for.
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
