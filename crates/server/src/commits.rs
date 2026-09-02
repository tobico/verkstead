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
//! the listing leaves out everything reachable from it — that branch and the
//! Repo's default branch both, the two being the same one only where nobody
//! picked. See [`excluded`].
//!
//! A branch is swept whole rather than followed from where the last sweep got
//! to, and a sweep subtracts as well as adds. What makes that cheap is the
//! store: it already knows which commits are on the Timeline, so the reading of
//! git that costs anything — a message and a patch per commit — happens only for
//! the ones that are not. And what makes it *correct* is the same thing, because
//! a branch is not a queue: one that was amended or rebased carries the same work
//! under new shas, so the sweep records those and forgets the recorded commits
//! the branch has stopped carrying. See [`forgotten`].
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

/// Take one look at the branch: record whatever is on it that is not on the
/// Timeline yet, and forget whatever is on the Timeline that the branch no
/// longer carries.
///
/// Both halves, because a branch is not a queue. A rebase or an amend leaves the
/// same work under new shas, and a sweep that only ever added would put it on
/// the Timeline a second time beside originals the repository has stopped
/// holding — see [`since`] for what is added and [`forgotten`] for what is taken
/// away.
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

    // Both halves of the sweep in one hop off the runtime: what the branch has
    // gained and what it has stopped carrying are two readings of the same set
    // of shas, and a second `spawn_blocking` would be a second thread for a git
    // read that costs what this one does.
    let (landed, rewritten) = {
        let branch = branch.clone();
        match tokio::task::spawn_blocking(move || {
            (since(&branch, &recorded), forgotten(&branch, &recorded))
        })
        .await
        {
            Ok(swept) => swept,
            Err(error) => {
                tracing::error!(error = ?error, conversation_id, "sweeping a branch failed");
                return;
            }
        }
    };

    // Nothing either way, which is what nearly every sweep finds: no store
    // write, and nobody told the world moved.
    if landed.is_empty() && rewritten.is_empty() {
        return;
    }

    let mut moved = false;

    // What the branch has stopped carrying goes first, so that a rebase's
    // Timeline is never seen holding the work twice: the commits that replaced
    // these are in `landed` below, and this sweep is what puts them there.
    for sha in rewritten {
        match store::forget_commit(pool, conversation_id, branch.repo_id, &sha).await {
            Ok(Some(event_id)) => {
                tracing::info!(
                    conversation_id,
                    event_id,
                    sha,
                    repo = %branch.repo.display(),
                    "a rewritten commit came off the Timeline"
                );
                moved = true;
            }
            // Forgotten by another sweep between the read above and this write,
            // which costs nothing to be wrong about for the reason recording one
            // twice does.
            Ok(None) => {}
            Err(error) => {
                tracing::error!(error = ?error, conversation_id, sha, "forgetting a commit failed");
                // Gone on from rather than stopped, unlike the recording below:
                // these are Events being taken away rather than put in order, so
                // one that will not go says nothing about the next. The branch
                // still does not carry it, and the next sweep offers it again.
            }
        }
    }

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
                moved = true;
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

    if moved {
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

/// The recorded commits the branch has stopped carrying, which are the ones to
/// take off the Timeline.
///
/// A Repo whose conflicts are settled by rebasing has every commit of the branch
/// rewritten under a new sha when one is, and an amend does the same to one
/// commit on any Repo. The rewritten work arrives through [`since`] as commits
/// no sweep has seen, so without this the Timeline would hold it twice: once
/// under the shas the branch carries, and once under shas the repository has let
/// go of.
///
/// One read for the whole set rather than one per commit: `--no-walk` says each
/// argument stands for itself rather than for its history, so what comes back is
/// exactly the recorded shas the branch no longer holds.
///
/// **Ancestry, and not the listing [`since`] makes.** What decides this is
/// whether git still has the commit on the branch — never whether it survived
/// the base branch's exclusion. A Conversation whose pull request has been
/// merged is wholly reachable from its base branch, and one swept by the listing
/// would have its entire Timeline taken away the first time a Follow-up session
/// ran. It also leaves the base-branch commits already recorded on older
/// Timelines exactly where they are.
///
/// `--ignore-missing` for the shas, so that one git has garbage collected does
/// not fail the whole read. Such a sha is simply not reported, and its Event
/// stays — which is the safe way round: a commit nothing can be said about is
/// better drawn than silently taken off the record.
///
/// The branch is resolved before it is asked about, and that is the same care
/// the other way round. `--ignore-missing` drops whatever it cannot make sense
/// of, the exclusion included, so a branch that has been deleted would come back
/// as *every commit is gone* and empty the Timeline. A branch that will not
/// resolve forgets nothing instead.
///
/// Blocking, like everything that shells out to git.
fn forgotten(branch: &Branch, recorded: &[String]) -> Vec<String> {
    if recorded.is_empty() {
        return Vec::new();
    }

    let Some(carrying) = crate::worktrees::resolve(&branch.repo, &branch.branch) else {
        tracing::warn!(
            repo = %branch.repo.display(),
            branch = branch.branch,
            "the repository has no such branch, so nothing is taken off the Timeline",
        );
        return Vec::new();
    };

    let mut listing = vec![
        "rev-list".to_owned(),
        "--ignore-missing".to_owned(),
        "--no-walk".to_owned(),
        "--end-of-options".to_owned(),
    ];

    listing.extend(recorded.iter().cloned());
    listing.push(format!("^{carrying}"));

    let arguments: Vec<&str> = listing.iter().map(String::as_str).collect();

    let Some(listed) = git(&branch.repo, &arguments) else {
        tracing::warn!(
            repo = %branch.repo.display(),
            branch = branch.branch,
            "the repository would not say which of these commits it still holds"
        );
        return Vec::new();
    };

    listed
        .lines()
        .map(str::trim)
        .filter(|sha| !sha.is_empty())
        .map(str::to_owned)
        .collect()
}

/// The branches whose commits a sweep leaves out: the one the work was cut off
/// and the Repo's default branch, each with its counterpart across `origin`.
///
/// Both sides of `origin`, because an agent told to fetch and merge the base
/// branch in may end up on either — `git merge origin/main` and `git merge main`
/// are the same instruction followed two ways, and only the branch it actually
/// merged holds the commits to leave out.
///
/// **And the default branch whether or not a base was picked**, because the
/// branch the work was cut off and the branch it is merged back into are not
/// always the same one. Nothing here passes `--base` to `gh pr create`, so every
/// pull request opens against the repository's default branch — and what a
/// resolution session is told to bring in is the *pull request's* base branch,
/// which for a Conversation started off a picked branch is the default and not
/// the pick. Excluding only what was recorded would put every commit the default
/// branch had gained onto that Conversation's Timeline, which is the whole of
/// what this is for. It costs nothing the other way round: a commit the default
/// branch already holds was never this Conversation's work, whatever it was cut
/// off.
///
/// A stacked stage is the one case where the two differ and the recorded name is
/// the right one — `gh stack submit` puts its pull request against the
/// predecessor's branch, which is what [`crate::continuing`] records — and it is
/// covered by being one of the two rather than by being chosen between.
///
/// No fetch is made for any of it. This runs every couple of seconds, and a
/// resolution session fetches before it merges: origin's copy is already as
/// current as the merge that put anything here to leave out.
///
/// Where nothing at all resolves — a Repo with no default branch and a
/// Conversation whose recorded name has gone — the sweep is
/// `<base commit>..<branch>` exactly as it always was.
///
/// A ref that will not resolve is dropped rather than passed on: git refuses the
/// whole listing over one argument it cannot make sense of, and a sweep that
/// stopped reading because a branch had been deleted would be a Timeline that
/// silently stopped growing.
///
/// Blocking, like everything that shells out to git.
fn excluded(branch: &Branch) -> Vec<String> {
    let mut excluding: Vec<String> = Vec::new();

    for named in branch
        .base_ref
        .iter()
        .map(String::as_str)
        .chain([branch.default_branch.as_str()])
        .flat_map(|named| [named.to_owned(), counterpart(named)])
    {
        // The same branch reached two ways — a base nobody picked is the default
        // branch, and its counterpart is the default's — so a name already here
        // is one git would be handed twice.
        if excluding.contains(&named) {
            continue;
        }

        if crate::worktrees::resolve(&branch.repo, &named).is_some() {
            excluding.push(named);
        }
    }

    excluding
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
/// Two reads rather than one that says both: the message, and the patch the
/// counts are taken off. That patch is the one [`patch`] gives the details
/// pane, so a row and the pane under it are two readings of one diff and
/// cannot disagree about what the commit moved.
///
/// Counted off the patch rather than asked of `--numstat`, and merges are why.
/// A merge is described by its combined diff, which is the hunks that differ
/// from *both* parents — the resolution the agent actually made. `--numstat`
/// prunes nothing, so a resolution session's merge would draw with every file
/// the base branch brought in beside the one it settled. Counting the patch
/// counts what is in it, and on a commit with one parent the two agree line
/// for line — which is every commit but the merge.
///
/// The message is one read all the same, and it carries the parents with it.
/// `%P` and `%s` are a single line each whatever the commit did to its first
/// paragraph, so the parents are the first line, the subject the second and the
/// body the rest — and asking for any of them separately would be another git
/// process per commit for a string git already had open.
///
/// Whether it is a merge is read here rather than whenever a page looks, for the
/// reason the subject and the counts are: it is kept beside the commit, and a
/// Timeline that asked git per row would be a git process per commit of it.
///
/// `None` where the repository will not say, which is a commit that has gone
/// between being listed and being asked about.
fn describe(repo: &Path, sha: &str) -> Option<store::Commit> {
    let message = git(
        repo,
        &[
            "show",
            "--no-patch",
            "--format=%P%n%s%n%b",
            "--end-of-options",
            sha,
        ],
    )?;

    let (parents, message) = message.split_once('\n').unwrap_or((message.as_str(), ""));
    let (subject, body) = message.split_once('\n').unwrap_or((message, ""));

    let (files, insertions, deletions) = counts(&patch(repo, sha)?);

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
        // A merge is a commit with more than one parent, which is what `%P`
        // says: one hash for an ordinary commit, none at all for a root one,
        // and two or more for the merge a resolution session left behind.
        merge: parents.split_whitespace().count() > 1,
    })
}

