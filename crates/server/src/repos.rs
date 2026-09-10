//! Registering a Repo: everything between a path the human typed and a row in
//! the store — making one where there was no repository to name, and taking one
//! off the registry again, which is the store's own and passes straight
//! through.
//!
//! Three things have to be true, and each of them is checked here rather than in
//! the browser: the path is absolute and something is there, it is the root of a
//! git repository, and that repository can say which branch it works from. A
//! form that checked any of them would be a courtesy — this endpoint is
//! reachable without one.
//!
//! Anywhere the server can read is somewhere a Repo can be registered from.
//! There was a fourth thing once — that the path was inside a Watched Path — and
//! it is asked nowhere now: what keeps a session to its own Conversation is the
//! Sandbox, composed from the Repo and the Profile that Conversation names.
//!
//! Git is shelled out to rather than linked, as it is in the CLI: the answers
//! are one-liners, and what git says about a repository on this machine is what
//! the sandboxed sessions will see later when they run it themselves. Making one
//! is git as well: a directory, `git init` onto `main`, and a `README.md`
//! committed as the configured author, after which it is registered through the
//! very call a typed path goes through — see [`create`]. And, where it was asked
//! for, `gh` puts the same repository on GitHub and pushes to it, which is the
//! one thing here that can fail without the create failing.
//!
//! And the one thing a registered Repo is *told* rather than read: how a merge
//! conflict on its pull requests is resolved, which is an override of the
//! setting every Repo shares and passes through to the store like the removal.

use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result};
use sqlx::SqlitePool;
use verkstead_render::{ConflictResolution, Created, Registered, RepoEntry, RepoRemoved, RepoView};

use crate::github::Gh;
use crate::resolved::{Resolved, resolve};
use crate::settings::GitAuthor;
use crate::store;
use crate::unseen::Unseen;

/// Register the repository at `asked`, or say why not.
///
/// The filesystem half runs off the runtime: resolving a path and asking git
/// about a repository are both blocking, and a registration is rare enough that
/// the thread it borrows costs nothing.
pub(crate) async fn register(pool: &SqlitePool, asked: &str) -> Result<Registered> {
    registering(pool, PathBuf::from(asked)).await
}

/// The same, over a path the server is holding rather than one it was sent.
///
/// What a create registers what it has just made through: the directory it
/// stands in is one it built, so there is nothing to parse and nothing to be
/// lost putting a path through a string on the way.
async fn registering(pool: &SqlitePool, asked: PathBuf) -> Result<Registered> {
    let facts = match tokio::task::spawn_blocking(move || inspect(&asked)).await? {
        Ok(facts) => facts,
        Err(refusal) => return Ok(refusal),
    };

    // The Repo goes back either way, because both of these leave one registered
    // and whoever asked is usually about to work in it — see [`Registered`]. The
    // second read is the price of the insert deciding for itself whether the
    // path was taken: it is asked after the write rather than before it, so
    // nothing here looks first.
    Ok(
        match store::register_repo(pool, &facts.path, &facts.name, &facts.default_branch).await? {
            Some(repo) => Registered::Added(entry(repo)),
            None => {
                let held = store::registered_repo_at(pool, &facts.path)
                    .await?
                    // The insert refused because a row that nobody has flagged
                    // holds this path, and unregistering flags rather than
                    // deletes — so the row is there. Nothing left to answer with
                    // if it is not, which is a broken store rather than an
                    // outcome to put in front of anybody.
                    .with_context(|| {
                        format!(
                            "the Repo at {} is registered but could not be read back",
                            facts.path.display()
                        )
                    })?;

                Registered::AlreadyRegistered(entry(held))
            }
        },
    )
}

