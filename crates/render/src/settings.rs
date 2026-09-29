//! What Verkstead has been told — the GitHub token, the git author, the
//! languages a session gets build support for and whether a Conversation is
//! shared to its pull request when it settles to Done — as the viewer receives
//! it, and what it sends to change any of them.
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
//! The languages are the plain half of all this: a switch each and, on the one
//! whose store an sccache bounds, a size — all of them values, all of them
//! readable back. They are the one thing about a Sandbox here that nobody has
//! to configure: a language is on with nothing said, and the switch is the one
//! that takes it away, where the paths below are holes somebody typed. What
//! they are drawn from is the descriptors the server loaded, so a language an
//! installer wrote into `config.yaml` has a box here without a line of this
//! crate knowing its name. One fact travels one way only: whether a session's
//! compiling is cached as well as its downloads and, where it is not, why not —
//! which is the server's own environment and its own platform, and nobody's
//! setting.
//!
//! The Cleanup is that shape twice over: two rows, each a switch and a
//! duration, each read back with the flag that says whether the duration is one
//! somebody typed. What is different is that the two rows fall back the
//! two different ways — the trim is on with nothing configured and the delete
//! is off — and that neither is ever refused: a delete sooner than a trim is
//! two independent clocks doing exactly what they were told.
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
//! And the paths — the Sandbox Configuration binds — are the one thing here
//! said in two places at once. The installation says its own on the command
//! line and the human says theirs in `config.yaml`, and what Verkstead goes by
//! is the union. So each entry comes back saying which of the two said it, and
//! whether the server can see what it names right now: the first is what makes
//! an entry editable here rather than read-only, and the second is a report
//! rather than a refusal — a save lands whatever it was told, and an entry the
//! server cannot see is a row that says so.
//!
//! And the ignore rules are the one thing here a save can be *refused* over: a
//! list of patterns for the comments no agent is ever to be spun up about, and
//! a pattern the regex engine will not take is a rule that would silence
//! nothing while reading as though it silenced something. So they travel as an
//! action rather than as a value — a section that is not about them says
//! nothing about them, and cannot have its own save turned down by a rule
//! somebody hand-edited into the file weeks ago. What comes back names the row
//! and the box, because that is what the page has to draw the error at.
//!
//! And the MCP servers are the ignore rules' shape a second time, for the same
//! reason: a name, a URL and the headers each is spoken to with, and the one
//! other thing on this page a save can be *refused* over — a name that is not
//! lowercase letters, digits and hyphens, or one another declaration already
//! has, is a name nothing could refer to a server by. So they travel as an
//! action too, and what comes back names the row and the box the error is drawn
//! at.
//!
//! **And their header values go the way the token goes.** They are what an API
//! key is sent in, static headers being the only authentication a declaration
//! has, so every one of them is a secret: what comes back about a server's
//! headers is their names, and nothing here can be made to hand over a value.
//! Which is why the declarations are the one thing on this page whose two
//! directions are different shapes — [`McpServer`] going out and
//! [`McpServerEdit`] coming in, with the token's three actions said once per
//! header. A value box left blank keeps what is there, so correcting a URL does
//! not take a key away.
//!
//! **And a declaration that is written down is then tried**, the way a token
//! that is written down is: one MCP `initialize` over streamable HTTP, made by
//! this server with the declaration's own headers, after the save has landed
//! and never before it. What comes back is per server and says one of two
//! things — reached, with the name the server gives for itself, or refused,
//! with which of the three ways it went wrong. A report rather than a refusal:
//! the declaration is saved either way, because a server that cannot be reached
//! from here may be reachable from inside a session's network. See
//! [`ServerTried`].
//!
//! What is different is which way a refusal points. A rule that will not compile
//! silences nothing while reading as though it silenced something; a name that
//! is taken would leave a Conversation's chip pointing at either of two servers.
//! Both are worth turning a save down over, and neither is something a section
//! that is not about them should be able to be turned down by.
//!
//! **A declaration is never renamed here.** The name is what everything that
//! refers to a server refers to it by, so what travels back is the list as it is
//! to stand and the page offers a name field on a new row alone — renaming one
//! is deleting it and declaring another.
//!
//! And the instructions are one text, both ways, and nothing else: the words
//! every session is given whatever harness runs it, in place of the global
//! `CLAUDE.md` a Built Root does not carry. A value like the binds — what is
//! sent is what the file holds afterwards — and the one setting here that
//! nothing could refuse, there being no grammar in a paragraph of prose to get
//! wrong. It travels verbatim in both directions, because verbatim is what a
//! harness is handed.

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

    /// And the languages a session is given build support for, one per
    /// descriptor the server loaded — the ones Verkstead ships with whatever
    /// `config.yaml` says merged over them.
    ///
    /// In the order the descriptors were written, which is the order the pane
    /// draws them in: the built-ins first and an installer's own after them.
    /// Never empty, Rust being built into the binary.
    pub languages: Vec<LanguageView>,

    /// And what the Cleanup does to an archived Conversation, and how long
    /// after the archiving it does it.
    pub cleanup: CleanupView,

    /// And how a conflicted pull request is resolved in every Repo that has not
    /// said otherwise.
    ///
    /// Never null, the way a language's switch is never null: nothing
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

    /// And the Sandbox Configuration binds, from both of the places they are said.
    pub paths: PathsView,

    /// And the comments nobody wants addressed, in the order they were written
    /// down — empty on a Verkstead that has been told to ignore nothing, which
    /// is the ordinary condition rather than a setting half made.
    ///
    /// Exactly as the file holds them, a pattern that will not compile
    /// included: this is what the editor draws back into its rows, and a rule
    /// quietly left out of the read would be one the human could not correct.
    pub ignored_comments: Vec<IgnoreRule>,

    /// And the MCP servers declared for this installation, in the order they
    /// were written down — empty on a Verkstead nobody has declared any on,
    /// which is a Conversation with nothing to attach.
    ///
    /// Exactly as the file holds them, a name the page would have refused
    /// included: this is what the section draws back into its rows, and a
    /// declaration quietly left out of the read would be one the human could
    /// neither use nor correct.
    pub mcp_servers: Vec<McpServer>,

    /// And the one text every session is given, whatever harness runs it —
    /// empty on a Verkstead nobody has typed one into, which is a session told
    /// nothing beyond what its Repo carries.
    ///
    /// A string rather than an optional, and empty for nothing, the way the
    /// author's two halves are: the box on the page holds a string either way,
    /// and clearing it is how the text is taken off.
    ///
    /// Verbatim, line breaks and leading spaces and all. What a harness is
    /// handed is these words, so what comes back here has to be the ones that
    /// were typed rather than a tidied copy of them.
    pub instructions: String,
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
/// extra directories a sandbox is given beyond the surface every one of them
/// has.
///
/// The installation's own entries come first, and the settings' follow in the
/// order they were written down. That is the order the two were decided in: a
/// flag is said once when the machine is set up, and the file is where somebody
/// has been adding to it since.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct PathsView {
    /// Every configured bind, which is a directory every sandbox gets.
    pub binds: Vec<BindEntry>,
}

