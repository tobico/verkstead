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
//! **And its account, the other half.** What an account of it is made of and
//! where each path is joined, what a Profile is judged broken by, which file
//! holds the login and whether Verkstead runs that login, what a Built Root
//! holds of the account — its configuration composed out of the files as found
//! with the attached MCP servers, its instructions file, its memory store, and
//! what of that store is synced and mirrored — and where the record of a
//! session is found. What is done with those answers stays the host's: the
//! mounting, the strip-and-never-write-back of ADR-0011 and the merging back
//! are [`crate::sandbox`]'s, whichever harness it is.
//!
//! **Five implementations, the built-ins**, a module apiece: [`claude`],
//! [`codex`], [`grok`], [`opencode`] and [`agy`]. Each holds everything
//! about its harness that used to be an arm of somebody else's `match`, and the
//! wording in each is the harness's own and will move — so it is kept in one
//! place, and moving it costs one edit there. **And a sixth**, [`plugged`]:
//! a harness a plugin declares, answering out of its manifest and its
//! component, which is the only one a third-party harness is. [`of`] is the
//! one place a type is turned into its harness.

use std::collections::BTreeMap;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::RwLock;
use std::time::SystemTime;

use crate::logins::Speaking;
use crate::platform::Platform;
use crate::sandbox::Home;
use crate::sandbox::root::{Part, Place};
use crate::settings::AttachedServer;
use crate::store;
use crate::transcript::{Carried, Search};

mod agy;
mod claude;
mod codex;
mod grok;
mod opencode;
mod plugged;

/// One harness: everything about it the server needs to launch a session on
/// it and to watch that session run, and what it keeps of an account.
pub(crate) trait Harness: Send + Sync {
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

    /// Its mark, as an SVG document the viewer draws in front of its name.
    ///
    /// **The built-ins' are lobehub's**:
    /// [lobehub/lobe-icons](https://github.com/lobehub/lobe-icons), MIT
    /// licensed, copied out of `@lobehub/icons-static-svg` into `marks/` beside
    /// this module rather than depended on, under lobehub's own names — so
    /// updating one is copying it again — with its licence beside them, MIT
    /// asking for the notice to travel with the copy. Verbatim but for one
    /// edit: each file's `<title>` is taken out, being what a browser draws as
    /// a tooltip, and a mark whose tooltip said "Claude" beside words already
    /// reading *Claude Code Fable 5 — Work* would be the line saying itself
    /// twice.
    ///
    /// Four of the five are the mono cut, painted in `currentColor` so a mark
    /// sits in whatever ink surrounds it. Claude Code's is lobehub's colour
    /// variant, which names its fill and so stands in its own orange. The
    /// Antigravity CLI's is the mono cut even so: lobehub's colour one is a
    /// stack of masked, blurred gradients, which at `1em` is a smudge.
    fn mark(&self) -> &str;

    /// The paths an account of it is made of, in the order it declares them:
    /// what the Profile form draws a field apiece for, and what a save judges
    /// — see [`crate::profiles`]. Every key is one [`store::Account`] holds a
    /// path under.
    fn paths(&self) -> Vec<Declared<'_>>;

    /// Where an account of it is kept in a home of the human's, said from
    /// `~` as they would recognise it — what onboarding draws under an account
    /// it found.
    fn found_at(&self) -> Vec<&str>;

    /// What to run in a terminal to make an account of it: the program
    /// itself, once, where its first start is its login.
    fn first_run(&self) -> &str {
        self.binary()
    }

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

    /// What the harness writes inside the Worktree it runs in, each as the
    /// line of `info/exclude` that keeps it out of git — see
    /// [`crate::worktrees::excluded`]. Nothing, for most.
    fn written_in_worktree(&self) -> Vec<&str> {
        Vec::new()
    }

    /// Whether the file `at`, where this harness's binary resolved on
    /// `platform`, is a desktop app standing where the harness should be
    /// rather than the harness itself — see [`crate::sandbox::Standing`].
    ///
    /// Claude Code's alone: no other harness has been seen to have one.
    fn desktop_app(&self, _platform: Platform, _at: &Path) -> bool {
        false
    }

