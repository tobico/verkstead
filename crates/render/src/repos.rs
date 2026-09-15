//! Registering a Repo, making one, the list of the ones that are, one of them
//! opened, and taking one away again: what the viewer sends and what it is
//! handed back.
//!
//! Every way registering can be refused is a named outcome rather than a status
//! code, as answering and locking are — because each of them is a different
//! sentence to put in front of the human, and none of them is something to
//! retry. A directory that is not a repository is something to go and put right,
//! not an error.
//!
//! Making one is refused the same way and for the same reason, with one more
//! of its own behind it: a create that got half way is a directory on somebody's
//! disk rather than a form to fill in again.
//!
//! And a create may reach GitHub as well, which is the one outcome here that is
//! neither a Repo nor a refusal: the repository is made, and the remote it was
//! to have is not. Both halves travel — see [`Created::MadeWithoutRemote`].

use serde::{Deserialize, Serialize};

use crate::AbandonedRoadmap;

#[cfg(feature = "typescript")]
use ts_rs::TS;

/// One row of the Repo list.
///
/// The path is the resolved one the server recorded rather than whatever was
/// typed to register it: that is the directory Verkstead will actually work in,
/// so it is the one worth showing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct RepoEntry {
    pub id: i64,
    pub name: String,
    pub path: String,
    pub default_branch: String,
}

/// A repository the human is asking Verkstead to take on, named by its absolute
/// path.
///
/// A path and nothing else: the name and the default branch are read off the
/// repository rather than claimed, for the same reason the CLI derives a Set's
/// `project` and `branch` instead of trusting them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct Registration {
    pub path: String,
}

/// What became of a registration.
///
/// The refusals are the server's and not the form's: a check the browser made
/// is a courtesy, and every request reaching this endpoint is decided here
/// whether or not a form was involved.
///
/// The two outcomes that leave a Repo registered carry it, because whoever
/// asked is usually about to put something *on* it — the Repo dropdown's **Open
/// repo** row registers one and lands the draft on it — and the path that was
/// typed is not the resolved path the Repo is recorded under. A caller left to
/// match its own spelling against the list afterwards would be guessing at an
/// answer this endpoint is already holding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum Registered {
    /// Recorded. It is on the list, and it is there after a restart.
    Added(RepoEntry),

    /// The path was relative. There is nothing to resolve it against that would
    /// mean the same thing twice, so it is refused rather than guessed at.
    NotAbsolute,

    /// Nothing is at that path.
    Missing,

    /// It is a directory, but not the root of a git repository. Asked of the
    /// resolved path, so a symlink pointing at one is one.
    NotARepository,

    /// A git repository with no branch to call its default — a detached HEAD,
    /// most likely. A Conversation has nothing to branch from until there is
    /// one.
    NoDefaultBranch,

    /// That repository is registered already, under this path or another
    /// spelling of it.
    ///
    /// A path a Repo that was taken away still holds is not this: registering it
    /// again revives that Repo, and the answer is [`Registered::Added`].
    ///
    /// It carries the Repo for the reason [`Registered::Added`] does, and it is
    /// the same Repo either way: a settings pane goes on saying *registered
    /// already* and nothing else, while a dropdown that was registering one to
    /// work in has the repository it named rather than a dead end.
    AlreadyRegistered(RepoEntry),
}

/// What became of taking one off the registry.
///
/// A removal rather than a deletion, which is why nothing here says anything
/// about a Timeline: every Conversation ever started on a Repo goes on naming
/// it, and what a removal changes is only what is offered for new work.
///
/// Shaped like [`ProfileDeleted`](crate::ProfileDeleted), because it is the same
/// sentence about the other thing the settings page configures — with the
/// refusal that one no longer has. A Repo is where the work is *being done*, and
/// unregistering one out from under a run would take the directory the session
/// is standing in; a Profile is an account the next session would have been
/// launched under, and losing it costs that session rather than this one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum RepoRemoved {
    /// Taken off the registry. Every list stops offering it, and the pane it was
    /// removed from is spent.
    Removed,

    /// There is no registered Repo with that id — one somebody has already taken
    /// away included, which is a pane left open in another tab.
    NoSuchRepo,

    /// A Conversation that is neither Done nor Closed is on it. Work still going
    /// on in a repository is the reason to keep it registered.
    InUse,
}