/// And one Sandbox Configuration bind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct BindEntry {
    /// The directory bound in, read out of the entry — and the whole entry as
    /// it was written, where nothing could be read out of it at all. A row
    /// nobody can see is a row nobody can correct.
    pub path: String,

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
    /// A `--sandbox-bind`, from the command line or from the environment the
    /// server was started in.
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
    /// The server can see it, which for a bind is anything at all being there.
    Resolves,

    /// It cannot, and this is why, in the words it is logged in.
    Unresolved { why: String },
}

/// One language as the settings page draws it: what to call it, whether
/// sessions get it, and — for the one whose store an sccache bounds — how big
/// that store may grow.
///
/// The switch is never null. Nothing configured is on, so what comes back is
/// where the switch *sits* rather than whether anybody has touched it — a page
/// that drew a third state would be asking the human to understand a
/// distinction the server does not make.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct LanguageView {
    /// The name `config.yaml` keys it by — `rust` — which is what a save names
    /// it back and what nothing on the page ever shows.
    pub name: String,

    /// And what to call it where somebody reads it: the descriptor's `label`,
    /// or the name above where the file gave none. Always a word to draw, so
    /// the page never has to decide what an unlabelled language is called.
    pub label: String,

    /// Whether sessions get it at all.
    pub enabled: bool,

    /// The store an sccache bounds, for the language whose descriptor names
    /// that capability — null for every other, which is what says there is no
    /// size to draw under its box.
    ///
    /// Hung off the language rather than standing beside the list, because it
    /// is the language's: it is that descriptor's own `size`, and the server
    /// runs one Compile Server sized by whichever switched-on language asks
    /// for it.
    pub compiling: Option<CompilingView>,

    /// And why this language's entry in `config.yaml` was not used, where it
    /// was not — null for the ordinary case, which is every language on a
    /// machine whose file reads.
    ///
    /// Read-only, like the one fact inside `compiling` is: what it reports is
    /// the file rather than a setting, and the fix is in the file. Which is
    /// what makes it the one thing on this page that turns a language's
    /// controls off — see [`UnreadEntry`].
    pub unread: Option<UnreadEntry>,
}

