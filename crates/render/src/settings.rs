//! What Verkstead has been told — the GitHub token and the git author — as the
//! viewer receives it, and what it sends to change either.
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
    Account { login: String },

    /// GitHub would not say, in `gh`'s own words or Verkstead's about `gh`.
    Refused { why: String },
}