/// One registered Repo, whole: the row, and everything a reading of the
/// repository itself adds to it.
///
/// What a create answers with — see [`Created`], which is the one thing left
/// carrying one. The settings had a pane per Repo drawing every field here and
/// it is gone; what is drawn of a Repo there is its name, and each of these is
/// read where it is used instead.
///
/// The row's own three facts come along with the rest rather than being left to
/// the list: whoever made a repository is about to put a draft on it, and a
/// caller that had to go and read the Repo it just made would be asking for
/// something the answer was already holding.
///
/// Nothing here is stored beyond those three. The branches are git's own answer,
/// the counts are the store's, and the roadmaps are read off the repository the
/// way the notice under the new-conversation box reads them — so a branch
/// somebody pushed a minute ago is on this list, and a roadmap somebody has
/// since picked up is not.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct RepoView {
    pub id: i64,
    pub name: String,

    /// The resolved path, which is the directory Verkstead will work in.
    pub path: String,

    /// And what a Conversation branches from unless it is told otherwise.
    pub default_branch: String,

    /// Every branch the repository has, local and remote-tracking both — the
    /// same reading the base dropdown is filled from.
    pub branches: Vec<String>,

    /// How many Conversations are on this Repo and still going: everything that
    /// is neither Done nor Closed, a Draft included.
    pub live: i64,

    /// And how many are over, which is Done and Closed together. The two are
    /// counted apart because they are read for different reasons — what is on
    /// this Repo now, and what has been.
    pub finished: i64,

    /// The roadmaps in it that nothing is driving, as the notice under the
    /// new-conversation box finds them. Empty where there are none, which is
    /// most repositories most days.
    pub roadmaps: Vec<AbandonedRoadmap>,
}

/// A repository the human is asking Verkstead to *make*, said as where it is to
/// go and what it is to be called.
///
/// Two fields rather than the one path a [`Registration`] carries, because the
/// two halves are answered differently: the parent is browsed for, and the name
/// is typed. Joining them in the browser would be the one place a path is built
/// out of a separator the server never agreed to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct Creation {
    /// The directory the new repository goes in. Absolute, and somewhere the
    /// server can write.
    pub parent: String,

    /// And what to call it, which is the directory's name and so the Repo's:
    /// what a Repo is called is read off the directory rather than claimed, and
    /// a create is the one moment the human chooses the directory.
    pub name: String,

    /// And whether the same repository is to be made on GitHub, pushed to, and
    /// left as this one's `origin`.
    ///
    /// Asked because the pipeline ends in a push and a pull request: a
    /// repository with nowhere to push is one that will stop halfway through
    /// the first Conversation. What is made there is private — a repository
    /// made from here is somebody's work before it is anybody else's business,
    /// and public is a decision to take deliberately rather than by leaving a
    /// box alone.
    ///
    /// False where no token is configured, there being nothing to make it as:
    /// the modal draws no tick at all then, and says a remote is needed before
    /// the work is finished.
    pub github: bool,
}

/// What became of a create.
///
/// Every refusal is a named outcome for the reason [`Registered`]'s are, and one
/// more of its own: a create that got half way is a directory on somebody's
/// disk, so what comes back has to be a sentence about their machine rather
/// than a status code.
///
/// A refusal registers nothing. The two outcomes that leave a Repo both carry
/// the whole opened Repo rather than the row: the modal that asked for it is
/// about to put a draft on it, and a page that had to go and read the Repo it
/// just made would be asking for something the server was already holding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum Created {
    /// Made: the directory is there, `main` holds one commit by the configured
    /// author, and the Repo is on the registry — and where GitHub was asked
    /// for, the repository is there too, private, with `origin` set and `main`
    /// pushed.
    Made(RepoView),

    /// Made here, and not on GitHub.
    ///
    /// Not a failed create, which is why it carries the Repo the way
    /// [`Created::Made`] does: the directory, the commit and the registration
    /// all stand, and what is missing is a remote that can be added afterwards.
    /// So the answer holds both halves rather than choosing between them — the
    /// Repo lands on the draft exactly as a clean create's does, and the modal
    /// says what failed.
    ///
    /// `why` is `gh`'s own account of it, for the reason [`Created::Refused`]
    /// carries git's.
    MadeWithoutRemote { repo: RepoView, why: String },

    /// Nothing is at the parent, or what was typed was not an absolute path —
    /// which is the same sentence, there being no directory either way for the
    /// new one to go in.
    ParentMissing,

    /// Something of that name is in that parent already. A create never writes
    /// into a directory that is there: what is in it is somebody's, and a
    /// repository made around it would be a repository nobody asked for.
    ///
    /// One that is already a repository is **Open repo**'s to register rather
    /// than this one's to make.
    AlreadyThere,

    /// The name is not one a directory can have — blank, or a path rather than a
    /// name.
    BadName,

    /// Nobody is configured to commit as. The first commit is this repository's
    /// own history from here on, so it is refused rather than made by a
    /// stand-in — see the settings page's **Git author**.
    NoAuthor,

    /// Anything else, in git's own words where git is what failed: the directory
    /// could not be made, or `git` would not do one of the four things a fresh
    /// repository is made of.
    ///
    /// Nothing is on the registry either way, and a directory this got half way
    /// through making is taken back: a create that did not happen leaves nothing
    /// behind that looks as though it did.
    Refused(String),
}