/// A stored Repo as the viewer's list draws one.
///
/// Here rather than in the router, because the registration hands one back now
/// and the list is not the only place a Repo crosses the wire — two spellings of
/// the same row would be two things to keep in step.
pub(crate) fn entry(repo: store::Repo) -> RepoEntry {
    RepoEntry {
        id: repo.id,
        name: repo.name,
        // Stored as UTF-8 in the first place — a path that is not cannot be
        // registered — so nothing is lost putting it back on the wire.
        path: repo.path.to_string_lossy().into_owned(),
        default_branch: repo.default_branch,
    }
}

/// Make a repository called `name` under `parent`, register it, and hand back
/// the Repo the registry's own pane is drawn from — or say why there is none.
///
/// A fresh repository is four things rather than one: the directory, `git init`
/// onto `main`, a `README.md` holding the name, and the commit that puts it
/// there. **The commit is why there is a README at all** — an empty repository
/// has a default branch and no commit, and a Conversation cannot take a base
/// from one, so a repository with nothing on `main` would be registered and
/// unusable. A README is what a fresh repository conventionally holds.
///
/// **It is committed as the configured author or not at all.** Said on the
/// command line rather than written into the new repository's config, the way
/// [`crate::publishing`] says it: it is the same fact either way and one of them
/// leaves a file behind. Where nobody is configured this refuses, which is where
/// publishing falls back instead — a share published as Verkstead is a share,
/// and a repository whose first commit is by nobody is that repository's history
/// from now on.
///
/// **And where `github` is asked for, a fifth thing: the same repository on
/// GitHub.** `gh repo create`, private, sourced from the directory it has just
/// made, with `origin` written and `main` pushed — see
/// [`crate::github::create_repository`]. Run after the first commit, there being
/// nothing to push before it, and before the registration, so that what is read
/// back is a Repo with its remote on it.
///
/// A GitHub failure there is **not** a failed create. The directory, the commit
/// and the registration all stand, and what is missing is a remote that can be
/// added afterwards — so the answer carries the Repo *and* what failed rather
/// than choosing between them: [`Created::MadeWithoutRemote`].
///
/// The filesystem half runs off the runtime the way the registration's does:
/// making a directory and shelling out to git are both blocking, and a create is
/// rare enough that the thread it borrows costs nothing.
pub(crate) async fn create(
    pool: &SqlitePool,
    author: &GitAuthor,
    gh: &Gh,
    parent: &str,
    name: &str,
    github: bool,
) -> Result<Created> {
    // Both halves or neither. Git wants an identity rather than half of one, and
    // what it would say about the half that was missing is a sentence about git
    // where the answer is about the settings page that has the field.
    let (Some(who), Some(email)) = (author.name(), author.email()) else {
        return Ok(Created::NoAuthor);
    };

    let author = Author {
        name: who.to_owned(),
        email: email.to_owned(),
    };

    let parent = PathBuf::from(parent);
    // Trimmed, because a directory named with a space on the end is nobody's
    // intention and every later reading of the name would carry it.
    let name = name.trim().to_owned();

    let called = name.clone();
    let made = tokio::task::spawn_blocking(move || make(&parent, &called, &author)).await?;

    let path = match made {
        Ok(path) => path,
        Err(refusal) => return Ok(refusal.into()),
    };

    // GitHub, where it was asked for — after the commit, because there is
    // nothing to push before it, and before the registration, so that the Repo
    // read back at the end is one with its remote already on it.
    //
    // What comes back is the reason it did not happen rather than a failure to
    // return: a repository that is on somebody's disk is made, whatever GitHub
    // said about it, and taking the local one back over a token that has
    // expired would be the worse of the two mistakes.
    let unpushed = match github {
        false => None,
        true => {
            let gh = gh.clone();
            let dir = path.clone();
            let called = name.clone();

            tokio::task::spawn_blocking(move || {
                crate::github::create_repository(&gh, &dir, &called)
            })
            .await?
            .err()
            .map(|trouble| trouble.why())
        }
    };

    // Registered through the very call the form's registration goes through:
    // what was just made is a repository like any other, and a second way of
    // taking one on would be a second chance for the two to come apart.
    let registered = registering(pool, path.clone()).await?;

    let (Registered::Added(repo) | Registered::AlreadyRegistered(repo)) = registered else {
        // A repository this call made a moment ago, with a commit on `main`,
        // that the registration will not have: a broken reading rather than
        // anything the human did. The directory stands — taking away a
        // repository that is now theirs would be the worse of the two mistakes —
        // and nothing is on the registry, which is what the answer says.
        tracing::error!(
            path = %path.display(),
            refusal = ?registered,
            "a repository that was made could not be registered",
        );

        return Ok(Created::Refused(
            "the repository was made but could not be registered".to_owned(),
        ));
    };

    // The whole opened Repo rather than the row it was registered as — see
    // [`Created::Made`]. `None` is a Repo taken off the registry between the two
    // reads, which is nothing that happens to somebody making one.
    Ok(match opened(pool, repo.id).await? {
        Some(repo) => match unpushed {
            Some(why) => Created::MadeWithoutRemote { repo, why },
            None => Created::Made(repo),
        },
        None => {
            Created::Refused("the repository was registered but could not be read back".to_owned())
        }
    })
}

