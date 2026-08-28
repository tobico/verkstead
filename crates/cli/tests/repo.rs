//! What the CLI derives from the working directory instead of trusting the
//! agent for it: the project and the branch.
//!
//! The Diff is no longer among them — the server reads it off the Worktree the
//! Set was asked from, and it is tested there.

mod support;

use support::{REPO_NAME, linked_worktree, repo_with_a_commit};
use verkstead_cli::repo;

#[test]
fn a_linked_worktree_reports_the_root_repos_name() {
    let tmp = tempfile::tempdir().unwrap();
    let root = repo_with_a_commit(tmp.path());
    let linked = linked_worktree(&root, "feature");

    assert_eq!(
        repo::project(&linked).as_deref(),
        Some(REPO_NAME),
        "a linked worktree belongs to the root repo, and reports its name"
    );
    assert_eq!(repo::branch(&linked).as_deref(), Some("feature"));
}

#[test]
fn the_main_worktree_reports_the_same_name_from_any_depth() {
    let tmp = tempfile::tempdir().unwrap();
    let root = repo_with_a_commit(tmp.path());
    let deep = root.join("crates/cli/src");
    std::fs::create_dir_all(&deep).unwrap();

    assert_eq!(repo::project(&root).as_deref(), Some(REPO_NAME));
    assert_eq!(repo::project(&deep).as_deref(), Some(REPO_NAME));
    assert_eq!(repo::branch(&root).as_deref(), Some("main"));
}

#[test]
fn outside_a_repository_nothing_is_derived() {
    let tmp = tempfile::tempdir().unwrap();

    assert_eq!(repo::enrichment(tmp.path()), repo::Enrichment::default());
}
