//! The files half of Code: the roots its tree stands on, one folder of one of
//! them at a time, one file of one of those opened — and a file or a folder
//! made in one of them, or renamed.
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
//! **A file is read whole, with a version and a kind.** The version is a hash
//! of the bytes, which a write names itself as being over; the kind is what the
//! bytes turned out to be — text, an image, a binary the server will not send,
//! or a file over the size cap — see [`FileReading`].
//!
//! **And a write names the version it is over** — see [`FileWrite`] and
//! [`FileWritten`]. A write over a file that has moved since is refused, which
//! is what the bar in front of the human is drawn from: *Reload* takes the
//! disk's text, and *Keep mine* keeps theirs over the version the disk now has
//! so that the next save lands. Last writer wins was decided against — an
//! agent's edit silently overwritten by a human who never saw it is exactly
//! what the version exists to surface.
//!
//! **And a row of the tree makes one** — see [`FileMaking`] and [`FileMade`]:
//! an empty file or a folder, under a folder of a root, named in full. One
//! refusal of its own, a name that is already taken, and every other one is the
//! write's said about a path that is not there yet.
//!
//! **And renames one** — see [`FileRenaming`] and [`FileRenamed`]: a path in a
//! root, and the name it is to have. A *name* rather than a path, which is what
//! keeps a rename inside the root it started in — two roots are two
//! repositories, and a file taken out of one checkout and put in another is not
//! a thing a tree gets to do. One refusal of its own beyond the making's: a root
//! itself, which is a Worktree rather than something in one.
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

/// What one file of one of those roots is, read — or the named reason it is
/// not drawn.
///
/// **A read says which of four kinds of thing it read**
/// ([ADR 0019](../../../docs/adr/0019-the-code-pane.md), *Monaco, whole*):
/// text, which opens in the editor; an image, which is previewed in its tab;
/// any other binary, which is a line saying so; and a file over the size cap,
/// which is another. The kinds are the server's reading rather than the
/// viewer's guess, because the bytes are the server's and the whole point of
/// the last two is that they never cross the wire.
///
/// **And a read carries a version**, which is a hash of the bytes it read: a
/// [`FileWrite`] names the version it is over, and a write over a file the
/// agent has changed since is refused (*Versioned reads, and a stale write is
/// refused*). On the text alone, that being the only kind anything writes back.
///
/// The refusals are [`FolderListing`]'s, said about a file: each of them is a
/// different sentence for the human and none of them is a status code.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum FileReading {
    /// Text, which is what opens in the editor.
    ///
    /// Anything that is valid UTF-8 with no NUL byte in it, which is git's own
    /// reading of what a text file is — and is a wider net than a list of
    /// extensions, a repository being full of files named for nothing in
    /// particular.
    Text {
        /// The file this read, as it was asked for — the folder listing's own
        /// rule, and for its reason: a path answered back through a resolution
        /// would read as somewhere else on a machine whose temporary directory
        /// is a symlink.
        path: String,

        /// A hash of the bytes that were read, which a write names itself as
        /// being over.
        ///
        /// Of the bytes rather than of the text, so that what a write is
        /// measured against is what is on the disk: a file is read and hashed
        /// in one pass, and the same pass is what a save compares against.
        version: String,

        /// What is in it.
        text: String,

        /// Whether the root it is in can be written.
        ///
        /// The root's own flag rather than the file's mode: a read-only
        /// companion is checked out detached and nothing in it is to be
        /// written, whatever its permissions happen to say. A file that opens
        /// read-only takes no typing, which is what saves a human finding out
        /// by typing.
        writable: bool,
    },

    /// A picture, which is previewed in its tab rather than edited.
    ///
    /// The bytes come with it, base64, rather than through a second endpoint
    /// the tab would then fetch: one read is one answer, and what bounds its
    /// size is the same cap everything else here is under.
    Image {
        path: String,

        /// What to draw it as — `image/png` and the rest — read off the name.
        ///
        /// The extension rather than the bytes: what a browser will draw is
        /// decided by this string, and a file named `.png` that is not one is
        /// a broken picture either way.
        media_type: String,

        /// Its bytes, base64.
        base64: String,
    },

    /// Something else, which the server will not send.
    ///
    /// A line saying so rather than the bytes: an object file in an editor is
    /// mojibake, and one in a tab is a megabyte across the wire for nothing.
    Binary,

    /// It is larger than Code opens — see the cap in `verkstead_server`'s
    /// `files`.
    ///
    /// Its own answer rather than *binary*, because a text file can be over the
    /// cap too and what to do about it is different: nothing is wrong with the
    /// file, and a terminal beside the tab will open it.
    TooLarge,

    /// It is under none of this Conversation's Worktrees — [`FolderListing::Outside`].
    Outside,

    /// It is inside a repository's git directory, which Code does not show.
    UnderGit,

    /// The Worktree it is in is no longer on disk.
    RootGone,

    /// The root is there and this is not: a file deleted, renamed, or committed
    /// away by a checkout in a terminal beside the tree.
    Missing,

    /// Something is at it and it is not a file — a folder, which the tree
    /// expands rather than opens.
    NotAFile,

    /// It is a file the server cannot read, and this is why — permissions, or a
    /// file that went between the request and the reading.
    Unreadable { why: String },
}

