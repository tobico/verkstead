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
//! What a sweep counts as the Conversation's is what the base branch does not
//! already hold. A resolution session settling a pull request's conflicts merges
//! that branch in, and everything it has gained since the work was cut arrives
//! with it — none of which is this Conversation's work, and all of which would
//! bury what is. So the branch it was cut off is recorded beside the commit, and
//! the listing leaves out everything reachable from it. See [`excluded`].
//!
//! A branch is swept whole rather than followed from where the last sweep got
//! to. What makes that cheap is the store: it already knows which commits are on
//! the Timeline, so the reading of git that costs anything — a message and a set
//! of counts per commit — happens only for the ones that are not. And what makes
//! it *correct* is the same thing, because a branch is not a queue: one that was
//! amended, reset or rebased has commits before its tip that no sweep has seen.
//!
//! One session may be watching several branches. A Conversation's own is one of
//! them and each read-write companion is another — see [`watched`] — and each
//! gets a watcher of exactly this shape, reading its own repository. A read-only
//! companion is checked out detached and bound read-only, so there is nothing
//! there for a commit to land on.

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
    branch: Branch,
    mut stopping: oneshot::Receiver<()>,
) {
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
/// work is on, the commit it started from, and the branch that commit was
/// resolved through.
///
/// The repository rather than the worktree, and not only because a worktree can
/// be removed while its branch lives on. The refs are the repository's — a
/// worktree shares them — so this is asking the thing that actually knows.
#[derive(Debug, Clone)]
pub(crate) struct Branch {
    /// The registered Repo the branch is in, which is what a commit is recorded
    /// against: two repositories are two histories, and a sha says nothing
    /// across them.
    repo_id: i64,

    repo: PathBuf,

    /// What the branch was called when the watch started, which is what nearly
    /// every sweep of it goes on using. What can move it is [`Self::following`].
    branch: String,

    /// Whether that name is still the name, and where a sweep looks to find out.
    following: Following,

    /// What the work branched from. It is the far end of the range, so it is
    /// what stops the whole history of the default branch arriving as this
    /// Conversation's commits.
    base: String,

    /// And the branch that commit was resolved through, where the record holds
    /// one. Everything reachable from it is left out of the sweep, which is what
    /// keeps a resolution session's merge of the base branch from dragging every
    /// commit the base has gained onto the Timeline — see [`excluded`].
    base_ref: Option<String>,

    /// The Repo's default branch, which is what a sweep excludes by where the
    /// record names no branch or names one that has stopped resolving.
    default_branch: String,
}

/// What moves a watched branch's name under the watcher.
///
/// A session may rename the branch it is working on, and Verkstead follows that
/// rather than repairing it — see [`crate::renames`]. Which leaves a sweep
/// reading a name that has changed since the watcher was spawned, so each sweep
/// asks first, and this is what it asks about.
#[derive(Debug, Clone)]
enum Following {
    /// Nothing: a companion the human gave a branch name of its own, which is
    /// that companion's name whatever the Conversation's branch is called.
    Nothing,

    /// The Conversation's own branch, in its own Worktree — the checkout a
    /// rename is read off, and the one whose reading moves the record.
    Own(PathBuf),

    /// A companion left on the empty *mirroring* setting: the Conversation's
    /// branch name, so the record that says what that is says what this is.
    Mirroring,
}

/// Every branch a session running for `conversation` can land a commit on: the
/// Conversation's own, and one per read-write companion.
///
/// Worked out here rather than where the watchers are spawned, so that what is
/// watched and what a sweep does with it are the one module's answer. Each entry
/// names the repository rather than the checkout — the refs are the
/// repository's, which a worktree shares — and the commit that repository's own
/// base resolved to when its checkout was made, with the branch it resolved
/// through beside it. A read-write companion is swept the same way, off the
/// `base_ref` its row has always held.
///
/// The Conversation's own checkout comes along beside its repository all the
/// same, because that is where a rename shows up: the branch a session is
/// working on may not be the branch it was given, and each sweep asks — see
/// [`Following`].
///
/// What is left out is left out with a line in the log, because each absence is
/// a record that has been got at rather than an ordinary state: a Conversation
/// with a session running has a base commit, and so has every companion that was
/// checked out for it. A read-only companion is the one silent omission — its
/// checkout is detached and bound read-only, so there is nothing to sweep.
pub(crate) fn watched(conversation: &store::Conversation) -> Vec<Branch> {
    let mut watching = Vec::new();

    match conversation.base_commit.clone() {
        Some(base) => watching.push(Branch {
            repo_id: conversation.repo.id,
            repo: conversation.repo.path.clone(),
            branch: conversation.branch.clone(),
            // A Conversation with a session running has a Worktree, and one
            // without is a record nothing here can repair: a rename is read off
            // a checkout, so with no checkout to read there is nothing to
            // follow and the name stands as the record has it.
            following: match conversation.worktree.clone() {
                Some(worktree) => Following::Own(worktree),
                None => Following::Nothing,
            },
            base,
            base_ref: conversation.base_ref.clone(),
            default_branch: conversation.repo.default_branch.clone(),
        }),
        None => tracing::error!(
            conversation_id = conversation.id,
            "a session is running on a Conversation with no base commit, \
             so its commits cannot be told from what it branched off"
        ),
    }

    for companion in &conversation.companions {
        if companion.mode != store::CompanionMode::ReadWrite {
            continue;
        }

        // Mirroring resolved, which is the record's own business rather than
        // this sweep's: an empty name on the row is the Conversation's branch
        // followed as it is renamed.
        let Some(branch) = companion.branch_for(&conversation.branch) else {
            continue;
        };

        let Some(base) = companion.base_commit.clone() else {
            tracing::error!(
                conversation_id = conversation.id,
                repo = companion.repo.name,
                "a read-write companion has no base commit, so what its session \
                 commits cannot be told from what it branched off"
            );
            continue;
        };

        watching.push(Branch {
            repo_id: companion.repo.id,
            repo: companion.repo.path.clone(),
            branch,
            // And what the mirror rule resolved to is only what it resolves to
            // *now*: a mirroring companion's branch is renamed along with the
            // Conversation's, so this one's name moves with the record too.
            following: match companion.branch.is_empty() {
                true => Following::Mirroring,
                false => Following::Nothing,
            },
            base,
            base_ref: companion.base_ref.clone(),
            default_branch: companion.repo.default_branch.clone(),
        });
    }

    watching
}

/// Take one look at the branch, and record whatever is on it that is not on the
/// Timeline yet.
///
/// What the branch is called is asked before it is read, because it may have
/// moved since the watcher was spawned — see [`now`], and [`crate::renames`] for
/// why it moves at all.
///
/// Nothing here is refused for. A repository that will not answer, a branch that
/// has gone, a store that will not take a row: each is logged and the next sweep
/// tries again. What is being watched is a side effect of work happening
/// elsewhere, and a watcher that gave up the first time git was busy would be
/// one that stopped watching without anybody noticing.
async fn sweep(pool: &SqlitePool, nudges: &Nudges, conversation_id: i64, branch: &Branch) {
    let Some(branch) = now(pool, conversation_id, branch).await else {
        return;
    };

    let recorded = match store::recorded_commits(pool, conversation_id, branch.repo_id).await {
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
        match store::record_commit(pool, conversation_id, branch.repo_id, &commit).await {
            Ok(Some(event_id)) => {
                tracing::info!(
                    conversation_id,
                    event_id,
                    sha = commit.sha,
                    repo = %branch.repo.display(),
                    "a commit landed on the Timeline"
                );
                recorded_any = true;
            }
            // Recorded by another sweep between the read above and this write.
            // One watcher per repository per Conversation makes that unlikely
            // rather than impossible, and the point of the store's unique index
            // is that being wrong about it costs nothing.
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

/// The branch as it stands at the top of this sweep: the same one, unless its
/// name has moved.
///
/// A companion the human named is the same one every time and is handed back
/// unasked. The Conversation's own is where a rename is looked for — one
/// `git symbolic-ref` in the ordinary case, and a followed record in the rare
/// one — and a mirroring companion takes the answer the record now holds,
/// having been renamed along with it.
///
/// `None` is a Conversation the record no longer has, which is a Conversation
/// with no branch to sweep. A reading that merely failed is not that: the name
/// the watcher was started on stands, and the sweep goes on with it rather than
/// skipping a poll over a busy database.
async fn now(pool: &SqlitePool, conversation_id: i64, branch: &Branch) -> Option<Branch> {
    let worktree = match &branch.following {
        Following::Nothing => return Some(branch.clone()),
        Following::Mirroring => None,
        Following::Own(worktree) => Some(worktree.clone()),
    };

    let named = match store::conversation_branch(pool, conversation_id).await {
        Ok(Some(named)) => named,
        Ok(None) => return None,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading a Conversation's branch failed");
            branch.branch.clone()
        }
    };

    let named = match worktree {
        Some(worktree) => {
            crate::renames::follow(pool, conversation_id, &branch.repo, &worktree, &named)
                .await
                .unwrap_or(named)
        }
        None => named,
    };

    Some(Branch {
        branch: named,
        ..branch.clone()
    })
}

/// Every commit on the branch that is not among `recorded`, oldest first.
///
/// The whole of the branch is listed and then filtered, rather than asked for
/// as *what is new*: git has no way to answer the second question, and the
/// answer to the first is a list of hashes that costs nothing to read. What
/// costs something is describing a commit, and that happens only for the ones
/// left over.
///
/// What is on the branch is the branch minus the base commit *and* minus
/// everything the base branch holds — see [`excluded`]. A resolution session
/// that merges the base branch in to settle a pull request's conflicts brings
/// every commit the base has gained since the work was cut along with it, and
/// none of that is the Conversation's work. The merge commit itself is not on
/// the base branch, so it stays.
///
/// Written as `<branch> ^<excluded>...` rather than `--not`, which is the same
/// listing: the caret form is a revision like any other, so `--end-of-options`
/// can stand in front of the whole lot and nothing here can be read as a flag.
///
/// Blocking, like everything that shells out to git.
fn since(branch: &Branch, recorded: &[String]) -> Vec<store::Commit> {
    let mut listing = vec![
        "rev-list".to_owned(),
        "--reverse".to_owned(),
        "--end-of-options".to_owned(),
        branch.branch.clone(),
        format!("^{}", branch.base),
    ];

    listing.extend(
        excluded(branch)
            .into_iter()
            .map(|named| format!("^{named}")),
    );

    let arguments: Vec<&str> = listing.iter().map(String::as_str).collect();

    let Some(listed) = git(&branch.repo, &arguments) else {
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

/// The branches whose commits a sweep leaves out: the one the work was cut off
/// and its counterpart across `origin`.
///
/// Both, because an agent told to fetch and merge the base branch in may end up
/// on either — `git merge origin/main` and `git merge main` are the same
/// instruction followed two ways, and only the branch it actually merged holds
/// the commits to leave out.
///
/// No fetch is made for it. This runs every couple of seconds, and a resolution
/// session fetches before it merges: origin's copy is already as current as the
/// merge that put anything here to leave out.
///
/// The fallbacks, in order. A record naming no branch — every Conversation
/// started before the name was kept — and one whose name has stopped resolving
/// both fall to the Repo's default branch as origin holds it, which is the rule
/// an unpicked base started under anyway. A Repo where that does not resolve
/// either excludes nothing, and the sweep is `<base commit>..<branch>` exactly
/// as it always was.
///
/// A ref that will not resolve is dropped rather than passed on: git refuses the
/// whole listing over one argument it cannot make sense of, and a sweep that
/// stopped reading because a branch had been deleted would be a Timeline that
/// silently stopped growing.
///
/// Blocking, like everything that shells out to git.
fn excluded(branch: &Branch) -> Vec<String> {
    let named: Vec<String> = branch
        .base_ref
        .iter()
        .flat_map(|named| [named.clone(), counterpart(named)])
        .filter(|named| crate::worktrees::resolve(&branch.repo, named).is_some())
        .collect();

    if !named.is_empty() {
        return named;
    }

    let default = crate::worktrees::default_ref(&branch.repo, &branch.default_branch);

    crate::worktrees::resolve(&branch.repo, &default)
        .map(|_| vec![default])
        .unwrap_or_default()
}

/// The same branch on the other side of `origin`: `main` for `origin/main`, and
/// `origin/main` for `main`.
///
/// Both directions, because either can be the recorded one. A base nobody picked
/// is recorded as origin holds it, and a base the human picked is whatever they
/// typed into the field.
fn counterpart(named: &str) -> String {
    match named.strip_prefix("origin/") {
        Some(local) => local.to_owned(),
        None => format!("origin/{named}"),
    }
}

/// What one commit is, as its Timeline row says it: the message git recorded,
/// and how much of the repository it moved.
///
/// Two reads rather than one that says both. `--numstat` and a format string can
/// be asked for together, but then the counts arrive underneath a header that
/// has to be told apart from them — and the counts are the half that has to be
/// parsed exactly.
///
/// The message is one read all the same. `%s` is a single line whatever the
/// commit did to its first paragraph, so the subject is everything before the
/// first newline and the body is the rest — and asking for the body separately
/// would be a third git process per commit for a string git already had open.
///
/// `None` where the repository will not say, which is a commit that has gone
/// between being listed and being asked about.
fn describe(repo: &Path, sha: &str) -> Option<store::Commit> {
    let message = git(
        repo,
        &[
            "show",
            "--no-patch",
            "--format=%s%n%b",
            "--end-of-options",
            sha,
        ],
    )?;

    let (subject, body) = message.split_once('\n').unwrap_or((message.as_str(), ""));

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
        subject: subject.trim_end().to_owned(),
        files,
        insertions,
        deletions,
        summary: without_trailers(body),
        // Which repository this is read out of is what the sweep already knows,
        // and the row is written with its id — see [`store::record_commit`].
        // The name here is what a read gives back for drawing.
        repo: None,
    })
}

/// What a commit says about itself: its message body with the trailing trailer
/// block taken off, or `None` where that leaves nothing.
///
/// The trailers are git's own convention rather than anything of Verkstead's —
/// `Co-Authored-By`, `Signed-off-by` and their kin — and every session's commits
/// end with at least one. They are bookkeeping about who wrote the commit, not
/// part of what the agent had to say about it, so the pane would be showing the
/// reader a line they did not come for. git keeps the whole message regardless:
/// this is what is *shown*, not what is kept.
///
/// The block is the last paragraph, and only where every line of it is a trailer
/// — `Token: value`, or a line indented under one. That is git's own reading, so
/// a commit whose last paragraph is prose keeps it, and a body that is nothing
/// but trailers comes back as `None`: bookkeeping alone is no summary at all.
fn without_trailers(body: &str) -> Option<String> {
    let body = body.trim_start_matches('\n').trim_end();

    let lines: Vec<&str> = body.lines().collect();

    // Where the last paragraph starts: past the last blank line there is one.
    let opens = lines
        .iter()
        .rposition(|line| line.trim().is_empty())
        .map_or(0, |blank| blank + 1);

    let kept = if trailers(&lines[opens..]) {
        lines[..opens].join("\n")
    } else {
        body.to_owned()
    };

    let kept = kept.trim_end();

    (!kept.is_empty()).then(|| kept.to_owned())
}

/// Whether these lines are a trailer block: at least one line, the first of them
/// a `Token: value`, and every line after it either another one or a
/// continuation indented under the one above.
fn trailers(paragraph: &[&str]) -> bool {
    let Some((first, rest)) = paragraph.split_first() else {
        return false;
    };

    trailer(first)
        && rest
            .iter()
            .all(|line| trailer(line) || line.starts_with([' ', '\t']))
}

/// Whether one line opens a trailer: a token of letters, digits and hyphens,
/// then a colon.
fn trailer(line: &str) -> bool {
    line.split_once(':').is_some_and(|(token, _)| {
        !token.is_empty()
            && token
                .chars()
                .all(|it| it.is_ascii_alphanumeric() || it == '-')
    })
}

/// Whether anything has landed on `branch` past the commit it was cut from.
///
/// What *touched* means about a companion repository, and the whole of what
/// decides whether a wrap-up expects a pull request of it: a read-write
/// companion the work committed in is carried to one of its own, and one nobody
/// committed in is ignored by the whole of wrap-up — nothing asked of GitHub,
/// nothing recorded, nothing waited on. A read-only companion is never asked
/// this at all: its checkout is detached and bound read-only, so nothing can
/// have landed on it.
///
/// Asked of git rather than of the commits the sweep has already put on the
/// Timeline. The sweep should agree — it sweeps once more as a session ends —
/// but it is a poller's record, and one that failed on a busy repository would
/// leave a touched companion looking untouched, and Verkstead would silently
/// expect no pull request from it.
///
/// A repository that will not say reads as untouched, with a line in the log.
/// It is the only answer with anything behind it: a branch git cannot list is
/// one nothing could have opened a pull request on either, and the other way
/// round would stop a run over a repository nothing can be read from.
///
/// Blocking, like everything that shells out to git.
pub(crate) fn touched(repo: &Path, base: &str, branch: &str) -> bool {
    let Some(counted) = git(
        repo,
        &[
            "rev-list",
            "--count",
            "--end-of-options",
            &format!("{base}..{branch}"),
        ],
    ) else {
        tracing::warn!(
            repo = %repo.display(),
            branch,
            "the repository would not say what is on this branch, so nothing is expected of it",
        );
        return false;
    };

    counted.trim().parse::<i64>().unwrap_or(0) > 0
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

    /// A session that renamed its branch goes on having its commits swept up,
    /// on the name it renamed it to.
    ///
    /// The watcher was spawned on the name the record held when the session
    /// started, so this is the sweep asking again rather than the watcher being
    /// told: what is on the branch now is what the human is waiting to see, and
    /// the branch the watcher was given no longer exists.
    #[tokio::test]
    async fn a_sweep_follows_the_branch_where_a_session_renamed_it() {
        let dir = repository();
        let repo = dir.path().to_owned();
        let base = head(&repo);

        let pool = crate::open_database(&repo.join("../verkstead.db"))
            .await
            .unwrap();

        let repo_id = store::register_repo(&pool, &repo, "verkstead", "main")
            .await
            .unwrap()
            .unwrap()
            .id;

        let conversation = store::start_unnamed_conversation(&pool, repo_id, "verkstead-7f3a")
            .await
            .unwrap()
            .unwrap();

        let worktree = dir.path().join("worktrees/verkstead-work");

        assert!(crate::worktrees::add(
            &repo,
            &worktree,
            "verkstead-7f3a",
            "HEAD"
        ));

        store::start_grilling(&pool, conversation, &base, &worktree, &[])
            .await
            .unwrap();

        // What the session did: a commit, and then the rename the naming
        // instruction asked it for.
        run(
            &worktree,
            &["config", "user.email", "test@verkstead.invalid"],
        );
        run(&worktree, &["config", "user.name", "Verkstead Test"]);
        std::fs::write(worktree.join("limiter.rs"), "fn allow() {}\n").unwrap();
        run(&worktree, &["add", "limiter.rs"]);
        run(&worktree, &["commit", "-m", "feat: rate limiting"]);
        run(&worktree, &["branch", "-m", "rate-limiting"]);

        let watched = Branch {
            repo_id,
            repo: repo.clone(),
            branch: "verkstead-7f3a".to_owned(),
            following: Following::Own(worktree.clone()),
            base,
            base_ref: None,
            default_branch: "main".to_owned(),
        };

        sweep(&pool, &Nudges::new(), conversation, &watched).await;

        let recorded = store::recorded_commits(&pool, conversation, repo_id)
            .await
            .unwrap();

        assert_eq!(
            recorded.len(),
            1,
            "the commit is on the Timeline, read off a branch by a name the \
             watcher was never given",
        );
        assert_eq!(
            store::conversation_branch(&pool, conversation)
                .await
                .unwrap()
                .as_deref(),
            Some("rate-limiting"),
            "and the record is on the new name for everything else that reads it",
        );
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

    /// What the agent wrote under the subject is the Commit Summary, and the
    /// trailers every session's commits end with are not part of it.
    #[test]
    fn a_commit_carries_its_body_as_a_summary_without_the_trailers() {
        let dir = repository();
        let path = dir.path();

        std::fs::write(path.join("README.md"), "# a repository\n\nWith words.\n").unwrap();
        run(
            path,
            &[
                "commit",
                "-am",
                "feat: rate limiting\n\n```mermaid\nflowchart LR\n  in --> out\n```\n\n\
                 A bucket per account.\n\nCo-Authored-By: Claude <noreply@anthropic.com>",
            ],
        );

        let described = describe(path, &head(path)).unwrap();

        assert_eq!(described.subject, "feat: rate limiting");
        assert_eq!(
            described.summary.as_deref(),
            Some("```mermaid\nflowchart LR\n  in --> out\n```\n\nA bucket per account."),
            "the diagram and the prose, and nothing about who wrote them",
        );
    }

    /// The two commits that carry no summary: the bookkeeping one that said only
    /// what it was, and the one whose body is trailers and nothing else.
    #[test]
    fn a_commit_that_said_nothing_about_itself_has_no_summary() {
        let dir = repository();
        let path = dir.path();

        std::fs::write(path.join("README.md"), "# a repository\n\nWith words.\n").unwrap();
        run(path, &["commit", "-am", "chore: plan the tasks"]);

        assert_eq!(
            describe(path, &head(path)).unwrap().summary,
            None,
            "a subject on its own is no summary",
        );

        std::fs::write(path.join("README.md"), "# a repository\n\nAnd more.\n").unwrap();
        run(
            path,
            &[
                "commit",
                "-am",
                "chore: finish commit-summaries\n\nCo-Authored-By: Claude <noreply@anthropic.com>",
            ],
        );

        assert_eq!(
            describe(path, &head(path)).unwrap().summary,
            None,
            "and neither is bookkeeping about who wrote it",
        );
    }

    /// The trailer block is the *last* paragraph, and only where the whole of it
    /// is trailers. Prose that happens to have a colon in it is prose.
    #[test]
    fn only_a_trailing_block_of_trailers_is_taken_off() {
        assert_eq!(
            without_trailers("What it does.\n\nNote: it is fast.\nAnd it is small.\n"),
            Some("What it does.\n\nNote: it is fast.\nAnd it is small.".to_owned()),
            "a last paragraph that is not all trailers is kept whole",
        );

        assert_eq!(
            without_trailers(
                "What it does.\n\nCo-Authored-By: Claude <noreply@anthropic.com>\n\
                 Signed-off-by: Someone\n    <who@wrapped.example>\n"
            ),
            Some("What it does.".to_owned()),
            "several trailers and a wrapped one are one block",
        );

        assert_eq!(
            without_trailers("Reviewed-by: Someone\n\nWhat it does.\n"),
            Some("Reviewed-by: Someone\n\nWhat it does.".to_owned()),
            "a trailer that is not last is a paragraph like any other",
        );

        assert_eq!(without_trailers(""), None);
        assert_eq!(
            without_trailers("\n  \n"),
            None,
            "and whitespace is nothing"
        );
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
            verkstead_render::commit_pane(None, &patch).diff.is_some(),
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
            repo_id: 1,
            repo: path.to_owned(),
            branch: "rate-limiting".to_owned(),
            following: Following::Nothing,
            base: base.clone(),
            base_ref: None,
            default_branch: "main".to_owned(),
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

    /// A resolution session that merges the base branch in to settle a pull
    /// request's conflicts brings every commit the base has gained since the
    /// work was cut along with it, and none of that is the Conversation's work.
    ///
    /// So a commit is the Conversation's when the base branch does not already
    /// hold it. The merge commit itself is nobody else's — it is the hunks the
    /// agent resolved — so it stays.
    #[test]
    fn a_sweep_leaves_out_what_the_base_branch_already_holds() {
        let dir = repository();
        let path = dir.path();
        let base = head(path);

        run(path, &["checkout", "--quiet", "-b", "rate-limiting"]);
        std::fs::write(path.join("limiter.rs"), "fn allow() {}\n").unwrap();
        run(path, &["add", "limiter.rs"]);
        run(path, &["commit", "-m", "feat: rate limiting"]);

        // What the base branch gained while the work was going on, which is
        // somebody else's and belongs on nobody's Timeline.
        run(path, &["checkout", "--quiet", "main"]);
        std::fs::write(path.join("elsewhere.rs"), "fn elsewhere() {}\n").unwrap();
        run(path, &["add", "elsewhere.rs"]);
        run(path, &["commit", "-m", "feat: somebody else's work"]);

        // And the resolution session settling the conflict it caused.
        run(path, &["checkout", "--quiet", "rate-limiting"]);
        run(path, &["merge", "--no-ff", "-m", "merge: main", "main"]);

        let branch = Branch {
            repo_id: 1,
            repo: path.to_owned(),
            branch: "rate-limiting".to_owned(),
            following: Following::Nothing,
            base,
            base_ref: Some("main".to_owned()),
            default_branch: "main".to_owned(),
        };

        assert_eq!(
            since(&branch, &[])
                .iter()
                .map(|it| it.subject.as_str())
                .collect::<Vec<_>>(),
            vec!["feat: rate limiting", "merge: main"],
            "the work's own commit and the merge that resolved it, and nothing \
             the base branch was already holding",
        );
    }

    /// And on either side of `origin`, because an agent told to fetch and merge
    /// the base branch in may end up on either.
    ///
    /// The record here names the local branch, which is a week behind — and what
    /// was merged is origin's copy of it. Excluding only what was recorded would
    /// put every commit origin had gained on the Timeline.
    #[test]
    fn a_sweep_leaves_out_the_base_branch_on_either_side_of_origin() {
        let dir = repository();
        let path = dir.path();
        let base = head(path);

        run(path, &["checkout", "--quiet", "-b", "rate-limiting"]);
        std::fs::write(path.join("limiter.rs"), "fn allow() {}\n").unwrap();
        run(path, &["add", "limiter.rs"]);
        run(path, &["commit", "-m", "feat: rate limiting"]);

        // What origin has gained, with this checkout's own `main` left where it
        // was: the ordinary state of a repository nobody has pulled in.
        run(path, &["checkout", "--quiet", "-b", "upstream", "main"]);
        std::fs::write(path.join("elsewhere.rs"), "fn elsewhere() {}\n").unwrap();
        run(path, &["add", "elsewhere.rs"]);
        run(path, &["commit", "-m", "feat: somebody else's work"]);
        run(
            path,
            &["update-ref", "refs/remotes/origin/main", "upstream"],
        );

        run(path, &["checkout", "--quiet", "rate-limiting"]);
        run(path, &["branch", "--quiet", "-D", "upstream"]);
        run(
            path,
            &[
                "merge",
                "--no-ff",
                "-m",
                "merge: origin/main",
                "origin/main",
            ],
        );

        let branch = Branch {
            repo_id: 1,
            repo: path.to_owned(),
            branch: "rate-limiting".to_owned(),
            following: Following::Nothing,
            base,
            base_ref: Some("main".to_owned()),
            default_branch: "main".to_owned(),
        };

        assert_eq!(
            since(&branch, &[])
                .iter()
                .map(|it| it.subject.as_str())
                .collect::<Vec<_>>(),
            vec!["feat: rate limiting", "merge: origin/main"],
            "the branch that was merged is the one the record named, on the \
             other side of origin",
        );
    }

    /// The fallbacks, in order: a record naming no branch and one naming a
    /// branch that has gone both sweep by the Repo's default branch, and a Repo
    /// whose default branch does not resolve either sweeps by the base commit
    /// exactly as it always did.
    #[test]
    fn a_sweep_with_no_base_branch_left_falls_back_to_the_default_and_then_to_the_base() {
        let dir = repository();
        let path = dir.path();
        let base = head(path);

        run(path, &["checkout", "--quiet", "-b", "rate-limiting"]);
        std::fs::write(path.join("limiter.rs"), "fn allow() {}\n").unwrap();
        run(path, &["add", "limiter.rs"]);
        run(path, &["commit", "-m", "feat: rate limiting"]);

        run(path, &["checkout", "--quiet", "main"]);
        std::fs::write(path.join("elsewhere.rs"), "fn elsewhere() {}\n").unwrap();
        run(path, &["add", "elsewhere.rs"]);
        run(path, &["commit", "-m", "feat: somebody else's work"]);

        run(path, &["checkout", "--quiet", "rate-limiting"]);
        run(path, &["merge", "--no-ff", "-m", "merge: main", "main"]);

        let sweeping = |base_ref: Option<&str>, default: &str| Branch {
            repo_id: 1,
            repo: path.to_owned(),
            branch: "rate-limiting".to_owned(),
            following: Following::Nothing,
            base: base.clone(),
            base_ref: base_ref.map(str::to_owned),
            default_branch: default.to_owned(),
        };

        let subjects = |branch: &Branch| {
            since(branch, &[])
                .iter()
                .map(|it| it.subject.clone())
                .collect::<Vec<_>>()
        };

        assert_eq!(
            subjects(&sweeping(None, "main")),
            vec!["feat: rate limiting", "merge: main"],
            "a Conversation with no branch recorded — every one alive today — \
             excludes by the Repo's default branch",
        );
        assert_eq!(
            subjects(&sweeping(Some("release-1.4"), "main")),
            vec!["feat: rate limiting", "merge: main"],
            "and so does one whose recorded branch has stopped resolving",
        );
        assert_eq!(
            subjects(&sweeping(None, "trunk")),
            vec![
                "feat: somebody else's work",
                "feat: rate limiting",
                "merge: main",
            ],
            "a Repo with no base branch that resolves at all sweeps by the base \
             commit, exactly as it did before any of this",
        );
    }

    /// What a wrap-up asks of a companion repository before it expects a pull
    /// request of it: whether the work committed in it at all.
    ///
    /// The base is the commit its checkout was cut from, so a branch sitting
    /// exactly where it started is untouched however much history is behind it —
    /// which is the ordinary state of a companion somebody only read.
    #[test]
    fn a_branch_is_touched_once_something_has_landed_past_its_base() {
        let dir = repository();
        let path = dir.path();
        let base = head(path);

        run(path, &["checkout", "--quiet", "-b", "rate-limiting"]);

        assert!(
            !touched(path, &base, "rate-limiting"),
            "a branch cut and not committed on is one nothing is expected of",
        );

        std::fs::write(path.join("halves.md"), "the other half\n").unwrap();
        run(path, &["add", "halves.md"]);
        run(path, &["commit", "-m", "feat: the other half"]);

        assert!(
            touched(path, &base, "rate-limiting"),
            "and one commit past the base is the whole of what touched means",
        );

        assert!(
            !touched(path, &base, "no-such-branch"),
            "a branch git cannot list is one nothing could have opened a pull \
         request on either",
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
            repo_id: 1,
            repo: path.to_owned(),
            branch: "rate-limiting".to_owned(),
            following: Following::Nothing,
            base,
            base_ref: None,
            default_branch: "main".to_owned(),
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
            repo_id: 1,
            repo: path.to_owned(),
            branch: "main".to_owned(),
            following: Following::Nothing,
            base: head(path),
            base_ref: None,
            default_branch: "main".to_owned(),
        };

        assert!(since(&branch, &[]).is_empty());
    }
}
