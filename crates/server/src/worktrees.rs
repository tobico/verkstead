//! Where a Conversation's work is done: the branch it is on and the worktree it
//! is checked out in, made when grilling starts and removed when it is closed.
//!
//! Worktrees live under Verkstead's own data directory rather than inside a
//! Watched Path. The Watched Paths are the boundary on what the *human* may
//! point Verkstead at — repositories to register, accounts to run under — and a
//! worktree is neither: it is something Verkstead made, in the one directory the
//! packaged unit is given to write. Putting it inside a Watched Path would mean
//! Verkstead's own scratch space appearing under a directory the human is also
//! working in.
//!
//! The branch is made in the Repo's own git directory, not in the worktree —
//! `git worktree add -b` does both at once, which is the point of asking git for
//! this rather than checking a tree out by hand. Closing removes the worktree
//! and leaves the branch: a branch is a name and a commit, and it may hold work
//! worth reading; a worktree is a directory the human never asked to keep.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use crate::repos::git;
use crate::store;

/// The directory every Worktree goes under, inside `data`.
///
/// Named in one place because two things want it whole rather than one path
/// inside it: this module, choosing where a checkout goes, and
/// [`crate::build_cache`], which binds the lot of it into the compile server so
/// that a Worktree made later is one that server can already see.
pub(crate) fn directory(data: &Path) -> PathBuf {
    data.join("worktrees")
}

/// The directory a worktree goes in, under `data`.
///
/// Named for the Repo and the branch, which is what the human calls the work —
/// and what an agent working inside it sees itself working in. The branch may
/// hold slashes and two Repos may share a name, so neither is trusted as a path
/// component: everything that is not a plain name character becomes a hyphen,
/// and a name already taken falls back to one with the Conversation's id on the
/// end, which nothing else can collide with.
pub(crate) fn worktree_path(data: &Path, id: i64, repo: &str, branch: &str) -> PathBuf {
    unclaimed_path(data, id, repo, branch, &[])
}

/// The same directory, chosen where others are being chosen in the same breath.
///
/// [`worktree_path`]'s rule with one addition: a path `claimed` already holds
/// counts as taken. A grill start names every directory it is about to make —
/// the Conversation's own and one per companion — before it makes any of them,
/// which is what lets it refuse without leaving half of them behind. Until they
/// exist the filesystem cannot tell two of them apart, so two companion Repos
/// of one name coming off one branch name would otherwise be handed the same
/// directory and the second checkout would land on top of the first.
pub(crate) fn unclaimed_path(
    data: &Path,
    id: i64,
    repo: &str,
    branch: &str,
    claimed: &[PathBuf],
) -> PathBuf {
    let worktrees = data.join("worktrees");
    let stem = format!("{}-{}", component(repo), component(branch));

    // A directory that is already there is not one to check a branch out into,
    // whether it is another Conversation's or something else's entirely.
    let free = |path: &PathBuf| !path.exists() && !claimed.contains(path);

    let named = worktrees.join(&stem);

    if free(&named) {
        return named;
    }

    // The Conversation's id, which nothing outside this Conversation collides
    // with — and then a count, for the one thing that shares it: a second
    // companion of this Conversation asking for the same name.
    std::iter::once(worktrees.join(format!("{stem}-{id}")))
        .chain((2..).map(|nth| worktrees.join(format!("{stem}-{id}-{nth}"))))
        .find(free)
        .expect("the count is unbounded, so some name is free")
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

/// Whether `repo` has a branch by this name, with a read that could not be made
/// counting as one that is.
///
/// The same question [`branch_exists`] answers, asked where the answer decides
/// whether an agent is let loose on a branch. Git saying nothing at all is not
/// git saying the name is free, and taking a name somebody else's work is on is
/// the one way of being wrong here that pressing the button again cannot undo —
/// so a reading that failed reads as taken.
///
/// Which is the whole reason this is not [`branch_exists`]: that one asks
/// `show-ref`, which exits non-zero both for a ref that is not there and for a
/// repository it could not read. `for-each-ref` comes back with nothing to say
/// in the first case and does not come back at all in the second, and the
/// difference between those two is the difference this turns on.
pub(crate) fn branch_taken(repo: &Path, branch: &str) -> bool {
    let listed = git(
        repo,
        &[
            "for-each-ref",
            "--format=%(refname)",
            "--end-of-options",
            &format!("refs/heads/{branch}"),
        ],
    );

    match listed {
        Some(refs) => !refs.trim().is_empty(),
        None => true,
    }
}

/// Every branch of `repo` a Conversation could be based on: the local ones and
/// the remote-tracking ones both, in the order git lists them — the locals
/// first, then whatever the remotes are carrying.
///
/// Both, because both are things the human works from: a branch of their own
/// they have not pushed, and one somebody else pushed that is not merged yet.
/// A symbolic ref is not one of them and is left out — `origin/HEAD` is another
/// name for a branch that is already in the list, and offering it twice would
/// be offering a choice that is not one.
///
/// Empty for a repository git would not read, which is the same answer as a
/// repository with no branches. Nothing is decided on the difference: what this
/// list is for is a dropdown, and a dropdown offering nothing but the default
/// rule is the honest reading of *there is nothing here to pick*.
pub(crate) fn branches(repo: &Path) -> Vec<String> {
    let listed = git(
        repo,
        &[
            "for-each-ref",
            "--format=%(symref)\t%(refname:short)",
            "refs/heads",
            "refs/remotes",
        ],
    );

    listed
        .unwrap_or_default()
        .lines()
        .filter_map(|line| line.split_once('\t'))
        .filter(|(symref, _)| symref.is_empty())
        .map(|(_, name)| name.to_owned())
        .collect()
}

/// Every name `repo` already answers to as a branch: its own, and each remote's
/// with the remote's own name taken off the front.
///
/// What this is for is choosing a name nothing has, which is a different
/// question from whether one particular name is free — [`branch_exists`] is
/// that one, and it is what a refusal is written on. This is deliberately the
/// wider reading: a name only origin holds can be cut locally and is still a
/// name to leave alone, because what would be pushed to it is somebody's
/// branch, and there is no shortage of other names to pick.
///
/// Read whole rather than asked name by name, because that is what the caller
/// is doing: a walk over a thousand candidates is one `for-each-ref` here and a
/// thousand lookups in memory, against a thousand git processes the other way.
///
/// Empty for a repository git will not read, which is [`branches`]'s reading
/// and the safe one here. What follows a name chosen against nothing is a
/// worktree git refuses, said in those words; a caller that read an unreadable
/// repository as holding every name would have nothing left to choose.
pub(crate) fn cut_names(repo: &Path) -> std::collections::HashSet<String> {
    let listed = |refs: &str, format: &str| {
        git(repo, &["for-each-ref", &format!("--format={format}"), refs])
            .unwrap_or_default()
            .lines()
            .filter_map(|line| line.split_once('\t'))
            .filter(|(symref, _)| symref.is_empty())
            .map(|(_, name)| name.to_owned())
            .collect::<Vec<String>>()
    };

    // The remotes with `refs/remotes/<remote>/` off the front, so that
    // `origin/rate-limiting` is read as the name a branch would be cut under —
    // and `origin/HEAD` falls out with the other symbolic refs, being another
    // name for a branch already in the list.
    listed("refs/heads", "%(symref)\t%(refname:short)")
        .into_iter()
        .chain(listed("refs/remotes", "%(symref)\t%(refname:lstrip=3)"))
        .collect()
}

/// What fetching `repo`'s remotes came to.
///
/// Three answers rather than a boolean, because the middle one is not a
/// failure: a repository with no remote has nothing to fetch and nothing to be
/// stale against, and treating that as a fetch that did not happen would refuse
/// work on repositories that are entirely local.
pub(crate) enum Fetched {
    /// The remote-tracking refs are as fresh as the remote is.
    Fresh,

    /// There is no remote to fetch from, so there is nothing to be behind.
    NoRemote,

    /// Git would not fetch, in git's own words — offline, or an authentication
    /// that has expired, or a remote that has gone. Or, where the caller gave
    /// it a deadline, git having been stopped for running past it: a fetch that
    /// never answers is a failure like any other to whoever was waiting.
    Failed(String),
}

/// Bring `repo`'s remote-tracking refs up to what its remotes are holding.
///
/// Run before anything resolves a branch to a commit, because a
/// remote-tracking ref is only ever as fresh as the last fetch: without this a
/// Conversation comes off wherever the human's checkout last stood rather than
/// off what origin holds now.
///
/// Safe to run against a repository somebody is working in. A fetch moves
/// remote-tracking refs and nothing else — no local branch, no index, no
/// working tree — so the worst it can do to the human's own checkout is tell it
/// the truth about the remote.
///
/// A repository git will not answer about at all reads as having no remote, and
/// then fails again a moment later when its base commit will not resolve. The
/// difference is not worth splitting here: what follows either way is a refusal
/// naming something the human can go and look at.
pub(crate) fn fetch(repo: &Path) -> Fetched {
    fetching(repo, None)
}

/// The same fetch, given only so long to answer.
///
/// `GIT_TERMINAL_PROMPT` and a null stdin stop git waiting for a *human*, and
/// nothing stops it waiting for the *network*: a route that is dropping packets
/// rather than refusing them leaves `git fetch` sitting there indefinitely, and
/// git has no deadline of its own to fall back on. Whoever is reading is left
/// looking at a page that never resolves, which is worse than any answer.
///
/// So this is for the callers that are *drawing* something — where an answer
/// off what was last fetched beats no answer at all. The presses that fetch
/// have no deadline: what they do with the result is act on it, and acting on a
/// stale reading is not the same trade.
///
/// The timeout kills the fetch rather than walking away from it. Git spawns a
/// transport helper of its own and that is nearly always where a hang really
/// is, so the child is put in a process group of its own and the whole group is
/// killed — a signal to the parent alone would leave the helper running with
/// nothing left to wait for it.
pub(crate) fn fetch_within(repo: &Path, limit: Duration) -> Fetched {
    fetching(repo, Some(limit))
}

fn fetching(repo: &Path, limit: Option<Duration>) -> Fetched {
    let remotes = git(repo, &["remote"]).unwrap_or_default();

    if remotes.trim().is_empty() {
        return Fetched::NoRemote;
    }

    // Not [`crate::repos::git`]: that one throws git's stderr away, and git's
    // stderr is the whole of what a fetch that failed has to say.
    let mut command = Command::new("git");

    command
        .args(["fetch", "--all", "--quiet"])
        .current_dir(repo)
        // A fetch that wants a password must fail rather than wait for one:
        // there is nobody at this terminal to type it, and an authentication
        // that has gone is one of the two things this is expected to catch.
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let Some(limit) = limit else {
        return answered(command.output());
    };

    // A group of its own where the platform has them, so that the kill below
    // reaches the transport helper git started as well as git itself — see
    // [`stop`], which is where the whole of that difference is.
    in_its_own_group(&mut command);

    let child = match command.spawn() {
        Ok(child) => child,
        Err(error) => return Fetched::Failed(error.to_string()),
    };

    // Kept before the child is handed over, because what is killed is named by
    // it and the thread below is where the child goes.
    let fetch = child.id();

    // Waited on by a thread rather than by polling, so that the pipes are
    // drained while git is still writing to them: a fetch blocked on a full
    // pipe would look exactly like the hang this is here to end.
    let (finished, waited) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = finished.send(child.wait_with_output());
    });

    match waited.recv_timeout(limit) {
        Ok(output) => answered(output),
        Err(_) => {
            stop(fetch);

            // The thread above is left to reap it, which it does as soon as the
            // group is gone.
            Fetched::Failed(format!(
                "git did not answer within {limit:?}, so it was stopped"
            ))
        }
    }
}

