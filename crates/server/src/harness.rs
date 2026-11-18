//! Every harness a session can be run on, reached through one trait
//! (ADR-0030, *Agent*).
//!
//! **What a harness is, rather than what Verkstead does with it.** Each method
//! of [`Harness`] answers one question the server used to ask by matching on
//! [`store::AgentType`]: which program, which line, what its Screen says while
//! it works, which sentences say its account is spent or signed out, how its
//! sessions ask, what it calls an MCP server's tools, and what it is told in its
//! environment. Most of the answers are data rather than behaviour, so that a
//! harness described by a plugin's manifest can give them as readily as one
//! written here.
//!
//! **Five implementations, the built-ins**, a module apiece: [`claude`],
//! [`codex`], [`grok`], [`opencode`] and [`agy`]. Each holds everything
//! about its harness that used to be an arm of somebody else's `match`, and the
//! wording in each is the harness's own and will move — so it is kept in one
//! place, and moving it costs one edit there. [`of`] is the one place a type is
//! turned into its harness.

use std::path::{Path, PathBuf};

use crate::platform::Platform;
use crate::store;

mod agy;
mod claude;
mod codex;
mod grok;
mod opencode;

/// One harness: everything about it the server needs to launch a session on
/// it and to watch that session run.
pub trait Harness: Send + Sync {
    /// The program a session is run as.
    ///
    /// The host provides them, as it provides `claude` (ADR-0011): the installer
    /// puts each harness's binary on the system profile the sandbox already
    /// reads, and a Profile whose binary is missing fails at session start,
    /// named in the Capture of the session that could not run.
    fn binary(&self) -> &str;

    /// What the harness is called where a sentence names it: *Claude Code*,
    /// *Grok Build*.
    fn named(&self) -> &str;

    /// Its launch line, for a session working in `worktree` beside the
    /// Worktrees of its Companion Repos, `companions`.
    ///
    /// **Every backend is launched with its approval bypass on.** Running
    /// unattended is what Verkstead promises rather than something the account's
    /// own configuration is trusted to have been left holding: a session that
    /// stopped to ask for approval would be asking it in front of nobody, with the
    /// whole backlog behind it waiting on an answer that is not coming. What stops
    /// a session doing harm is the Sandbox, which this does not touch and which is
    /// still the boundary — and codex's own sandbox is off rather than nested,
    /// because it will not start inside bwrap and bwrap is already the boundary.
    ///
    /// Codex and grok both draw a full-screen TUI, and `--no-alt-screen` keeps each
    /// drawing inline instead: the Capture and the Screen are the record of what a
    /// session did, and an alternate screen is a record that is thrown away as the
    /// program leaves it.
    fn line(&self, worktree: Option<&Path>, companions: &[PathBuf]) -> Line;

    /// What the harness has on its Screen while it is working, and nothing of
    /// what it has there when it is waiting for a human — or `None` where its
    /// idle is the silence itself. See [`crate::sessions::Signature::AtWork`].
    ///
    /// Every harness that draws a screen has been measured and every one of
    /// them came out this way round, so this is the one reading a harness
    /// gives. What puts a signature that has drifted in front of the human,
    /// rather than leaving a session nothing ever catches, is the long-stop
    /// behind it.
    fn at_work(&self) -> Option<&str>;

    /// The sentence a session prints when its account is out of window — or
    /// nothing, where no such wording has ever been seen. See
    /// [`crate::limits`], which says why nothing is an answer rather than a
    /// gap.
    fn exhausted(&self) -> Option<&str>;

    /// What a session says when its account has signed out — or nothing, where
    /// Verkstead cannot run the harness's login. See [`crate::signouts`].
    fn signed_out(&self) -> Option<SignedOut<'_>>;

    /// How a session puts a Question Set to the human.
    ///
    /// A fact about the harness rather than about the Set (ADR-0011): the CLI
    /// asks the same way everywhere, and what differs is whether the harness
    /// can afford to hold a shell command open for hours. Answered here so that
    /// the Guide a session reads and the server that takes its Set answer the
    /// question the same way.
    fn channel(&self) -> store::Channel;

    /// What the harness puts in front of an MCP server's tool names, with
    /// `<server>` and `<tool>` where the two names go — see
    /// [`crate::skills::served`].
    ///
    /// Each read off a real run against a server that recorded what it was
    /// asked, rather than off anybody's documentation.
    fn tools_named(&self) -> &str;

    /// What a session of this harness on `platform` is told about itself in
    /// its environment, beyond what every session is told — see
    /// [`crate::sandbox`], where the names are and where they are set.
    fn environment(&self, platform: Platform) -> Vec<(&str, &str)>;

    /// What the harness writes inside the Worktree it runs in, as the line of
    /// `info/exclude` that keeps it out of git — see
    /// [`crate::worktrees::excluded`].
    fn written_in_worktree(&self) -> Option<&str>;

    /// Whether the file `at`, where this harness's binary resolved on
    /// `platform`, is a desktop app standing where the harness should be
    /// rather than the harness itself — see [`crate::sandbox::Standing`].
    ///
    /// Claude Code's alone: no other harness has been seen to have one.
    fn desktop_app(&self, _platform: Platform, _at: &Path) -> bool {
        false
    }
}

