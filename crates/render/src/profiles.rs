//! Agent Profiles as the viewer receives them, and what it sends to manage one.
//!
//! A row carries whether the Profile is **broken**, which is a fact about the
//! filesystem now rather than about what was saved: the pair was there when it
//! was checked at save time, and a directory can be moved afterwards. Answered
//! by the server on every read, because the server is the side that can look —
//! and because a session launched under a Profile whose pair has gone is a
//! failure with nobody watching, which is what this whole stage is arranged to
//! move forward in time.
//!
//! Every way of being refused a save is a named outcome rather than a status
//! code, as registering a Repo is: each is a different sentence to put in front
//! of the human, and each names which of the two paths it is about — pointing
//! the config field at a directory is an easy mistake, and "that path is wrong"
//! would not say which one.

use serde::{Deserialize, Serialize};

#[cfg(feature = "typescript")]
use ts_rs::TS;

/// Which coding agent a Profile runs.
///
/// One value. It is on the wire so that a second backend is a variant added
/// beside `Claude` rather than a field the viewer has to start being told about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum AgentType {
    Claude,
}

/// Why a saved Profile cannot be run under as things stand.
///
/// Not a way of being saved: every Profile here passed the same checks when it
/// was written down. This is what has become of its pair since.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum Broken {
    /// The claude directory is not there any more.
    DirMissing,

    /// The config file is not there any more.
    ConfigMissing,

    /// One of the pair now resolves outside every Watched Path — the directory
    /// was replaced by a symlink, or the boundary itself was reconfigured.
    OutsideWatchedPaths,
}

/// One row of the Profile list.
///
/// The paths are the resolved ones the server recorded rather than whatever was
/// typed to save them: those are what will be bind-mounted, so those are what is
/// worth showing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct ProfileEntry {
    pub id: i64,
    pub name: String,
    pub claude_dir: String,
    pub config_file: String,
    pub model: String,
    pub agent_type: AgentType,

    /// `null` while the pair is where it was left, which is the ordinary case.
    pub broken: Option<Broken>,
}

/// A Profile as the human has just written it, for saving or for rewriting.
///
/// No agent type: there is one, and offering a choice of one is theatre. The
/// server records `Claude`, and the field arrives here when there is something
/// to choose between.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct ProfileEdit {
    pub name: String,

    /// The absolute path of the directory bind-mounted over `~/.claude`.
    pub claude_dir: String,

    /// The absolute path of the file bind-mounted over `~/.claude.json`.
    pub config_file: String,

    /// What a session runs on unless it is told otherwise.
    pub model: String,
}

/// What became of saving a Profile.
///
/// The refusals are the server's and not the form's: the Watched Paths are a
/// security boundary, and every request reaching the endpoint is decided there
/// whether or not a form was involved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum ProfileSaved {
    /// Recorded. It is on the list, and it is there after a restart.
    Saved,

    /// There is no Profile with that id to rewrite.
    NoSuchProfile,

    /// It was given no name. A Profile is picked out of a list by its name, so
    /// one without a name is one nobody can choose.
    Nameless,

    /// It was given no model. A session has to know what it runs on.
    Modelless,

    /// Another Profile is called that already.
    NameTaken,

    /// The claude directory was named relatively. There is nothing to resolve it
    /// against that would mean the same thing twice.
    DirNotAbsolute,

    /// Nothing is at the claude directory's path.
    DirMissing,

    /// It resolves to somewhere outside every Watched Path. Checked after `..`
    /// and every symlink have been taken out, so a path that merely reads as
    /// inside one lands here too.
    DirOutsideWatchedPaths,

    /// Something is there and it is not a directory — `~/.claude` is a directory
    /// bind-mounted over, so a file cannot stand in for it.
    NotADirectory,

    /// The config file was named relatively.
    ConfigNotAbsolute,

    /// Nothing is at the config file's path.
    ConfigMissing,

    /// The config file resolves to somewhere outside every Watched Path.
    ConfigOutsideWatchedPaths,

    /// Something is there and it is not a file — the pair is a directory and a
    /// file, and this is the file half.
    NotAFile,
}

/// What became of removing a Profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum ProfileDeleted {
    Removed,
    NoSuchProfile,

    /// A Conversation has chosen it. Taking it away would leave that
    /// Conversation pointing at nothing.
    InUse,
}

/// Which Profile a Conversation is choosing for one of its two roles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct ProfileChoice {
    pub profile_id: i64,
}

/// What became of choosing one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum ProfileChosen {
    Chosen,
    NoSuchConversation,

    /// There is no Profile with that id — it was removed between the list this
    /// page read and the choice it made from it.
    NoSuchProfile,
}