    // The account half: what an account of this harness is made of, and what a
    // session is given of it — see [`crate::sandbox::root`].

    /// The account this harness keeps directly in `home`, rooted where the
    /// harness itself would have written it — see [`crate::sandbox::kept_in`].
    fn kept_in(&self, home: &Path) -> store::Account;

    /// The directory `account` is, which its login and its memory are said
    /// from: the first path it declares, which [`store::Account::directory`]
    /// is for every built-in. A plugin's account is held by its keys, so its
    /// harness names which.
    fn directory<'a>(&self, account: &'a store::Account) -> &'a Path {
        account.directory()
    }

    /// Each of `account`'s own paths, and where it is joined into a session's
    /// HOME at `home` — see [`crate::sandbox`]'s `account_inside`, which says
    /// why one list answers both the onboarding wizard and the mounting.
    fn account_inside(&self, account: &store::Account, home: &Path) -> Vec<(PathBuf, PathBuf)>;

    /// The paths a Profile of `account` is judged by, in order, each with what
    /// the Profile is broken as where that path is not there — see
    /// [`crate::profiles`]'s `broken`.
    fn judged(&self, account: &store::Account) -> Vec<(PathBuf, verkstead_render::Broken)>;

    /// The file of `account` a session is given a copy of beside its root,
    /// with the trust it needs seeded and what it changes merged back as it
    /// ends — Claude's `.claude.json` — or `None` for a harness with none.
    fn copied<'a>(&self, _account: &'a store::Account) -> Option<&'a Path> {
        None
    }

    /// What a root of this harness is called among a Conversation's roots —
    /// see [`crate::sandbox::root::Root::named`].
    fn root(&self) -> &str;

    /// Where the account's directory lands inside HOME, which every path of
    /// the account's is said from there: its root's name, and HOME itself for
    /// a harness whose account is a home.
    fn landing(&self) -> &str {
        self.root()
    }

    /// The directories of Verkstead's own a root is made of, each said from
    /// HOME, with the Profile's `memory` switch — see
    /// [`crate::sandbox::root::Root::built`].
    fn built(&self, _memory: bool) -> Vec<&str> {
        vec![self.root()]
    }

    /// The directories made empty inside a root as it is built, said from
    /// [`Harness::landing`], for what is written and joined there.
    fn furnished(&self) -> Vec<PathBuf> {
        Vec::new()
    }

    /// The file the account keeps its login in, said from the account's
    /// directory.
    fn login(&self) -> &str;

    /// Whether the login is inside the memory store, so that a root sharing
    /// its memory joins it with the store rather than as a file of its own.
    fn login_in_memory(&self) -> bool {
        false
    }

    /// Whether a session on `platform` is given a copy of the login rather
    /// than a link to it — see [`crate::sandbox::root::Root::login_copied`].
    fn login_copied(&self, _platform: Platform) -> bool {
        false
    }

    /// The account's own configuration files, said from its directory, that
    /// are read as they are found and handed to [`Harness::configured`].
    fn configuration(&self) -> Vec<PathBuf>;

    /// What a root holds of the account's configuration: each file said from
    /// [`Harness::landing`], and its contents.
    ///
    /// **What the later `write-configuration` export answers** (ADR-0030):
    /// `found` is each of [`Harness::configuration`]'s files as the account
    /// holds it, or nothing where it has none, and `servers` the MCP servers
    /// the Conversation attached. The account's own servers are never carried
    /// across, which is ADR-0011's rule and the host's: what is written back to
    /// the account is decided there, and nothing composed here is.
    fn configured(
        &self,
        found: Vec<Option<Vec<u8>>>,
        servers: &[AttachedServer],
    ) -> Vec<(PathBuf, Vec<u8>)>;

    /// The file the harness reads as its global instructions, said from
    /// [`Harness::landing`] — see [`crate::sandbox::root::Root::instructed`].
    fn instructions(&self) -> PathBuf;

    /// Whether the harness keys its memory store by the directory a session
    /// runs in, so that a root is told the [`Place`] it runs in.
    fn placed(&self) -> bool {
        false
    }

    /// The memory store a root sharing its memory joins from the account,
    /// each said from the account's directory, for a session at `place`.
    fn memory(&self, place: Option<&Place>) -> Vec<PathBuf>;

    /// Which of [`Harness::memory`] are files rather than directories, each
    /// made empty in the account where it is missing.
    fn memory_files(&self) -> Vec<PathBuf> {
        Vec::new()
    }

    /// The directories of the memory store a root not sharing its memory
    /// makes empty of its own, said from [`Harness::landing`].
    fn unshared(&self) -> Vec<&str>;

    /// The parts of the memory store a session that runs from a copy syncs —
    /// see [`crate::sandbox::root::Root::synced`].
    ///
    /// **The unit is the smallest one each harness's store has**, because only
    /// one of them keys its store by path:
    ///
    /// - **Claude** — the two entries a root already names, one for the Repo and
    ///   one for the Worktree.
    /// - **Codex** — the memory files whole, and out of the one flat directory
    ///   of rollouts the ones this Conversation's sessions wrote, which is what
    ///   `worktree` picks out: the store is filed by date and holds every
    ///   repository the human has ever worked in, and none of the rest of it is
    ///   this Repo's anything.
    /// - **Grok Build** — the same, its sessions picked out by the ids Verkstead
    ///   named them with, which is what its store files a session's directory
    ///   under. Not `session_search.sqlite` at the top of it, which is the
    ///   human's index of the whole store: a copy of a mirror's partial one
    ///   written over it on the bound device would be a search that had lost the rest.
    /// - **OpenCode** — the one database Verkstead pins every session it runs
    ///   onto, and the two siblings SQLite keeps beside it: the store runs in
    ///   write-ahead-log mode, and a database carried without its `-wal` and
    ///   `-shm` will not open. Not the data directory whole, which also holds
    ///   the login the account mirror carries and whatever other channel of
    ///   opencode the host has installed — see [`crate::sandbox::OPENCODE_DB_FILE`].
    ///
    ///   **And this is the one harness whose unit is not this Repo's.** opencode
    ///   keeps a row per session in that one database rather than a directory
    ///   per working directory, so the sessions of every repository the human
    ///   has run it in are inside the file, and the file is the smallest thing
    ///   there is to carry. Narrowing further means reading and writing rows of
    ///   a schema that is opencode's own and moves between releases — which is
    ///   the dependency [`crate::records`] deliberately does not take, it being
    ///   allowed to stop reading where this would have to keep writing. So what
    ///   crosses is narrower than it was and is still the whole store, and the
    ///   bound in [`crate::mirroring::memory::MOST_A_STORE_IS`] is what stands
    ///   in front of a big one.
    /// - **Antigravity CLI** — Grok's rule twice over, by the ids agy gave this
    ///   Conversation's sessions (see [`crate::transcript`]): the one database
    ///   per conversation under `conversations/`, and the conversation's own
    ///   directory under `brain/`, where its Transcript is. Not the summaries
    ///   database beside them, for the reason Grok's index stays: it is the
    ///   human's list of every conversation, and agy 1.4.12 resumes one it has no
    ///   row for, writing the row as it does.
    fn synced(
        &self,
        place: Option<&Place>,
        worktree: Option<&str>,
        sessions: &[String],
    ) -> Vec<Part>;

    /// The paths a mirror of an account holds beside its login, each said
    /// from the home the mirror is — see [`crate::sandbox::root::mirrored_of`].
    fn mirrored(&self) -> Vec<PathBuf>;

    /// And what a mirror of `account` holds at each of them, read off the
    /// account as it is at this moment, or nothing where it has no such file —
    /// see [`crate::sandbox::root::mirrored`].
    fn mirroring(&self, account: &store::Account) -> Vec<Option<Vec<u8>>>;

    /// How a login of this harness goes, or `None` for a harness whose login
    /// Verkstead does not run — see [`crate::logins`].
    fn speaking(&self) -> Option<&'static Speaking> {
        None
    }

    /// How the record of a session is looked for, under the account at
    /// `account` with the Profile's `memory` switch, in the Conversation's own
    /// `home` — see [`crate::transcript::Tail::of`].
    fn search(
        &self,
        account: &Path,
        memory: bool,
        home: &Home,
        session: &str,
        worktree: Option<&Path>,
        launched: SystemTime,
    ) -> Search;

    /// The record of an earlier session a resumed one carries on from — see
    /// [`crate::transcript::carried`].
    fn carried<'a>(
        &'a self,
        account: &'a Path,
        memory: bool,
        session: &'a str,
        worktree: &'a Path,
        home: &'a Home,
    ) -> Carrying<'a>;

    /// The model a default Pairing on this harness runs for `role` — see
    /// [`crate::pairing_defaults`].
    fn model(&self, role: store::Role) -> &str;
}

