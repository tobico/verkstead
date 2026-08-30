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
//! values, both readable back. It is here because it is the one thing about a
//! Sandbox the human sets rather than the installer, and one fact about it
//! travels one way only — whether the server found an sccache to compile
//! through, which is its own environment and nobody's setting.
//!
//! The share viewer's URL is plainer still: one value, written and read back as
//! itself. It is a public page the human hosts a copy of, and every link to a
//! Published Share goes through it — so it is configuration in the ordinary
//! sense, and the page shows it as it stands. An empty one is *no copy of their
//! own* rather than no viewer: links are then composed through the copy
//! Verkstead itself hosts, which is `HOSTED` in `crates/server/src/sharing.rs`.
//!
//! And how a conflict is resolved is the plainest of the lot: one of two words,
//! written and read back as itself. What travels with it is the warning the page
//! draws beside the second of them — a rebase is force-pushed, and a
//! force-pushed branch rewrites what reviewers have read.

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

    /// And how a conflicted pull request is resolved in every Repo that has not
    /// said otherwise.
    ///
    /// Never null, the way the build cache's switch is never null: nothing
    /// configured is a merge, so what comes back is where the setting sits
    /// rather than whether anybody has been here. A Repo's own override is on
    /// the Repo — see [`crate::RepoView::conflict_resolution`].
    pub conflict_resolution: Resolution,
}

/// How a merge conflict between a pull request and its base branch is resolved.
///
/// Two words for two ways of putting the base's work on a branch that has
/// diverged from it, and what tells them apart is what happens to the commits
/// already pushed: a merge leaves every one of them where it is, and a rebase
/// writes them again and has to be force-pushed — which rewrites what reviewers
/// have read and breaks anything stacked on the branch.
///
/// Which is why the page says so beside the choice rather than leaving it to be
/// found later, and why merge is what nobody choosing anything gets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum Resolution {
    /// Merge the base branch into the work branch and push the merge.
    Merge,

    /// Rebase the work branch onto the base branch and force-push what comes
    /// out.
    Rebase,
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

    /// And how a conflicted pull request is resolved where its Repo says
    /// nothing, as a value for the same reason: there are two answers and a save
    /// says which of them this is to be.
    pub conflict_resolution: Resolution,
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
