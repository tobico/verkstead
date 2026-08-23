//! Watching a Conversation's branch, and what each commit on it becomes.
//!
//! Commits are the only visible product of unattended execution. Verkstead
//! launches the sessions but does not drive them — there is no hook to be told
//! *a commit happened*, and asking the agent to report one would be asking it to
//! be honest about the one thing that can't be half done. So the branch is
//! watched: swept every few seconds while a session is running, and once more as
//! it ends.
//!
//! The sweep asks the repository rather than the worktree, and never takes a
//! lock. Everything here goes through [`crate::repos::git`], which passes
//! `--no-optional-locks` — a session is committing in this repository while this
//! is reading it, and a reader that took `index.lock` would be a reader that
//! made the agent's own `git commit` fail.
//!
//! A branch is swept whole rather than followed from where the last sweep got
//! to. What makes that cheap is the store: it already knows which commits are on
//! the Timeline, so the reading of git that costs anything — a subject and a set
//! of counts per commit — happens only for the ones that are not. And what makes
//! it *correct* is the same thing, because a branch is not a queue: one that was
//! amended, reset or rebased has commits before its tip that no sweep has seen.

use std::path::{Path, PathBuf};
use std::time::Duration;

use sqlx::SqlitePool;
use tokio::sync::oneshot;
use verkstead_schema::Nudge;

use crate::nudge::Nudges;
use crate::repos::git;
use crate::store;

/// How often a branch is looked at while a session is running.
///
/// Commits arrive minutes apart at best, so this is not a race to notice one —
/// it is how long the human waits to see work they can already read in the
/// Capture. Two seconds costs a handful of short git reads a minute and is
/// under what anyone reads as a delay.
const SWEEP_EVERY: Duration = Duration::from_secs(2);

/// Follow `conversation`'s branch until `stopping` says the session is over,
/// putting every commit that lands on it on the Timeline.
///
/// Swept once before the waiting starts and once after it ends. The first is for
/// whatever landed while nothing was watching — a session that committed as the
/// server was restarted, or a Conversation whose last session ended badly. The
/// last is the one that matters most: a session's final act is usually a commit,
/// and a watcher that stopped when the process did would miss it by a poll.
pub(crate) async fn watch(
    pool: SqlitePool,
    nudges: Nudges,
    conversation_id: i64,
    repo: PathBuf,
    branch: String,
    base: String,
    mut stopping: oneshot::Receiver<()>,
) {
    let branch = Branch { repo, branch, base };

    sweep(&pool, &nudges, conversation_id, &branch).await;

    loop {
        tokio::select! {
            _ = tokio::time::sleep(SWEEP_EVERY) => {
                sweep(&pool, &nudges, conversation_id, &branch).await;
            }
            _ = &mut stopping => break,
        }
    }

    sweep(&pool, &nudges, conversation_id, &branch).await;
}

/// Where a Conversation's commits are read from: the repository, the branch the
/// work is on, and the commit it started from.
///
/// The repository rather than the worktree, and not only because a worktree can
/// be removed while its branch lives on. The refs are the repository's — a
/// worktree shares them — so this is asking the thing that actually knows.
#[derive(Debug, Clone)]
struct Branch {
    repo: PathBuf,
    branch: String,

    /// What the work branched from. It is the far end of the range, so it is
    /// what stops the whole history of the default branch arriving as this
    /// Conversation's commits.
    base: String,
}

