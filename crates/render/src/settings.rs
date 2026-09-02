//! What Verkstead has been told — the GitHub token, the git author, the shared
//! Rust build cache and whether a Conversation is shared to its pull request
//! when it settles to Done — as the viewer receives it, and what it sends to
//! change any of them.
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
//! Sharing to the pull request is plainer still: one switch, written and read
//! back as itself. It is the one setting here that is **off** with nothing
//! configured — what it turns on writes to GitHub under the human's own
//! account — so what comes back is where it sits rather than whether anybody
//! has been to the page.
//!
//! And how a conflict is resolved is the plainest of the lot: one of two words,
//! written and read back as itself. What travels with it is the warning the page
//! draws beside the second of them — a rebase is force-pushed, and a
//! force-pushed branch rewrites what reviewers have read.
//!
//! And the paths — the Watched Paths and the Sandbox Configuration binds — are
//! the one thing here said in two places at once. The installation says its own
//! on the command line and the human says theirs in `config.yaml`, and what
//! Verkstead goes by is the union. So each entry comes back saying which of the
//! two said it, and whether the server can see what it names right now: the
//! first is what makes an entry editable here rather than read-only, and the
//! second is a report rather than a refusal — a save lands whatever it was
//! told, and an entry the server cannot see is a row that says so.
//!
//! And the ignore rules are the one thing here a save can be *refused* over: a
//! list of patterns for the comments no agent is ever to be spun up about, and
//! a pattern the regex engine will not take is a rule that would silence
//! nothing while reading as though it silenced something. So they travel as an
//! action rather than as a value — a section that is not about them says
//! nothing about them, and cannot have its own save turned down by a rule
//! somebody hand-edited into the file weeks ago. What comes back names the row
//! and the box, because that is what the page has to draw the error at.

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

    /// And how a conflicted pull request is resolved in every Repo that has not
    /// said otherwise.
    ///
    /// Never null, the way the build cache's switch is never null: nothing
    /// configured is a merge, so what comes back is where the setting sits
    /// rather than whether anybody has been here. A Repo's own override is on
    /// the Repo — see [`crate::RepoView::conflict_resolution`].
    pub conflict_resolution: ConflictResolution,

    /// And whether a Conversation's record is published and linked on its pull
    /// request when the work settles to Done.
    ///
    /// Never null either, and false where nobody has said: this is the one
    /// setting on the page whose unconfigured state is the off one, because
    /// what it turns on writes to GitHub under the human's own account.
    pub share_on_done: bool,

    /// And the Watched Paths and the Sandbox Configuration binds, from both of
    /// the places either of them is said.
    pub paths: PathsView,

    /// And the comments nobody wants addressed, in the order they were written
    /// down — empty on a Verkstead that has been told to ignore nothing, which
    /// is the ordinary condition rather than a setting half made.
    ///
    /// Exactly as the file holds them, a pattern that will not compile
    /// included: this is what the editor draws back into its rows, and a rule
    /// quietly left out of the read would be one the human could not correct.
    pub ignored_comments: Vec<IgnoreRule>,
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
pub enum ConflictResolution {
    /// Merge the base branch into the work branch and push the merge.
    Merge,

    /// Rebase the work branch onto the base branch and force-push what comes
    /// out.
    Rebase,
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

    pub resolution: PathResolution,
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

    pub resolution: PathResolution,
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
pub enum PathResolution {
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

    /// And how a conflicted pull request is resolved where its Repo says
    /// nothing, as a value for the same reason: there are two answers and a save
    /// says which of them this is to be.
    pub conflict_resolution: ConflictResolution,

    /// And whether Done shares the record to the pull request, as a value for
    /// that reason too — a switch has two answers and a save says which of them
    /// this is to be.
    pub share_on_done: bool,

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

    /// And what is to become of the ignore rules, which is an action rather
    /// than a value — the one other field here that is.
    ///
    /// The token's half is an action because it is write-only. This one is
    /// because it is the only setting a save can be *refused* over: a pattern
    /// that will not compile is turned down, and a section that rode the rules
    /// along as values would have the build cache's switch refused over a
    /// pattern somebody hand-edited into the file weeks ago. So a save that is
    /// not about the rules says nothing about them, and the ones on disk are
    /// left exactly where they are.
    pub ignored_comments: IgnoredCommentsEdit,
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
/// One refusal to name, and it is the ignore rules' — see [`RuleRefused`].
/// There is nothing about a name, an address or a token this server declines to
/// write down, and a file it could not write at all is the other failure —
/// which is a status code rather than a named outcome, because it is something
/// to try again rather than something to read.
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

    /// The rules that would not be written down, or empty where the save
    /// landed — which is every save that did not send any.
    ///
    /// A refusal here is the whole request refused: not one rule dropped and
    /// the rest kept, and not the author written while the rules were turned
    /// down. Neither file is touched, so `settings` above is how things stood
    /// before the save as much as after it, and the page has one thing to do
    /// with it — draw the errors at the rows and leave what the human typed
    /// where it is.
    pub refused: Vec<RuleRefused>,
}

/// One class of comment nobody wants an agent addressing.
///
/// Two patterns, either of which may be empty for *no constraint on that part*
/// — strings rather than optionals, and empty for nothing, the way the author's
/// two halves are: the row on the page holds a box either way, and clearing one
/// is how the constraint is taken off.
///
/// Regular expressions in the regex crate's syntax, matched anywhere in their
/// text rather than against the whole of it, and case-sensitive unless the
/// pattern opens with `(?i)`. The author's is matched against the login of
/// whoever wrote the comment and the body's against the markdown as it was
/// written.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct IgnoreRule {
    pub author: String,
    pub body: String,
}

/// What is to become of the ignore rules on a save.
///
/// An action rather than a value, for the reason [`TokenEdit`] is one and not
/// the same reason: nothing about a rule is secret, but the rules are the one
/// thing a save can be refused over, and a section that is not about them
/// should not be able to have its own save turned down by a pattern it never
/// showed anybody.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum IgnoredCommentsEdit {
    /// Leave whatever is written down alone. What every section but the rules'
    /// own sends, and what makes those saves ones that cannot be refused.
    Keep,

    /// Write these in place of whatever is there — the whole list, in the order
    /// it is to be read back in, so a row taken off the page is a rule taken
    /// out of the file. An empty list is the human having removed the last one.
    Set { rules: Vec<IgnoreRule> },
}

/// One rule a save was turned down over, by where it stood in what was sent.
///
/// By position rather than by content, because the row it names is the row the
/// human is looking at: what they typed is still in front of them, and a
/// refusal that described the rule instead would leave the page matching it up
/// against its own boxes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct RuleRefused {
    /// Where it stood among the rules that were sent, counting from zero.
    pub rule: u32,

    /// Which of the two patterns is at fault, or `null` where the rule itself
    /// is — a rule giving neither field is refused as a whole, and there is no
    /// box to draw that at.
    pub field: Option<RuleField>,

    /// Why, in words to put on the row. The regex engine's own for a pattern it
    /// would not take, on one line: what draws this is a small box under a text
    /// field, and the engine's message is a diagram across three or four.
    pub why: String,
}

/// Which of a rule's two patterns something is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum RuleField {
    /// The one matched against who wrote the comment.
    Author,

    /// And the one matched against what it says.
    Body,
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
