//! Registering a Repo, the list of the ones that are, one of them opened, and
//! taking one away again: what the viewer sends and what it is handed back.
//!
//! Every way registering can be refused is a named outcome rather than a status
//! code, as answering and locking are — because each of them is a different
//! sentence to put in front of the human, and none of them is something to
//! retry. A path outside the Watched Paths is the boundary doing its job, not an
//! error.

use serde::{Deserialize, Serialize};

use crate::{AbandonedRoadmap, ConflictResolution};

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum Registered {
    /// Recorded. It is on the list, and it is there after a restart.
    Added,

    /// The path was relative. There is nothing to resolve it against that would
    /// mean the same thing twice, so it is refused rather than guessed at.
    NotAbsolute,

    /// Nothing is at that path.
    Missing,

    /// It resolves to somewhere outside every Watched Path. The boundary is
    /// checked after `..` and every symlink have been taken out, so a path that
    /// merely reads as inside one lands here too.
    OutsideWatchedPaths,

    /// It is a directory inside a Watched Path, but not the root of a git
    /// repository.
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
    AlreadyRegistered,
}

/// What became of taking one off the registry.
///
/// A removal rather than a deletion, which is why nothing here says anything
/// about a Timeline: every Conversation ever started on a Repo goes on naming
/// it, and what a removal changes is only what is offered for new work.
///
/// Shaped like [`ProfileDeleted`](crate::ProfileDeleted), because it is the same
/// sentence about the other thing the settings page configures — and refused for
/// the same kind of reason.
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

/// One registered Repo opened: everything the card cannot hold, read at the
/// moment it is asked for.
///
/// The card's own three facts come along with it rather than being left to the
/// list behind the pane. The pane is a page of its own as far as a link is
/// concerned — somebody reloads on it, or arrives from a message — and a pane
/// that drew its own title out of another read would have nothing to say until
/// that read landed.
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

    /// How a conflicted pull request in this repository is resolved, where this
    /// Repo has been told something other than what every other one does.
    ///
    /// `null` is *whatever the global setting says* rather than *merge*: the two
    /// are the same answer today and stop being the same the moment the global
    /// is changed, and a Repo that had quietly frozen this morning's global
    /// would be a choice nobody made. What that global is, is on the settings
    /// themselves — see [`crate::SettingsView::conflict_resolution`].
    pub conflict_resolution: Option<ConflictResolution>,
}

/// How one Repo is to resolve a conflict from now on, which is the one thing
/// there is to *say* to a registered Repo besides taking it away.
///
/// `null` takes the override back rather than writing the global's word down:
/// what *use the global setting* means is that this Repo says nothing, and a
/// Repo holding a copy of today's global would go on holding it after the global
/// moved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct ResolutionEdit {
    pub resolution: Option<ConflictResolution>,
}