/// What [`Harness::carried`] answers: the record found, once it has been
/// looked for.
pub(crate) type Carrying<'a> = Pin<Box<dyn Future<Output = Option<Carried>> + Send + 'a>>;

/// The harness a Profile of `agent_type` runs, or `None` where this server
/// has no harness of that id — a Profile written on a device with a plugin
/// this one has not got, which is broken here rather than run (see
/// [`crate::profiles`]'s `broken`).
///
/// The one place an id is turned into its harness.
pub(crate) fn of(agent_type: store::AgentType) -> Option<&'static dyn Harness> {
    BUILT_IN
        .iter()
        .zip(store::BUILT_IN)
        .find(|(_, built_in)| built_in.agent_type == agent_type)
        .map(|(harness, _)| *harness)
        .or_else(|| {
            PLUGGED
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .get(&agent_type)
                .map(|plugged| *plugged as &'static dyn Harness)
        })
}

/// The harnesses plugins have declared in this process, by their plugin's
/// uuid — see [`plugged`].
static PLUGGED: RwLock<BTreeMap<store::AgentType, &'static plugged::Plugged>> =
    RwLock::new(BTreeMap::new());

/// Stand up the harness the plugin `uuid` declares as `agent`, drawn with
/// `mark` and run by its component `loaded`, so that [`of`] finds it — and
/// answer its id.
///
/// **Held for the life of the process**, as a built-in is: every answer of
/// [`Harness`] is borrowed from it, by sessions that may outlive a reload of
/// the plugin. So a plugin read again and declaring the same harness keeps
/// the one already standing and the component it was handed is let go, and
/// one declaring a different harness — after an **Update** — replaces it, the
/// old one left behind for whatever is still holding it. A plugin disabled or
/// removed keeps its harness here too, which is what lets a session already
/// running under it finish: what is offered is each server's own list — see
/// [`crate::plugins::installed`].
pub(crate) fn plugged(
    uuid: &str,
    agent: &crate::plugins::manifest::Agent,
    mark: String,
    loaded: crate::plugins::component::Loaded,
) -> store::AgentType {
    let id = store::AgentType::read(uuid);
    let mut plugged = PLUGGED
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let standing = plugged
        .get(&id)
        .is_some_and(|held| held.declared() == agent && held.mark() == mark);
    if !standing {
        let harness = plugged::Plugged::new(id, agent.clone(), mark, loaded);
        plugged.insert(id, Box::leak(Box::new(harness)));
    }

    id
}

