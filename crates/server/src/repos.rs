//! Registering a Repo: everything between a path the human typed and a row in
//! the store — and taking one off the registry again, which is the store's own
//! and passes straight through.
//!
//! Three things have to be true, and each of them is checked here rather than in
//! the browser: the path is inside a Watched Path once resolved, it is the root
//! of a git repository, and that repository can say which branch it works from.
//! A form that checked any of them would be a courtesy — this endpoint is
//! reachable without one, and the boundary is the server's.
//!
//! Git is shelled out to rather than linked, as it is in the CLI: the answers
//! are one-liners, and what git says about a repository on this machine is what
//! the sandboxed sessions will see later when they run it themselves.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::Result;
use sqlx::SqlitePool;
use verkstead_render::{Registered, RepoRemoved, RepoView};

use crate::store;
use crate::watched::{Admission, WatchedPaths};

/// Register the repository at `asked`, or say why not.
///
/// The filesystem half runs off the runtime: resolving a path and asking git
/// about a repository are both blocking, and a registration is rare enough that
/// the thread it borrows costs nothing.
pub(crate) async fn register(
    pool: &SqlitePool,
    watched: &WatchedPaths,
    asked: &str,
) -> Result<Registered> {
    let watched = watched.clone();
    let asked = PathBuf::from(asked);

    let facts = match tokio::task::spawn_blocking(move || inspect(&watched, &asked)).await? {
        Ok(facts) => facts,
        Err(refusal) => return Ok(refusal),
    };

    Ok(
        match store::register_repo(pool, &facts.path, &facts.name, &facts.default_branch).await? {
            Some(_) => Registered::Added,
            None => Registered::AlreadyRegistered,
        },
    )
}

/// What the store needs to record a Repo, read off the repository itself rather
/// than claimed by whoever registered it.
struct Facts {
    path: PathBuf,
    name: String,
    default_branch: String,
}

/// Everything the filesystem and git have to say about a path someone wants
/// registered — or the reason it is not going to be.
fn inspect(watched: &WatchedPaths, asked: &Path) -> Result<Facts, Registered> {
    let path = match watched.admit(asked) {
        Admission::Inside(path) => path,
        Admission::NotAbsolute => return Err(Registered::NotAbsolute),
        Admission::Missing => return Err(Registered::Missing),
        Admission::Outside => return Err(Registered::OutsideWatchedPaths),
    };

    if !is_repository_root(&path) {
        return Err(Registered::NotARepository);
    }

    let Some(default_branch) = default_branch(&path) else {
        return Err(Registered::NoDefaultBranch);
    };

    Ok(Facts {
        name: name(&path),
        path,
        default_branch,
    })
}

/// Whether `path` is the top of a worktree rather than somewhere inside one.
///
/// The root and not merely "in a repository": a Conversation's worktree is added
/// from the repository, and registering `…/repo/src` would leave every path
/// Verkstead later builds hanging off a directory that is not the repository at
/// all. Both sides are resolved before they are compared, because git answers
/// with the path the filesystem means and the caller may not have.
fn is_repository_root(path: &Path) -> bool {
    git(path, &["rev-parse", "--show-toplevel"])
        .map(|top| PathBuf::from(top.trim()))
        .and_then(|top| top.canonicalize().ok())
        .is_some_and(|top| top == path)
}

/// Every branch of a registered Repo, for the dropdown a Conversation picks the
/// one it comes off out of.
///
/// `None` is a Repo that is not registered. The reading itself is git's — see
/// [`crate::worktrees::branches`] — and it runs off the runtime, because a
/// repository with a great many refs is not a quick call.
pub(crate) async fn branches(pool: &SqlitePool, id: i64) -> Result<Option<Vec<String>>> {
    let Some(repo) = store::load_repo(pool, id).await? else {
        return Ok(None);
    };

    Ok(Some(
        tokio::task::spawn_blocking(move || crate::worktrees::branches(&repo.path)).await?,
    ))
}