/// What became of a language whose entry in `config.yaml` could not be read.
///
/// **Both halves are drawn**, because they are two different things to do. The
/// reason is what there is to fix — and where a variable is what was refused it
/// names the variable, that being the whole of the fix. What it is running on
/// meanwhile is whether anything is broken right now: a language that fell back
/// to the descriptor Verkstead ships is one whose cache is working exactly as it
/// did before the entry was written, and a language with nothing to fall back to
/// is off until somebody goes and looks.
///
/// And while it is here the page draws that language's controls disabled. The
/// two keys they write go into the entry this reports on, and an entry nothing
/// could read is one nothing can be written into: a box that sprang back the
/// moment it was ticked would be a worse answer than a box that says why it
/// cannot be.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct UnreadEntry {
    /// Why, as a clause the page puts the words *its entry in `config.yaml`* in
    /// front of — `sets PATH, which is a variable the Sandbox sets itself`, or
    /// `could not be read: …` in the reader's own words.
    ///
    /// The server's sentence rather than a code the page turns into one: what
    /// goes wrong in a file somebody hand-wrote is open-ended, and a viewer
    /// holding the vocabulary would be a release that could not add a reason.
    pub why: String,

    /// And what the language is running on meanwhile.
    pub running_on: RunningOn,
}

/// Which of the two happened to a language whose entry would not load.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum RunningOn {
    /// The descriptor Verkstead ships, which is what this language had before
    /// that entry was written. Nothing is broken; the entry is.
    BuiltIn,

    /// Nothing — there is no built-in of that name — so the language is off
    /// until the entry is fixed.
    Nothing,
}

/// How big a language's compiled store may grow, and whether a session's
/// compiling is being cached at all.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct CompilingView {
    /// How big it may grow, in sccache's own words — `30G`, `500M`. Always a
    /// value: the default is what an untouched setting means, and the field
    /// shows it rather than standing empty.
    pub size: String,

    /// Whether that size is one somebody typed, rather than the default being
    /// shown. What lets the field draw the default as a placeholder — a value
    /// nobody chose should not look like a choice.
    pub size_configured: bool,

    /// Whether a session's *compiling* is cached as well as its downloads, and
    /// where it is not, why not.
    ///
    /// Read-only, and the one fact here nobody can set from a page: it is the
    /// server's own environment and its own platform. Anything but
    /// [`CompileCaching::Cached`] means a session's downloads are still shared
    /// and its dependencies are compiled every time, which is a slow build
    /// rather than a broken one.
    pub cached: CompileCaching,
}

/// Whether a session's compiling is cached, and where it is not, what would
/// have to change.
///
/// **It carried a third answer and does not any more.** There used to be a
/// platform where no session compiled through an sccache whatever was
/// installed: a Windows session ran inside an AppContainer, which is refused
/// the loopback an sccache client reaches its server over. That was a thing
/// nobody could fix and a page telling them to install something would have
/// been lying about — so it was its own answer, drawn with its own sentence. A
/// Windows session runs as a local account of Verkstead's own now and reaches
/// the loopback like anything else, so the answer went with the reason for it.
///
/// An enum rather than the boolean two answers come to, because what is being
/// said is *why not* rather than *whether*: a platform that could not reach a
/// compile server would be an answer here again, with its own true reason —
/// see `build_cache::compiles_through_an_sccache` on the server, which is the
/// one place a platform's answer is said.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum CompileCaching {
    /// The server found an sccache, and every session compiles through the one
    /// Compile Server this machine runs.
    Cached,

    /// It found none on its own `PATH`, so a session downloads once for the
    /// machine and compiles for itself. Installing sccache where the server can
    /// see it is the whole of what is missing.
    NoSccache,
}

