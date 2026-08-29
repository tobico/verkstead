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

use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use crate::repos::git;

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

    // A group of its own, so that the kill below reaches the transport helper
    // git started as well as git itself.
    command.process_group(0);

    let child = match command.spawn() {
        Ok(child) => child,
        Err(error) => return Fetched::Failed(error.to_string()),
    };

    // Kept before the child is handed over, because the group is named by it and
    // the thread below is where the child goes.
    let group = child.id();

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
            stop(group);

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

/// Kill a fetch that has run past its deadline, and everything it started.
///
/// The group rather than the process: `git fetch` does its talking through a
/// transport helper it spawns, and that helper is where a stalled network
/// leaves things waiting. Signalling git alone would end the wait here and
/// leave the helper behind with nobody to reap it.
fn stop(group: u32) {
    let Ok(group) = i32::try_from(group) else {
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
        connection.read_to_end(&mut read).unwrap();
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
