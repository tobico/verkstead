//! Where a Conversation's work is done: the branch it is on and the worktree it
//! is checked out in, made when grilling starts and removed when it is aborted.
//!
//! Worktrees live under Verkstead's own state directory rather than inside a
//! Watched Path. The Watched Paths are the boundary on what the *human* may
//! point Verkstead at — repositories to register, accounts to run under — and a
//! worktree is neither: it is something Verkstead made, in the one directory the
//! packaged unit is given to write. Putting it inside a Watched Path would mean
//! Verkstead's own scratch space appearing under a directory the human is also
//! working in.
//!
//! The branch is made in the Repo's own git directory, not in the worktree —
//! `git worktree add -b` does both at once, which is the point of asking git for
//! this rather than checking a tree out by hand. Aborting removes the worktree
//! and leaves the branch: a branch is a name and a commit, and it may hold work
//! worth reading; a worktree is a directory the human never asked to keep.

use std::path::{Path, PathBuf};

use crate::repos::git;

/// The directory a worktree goes in, under `state`.
///
/// Named for the Repo and the branch, which is what the human calls the work —
/// and what an agent working inside it sees itself working in. The branch may
/// hold slashes and two Repos may share a name, so neither is trusted as a path
/// component: everything that is not a plain name character becomes a hyphen,
/// and a name already taken falls back to one with the Conversation's id on the
/// end, which nothing else can collide with.
pub(crate) fn worktree_path(state: &Path, id: i64, repo: &str, branch: &str) -> PathBuf {
    let worktrees = state.join("worktrees");
    let stem = format!("{}-{}", component(repo), component(branch));

    let named = worktrees.join(&stem);

    // A directory that is already there is not one to check a branch out into,
    // whether it is another Conversation's or something else's entirely.
    if named.exists() {
        return worktrees.join(format!("{stem}-{id}"));
    }

    named
}

/// A string as one path component: the name characters kept, everything else a
/// hyphen.
///
/// Deliberately narrow. What comes through here is a directory name and a git
/// branch name, and while git already refuses most of what would be dangerous,
/// "most" is not the standard for something being turned into a path — a
/// separator that survived would put the worktree somewhere nobody named.
fn component(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => c,
            _ => '-',
        })
        .collect();

    // A leading dot cannot survive the mapping above, but an empty name can, and
    // a component of nothing would silently make the parent the worktree.
    match cleaned.trim_matches('-') {
        "" => "unnamed".to_owned(),
        trimmed => trimmed.to_owned(),
    }
}

/// Whether `repo` already has a branch by this name.
///
/// Asked before the worktree is made rather than read out of git's failure,
/// because it is the one way of failing that is the human's to fix and it wants
/// saying in those terms.
pub(crate) fn branch_exists(repo: &Path, branch: &str) -> bool {
    git(
        repo,
        &[
            "show-ref",
            "--verify",
            "--quiet",
            "--end-of-options",
            &format!("refs/heads/{branch}"),
        ],
    )
    .is_some()
}

/// The commit `named` resolves to in `repo`, in full, or `None` if nothing there
/// answers to it.
///
/// The same question [`crate::conversations`] asks when the human types a base
/// commit, asked again here: what resolved when they typed it may not resolve
/// now, and a default branch that has been renamed since resolves for the first
/// time at exactly this moment.
pub(crate) fn resolve(repo: &Path, named: &str) -> Option<String> {
    let commit = git(
        repo,
        &[
            "rev-parse",
            "--verify",
            "--quiet",
            "--end-of-options",
            &format!("{named}^{{commit}}"),
        ],
    )?;

    let commit = commit.trim();

    (!commit.is_empty()).then(|| commit.to_owned())
}