/// What a fetch that ran to the end came to.
fn answered(output: std::io::Result<std::process::Output>) -> Fetched {
    let output = match output {
        Ok(output) => output,
        Err(error) => return Fetched::Failed(error.to_string()),
    };

    if output.status.success() {
        return Fetched::Fresh;
    }

    let said = String::from_utf8_lossy(&output.stderr).trim().to_owned();

    Fetched::Failed(match said.is_empty() {
        true => "git would not say why".to_owned(),
        false => said,
    })
}

/// Put the fetch in a process group of its own, on the platforms that have
/// them, so that ending it is ending everything it started — see [`stop`].
#[cfg(unix)]
fn in_its_own_group(command: &mut Command) {
    use std::os::unix::process::CommandExt;

    command.process_group(0);
}

/// And where there are none — see [`stop`]'s Windows arm, which reaches what it
/// can reach without one.
#[cfg(not(unix))]
fn in_its_own_group(_command: &mut Command) {}

/// Kill a fetch that has run past its deadline, and everything it started.
///
/// The group rather than the process: `git fetch` does its talking through a
/// transport helper it spawns, and that helper is where a stalled network
/// leaves things waiting. Signalling git alone would end the wait here and
/// leave the helper behind with nobody to reap it.
#[cfg(unix)]
fn stop(fetch: u32) {
    let Ok(group) = i32::try_from(fetch) else {
        return;
    };

    let Some(group) = rustix::process::Pid::from_raw(group) else {
        return;
    };

    if let Err(error) = rustix::process::kill_process_group(group, rustix::process::Signal::KILL) {
        tracing::warn!(
            error = ?error,
            "a fetch that ran past its deadline could not be killed"
        );
    }
}

