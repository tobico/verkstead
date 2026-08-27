//! The Diff a Question Set carries: the uncommitted changes of every Worktree
//! the Set was asked from, read here rather than sent.
//!
//! The server knows which Conversation a Set arrives from without inferring it —
//! the endpoint is conversation-scoped, because that is the base URL the sandbox
//! was given — and it knows where that Conversation was checked out, its
//! companions along with its own. So the Diff is composed from its own read of
//! those Worktrees, one block per repository, and whatever the Set claimed is
//! overwritten.
//!
//! One block per repository a session can write in: the Conversation's own
//! first, then each read-write companion. A read-only companion is checked out
//! detached and bound read-only, so there is nothing uncommitted in it to find.
//! Uncommitted only, throughout — committed work is on the Timeline as Events
//! already — and a repository with a clean Worktree contributes no block at all.
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

use std::path::{Path, PathBuf};

use verkstead_schema::RepoDiff;

use sqlx::SqlitePool;

use crate::repos::{accepting, git};
use crate::store;

/// What to attach to a Set asked from `conversation_id`, one block per
/// repository with something uncommitted in it.
///
/// Empty where there is nothing to attach: every Worktree clean, a Conversation
/// with none — nothing has been checked out, or it has been closed — or
/// repositories that will not answer. None of those refuses the Set: a Question
/// is worth putting to the human whether or not there is code beside it.
pub(crate) async fn compose(pool: &SqlitePool, conversation_id: i64) -> Vec<RepoDiff> {
    let conversation = match store::load_conversation(pool, conversation_id).await {
        Ok(Some(conversation)) => conversation,
        Ok(None) => return Vec::new(),
        Err(error) => {
            tracing::error!(
                error = ?error,
                conversation_id,
                "reading a Conversation to compose its Set's Diff failed"
            );
            return Vec::new();
        }
    };

    let read = writable(&conversation);

    match tokio::task::spawn_blocking(move || blocks(read)).await {
        Ok(diffs) => diffs,
        Err(error) => {
            tracing::error!(
                error = ?error,
                conversation_id,
                "reading a Worktree's uncommitted changes failed"
            );
            Vec::new()
        }
    }
}

/// Every Worktree a session running for this Conversation can leave uncommitted
/// work in, named by the Repo it is a checkout of: the Conversation's own first,
/// then each read-write companion in the order the Conversation carries them.
///
/// A repository with no Worktree is not among them — a Conversation before
/// grilling or after closing has none, and neither has a companion of one, which
/// is the ordinary state rather than a missing record. A read-only companion is
/// left out for what it is: detached and bound read-only, with nothing to
/// commit and so nothing uncommitted.
///
/// Each is marked for whether it is the Conversation's own repository, which is
/// carried onto the block: a clean Worktree contributes none, so where the
/// work's own repository comes in the list is not something a reader of the
/// blocks could work out for itself.
fn writable(conversation: &store::Conversation) -> Vec<Reading> {
    let mut worktrees = Vec::new();

    if let Some(worktree) = conversation.worktree.clone() {
        worktrees.push(Reading {
            repo: conversation.repo.name.clone(),
            own: true,
            worktree,
        });
    }

    for companion in &conversation.companions {
        if companion.mode != store::CompanionMode::ReadWrite {
            continue;
        }

        if let Some(worktree) = companion.worktree.clone() {
            worktrees.push(Reading {
                repo: companion.repo.name.clone(),
                own: false,
                worktree,
            });
        }
    }

    worktrees
}

/// One Worktree to read, and what the block read out of it is to say for itself.
struct Reading {
    repo: String,
    own: bool,
    worktree: PathBuf,
}

/// The blocks those Worktrees come to, in the order they were given. Blocking,
/// which is why it is called from a worker of its own.
fn blocks(worktrees: Vec<Reading>) -> Vec<RepoDiff> {
    worktrees
        .into_iter()
        .filter_map(|read| {
            uncommitted(&read.worktree).map(|diff| RepoDiff {
                repo: read.repo,
                own: read.own,
                diff,
            })
        })
        .collect()
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