/// Who the first commit is by, which is both halves or the create does not
/// happen — see [`create`].
struct Author {
    name: String,
    email: String,
}

/// What the filesystem half can refuse with, which is every outcome of a create
/// but the two it never decides: the Repo that was made, and an author nobody
/// has configured.
///
/// Its own enum rather than the wire's, so that a call which can answer four of
/// the six is not written as one that could answer any of them — and so that the
/// whole opened Repo is not carried through a return that never holds one.
enum Unmade {
    ParentMissing,
    AlreadyThere,
    BadName,
    Refused(String),
}

impl From<Unmade> for Created {
    fn from(unmade: Unmade) -> Created {
        match unmade {
            Unmade::ParentMissing => Created::ParentMissing,
            Unmade::AlreadyThere => Created::AlreadyThere,
            Unmade::BadName => Created::BadName,
            Unmade::Refused(why) => Created::Refused(why),
        }
    }
}

/// The filesystem half: the directory, the repository in it, and the first
/// commit on `main` — or the refusal to put in front of the human.
///
/// Blocking from end to end. What comes back is the resolved path the directory
/// really has, which is what the registration is then asked about.
fn make(parent: &Path, name: &str, author: &Author) -> Result<PathBuf, Unmade> {
    let parent = match resolve(parent) {
        Resolved::At(parent) => parent,
        // One sentence where a registration tells the two apart: either way
        // there is no directory to make this one in, and a create is answered by
        // browsing to a parent rather than by rewriting a path.
        Resolved::NotAbsolute | Resolved::Missing => return Err(Unmade::ParentMissing),
    };

    if !parent.is_dir() {
        return Err(Unmade::ParentMissing);
    }

    if !is_a_name(name) {
        return Err(Unmade::BadName);
    }

    let path = parent.join(name);

    // Making the directory is how it is asked whether one is there, rather than
    // a look followed by a write: nothing is registered under a path a create
    // has looked at, so a look would be an answer that could stop being true
    // before it was used.
    if let Err(error) = std::fs::create_dir(&path) {
        return Err(match error.kind() {
            ErrorKind::AlreadyExists => Unmade::AlreadyThere,
            // The parent went between being resolved and being written in.
            ErrorKind::NotFound => Unmade::ParentMissing,
            _ => Unmade::Refused(format!("the directory could not be made: {error}")),
        });
    }

    match filled(&path, name, author) {
        Ok(()) => Ok(path),
        Err(why) => {
            // The directory outlives the failure otherwise, holding half a
            // repository and standing where the human asked for a whole one —
            // the reason [`crate::publishing`] takes back the gist it could not
            // fill. What is in it was put there in the last moment by the calls
            // above, and there was nothing at this path at all before them.
            if let Err(error) = std::fs::remove_dir_all(&path) {
                tracing::error!(
                    error = ?error,
                    path = %path.display(),
                    "taking back the directory of a repository that could not be made failed",
                );
            }

            Err(Unmade::Refused(why))
        }
    }
}