/// The same on Windows, which has no process group of that kind to put a fetch
/// in — so the tree is what stands in for one.
///
/// **A fetch that ran past its deadline is killed here too.** What that reaches
/// is narrower than a group: `taskkill /T` walks the parent-child relationships
/// as they stand at the moment it looks, so it ends git and the transport
/// helper git started, and would miss a grandchild that had already been
/// reparented. Narrower is the whole of the difference — the process holding
/// the stalled connection is git or its helper, which is what the group was for
/// on the other platform.
///
/// `taskkill` rather than a kill of the one process, which is all the standard
/// library offers by the time the child has been handed to the thread waiting
/// on it: a helper left behind holds the socket open, and the point of the
/// deadline is that nothing is left holding one.
#[cfg(windows)]
fn stop(fetch: u32) {
    let killed = Command::new("taskkill")
        .args(["/F", "/T", "/PID"])
        .arg(fetch.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    match killed {
        Ok(status) if status.success() => {}
        Ok(status) => tracing::warn!(
            %status,
            "a fetch that ran past its deadline could not be killed"
        ),
        Err(error) => tracing::warn!(
            error = ?error,
            "a fetch that ran past its deadline could not be killed"
        ),
    }
}

/// The name an unpicked base resolves through: origin's copy of `default`,
/// where origin carries one, and the local branch where it does not.
///
/// What "the default branch" means to everybody who works on the repository is
/// what the remote is holding, not wherever the human's own copy of it last
/// stood — a local `main` that has not been pulled for a week is a week behind
/// the work the branch is meant to come off.
///
/// The local branch is the honest answer for a repository with no origin, and
/// for one whose origin does not carry a branch by that name: neither has an
/// origin copy to prefer, and there is nothing stale about the only branch
/// there is. Ask [`fetch`] first, or origin's copy is as old as the last fetch.
pub(crate) fn default_ref(repo: &Path, default: &str) -> String {
    let remote = format!("origin/{default}");

    match resolve(repo, &remote) {
        Some(_) => remote,
        None => default.to_owned(),
    }
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

/// Whether `into`'s history already holds `commit`, or `None` where git would
/// not say.
///
/// What adoption asks about the base a human fixed: a branch already merged
/// into the default branch is not a predecessor to stack on, because there is
/// nothing left of it for a stack to hold — the work is in the default branch
/// and the pull request that carried it is closed.
///
/// Asked as a merge base rather than as `--is-ancestor`, which answers by exit
/// code alone: the answer here is wanted apart from *git could not read this
/// repository*, and an exit code cannot tell the caller which of the two it got.
/// `commit` is a full commit id, as everything that resolves one here produces,
/// so it is the merge base exactly when its history is contained.
pub(crate) fn merged(repo: &Path, commit: &str, into: &str) -> Option<bool> {
    let base = git(repo, &["merge-base", "--end-of-options", commit, into])?;

    Some(base.trim() == commit)
}

/// Make the directory a worktree goes under, and say whether it is there.
///
/// Git creates the worktree directory but not the `worktrees/` above it, which
/// on a fresh install has never existed — and which the human may since have
/// taken away along with everything under it.
fn room(path: &Path) -> bool {
    let Some(parent) = path.parent() else {
        return true;
    };

    if let Err(error) = std::fs::create_dir_all(parent) {
        tracing::error!(error = ?error, path = %parent.display(), "making room for a worktree failed");
        return false;
    }

    true
}

/// Make `branch` off `commit` in `repo`, checked out at `path`.
///
/// One git call, because it is one thing: a worktree registered with the branch
/// it holds. Doing it in two would leave a branch behind whenever the checkout
/// failed.
pub(crate) fn add(repo: &Path, path: &Path, branch: &str, commit: &str) -> bool {
    if !room(path) {
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

/// Check `commit` out at `path` as a worktree of `repo`, holding no branch at
/// all.
///
/// The shape a read-only companion is given. Every other worktree Verkstead
/// makes is somewhere work will be committed, and so is cut a branch to commit
/// on; a companion that is only ever read has nothing to commit and no name to
/// take in somebody else's repository — so what it gets is git's detached
/// checkout of the commit its base resolved to.
pub(crate) fn add_detached(repo: &Path, path: &Path, commit: &str) -> bool {
    if !room(path) {
        return false;
    }

    git(
        repo,
        &[
            "worktree",
            "add",
            "--detach",
            "--end-of-options",
            &path.to_string_lossy(),
            commit,
        ],
    )
    .is_some()
}

/// Take back a worktree that was just made, and the branch it cut with it.
///
/// The undoing of an [`add`] or an [`add_detached`] that a *later* one made
/// pointless: a grill start makes the Conversation's checkout and each
/// companion's one after another, and one that will not be made refuses the
/// whole start — which has to leave nothing behind, no directory and no branch,
/// including for the ones already made.
///
/// The opposite of [`remove`] in the one way that matters: this takes the branch
/// too. Closing keeps a branch because it may hold work worth reading, and a
/// branch cut moments ago by a start that then refused holds nothing at all.
///
/// Both halves are asked whether there is anything there first, because this is
/// also what unwinds the checkout that *failed*: an [`add`] that fell over may
/// have made the directory, or the branch, or neither, and complaining about the
/// half it never got to would be complaining about the thing that went right.
///
/// Best effort, and says nothing back. It runs where something has already
/// failed and the answer to the human is already decided; what it can do about
/// a directory git will not give up is put it in the log.
pub(crate) fn unmake(repo: &Path, path: &Path, branch: Option<&str>) {
    if path.exists() && !remove(repo, path) {
        tracing::error!(
            path = %path.display(),
            "a worktree made by a start that was then refused could not be removed",
        );
    }

    let Some(branch) = branch.filter(|branch| branch_exists(repo, branch)) else {
        return;
    };

    // `-D` rather than `-d`: what is being deleted is a branch this start made a
    // moment ago, and git refusing it for being unmerged would be git refusing
    // to tidy up after work that never happened.
    if git(repo, &["branch", "-D", "--end-of-options", branch]).is_none() {
        tracing::error!(
            branch,
            repo = %repo.display(),
            "a branch cut by a start that was then refused could not be deleted",
        );
    }
}

/// The git directory `worktree` shares with the repository it was made from, in
/// full.
///
/// The *common* one, which is the repository's own `.git` rather than the
/// worktree's: what sits in a worktree is a file pointing back into
/// `…/.git/worktrees/<name>`, and a sandbox given only that would have a
/// checkout with no object database behind it. Asking git rather than joining
/// `.git` onto the Repo's path, because where a repository keeps its git
/// directory is git's answer to give — a `.git` file, a separated directory, a
/// worktree of a worktree.
///
/// `None` where git will not say, which is a directory that is not a worktree at
/// all.
pub(crate) fn common_git_dir(worktree: &Path) -> Option<PathBuf> {
    let dir = git(
        worktree,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )?;

    let dir = dir.trim();

    (!dir.is_empty()).then(|| PathBuf::from(dir))
}

/// Take the worktree at `path` away, and tell `repo` it has gone.
///
/// True when there is no longer a worktree there, which includes there never
/// having been one: closing twice is not an error, and neither is closing a
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

/// The branch checked out at `worktree`, or `None` where nothing is — a
/// detached HEAD, or a directory git will not answer about at all.
fn head(worktree: &Path) -> Option<String> {
    let head = git(worktree, &["symbolic-ref", "--quiet", "HEAD"])?;

    let head = head.trim();

    (!head.is_empty()).then(|| head.to_owned())
}

/// Whether there is still a worktree at `path` to do `repo`'s work in, on
/// `branch`.
///
/// Three things at once, and git answers all three: the directory is there, git
/// answers inside it, and what answers is this repository with the branch
/// checked out. A directory that has gone, one hollowed out, and one git no
/// longer holds a registration for all fail the same reading, because in each
/// of them the `.git` file no longer leads anywhere — and all three are a
/// Conversation with nowhere to work.
///
/// Anything short of a clear no is a yes. The one unrecoverable mistake here is
/// calling a worktree broken when it is not — what follows a no is a rebuild,
/// and a rebuild takes the directory away — so a reading that failed for
/// reasons of its own leaves the worktree alone. Which is why the repository
/// half is asked as *does this say otherwise* rather than *does this agree*.
pub(crate) fn healthy(repo: &Path, path: &Path, branch: &str) -> bool {
    // Git answering inside the directory, and where its object database is.
    let Some(inside) = common_git_dir(path) else {
        return false;
    };

    // Which had better be this Repo's, a directory belonging to some other
    // repository being no place to do this Conversation's work. Only where the
    // Repo itself reads: git saying nothing about it is a repository Verkstead
    // cannot see rather than a worktree that is wrong.
    if common_git_dir(repo).is_some_and(|ours| ours != inside) {
        return false;
    }

    head(path).is_some_and(|head| head == format!("refs/heads/{branch}"))
}

/// The name `branch` has been renamed to in the worktree at `path`, or `None`
/// where nothing there is a rename.
///
/// A session may rename the branch it is working on — the work turning out to
/// be about something other than the name it was started under — and Verkstead
/// follows that rather than repairing it. What tells a rename from a breakage
/// is one reading with two halves: the recorded branch is **gone** from the
/// repository, and the worktree's HEAD is on another branch. Both, because
/// either alone is something else entirely. A recorded branch still standing
/// while HEAD is elsewhere is a checkout that has wandered off the work, and a
/// detached HEAD holds no name to follow to; each of those is broken and
/// rebuilds the way it always has — see [`healthy`], which is the reading that
/// says so.
///
/// The mistake to avoid is the mirror of [`healthy`]'s: what follows a yes here
/// is the record moving to a name read out of a directory, so anything short of
/// a clear rename is a no. Which is why the recorded branch is asked about with
/// [`branch_taken`] rather than [`branch_exists`] — a reading that failed says
/// it may well still be standing, and a branch that is still standing is not
/// one that has been renamed.
///
/// HEAD is read first and on its own, because it settles the ordinary case in
/// one call: a worktree still on the branch it was made for is every sweep but
/// the one after a rename.
pub(crate) fn renamed(repo: &Path, path: &Path, branch: &str) -> Option<String> {
    let head = head(path)?;
    let head = head.strip_prefix("refs/heads/")?;

    if head == branch {
        return None;
    }

    // The same two questions [`healthy`] asks about the directory: git answers
    // inside it, and what answers is this Repo. A worktree of somebody else's
    // repository is no place to read this Conversation's branch name off.
    let inside = common_git_dir(path)?;

    if common_git_dir(repo).is_some_and(|ours| ours != inside) {
        return None;
    }

    if branch_taken(repo, branch) {
        return None;
    }

    Some(head.to_owned())
}

/// Rename whatever branch is checked out at `path` to `branch`, and say whether
/// it took.
///
/// Run in the worktree rather than in the repository, and given one name rather
/// than two: what is being renamed is whatever that checkout is on. This is how
/// a mirroring companion's branch is made to match the Conversation's after the
/// Conversation's own has moved, and a companion checkout is the only thing that
/// knows what name it is currently under.
pub(crate) fn rename(path: &Path, branch: &str) -> bool {
    // Already there is already done, and said here rather than left to git:
    // this is asked again wherever the act it is part of did not finish, and
    // what git makes of being asked to rename a branch to the name it already
    // has is not something to have an opinion about.
    if head(path).is_some_and(|head| head == format!("refs/heads/{branch}")) {
        return true;
    }

    git(path, &["branch", "-m", "--end-of-options", branch]).is_some()
}

/// Make the worktree at `path` again, checked out on `branch`.
///
/// A worktree is derived state: the branch holds everything that was committed,
/// so a rebuilt one has lost nothing git could still have reported. What is in
/// the way goes first — git's own removal where git still knows the directory,
/// and the directory itself where it does not, that being the only case in
/// which nothing there can be reported on.
///
/// The prune is what clears a registration that outlived its directory, which
/// is the state that leaves git refusing to check the branch out anywhere.
pub(crate) fn rebuild(repo: &Path, path: &Path, branch: &str) -> bool {
    if path.exists() && !remove(repo, path) {
        // Git would not have it. Where git can still report on what is there,
        // that is a worktree this has no business deleting by hand — and where
        // it cannot, the directory is the whole of what is in the way.
        if common_git_dir(path).is_some() {
            tracing::error!(path = %path.display(), "git refused to remove a worktree, so it is not being rebuilt");
            return false;
        }

        if let Err(error) = std::fs::remove_dir_all(path) {
            tracing::error!(error = ?error, path = %path.display(), "clearing the way for a worktree failed");
            return false;
        }
    }

    // The registration outlives the directory, and git will not check a branch
    // out that it believes is already checked out somewhere.
    git(repo, &["worktree", "prune"]);

    if !room(path) {
        return false;
    }

    let made = git(
        repo,
        &[
            "worktree",
            "add",
            "--end-of-options",
            &path.to_string_lossy(),
            branch,
        ],
    )
    .is_some();

    match made {
        true => {
            tracing::info!(path = %path.display(), branch, "a broken worktree was rebuilt from its branch")
        }
        false => {
            tracing::error!(path = %path.display(), branch, "a broken worktree could not be rebuilt")
        }
    }

    made
}

/// Take back every directory under the worktrees directory that no Conversation
/// is working in any more.
///
/// Closing already removes a Conversation's own checkouts, politely and by name.
/// This is the backstop under that: git refuses to remove a directory it no
/// longer reads as a worktree, and a close that hit one logs the path and closes
/// around it — see [`crate::conversations::close`] — while a server that died
/// mid-run had no close at all. Neither leaves anything that would ever come
/// back for the directory. This is what comes back for it.
///
/// **A directory is kept exactly when a record names it.** Closing deletes both
/// worktree rows, so *there is a row for it* and *a live Conversation still
/// works in it* are one statement — see [`store::recorded_worktrees`], which is
/// the whole of the rule. Everything else under this directory is orphaned by
/// definition: it is Verkstead's own data directory, made for exactly one thing,
/// and nothing but a checkout has any business in it.
///
/// **Nothing is deleted on a reading that failed.** A store error would come
/// back as an empty keep-set, and an empty keep-set is every live worktree there
/// is — so the error ends the sweep instead, and the orphans wait for the next
/// one. That is the shape of every judgement here: the unrecoverable mistake is
/// deleting work somebody is still doing, and an orphan left behind is swept
/// again in a minute.
pub(crate) async fn sweep(state: &crate::AppState) {
    // Held across the read and everything it decides, because the window between
    // a checkout being made and the record naming it is exactly the window in
    // which a keep-set is a lie — see [`crate::AppState::checkouts`], which the
    // other end of the same window takes.
    let _checkouts = state.checkouts.lock().await;

    swept(&state.pool, &state.data_dir).await;
}

/// The same sweep, off what it needs rather than off the whole of the server:
/// the store to ask, and the directory to sweep.
///
/// Split from [`sweep`] so that the refusal above it is something a test can
/// reach — a store with the table taken out from under it is a reading that
/// failed, and *that deletes nothing* is the one behaviour here worth proving
/// twice.
async fn swept(pool: &sqlx::SqlitePool, data: &Path) {
    let kept = match store::recorded_worktrees(pool).await {
        Ok(kept) => kept,
        Err(error) => {
            tracing::error!(
                error = ?error,
                "reading which worktrees are still a Conversation's failed, so none are being swept",
            );
            return;
        }
    };

    // Where the registrations left behind are cleared afterwards. Read under the
    // same rule as the keep-set: a sweep that cannot say what it is working over
    // does not start.
    let repos = match store::recorded_repos(pool).await {
        Ok(repos) => repos,
        Err(error) => {
            tracing::error!(
                error = ?error,
                "listing where the Repos are failed, so the orphaned worktrees are not being swept",
            );
            return;
        }
    };

    let data = data.to_owned();

    // Every part of this blocks — a directory listing, a git call per orphan and
    // a tree deleted — so it goes off the runtime's threads, as the removals a
    // close makes do.
    if let Err(error) = tokio::task::spawn_blocking(move || sweeping(&data, &kept, &repos)).await {
        tracing::error!(error = ?error, "sweeping the orphaned worktrees failed");
    }
}

/// The filesystem half: what is under the worktrees directory, held against what
/// the records name, with everything else taken away. Hands back what it
/// deleted.
///
/// **Every candidate comes out of reading this one directory**, and nothing
/// else is ever a candidate. No path is built from git's output, from a record
/// or from anything a request carried — the only thing a record does here is
/// save a directory, never name one to delete. Which is what makes the boundary
/// checkable at all: one parent, resolved once, and every deletion an immediate
/// child of it.
///
/// Every reading an entry is put through asks one question of a different
/// thing — *is this certainly an orphan under this directory?* — and any of
/// them coming back short of a yes leaves the entry exactly where it is. A
/// reading that failed is one of those: it is not a reading that says *delete*.
fn sweeping(data: &Path, kept: &[PathBuf], repos: &[PathBuf]) -> Vec<PathBuf> {
    let mut swept = Vec::new();

    // A router with no data directory has nowhere to have put a worktree, and
    // the empty path would resolve to the working directory — which is somebody
    // else's. See [`crate::nowhere`].
    if data.as_os_str().is_empty() {
        return swept;
    }

    let worktrees = directory(data);

    // Resolved once, and it is the whole of the boundary: every deletion below
    // is checked against this rather than against the path it was joined from,
    // so a link anywhere above cannot move it. A directory that will not resolve
    // is one that is not there — a fresh install, or a data directory the human
    // has taken away — and there is nothing in it to sweep.
    let Ok(root) = worktrees.canonicalize() else {
        return swept;
    };

    let entries = match std::fs::read_dir(&worktrees) {
        Ok(entries) => entries,
        Err(error) => {
            tracing::error!(
                error = ?error,
                path = %worktrees.display(),
                "the worktrees directory could not be read, so nothing is being swept",
            );
            return swept;
        }
    };

    // Each record as it was written and as it resolves. Both, because they are
    // the same directory under two names as soon as anything above it is a link
    // or has been moved, and a keep-set that missed one of them would be a
    // keep-set that deleted live work.
    let recorded: Vec<(PathBuf, Option<PathBuf>)> = kept
        .iter()
        .map(|path| (path.clone(), path.canonicalize().ok()))
        .collect();

    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                tracing::error!(error = ?error, path = %worktrees.display(), "an entry of the worktrees directory could not be read, so it is being left alone");
                continue;
            }
        };

        let path = entry.path();

        // What `read_dir` hands back is an entry of this directory, said out
        // loud rather than trusted: everything below deletes, and a candidate
        // that is not a child of the directory that was resolved is one no
        // check here describes.
        if path.parent() != Some(worktrees.as_path()) {
            tracing::error!(path = %path.display(), "a worktrees entry is not a child of the worktrees directory, so it is being left alone");
            continue;
        }

        // A link is deleted as a link and never followed. `remove_dir_all` on
        // one walks what is on the other end, which is by definition somewhere
        // else — and somewhere else is the one place nothing here may touch.
        let linked = match path.symlink_metadata() {
            Ok(metadata) => metadata.is_symlink(),
            Err(error) => {
                tracing::error!(error = ?error, path = %path.display(), "a worktrees entry could not be read, so it is being left alone");
                continue;
            }
        };

        if linked {
            if names(&recorded, &path, None) {
                continue;
            }

            match std::fs::remove_file(&path) {
                Ok(()) => {
                    tracing::info!(path = %path.display(), "a link in the worktrees directory that no Conversation names was removed");
                    swept.push(path);
                }
                Err(error) => {
                    tracing::error!(error = ?error, path = %path.display(), "a link in the worktrees directory could not be removed");
                }
            }

            continue;
        }

        // And where it actually is, which is what the boundary is checked
        // against. Nothing that will not resolve is deleted: a reading that
        // failed is not a reading that says *outside*.
        let resolved = match path.canonicalize() {
            Ok(resolved) => resolved,
            Err(error) => {
                tracing::error!(error = ?error, path = %path.display(), "a worktrees entry could not be resolved, so it is being left alone");
                continue;
            }
        };

        if resolved.parent() != Some(root.as_path()) {
            tracing::error!(path = %path.display(), resolved = %resolved.display(), "a worktrees entry resolves outside the worktrees directory, so it is being left alone");
            continue;
        }

        if names(&recorded, &path, Some(&resolved)) {
            continue;
        }

        if discard(&path) {
            tracing::info!(path = %path.display(), "an orphaned worktree that no Conversation names was deleted");
            swept.push(path);
        }
    }

    // And then the registrations git is still holding for directories that have
    // gone. Every Repo on record rather than the ones this could name: a
    // directory hollowed out no longer says which repository made it, and the
    // registration it leaves is what has git refusing to check that branch out
    // anywhere later. The ones taken away with them — see
    // [`store::recorded_repos`], an unregistering leaving the repository and its
    // registrations exactly where they were. Only where something went — a sweep
    // that found nothing has left nothing stale.
    if !swept.is_empty() {
        for repo in repos {
            git(repo, &["worktree", "prune"]);
        }
    }

    swept
}

/// Whether any record names `path`, or anything inside it.
///
/// Both readings of every record, and either matching is enough to keep the
/// directory. `resolved` is what the candidate resolves to, or `None` for a
/// candidate that is a symlink — nothing there was followed, so there is nothing
/// resolved to compare.
///
/// Anything *inside* rather than the directory itself, because the rule is
/// *keep what a record could be talking about*. Every path Verkstead writes is
/// an immediate child of the worktrees directory — see [`worktree_path`] — so
/// the wider reading costs nothing today and is the safe way to be wrong if a
/// record ever holds something deeper.
fn names(recorded: &[(PathBuf, Option<PathBuf>)], path: &Path, resolved: Option<&Path>) -> bool {
    recorded.iter().any(|(named, at)| {
        named.starts_with(path)
            || match (at, resolved) {
                (Some(at), Some(resolved)) => at.starts_with(resolved),
                _ => false,
            }
    })
}

/// Take one orphan away: git's own removal where the directory still says which
/// repository it belongs to, and the directory itself where it does not or where
/// git will not have it.
///
/// Forceful, which is the whole point of sweeping at all. What is left under the
/// worktrees directory after a close is precisely what the close's own polite
/// removal already failed on, so a sweep that asked as nicely would reclaim
/// exactly nothing. [`remove`] is already the forceful one and already prunes
/// the repository it removed from, which is why the polite half is a call to it
/// rather than a second way of saying the same thing.
///
/// The directory is deleted outright where git would not name it. That is the
/// state this exists for: a `.git` file that has gone, or a directory that was
/// never a worktree — and either way it is unrecorded, under Verkstead's own
/// data directory, which is the definition of something to reclaim.
fn discard(path: &Path) -> bool {
    if let Some(repo) = common_git_dir(path)
        && remove(&repo, path)
    {
        return true;
    }

    // A directory or something that is not one. A stray file under here is as
    // unrecorded as a stray checkout and goes the same way; what it must not do
    // is fail a `remove_dir_all` and be reported as a worktree that would not go.
    let taken = match path.is_dir() {
        true => std::fs::remove_dir_all(path),
        false => std::fs::remove_file(path),
    };

    match taken {
        Ok(()) => true,
        Err(error) => {
            tracing::error!(error = ?error, path = %path.display(), "an orphaned worktree could not be deleted");
            false
        }
    }
}

/// Sweep once, as the server comes up.
///
/// The other place orphans come from. A close sweeps after itself, so what is
/// on disk unrecorded when a server starts is what the last one never got to: a
/// process that died between making a checkout and recording it, or between
/// being asked to remove one and removing it. Nothing else will ever look at
/// those, because nothing else knows they are there.
///
/// Started rather than waited on, and nothing waits on it: what it is racing is
/// [`crate::resume::at_startup`] rebuilding the checkouts a restart left broken,
/// and those are recorded, so the keep-set holds them whichever of the two looks
/// first.
pub(crate) fn at_startup(state: &crate::AppState) {
    let state = state.clone();

    tokio::spawn(async move { sweep(&state).await });
}

#[cfg(test)]
mod tests {
    use std::io::Read;
    use std::net::{TcpListener, TcpStream};

    use super::*;

    /// A remote that accepts the connection and then says nothing, ever.
    ///
    /// Which is the hang this is all about: a route that drops packets rather
    /// than refusing them leaves git waiting on a socket that will never
    /// answer, with no deadline of its own to fall back on. Hands back the
    /// `git://` url to point a remote at, and the accepted connection — reading
    /// EOF off it is how the test knows git really died rather than being
    /// walked away from.
    fn a_remote_that_never_answers() -> (String, std::thread::JoinHandle<TcpStream>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();

        let held = std::thread::spawn(move || listener.accept().unwrap().0);

        (format!("git://127.0.0.1:{port}/repo"), held)
    }

    /// A fetch that would wait for ever answers inside its deadline, and git is
    /// stopped rather than left running.
    ///
    /// Both halves matter. Answering is what keeps the page the fetch is read
    /// behind from hanging; killing is what keeps a server that has drawn that
    /// page a hundred times from holding a hundred stalled gits.
    #[test]
    fn a_fetch_given_a_deadline_answers_at_it_and_kills_the_git_it_stopped() {
        let (_dir, repo) = repository();
        let (url, held) = a_remote_that_never_answers();
        run(&repo, &["remote", "add", "origin", &url]);

        let started = std::time::Instant::now();
        let fetched = fetch_within(&repo, Duration::from_millis(500));

        let Fetched::Failed(said) = fetched else {
            panic!("a fetch that never answers is a fetch that failed");
        };

        assert!(said.contains("did not answer"), "it said {said:?}");
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "the deadline is what ends the wait, not the fetch"
        );

        // And the far end sees the connection go, which nothing but the death
        // of every process holding it could do.
        let mut connection = held.join().unwrap();
        connection
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();

        let mut read = Vec::new();

        match connection.read_to_end(&mut read) {
            // Gone is gone, and the two platforms say it differently: a socket
            // that went with the process holding it ends the stream on Unix and
            // arrives as a reset on Windows. Anything else — the read timeout
            // above all — is a git still on the other end of it.
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::ConnectionReset => {}
            Err(error) => panic!("the connection outlived the git that held it: {error}"),
        }
    }

    /// And a repository with a remote that answers is fetched as it always was:
    /// the deadline is a ceiling on the wait rather than a change to the fetch.
    #[test]
    fn a_fetch_inside_its_deadline_is_an_ordinary_fetch() {
        let (dir, upstream) = repository();
        let clone = dir.path().join("clone");

        run(
            dir.path(),
            &[
                "clone",
                &upstream.to_string_lossy(),
                &clone.to_string_lossy(),
            ],
        );

        assert!(matches!(
            fetch_within(&clone, Duration::from_secs(30)),
            Fetched::Fresh
        ));

        // And one with nowhere to fetch from still says so rather than waiting.
        assert!(matches!(
            fetch_within(&upstream, Duration::from_secs(30)),
            Fetched::NoRemote
        ));
    }

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

    /// A repository with the branch, without it, and a directory that is not a
    /// repository at all — which is the reading that failed, and the one this
    /// answers differently from [`branch_exists`].
    #[test]
    fn a_branch_reading_that_failed_counts_as_a_branch_that_is_taken() {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path();

        run(repo, &["init", "--initial-branch", "main"]);
        run(repo, &["config", "user.email", "test@verkstead.invalid"]);
        run(repo, &["config", "user.name", "Verkstead Test"]);
        std::fs::write(repo.join("README.md"), "# a repository\n").unwrap();
        run(repo, &["add", "-A"]);
        run(repo, &["commit", "-m", "chore: something to branch from"]);

        assert!(!branch_taken(repo, "wrap-up"), "nothing is on it");

        run(repo, &["branch", "wrap-up"]);

        assert!(branch_taken(repo, "wrap-up"), "and now something is");

        // Git says nothing at all here, which is not git saying the name is
        // free: what is on the other side of this answer is making a branch and
        // letting an agent loose on it.
        let nowhere = tempfile::tempdir().unwrap();

        assert!(branch_taken(nowhere.path(), "wrap-up"));
        assert!(
            !branch_exists(nowhere.path(), "wrap-up"),
            "which is the whole difference between the two readings",
        );
    }

    /// The names a start has to pick around: the local branches and the remote
    /// ones as they would be cut, with the remote's own name off the front.
    ///
    /// A name only origin holds is one to leave alone — cutting it locally
    /// works, and then the push behind it is into somebody's branch.
    #[test]
    fn the_names_a_repository_answers_to_are_its_locals_and_its_remotes() {
        let (_dir, repo) = repository();
        let tip = resolve(&repo, "main").expect("the branch resolves");

        run(&repo, &["branch", "rate-limiting"]);
        run(&repo, &["branch", "feature/throttling"]);
        run(
            &repo,
            &["update-ref", "refs/remotes/origin/pushed-elsewhere", &tip],
        );
        run(&repo, &["update-ref", "refs/remotes/origin/main", &tip]);
        run(
            &repo,
            &[
                "symbolic-ref",
                "refs/remotes/origin/HEAD",
                "refs/remotes/origin/main",
            ],
        );

        let names = cut_names(&repo);

        assert!(names.contains("main"));
        assert!(names.contains("rate-limiting"));
        assert!(names.contains("feature/throttling"), "slashes and all");
        assert!(
            names.contains("pushed-elsewhere"),
            "a remote's branch under the name it would be cut as",
        );
        assert!(
            !names.contains("origin/pushed-elsewhere"),
            "and not under the remote's own name for it",
        );
        assert!(
            !names.contains("HEAD"),
            "origin/HEAD is another name for a branch already in the list",
        );
        assert!(!names.contains("hushed-otter"), "and nothing else is in it");

        // A directory git will not read holds no names, which is what leaves a
        // caller something to choose.
        let nowhere = tempfile::tempdir().unwrap();

        assert!(cut_names(nowhere.path()).is_empty());
    }

    /// A worktree git still answers about, on the branch it was made for, is
    /// one to work in — and it stays one when there is uncommitted work in it.
    ///
    /// Which is the case validation exists to leave alone: a session that died
    /// mid-edit leaves exactly this, and rebuilding it would throw away the only
    /// copy of what it had written.
    #[test]
    fn a_worktree_git_answers_about_is_left_alone() {
        let (dir, repo) = repository();
        let path = dir.path().join("worktrees/verkstead-rate-limiting");

        assert!(add(&repo, &path, "rate-limiting", "HEAD"));

        std::fs::write(path.join("half-written.rs"), "// as far as it got\n").unwrap();

        assert!(healthy(&repo, &path, "rate-limiting"));
    }

    /// The three ways a worktree stops being one, and the rebuild that answers
    /// each: the directory deleted, the directory hollowed out, and the
    /// registration dropped from the repository — which is the one that has a
    /// Conversation stuck under a Resume that cannot work.
    #[test]
    fn a_worktree_that_is_no_longer_one_is_rebuilt_from_its_branch() {
        for broken in ["deleted", "hollowed", "deregistered"] {
            let (dir, repo) = repository();
            let path = dir.path().join("worktrees/verkstead-rate-limiting");

            assert!(add(&repo, &path, "rate-limiting", "HEAD"));

            run(&path, &["config", "user.email", "test@verkstead.invalid"]);
            run(&path, &["config", "user.name", "Verkstead Test"]);
            std::fs::write(path.join("counter.rs"), "// the work so far\n").unwrap();
            run(&path, &["add", "-A"]);
            run(&path, &["commit", "-m", "feat: count the requests"]);

            match broken {
                "deleted" => std::fs::remove_dir_all(&path).unwrap(),
                "hollowed" => std::fs::remove_file(path.join(".git")).unwrap(),
                _ => std::fs::remove_dir_all(repo.join(".git/worktrees/verkstead-rate-limiting"))
                    .unwrap(),
            }

            assert!(
                !healthy(&repo, &path, "rate-limiting"),
                "a {broken} worktree is nowhere to do the work",
            );
            assert!(
                rebuild(&repo, &path, "rate-limiting"),
                "so it is made again from the branch: {broken}",
            );
            assert!(
                healthy(&repo, &path, "rate-limiting"),
                "and now it is somewhere to do the work: {broken}",
            );
            assert!(
                path.join("counter.rs").exists(),
                "with everything the branch was holding: {broken}",
            );
        }
    }

    /// A directory belonging to some other repository is no place to do this
    /// Conversation's work, whatever git says about it in its own terms.
    #[test]
    fn a_worktree_of_another_repository_is_not_this_one_to_work_in() {
        let (dir, repo) = repository();
        let (elsewhere, other) = repository();

        let path = elsewhere.path().join("worktrees/verkstead-rate-limiting");

        assert!(add(&other, &path, "rate-limiting", "HEAD"));

        assert!(healthy(&other, &path, "rate-limiting"));
        assert!(!healthy(&repo, &path, "rate-limiting"));

        drop(dir);
    }

    /// And a rebuild that cannot happen says so rather than leaving the caller
    /// believing there is a worktree there.
    ///
    /// Something that is not a directory at all sitting where the worktree goes
    /// is the plainest way to have one: it cannot be removed as a worktree, it
    /// is not a directory to take away, and git will not check a branch out over
    /// it.
    #[test]
    fn a_rebuild_that_cannot_clear_the_way_refuses() {
        let (dir, repo) = repository();
        let path = dir.path().join("worktrees/verkstead-rate-limiting");

        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "not a worktree\n").unwrap();

        assert!(!rebuild(&repo, &path, "rate-limiting"));
    }

    /// A branch renamed in the worktree reads as a rename, and the reading says
    /// what it was renamed to.
    ///
    /// Which is what a session is asked to do with a name Verkstead invented,
    /// so this is the ordinary end of the first session of most Conversations
    /// rather than an edge of anything.
    #[test]
    fn a_branch_renamed_in_its_worktree_reads_as_a_rename() {
        let (dir, repo) = repository();
        let path = dir.path().join("worktrees/verkstead-verkstead-7f3a");

        assert!(add(&repo, &path, "verkstead-7f3a", "HEAD"));
        assert_eq!(renamed(&repo, &path, "verkstead-7f3a"), None, "nothing yet");

        run(&path, &["branch", "-m", "rate-limiting"]);

        assert_eq!(
            renamed(&repo, &path, "verkstead-7f3a").as_deref(),
            Some("rate-limiting"),
        );
        assert!(
            !healthy(&repo, &path, "verkstead-7f3a"),
            "which is a checkout the record can no longer describe",
        );
        assert!(
            healthy(&repo, &path, "rate-limiting"),
            "and one the followed record describes exactly",
        );
        assert!(
            path.exists(),
            "the directory keeps the name it was made with: it is cosmetic, and \
             moving a live worktree is another way to fail",
        );
    }

    /// The two mismatches that are not renames, and the one thing they have in
    /// common: there is no name to follow to.
    ///
    /// A recorded branch still standing while HEAD is elsewhere is a checkout
    /// that wandered off the work rather than a branch that moved, and a
    /// detached HEAD holds no branch name at all. Both are what they always
    /// were — broken, and rebuilt from the branch the record names.
    #[test]
    fn a_mismatch_that_is_not_a_rename_is_still_broken() {
        for wandered in ["another branch", "a detached HEAD"] {
            let (dir, repo) = repository();
            let path = dir.path().join("worktrees/verkstead-rate-limiting");

            assert!(add(&repo, &path, "rate-limiting", "HEAD"));

            match wandered {
                "another branch" => run(&path, &["checkout", "-b", "elsewhere"]),
                _ => run(&path, &["checkout", "--detach"]),
            }

            assert_eq!(
                renamed(&repo, &path, "rate-limiting"),
                None,
                "{wandered} is not a rename",
            );
            assert!(
                !healthy(&repo, &path, "rate-limiting"),
                "so it reads as broken: {wandered}",
            );
            assert!(
                rebuild(&repo, &path, "rate-limiting"),
                "and it rebuilds on the branch the record names: {wandered}",
            );
            assert!(healthy(&repo, &path, "rate-limiting"), "{wandered}");
            assert_eq!(
                path.file_name().unwrap(),
                "verkstead-rate-limiting",
                "with the directory still called what it was called: {wandered}",
            );
        }
    }

    /// A worktree of somebody else's repository is no place to read this
    /// Conversation's branch name off, whatever it says about itself.
    #[test]
    fn a_worktree_of_another_repository_is_not_a_rename_of_this_one() {
        let (dir, repo) = repository();
        let (elsewhere, other) = repository();

        let path = elsewhere.path().join("worktrees/verkstead-rate-limiting");

        assert!(add(&other, &path, "rate-limiting", "HEAD"));
        run(&path, &["branch", "-m", "renamed-over-there"]);

        assert_eq!(renamed(&repo, &path, "rate-limiting"), None);
        assert_eq!(
            renamed(&other, &path, "rate-limiting").as_deref(),
            Some("renamed-over-there"),
            "which is the whole of the difference: the same directory, asked \
             about by the repository it belongs to",
        );

        drop(dir);
    }

    /// A rename is what a checkout is on rather than a pair of names, so one
    /// already on the name it is being renamed to is one there is nothing to do
    /// about — and git, asked to rename a branch to its own name, refuses.
    #[test]
    fn renaming_a_checkout_to_the_name_it_already_has_is_done_already() {
        let (dir, repo) = repository();
        let path = dir.path().join("worktrees/verkstead-companion");

        assert!(add(&repo, &path, "verkstead-7f3a", "HEAD"));

        assert!(rename(&path, "rate-limiting"));
        assert!(branch_exists(&repo, "rate-limiting"));
        assert!(!branch_exists(&repo, "verkstead-7f3a"));

        assert!(
            rename(&path, "rate-limiting"),
            "asked again, it is still yes"
        );
    }

    /// The whole of the sweep's rule in one run: the directory a record names
    /// stays, and the one no record names goes — along with the registration
    /// git was holding for it.
    #[test]
    fn a_worktree_a_conversation_names_survives_a_sweep_and_one_it_does_not_goes() {
        let (dir, repo) = repository();
        let kept = dir.path().join("worktrees/verkstead-rate-limiting");
        let orphan = dir.path().join("worktrees/verkstead-abandoned");

        assert!(add(&repo, &kept, "rate-limiting", "HEAD"));
        assert!(add(&repo, &orphan, "abandoned", "HEAD"));

        let swept = sweeping(
            dir.path(),
            std::slice::from_ref(&kept),
            std::slice::from_ref(&repo),
        );

        assert_eq!(swept, vec![orphan.clone()]);
        assert!(kept.exists(), "a Conversation is still working in it");
        assert!(!orphan.exists(), "and nothing is working in this one");
        assert!(
            !repo.join(".git/worktrees/verkstead-abandoned").exists(),
            "the registration goes with the directory",
        );
    }

    /// The one this exists for: a directory git no longer reads as a worktree,
    /// which is exactly what a close logs and closes around.
    ///
    /// Git's own removal is asked first and refuses, as it refused at the close;
    /// what reclaims the directory is the sweep deleting it outright, and the
    /// prune afterwards is what clears the registration nothing else would have.
    #[test]
    fn a_worktree_git_will_not_remove_is_swept_anyway() {
        let (dir, repo) = repository();
        let path = dir.path().join("worktrees/verkstead-hollowed");

        assert!(add(&repo, &path, "hollowed", "HEAD"));
        std::fs::remove_file(path.join(".git")).unwrap();

        assert!(
            !remove(&repo, &path),
            "which is the state the close's own removal leaves behind",
        );

        let swept = sweeping(dir.path(), &[], std::slice::from_ref(&repo));

        assert_eq!(swept, vec![path.clone()]);
        assert!(!path.exists());
        assert!(
            !repo.join(".git/worktrees/verkstead-hollowed").exists(),
            "and the prune cleared what git was still holding for it",
        );
    }

    /// Everything unrecorded goes, whether or not git has ever heard of it. This
    /// is Verkstead's own data directory: a directory that was never a checkout
    /// and a file that was never anything are both something nobody named.
    #[test]
    fn what_was_never_a_worktree_at_all_is_swept_too() {
        let (dir, repo) = repository();
        let worktrees = dir.path().join("worktrees");

        std::fs::create_dir_all(worktrees.join("a-directory/deeper")).unwrap();
        std::fs::write(worktrees.join("a-directory/deeper/notes.md"), "left\n").unwrap();
        std::fs::write(worktrees.join("a-file"), "left\n").unwrap();

        let mut swept = sweeping(dir.path(), &[], std::slice::from_ref(&repo));
        swept.sort();

        assert_eq!(
            swept,
            vec![worktrees.join("a-directory"), worktrees.join("a-file")],
        );
    }

    /// A link is deleted as a link and never followed. What is on the other end
    /// is by definition outside the one directory this may delete inside, so
    /// walking it would be the whole of the mistake this is written against.
    ///
    /// On the platforms where a test may make a link at all — Windows wants a
    /// privilege for one, and the sweep's own reasoning is the same there.
    #[cfg(unix)]
    #[test]
    fn a_link_under_the_worktrees_directory_goes_as_a_link() {
        let (dir, repo) = repository();
        let worktrees = dir.path().join("worktrees");

        std::fs::create_dir_all(&worktrees).unwrap();

        let elsewhere = dir.path().join("somebody-elses");
        std::fs::create_dir_all(&elsewhere).unwrap();
        std::fs::write(elsewhere.join("their-work.md"), "not Verkstead's\n").unwrap();

        let link = worktrees.join("a-link");
        std::os::unix::fs::symlink(&elsewhere, &link).unwrap();

        let swept = sweeping(dir.path(), &[], std::slice::from_ref(&repo));

        assert_eq!(swept, vec![link.clone()]);
        assert!(
            link.symlink_metadata().is_err(),
            "the link itself is what was removed",
        );
        assert!(
            elsewhere.join("their-work.md").exists(),
            "and what it pointed at is untouched",
        );
    }

    /// A store that will not answer deletes nothing. An error read as an empty
    /// keep-set would be every live worktree there is, which is the one mistake
    /// here that cannot be undone by sweeping again.
    #[tokio::test]
    async fn a_keep_set_that_could_not_be_read_sweeps_nothing() {
        let (dir, repo) = repository();
        let orphan = dir.path().join("worktrees/verkstead-abandoned");

        assert!(add(&repo, &orphan, "abandoned", "HEAD"));

        let pool = store::open_database(&dir.path().join("verkstead.db"))
            .await
            .unwrap();

        // The table the keep-set is read out of, taken out from under it: any
        // way of failing would do, and this is the one a test can arrange.
        sqlx::query("DROP TABLE worktrees")
            .execute(&pool)
            .await
            .unwrap();

        swept(&pool, dir.path()).await;

        assert!(
            orphan.exists(),
            "nothing is deleted on a reading that failed",
        );
    }

    /// Both tables are the keep-set, so a companion's checkout is as safe as the
    /// Conversation's own — and the orphan beside them still goes.
    ///
    /// End to end through the store rather than over a list of paths, because
    /// what is being proved is which rows are read: a keep-set built from
    /// `worktrees` alone would delete every companion checkout on the machine.
    #[tokio::test]
    async fn a_companion_checkout_is_kept_by_the_row_that_names_it() {
        let (dir, repo) = repository();
        let (beside, companion) = repository();

        let pool = store::open_database(&dir.path().join("verkstead.db"))
            .await
            .unwrap();

        let registered = store::register_repo(&pool, &repo, "verkstead", "main")
            .await
            .unwrap()
            .expect("nothing is registered at that path yet");
        let alongside = store::register_repo(&pool, &companion, "askance", "main")
            .await
            .unwrap()
            .expect("nothing is registered at that path yet");

        let id = store::start_conversation(&pool, registered.id, "rate-limiting")
            .await
            .unwrap()
            .expect("the Repo was just registered");

        store::add_companion(&pool, id, alongside.id).await.unwrap();

        let own = dir.path().join("worktrees/verkstead-rate-limiting");
        let alongside_path = dir.path().join("worktrees/askance-rate-limiting");
        let orphan = dir.path().join("worktrees/verkstead-abandoned");

        assert!(add(&repo, &own, "rate-limiting", "HEAD"));
        assert!(add(&companion, &alongside_path, "rate-limiting", "HEAD"));
        assert!(add(&repo, &orphan, "abandoned", "HEAD"));

        store::start_grilling(
            &pool,
            id,
            "6f32b11a0c4d1e8f5b3a97c2d0e4f6a8b1c3d5e7",
            &own,
            &[store::CompanionWorktree {
                repo_id: alongside.id,
                path: alongside_path.clone(),
                base_commit: None,
            }],
        )
        .await
        .unwrap();

        swept(&pool, dir.path()).await;

        assert!(own.exists(), "the Conversation's own checkout");
        assert!(alongside_path.exists(), "and the companion's beside it");
        assert!(!orphan.exists(), "and the orphan between them is gone");

        drop(beside);
    }

    /// A Repo the human took away is pruned like any other. Unregistering
    /// leaves the repository exactly where it was, so a directory this deletes
    /// out of one leaves git holding a registration — and nothing else would
    /// ever clear it, while git goes on refusing that branch a checkout
    /// anywhere.
    ///
    /// Hollowed out, because that is the state where the prune is the only
    /// thing that would: a directory that cannot say which repository made it
    /// is one [`discard`] deletes outright.
    #[tokio::test]
    async fn a_repo_that_was_taken_away_is_pruned_like_any_other() {
        let (dir, repo) = repository();
        let orphan = dir.path().join("worktrees/verkstead-abandoned");

        assert!(add(&repo, &orphan, "abandoned", "HEAD"));
        std::fs::remove_file(orphan.join(".git")).unwrap();

        let pool = store::open_database(&dir.path().join("verkstead.db"))
            .await
            .unwrap();

        let registered = store::register_repo(&pool, &repo, "verkstead", "main")
            .await
            .unwrap()
            .expect("nothing is registered at that path yet");

        assert_eq!(
            store::unregister_repo(&pool, registered.id).await.unwrap(),
            store::Unregistering::Unregistered,
        );

        swept(&pool, dir.path()).await;

        assert!(!orphan.exists(), "the orphan goes as it always did");
        assert!(
            !repo.join(".git/worktrees/verkstead-abandoned").exists(),
            "and the registration goes with it, registry or no registry",
        );
    }

    /// A router with no data directory sweeps nothing. The empty path would
    /// resolve against the working directory, which is somebody else's — see
    /// [`crate::nowhere`].
    #[test]
    fn a_server_with_no_data_directory_sweeps_nothing() {
        assert!(sweeping(Path::new(""), &[], &[]).is_empty());
    }
    /// Whether the default branch has already swallowed a branch, which is what
    /// says a predecessor is finished work rather than something to stack on.
    #[test]
    fn a_branch_is_merged_once_the_default_branch_holds_its_commits() {
        let (_dir, repo) = repository();

        run(&repo, &["checkout", "-q", "-b", "feature"]);
        std::fs::write(repo.join("feature.md"), "# a feature\n").unwrap();
        run(&repo, &["add", "-A"]);
        run(&repo, &["commit", "-m", "feat: the feature"]);

        let tip = resolve(&repo, "feature").expect("the branch resolves");

        assert_eq!(
            merged(&repo, &tip, "main"),
            Some(false),
            "unmerged, which is a predecessor with something left to stack on",
        );

        run(&repo, &["checkout", "-q", "main"]);
        run(
            &repo,
            &["merge", "-q", "--no-ff", "-m", "merge it", "feature"],
        );

        assert_eq!(
            merged(&repo, &tip, "main"),
            Some(true),
            "and merged, which is work that has landed",
        );

        // The default branch's own tip is in its own history, which is the
        // answer that keeps a stage off the default branch from stacking on it.
        let main = resolve(&repo, "main").expect("main resolves");

        assert_eq!(merged(&repo, &main, "main"), Some(true));
    }

    /// And a repository git will not read says nothing rather than saying no —
    /// the difference the caller turns on, because *no* is what makes it stack.
    #[test]
    fn a_repository_git_will_not_read_says_nothing_about_merging() {
        let dir = tempfile::tempdir().unwrap();

        assert_eq!(merged(dir.path(), "abc123", "main"), None);
    }

    /// A repository with one commit on it and a branch to check out, and the
    /// directory that keeps it alive.
    fn repository() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path().join("repo");

        std::fs::create_dir_all(&repo).unwrap();

        run(&repo, &["init", "--initial-branch", "main"]);
        run(&repo, &["config", "user.email", "test@verkstead.invalid"]);
        run(&repo, &["config", "user.name", "Verkstead Test"]);
        std::fs::write(repo.join("README.md"), "# a repository\n").unwrap();
        run(&repo, &["add", "-A"]);
        run(&repo, &["commit", "-m", "chore: something to branch from"]);

        (dir, repo)
    }

    fn run(dir: &Path, args: &[&str]) {
        let output = std::process::Command::new("git")
            .args(args)
            .current_dir(dir)
            .stdin(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .output()
            .expect("git should be on the PATH for these tests");

        assert!(output.status.success(), "git {args:?} failed");
    }
}
