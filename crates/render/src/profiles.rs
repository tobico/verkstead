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

/// The account a Profile names, in the shape the agent type running it keeps
/// one.
///
/// Flat on the wire — `{"agent_type": "Claude", "claude_dir": "…",
/// "config_file": "…"}` — so the type is a field the viewer can read and narrow
/// on rather than a name it has to unwrap the account out of. Which is what the
/// form draws its fields off: a shape per type, and adding a backend adds a
/// variant here and the fields beside it.
///
/// One shape for both directions. A Profile as the viewer receives it carries
/// the resolved paths the server recorded and a Profile as the human has just
/// written it carries what they typed, but they are the same fields either way,
/// and two types for one shape would be two opinions about what an account is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "agent_type")]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum ProfileAccount {
    /// Claude Code's pair: what is bind-mounted over `~/.claude` and
    /// `~/.claude.json` inside the sandbox.
    Claude {
        claude_dir: String,
        config_file: String,
    },

    /// Codex's one home: what is bind-mounted over `~/.codex`, and the whole of
    /// what a Codex Profile names.
    Codex { home: String },

    /// And Grok Build's, bind-mounted over `~/.grok`: the same one-home shape
    /// Codex's is, under the directory grok keeps an account in.
    Grok { home: String },

    /// And OpenCode's, which is one home again but not one mount: opencode
    /// keeps no dot-directory of its own, so what this names is a home holding
    /// `.config/opencode` and `.local/share/opencode`, each bound where
    /// opencode's XDG defaults look for it inside the sandbox.
    OpenCode { home: String },
}

/// Why a saved Profile cannot be run under as things stand.
///
/// Not a way of being saved: every Profile here passed the same checks when it
/// was written down. This is what has become of its account since — the pair
/// for a Claude Profile, and the one home for every type that keeps one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum Broken {
    /// The claude directory is not there any more.
    DirMissing,

    /// The config file is not there any more.
    ConfigMissing,

    /// The home the account was kept under is not there any more.
    HomeMissing,

    /// The account now resolves outside every Watched Path — a directory was
    /// replaced by a symlink, or the boundary itself was reconfigured.
    OutsideWatchedPaths,
}

/// One row of the Profile list.
///
/// The account's paths are the resolved ones the server recorded rather than
/// whatever was typed to save them: those are what will be bind-mounted, so
/// those are what is worth showing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct ProfileEntry {
    pub id: i64,
    pub name: String,

    /// Which agent this Profile runs, and the account it runs as — one field,
    /// because the type is what says which fields the account has.
    pub account: ProfileAccount,

    /// Every model this account can run a session on. At least one, and none of
    /// them preferred over the others: the list says what is available and
    /// nothing more.
    pub models: Vec<String>,

    /// `null` while the account is where it was left, which is the ordinary
    /// case.
    pub broken: Option<Broken>,
}

/// A Profile as the human has just written it, for saving or for rewriting.
///
/// The account says which type it is, because the fields beside it are that
/// type's. The form picks one from the types whose stage has landed — a type
/// that cannot launch the real binary yet would be a lie in a picker — so a
/// type this knows about may still be one nothing arrives as.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct ProfileEdit {
    pub name: String,

    /// The absolute paths this Profile's account is, in its type's shape.
    pub account: ProfileAccount,

    /// The models this account can run a session on, in the order they were
    /// typed. The form takes them a line apiece; blank lines and repeated
    /// whitespace are the server's to drop, and a list that comes to nothing is
    /// refused.
    pub models: Vec<String>,
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

    /// It was given no models. A session has to know what it runs on, and a
    /// Profile naming none is one nothing could be launched under.
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

    /// The home was named relatively. There is nothing to resolve it against
    /// that would mean the same thing twice.
    HomeNotAbsolute,

    /// Nothing is at the home's path.
    HomeMissing,

    /// The home resolves to somewhere outside every Watched Path.
    HomeOutsideWatchedPaths,

    /// Something is there and it is not a directory — a home is a directory
    /// bind-mounted over, so a file cannot stand in for it.
    HomeNotADirectory,
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

/// One of a Conversation's Pairings, as the page shows it: the Profile
/// whole, and the model paired with it.
///
/// The Profile whole rather than by id because the pane says what it is and
/// whether it is still runnable — and the model beside it because a Pairing is
/// both halves, and either half alone is not something to launch a session
/// with.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct PairingView {
    pub profile: ProfileEntry,

    /// The model of that Profile's list this Conversation's sessions run on.
    ///
    /// `null` is a Profile chosen before pairings existed, which is not a
    /// Pairing: the page draws it as nothing chosen, because while the
    /// Conversation is drafting that is a choice to make again. One past
    /// drafting keeps running on the model its Profile carried, which nothing
    /// here has to say — its Pairings are fixed and there is no picking left.
    pub model: Option<String>,
}

/// What a Conversation has settled about one of its roles, as the page shows
/// it: the Pairing its sessions run under, that the role runs none, or nothing
/// picked yet.
///
/// Three rather than a nullable Pairing, because a picker offers *no grilling*
/// or *no review* as a row of its own: a Conversation that picked one is as
/// ready to start as one that picked a Pairing, and a page that could not tell
/// it from an empty picker would draw the placeholder over a settled choice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum PickedView {
    /// Nothing picked — which includes a Profile chosen before pairings
    /// existed, that being half a choice and so a choice to make again.
    Nothing,

    /// The row that runs no session at all.
    Skipped,

    /// The Profile and model this role's sessions run under.
    Under(PairingView),
}

impl PickedView {
    /// The Pairing where one was picked, for the readers that want the account
    /// rather than which of the three this is.
    pub fn pairing(&self) -> Option<&PairingView> {
        match self {
            Self::Under(pairing) => Some(pairing),
            _ => None,
        }
    }

    /// Whether the human picked the row that runs no session, which is what the
    /// presses that behave differently for it turn on.
    pub fn skipped(&self) -> bool {
        matches!(self, Self::Skipped)
    }
}

/// Which Pairing one of a Conversation's roles runs under, or that it runs none.
///
/// `null` is the row that runs no session: a picker that offers one offers it
/// beside the Pairings, so the one press that picks either sends the same body
/// — see [`PickedView`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct RoleChoice {
    pub pairing: Option<ProfileChoice>,
}

/// Which Profile and model a Conversation is pairing for one of its roles.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct ProfileChoice {
    pub profile_id: i64,

    /// One of that Profile's models. Never absent: there is no default model
    /// anywhere, so a Pairing is picked whole or not at all.
    pub model: String,
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

    /// That Profile does not list that model, for the same reason: its list was
    /// edited between the read and the pick.
    NoSuchModel,

    /// The Conversation is past drafting, so both its Pairings are fixed.
    NotDrafting,
}