/// A repository inside `path`, with `name`'s README on its first commit.
fn filled(path: &Path, name: &str, author: &Author) -> Result<(), String> {
    // `--initial-branch` rather than whatever this machine's `init.defaultBranch`
    // happens to say: what is registered is a repository working from `main`,
    // wherever it was made. It wants git 2.28, which is the floor the packaged
    // builds already assume.
    run(path, &["init", "--quiet", "--initial-branch", MAIN])?;

    // The name rather than a sentence about Verkstead: the human is about to
    // open this repository, and the first thing in it should be theirs.
    std::fs::write(path.join(README), format!("# {name}\n"))
        .map_err(|error| format!("the README could not be written: {error}"))?;

    run(path, &["add", "--", README])?;
    run(
        path,
        &[
            "-c",
            &format!("user.name={}", author.name),
            "-c",
            &format!("user.email={}", author.email),
            "commit",
            "--quiet",
            "--message",
            FIRST_COMMIT,
        ],
    )?;

    Ok(())
}

/// The branch a created repository works from, which is the one every packaged
/// build's git will take a `-b` for.
const MAIN: &str = "main";

/// What a fresh repository holds, and the reason it has a commit to be
/// registered on.
const README: &str = "README.md";

/// And what that commit is called. Git's own words for the thing it is, because
/// nothing about it is Verkstead's: what the human sees in their log is the
/// message any first commit carries.
const FIRST_COMMIT: &str = "Initial commit";

/// Whether `name` is a name a directory can have, rather than a path or nothing
/// at all.
///
/// A create joins this onto a parent, so anything that would climb out of that
/// parent or point somewhere else is refused before a directory is made: what
/// goes in a name field is a name, and a separator in it means the human meant
/// something the field cannot say. Both separators everywhere rather than the
/// platform's own — a backslash is legal in a Unix filename and is nobody's
/// intention in one.
fn is_a_name(name: &str) -> bool {
    !name.is_empty() && name != "." && name != ".." && !name.contains(['/', '\\', '\0'])
}

/// One git command in `dir`, or what it said about why not.
///
/// Its own runner rather than [`git`] above, which is for the reads: what a read
/// wants is the output or nothing at all, and what a create wants is the reason,
/// which git writes on stderr. Read as one line the way a publish reads it — see
/// [`crate::publishing::said`].
fn run(dir: &Path, args: &[&str]) -> Result<(), String> {
    let output = Command::new("git")
        .args(args)
        .unseen()
        .current_dir(dir)
        .stdin(Stdio::null())
        .output()
        .map_err(|error| format!("git could not be run: {error}"))?;

    if output.status.success() {
        return Ok(());
    }

    Err(crate::publishing::said(&String::from_utf8_lossy(
        &output.stderr,
    )))
}

/// What the store needs to record a Repo, read off the repository itself rather
/// than claimed by whoever registered it.
struct Facts {
    path: PathBuf,
    name: String,
    default_branch: String,
}

/// Everything the filesystem and git have to say about a path someone wants
/// registered — or the reason it is not going to be.
fn inspect(asked: &Path) -> Result<Facts, Registered> {
    let path = match resolve(asked) {
        Resolved::At(path) => path,
        Resolved::NotAbsolute => return Err(Registered::NotAbsolute),
        Resolved::Missing => return Err(Registered::Missing),
    };

    if !is_repository_root(&path) {
        return Err(Registered::NotARepository);
    }

    let Some(default_branch) = default_branch(&path) else {
        return Err(Registered::NoDefaultBranch);
    };

    Ok(Facts {
        name: name(&path),
        path,
        default_branch,
    })
}

