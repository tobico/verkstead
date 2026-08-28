//! What the CLI derives from the working directory: the project and the branch.
//!
//! Agents never supply these — determinism over trust (ADR-0001). Everything
//! here shells out to `git` rather than linking a git library: the answers are
//! one-liners, and shelling out means the CLI sees exactly what the agent sees
//! when it runs git itself.
//!
//! The Diff was derived here too, until the server took it: it knows which
//! Conversation a Set is asked from and can read that Worktree itself, so the
//! whole of it is composed in one place rather than half of it here. What is
//! left is what only the working directory can answer.
//!
//! Nothing here fails: outside a repository, or with no git on the PATH, each
//! field is simply absent. A Question Set is worth putting to the human either
//! way.

use std::ffi::OsStr;
use std::path::Path;
use std::process::{Command, Stdio};

/// The fields the CLI fills in on every Set. Both are absent outside a git
/// repository.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Enrichment {
    pub project: Option<String>,
    pub branch: Option<String>,
}

/// Derive everything the CLI attaches to a Set, as seen from `dir`.
pub fn enrichment(dir: &Path) -> Enrichment {
    Enrichment {
        project: project(dir),
        branch: branch(dir),
    }
}

/// The repository's name, worktree-smart: from a linked worktree this is the
/// root repo's name, because that is the project the human recognises.
///
/// `--git-common-dir` is the root repo's `.git` however deep, and however
/// linked, the working directory is — unlike `--git-dir`, which in a linked
/// worktree points inside `.git/worktrees/`.
pub fn project(dir: &Path) -> Option<String> {
    let common = git(dir, &["rev-parse", "--git-common-dir"])?;

    // The path comes back relative to `dir` in the main worktree and absolute
    // in a linked one; joining and canonicalising settles both, and resolves
    // the `..` in the `../.git` a subdirectory reports.
    let common = dir.join(common.trim()).canonicalize().ok()?;

    let name = match common.file_name().and_then(OsStr::to_str)? {
        // The ordinary layout: the repository is `.git`'s parent.
        ".git" => common.parent()?.file_name().and_then(OsStr::to_str)?,
        // A bare or separate-git-dir repository is named by the directory
        // itself, conventionally `<name>.git`.
        directory => directory.strip_suffix(".git").unwrap_or(directory),
    };

    Some(name.to_owned())
}

/// The branch checked out where the CLI is running. A detached HEAD has no
/// branch to report, so the commit stands in — the human still needs to know
/// what the agent was looking at.
pub fn branch(dir: &Path) -> Option<String> {
    let current = git(dir, &["branch", "--show-current"])?;
    if !current.trim().is_empty() {
        return Some(current.trim().to_owned());
    }

    let commit = git(dir, &["rev-parse", "--short", "HEAD"])?;
    Some(commit.trim().to_owned()).filter(|commit| !commit.is_empty())
}

/// Run git in `dir` and take its stdout, or `None` if it failed.
fn git(dir: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        // Reading the working tree should never take a lock: the CLI is on its
        // way to submitting a Question Set, not driving git.
        .arg("--no-optional-locks")
        .args(args)
        .current_dir(dir)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    // A name is whatever bytes the filesystem holds; the Set is UTF-8 either
    // way, so anything else is replaced rather than refused.
    Some(String::from_utf8_lossy(&output.stdout).into_owned())
}