/// One registered Repo opened: everything its card cannot hold.
///
/// `None` is a Repo that is not registered, which the pane reads as the repo
/// being gone — a link followed after somebody took it away.
///
/// The two filesystem reads go together in one blocking task rather than one
/// apiece: they are both git against the same directory, and a pane is one thing
/// the human opened rather than two. The counts are the store's and are awaited
/// beside them.
///
/// Nothing here is stored but the three facts the card already carries. The
/// branches move without Verkstead hearing about it and a roadmap somebody picks
/// up stops being abandoned the moment they do, so both are asked afresh every
/// time the pane is opened — a kept copy would be a second opinion that went
/// wrong on somebody else's push.
pub(crate) async fn opened(pool: &SqlitePool, id: i64) -> Result<Option<RepoView>> {
    let Some(repo) = store::load_repo(pool, id).await? else {
        return Ok(None);
    };

    let work = store::work_on_repo(pool, id).await?;

    let read = repo.clone();
    let (branches, roadmaps) = tokio::task::spawn_blocking(move || {
        (
            crate::worktrees::branches(&read.path),
            crate::stages::waiting(&read),
        )
    })
    .await?;

    Ok(Some(RepoView {
        id: repo.id,
        name: repo.name,
        // Stored as UTF-8 in the first place — a path that is not cannot be
        // registered — so nothing is lost putting it back on the wire.
        path: repo.path.to_string_lossy().into_owned(),
        default_branch: repo.default_branch,
        branches,
        live: work.live,
        finished: work.finished,
        roadmaps,
    }))
}

/// Take a Repo off the registry, if nothing live is being worked in it.
///
/// An unregistering rather than a delete, and the whole of that is the store's —
/// see [`store::unregister_repo`]. Nothing here touches the repository itself:
/// what Verkstead is being told is that it may stop offering it, and the
/// directory is the human's either way.
pub(crate) async fn remove(pool: &SqlitePool, id: i64) -> Result<RepoRemoved> {
    Ok(match store::unregister_repo(pool, id).await? {
        store::Unregistering::Unregistered => RepoRemoved::Removed,
        store::Unregistering::NoSuchRepo => RepoRemoved::NoSuchRepo,
        store::Unregistering::InUse => RepoRemoved::InUse,
    })
}

/// The branch a Conversation branches from unless it is told otherwise.
///
/// What the remote calls its default, where there is a remote to ask — that is
/// what "default branch" means to everyone who works on the repository, and it
/// does not move when somebody leaves another branch checked out. Failing that,
/// whatever is checked out here.
///
/// `None` on a detached HEAD with no remote to fall back on: there is no branch
/// to name, and naming one anyway would put work on a branch nobody chose.
fn default_branch(path: &Path) -> Option<String> {
    let remote = git(
        path,
        &["symbolic-ref", "--short", "refs/remotes/origin/HEAD"],
    )
    .and_then(|head| Some(head.trim().strip_prefix("origin/")?.to_owned()));

    remote
        .or_else(|| {
            git(path, &["symbolic-ref", "--short", "HEAD"]).map(|head| head.trim().to_owned())
        })
        .filter(|branch| !branch.is_empty())
}

/// What to call the repository in a list: the directory's own name, which is
/// what the human calls it.
///
/// A directory with no name of its own is the filesystem root, which nobody is
/// registering; its whole path stands in rather than leaving a row blank.
fn name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

/// Run git in `dir` and take its stdout, or `None` if it failed.
///
/// Shared with [`crate::conversations`], which asks git the two questions a
/// Conversation raises — whether a name is one it would take for a branch, and
/// what commit something resolves to.
pub(crate) fn git(dir: &Path, args: &[&str]) -> Option<String> {
    accepting(dir, args, &[0])
}

/// The same run, accepting any of the `ok` exit codes rather than success alone.
///
/// There is one read here that is not a failure when it exits non-zero:
/// `git diff --no-index` exits 1 when the two files differ, which for the
/// untracked file [`crate::diffs`] asks it about is the ordinary case.
pub(crate) fn accepting(dir: &Path, args: &[&str], ok: &[i32]) -> Option<String> {
    let output = Command::new("git")
        // Reading a repository should never take a lock on it: an agent may well
        // be working in this one right now.
        .arg("--no-optional-locks")
        .args(args)
        .current_dir(dir)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;

    if !ok.contains(&output.status.code()?) {
        return None;
    }

    // Paths and patches are whatever bytes the filesystem holds; a Set is UTF-8
    // either way, so anything else is replaced rather than refused.
    Some(String::from_utf8_lossy(&output.stdout).into_owned())
}