/// A file as the human has it, written back over the version it was read at.
///
/// The path, so that a write is bounded by exactly the roots a read is; the
/// version, which is what the read handed over and what the disk is measured
/// against; and the text itself. Three fields and no flag: there is one kind of
/// write, and it is *this text, if the file is still the one I read*
/// ([ADR 0019](../../../docs/adr/0019-the-code-pane.md), *Versioned reads, and
/// a stale write is refused*).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct FileWrite {
    pub path: String,

    /// The version the read carried, which is what this write is over.
    ///
    /// Not optional and never blank: a write that named no version would be
    /// last-writer-wins by the back door, and the collision is the point.
    pub version: String,

    /// And what to put there.
    pub text: String,
}

/// What became of writing it.
///
/// **A write over a version that has moved is refused**, which is what draws
/// the bar in front of the human: *Reload* takes the disk's text, and *Keep
/// mine* keeps theirs over the version the disk now has so that their next save
/// lands. Both of them read the file afresh through the endpoint beside this
/// one, and they differ only in what becomes of the text in the editor.
///
/// So this refusal is a bare word where [`FileWritten::Written`] carries a
/// version, and the asymmetry is the point: after a write that landed the
/// viewer knows what is on the disk, because it is what it just sent. After one
/// that was refused it does not, and a version handed over without the text it
/// belongs to would be half an answer — enough for the next save to land, and
/// not enough for the viewer to say whether the editor still differs from the
/// disk at all.
///
/// The rest are [`FileReading`]'s refusals said about a write, plus the one
/// that is a write's alone: a root that takes none. Each is a sentence for the
/// human rather than a status code, the way every other refusal in this module
/// is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum FileWritten {
    /// It is on disk, and this is the version it now has.
    ///
    /// Answered back rather than left to be read again, because the tab has to
    /// know it: the next save is a write over this, and a tab that had to
    /// re-read the file to find out would be a read after every save.
    Written { version: String },

    /// The file has moved since it was read, so nothing was written.
    Stale,

    /// The root it is in takes no writes: a read-only companion, checked out
    /// detached and there to be read.
    ///
    /// The root's own flag rather than the file's mode, which is the same thing
    /// [`FileReading::Text::writable`] says — so an editor that took no typing
    /// is an editor whose save was never going to land either.
    ReadOnly,

    /// It is under none of this Conversation's Worktrees.
    Outside,

    /// It is inside a repository's git directory, which Code does not touch.
    UnderGit,

    /// The Worktree it is in is no longer on disk.
    RootGone,

    /// The root is there and the file is not: deleted, renamed, or committed
    /// away by a checkout in a terminal beside the tab.
    Missing,

    /// Something is at it and it is not a file.
    NotAFile,

    /// The server could not write it, and this is why — permissions, a full
    /// disk, or a file that went between the reading and the writing.
    Unwritable { why: String },
}

/// A file or a folder to be made, named in full.
///
/// One field, because a path is the one thing the tree has to say: a row of it
/// holds the folder it was listed from, and what a human types into the field
/// under that row is a name joined onto it — which is exactly how
/// [`FolderEntry::path`] is built, and is why every other endpoint here is
/// asked by path as well.
///
/// What is at the end of that path is what is made, and the folder above it is
/// the one it is made in: so a Worktree that has gone, a folder that has gone
/// and a root that takes no writes are all answers about the path's *parent*,
/// and [`FileMade::Taken`] is the one about its last segment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct FileMaking {
    pub path: String,
}

/// What became of making one.
///
/// **A new file is empty and a new folder holds nothing** — there is no
/// template and no content in the request, because what is being made is a row
/// in the tree. A file made this way opens as a tab the human then types into
/// and saves through [`FileWrite`], which is where content has always come
/// from.
///
/// The refusals are [`FileWritten`]'s said about a path that is not there yet,
/// with one of their own: a name that is already taken. Each of them is a
/// sentence drawn beside the field the name was typed into rather than a status
/// code, the way every other answer of this API is
/// ([ADR 0019](../../../docs/adr/0019-the-code-pane.md), *The tree*).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum FileMade {
    /// It is on disk, and this is where — the path as it was asked for, which
    /// is what the tab a new file opens in is opened at.
    ///
    /// Answered back rather than assumed, for [`FolderEntry::path`]'s reason:
    /// what the tree holds is spelled the way the root it came from is, and a
    /// path the viewer built for itself out of a resolved one would read as
    /// somewhere else on a machine whose temporary directory is a symlink.
    Made { path: String },

    /// Something of that name is already in the folder, and nothing was
    /// written.
    ///
    /// Whatever it is: a file where a folder was asked for, a folder where a
    /// file was, or a link to either. What the human has to do about it is the
    /// same in every case — type another name — and a refusal that told them
    /// which kind of thing is in the way would be telling them about a row the
    /// tree is already drawing.
    Taken,

    /// The root it would be in takes no writes: a read-only companion, checked
    /// out detached and there to be read.
    ///
    /// The root's own flag rather than the folder's mode — [`FileWritten::ReadOnly`]'s
    /// rule, for its reason. The tree draws neither row under such a root, so
    /// this is the endpoint refusing on its own account rather than anything a
    /// press can reach.
    ReadOnly,

    /// It is under none of this Conversation's Worktrees.
    ///
    /// Which is what a path that climbs comes to as well: the name typed into
    /// the field is joined onto a folder of the tree, and a `..` in it is a
    /// path spelled out of a root rather than a name.
    Outside,

    /// It is inside a repository's git directory, which Code does not touch.
    UnderGit,

    /// The Worktree it would be in is no longer on disk.
    RootGone,

    /// The folder it would go in is not there: deleted, renamed, or committed
    /// away by a checkout in a terminal beside the tree.
    Missing,

    /// Something is at the folder it would go in and it is not a folder.
    NotAFolder,

    /// The server could not make it, and this is why — permissions, a full
    /// disk, or a folder that went between the check and the making.
    Unwritable { why: String },
}