/// The harness a Profile of `agent_type` runs.
///
/// The one place a type is turned into its harness, and the one `match` on it
/// this module has.
pub fn of(agent_type: store::AgentType) -> &'static dyn Harness {
    match agent_type {
        store::AgentType::Claude => &claude::Claude,
        store::AgentType::Codex => &codex::Codex,
        store::AgentType::Grok => &grok::Grok,
        store::AgentType::OpenCode => &opencode::OpenCode,
        store::AgentType::Agy => &agy::Agy,
    }
}

/// Every harness built into this binary.
pub(crate) static BUILT_IN: [&dyn Harness; 5] = [
    &claude::Claude,
    &codex::Codex,
    &grok::Grok,
    &opencode::OpenCode,
    &agy::Agy,
];

/// The programs those are run as, in that order, for the one list of names a
/// session is followed into — see [`crate::sandbox::PROGRAMS`].
pub(crate) const BINARIES: [&str; 5] = [
    claude::BINARY,
    codex::BINARY,
    grok::BINARY,
    opencode::BINARY,
    agy::BINARY,
];

/// What a session says when its account has signed out — see
/// [`Harness::signed_out`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SignedOut<'a> {
    /// The sentence a line ends with.
    pub(crate) ends: &'a str,

    /// The line that has to be the next one drawn after it, blank lines
    /// aside, where the sentence alone is also said by a session signed in.
    pub(crate) then: Option<&'a str>,

    /// Which of the session's records it is read in.
    pub(crate) read_in: ReadIn,
}

/// Which of the three records a session leaves a sign-out is read in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadIn {
    /// What it printed, the frame it drew, and its backend's own log.
    Every,

    /// Its backend's own log, and nothing it puts on the terminal — for a
    /// harness that draws its conversation so far again on the screen, where a
    /// sign-out it stopped on before is a line drawn rather than said.
    Log,
}

/// The rest of a backend's launch line: how it is told its model, how it is
/// given the Brief, whether it takes the session id Verkstead named, and what
/// goes on the end.
///
/// One value rather than a mapping apiece, because they are one fact — what
/// this harness's command line looks like — and a harness answers it once, in
/// [`Harness::line`].
pub struct Line {
    /// What the model is said with. `--model` reads as claude's own; codex
    /// takes `-m`.
    pub(crate) model: String,

    /// The flag the Brief goes under, or `None` where it is the one positional
    /// argument.
    ///
    /// The positional is what every backend up to this one takes, and opencode
    /// is the first that does not: its positional is the project to start in,
    /// and the prompt is `--prompt`. Said here rather than assumed, so that a
    /// harness taking either shape is one answer of [`Harness::line`] rather
    /// than a branch in the builder.
    pub(crate) prompt: Option<String>,

    /// Whether the session Verkstead named is named on the line.
    ///
    /// False for a backend that takes no session id at all, whose log is
    /// therefore found rather than named — see [`crate::transcript`].
    pub(crate) names_the_session: bool,

    /// And the other line this same backend has: the one that **continues** a
    /// conversation instead of opening one, under the id of the session that was
    /// having it.
    ///
    /// Mutually exclusive with [`Line::names_the_session`] wherever both could
    /// be said, and that is a fact about what a session is rather than a
    /// convention: a conversation carried on keeps the id it already had, so
    /// there is nothing left to name it.
    ///
    /// And it is not always a flag — see [`Resume`]: codex's is a subcommand, and
    /// what a subcommand changes is the shape of the whole line rather than what
    /// is on the end of it.
    ///
    /// `None` where Verkstead does not carry a conversation on from a session of
    /// this backend, which is now no backend at all: all five have a resume, and
    /// what separated them was never that but whether Verkstead knew the id to
    /// resume by and whether their store had to be put right first. It stays an
    /// `Option` because a sixth backend lands here before either of those is
    /// answered for it, and until they are a launch under it opens a session of
    /// its own, which is Verkstead's own Resume and nothing lost.
    pub(crate) resume: Option<Resume>,

    /// The flags and configuration overrides that go last, after the prompt.
    ///
    /// Owned rather than borrowed, because the trust pre-seed below names the
    /// Worktree this session is being launched in.
    pub(crate) tail: Vec<String>,
}

/// How a backend is told to carry a conversation on rather than open one — see
/// [`Line::resume`].
///
/// **Two shapes rather than one spelling**, because the backends do not agree on
/// what a resume *is*. For three of them it is another option on the same line,
/// and the id goes where the id would have gone. For codex it is a subcommand:
/// the line is `codex resume <session-id> <prompt>`, so the word comes ahead of
/// everything the ordinary line says and the id is a positional in front of the
/// prompt rather than a value after a flag.
///
/// What does *not* change with the shape is the rest of the line. A resumed
/// session is the same session under the same account in the same Worktree, so the
/// model, the approval bypass and every configuration override the ordinary line
/// carries come across onto the subcommand — a resume that dropped them would come
/// up logged out and sitting on a trust prompt.
pub enum Resume {
    /// A flag and the id, written where the session id it is mutually exclusive
    /// with would have gone. Claude's and Grok Build's, both spelled `--resume`,
    /// and OpenCode's, spelled `--session`.
    Flag(String),

    /// A subcommand and the id, written in front of the line rather than on the
    /// end of it. Codex's.
    Subcommand {
        /// The word itself, straight after the binary.
        word: String,

        /// And what the continuation needs said that the ordinary line does not,
        /// which goes on the end with the tail.
        ///
        /// Here rather than in [`Line::tail`] because it is about resuming: a
        /// launch opening a conversation would be carrying a setting for a
        /// question it is not asking.
        also: Vec<String>,
    },
}
