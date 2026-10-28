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
//! committed as the Author picked, after which it is registered through the
//! very call a typed path goes through — see [`create`]. And, where it was asked
//! for, `gh` puts the same repository on GitHub and pushes to it, which is the
//! one thing here that can fail without the create failing.
//!
//! And the third way one arrives is a clone of a **Clonable Repo**: git, run as
//! this device with its own token, into a directory named off the origin, and
//! then that same registration — see [`clone`].

use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result};
use sqlx::SqlitePool;
use verkstead_render::{
    AuthorRemoved, Cloned, Created, Creation, Registered, RepoEntry, RepoRemoved, RepoView,
    Repointed,
};

use crate::github::Gh;
use crate::resolved::{Resolved, resolve};
use crate::settings::{Author, Config, Signature};
use crate::store;
use crate::unseen::Unseen;

/// Register the repository at `asked` as the Author with the uuid `author`, or
/// say why not.
///
/// The Author is settled first — see [`picked`] — because a refusal there is
/// about the form rather than the repository, and asks nothing of the disk.
///
/// The filesystem half runs off the runtime: resolving a path and asking git
/// about a repository are both blocking, and a registration is rare enough that
/// the thread it borrows costs nothing.
pub(crate) async fn register(
    pool: &SqlitePool,
    config: &Config,
    asked: &str,
    author: Option<&str>,
) -> Result<Registered> {
    let author = match picked(config, author) {
        Ok(author) => author,
        Err(refusal) => return Ok(refusal.into()),
    };

    registering(
        pool,
        config.authors(),
        author.map(Author::id),
        PathBuf::from(asked),
    )
    .await
}

/// Which Author a Repo registered now is recorded as, out of the uuid the
/// registration named.
///
/// **A uuid named is that Author or a refusal**, whatever else is configured:
/// a form that named one the one list no longer holds was drawn over a list
/// that has since changed. An Author made on another device is one like any
/// other (ADR-0028). **None named is the one Author** where there is exactly
/// one — there is nobody else it could be (ADR-0025) — nobody where there is
/// none, and a refusal where there are several, there being no saying which.
fn picked<'a>(config: &'a Config, asked: Option<&str>) -> Result<Option<&'a Author>, Unpicked> {
    match (asked, config.authors()) {
        (Some(id), _) => config.author(id).map(Some).ok_or(Unpicked::NoSuchAuthor),
        (None, []) => Ok(None),
        (None, [only]) => Ok(Some(only)),
        (None, _) => Err(Unpicked::AuthorUnpicked),
    }
}

/// Why no Author could be settled on, which a registration and a create both
/// answer with a word of their own.
enum Unpicked {
    AuthorUnpicked,
    NoSuchAuthor,
}

impl From<Unpicked> for Registered {
    fn from(unpicked: Unpicked) -> Registered {
        match unpicked {
            Unpicked::AuthorUnpicked => Registered::AuthorUnpicked,
            Unpicked::NoSuchAuthor => Registered::NoSuchAuthor,
        }
    }
}

impl From<Unpicked> for Cloned {
    fn from(unpicked: Unpicked) -> Cloned {
        match unpicked {
            Unpicked::AuthorUnpicked => Cloned::AuthorUnpicked,
            Unpicked::NoSuchAuthor => Cloned::NoSuchAuthor,
        }
    }
}

impl From<Unpicked> for Created {
    fn from(unpicked: Unpicked) -> Created {
        match unpicked {
            Unpicked::AuthorUnpicked => Created::AuthorUnpicked,
            Unpicked::NoSuchAuthor => Created::NoSuchAuthor,
        }
    }
}

/// The same, over a path the server is holding rather than one it was sent.
///
/// What a create registers what it has just made through: the directory it
/// stands in is one it built, so there is nothing to parse and nothing to be
/// lost putting a path through a string on the way.
///
/// **The Repo is recorded as `author`**, already settled by [`picked`]. A Repo
/// revived keeps the Author it had — see [`store::register_repo_as`].
async fn registering(
    pool: &SqlitePool,
    authors: &[Author],
    author: Option<&str>,
    asked: PathBuf,
) -> Result<Registered> {
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
        match store::register_repo_as(
            pool,
            &facts.path,
            &facts.name,
            &facts.default_branch,
            author,
        )
        .await?
        {
            Some(repo) => Registered::Added(entry(repo, authors)),
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

                Registered::AlreadyRegistered(entry(held, authors))
            }
        },
    )
}