/// Take one look at the branch, and record whatever is on it that is not on the
/// Timeline yet.
///
/// Nothing here is refused for. A repository that will not answer, a branch that
/// has gone, a store that will not take a row: each is logged and the next sweep
/// tries again. What is being watched is a side effect of work happening
/// elsewhere, and a watcher that gave up the first time git was busy would be
/// one that stopped watching without anybody noticing.
async fn sweep(pool: &SqlitePool, nudges: &Nudges, conversation_id: i64, branch: &Branch) {
    let recorded = match store::recorded_commits(pool, conversation_id).await {
        Ok(recorded) => recorded,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading a Conversation's commits failed");
            return;
        }
    };

    let landed = {
        let branch = branch.clone();
        match tokio::task::spawn_blocking(move || since(&branch, &recorded)).await {
            Ok(landed) => landed,
            Err(error) => {
                tracing::error!(error = ?error, conversation_id, "sweeping a branch failed");
                return;
            }
        }
    };

    // Nothing new, which is what nearly every sweep finds: no store write, and
    // nobody told the world moved.
    if landed.is_empty() {
        return;
    }

    let mut recorded_any = false;

    for commit in landed {
        match store::record_commit(pool, conversation_id, &commit).await {
            Ok(Some(event_id)) => {
                tracing::info!(
                    conversation_id,
                    event_id,
                    sha = commit.sha,
                    "a commit landed on the Timeline"
                );
                recorded_any = true;
            }
            // Recorded by another sweep between the read above and this write.
            // One watcher per Conversation makes that unlikely rather than
            // impossible, and the point of the store's unique index is that
            // being wrong about it costs nothing.
            Ok(None) => {}
            Err(error) => {
                tracing::error!(error = ?error, conversation_id, sha = commit.sha, "recording a commit failed");
                // Stopped rather than skipped: the commits after this one are
                // later on the branch, and recording them now would put the
                // Timeline out of order for good. The next sweep starts again
                // from here.
                break;
            }
        }
    }

    if recorded_any {
        nudges.announce(Nudge::Commit {
            conversation: conversation_id,
        });
    }
}

/// Every commit on the branch that is not among `recorded`, oldest first.
///
/// The whole of the branch is listed and then filtered, rather than asked for
/// as *what is new*: git has no way to answer the second question, and the
/// answer to the first is a list of hashes that costs nothing to read. What
/// costs something is describing a commit, and that happens only for the ones
/// left over.
///
/// Blocking, like everything that shells out to git.
fn since(branch: &Branch, recorded: &[String]) -> Vec<store::Commit> {
    let Some(listed) = git(
        &branch.repo,
        &[
            "rev-list",
            "--reverse",
            "--end-of-options",
            &format!("{}..{}", branch.base, branch.branch),
        ],
    ) else {
        tracing::warn!(
            repo = %branch.repo.display(),
            branch = branch.branch,
            "the repository would not list what is on this branch"
        );
        return Vec::new();
    };

    listed
        .lines()
        .map(str::trim)
        .filter(|sha| !sha.is_empty())
        .filter(|sha| !recorded.iter().any(|known| known == sha))
        .filter_map(|sha| describe(&branch.repo, sha))
        .collect()
}

/// What one commit is, as its Timeline row says it: the subject git recorded,
/// and how much of the repository it moved.
///
/// Two reads rather than one that says both. `--numstat` and a format string can
/// be asked for together, but then the counts arrive underneath a header that
/// has to be told apart from them — and the counts are the half that has to be
/// parsed exactly.
///
/// `None` where the repository will not say, which is a commit that has gone
/// between being listed and being asked about.
fn describe(repo: &Path, sha: &str) -> Option<store::Commit> {
    let subject = git(
        repo,
        &["show", "--no-patch", "--format=%s", "--end-of-options", sha],
    )?;

    let counted = git(
        repo,
        &[
            "diff-tree",
            "--no-commit-id",
            "--numstat",
            // The one flag that makes a repository's first commit describable:
            // it has no parent to be compared against, and without this git
            // compares it with nothing and says nothing changed.
            "--root",
            "--end-of-options",
            sha,
        ],
    )?;

    let mut files = 0;
    let mut insertions = 0;
    let mut deletions = 0;

    for line in counted.lines().filter(|line| !line.trim().is_empty()) {
        let mut columns = line.split('\t');

        // Added, removed, path. A binary file has `-` for both counts, which
        // parses as nothing and counts as a file — which is what it is.
        let added = columns.next().unwrap_or_default();
        let removed = columns.next().unwrap_or_default();

        files += 1;
        insertions += added.parse::<i64>().unwrap_or(0);
        deletions += removed.parse::<i64>().unwrap_or(0);
    }

    Some(store::Commit {
        sha: sha.to_owned(),
        subject: subject.trim_end_matches('\n').to_owned(),
        files,
        insertions,
        deletions,
    })
}

