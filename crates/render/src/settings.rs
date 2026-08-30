//! What Verkstead has been told — the GitHub token, the git author, the shared
//! Rust build cache and where the share viewer is hosted — as the viewer
//! receives it, and what it sends to change any of them.
//!
//! The token goes one way only. What comes back about it is that there is one,
//! its last four characters and when it was saved, and nothing here can be made
//! to hand over the rest: a page that could show a token is a page that puts one
//! in a browser's history, a screenshot and a scroll-back, and there is nothing
//! the human does with the whole of it that seeing the tail will not do. The
//! four characters are what tells one token from another, which is the only
//! question a settings page has to answer about a secret it holds.
//!
//! Saving one asks GitHub who it authenticates as, and the answer rides back
//! with the save rather than being fetched afterwards: the moment a token is
//! pasted is the moment a wrong one is worth saying so about, and the human is
//! looking at the page then. The save itself happens either way — see
//! [`SettingsSaved`].
//!
//! The build cache is the plain half of all this: a switch and a size, both
//! values, both readable back. It is the one thing about a Sandbox here that
//! nobody has to configure — it is on with nothing said, and the switch is the
//! one that takes it away, where the paths below are holes somebody typed. One
//! fact about it travels one way only: whether the server found an sccache to
//! compile through, which is its own environment and nobody's setting.
//!
//! The share viewer's URL is plainer still: one value, written and read back as
//! itself. It is a public page the human hosts a copy of, and every link to a
//! Published Share goes through it — so it is configuration in the ordinary
//! sense, and the page shows it as it stands. An empty one is *no copy of their
//! own* rather than no viewer: links are then composed through the copy
//! Verkstead itself hosts, which is `HOSTED` in `crates/server/src/sharing.rs`.
//!
//! And the paths — the Watched Paths and the Sandbox Configuration binds — are
//! the one thing here said in two places at once. The installation says its own
//! on the command line and the human says theirs in `config.yaml`, and what
//! Verkstead goes by is the union. So each entry comes back saying which of the
//! two said it, and whether the server can see what it names right now: the
//! first is what makes an entry editable here rather than read-only, and the
//! second is a report rather than a refusal — a save lands whatever it was
//! told, and an entry the server cannot see is a row that says so.

use serde::{Deserialize, Serialize};

#[cfg(feature = "typescript")]
use ts_rs::TS;

/// The settings as they stand, read off the two files at the moment they are
/// asked for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct SettingsView {
    pub git_author: Author,

    /// What can be said about the configured token, or `null` where there is
    /// none — which is what a Verkstead nobody has told anything looks like.
    pub github_token: Option<TokenSaved>,

    /// And how the shared Rust build cache stands.
    pub rust_build_cache: BuildCacheView,

    /// Where the human hosts a share viewer of their own, or empty where they
    /// host none.
    ///
    /// A string rather than an optional, empty for nothing configured, the way
    /// the author's two halves are: the field on the page holds it either way,
    /// and clearing the box is how it is taken away.
    ///
    /// Empty is not *no viewer*. Links are then composed through the copy
    /// Verkstead hosts, and this field is the override — which is why nothing
    /// fills it in on the human's behalf: a field holding an address nobody
    /// typed is a setting they cannot tell they have not chosen.
    ///
    /// Configuration rather than a secret — it is a public page, and its URL
    /// goes in a comment on a pull request — so unlike the token it reads back
    /// exactly as it was written.
    pub share_viewer_url: String,

    /// And the Watched Paths and the Sandbox Configuration binds, from both of
    /// the places either of them is said.
    pub paths: PathsView,
}

/// Every path Verkstead has been told about, from both sources at once: the
/// directories it may operate inside, and the extra directories a sandbox is
/// given beyond the surface every one of them has.
///
/// Two lists rather than one, because they are two different permissions — a
/// Watched Path says where the human may point Verkstead, and a bind says what
/// a session may write in — and the page draws them apart for that reason.
///
/// The installation's own entries come first in each list, and the settings'
/// follow in the order they were written down. That is the order the two were
/// decided in: a flag is said once when the machine is set up, and the file is
/// where somebody has been adding to it since.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct PathsView {
    pub watched: Vec<WatchedPathEntry>,

    /// Every configured bind, the ones every sandbox gets and the ones one Repo
    /// does together — see [`BindEntry::repo`], which is what says which of the
    /// two an entry is.
    pub binds: Vec<BindEntry>,
}