/// Every harness this server can run, each with its id, in the order the
/// pickers offer them.
pub(crate) fn every() -> impl Iterator<Item = (store::AgentType, &'static dyn Harness)> {
    store::BUILT_IN
        .iter()
        .zip(BUILT_IN)
        .map(|(built_in, harness)| (built_in.agent_type, harness))
}

/// How a session of `agent_type` puts a Question Set to the human — see
/// [`Harness::channel`] — said for the CLI's Guide, which is outside the crate
/// the trait is reached in. Blocking for a harness this binary has not got,
/// which is how the Guide has always been printed outside a sandbox.
pub fn channel(agent_type: store::AgentType) -> store::Channel {
    of(agent_type).map_or(store::Channel::Blocking, |harness| harness.channel())
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

/// One path an account of a harness is made of — see [`Harness::paths`].
pub(crate) struct Declared<'a> {
    /// The key the account holds it under.
    pub(crate) key: &'a str,

    /// What the Profile form labels its field.
    pub(crate) label: &'a str,

    /// What it is for, said under the field, with what is to be read as code
    /// between backticks.
    pub(crate) purpose: &'a str,

    /// What the field shows before anything is typed into it.
    pub(crate) placeholder: &'a str,

    /// Whether it is a file rather than a directory.
    pub(crate) file: bool,

    /// What a save of it is refused as, where it was named relatively, where
    /// nothing is there, and where what is there is the wrong kind of thing —
    /// each its own sentence on the form, so that a refusal says which field.
    pub(crate) refused: [verkstead_render::ProfileSaved; 3],

    /// Directories inside it a save judges as well, said from it, refused as
    /// it is: what is mounted of an account kept somewhere inside a home.
    pub(crate) within: &'a [&'a str],
}