/// The Cleanup as the settings page draws it: the two things that happen to an
/// archived Conversation, and when.
///
/// Two rows of the same shape and two different answers with nothing
/// configured, which is what the page has to draw: the trim is on at three
/// days, and the delete is off at thirty.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct CleanupView {
    /// The bulk taken out of an archived Conversation: the full agent output,
    /// the Transcripts and the session names, which is everything a Share never
    /// carried.
    pub trim: CleanupStepView,

    /// And the whole of it taken away, which is the one thing here that
    /// forgets.
    pub delete: CleanupStepView,
}

/// One of those two: whether it happens, and how long after the archiving.
///
/// The switch is never null, the way a language's is not: what comes back
/// is where the switch *sits* rather than whether anybody has touched it. The
/// days are always a number for the same reason, with the flag beside them
/// saying whether it is one somebody chose — which is what lets the field draw
/// the default as a placeholder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct CleanupStepView {
    pub enabled: bool,

    /// How many days after the archiving, counted from the archiving itself
    /// rather than from the other step: the two clocks are independent, so a
    /// delete sooner than a trim is a Conversation deleted before it was ever
    /// trimmed rather than anything to put right.
    pub days: u32,

    /// Whether those days are ones somebody typed, rather than the default
    /// being shown.
    pub days_configured: bool,
}

/// And the Cleanup as the human has just set it.
///
/// The days are a string because that is what a form holds, and an empty one is
/// *no duration configured* rather than a duration of nothing — which is what
/// clearing the field means and what puts the default back. So is anything that
/// is not a whole number of days: the page sends what was typed, and nothing
/// here is refused over it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct CleanupEdit {
    pub trim: CleanupStepEdit,
    pub delete: CleanupStepEdit,
}

/// One row of it: the switch, and the days as they were typed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct CleanupStepEdit {
    pub enabled: bool,
    pub days: String,
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

    /// The languages as the page has just left them, as values rather than as
    /// an action: there is nothing secret about a switch or a size, so a save
    /// says where each of them is to stand and the server writes it down.
    ///
    /// One entry per language the page was given, carrying the two keys it
    /// draws and no others — see [`LanguageEdit`]. The rest of a descriptor is
    /// an installer's and is never sent, so a save from this page cannot take a
    /// language off the machine.
    pub languages: Vec<LanguageEdit>,

    /// And what the Cleanup is to do after an archiving, as values for that
    /// reason again: two switches and two durations, and a save says where each
    /// of them is to stand.
    pub cleanup: CleanupEdit,

    /// And how a conflicted pull request is resolved where its Repo says
    /// nothing, as a value for the same reason: there are two answers and a save
    /// says which of them this is to be.
    pub conflict_resolution: ConflictResolution,

    /// And whether Done shares the record to the pull request, as a value for
    /// that reason too — a switch has two answers and a save says which of them
    /// this is to be.
    pub share_on_done: bool,

    /// The Sandbox Configuration binds the settings own, as values again: what
    /// is sent is what `config.yaml` holds afterwards, so a row taken off the
    /// page is a row taken out of the file. In the grammar `--sandbox-bind`
    /// uses, which is `/abs/path` for a bind every sandbox gets.
    ///
    /// Strings rather than a shape of their own, because a string is what the
    /// file holds — and one grammar for both of the places a bind is said is
    /// one thing to learn rather than two.
    ///
    /// The installation's own are not here and cannot be sent. They are the
    /// unit's word rather than this page's, and a save leaves them exactly
    /// where they are — see [`PathSource`].
    pub sandbox_binds: Vec<String>,

    /// And what is to become of the ignore rules, which is an action rather
    /// than a value — the one other field here that is.
    ///
    /// The token's half is an action because it is write-only. This one is
    /// because it is the only setting a save can be *refused* over: a pattern
    /// that will not compile is turned down, and a section that rode the rules
    /// along as values would have a language's switch refused over a
    /// pattern somebody hand-edited into the file weeks ago. So a save that is
    /// not about the rules says nothing about them, and the ones on disk are
    /// left exactly where they are.
    pub ignored_comments: IgnoredCommentsEdit,

    /// And what is to become of the declared MCP servers, which is an action for
    /// the reason the rules above it are one: they are the other thing here a
    /// save can be refused over, and a section that rode them along as values
    /// could have its own save turned down by a name somebody hand-edited into
    /// the file weeks ago.
    ///
    /// The whole list where it is sent, in the order it is to be read back in:
    /// a row taken off the page is a declaration taken out of the file, and a
    /// row whose URL was rewritten is that declaration with the new one.
    pub mcp_servers: McpServersEdit,

    /// And the text every session is given, as a value again — the plainest one
    /// here. What is sent is what `config.yaml` holds afterwards, so a box
    /// cleared on the page is the key taken out of the file.
    ///
    /// Nothing about it can be turned down. It is a paragraph of somebody's
    /// prose for an agent to read: there is no grammar to get wrong, nothing to
    /// compile and nobody to ask about it, so a save carrying it cannot fail
    /// the way one carrying a rule can.
    pub instructions: String,
}