/// Make `branch` off `commit` in `repo`, checked out at `path`.
///
/// One git call, because it is one thing: a worktree registered with the branch
/// it holds. Doing it in two would leave a branch behind whenever the checkout
/// failed.
pub(crate) fn add(repo: &Path, path: &Path, branch: &str, commit: &str) -> bool {
    // The parent has to be there — git creates the worktree directory but not
    // the `worktrees/` above it, which on a fresh install has never existed.
    if let Some(parent) = path.parent()
        && let Err(error) = std::fs::create_dir_all(parent)
    {
        tracing::error!(error = ?error, path = %parent.display(), "making room for a worktree failed");
        return false;
    }

    git(
        repo,
        &[
            "worktree",
            "add",
            "-b",
            branch,
            "--end-of-options",
            &path.to_string_lossy(),
            commit,
        ],
    )
    .is_some()
}

/// Take the worktree at `path` away, and tell `repo` it has gone.
///
/// True when there is no longer a worktree there, which includes there never
/// having been one: aborting twice is not an error, and neither is aborting a
/// Conversation whose directory the human already deleted by hand.
///
/// `--force` because the whole point is to stop: a worktree with uncommitted
/// changes in it is the ordinary case for work being abandoned, and git refuses
/// to remove one without being told. What is being protected is the branch, and
/// that is not what this removes.
pub(crate) fn remove(repo: &Path, path: &Path) -> bool {
    let removed = git(
        repo,
        &[
            "worktree",
            "remove",
            "--force",
            "--end-of-options",
            &path.to_string_lossy(),
        ],
    )
    .is_some();

    // Git refuses to remove a worktree whose directory has already gone, and
    // that is the state this is trying to reach — so what settles it is the
    // filesystem rather than git's exit code. Pruning afterwards is what clears
    // the registration git is still holding for a directory that is not there.
    if !path.exists() {
        git(repo, &["worktree", "prune"]);
        return true;
    }

    removed && !path.exists()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_worktree_is_named_for_its_repo_and_its_branch() {
        let path = worktree_path(Path::new("/state"), 7, "verkstead", "rate-limiting");

        assert_eq!(
            path,
            Path::new("/state/worktrees/verkstead-rate-limiting"),
            "the name is what the human calls the work"
        );
    }

    /// A branch name is allowed slashes and a Repo is named by a directory, so
    /// neither can be dropped into a path as it stands.
    #[test]
    fn nothing_in_a_name_can_reach_out_of_the_worktrees_directory() {
        for (repo, branch) in [
            ("verkstead", "feature/rate-limiting"),
            ("verkstead", "../../etc"),
            ("..", ".."),
            ("a repo", "a branch"),
        ] {
            let path = worktree_path(Path::new("/state"), 7, repo, branch);

            assert_eq!(
                path.parent(),
                Some(Path::new("/state/worktrees")),
                "{repo}/{branch} escaped its directory as {}",
                path.display()
            );
            assert!(
                !path.to_string_lossy().contains(".."),
                "{repo}/{branch} kept a `..` as {}",
                path.display()
            );
        }
    }

    #[test]
    fn a_slash_in_a_branch_name_becomes_one_readable_component() {
        assert_eq!(
            worktree_path(Path::new("/state"), 7, "verkstead", "feature/rate-limiting"),
            Path::new("/state/worktrees/verkstead-feature-rate-limiting")
        );
    }

    /// Two Repos of one name in different places are two Repos, and two
    /// Conversations on one branch name are two pieces of work. Whichever asks
    /// second gets a name of its own rather than the other's directory.
    #[test]
    fn a_name_already_taken_falls_back_to_one_carrying_the_conversation() {
        let state = tempfile::tempdir().unwrap();
        let taken = state.path().join("worktrees/verkstead-rate-limiting");
        std::fs::create_dir_all(&taken).unwrap();

        assert_eq!(
            worktree_path(state.path(), 7, "verkstead", "rate-limiting"),
            state.path().join("worktrees/verkstead-rate-limiting-7")
        );
    }

    /// A name that is nothing but separators would otherwise come out empty, and
    /// an empty component makes the parent directory the worktree.
    #[test]
    fn a_name_with_nothing_usable_in_it_still_makes_a_component() {
        let path = worktree_path(Path::new("/state"), 7, "///", "...");

        assert_eq!(path, Path::new("/state/worktrees/unnamed-unnamed"));
    }
}