/// Something in a root, renamed: what it is now, and what it is to be called.
///
/// **A name rather than a path**, which is the whole of why a rename cannot
/// cross two roots ([ADR 0019](../../../docs/adr/0019-the-code-pane.md), *The
/// tree*). Two roots are two repositories, and a file taken out of one checkout
/// and put in another is not something the pane has any way to ask for: the
/// field is drawn over the row, it holds a name, and what the server does with
/// it is join it onto the folder the row is already in.
///
/// So this is [`FileMaking`] split in two. The making carries one path because
/// what it names is not there yet and the name is the last segment of it; this
/// carries the path of something that *is* there and the name it is to have,
/// and a `name` with a separator in it is a path rather than a name — see
/// [`FileRenamed::Outside`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct FileRenaming {
    /// What is being renamed, as the tree has it.
    pub path: String,

    /// And what it is to be called: one segment, which is what was typed into
    /// the field drawn over the row.
    pub name: String,
}

/// What became of renaming it.
///
/// [`FileMade`]'s answers said about a move rather than a making — a name
/// already taken, a root that takes no writes, a path outside every root, a path
/// under `.git`, a Worktree that has gone, a path that has — with the one that
/// is a rename's alone: a root itself, which is a Worktree rather than anything
/// in one and has no name here to change. What the human knows a root by is the
/// Repo it is a checkout of, and that is the registry's business rather than the
/// tree's.
///
/// Each of them is a sentence drawn beside the field the name was typed into
/// rather than a status code, the way every other answer of this API is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum FileRenamed {
    /// It has moved, and this is where — the folder it was already in with the
    /// new name joined on.
    ///
    /// Answered back rather than assumed, for [`FileMade::Made`]'s reason: what
    /// the tree holds is spelled the way the root it came from is, and a viewer
    /// that joined with a `/` on a machine whose paths use a `\` would be
    /// handing the tab a second name for the one file.
    ///
    /// **And it is what every open tab follows.** A tab at the old path is
    /// retitled and re-keyed onto this one, and a folder renamed carries
    /// everything under it — see `Code.tsx`, where that is done.
    Renamed { path: String },

    /// Something of that name is already in the folder, and nothing moved.
    ///
    /// Whatever it is, for [`FileMade::Taken`]'s reason: what the human does
    /// about it is type another name in every case.
    ///
    /// Not what the thing being renamed is: a name that differs from the one it
    /// has only in its case is a rename this allows, a filesystem that does not
    /// tell two cases apart being the one place where a thing is standing in its
    /// own way.
    Taken,

    /// It is a root, which is a Worktree rather than something in one.
    ///
    /// The refusal that is a rename's alone. The tree offers no Rename row on a
    /// root, so this is the endpoint refusing on its own account — and a root
    /// renamed would be a Worktree moved out from under the session working in
    /// it, which is not something a file tree gets to do.
    IsRoot,

    /// The root it is in takes no writes: a read-only companion, checked out
    /// detached and there to be read.
    ReadOnly,

    /// It is under none of this Conversation's Worktrees.
    ///
    /// Which is what a name that is not a name comes to as well: a `name` with a
    /// separator in it, or one spelled `..`, is a path out of the folder the row
    /// is in rather than something to call the row — and the one thing this
    /// endpoint will not do is move anything between two roots.
    Outside,

    /// It is inside a repository's git directory, which Code does not touch —
    /// asked of what it is now and of what it would be called both, a name
    /// spelled `.git` being the one way past the first.
    UnderGit,

    /// The Worktree it is in is no longer on disk.
    RootGone,

    /// The root is there and this is not: deleted, renamed, or committed away by
    /// a checkout in a terminal beside the tree.
    Missing,

    /// The server could not move it, and this is why — permissions, a folder
    /// that is not empty where one is being moved over, or a path that went
    /// between the check and the move.
    Unwritable { why: String },
}