/// One language as the human has just left it: the two keys of its entry the
/// settings page writes, and nothing else.
///
/// The size is a string because it is sccache's own word for one, and an empty
/// one is *no size configured* rather than a size of nothing — which is what
/// clearing the field means and what puts the default back. A language with no
/// size field of its own sends the empty string, which is the same nothing.
///
/// Whether compiling is cached is not here. It is the server's own
/// circumstance rather than anything a page can decide, so it travels one way
/// only — see [`CompilingView::cached`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct LanguageEdit {
    /// Which language this is about, by the name `config.yaml` keys it by — a
    /// name the server has never heard of is an entry written into that file
    /// under it, which is how an installer's own language is saved.
    pub name: String,

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

    /// And the declarations that would not be written down, or empty where the
    /// save landed — which is every save that did not send any.
    ///
    /// Its own list rather than the one above, because the two name different
    /// things: a rule is refused by where it stood among the rules, and a
    /// declaration by where it stood among the declarations. A save turned down
    /// over either is the whole request refused and neither file touched, so
    /// what this says and what `refused` says are both drawn over what the human
    /// still has in front of them.
    pub refused_servers: Vec<ServerRefused>,

    /// And what came of speaking to each declaration that *was* written down,
    /// in the order they were declared — empty on every save that said nothing
    /// about them, and on one that was turned down, nothing having been written
    /// to speak to.
    ///
    /// By name rather than by position, unlike the two lists above: this is
    /// only ever about declarations that landed, so each of them has a name
    /// that is a name and no two share one — see [`McpServer`], where the name
    /// is the identity.
    pub tried: Vec<ServerTried>,
}

/// What came of trying one declared server as it was saved.
///
/// The token's [`Verified`] said about a server, and for the reason that is
/// carried back with a save rather than fetched afterwards: the moment a URL is
/// typed is the moment a wrong one is worth saying something about, and the
/// human is looking at the page then.
///
/// **It is saved either way.** The outcome is told rather than enforced — a
/// server that cannot be reached today is still declared, because it may be
/// reachable tomorrow or only from inside a session's network. And it is tried
/// here and nowhere else: an unreachable server never holds a launch, which is
/// ADR-0021's.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct ServerTried {
    /// The declaration this is about, by the name it was saved under.
    pub server: String,

    /// And what came of it.
    pub outcome: Tried,
}

/// Whether a declaration answered, and what it called itself or why it did not.
///
/// **Nothing the server sent back is quoted in a refusal.** What it is spoken
/// to with are the header values, which are secrets — see [`McpHeader`] — and a
/// service that echoed one into an error message would otherwise put it on the
/// page. So a refusal is Verkstead's own words about which of the three ways it
/// went wrong, and the one thing carried over from the server itself is the
/// name it gives for itself, which is what `initialize` is asked for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum Tried {
    /// It answered `initialize`, which is the whole of what being reachable
    /// means here.
    Reached {
        /// The name it gives for itself, where it gives one — `serverInfo.name`
        /// in what it answered. `null` on a server that named itself nothing,
        /// which is one that is reachable and says so in fewer words.
        named: Option<String>,
    },

    /// Or it did not, in words to put on the row: it did not answer, it
    /// answered with a status that means the headers were not accepted, or what
    /// answered was not an MCP server.
    Refused { why: String },
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