/// Whether `path` is the top of a worktree rather than somewhere inside one.
///
/// The root and not merely "in a repository": a Conversation's worktree is added
/// from the repository, and registering `…/repo/src` would leave every path
/// Verkstead later builds hanging off a directory that is not the repository at
/// all. Both sides are resolved before they are compared, because git answers
/// with the path the filesystem means and the caller may not have.
fn is_repository_root(path: &Path) -> bool {
    git(path, &["rev-parse", "--show-toplevel"])
        .map(|top| PathBuf::from(top.trim()))
        .and_then(|top| top.canonicalize().ok())
        .is_some_and(|top| top == path)
}

/// Every branch of a registered Repo, for the dropdown a Conversation picks the
/// one it comes off out of.
///
/// `None` is a Repo that is not registered. The reading itself is git's — see
/// [`crate::worktrees::branches`] — and it runs off the runtime, because a
/// repository with a great many refs is not a quick call.
pub(crate) async fn branches(pool: &SqlitePool, id: i64) -> Result<Option<Vec<String>>> {
    let Some(repo) = store::load_repo(pool, id).await? else {
        return Ok(None);
    };

    Ok(Some(
        tokio::task::spawn_blocking(move || crate::worktrees::branches(&repo.path)).await?,
    ))
}

/// One registered Repo opened: everything its card cannot hold.
///
/// `None` is a Repo that is not on the registry, which the pane reads as the
/// repo being gone — a link followed after somebody took it away, or a pane left
/// open in another tab while they did. Read through
/// [`store::registered_repo`] for that reason rather than through `load_repo`,
/// which goes on finding a Repo that was taken away because a Timeline still has
/// to name it.
///
/// The two filesystem reads go together in one blocking task rather than one
/// apiece: they are both git against the same directory, and a pane is one thing
/// the human opened rather than two. The counts are the store's and are awaited
/// beside them.
///
/// Nothing here is stored but the three facts the card already carries. The
/// branches move without Verkstead hearing about it and a roadmap somebody picks
/// up stops being abandoned the moment they do, so both are asked afresh every
/// time the pane is opened — a kept copy would be a second opinion that went
/// wrong on somebody else's push.
pub(crate) async fn opened(pool: &SqlitePool, id: i64) -> Result<Option<RepoView>> {
    let Some(repo) = store::registered_repo(pool, id).await? else {
        return Ok(None);
    };

    let work = store::work_on_repo(pool, id).await?;

    let read = repo.clone();
    let (branches, roadmaps) = tokio::task::spawn_blocking(move || {
        (
            crate::worktrees::branches(&read.path),
            crate::stages::waiting(&read),
        )
    })
    .await?;

    Ok(Some(RepoView {
        id: repo.id,
        name: repo.name,
        // Stored as UTF-8 in the first place — a path that is not cannot be
        // registered — so nothing is lost putting it back on the wire.
        path: repo.path.to_string_lossy().into_owned(),
        default_branch: repo.default_branch,
        branches,
        live: work.live,
        finished: work.finished,
        roadmaps,
        conflict_resolution: store::repo_resolution(pool, id).await?.map(resolution),
    }))
}

/// Say how this Repo resolves a conflict from now on, and hand back the Repo as
/// it now stands.
///
/// `None` takes the override back rather than writing the global's word down —
/// see [`verkstead_render::ConflictResolutionEdit`]. Nothing is refused: there is no
/// work this could be taken out from under, unlike an unregistering, and the
/// next conflict is simply resolved the new way.
///
/// The Repo that comes back is the whole pane's worth, read afresh: the pane
/// draws what the server says rather than what it just sent, which is the same
/// rule the settings page saves under.
///
/// `None` is a Repo nothing is registered under, which is a pane somebody left
/// open in another tab.
pub(crate) async fn set_resolution(
    pool: &SqlitePool,
    id: i64,
    resolution: Option<ConflictResolution>,
) -> Result<Option<RepoView>> {
    if store::registered_repo(pool, id).await?.is_none() {
        return Ok(None);
    }

    store::set_repo_resolution(pool, id, resolution.map(stored)).await?;

    opened(pool, id).await
}