/// One Watched Path, whichever of the two places said it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct WatchedPathEntry {
    /// The directory: resolved, for the installation's own, which were resolved
    /// when the server started; and exactly as it was written, for one out of
    /// the settings — that is what a save sends back, so it has to come back as
    /// it went in.
    pub path: String,

    pub source: PathSource,

    pub resolution: Resolution,
}

/// And one Sandbox Configuration bind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct BindEntry {
    /// The directory bound in, read out of the entry — and the whole entry as
    /// it was written, where nothing could be read out of it at all. A row
    /// nobody can see is a row nobody can correct.
    pub path: String,

    /// The Repo this bind is only for, by the name it is registered under, or
    /// `null` for one every sandbox gets.
    pub repo: Option<String>,

    pub source: PathSource,

    pub resolution: Resolution,
}

/// Which of the two places an entry was said in.
///
/// What decides whether the page will let it be edited: the installation's are
/// the unit's word or the command line's and are read-only wherever they are
/// drawn, and the settings' own are the human's to add to and take away.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum PathSource {
    /// A `--watched-path` or a `--sandbox-bind`, from the command line or from
    /// the environment the server was started in.
    Installation,

    /// And one out of `config.yaml`, which is the file this page writes.
    Settings,
}

/// Whether the server can see what an entry names, at the moment it was asked.
///
/// Reported rather than refused: a save lands whatever it was told, so an entry
/// naming a directory nobody has made yet is something to say on the row rather
/// than something to turn a save down over. It is also how a nix install learns
/// that a path added here needs the installer to widen the unit's namespace
/// before it can do anything — the file says it, and the server cannot see it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum Resolution {
    /// The server can see it: a directory, for a Watched Path, and anything at
    /// all for a bind.
    Resolves,

    /// It cannot, and this is why, in the words it is logged in.
    Unresolved { why: String },
}

/// The shared Rust build cache as the settings page draws it: the switch, the
/// size, and the one thing about it the human cannot set.
///
/// The switch is never null. Nothing configured is on, so what comes back is
/// where the switch *sits* rather than whether anybody has touched it — a page
/// that drew a third state would be asking the human to understand a
/// distinction the server does not make.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct BuildCacheView {
    /// Whether sessions get one at all.
    pub enabled: bool,

    /// How big its compiled half may grow, in sccache's own words — `30G`,
    /// `500M`. Always a value: the default is what an untouched setting means,
    /// and the field shows it rather than standing empty.
    pub size: String,

    /// Whether that size is one somebody typed, rather than the default being
    /// shown. What lets the field draw the default as a placeholder — a value
    /// nobody chose should not look like a choice.
    pub size_configured: bool,

    /// Whether the server found an sccache to compile through.
    ///
    /// Read-only, and the one fact here nobody can set from a page: it is the
    /// server's own environment. False means a session's crate downloads are
    /// still shared and its dependencies are compiled every time — which is
    /// what the workbench warns about on a Rust repository, and what installing
    /// sccache on the server fixes.
    pub compiles_cached: bool,
}

/// Who a session's commits are by.
///
/// Two strings rather than two optionals, empty where nothing is configured:
/// the form holds them that way, and half an author is a real state — a name
/// with no address is what git complains about by name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct Author {
    pub name: String,
    pub email: String,
}

/// Everything about the configured token that is not the token.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct TokenSaved {
    /// Its last four characters — fewer, for a token shorter than that, which
    /// is a hand-edit rather than anything GitHub issued.
    pub last_four: String,

    /// When the secrets file was last written, RFC 3339. The file's own
    /// modification time rather than a stamp stored beside the token: the file
    /// is the source of truth, and a hand-edit that moved the token would leave
    /// a stored stamp saying the wrong day.
    pub at: String,
}