/// One commit's diff, as the renderer takes it: the patch alone, with no commit
/// header above it.
///
/// Headerless on purpose. The renderer splits a diff on `diff --git`, so
/// anything ahead of the first file would be dropped rather than shown — which
/// is why what a commit was called comes off its Event instead. `diff-tree` says
/// only the patch, where `git show` says the message too.
///
/// `--root` for the reason [`describe`] has it: a repository's first commit has
/// no parent, and without this git compares it against nothing.
///
/// `None` where the repository will not say — a commit that has been garbage
/// collected, or a repository that has moved out from under Verkstead.
pub(crate) fn patch(repo: &Path, sha: &str) -> Option<String> {
    git(
        repo,
        &[
            "diff-tree",
            "--no-commit-id",
            "-p",
            "--root",
            "--end-of-options",
            sha,
        ],
    )
}

#[cfg(test)]
mod tests {
    use std::process::{Command, Stdio};

    use super::*;

    /// A repository with one commit on `main`, and the tools to add more.
    fn repository() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path();

        run(path, &["init", "--initial-branch", "main"]);
        run(path, &["config", "user.email", "test@verkstead.invalid"]);
        run(path, &["config", "user.name", "Verkstead Test"]);

        std::fs::write(path.join("README.md"), "# a repository\n").unwrap();
        run(path, &["add", "README.md"]);
        run(path, &["commit", "-m", "first"]);