/// A stored Repo as the viewer's list draws one.
///
/// Here rather than in the router, because the registration hands one back now
/// and the list is not the only place a Repo crosses the wire — two spellings of
/// the same row would be two things to keep in step.
///
/// `authors` is every Author configured, read by the caller at the moment of
/// the request: it is what says whether a Repo with none recorded has one
/// still to pick.
pub(crate) fn entry(repo: store::Repo, authors: &[Author]) -> RepoEntry {
    RepoEntry {
        id: repo.id,
        name: repo.name,
        // Stored as UTF-8 in the first place — a path that is not cannot be
        // registered — so nothing is lost putting it back on the wire.
        path: repo.path.to_string_lossy().into_owned(),
        default_branch: repo.default_branch,
        author_unpicked: unpicked(repo.author.as_deref(), authors),
        author: repo.author,
    }
}

/// Whether a Repo recorded as `author` has an Author still to pick: none is
/// recorded, and there is more than one it could be.
fn unpicked(author: Option<&str>, authors: &[Author]) -> bool {
    author.is_none() && authors.len() > 1
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
/// **It is committed as the Author picked, or not at all** — the one Author
/// where only one is configured, and [`picked`] otherwise. Said on the
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
/// **Only as an Author with a token.** Where the one it is made as has none, no
/// `gh` is run at all — it would fall back to the host's own login, a repository
/// in an account nobody picked — and the answer is the one below, saying so.
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
    config: &Config,
    gh: &Gh,
    creation: &Creation,
    at_once: usize,
) -> Result<Created> {
    let authors = config.authors();

    let Creation {
        parent,
        name,
        github,
        author,
    } = creation;

    let picked = match picked(config, author.as_deref()) {
        Ok(picked) => picked,
        Err(refusal) => return Ok(refusal.into()),
    };

    // Both halves or neither, which is the one shape git will take an identity
    // in — see [`Signature`], the rule everything Verkstead commits on its own
    // account is made under.
    let Some(author) = picked.and_then(Author::configured) else {
        return Ok(Created::NoAuthor);
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
            // As the Author it was committed as, whose Repo it is about to be.
            let gh = gh.as_author(picked.map(Author::id));

            let dir = path.clone();
            let called = name.clone();

            // And not at all where that Author has no token: `gh` would fall
            // back to the host's own login and make the repository in an
            // account nobody picked. Made here only, and said why — the shape a
            // GitHub that would not finish already has. An Author whose token's
            // copy has not arrived says that instead.
            let awaited = picked.and_then(|author| config.token_awaited(author.id()));

            if let Some(device) = awaited {
                Some(crate::github::awaited(device))
            } else if gh.tokened() {
                tokio::task::spawn_blocking(move || {
                    crate::github::create_repository(&gh, &dir, &called)
                })
                .await?
                .err()
                .map(|trouble| trouble.why())
            } else {
                Some(untokened(authors, picked))
            }
        }
    };

    // Registered through the very call the form's registration goes through:
    // what was just made is a repository like any other, and a second way of
    // taking one on would be a second chance for the two to come apart.
    let registered = registering(pool, authors, picked.map(Author::id), path.clone()).await?;

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
            "The repository was made but couldn't be registered.".to_owned(),
        ));
    };

    // The whole opened Repo rather than the row it was registered as — see
    // [`Created::Made`]. `None` is a Repo taken off the registry between the two
    // reads, which is nothing that happens to somebody making one.
    Ok(match opened(pool, authors, repo.id, at_once).await? {
        Some(repo) => match unpushed {
            Some(why) => Created::MadeWithoutRemote { repo, why },
            None => Created::Made(repo),
        },
        None => Created::Refused("The repo was registered but couldn't be read back.".to_owned()),
    })
}

