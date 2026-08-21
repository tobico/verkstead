//! Registering a Repo, and the list of the ones that are: what the viewer sends
//! and what it is handed back.
//!
//! Every way registering can be refused is a named outcome rather than a status
//! code, as answering and archiving are — because each of them is a different
//! sentence to put in front of the human, and none of them is something to
//! retry. A path outside the Watched Paths is the boundary doing its job, not an
//! error.

use serde::{Deserialize, Serialize};

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
    AlreadyRegistered,
}
