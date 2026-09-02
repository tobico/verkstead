//! Browsing the filesystem one directory at a time: what a path field's
//! dropdown asks for, and what it is handed back.
//!
//! One directory per request, and never a walk. A field that browses opens on
//! the directory its text names, and asks again for each level the human drills
//! into — so what crosses the wire is a listing of what somebody is looking at
//! rather than a tree nobody will read the whole of.
//!
//! Two scopes, because there are two kinds of field. Some values the server
//! refuses outside the Watched Paths — a Repo is registered from inside one, an
//! Agent Profile's account is read from inside one — and a dropdown that
//! offered anything else would be offering what the save is going to turn down.
//! The rest are values the boundary says nothing about, and those browse
//! anywhere the server can read. Which of the two a field is, is the field's own
//! word: see [`BrowseScope`].
//!
//! Every refusal is a named outcome rather than a status code, as registering a
//! Repo refuses — because each of them is something the dropdown draws in words
//! where its rows would be, and none of them is a failure to retry. A directory
//! the server cannot read is included in that: permissions, or a directory that
//! went between the request and the reading, are the filesystem answering rather
//! than the server breaking.

use serde::{Deserialize, Serialize};

#[cfg(feature = "typescript")]
use ts_rs::TS;

/// Which kind of field is asking, which is what decides where it may look.
///
/// Sent in the query rather than being a route of its own: it is one reading,
/// asked two ways round, and the answer has the same shape either way.
///
/// Spelled in lower case, unlike everything else the viewer sends: this one
/// travels in a URL beside the path, where a capital would read as a mistake.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum BrowseScope {
    /// For a field whose value the server refuses outside the Watched Paths.
    /// Asked for no path it answers the roots themselves, and asked for one it
    /// answers only where that path resolves to somewhere inside a root — the
    /// same decision the save is going to make, made early enough to be a
    /// dropdown.
    Watched,

    /// And for a field the boundary says nothing about, which browses from `/`
    /// down. Anything the server can read lists.
    Anywhere,
}

/// What one directory holds, or the reason it does not answer.
///
/// [`DirectoryListing::Listed`] is the ordinary answer and the rest are the
/// dropdown's other rows: a line where the entries would be, saying what the
/// server made of the path the field holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum DirectoryListing {
    /// What is in it, directories first and then by name — see
    /// [`DirectoryEntry`]. Dotfiles are in here whether or not the field draws
    /// them: what is hidden is a decision about a field, and a listing that had
    /// already taken them out could not be the one a field pointed at `.claude`
    /// browses with.
    Listed {
        /// The directory this lists, resolved — `..` taken out and every
        /// symlink followed, which is what the entries below hang off.
        ///
        /// `null` for the [`BrowseScope::Watched`] roots, which are a listing
        /// with no one directory above them: the boundary is a set of
        /// directories rather than a place.
        path: Option<String>,

        entries: Vec<DirectoryEntry>,
    },

    /// The path was relative. Nothing here resolves one — the directory the
    /// server happens to be running in is not something a path should mean.
    NotAbsolute,

    /// Nothing is at it, which is the ordinary state of a field being typed
    /// into: the dropdown follows the deepest directory the text does name.
    Missing,

    /// Something is, and it is not a directory. A file names no listing, and a
    /// field pointed at one is a field whose browse has gone as deep as it goes.
    NotADirectory,

    /// It resolves to somewhere no Watched Path covers, asked in the scope that
    /// is bounded by them. The boundary is consulted on the resolved path, so a
    /// path that merely reads as inside one lands here too.
    OutsideWatchedPaths,

    /// It is a directory the server cannot read, and this is why. Permissions,
    /// or a directory that went between one request and the next — the
    /// filesystem's answer rather than the server's failure, so it is a row the
    /// dropdown draws rather than a 500 it reports.
    Unreadable { why: String },
}

/// One thing in a directory.
///
/// Both the name and the whole path, because the field uses each for something
/// different: the name is the row, and the path is what a tap writes into the
/// input and asks the next listing for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct DirectoryEntry {
    /// What it is called in the directory holding it — the row's own word.
    ///
    /// The Watched Paths' roots have no directory holding them, so what comes
    /// back for one of those is the last segment of it. A field drawing that
    /// listing has the whole path beside it and may say more.
    pub name: String,

    /// And where it is: absolute, and under the resolved directory it was read
    /// out of.
    pub path: String,

    pub kind: EntryKind,
}

/// What one entry is, which decides what the field drawing it does with the row.
///
/// Three rather than two, because a repository is the thing one of these fields
/// is looking *for*: the Repos' form marks it and stops there, where every other
/// field treats it as the directory it also is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum EntryKind {
    /// A directory, which is a row that drills in.
    Directory,

    /// Anything else — a file, and whatever else the filesystem holds that is
    /// not a directory. Drawn only by the fields that name files, and a leaf
    /// wherever it is drawn.
    File,

    /// A directory holding a `.git`, which is a repository. A directory in every
    /// other respect: what the mark means is *this is one*, and it is the field
    /// asking that decides whether that makes it a leaf.
    Repository,
}