/// What a home an account is kept under is refused as — see
/// [`Declared::refused`].
pub(crate) const HOME_REFUSED: [verkstead_render::ProfileSaved; 3] = [
    verkstead_render::ProfileSaved::HomeNotAbsolute,
    verkstead_render::ProfileSaved::HomeMissing,
    verkstead_render::ProfileSaved::HomeNotADirectory,
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
}

/// The words a launch line may hold for the host to fill — see [`Line`] and
/// [`filled`]: the model, the Brief, the session id Verkstead named, the
/// Worktree the session works in, and the file its session-only settings are
/// written to.
pub(crate) const MODEL: &str = "{model}";
pub(crate) const PROMPT: &str = "{prompt}";
pub(crate) const SESSION: &str = "{session}";
pub(crate) const WORKTREE: &str = "{worktree}";
pub(crate) const SETTINGS: &str = "{settings}";

/// What a session's `{settings}` is filled with: the file inside the root,
/// said from [`Harness::landing`], that its session-only settings are written
/// to — see [`Harness::configured`].
pub(crate) const SETTINGS_FILE: &str = "verkstead-settings.json";

/// The rest of a backend's launch line: how it is told its model, how it is
/// given the Brief, whether it takes the session id Verkstead named, and what
/// goes on the end.
///
/// One value rather than a mapping apiece, because they are one fact — what
/// this harness's command line looks like — and a harness answers it once, in
/// [`Harness::line`].
///
/// **Each part is words with placeholders in them**, [`MODEL`], [`PROMPT`],
/// [`SESSION`], [`WORKTREE`] and [`SETTINGS`], which the host fills as the line
/// is built — see [`filled`]. That is the shape a plugin's manifest writes a
/// line in (ADR-0030, *Agent*), so a harness declared there answers this as
/// readily as one written here.
pub struct Line {
    /// How the model is said, holding [`MODEL`]. `--model` reads as claude's
    /// own; codex takes `-m`.
    pub(crate) model: Vec<String>,

    /// How the Brief is given, holding [`PROMPT`]: a flag and it, or it alone
    /// as the one positional argument.
    ///
    /// The positional is what every backend up to this one takes, and opencode
    /// is the first that does not: its positional is the project to start in,
    /// and the prompt is `--prompt`. Said here rather than assumed, so that a
    /// harness taking either shape is one answer of [`Harness::line`] rather
    /// than a branch in the builder.
    pub(crate) prompt: Vec<String>,

    /// How the session Verkstead named is named on the line, holding
    /// [`SESSION`].
    ///
    /// `None` for a backend that takes no session id at all, whose log is
    /// therefore found rather than named — see [`crate::transcript`].
    pub(crate) session: Option<Vec<String>>,