/// Why a create asked onto GitHub was made here only, where the Author it was
/// made as has no token: named where there are several to tell apart, and not
/// where the one there is is the only one it could be.
fn untokened(authors: &[Author], picked: Option<&Author>) -> String {
    let named = picked
        .filter(|_| authors.len() > 1)
        .and_then(|author| author.display_name().or(author.name()));

    match named {
        Some(named) => format!("{named} has no GitHub token saved, so it wasn't made on GitHub."),
        None => "No GitHub token is saved, so it wasn't made on GitHub.".to_owned(),
    }
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
fn make(parent: &Path, name: &str, author: &Signature) -> Result<PathBuf, Unmade> {
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
            _ => Unmade::Refused(format!("The directory couldn't be made: {error}.")),
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
fn filled(path: &Path, name: &str, author: &Signature) -> Result<(), String> {
    // `--initial-branch` rather than whatever this machine's `init.defaultBranch`
    // happens to say: what is registered is a repository working from `main`,
    // wherever it was made. It wants git 2.28, which is the floor the packaged
    // builds already assume.
    run(path, &["init", "--quiet", "--initial-branch", MAIN])?;

    // The name rather than a sentence about Verkstead: the human is about to
    // open this repository, and the first thing in it should be theirs.
    std::fs::write(path.join(README), format!("# {name}\n"))
        .map_err(|error| format!("The README.md couldn't be written: {error}."))?;

    run(path, &["add", "--", README])?;
    run(
        path,
        &[
            "-c",
            &format!("user.name={}", author.name()),
            "-c",
            &format!("user.email={}", author.email()),
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

/// Clone `origin` under `parent`, register it, and hand back the Repo — or say
/// why there is none (ADR-0024, *Clonable repos*).
///
/// **The directory is named off the origin**, the way the clonable row was
/// named — see [`crate::matching::clone_name`] — so what the human picked and
/// what lands are called the same thing.
///
/// **Nothing registers twice.** Where a Repo is registered at that path
/// already, it is the answer and nothing is cloned, the way **Open repo**
/// answers a registered path. A directory that is there and *not* registered is
/// git's to refuse, and it does, in its own words: the clone never adopts what
/// somebody else put there.
///
/// **It is the Author the member's Repo works as** — `cloning.author`, which
/// the clonable row carried — settled by [`picked`] the way a registration's
/// is: a uuid this device's one list holds, or the one Author where none was
/// said. The Repo is recorded as it, and git fetches as its token, read at the
/// moment of the call — see [`cloning`]. An Author whose token's copy has not
/// arrived refuses by name rather than leave the fetch to the host's login.
///
/// **And a clone that failed leaves nothing behind**: no registration, and no
/// directory where there was none before, git's own words being the answer.
/// One that cloned and would not register — an empty repository has no branch
/// to call its default — is taken back too, there being no Repo for it to be.
///
/// There is no deadline on any of it. A big repository takes as long as it
/// takes, and the relayed call that reaches here has none either — see
/// [`crate::peer::dialling`] — so what bounds the wait is the browser's own
/// patience, as it is for a local press.
pub(crate) async fn clone(
    pool: &SqlitePool,
    config: &Config,
    gh: &Gh,
    cloning: &verkstead_render::Cloning,
) -> Result<Cloned> {
    let authors = config.authors();

    let picked = match picked(config, cloning.author.as_deref()) {
        Ok(picked) => picked,
        Err(refusal) => return Ok(refusal.into()),
    };

    if let Some(device) = picked.and_then(|author| config.token_awaited(author.id())) {
        return Ok(Cloned::Refused(crate::github::awaited(device)));
    }

    let credential = gh.as_author(picked.map(Author::id)).credential();

    let parent = PathBuf::from(&cloning.parent);
    let origin = cloning.origin.trim().to_owned();
    let name = crate::matching::clone_name(&origin);

    let parent = match resolve(&parent) {
        Resolved::At(parent) if parent.is_dir() => parent,
        Resolved::At(_) | Resolved::NotAbsolute | Resolved::Missing => {
            return Ok(Cloned::ParentMissing);
        }
    };

    if !is_a_name(&name) {
        return Ok(Cloned::Refused(
            "That origin doesn't name a directory to clone into.".to_owned(),
        ));
    }

    let path = parent.join(&name);

    // Asked of the resolved path, which is how the store records one: a parent
    // reached through a symlink still finds the Repo under it.
    if let Resolved::At(there) = resolve(&path)
        && let Some(held) = store::registered_repo_at(pool, &there).await?
    {
        return Ok(Cloned::AlreadyRegistered(entry(held, authors)));
    }

    let into = path.clone();
    let cloned = tokio::task::spawn_blocking(move || {
        // Looked at before git runs rather than after, because what is taken
        // back on a failure is only ever a directory this call made.
        let there_before = into.symlink_metadata().is_ok();
        let answered = cloned_by(&origin, &parent, &name, &credential).output();

        let why = match answered {
            Ok(output) if output.status.success() => return Ok(()),
            Ok(output) => crate::publishing::said(&String::from_utf8_lossy(&output.stderr)),
            Err(error) => format!("Git couldn't be run: {error}."),
        };

        if !there_before {
            taken_back(&into);
        }

        Err(why)
    })
    .await?;

    if let Err(why) = cloned {
        return Ok(Cloned::Refused(why));
    }

    Ok(
        match registering(pool, authors, picked.map(Author::id), path.clone()).await? {
            Registered::Added(repo) => Cloned::Cloned(repo),
            Registered::AlreadyRegistered(repo) => Cloned::AlreadyRegistered(repo),
            refusal => {
                tracing::warn!(
                    path = %path.display(),
                    refusal = ?refusal,
                    "a repository that was cloned could not be registered",
                );

                let gone = path.clone();
                tokio::task::spawn_blocking(move || taken_back(&gone)).await?;

                Cloned::Refused(match refusal {
                    Registered::NoDefaultBranch => {
                        "That repository has no branch to work from yet.".to_owned()
                    }
                    _ => "The repository was cloned but couldn't be registered.".to_owned(),
                })
            }
        },
    )
}

/// The `git clone` a clone onto this device runs: `origin` into `name` under
/// `parent`, as `credential`.
///
/// **Run in the parent with the bare name as where it goes**, the way a create
/// runs git inside the directory it made: a resolved path on Windows is spelled
/// `\\?\C:\…`, which the process can stand in and git cannot be handed.
///
/// **Authenticated the way the rest of Verkstead's own git is** — see
/// [`crate::github::Credential`]: the Author's token and no other, the host's
/// login where it holds none. And GitHub's SSH spellings rewritten to HTTPS, as
/// a session's are — see [`crate::sandbox::github_credentials`] — so an origin a
/// member cloned over SSH is fetched here with the token rather than with keys
/// this machine may not have. Only the rewrites of those: the helper there
/// answers out of `gh`'s own login where `GH_TOKEN` is missing, which is the
/// host's login a [`Credential`](crate::github::Credential) exists to keep out.
/// Said as `-c` on the command line rather than in the environment, which may
/// be carrying counted pairs of its own.
///
/// And nothing is asked of anybody — no terminal prompt, and a null stdin — so a
/// missing credential is a refusal rather than a clone waiting for ever.
fn cloned_by(
    origin: &str,
    parent: &Path,
    name: &str,
    credential: &crate::github::Credential,
) -> Command {
    let mut command = Command::new("git");

    for (key, value) in crate::sandbox::github_credentials() {
        if key.ends_with(".insteadOf") {
            command.arg("-c").arg(format!("{key}={value}"));
        }
    }

    credential.on(&mut command);

    command
        .args(["clone", "--quiet", "--", origin, name])
        .current_dir(parent)
        .unseen()
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());

    command
}

/// Take back a directory a clone made, logging rather than failing where it
/// will not go: the refusal the human reads is the clone's, and this is
/// tidying after it.
fn taken_back(path: &Path) {
    if path.symlink_metadata().is_err() {
        return;
    }

    if let Err(error) = std::fs::remove_dir_all(path) {
        tracing::error!(
            error = ?error,
            path = %path.display(),
            "taking back the directory of a clone that did not land failed",
        );
    }
}

/// One git command in `dir`, or what it said about why not.
///
/// Its own runner rather than [`git`] above, which is for the reads: what a read
/// wants is the output or nothing at all, and what a *write* wants is the
/// reason, which git writes on stderr. Read as one line the way a publish reads
/// it — see [`crate::publishing::said`].
///
/// Which is why every git call Verkstead makes on its own account comes through
/// here rather than through [`git`]: a create's first commit, and the clearing
/// of an inherited task list in [`crate::tasks::clear`]. Both of them take the
/// whole press back when they fail, and a refusal with no reason in the log is
/// one nobody can do anything about.
pub(crate) fn run(dir: &Path, args: &[&str]) -> Result<(), String> {
    let output = Command::new("git")
        .args(args)
        .unseen()
        .current_dir(dir)
        .stdin(Stdio::null())
        .output()
        .map_err(|error| format!("Git couldn't be run: {error}."))?;

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

/// One registered Repo, whole: the row, and everything a reading of the
/// repository itself adds to it.
///
/// What a create answers with — see [`create`], which is the one caller there
/// is. The settings had a pane per Repo drawing all of this and it is gone: what
/// is drawn of a Repo there is its name, and each of these is read where it is
/// used instead — the branches on a composer, the roadmaps in the new
/// conversation dropdown.
///
/// `None` is a Repo that is not on the registry. Read through
/// [`store::registered_repo`] for that reason rather than through `load_repo`,
/// which goes on finding a Repo that was taken away because a Timeline still has
/// to name it.
///
/// The two filesystem reads go together in one blocking task rather than one
/// apiece: they are both git against the same directory. The counts are the
/// store's and are awaited beside them.
///
/// Nothing here is stored but the three facts the row already carries. The
/// branches move without Verkstead hearing about it and a roadmap somebody picks
/// up stops being abandoned the moment they do, so both are asked afresh — a
/// kept copy would be a second opinion that went wrong on somebody else's push.
async fn opened(
    pool: &SqlitePool,
    authors: &[Author],
    id: i64,
    at_once: usize,
) -> Result<Option<RepoView>> {
    let Some(repo) = store::registered_repo(pool, id).await? else {
        return Ok(None);
    };

    let work = store::work_on_repo(pool, id).await?;

    // What Verkstead knows about this Repo's roadmap stages, which is half of what
    // says a stage is done — the boxes being the other half. Read here rather than
    // inside the reading below, which asks nothing of the database.
    let record = store::stage_standings(pool, id).await?;

    let read = repo.clone();
    let (branches, roadmaps) = tokio::task::spawn_blocking(move || {
        (
            crate::worktrees::branches(&read.path),
            crate::stages::waiting(&read, &record, at_once),
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
        author_unpicked: unpicked(repo.author.as_deref(), authors),
        author: repo.author,
        branches,
        live: work.live,
        finished: work.finished,
        roadmaps,
    }))
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

/// Point a Repo at the Author with the uuid `author`, from its row on the Repos
/// pane, if nothing live has it in it.
///
/// **The uuid is one in the one list or a refusal**, read off `config` at the
/// moment of the press, for [`picked`]'s reason. The rest is the store's — see
/// [`store::repoint_repo`], which refuses under live work on the Repo and
/// beside it. Nothing is handed to a session already running: the sandbox
/// reads the Author at launch, so the next one is the new Author's.
pub(crate) async fn repoint(
    pool: &SqlitePool,
    config: &Config,
    id: i64,
    author: &str,
) -> Result<Repointed> {
    if config.author(author).is_none() {
        return Ok(Repointed::NoSuchAuthor);
    }

    Ok(match store::repoint_repo(pool, id, author).await? {
        store::Repointing::Repointed(repo) => Repointed::Repointed(entry(repo, config.authors())),
        store::Repointing::NoSuchRepo => Repointed::NoSuchRepo,
        store::Repointing::InUse { on, beside } => Repointed::InUse { on, beside },
    })
}

/// Remove the Author with the uuid `id` — a deletion stamp on its record —
/// from the Git section's switcher, if no registered Repo points at it.
///
/// **Points at** as [`crate::settings::Config::author_of`] reads it, so a Repo
/// recording nothing is the only Author's and blocks removing it. An
/// unregistered Repo keeps its association as history and blocks nothing: it
/// is not on [`store::registered_repos`]. The look and the write are not one
/// transaction — the Repos are read before the record is stamped — so a Repo
/// registered as this Author in the instant between is the vanished Author a
/// hand edit makes, refused by name at its next start.
pub(crate) async fn remove_author(
    pool: &SqlitePool,
    settings: &crate::settings::Settings,
    device: &str,
    id: &str,
) -> Result<AuthorRemoved> {
    crate::replicating::authors::taken_up(pool, settings, device).await?;

    let config = settings.config();

    if config.author(id).is_none() {
        return Ok(AuthorRemoved::NoSuchAuthor);
    }

    let repos: Vec<String> = store::registered_repos(pool)
        .await?
        .into_iter()
        .filter(|repo| config.author_of(repo.author.as_deref()).map(Author::id) == Some(id))
        .map(|repo| repo.name)
        .collect();

    if !repos.is_empty() {
        return Ok(AuthorRemoved::InUse { repos });
    }

    Ok(
        match crate::replicating::authors::removed(pool, settings, device, id).await? {
            true => AuthorRemoved::Removed,
            false => AuthorRemoved::NoSuchAuthor,
        },
    )
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

/// The URL of the repository's `origin` remote, where it has one.
///
/// **Asked of git rather than kept**, which is the stance every other reading of
/// a repository takes here — the branches, the default branch, the roadmaps. A
/// registration records a path, a name and a default branch and no origin at
/// all, and a remote is added, changed and taken away without Verkstead hearing
/// about it; a stored copy would be a second opinion about the one fact two
/// devices have to agree on when they are settling whether they hold the same
/// repository. See [`crate::matching`], which is what asks.
///
/// `None` where there is no `origin` — `git remote get-url` exits non-zero on
/// a remote that is not there — and where what it answered was blank, which a
/// remote configured with an empty URL is. A repository with no origin is
/// matched by name or not at all, and an empty string standing in for one would
/// be two such repositories reading as the same one.
pub(crate) fn origin(path: &Path) -> Option<String> {
    git(path, &["remote", "get-url", "origin"])
        .map(|url| url.trim().to_owned())
        .filter(|url| !url.is_empty())
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
    // Paths and patches are whatever bytes the filesystem holds; a Set is UTF-8
    // either way, so anything else is replaced rather than refused.
    bytes(dir, args, ok).map(|stdout| String::from_utf8_lossy(&stdout).into_owned())
}

/// And the same run again with the bytes kept as bytes.
///
/// [`accepting`]'s own body, split out for the one reader that must not have
/// its answer replaced a character at a time: a **git bundle** is a pack file
/// on standard output, and every byte of it that did not read as UTF-8 would
/// come back as a replacement character and the bundle would not open — see
/// `crate::transfers::checkouts`.
///
/// Every other reader here is asking for text and takes [`accepting`], which is
/// this with the lossy read on the end.
pub(crate) fn bytes(dir: &Path, args: &[&str], ok: &[i32]) -> Option<Vec<u8>> {
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

    Some(output.stdout)
}

/// And the same run again with something written to it, which is the one git
/// read here that has an input as well as an answer.
///
/// [`crate::files`] asks `check-ignore` which of a folder's entries a checkout
/// ignores, and the paths go in on standard input rather than on the command
/// line: a folder wide enough for the question to be worth asking in one run is
/// a folder wide enough to overrun an argument list.
///
/// **Written and read at the same time**, which is the whole of why this is not
/// [`accepting`] with one more argument. `check-ignore` answers as it reads —
/// every path it has decided is ignored goes back down its own pipe before it
/// has finished with the ones still coming — so a caller that wrote the input
/// whole before reading a byte of the answer would deadlock on exactly the
/// folder this exists for: once the answer fills a pipe, git blocks writing
/// while this side blocks writing, and neither moves again. Around eight
/// hundred ignored entries in one folder is enough. So the input goes down a
/// thread of its own while [`std::process::Child::wait_with_output`] drains the
/// answer, and the pipe is closed by that thread ending, which is the end
/// `--stdin` is waiting for.
///
/// Scoped rather than spawned loose, so the thread cannot outlive the borrow it
/// writes from and nothing is copied to give it one.
pub(crate) fn feeding(dir: &Path, args: &[&str], input: &str, ok: &[i32]) -> Option<String> {
    use std::io::Write;

    let mut child = Command::new("git")
        // Reading a repository should never take a lock on it — see
        // [`accepting`], which is where that is said and why.
        .arg("--no-optional-locks")
        .args(args)
        .unseen()
        .current_dir(dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    let output = std::thread::scope(|writers| {
        // A git that has already exited — the directory is no repository, say —
        // is a pipe with nobody at the other end, which is an error to swallow
        // rather than a reason to fail: what it exited with is read below like
        // any other answer. The same goes for one that stops reading partway.
        if let Some(mut writing) = child.stdin.take() {
            writers.spawn(move || {
                let _ = writing.write_all(input.as_bytes());
            });
        }

        child.wait_with_output()
    })
    .ok()?;

    if !ok.contains(&output.status.code()?) {
        return None;
    }

    Some(String::from_utf8_lossy(&output.stdout).into_owned())
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;

    use super::*;

    /// What one variable is set to on `command`, where it sets one.
    fn set<'a>(command: &'a Command, name: &str) -> Option<Option<&'a OsStr>> {
        command
            .get_envs()
            .find(|(key, _)| *key == OsStr::new(name))
            .map(|(_, value)| value)
    }

    /// The Author's token reaches a clone's git the way it reaches the rest of
    /// Verkstead's own: as `GH_TOKEN` under [`Credential`]'s helper, behind the
    /// SSH rewrites a session's sandbox configures, with nobody asked for
    /// anything — and never `gh`'s own helper, which would answer as the host.
    ///
    /// [`Credential`]: crate::github::Credential
    #[test]
    fn a_clone_authenticates_as_its_author() {
        let command = cloned_by(
            "git@github.com:you/widgets.git",
            Path::new("/srv/repos"),
            "widgets",
            &crate::github::Credential::token("ghp_thetoken"),
        );

        assert_eq!(
            set(&command, "GH_TOKEN"),
            Some(Some(OsStr::new("ghp_thetoken")))
        );
        assert_eq!(
            set(&command, "GIT_TERMINAL_PROMPT"),
            Some(Some(OsStr::new("0")))
        );

        let args: Vec<String> = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();

        assert!(
            args.contains(&"url.https://github.com/.insteadOf=git@github.com:".to_owned()),
            "its SSH spelling is fetched over HTTPS: {args:?}",
        );
        assert!(
            !args
                .iter()
                .any(|arg| arg.contains("gh auth git-credential")),
            "gh's own helper is left out: {args:?}",
        );
        assert!(
            args.contains(&"credential.helper=".to_owned()),
            "the host's helpers are cleared: {args:?}",
        );
        assert!(
            args.ends_with(&[
                "clone".to_owned(),
                "--quiet".to_owned(),
                "--".to_owned(),
                "git@github.com:you/widgets.git".to_owned(),
                "widgets".to_owned(),
            ]),
            "{args:?}",
        );
    }

    /// An Author with no token leaves git to the host's own credentials, and
    /// sets no `GH_TOKEN`: present and empty is a login `gh` fails on obscurely.
    #[test]
    fn no_token_sets_no_gh_token() {
        let command = cloned_by(
            "https://github.com/you/widgets",
            Path::new("/srv"),
            "w",
            &crate::github::Credential::host(),
        );

        assert_eq!(set(&command, "GH_TOKEN"), None);
    }
}