/// One MCP server declared for this installation, as the page is told about it:
/// a name, the URL it is reached at, and the names of the headers it is spoken
/// to with.
///
/// **The names of the headers, and no part of a value.** Every header value is
/// a secret and goes the way the GitHub token goes — written where a secret is
/// written and never returned. What the page has to draw is which headers a
/// server has, so that one can be kept, rewritten or taken away; what it does
/// with a value is send a new one. So this is not the shape a save sends, unlike
/// [`IgnoreRule`], which is the same both ways: see [`McpServerEdit`].
///
/// **The name is the identity** — lowercase letters, digits and hyphens, unique
/// among the declarations, and never changed. It is what a Conversation's chip
/// refers to and what the agent sees in front of the server's tool names, so a
/// renamed declaration would be one every chip pointing at it had lost.
///
/// **HTTP only.** There is no command, no arguments and no transport to choose:
/// a stdio server is a child process an agent starts inside its own sandbox, and
/// it was turned down in the grilling this was settled in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct McpServer {
    pub name: String,
    pub url: String,

    /// The headers it is spoken to with, in the order they are sent. Empty is a
    /// server that wants none.
    pub headers: Vec<McpHeader>,
}

/// One header of a declaration, as the page is told about it: its name, and
/// whether there is a value kept to send in it.
///
/// **Whether, and nothing more.** That is exactly what the token's own view
/// gives — see [`TokenSaved`] — and it is what the page has to know to draw the
/// box: a header with a value kept says so and offers to replace or clear it, a
/// header declared and never given one says that instead. Neither says what the
/// value is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct McpHeader {
    pub name: String,

    /// Whether anything is kept to send in it. False is a header the
    /// declaration names with nothing behind it — which is a header nothing is
    /// sent in, rather than one sent empty.
    pub set: bool,
}

/// And one as a save sends it: the same two halves, and an action per header
/// rather than a value.
///
/// Its own shape because a value never comes back. What the page was shown is
/// the header names — see [`McpServer`] — so what it can say about a value is
/// what is to *become* of it, which is the token's three actions said once per
/// header.
///
/// The whole list of headers, in the order they are to be read back in: a header
/// taken off the row is one the declaration no longer names, and one added is a
/// name with a value to set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct McpServerEdit {
    pub name: String,
    pub url: String,
    pub headers: Vec<McpHeaderEdit>,
}

/// One header on a declaration a save is sending: its name, and what is to
/// become of the value sent in it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct McpHeaderEdit {
    pub name: String,
    pub value: HeaderEdit,
}

/// What is to become of one header's value.
///
/// [`TokenEdit`]'s three actions, once per header and for the same reason: the
/// value is write-only, so a blank box is the human not touching it rather than
/// the human emptying it. Correcting a URL leaves every key where it is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum HeaderEdit {
    /// Leave whatever is kept for this header alone. What an untouched value box
    /// sends, and what every header of a server somebody only renamed a URL on
    /// sends.
    Keep,

    /// Send this in it from now on.
    Set { value: String },

    /// And take away what is kept, leaving the header declared with nothing to
    /// send in it — which is how a key is withdrawn without the header being
    /// taken off the declaration.
    Clear,
}

/// What is to become of the declared MCP servers on a save.
///
/// An action for the reason [`IgnoredCommentsEdit`] is one: they are the other
/// thing on this page a save can be refused over, so a section that is not about
/// them says nothing about them and cannot be turned down by one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum McpServersEdit {
    /// Leave whatever is declared alone. What every section but the servers'
    /// own sends.
    Keep,

    /// Write these in place of whatever is there — the whole list, in the order
    /// it is to be read back in, so a row taken off the page is a declaration
    /// taken out of the file. An empty list is the human having removed the
    /// last one.
    Set { servers: Vec<McpServerEdit> },
}

/// One declaration a save was turned down over, by where it stood in what was
/// sent.
///
/// By position rather than by name, for the reason [`RuleRefused`] is by
/// position: the row it names is the row the human is looking at, and one of the
/// two things that can be wrong with a name is that it is the same as another's.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct ServerRefused {
    /// Where it stood among the declarations that were sent, counting from zero.
    pub server: u32,

    /// Which of its two boxes the error is drawn at. Never absent, unlike a
    /// rule's: every way a declaration goes wrong is a way one of its two halves
    /// does, so there is always a box to say it at.
    pub field: ServerField,

    /// Why, in words to put on the row.
    pub why: String,
}

/// Which of a declaration's two halves something is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum ServerField {
    /// The name a Conversation refers to the server by.
    Name,

    /// And the URL it is reached at.
    Url,
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