/// The store's word for a resolution as the viewer receives it.
///
/// Here rather than beside each caller: the settings page reads the global out
/// of `config.yaml` and this pane reads one Repo's override out of the store,
/// and the two are the same two words either way. A second mapping would be a
/// second chance for the viewer's word and the store's to come apart.
pub(crate) fn resolution(resolution: store::ConflictResolution) -> ConflictResolution {
    match resolution {
        store::ConflictResolution::Merge => ConflictResolution::Merge,
        store::ConflictResolution::Rebase => ConflictResolution::Rebase,
    }
}

/// And back, which is what a press on either page sends.
pub(crate) fn stored(resolution: ConflictResolution) -> store::ConflictResolution {
    match resolution {
        ConflictResolution::Merge => store::ConflictResolution::Merge,
        ConflictResolution::Rebase => store::ConflictResolution::Rebase,
    }
}

/// Take a Repo off the registry, if nothing live is being worked in it.
///
/// An unregistering rather than a delete, and the whole of that is the store's —
/// see [`store::unregister_repo`]. Nothing here touches the repository itself:
/// what Verkstead is being told is that it may stop offering it, and the
/// directory is the human's either way.
pub(crate) async fn remove(pool: &SqlitePool, id: i64) -> Result<RepoRemoved> {
    Ok(match store::unregister_repo(pool, id).await? {
        store::Unregistering::Unregistered => RepoRemoved::Removed,
        store::Unregistering::NoSuchRepo => RepoRemoved::NoSuchRepo,
        store::Unregistering::InUse => RepoRemoved::InUse,
    })
}

/// The branch a Conversation branches from unless it is told otherwise.
///
/// What the remote calls its default, where there is a remote to ask — that is
/// what "default branch" means to everyone who works on the repository, and it
/// does not move when somebody leaves another branch checked out. Failing that,
/// whatever is checked out here.
///
/// `None` on a detached HEAD with no remote to fall back on: there is no branch
/// to name, and naming one anyway would put work on a branch nobody chose.
fn default_branch(path: &Path) -> Option<String> {
    let remote = git(
        path,
        &["symbolic-ref", "--short", "refs/remotes/origin/HEAD"],
    )
    .and_then(|head| Some(head.trim().strip_prefix("origin/")?.to_owned()));

    remote
        .or_else(|| {
            git(path, &["symbolic-ref", "--short", "HEAD"]).map(|head| head.trim().to_owned())
        })
        .filter(|branch| !branch.is_empty())
}

/// What to call the repository in a list: the directory's own name, which is
/// what the human calls it.
///
/// A directory with no name of its own is the filesystem root, which nobody is
/// registering; its whole path stands in rather than leaving a row blank.
fn name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

/// Run git in `dir` and take its stdout, or `None` if it failed.
///
/// Shared with [`crate::conversations`], which asks git the two questions a
/// Conversation raises — whether a name is one it would take for a branch, and
/// what commit something resolves to.
pub(crate) fn git(dir: &Path, args: &[&str]) -> Option<String> {
    accepting(dir, args, &[0])
}

/// The same run, accepting any of the `ok` exit codes rather than success alone.
///
/// There is one read here that is not a failure when it exits non-zero:
/// `git diff --no-index` exits 1 when the two files differ, which for the
/// untracked file [`crate::diffs`] asks it about is the ordinary case.
pub(crate) fn accepting(dir: &Path, args: &[&str], ok: &[i32]) -> Option<String> {
    let output = Command::new("git")
        // Reading a repository should never take a lock on it: an agent may well
        // be working in this one right now.
        .arg("--no-optional-locks")
        .args(args)
        .unseen()
        .current_dir(dir)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;

    if !ok.contains(&output.status.code()?) {
        return None;
    }

    // Paths and patches are whatever bytes the filesystem holds; a Set is UTF-8
    // either way, so anything else is replaced rather than refused.
    Some(String::from_utf8_lossy(&output.stdout).into_owned())
}