/// The settings as the human has just written them.
///
/// The author fields and the token travel together because the page saves as
/// one — and the token's half is an action rather than a value, because most
/// saves are not about the token at all.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct SettingsEdit {
    pub git_author: Author,
    pub github_token: TokenEdit,

    /// The build cache switch and size, as values rather than as an action:
    /// there is nothing secret about either, so a save says where they are to
    /// stand and the server writes that down.
    pub rust_build_cache: BuildCacheEdit,

    /// And where the human hosts a share viewer of their own, as a value for
    /// the same reason: an empty one is nothing configured, which is what
    /// clearing the field means and what puts Verkstead's own hosted copy back.
    pub share_viewer_url: String,

    /// The Watched Paths the settings own, as values again: what is sent is
    /// what `config.yaml` holds afterwards, so a row taken off the page is a
    /// row taken out of the file.
    ///
    /// The installation's own are not here and cannot be sent. They are the
    /// unit's word rather than this page's, and a save leaves them exactly
    /// where they are — see [`PathSource`].
    pub watched_paths: Vec<String>,

    /// And the Sandbox Configuration binds the settings own, in the grammar
    /// `--sandbox-bind` uses: `/abs/path` for a bind every sandbox gets, and
    /// `name=/abs/path` for one the Repo registered under that name gets.
    ///
    /// Strings rather than a shape of their own, because a string is what the
    /// file holds — and one grammar for both of the places a bind is said is
    /// one thing to learn rather than two.
    pub sandbox_binds: Vec<String>,
}

/// The build cache as the human has just set it.
///
/// The size is a string because it is sccache's own word for one, and an empty
/// one is *no size configured* rather than a size of nothing — which is what
/// clearing the field means and what puts the default back.
///
/// Whether an sccache was found is not here. It is the server's own
/// circumstance rather than anything a page can decide, so it travels one way
/// only — see [`BuildCacheView::compiles_cached`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct BuildCacheEdit {
    pub enabled: bool,
    pub size: String,
}

/// What is to become of the configured token.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum TokenEdit {
    /// Leave whatever is there alone. What a save of the author fields alone
    /// sends, and what an empty write-only field means: a page that read a
    /// blank box as *clear this* would take a token away every time somebody
    /// corrected their email address.
    Keep,

    /// Save this one in place of whatever is there.
    Set { token: String },

    /// Take the configured one away.
    Clear,
}

/// What became of a save.
///
/// No refusals to name: there is nothing about a name, an address or a token
/// this server declines to write down, and a file it could not write at all is
/// the one failure — which is a status code, because it is something to try
/// again rather than something to read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct SettingsSaved {
    /// How the settings stand now, read back off the files rather than echoed
    /// from what came in — the files are the source of truth, and a hand-edit
    /// made a moment ago is part of what the page should be showing.
    pub settings: SettingsView,

    /// What GitHub made of the token that was just saved, or `null` where the
    /// save was not about a token.
    pub verified: Option<Verified>,
}

/// Who a token authenticates as, or why nobody could be asked.
///
/// The failure is an answer here rather than a failed save. A token is pasted
/// once, out of a page on GitHub that will not show it again, and a network
/// that was briefly down is no reason to make the human go back for another
/// one: the token is written down, and this says what happened when it was
/// tried.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum Verified {
    /// GitHub says the token is this account's.
    Account {
        login: String,

        /// The scopes Verkstead needs that GitHub says this token has not been
        /// given — empty on one that can do everything asked of it.
        ///
        /// `gist` is the whole of the list, and it is a list because the answer
        /// is *what to go and tick*: publishing a share writes a secret gist,
        /// which is Verkstead's own write to GitHub rather than a session's, and
        /// a token issued for reading repositories does not carry it. The
        /// scopes a *session* needs are not checked here — a session
        /// authenticates as this token too, but what it does with it is the
        /// repository's review process rather than anything this server asks
        /// for.
        ///
        /// Empty as well where GitHub said nothing about scopes at all, which is
        /// what a fine-grained token comes back as: it has permissions rather
        /// than scopes, and reporting the absence of a header as a missing scope
        /// would be sending the human to re-issue a token that works.
        missing: Vec<String>,
    },

    /// GitHub would not say, in `gh`'s own words or Verkstead's about `gh`.
    Refused { why: String },
}