    /// And the other line this same backend has: the one that **continues** a
    /// conversation instead of opening one, under the id of the session that was
    /// having it.
    ///
    /// Mutually exclusive with [`Line::session`] wherever both could be said,
    /// and that is a fact about what a session is rather than a convention: a
    /// conversation carried on keeps the id it already had, so there is
    /// nothing left to name it.
    ///
    /// And it is not always a flag — see [`Resume`]: codex's is a subcommand, and
    /// what a subcommand changes is the shape of the whole line rather than what
    /// is on the end of it.
    ///
    /// `None` where Verkstead does not carry a conversation on from a session of
    /// this backend, which is now no built-in at all: all five have a resume,
    /// and what separated them was never that but whether Verkstead knew the id
    /// to resume by and whether their store had to be put right first. It stays
    /// an `Option` because a plugin's harness may declare none, and a launch
    /// under it then opens a session of its own, which is Verkstead's own
    /// Resume and nothing lost.
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
    /// Words holding [`SESSION`], written where the session would have been
    /// named. Claude's and Grok Build's, both `--resume`, and OpenCode's,
    /// `--session`.
    Flag(Vec<String>),

    /// Words written straight after the binary, ahead of the line.
    ///
    /// Where they hold [`SESSION`], the id is said there, which is the shape a
    /// plugin's manifest writes. Where they do not, the id is a positional in
    /// front of the prompt, after the model — codex's, whose words are
    /// `resume` alone.
    Subcommand {
        /// The words themselves.
        words: Vec<String>,

        /// And what the continuation needs said that the ordinary line does not,
        /// which goes on the end with the tail.
        ///
        /// Here rather than in [`Line::tail`] because it is about resuming: a
        /// launch opening a conversation would be carrying a setting for a
        /// question it is not asking.
        also: Vec<String>,
    },
}

/// What the placeholders of a launch line are filled with — see [`filled`].
/// One left `None` is filled with nothing.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Filling<'a> {
    pub(crate) model: Option<&'a str>,
    pub(crate) prompt: Option<&'a str>,
    pub(crate) session: Option<&'a str>,
    pub(crate) worktree: Option<&'a str>,
    pub(crate) settings: Option<&'a str>,
}

/// `words` with each placeholder in them filled from `filling`.
///
/// **In one pass**, so that what a placeholder is filled with is never read for
/// placeholders in turn: a Brief that happens to say `{session}` reaches the
/// harness saying it.
pub(crate) fn filled(words: &[String], filling: Filling<'_>) -> Vec<String> {
    let placeholders = [
        (MODEL, filling.model),
        (PROMPT, filling.prompt),
        (SESSION, filling.session),
        (WORKTREE, filling.worktree),
        (SETTINGS, filling.settings),
    ];

    words
        .iter()
        .map(|word| {
            let mut out = String::with_capacity(word.len());
            let mut rest = word.as_str();

            'scanning: while let Some(open) = rest.find('{') {
                out.push_str(&rest[..open]);
                rest = &rest[open..];

                for (placeholder, value) in placeholders {
                    if let Some(after) = rest.strip_prefix(placeholder) {
                        out.push_str(value.unwrap_or_default());
                        rest = after;
                        continue 'scanning;
                    }
                }

                out.push('{');
                rest = &rest[1..];
            }

            out.push_str(rest);
            out
        })
        .collect()
}

/// Whether `words` hold `placeholder` anywhere in them.
pub(crate) fn holds(words: &[String], placeholder: &str) -> bool {
    words.iter().any(|word| word.contains(placeholder))
}

/// Each of `words`, owned: how a built-in writes a part of its line.
pub(crate) fn said(words: &[&str]) -> Vec<String> {
    words.iter().map(|word| (*word).to_owned()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each built-in declares the paths the store reads its binding by, in the
    /// same order: the store keeps the keys so that a binding an older release
    /// wrote reads as an account, and the harness declares them so that the
    /// form asks for them — two statements of one fact, held to each other here.
    #[test]
    fn every_built_in_declares_the_paths_the_store_reads_it_by() {
        for (agent_type, harness) in every() {
            let declared: Vec<&str> = harness.paths().iter().map(|path| path.key).collect();

            assert_eq!(
                declared,
                agent_type.built_in().expect("a built-in").paths,
                "{agent_type}"
            );
        }
    }
}