        dir
    }

    fn run(dir: &Path, args: &[&str]) -> String {
        let output = Command::new("git")
            .args(args)
            .current_dir(dir)
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .expect("git should be on the PATH for these tests");

        assert!(output.status.success(), "git {args:?} failed");

        String::from_utf8(output.stdout).unwrap()
    }

    fn head(dir: &Path) -> String {
        run(dir, &["rev-parse", "HEAD"]).trim().to_owned()
    }

    #[test]
    fn a_commit_is_described_by_its_subject_and_what_it_moved() {
        let dir = repository();
        let path = dir.path();

        std::fs::write(path.join("README.md"), "# a repository\n\nWith words.\n").unwrap();
        std::fs::write(
            path.join("limiter.rs"),
            "fn allow() -> bool {\n    true\n}\n",
        )
        .unwrap();
        run(path, &["add", "-A"]);
        run(path, &["commit", "-m", "feat: rate limiting\n\nAnd why."]);

        let described = describe(path, &head(path)).expect("the commit is right there");

        assert_eq!(
            described.subject, "feat: rate limiting",
            "the first line of the message, and not the rest of it",
        );
        assert_eq!(described.files, 2);
        assert_eq!(described.insertions, 5);
        assert_eq!(described.deletions, 0);
    }

    /// A repository's first commit has no parent to be compared against, and
    /// there is no reason the human should see an empty row where the whole
    /// beginning of a repository is.
    #[test]
    fn a_root_commit_is_described_and_renders() {
        let dir = repository();
        let path = dir.path();
        let root = head(path);

        let described = describe(path, &root).expect("a root commit is still a commit");

        assert_eq!(described.subject, "first");
        assert_eq!(described.files, 1);
        assert_eq!(described.insertions, 1);

        let patch = patch(path, &root).expect("and it still has a patch");

        assert!(
            patch.starts_with("diff --git"),
            "the patch arrives headerless, which is what the renderer takes: {patch:?}",
        );
        assert!(patch.contains("+# a repository"));
        assert!(
            verkstead_render::commit_diff(&patch).diff.is_some(),
            "and it renders",
        );
    }

    /// The patch is what the renderer splits on `diff --git`, so anything the
    /// commit's own message would have put above that must not be there.
    #[test]
    fn a_patch_carries_no_commit_header() {
        let dir = repository();
        let path = dir.path();

        std::fs::write(path.join("README.md"), "# a repository\n\nWith words.\n").unwrap();
        run(path, &["commit", "-am", "docs: say more"]);

        let patch = patch(path, &head(path)).unwrap();

        assert!(!patch.contains("docs: say more"), "{patch:?}");
        assert!(!patch.contains("Author:"), "{patch:?}");
        assert!(patch.starts_with("diff --git"), "{patch:?}");
    }

    /// What a sweep is: everything on the branch past where it started, oldest
    /// first, minus whatever is already on the Timeline.
    #[test]
    fn a_sweep_lists_what_is_on_the_branch_and_not_yet_recorded() {
        let dir = repository();
        let path = dir.path();
        let base = head(path);

        run(path, &["checkout", "-b", "rate-limiting"]);

        for (file, message) in [
            ("one.txt", "one"),
            ("two.txt", "two"),
            ("three.txt", "three"),
        ] {
            std::fs::write(path.join(file), "x\n").unwrap();
            run(path, &["add", file]);
            run(path, &["commit", "-m", message]);
        }

        let branch = Branch {
            repo: path.to_owned(),
            branch: "rate-limiting".to_owned(),
            base: base.clone(),
        };

        let landed = since(&branch, &[]);
        let subjects: Vec<&str> = landed.iter().map(|it| it.subject.as_str()).collect();

        assert_eq!(
            subjects,
            vec!["one", "two", "three"],
            "oldest first, which is the order they landed and the order they are read in",
        );
        assert!(
            !landed.iter().any(|commit| commit.sha == base),
            "and nothing from before the work started",
        );

        // What the second sweep sees, once the first two are on the Timeline.
        let known = vec![landed[0].sha.clone(), landed[1].sha.clone()];
        let left = since(&branch, &known);

        assert_eq!(
            left.iter()
                .map(|it| it.subject.as_str())
                .collect::<Vec<_>>(),
            vec!["three"],
        );
    }

    /// The repository being swept is one an agent is working in, and the moment
    /// a sweep is most likely to land on is the moment a session is committing
    /// — which is exactly when `index.lock` is held.
    ///
    /// So the sweep reads through it rather than waiting for it or, worse,
    /// taking one of its own: a reader that took the lock would be a reader that
    /// made the session's own `git commit` fail, on a machine with nobody
    /// watching.
    #[test]
    fn a_sweep_reads_through_a_lock_a_session_is_holding() {
        let dir = repository();
        let path = dir.path();
        let base = head(path);

        run(path, &["checkout", "-b", "rate-limiting"]);
        std::fs::write(path.join("limiter.rs"), "fn allow() {}\n").unwrap();
        run(path, &["add", "limiter.rs"]);
        run(path, &["commit", "-m", "feat: rate limiting"]);

        // What a session mid-commit has left in the repository.
        let lock = path.join(".git/index.lock");
        std::fs::write(&lock, "").unwrap();

        let branch = Branch {
            repo: path.to_owned(),
            branch: "rate-limiting".to_owned(),
            base,
        };

        let landed = since(&branch, &[]);

        assert_eq!(
            landed
                .iter()
                .map(|it| it.subject.as_str())
                .collect::<Vec<_>>(),
            vec!["feat: rate limiting"],
            "a locked repository is still a repository to read",
        );
        assert!(patch(path, &landed[0].sha).is_some(), "and to diff");

        assert!(
            lock.exists(),
            "the session's lock is still the session's: nothing here took it or cleared it",
        );
    }

    /// A branch that has committed nothing yet is the ordinary state of one for
    /// as long as a session is thinking, and it must not read as anything.
    #[test]
    fn a_branch_with_nothing_on_it_sweeps_up_nothing() {
        let dir = repository();
        let path = dir.path();

        let branch = Branch {
            repo: path.to_owned(),
            branch: "main".to_owned(),
            base: head(path),
        };

        assert!(since(&branch, &[]).is_empty());
    }
}
