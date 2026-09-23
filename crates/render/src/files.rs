//! The files half of Code: the roots its tree stands on, and one folder of one
//! of them at a time.
//!
//! **A root is a Worktree** — the Conversation's own first, then each
//! companion's in the order the Conversation carries them
//! ([ADR 0019](../../../docs/adr/0019-the-code-pane.md), *The tree*). What
//! bounds the files API is exactly these: the server reads and writes as
//! itself, with no Sandbox in front of it, and a path that is under none of
//! them is refused.
//!
//! **A read-only companion is a root here**, marked read-only rather than left
//! out. This is where it parts company with the Diff a Set carries, which is
//! composed from the writable Worktrees alone: a detached read-only checkout
//! has nothing uncommitted to show, and it has plenty to *read*.
//!
//! **A folder listing is one folder**, read when it is expanded and never a
//! walk — the shape a path field's dropdown browses with, and for the same
//! reason: a directory holding ten thousand files costs one reading of it
//! whatever is underneath.
//!
//! Every refusal is a named outcome rather than a status code, as registering
//! a Repo refuses and as that dropdown's listing does — because each of them is
//! a different sentence for the human and none of them is a failure to retry: a
//! path outside every root, a path inside the git directory, a Worktree that is
//! no longer on disk and a folder that has gone are four different things to be
//! told.

use serde::{Deserialize, Serialize};

#[cfg(feature = "typescript")]
use ts_rs::TS;

/// The Worktrees Code draws a root apiece for.
///
/// The Conversation's own first and each companion's after it, which is the
/// order the tree draws them in. Empty for a Conversation that has no
/// checkouts — one before grilling starts, and one that has been closed — which
/// is a tree with nothing in it rather than anything to report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct FileRootsView {
    pub roots: Vec<FileRoot>,
}

/// One of them: which repository it is a checkout of, where it is, and what may
/// be done in it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct FileRoot {
    /// The Repo's name, which is what the row is called: a root is a checkout
    /// and what the human knows it by is the repository it is of.
    pub repo: String,

    /// And the directory itself, which is what a folder listing under it is
    /// asked for by.
    ///
    /// As the Conversation recorded it rather than resolved: the tree asks for
    /// paths built out of this one, and a path answered back through a
    /// resolution would read as somewhere else on a machine whose temporary
    /// directory is a symlink.
    pub path: String,

    /// Whether this is the Conversation's own, as against a companion's.
    ///
    /// The first root is always the own one where there is one, so this says
    /// nothing a reader could not count — except on a Conversation whose own
    /// Worktree has gone and whose companions have not, which is where a tree
    /// drawing the first row as the work's own would be lying.
    pub own: bool,

    /// And whether anything here can be written. False for a read-only
    /// companion, which is checked out detached and is a root to read.
    pub writable: bool,
}

/// What one folder of a root holds, or the named reason it holds nothing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum FolderListing {
    /// What is in it, folders first and then by name — see [`FolderEntry`].
    ///
    /// Git-ignored paths are not among them, and neither is the git directory:
    /// a tree drawn over a Rust checkout is a tree whose first row is `target/`
    /// otherwise, and git is the one thing that knows what a repository ignores.
    Listed {
        /// The folder this lists, as it was asked for.
        path: String,

        entries: Vec<FolderEntry>,
    },

    /// It is under none of this Conversation's Worktrees.
    ///
    /// The whole of what bounds the files API, so it is the refusal that
    /// matters most: the server reads as itself, and a path that escapes a root
    /// — spelled with a `..`, or arrived at through a symlink out of one — is
    /// the machine being asked for through a pane that shows one Conversation.
    Outside,

    /// It is inside a repository's git directory, which Code does not show.
    ///
    /// Its own answer rather than *outside*, because it is inside a root: what
    /// it names is real, and what it is is a repository's insides rather than
    /// the work in it.
    UnderGit,

    /// The Worktree it is in is no longer on disk.
    ///
    /// A Conversation that has been closed, or a directory somebody removed by
    /// hand — the root itself has gone, rather than anything about the folder
    /// asked for.
    RootGone,

    /// The root is there and this is not: a folder deleted, renamed, or
    /// committed away by a checkout in a terminal beside the tree.
    Missing,

    /// Something is at it and it is not a folder.
    NotAFolder,

    /// It is a folder the server cannot read, and this is why — permissions, or
    /// a directory that went between the request and the reading. The
    /// filesystem's answer rather than the server's failure.
    Unreadable { why: String },
}

/// One thing in a folder.
///
/// The name and the whole path both, for the reason a browse's entry carries
/// both: the name is the row, and the path is what the next listing — or, from
/// the stage after this one, the read that opens the file — is asked for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct FolderEntry {
    pub name: String,

    /// Where it is: the folder that was asked for, with the name joined on.
    ///
    /// Built rather than read back off the directory, so that every path the
    /// tree holds is spelled the way the root it came from is. What follows a
    /// symlink is the reading, which happens afresh each time one of these is
    /// asked about.
    pub path: String,

    /// Whether it is a folder to expand, as against a file to open.
    ///
    /// Two kinds rather than the browse's three: a `.git` inside a checkout is
    /// not something Code shows at all, so there is no repository to mark.
    /// Followed rather than read off the link, which is what the filesystem
    /// itself would do with it.
    pub folder: bool,
}
