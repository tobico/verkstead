//! The Diff a Question Set carries: the uncommitted changes of the Worktree the
//! Set was asked from, read here rather than sent.
//!
//! The server knows which Conversation a Set arrives from without inferring it —
//! the endpoint is conversation-scoped, because that is the base URL the sandbox
//! was given — and it knows where that Conversation was checked out. So the
//! Diff is composed from its own read of that Worktree, and whatever the Set
//! claimed is overwritten.
//!
//! Determinism over trust (ADR-0001), strengthened rather than bent: the field
//! was already never the agent's to fill, and the authority for it moves from a
//! CLI running wherever the agent happened to be standing to the host that can
//! see the Worktree itself.
//!
//! Reading git blocks, so it happens off the async worker the request is being
//! served on. The ask is on the critical path of a session that is waiting, so
//! what is run is the repository's own cheap read and nothing more — and through
//! [`crate::repos`], which passes `--no-optional-locks`: the agent that is
//! waiting on this Set may hold `index.lock`, and a reader that waited on one
//! would be waiting on itself.

use std::path::Path;

use sqlx::SqlitePool;

use crate::repos::{accepting, git};
use crate::store;

/// What to attach to a Set asked from `conversation_id`.
///
/// `None` where there is nothing to attach: a clean Worktree, a Conversation
/// with none — nothing has been checked out, or it has been closed — or a
/// repository that will not answer. None of those refuses the Set: a Question
/// is worth putting to the human whether or not there is code beside it.
pub(crate) async fn compose(pool: &SqlitePool, conversation_id: i64) -> Option<String> {
    let read = match store::load_conversation(pool, conversation_id).await {
        Ok(conversation) => conversation?.worktree?,
        Err(error) => {
            tracing::error!(
                error = ?error,
                conversation_id,
                "reading a Conversation to compose its Set's Diff failed"
            );
            return None;
        }
    };

    match tokio::task::spawn_blocking(move || uncommitted(&read)).await {
        Ok(diff) => diff,
        Err(error) => {
            tracing::error!(
                error = ?error,
                conversation_id,
                "reading a Worktree's uncommitted changes failed"
            );
            None
        }
    }
}

/// A Worktree's uncommitted changes: everything not in the last commit, staged
/// or not, plus the contents of untracked files. Binary contents are left out —
/// git says the file differs and stops there.
///
/// `None` when the tree is clean, or when `worktree` is not a directory git will
/// answer about.
fn uncommitted(worktree: &Path) -> Option<String> {
    // Tracked changes, staged and unstaged, in one patch. Before the first
    // commit there is no HEAD to compare against and the index stands in.
    let mut diff =
        git(worktree, &["diff", "HEAD"]).or_else(|| git(worktree, &["diff", "--cached"]))?;

    // An untracked file has nothing to diff against, so it is compared with the
    // empty file. `--no-index` exits 1 when the two differ, which for a
    // non-empty untracked file is the ordinary case.
    for path in untracked(worktree) {
        if let Some(patch) = accepting(
            worktree,
            &["diff", "--no-index", "--", "/dev/null", &path],
            &[0, 1],
        ) {
            diff.push_str(&patch);
        }
    }

    (!diff.is_empty()).then_some(diff)
}

/// The paths of files git knows nothing about, ignores aside, relative to
/// `worktree`. NUL-separated so a path with a newline in it survives the trip.
fn untracked(worktree: &Path) -> Vec<String> {
    git(
        worktree,
        &["ls-files", "-z", "--others", "--exclude-standard"],
    )
    .unwrap_or_default()
    .split('\0')
    .filter(|path| !path.is_empty())
    .map(str::to_owned)
    .collect()
}