/// How much of the repository a patch moves: files, then lines put in, then
/// lines taken out.
///
/// git's own arithmetic done here rather than asked for: a file per `diff`
/// header, and a line per `+` or `-` inside a hunk. Held to the hunks on
/// purpose — the `---` and `+++` above one are a file's header rather than
/// lines of it, and so is a deleted `---` of front matter, which arrives as
/// `----` and is a deletion that anything reading the first three characters
/// would lose.
///
/// A combined diff counts by the same rule. Every line of one carries a column
/// per parent, so what stands in the first column is what the merge did to the
/// branch it was made on, and the files are the ones the combined diff kept:
/// what the agent resolved, rather than everything the other parent brought
/// along with it.
///
/// A binary file counts as a file and no lines, which is what `--numstat` says
/// of one too.
fn counts(patch: &str) -> (i64, i64, i64) {
    let mut files = 0;
    let mut insertions = 0;
    let mut deletions = 0;
    let mut hunk = false;

    for line in patch.lines() {
        // `diff --git` for an ordinary file and `diff --cc` for a merge's, and
        // never a line of a hunk: those carry a column per parent in front of
        // whatever they hold.
        if line.starts_with("diff --") {
            files += 1;
            hunk = false;
        } else if line.starts_with("@@") {
            hunk = true;
        } else if hunk {
            match line.as_bytes().first() {
                Some(b'+') => insertions += 1,
                Some(b'-') => deletions += 1,
                _ => {}
            }
        }
    }

    (files, insertions, deletions)
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
/// `--root` because a repository's first commit has no parent, and without it
/// git compares that commit against nothing and says it changed nothing.
///
/// `--cc` because a merge commit is the other one git says nothing about
/// unasked. What it gives back for one is the combined diff: the hunks that
/// differ from *both* parents, which on the merge a resolution session leaves
/// behind is the conflicts the agent settled and not the whole of what the base
/// branch brought in. The flag is passed unconditionally because it costs
/// nothing to — on a commit with one parent, a root commit included, the output
/// is the ordinary diff byte for byte.
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
            "--cc",
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

    /// A commit stamped with a moment of the test's choosing, for a history
    /// whose commits are read back oldest first.
    ///
    /// [`run`] leaves git to read the clock, and a whole history built inside
    /// one second is a history of commits that all carry the same moment. A
    /// listing then has nothing to order two branches' commits by, and falls
    /// back to the order its walk met them in — which, coming down from the
    /// merge, is the opposite of the order they were made in. A machine slow
    /// enough to cross a second between them reads them the other way round,
    /// so the same history says two different things on two machines.
    ///
    /// Naming the moment leaves it to neither: what the sweep reads back is the
    /// order these dates put the commits in.
    fn commit_at(dir: &Path, message: &str, when: &str) {
        let output = Command::new("git")
            .args(["commit", "-m", message])
            .current_dir(dir)
            .env("GIT_AUTHOR_DATE", when)
            .env("GIT_COMMITTER_DATE", when)
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .expect("git should be on the PATH for these tests");

        assert!(output.status.success(), "git commit -m {message:?} failed");
    }

    /// The same run, for a git that is expected to fail: a merge that conflicts
    /// exits non-zero, and the conflict is the point of asking for it.
    fn attempt(dir: &Path, args: &[&str]) {
        Command::new("git")
            .args(args)
            .current_dir(dir)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .expect("git should be on the PATH for these tests");
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
        assert!(
            !described.merge,
            "and it is the ordinary commit, which is what a card draws unlabelled",
        );
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
        assert!(
            !described.merge,
            "a commit with no parent at all is no merge either",
        );

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

    /// The merge a resolution session leaves behind carries the conflicts it
    /// settled, which is the agent's work and belongs on the Timeline. git says
    /// nothing at all about a merge unless it is asked for the combined diff,
    /// and what that says is the hunks that differ from both parents: what was
    /// resolved, and not the files the base branch brought in on its own.
    #[test]
    fn a_merge_is_described_by_what_it_resolved_and_renders() {
        let dir = repository();
        let path = dir.path();

        // The Conversation's own branch, and one commit of its work.
        run(path, &["checkout", "-b", "the-work"]);
        std::fs::write(path.join("README.md"), "# a repository\n\nThe work.\n").unwrap();
        run(path, &["commit", "-am", "feat: the work"]);

        // What the base branch gained meanwhile: a file nobody here touched,
        // and an edit to the one this branch is holding.
        run(path, &["checkout", "main"]);
        std::fs::write(path.join("README.md"), "# a repository\n\nThe base.\n").unwrap();
        std::fs::write(path.join("unrelated.md"), "one\ntwo\nthree\n").unwrap();
        run(path, &["add", "-A"]);
        run(path, &["commit", "-m", "docs: the base moves on"]);

        // And the resolution session: merge the base in, settle the conflict,
        // commit the merge.
        run(path, &["checkout", "the-work"]);
        attempt(path, &["merge", "main"]);
        std::fs::write(path.join("README.md"), "# a repository\n\nResolved.\n").unwrap();
        run(path, &["add", "-A"]);
        run(path, &["commit", "-m", "merge: settle the conflicts"]);

        let described = describe(path, &head(path)).expect("a merge is still a commit");

        assert_eq!(
            described.files, 1,
            "the file the agent resolved, and not the one the base brought in",
        );
        assert_eq!(described.insertions, 1);
        assert_eq!(described.deletions, 1);
        assert!(
            described.merge,
            "and it says it is a merge, which is what its card is labelled from: \
             the counts above are an ordinary small commit's",
        );

        let patch = patch(path, &head(path)).expect("and it still has a patch");

        assert!(
            patch.contains("Resolved."),
            "the hunk the agent settled is in it: {patch:?}",
        );
        assert!(
            !patch.contains("unrelated.md"),
            "and what the base brought in on its own is not: {patch:?}",
        );
        assert!(
            verkstead_render::commit_pane(None, &patch).diff.is_some(),
            "and the pane has something to show",
        );
    }

    /// The counting is held to the hunks, because a patch's first three
    /// characters do not say what a line is. A deleted `---` — front matter, or
    /// the rule under a heading — arrives as `----`, and anything that read it
    /// as the `---` of a file header would drop a deletion the human can see.
    #[test]
    fn a_deleted_dashed_line_is_counted_as_a_deletion() {
        let patch = "diff --git a/notes.md b/notes.md\n\
                     index 1111111..2222222 100644\n\
                     --- a/notes.md\n\
                     +++ b/notes.md\n\
                     @@ -1,3 +1,1 @@\n\
                     ----\n\
                     -title: notes\n\
                     ----\n\
                     +# notes\n";

        assert_eq!(
            counts(patch),
            (1, 1, 3),
            "one file, the heading put in, and the three lines of front matter taken out",
        );
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

        // The last of the three fallbacks sweeps both branches together, so
        // these two carry named moments rather than whatever the clock says:
        // the work first, and what landed on main while it was going on after
        // it.
        run(path, &["checkout", "--quiet", "-b", "rate-limiting"]);
        std::fs::write(path.join("limiter.rs"), "fn allow() {}\n").unwrap();
        run(path, &["add", "limiter.rs"]);
        commit_at(path, "feat: rate limiting", "2026-01-01T09:00:00Z");

        run(path, &["checkout", "--quiet", "main"]);
        std::fs::write(path.join("elsewhere.rs"), "fn elsewhere() {}\n").unwrap();
        run(path, &["add", "elsewhere.rs"]);
        commit_at(path, "feat: somebody else's work", "2026-01-01T10:00:00Z");

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
                "feat: rate limiting",
                "feat: somebody else's work",
                "merge: main",
            ],
            "a Repo with no base branch that resolves at all sweeps by the base \
             commit, exactly as it did before any of this — both branches, \
             oldest first",
        );
    }

    /// And the Repo's default branch is left out whether or not a base was
    /// picked, because the branch the work was cut off and the branch it is
    /// merged back into are not always the same one.
    ///
    /// Nothing here passes `--base` to `gh pr create`, so a pull request opens
    /// against the repository's default branch — and what a resolution session
    /// brings in is the *pull request's* base. The record here names the branch
    /// the human picked, and what was merged is the default.
    #[test]
    fn a_sweep_leaves_out_the_default_branch_as_well_as_the_one_that_was_picked() {
        let dir = repository();
        let path = dir.path();

        // The branch the human picked, off which the work is cut.
        run(path, &["checkout", "--quiet", "-b", "release-1.4"]);
        std::fs::write(path.join("release.rs"), "fn release() {}\n").unwrap();
        run(path, &["add", "release.rs"]);
        run(path, &["commit", "-m", "chore: cut the release branch"]);

        let base = head(path);

        run(path, &["checkout", "--quiet", "-b", "rate-limiting"]);
        std::fs::write(path.join("limiter.rs"), "fn allow() {}\n").unwrap();
        run(path, &["add", "limiter.rs"]);
        run(path, &["commit", "-m", "feat: rate limiting"]);

        // What the default branch gained meanwhile, which is what the pull
        // request is against and so what the resolution session merges in.
        run(path, &["checkout", "--quiet", "main"]);
        std::fs::write(path.join("elsewhere.rs"), "fn elsewhere() {}\n").unwrap();
        run(path, &["add", "elsewhere.rs"]);
        run(path, &["commit", "-m", "feat: somebody else's work"]);

        run(path, &["checkout", "--quiet", "rate-limiting"]);
        run(path, &["merge", "--no-ff", "-m", "merge: main", "main"]);

        let branch = Branch {
            repo_id: 1,
            repo: path.to_owned(),
            branch: "rate-limiting".to_owned(),
            following: Following::Nothing,
            base,
            base_ref: Some("release-1.4".to_owned()),
            default_branch: "main".to_owned(),
        };

        assert_eq!(
            since(&branch, &[])
                .iter()
                .map(|it| it.subject.as_str())
                .collect::<Vec<_>>(),
            vec!["feat: rate limiting", "merge: main"],
            "the default branch's own commit is left out too, though the record \
             names another branch entirely",
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

    /// A pool, a registered Repo and a Conversation on it: what the sweeps that
    /// write to a Timeline need beside the repository they read.
    ///
    /// The database goes inside `.git`, which is the one directory in a
    /// repository that no `git add -A` or rebase in these tests will look at.
    async fn recording(repo: &Path) -> (SqlitePool, i64, i64) {
        let pool = crate::open_database(&repo.join(".git/verkstead.db"))
            .await
            .unwrap();

        let repo_id = store::register_repo(&pool, repo, "verkstead", "main")
            .await
            .unwrap()
            .unwrap()
            .id;

        let conversation = store::start_conversation(&pool, repo_id, "rate-limiting")
            .await
            .unwrap()
            .unwrap();

        (pool, conversation, repo_id)
    }

    /// The commits on a Conversation's Timeline, as the sha and the subject each
    /// card draws.
    async fn on_the_timeline(pool: &SqlitePool, conversation: i64) -> Vec<(String, String)> {
        store::timeline(pool, conversation)
            .await
            .unwrap()
            .into_iter()
            .filter_map(|event| match event.event {
                store::Event::Commit(commit) => Some((commit.sha, commit.subject)),
                _ => None,
            })
            .collect()
    }

    /// Every commit the branch carries past `base`, oldest first.
    fn carrying(repo: &Path, base: &str, branch: &str) -> Vec<String> {
        run(
            repo,
            &["rev-list", "--reverse", &format!("{base}..{branch}")],
        )
        .lines()
        .map(str::to_owned)
        .collect()
    }

    /// A Repo whose conflicts are settled by rebasing has every commit of the
    /// branch rewritten under a new sha when one is. The rewritten work arrives
    /// as commits no sweep has seen, so a sweep that only ever added would put
    /// the work on the Timeline twice — once under shas the repository has let
    /// go of.
    #[tokio::test]
    async fn a_rebase_leaves_each_commit_on_the_timeline_once() {
        let dir = repository();
        let path = dir.path();
        let base = head(path);

        let (pool, conversation, repo_id) = recording(path).await;

        run(path, &["checkout", "--quiet", "-b", "rate-limiting"]);
        for (file, message) in [("one.txt", "feat: one"), ("two.txt", "feat: two")] {
            std::fs::write(path.join(file), "x\n").unwrap();
            run(path, &["add", file]);
            run(path, &["commit", "-m", message]);
        }

        let branch = Branch {
            repo_id,
            repo: path.to_owned(),
            branch: "rate-limiting".to_owned(),
            following: Following::Nothing,
            base: base.clone(),
            base_ref: Some("main".to_owned()),
            default_branch: "main".to_owned(),
        };

        sweep(&pool, &Nudges::new(), conversation, &branch).await;

        let before = carrying(path, &base, "rate-limiting");

        assert_eq!(
            on_the_timeline(&pool, conversation)
                .await
                .into_iter()
                .map(|(sha, _)| sha)
                .collect::<Vec<_>>(),
            before,
            "the two commits as the branch first carried them",
        );

        // What the base branch gained, and the rebase that settles it: every
        // commit of the branch is rewritten under a new sha.
        run(path, &["checkout", "--quiet", "main"]);
        std::fs::write(path.join("elsewhere.rs"), "fn elsewhere() {}\n").unwrap();
        run(path, &["add", "elsewhere.rs"]);
        run(path, &["commit", "-m", "feat: somebody else's work"]);

        run(path, &["checkout", "--quiet", "rate-limiting"]);
        run(path, &["rebase", "--quiet", "main"]);

        sweep(&pool, &Nudges::new(), conversation, &branch).await;

        let after = carrying(path, "main", "rate-limiting");

        assert_ne!(after, before, "the rebase rewrote both of them");
        assert_eq!(
            on_the_timeline(&pool, conversation).await,
            vec![
                (after[0].clone(), "feat: one".to_owned()),
                (after[1].clone(), "feat: two".to_owned()),
            ],
            "each commit is on the Timeline once, under the sha the branch now \
             carries",
        );
    }

    /// The same on any Repo, without a resolution strategy coming into it: an
    /// amended commit is a commit rewritten under a new sha, and it replaces the
    /// one it rewrote rather than joining it.
    #[tokio::test]
    async fn an_amended_commit_replaces_the_one_it_rewrote() {
        let dir = repository();
        let path = dir.path();
        let base = head(path);

        let (pool, conversation, repo_id) = recording(path).await;

        run(path, &["checkout", "--quiet", "-b", "rate-limiting"]);
        std::fs::write(path.join("limiter.rs"), "fn allow() {}\n").unwrap();
        run(path, &["add", "limiter.rs"]);
        run(path, &["commit", "-m", "feat: rate limitting"]);

        let branch = Branch {
            repo_id,
            repo: path.to_owned(),
            branch: "rate-limiting".to_owned(),
            following: Following::Nothing,
            base: base.clone(),
            base_ref: Some("main".to_owned()),
            default_branch: "main".to_owned(),
        };

        sweep(&pool, &Nudges::new(), conversation, &branch).await;

        run(
            path,
            &["commit", "--quiet", "--amend", "-m", "feat: rate limiting"],
        );

        sweep(&pool, &Nudges::new(), conversation, &branch).await;

        assert_eq!(
            on_the_timeline(&pool, conversation).await,
            vec![(head(path), "feat: rate limiting".to_owned())],
            "the commit the branch carries, and not the typo it was amended from",
        );
    }

    /// What decides that a commit is gone is whether git still holds it on the
    /// branch, and never whether it survived the base branch's exclusion.
    ///
    /// A Conversation whose pull request has been merged is wholly reachable
    /// from its base branch, so one swept by the listing would take its entire
    /// Timeline away the first time a Follow-up session ran.
    #[tokio::test]
    async fn a_conversation_whose_branch_was_merged_keeps_its_whole_timeline() {
        let dir = repository();
        let path = dir.path();
        let base = head(path);

        let (pool, conversation, repo_id) = recording(path).await;

        run(path, &["checkout", "--quiet", "-b", "rate-limiting"]);
        std::fs::write(path.join("limiter.rs"), "fn allow() {}\n").unwrap();
        run(path, &["add", "limiter.rs"]);
        run(path, &["commit", "-m", "feat: rate limiting"]);

        let branch = Branch {
            repo_id,
            repo: path.to_owned(),
            branch: "rate-limiting".to_owned(),
            following: Following::Nothing,
            base,
            base_ref: Some("main".to_owned()),
            default_branch: "main".to_owned(),
        };

        sweep(&pool, &Nudges::new(), conversation, &branch).await;

        let recorded = on_the_timeline(&pool, conversation).await;
        assert_eq!(recorded.len(), 1, "the work's own commit");

        // The pull request merging, which is what puts the whole branch on the
        // base branch — and what a Follow-up session sweeps after.
        run(path, &["checkout", "--quiet", "main"]);
        run(
            path,
            &[
                "merge",
                "--quiet",
                "--no-ff",
                "-m",
                "Merge pull request",
                "rate-limiting",
            ],
        );
        run(path, &["checkout", "--quiet", "rate-limiting"]);

        sweep(&pool, &Nudges::new(), conversation, &branch).await;

        assert_eq!(
            on_the_timeline(&pool, conversation).await,
            recorded,
            "the branch still carries it, whatever the base branch has swallowed",
        );
    }

    /// A recorded sha the repository has stopped holding at all — garbage
    /// collected after a rebase, or a repository restored from somewhere older —
    /// leaves its Event where it is.
    ///
    /// The safe way round: a commit nothing can be said about is better drawn
    /// than silently taken off the record.
    #[tokio::test]
    async fn a_sha_git_no_longer_holds_at_all_leaves_its_event_alone() {
        let dir = repository();
        let path = dir.path();
        let base = head(path);

        let (pool, conversation, repo_id) = recording(path).await;

        run(path, &["checkout", "--quiet", "-b", "rate-limiting"]);
        std::fs::write(path.join("limiter.rs"), "fn allow() {}\n").unwrap();
        run(path, &["add", "limiter.rs"]);
        run(path, &["commit", "-m", "feat: rate limiting"]);

        // A commit on the Timeline that this repository has never heard of.
        store::record_commit(
            &pool,
            conversation,
            repo_id,
            &store::Commit {
                sha: "0000000000000000000000000000000000000000".to_owned(),
                subject: "feat: collected away".to_owned(),
                files: 1,
                insertions: 1,
                deletions: 0,
                summary: None,
                repo: None,
                merge: false,
            },
        )
        .await
        .unwrap()
        .unwrap();

        let branch = Branch {
            repo_id,
            repo: path.to_owned(),
            branch: "rate-limiting".to_owned(),
            following: Following::Nothing,
            base,
            base_ref: Some("main".to_owned()),
            default_branch: "main".to_owned(),
        };

        sweep(&pool, &Nudges::new(), conversation, &branch).await;

        let subjects: Vec<String> = on_the_timeline(&pool, conversation)
            .await
            .into_iter()
            .map(|(_, subject)| subject)
            .collect();

        assert_eq!(
            subjects,
            vec![
                "feat: collected away".to_owned(),
                "feat: rate limiting".to_owned(),
            ],
            "the sha git cannot answer for is not reported gone, so its Event stays",
        );
    }

    /// And the branch itself is resolved before git is asked which commits it
    /// carries, because `--ignore-missing` drops whatever it cannot make sense
    /// of — the exclusion included. A branch that has been deleted would
    /// otherwise come back as *every commit is gone*.
    #[test]
    fn a_branch_that_will_not_resolve_forgets_nothing() {
        let dir = repository();
        let path = dir.path();
        let base = head(path);

        run(path, &["checkout", "--quiet", "-b", "rate-limiting"]);
        std::fs::write(path.join("limiter.rs"), "fn allow() {}\n").unwrap();
        run(path, &["add", "limiter.rs"]);
        run(path, &["commit", "-m", "feat: rate limiting"]);

        let recorded = vec![head(path)];

        let branch = Branch {
            repo_id: 1,
            repo: path.to_owned(),
            branch: "no-such-branch".to_owned(),
            following: Following::Nothing,
            base,
            base_ref: Some("main".to_owned()),
            default_branch: "main".to_owned(),
        };

        assert!(
            forgotten(&branch, &recorded).is_empty(),
            "a branch there is nothing to ask about is a branch nothing is \
             forgotten from",
        );
    }

    /// The page is told, so a Timeline open on the Conversation re-reads and a
    /// details pane on a forgotten Event has something to recover from.
    ///
    /// Swept after a reset, which is the one shape where a sweep forgets and
    /// records nothing: what says the page was told is this Nudge and nothing
    /// else.
    #[tokio::test]
    async fn the_page_is_told_when_a_commit_is_forgotten() {
        let dir = repository();
        let path = dir.path();
        let base = head(path);

        let (pool, conversation, repo_id) = recording(path).await;

        run(path, &["checkout", "--quiet", "-b", "rate-limiting"]);
        std::fs::write(path.join("limiter.rs"), "fn allow() {}\n").unwrap();
        run(path, &["add", "limiter.rs"]);
        run(path, &["commit", "-m", "feat: rate limiting"]);

        let branch = Branch {
            repo_id,
            repo: path.to_owned(),
            branch: "rate-limiting".to_owned(),
            following: Following::Nothing,
            base: base.clone(),
            base_ref: Some("main".to_owned()),
            default_branch: "main".to_owned(),
        };

        sweep(&pool, &Nudges::new(), conversation, &branch).await;

        run(path, &["reset", "--quiet", "--hard", &base]);

        let nudges = Nudges::new();
        let mut listening = nudges.subscribe();

        sweep(&pool, &nudges, conversation, &branch).await;

        assert!(
            on_the_timeline(&pool, conversation).await.is_empty(),
            "the commit the branch stopped carrying came off the Timeline",
        );
        assert!(
            matches!(
                listening.try_recv(),
                Ok(Nudge::Commit { conversation: told }) if told == conversation
            ),
            "and the pages were told to look again",
        );
    }
}
