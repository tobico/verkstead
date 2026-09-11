//! The Verkstead server: the agents' HTTP API and the human's web UI, over one
//! SQLite store and out of one binary.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::routing::{get, post};
use sqlx::SqlitePool;
use verkstead_store::{Settlements, Waits};

/// The same two presses made from the answer sheet, which put a file on an
/// Answer to a Question Set rather than on the Brief.
mod answer_files;

/// The files the human put on a Conversation for its sessions to read: where
/// the bytes are kept, and what a file is called once they are there.
///
/// Public for the reason [`handoffs`] is — a Conversation's directory is part
/// of the surface its sessions run on rather than an implementation detail of
/// an endpoint.
pub mod attachments;
mod browsing;

/// The shared Rust build cache every sandbox is given: where it is, and whether
/// there is an sccache to compile through.
///
/// Public for the reason [`sandbox`] is — what a session builds into is part of
/// the surface it runs on rather than an implementation detail of an endpoint,
/// and standing a router up that runs sessions means saying where it is.
/// How long a Conversation's boundary lasts on the platform whose boundary is
/// an identity: the entries written at its first session, taken away with its
/// Worktree, and swept for at startup.
///
/// Public for the reason the sandbox is: how long what a session may reach
/// lasts is part of the product's own promise rather than an implementation
/// detail of an endpoint, and what proves a boundary has really been taken back
/// is a suite standing where the close and the sweep do — see
/// `crates/server/tests/sandbox_windows.rs`.
pub mod boundaries;
pub mod build_cache;
mod capture;
mod checklist;
mod checks;
mod cleanup;
mod commenting;
mod comments;
mod commits;
mod continuing;

mod conversations;
mod deferrals;
/// The uncommitted changes the server reads for a Question Set's Diff.
mod diffs;
mod drivers;
mod exchanges;
/// What a follow-up session is started on, and read back from where it stands.
mod follow_ups;
mod followers;
/// Verkstead's own reach into GitHub: the host's `gh`, run against a Repo.
///
/// Public for the reason [`sandbox`] is — what Verkstead reaches out to is the
/// product's business rather than an endpoint's, and standing a router up is
/// choosing which `gh` it runs.
pub mod github;
/// Grilling a Conversation again, where the session that was grilling it died.
mod grillings;
/// Where a Conversation's handoff document is written, and how it reaches the
/// Timeline.
///
/// Public for the reason the sandbox is: the directory is part of the surface a
/// session runs on — every sandbox binds one — so standing a router up that runs
/// sessions means saying where they live.
pub mod handoffs;
/// The Workbench Key: the secret the human's browser holds and a session cannot
/// read, and the gate that answers 401 to everything which has not shown it.
///
/// Public for the reason [`sandbox`] is — what stands between a session and the
/// workbench is the product's boundary rather than an implementation detail of
/// an endpoint, and standing the served router up means saying which key it is
/// keyed with.
pub mod key;
mod limits;
/// Watching a pull request go on merging after the work on it is Done — see
/// [`checks`] for the watcher that covers a wrap-up, which this takes over from.
mod merges;
mod nudge;
/// Telling a session idling on a stored ask that its Answers are there to fetch.
mod nudging;
/// Whether this Verkstead can do anything yet: the objective a fresh one is
/// short of, and the mode it enters at startup where it is.
///
/// Public for the reason the sandbox is — what says a machine is ready is the
/// product's own answer rather than an endpoint's, and what proves each arm of
/// it is a server stood up over a stated machine, which is a test standing
/// where a start does.
pub mod onboarding;
/// Every Sandbox Configuration bind as the settings page reads them: which of
/// the two places said each one, and whether the server can see it.
mod paths;
/// The named pipe the server listens on beside its socket, which is what a
/// sandboxed Windows session asks Verkstead through — the one way in when that
/// platform's boundary was an AppContainer, and a transport that has stayed
/// because no firewall has to agree with it.
///
/// Public for the reason [`sandbox`] is: what a session reaches Verkstead
/// through is the product's answer rather than an endpoint's, and what proves a
/// pipe is a pipe is a request really made over one.
///
/// Windows' own. The other platforms have a loopback nothing refuses them, so
/// there is no pipe here and no Unix-socket twin beside it either.
#[cfg(windows)]
pub mod pipe;
/// Where a directory of Verkstead's own goes when nobody has said: the
/// platform's own place for the Data Directory, and the environment values it
/// is resolved out of.
///
/// Public for the reason [`sandbox`] is — where Verkstead keeps what it makes is
/// the product's business rather than an endpoint's, and the default is one rule
/// for every binary that parses a [`Config`] rather than the server's alone.
pub mod platform;
mod profiles;
/// Putting a share where a link reaches it, which is Verkstead's own write to
/// GitHub.
mod publishing;
/// The open pull requests Verkstead did not open, which is the door work
/// already somewhere else comes into the pipeline through.
mod pull_requests;
mod push;
/// The store an OpenCode session keeps of itself, followed while it runs.
mod records;
/// Whether this machine can be reached from a phone: what its Tailscale is
/// doing, and whether the tailnet name is in front of the workbench.
pub mod remote;
/// Following a Conversation's branch to the name a session renamed it to,
/// rather than repairing a checkout that has not come adrift after all.
mod renames;
mod reply;
mod repos;
/// Speaking to a session that has gone idle without asking anything.
mod rescues;
/// A path as the filesystem has it, which is the one a Repo and an Agent
/// Profile are recorded under.
mod resolved;
/// Getting a finished Conversation's merge conflict resolved, at the human's
/// press.
mod resolving;
mod responding;
mod responses;
/// Starting to drive a Conversation again, from wherever it now stands.
mod resume;
mod review;
mod runner;
/// What a session can reach: the bwrap surface built around one Conversation's
/// worktree.
///
/// Public because it is the product's boundary rather than an implementation
/// detail of the endpoints — and because what proves a boundary is a probe run
/// inside it, which is a test standing where the orchestrator does.
pub mod sandbox;
mod screen;
mod sessions;
mod sets;
/// What Verkstead was told about the human's credentials and identity: the
/// settings files under the Data Directory.
///
/// Public for the reason the sandbox is — what a session authenticates and
/// commits as is the product's business rather than an endpoint's, and standing
/// a router up that runs sessions means saying where both are read from.
pub mod settings;
mod settling;
mod sharing;
/// What a session is grilled by: the skills Verkstead ships and installs into
/// every sandbox.
///
/// Public for the reason the sandbox is — they are part of the surface a session
/// runs on rather than an implementation detail of an endpoint, and standing a
/// router up that runs sessions means saying where they are installed.
pub mod skills;
mod stages;
/// The check that says when a Conversation has Stalled: in a driven state,
/// with nothing driving it and nothing asking the human about it.
mod stalls;
/// The stand-in programs the tests run in place of the machine's own.
#[cfg(test)]
mod stand_ins;
/// The human steering a Conversation: the click that stops the drive and opens
/// the modal, and the submit that moves the work where they said.
mod steering;
mod stopping;
/// The human stopping a Conversation on purpose: Stop, which waits for the step
/// it is on, and Force stop, which does not.
mod stops;
mod tasks;
/// The pseudo-terminal a session runs on — Verkstead's own, rather than one
/// `script` made inside the sandbox.
///
/// Public for the reason the sandbox is: what a session runs on is part of the
/// surface it runs on rather than an implementation detail of an endpoint, and
/// what proves a terminal is a terminal is a process running on one saying so.
pub mod terminal;
/// The terminals a Conversation holds of its own: a human's shell inside its
/// Sandbox, in the Screen's own machinery pointed at a shell.
///
/// Public for the reason the sandbox is: which shell a human gets at their own
/// machine is the product's answer rather than an endpoint's, and what proves
/// one is a shell really running inside a Sandbox — a test standing where the
/// orchestrator does, asking the machine the same question it asks.
pub mod terminals;
mod transcript;
/// What Verkstead says to a running session: the keystrokes the rescue and the
/// nudge both go in as.
mod typing;
mod ui;
/// Running a program without putting a window on the human's screen, which is
/// Windows' question and nobody else's.
///
/// Public because the tray app spawns too, and it is the tray app that has no
/// console for a child to inherit — see [`crate::remote::Elevate`], whose one
/// graphical implementation lives in the desktop crate and runs the platform's
/// own asking.
pub mod unseen;
mod updates;
mod viewer;
mod worktrees;
mod wrapping;

/// How this server runs a Conversation's agents. Public for the reason the
/// sandbox is: what a session is launched as is the product's business rather
/// than an endpoint's, and standing a router up is choosing it.
pub use sessions::Agents;

/// And how it reaches GitHub, which is the same kind of choice one step out: the
/// host's `gh`, or something standing where it goes.
pub use github::Gh;

/// How fast the backlog is worked, which is part of the same choice — see
/// [`Agents::at_pace`].
pub use runner::Pace;

/// Persistence lives in its own crate so the viewer's endpoints can reach it
/// without depending on the binary that links them. It is re-exported here
/// because, from the API's side of things, it is still the server's store.
pub use verkstead_store as store;
pub use verkstead_store::open_database;

/// What a site the server can serve is, for the tests that stand one up in place
/// of the built viewer — see [`router_with_viewer`].
pub use rust_embed::Embed;

/// How large a submitted Question Set may be. Generous, because the CLI
/// attaches the whole uncommitted Diff to every Set.
const MAX_SET_BYTES: usize = 32 * 1024 * 1024;

/// What the agents' API hangs off, before the id of the Conversation asking.
///
/// Every Set is asked from a Conversation and lands on its Timeline, so the
/// whole of the agent contract is under here: a session is handed this and its
/// own id as `VERKSTEAD_SERVER` — see [`sandbox::Reachable`] — and the bundled
/// CLI then names the Conversation in every request it makes without knowing it
/// is doing so.
///
/// The same word the viewer routes a Conversation on, because it is the same
/// Conversation: `/conversations/7` in a browser is the workbench open on the
/// one whose sessions ask here.
pub(crate) const ASKING_FROM: &str = "/conversations";

/// The longest a client may ask to have a wait held open. There is no expiry
/// on the waiting itself — the client owns retry (ADR-0001), so it picks the
/// hold length and the server only bounds it.
const MAX_HOLD: Duration = Duration::from_secs(60);

/// How many settlements a held wait can fall behind before it gives up
/// following along and goes back to the store instead. One notification per
/// Set settled, for a single human settling them: this is generous.
const SETTLEMENT_BACKLOG: usize = 64;

/// What the handlers share: the store, word of what has just moved — so held
/// waits need not poll for a Set arriving and open pages hear about everything
/// else — which Sets a wait is being held on, which sessions are running, which
/// of them a pick has armed a watcher on, what is driving each Conversation,
/// whether a newer Verkstead has been released than this one, and which
/// directories any of it may touch.
#[derive(Clone)]
pub(crate) struct AppState {
    pool: SqlitePool,
    nudges: nudge::Nudges,
    settlements: Settlements,
    waits: Waits,
    sessions: sessions::Sessions,

    /// And the terminals each Conversation is holding of its own — see
    /// [`terminals`]. A register beside the sessions rather than a bend in it:
    /// a Conversation has one session and may have any number of terminals, and
    /// what runs on one is the human rather than an agent.
    terminals: terminals::Terminals,

    /// The watcher each Conversation's latest pick armed — see [`followers`].
    /// Beside the sessions rather than inside them, because a watcher is a task
    /// of Verkstead's own and a session is an agent's process.
    followers: followers::Followers,

    /// And what is driving each of them, which is the other half of the same
    /// question: a session is one agent running, and a driver is the task that
    /// keeps starting them — see [`drivers`].
    drivers: drivers::Drivers,

    updates: updates::Updates,

    /// And the Sandbox Configuration the installation was started with, which is
    /// here for the settings page rather than for a session: a session's binds
    /// are composed where its sandbox is built, and this page draws every bind
    /// there is and says which of the two places said each one — see [`paths`].
    binds: sandbox::SandboxConfig,

    /// How Verkstead itself asks GitHub about a pull request — the host's `gh`,
    /// authenticating as the configured token.
    github: Gh,

    /// And how it asks this machine whether a phone can reach the workbench —
    /// the host's `tailscale`, in front of the port this server is listening
    /// on. A handle rather than a reading: what it answers is read at the moment
    /// the Remote access pane asks, so a `tailscale up` run in a terminal shows
    /// on the next load rather than on the next restart — see [`remote`].
    remote: remote::Tailscale,

    /// The Workbench Key the gate in front of this router stands on, where it
    /// stands on one — see [`key`].
    ///
    /// Held for the one press that changes it, which is **Reset key** on the
    /// Remote access pane: the gate reads this handle on every request, so
    /// re-issuing through it logs every device holding the old secret out at
    /// once. `None` is a router with no gate over it, where there is no key to
    /// re-issue and nothing a re-issue would mean.
    key: Option<key::WorkbenchKey>,

    /// Whether this Verkstead can do anything yet, and the machine that is
    /// probed to say so — see [`onboarding`].
    ///
    /// A handle rather than a reading, like the two above it, and with one
    /// thing in it that is neither: the mode, which is settled once at startup
    /// and held for the length of the run. Everything else it answers is
    /// probed at the moment the wizard asks.
    onboarding: onboarding::Onboarding,

    /// The two files the human tells Verkstead their credentials and their
    /// identity in. A handle rather than what is in them: the files are read at
    /// the moment they are wanted, so the settings page and the next session to
    /// spawn see the same thing — see [`settings`].
    settings: settings::Settings,

    /// Where Verkstead keeps what it makes — the worktrees, for now. Not one of
    /// the directories the human points Verkstead at: this is the one Verkstead
    /// was given for its own things.
    data_dir: PathBuf,

    /// Held across the window between a checkout being made and the record
    /// naming it.
    ///
    /// A start makes its directories and *then* writes the rows that name them,
    /// which is the right way round — a row naming a directory that was never
    /// made is the worse of the two half-states, and it is the order every start
    /// here is written in. But it leaves a moment in which a live checkout is on
    /// disk and nothing in the store says so, and the sweep of orphaned
    /// worktrees decides what to delete by exactly that reading. So the two are
    /// serialised on this: every make-then-record window takes it, and
    /// [`worktrees::sweep`] holds it across reading the keep-set and acting on
    /// what it read.
    ///
    /// Nothing is inside it. What it protects is a window rather than a value,
    /// and the value that window is about is the store.
    ///
    /// **The window is the making, and nothing before it.** A start asks git
    /// plenty before it makes anything — a fetch per repository above all,
    /// which has no deadline to answer within — and a lock held around the
    /// asking as well would let one unreachable remote hold every close in the
    /// workbench behind it, a close being the one thing that must never be
    /// held. So each of these takes it as late as it can. The three that plan
    /// inside a blocking half take it in there, past the fetches, and hand the
    /// guard back out to the record; a steer plans before it takes anything,
    /// so it holds it from its own [`steering::make`] onwards.
    ///
    /// A rebuild does not take it. What that remakes is a directory the record
    /// already names, so the keep-set holds it whenever the sweep looks.
    checkouts: Arc<tokio::sync::Mutex<()>>,
}

/// The port Verkstead is served on when nobody has said otherwise, and so the
/// port `tailscale serve` is put in front of — see the Remote access pane in
/// [`remote`], which is what the adoption docs point at for it.
///
/// Written once and read twice: it is the `--listen` default below, and what a
/// router stood up without a listening socket reads a serve against.
const WORKBENCH_PORT: u16 = 8422;

/// The one name the database is ever kept under, inside the Data Directory.
/// Fixed rather than configurable: the directory is what an operator points
/// Verkstead at, and a file inside it is Verkstead's own business.
const DATABASE_NAME: &str = "verkstead.db";

/// How several directories are separated when one environment variable holds
/// more than one: however the platform writes `PATH`. A `:` on Unix; a `;` on
/// Windows, where a `:` is a drive letter's own punctuation and splitting on
/// it would cut `C:\src` into a drive that is not a path and a path that is
/// not absolute.
///
/// clap applies a delimiter to the flag as well as to the variable, so this is
/// what `--sandbox-bind` is parsed with too: wrong on Windows, it refuses
/// every startup that names a real directory.
#[cfg(windows)]
const PATH_LIST_SEPARATOR: char = ';';

/// See the Windows one above.
#[cfg(not(windows))]
const PATH_LIST_SEPARATOR: char = ':';

/// How the server is pointed at its data directory and its socket. There is no
/// app-level auth: the tailnet is the perimeter, so the defaults keep the
/// server on the loopback interface until told otherwise.
#[derive(Debug, Clone, clap::Parser)]
#[command(name = "verkstead serve", version, about = "Verkstead server")]
pub struct Config {
    /// Where Verkstead keeps everything it makes: the database, at
    /// `verkstead.db` inside it, the Conversations' worktrees, the installed
    /// Skills, the handoff directories and the settings files. Created if it
    /// does not exist.
    ///
    /// This is the Data Directory, and not one to point at a directory the
    /// human works in: a Repo and an Agent Profile are what the human points
    /// Verkstead at, and this is Verkstead's own.
    ///
    /// Unsaid, it is the platform's own place for it — `~/.local/share/verkstead`
    /// on Linux, `~/Library/Application Support/Verkstead` on macOS,
    /// `%APPDATA%\Verkstead` on Windows — so that a Verkstead started from an
    /// icon and one started from a shell keep their work in the same place. A
    /// developer running out of a checkout says `--data-dir .` for what that
    /// used to be by default.
    ///
    /// What is held here is what was *said*, which is why it is an option and
    /// not a resolved path: a machine with nowhere to resolve to is refused at
    /// startup, where a refusal has somewhere to be worded — see
    /// [`platform::data_dir`].
    #[arg(long, env = "VERKSTEAD_DATA_DIR", value_name = "DIR")]
    pub data_dir: Option<PathBuf>,

    /// Address and port to bind. Bind a tailnet address to reach the server
    /// from other devices.
    #[arg(
        long,
        env = "VERKSTEAD_LISTEN",
        default_value_t = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), WORKBENCH_PORT),
    )]
    pub listen: SocketAddr,

    /// An extra read-write bind every sandbox gets, or `name=DIR` for one only
    /// the Repo registered under that name gets. Repeat the flag, or separate
    /// several in the environment variable the way the platform writes `PATH`.
    ///
    /// This is the Sandbox Configuration: the package registries and the caches
    /// a session needs beyond its own worktree that Verkstead does not provide
    /// itself. Each names a directory of somebody else's and is a hole in the
    /// boundary a sandbox is, which is why a bind that is not there refuses
    /// startup rather than being skipped: a flag is the installation's own word,
    /// and nobody is watching when it is wrong.
    ///
    /// Not a requirement either, and not the only place they are said. The
    /// workbench settings take the same two grammars, a session gets the union
    /// of the two, and the settings' own are the ones that are never fatal — see
    /// [`sandbox::SandboxConfig`].
    ///
    /// A Rust build cache is not one of them: the server provides that one — see
    /// `--build-cache-dir` — and the switch that turns it off is in the
    /// workbench settings.
    #[arg(
        long = "sandbox-bind",
        env = "VERKSTEAD_SANDBOX_BINDS",
        value_delimiter = PATH_LIST_SEPARATOR,
        value_name = "DIR|NAME=DIR"
    )]
    pub sandbox_binds: Vec<String>,

    /// Where the shared Rust build cache goes: one directory every sandboxed
    /// session downloads its crates and compiles its dependencies into, so a
    /// dependency is built once for the machine rather than once per
    /// Conversation.
    ///
    /// Defaults to `$XDG_CACHE_HOME/verkstead`, or `~/.cache/verkstead` where
    /// that is unset. Made where it is not there, which is the one directory
    /// outside the Data Directory Verkstead creates — the path is Verkstead's
    /// own choice unless this says otherwise, and a feature that is on by
    /// default cannot ask for a `mkdir` first.
    ///
    /// Unlike a Sandbox Configuration bind this is not a hole somebody typed
    /// into the boundary: it is the server's own directory, holding nothing but
    /// build output, which is why it is opened for a human who never asked and
    /// the only control over it — in the workbench settings, beside the size it
    /// may grow to — is the one that closes it.
    #[arg(long, env = "VERKSTEAD_BUILD_CACHE_DIR", value_name = "DIR")]
    pub build_cache_dir: Option<PathBuf>,

    /// Don't ask GitHub whether a newer Verkstead has been released, and so
    /// never show the Update Notice. The check is one unauthenticated request
    /// a day and installs nothing, but anything that reaches the internet at
    /// all has to be able to be told not to.
    #[arg(
        long,
        env = "VERKSTEAD_NO_UPDATE_CHECK",
        action = clap::ArgAction::SetTrue,
        // Anything that is not a falsey word counts as set. This one is thrown
        // from a service unit or a shell as often as from the command line, and
        // `=1` is how a switch is thrown there; clap's own parser for a flag
        // would refuse it for not being the word `true`.
        value_parser = clap::builder::FalseyValueParser::new(),
    )]
    pub no_update_check: bool,
}

impl Config {
    /// Where the update check asks about releases, or `None` where it has been
    /// turned off — the one thing [`Config::no_update_check`] decides, named so
    /// that what it decided can be asked about rather than inferred.
    pub fn releases(&self) -> Option<&'static str> {
        (!self.no_update_check).then_some(updates::LATEST_RELEASE)
    }

    /// The Data Directory this configuration comes to, made where it is not
    /// there yet.
    ///
    /// What the flag holds is what was *said*, which may be nothing at all: a
    /// machine with nowhere to resolve one to is refused here, where a refusal
    /// still has somewhere to be worded — see [`platform::data_dir`].
    ///
    /// Resolving it is what a caller that is not the server itself comes for.
    /// The desktop app has to reach the Workbench Key before it has started
    /// serving, and both halves arriving at one directory is what makes it the
    /// same key rather than two.
    pub fn data_directory(&self) -> Result<PathBuf> {
        let data_dir = platform::data_dir(self.data_dir.as_deref())?;

        std::fs::create_dir_all(&data_dir)
            .with_context(|| format!("creating data directory {}", data_dir.display()))?;

        Ok(data_dir)
    }

    /// The Workbench Key this configuration's Data Directory holds: whatever is
    /// in there, or a new one written where a first start finds nothing.
    ///
    /// **The one call both halves of a start make.** `verkstead serve` reaches
    /// it through [`run_on`] and the desktop app reaches it before it spawns
    /// the server, because the browser it opens has to be opened on the link —
    /// and a desktop that invented a key of its own would be locked out of the
    /// Data Directory it restarts against. What comes back is a handle each of
    /// them holds a clone of, so a key re-issued through one of them is the key
    /// the other hands out — see [`key::WorkbenchKey`].
    pub fn workbench_key(&self) -> Result<key::WorkbenchKey> {
        let data_dir = self.data_directory()?;

        key::WorkbenchKey::issued(&data_dir)
            .with_context(|| format!("keeping the workbench key in {}", data_dir.display()))
    }
}

/// The SQLite file, which is [`DATABASE_NAME`] inside the Data Directory and is
/// never anywhere else: one directory is what an operator says, and everything
/// in it is Verkstead's to name.
///
/// Taking the directory rather than the [`Config`], because the directory is
/// what was resolved and the configuration only holds what was said.
pub fn database(data_dir: &Path) -> PathBuf {
    data_dir.join(DATABASE_NAME)
}

/// Everything the server answers in a serialised format: the agents' contract
/// under `/api/v1/`, and the viewer's own namespace under `/api/ui/`.
///
/// Both live under `/api/`, which is also the one prefix the viewer's fallback
/// refuses to answer with the document — see [`viewer`].
///
/// Keeping nothing: it is given no Data Directory, so it has nowhere to put a
/// worktree. That is what everything with no checkout to make wants — see
/// [`router_keeping`] for the other one.
pub fn router(pool: SqlitePool) -> Router {
    routed(
        pool,
        updates::Updates::nothing_learned(),
        nothing_bound(),
        nowhere(),
        sessions::Sessions::none(),
        Gh::on_path(),
        tailnet(),
        key::Gate::open(),
        onboarding::Machine::here(),
    )
}

/// The same, keeping what it makes in `data_dir`.
///
/// It runs no sessions: starting a grilling makes the branch and the worktree
/// and records that it did, and there is nothing here to launch inside them.
/// See [`router_running_sessions`] for the one that does.
pub fn router_keeping(pool: SqlitePool, data_dir: PathBuf) -> Router {
    routed(
        pool,
        updates::Updates::nothing_learned(),
        nothing_bound(),
        data_dir,
        sessions::Sessions::none(),
        Gh::on_path(),
        tailnet(),
        key::Gate::open(),
        onboarding::Machine::here(),
    )
}

/// The same, over what the *installation* configured — the Sandbox Configuration
/// binds its flags named — and reaching GitHub through `gh`.
///
/// What the settings endpoints are stood up over where the question is about
/// paths: the page draws both sources at once and says which of the two said
/// each entry, so a test of that labelling needs a router that was configured by
/// an installation as well as by a file — see [`paths`].
pub fn router_installed(
    pool: SqlitePool,
    binds: sandbox::SandboxConfig,
    data_dir: PathBuf,
    gh: Gh,
) -> Router {
    routed(
        pool,
        updates::Updates::nothing_learned(),
        binds,
        data_dir,
        sessions::Sessions::none(),
        gh,
        tailnet(),
        key::Gate::open(),
        onboarding::Machine::here(),
    )
}

/// And the same again, running its sessions under `agents` and reaching GitHub
/// through `gh` — which is what the served router does, and what a test asking
/// whether a session's output reaches the Timeline has to stand up for itself.
///
/// The `gh` is a parameter for the reason the agent inside `agents` is one: what
/// a finish step leaves behind is a pull request on GitHub, and asking the real
/// one would be a test that needed a network and an account.
pub fn router_running_sessions(
    pool: SqlitePool,
    data_dir: PathBuf,
    agents: Agents,
    gh: Gh,
) -> Router {
    // Taken off the agents rather than asked for again: the binds a session gets
    // and the binds the settings page draws as the installation's are the one
    // set, and two ways of saying it would be two things to keep in step.
    let binds = agents.binds().clone();

    routed(
        pool,
        updates::Updates::nothing_learned(),
        binds,
        data_dir,
        sessions::Sessions::under(agents),
        gh,
        tailnet(),
        key::Gate::open(),
        onboarding::Machine::here(),
    )
}

/// A router keeping its files in `data_dir` and reaching GitHub through `gh`,
/// running no session at all.
///
/// What the settings endpoints are stood up over: saving a GitHub token asks
/// GitHub who it authenticates as, and asking the real one would be a test that
/// needed a network and somebody's account — so the `gh` is a parameter here for
/// the reason it is one on [`router_running_sessions`].
pub fn router_asking_github(pool: SqlitePool, data_dir: PathBuf, gh: Gh) -> Router {
    routed(
        pool,
        updates::Updates::nothing_learned(),
        nothing_bound(),
        data_dir,
        sessions::Sessions::none(),
        gh,
        tailnet(),
        key::Gate::open(),
        onboarding::Machine::here(),
    )
}

/// The Sandbox Configuration of a router the installation configured none for,
/// which is every one of them but the served router and the test that is about
/// what an installation said.
fn nothing_bound() -> sandbox::SandboxConfig {
    sandbox::SandboxConfig::default()
}

/// A router whose onboarding probes `machine` rather than the one the tests are
/// running on, keeping its settings files in `data_dir`.
///
/// The seam the onboarding suite is stood up over, and a parameter for the
/// reason [`router_reading_tailscale`]'s Tailscale is one: what the wizard
/// answers is a fact about the machine underneath it, and a wizard that has to
/// say something about three platforms, eight distributions and a `bwrap` that
/// will not run cannot be asked about any of them on the one machine the suite
/// happens to be on. See [`onboarding::Machine::stated`].
///
/// The Data Directory because two of the three steps are read from there: the
/// git author is in `config.yaml`, and the Profiles are in the database beside
/// it.
pub fn router_onboarding(
    pool: SqlitePool,
    data_dir: PathBuf,
    machine: onboarding::Machine,
) -> Router {
    router_onboarding_asking_github(pool, data_dir, machine, Gh::on_path())
}

/// The same, reaching GitHub through `gh`.
///
/// What the wizard's last step is stood up over: its token field is prefilled
/// from the host `gh`'s own login where the environment holds nothing, and
/// asking the real one would be a test that answered differently on every box
/// — see [`router_asking_github`], which is a parameter for the same reason.
pub fn router_onboarding_asking_github(
    pool: SqlitePool,
    data_dir: PathBuf,
    machine: onboarding::Machine,
    gh: Gh,
) -> Router {
    routed(
        pool,
        updates::Updates::nothing_learned(),
        nothing_bound(),
        data_dir,
        sessions::Sessions::none(),
        gh,
        tailnet(),
        key::Gate::open(),
        machine,
    )
}

/// A router reading this machine's Tailscale through `remote`, over a database
/// with nothing in it.
///
/// What the Remote access pane's own suite is stood up over, and a parameter for
/// the reason the `gh` above is one: what it answers is a fact about the machine
/// running the tests, and a pane that has to say four different things about
/// four different machines cannot be asked about any of them on the one machine
/// it happens to be running on. See [`remote::Tailscale::running`].
pub fn router_reading_tailscale(pool: SqlitePool, remote: remote::Tailscale) -> Router {
    routed(
        pool,
        updates::Updates::nothing_learned(),
        nothing_bound(),
        nowhere(),
        sessions::Sessions::none(),
        Gh::on_path(),
        remote,
        key::Gate::open(),
        onboarding::Machine::here(),
    )
}

/// The same, gated on `key` — which is what the half of that suite about the
/// login link stands up.
///
/// Both halves rather than one, because the two are the same fact: the link the
/// pane draws is the served address with the key on it, and **Reset key** is the
/// press that changes what every request is checked against. A router reading a
/// stated machine and holding no key could be asked neither question.
///
/// Requests to it carry the cookie, the way a browser's do — see
/// [`key::WorkbenchKey::cookie`].
pub fn router_reading_tailscale_keyed(
    pool: SqlitePool,
    remote: remote::Tailscale,
    key: key::WorkbenchKey,
) -> Router {
    routed(
        pool,
        updates::Updates::nothing_learned(),
        nothing_bound(),
        nowhere(),
        sessions::Sessions::none(),
        Gh::on_path(),
        remote,
        key::Gate::keyed(key),
        onboarding::Machine::here(),
    )
}

/// The Tailscale of a router that was not stood up to be reached from a phone:
/// the host's own binary, in front of the port the workbench takes when nobody
/// has said otherwise.
///
/// Never asked anything, in practice — nothing but the served router answers the
/// Remote access pane — and honest where it is: what a suite that wants a
/// stated answer stands up is [`router_reading_tailscale`].
fn tailnet() -> remote::Tailscale {
    remote::Tailscale::on_path(WORKBENCH_PORT)
}

/// The served router's own: the host's `tailscale` in front of the port this
/// server bound, taking the operator grant through `escalation` where whatever
/// started the process handed one over.
///
/// One place rather than two, because the two arms are one behaviour: what the
/// pane does about a refused serve is show the line, and an app that can ask for
/// the grant asks for it first — see [`remote::Elevate`].
///
/// And holding the Workbench Key, because the pane draws more than the serve:
/// the address a serve puts in front of the workbench is only half of what a
/// phone needs, and the key on the end of it is the other half — see [`key`].
fn tailnet_over(
    port: u16,
    escalation: Option<Arc<dyn remote::Elevate>>,
    key: key::WorkbenchKey,
) -> remote::Tailscale {
    let tailscale = remote::Tailscale::on_path(port).keyed(key);

    match escalation {
        Some(escalation) => tailscale.escalating(escalation),
        None => tailscale,
    }
}

/// The data directory of a router that has no use for one.
///
/// The empty path, which nothing is created in — and nothing tries: a router
/// stood up for a question about neither a Conversation nor a checkout has no
/// worktree to put anywhere.
fn nowhere() -> PathBuf {
    PathBuf::new()
}

/// The same, with the update check running against `releases` — where to ask
/// about the latest release, which is GitHub in the running server and a server
/// the test stood up itself under test. `None` is the check turned off: nothing
/// is started, and no request is ever made.
///
/// Where GitHub lives is a parameter rather than a flag on [`Config`]: the
/// address is a fact about this project, not a choice anyone running the server
/// has to make.
pub fn router_checking_updates(pool: SqlitePool, releases: Option<&str>) -> Router {
    routed(
        pool,
        updates::watching(releases),
        nothing_bound(),
        nowhere(),
        sessions::Sessions::none(),
        Gh::on_path(),
        tailnet(),
        key::Gate::open(),
        onboarding::Machine::here(),
    )
}

/// Nine, because the state a router holds is what a router is built out of:
/// each of these is one thing the served router was given and every other one
/// stands in for. A struct of them would be this list with a name on it.
#[allow(clippy::too_many_arguments)]
fn routed(
    pool: SqlitePool,
    updates: updates::Updates,
    binds: sandbox::SandboxConfig,
    data_dir: PathBuf,
    sessions: sessions::Sessions,
    github: Gh,
    remote: remote::Tailscale,
    gate: key::Gate,
    machine: onboarding::Machine,
) -> Router {
    let state = AppState {
        pool,

        // A handle on the two files rather than what is in them: they are read
        // at the moment they are wanted, so what the settings page saves reaches
        // the next session without a restart — see [`settings`].
        settings: settings::Settings::in_data_dir(&data_dir),
        nudges: nudge::Nudges::new(),
        settlements: Settlements::new(SETTLEMENT_BACKLOG),
        waits: Waits::new(),
        sessions,
        terminals: terminals::Terminals::new(),
        followers: followers::Followers::new(),
        drivers: drivers::Drivers::new(),
        updates,

        // What the installation asked every sandbox to bind, kept whole for
        // the settings page: what a session gets is this composed with whatever
        // the file holds at the moment it spawns — see [`sandbox`].
        binds,

        github,

        // And the host's `tailscale`, which is the whole of what the Remote access
        // pane reads — see [`remote`].
        remote,

        // And the key the gate below stands on, so that the one press that
        // re-issues it goes through the very handle every request is checked
        // against — see [`key::Gate::held`].
        key: gate.held(),

        // And the machine the onboarding probes are made against, held with the
        // verdict they settle at startup — see [`onboarding`].
        onboarding: onboarding::Onboarding::probing(machine),

        data_dir,
        checkouts: Arc::new(tokio::sync::Mutex::new(())),
    };

    // First of all, the worktrees directory swept of everything no Conversation
    // is working in any more. A close sweeps after itself, so what is on disk
    // unrecorded when a server comes up is what the last one never got to — and
    // nothing else is ever going to look at it. See [`worktrees::at_startup`].
    worktrees::at_startup(&state);

    // And the attachments root swept the same way, for the same reason one step
    // along: the Cleanup's delete is the one thing that takes a Conversation's
    // files, and a delete that could not have the directory deleted the rows
    // anyway. See [`attachments::at_startup`].
    attachments::at_startup(&state);

    // And the boundaries of the Conversations that have stopped, which is the
    // same sweep one platform further out: a close takes a Conversation's
    // entries with its Worktree, and a server that died took nothing at all —
    // so what is written down under the Data Directory and belongs to a
    // Conversation that has finished or closed is a set of entries on the
    // human's own directories that nothing else will ever look at. See
    // [`boundaries::at_startup`].
    boundaries::at_startup(&state);

    // Before anything is served, because it is about what was already happening
    // rather than about anything a request will start: every Conversation the
    // last server was driving is one nothing is driving now, and nobody but this
    // is going to look at any of them — see [`resume::at_startup`].
    let resumed = vec![resume::at_startup(&state)];

    // And then, once that is done, the check for the Conversations it could not
    // take up: a restart holds no driver registrations at all, so what is still
    // undriven after everything that resumes has resumed is what genuinely has
    // nobody — see [`stalls`].
    stalls::sweeping(&state, resumed);

    // And the pull requests of everything that has already finished, which is a
    // sweep of its own at a pace of its own: a base goes on moving under a
    // branch nobody is working on, and a wrap-up's watchers stop at Done. See
    // [`merges`].
    merges::sweeping(&state);

    // And the Conversations the human has archived and finished looking at,
    // which is the one sweep that takes something away rather than writing
    // something down. See [`cleanup`].
    cleanup::sweeping(&state);

    // And a listener on the one channel a Set is settled through, so that a
    // session idling on a stored ask is told its Answers have landed whether the
    // human answered from the viewer or an agent answered over the API — see
    // [`nudging::listening`]. Here rather than in either of those endpoints,
    // because a nudge sent from one and silently not from the other is a session
    // waiting for a line nobody is going to type.
    nudging::listening(&state);

    // And the verdict about the machine itself, which is the one sweep here
    // that decides something rather than tidying something: whether this
    // Verkstead has what it takes to run a session at all, reached once, now,
    // and held for the length of the run — see [`onboarding::at_startup`].
    onboarding::at_startup(&state);

    Router::new()
        // The one route that is nobody's Conversation: whether the server is up
        // is not a question about a piece of work.
        .route("/api/v1/health", get(health))
        // And the rest of the agent contract, under the Conversation the session
        // asking is running for — see [`ASKING_FROM`].
        .route(
            &format!("{ASKING_FROM}/{{conversation}}/api/v1/sets"),
            post(sets::create_set).layer(DefaultBodyLimit::max(MAX_SET_BYTES)),
        )
        .route(
            &format!("{ASKING_FROM}/{{conversation}}/api/v1/sets/{{id}}/response"),
            post(responses::submit_response).get(responses::wait_for_response),
        )
        // The viewer's half. It shares this state rather than holding its own:
        // a submit or a locking from the browser has to reach an agent
        // waiting on the endpoint above, and both halves have to agree about
        // which Sets a wait is being held on.
        //
        // And it is behind the gate, where the two routes above are not: this
        // namespace is the human's browser asking about everybody's work, and a
        // session reaching the loopback must not be able to ask any of it. The
        // first of the gate's two attachments — the other is over the fallback
        // that answers every page of the workbench, which is put on in
        // [`router_with_ui`]. See [`key`].
        .merge(gate.guarding(ui::routes()))
        .with_state(state)
}

async fn health() -> &'static str {
    "ok"
}

/// Everything the one binary serves: the API above, plus the viewer built into
/// it on every other path.
///
/// The viewer takes the fallback, so `/api/v1/` and `/api/ui/` keep their exact
/// paths and everything else — the document, the bundles, the app shell's own
/// files — is [`viewer`]'s to answer.
///
/// This is also the only router that checks for updates, because it is the only
/// one with a viewer to draw the Notice in — see [`router_checking_updates`] for
/// what `releases` is.
///
/// And the only one that is keyed, because it is the only one anybody is served
/// by: `key` is the Workbench Key this Data Directory holds, and the gate over
/// the viewer's namespace and over the fallback is what a session reaching the
/// loopback finds instead of the workbench — see [`key`].
///
/// `remote` is the host's `tailscale` in front of the port this router is being
/// served on, which is the one thing here that has to be told where the server
/// is listening: a serve is this workbench's when it proxies to that port — see
/// [`remote`].
pub fn router_with_ui(
    pool: SqlitePool,
    releases: Option<&str>,
    data_dir: PathBuf,
    agents: Agents,
    gh: Gh,
    remote: remote::Tailscale,
    key: key::WorkbenchKey,
) -> Router {
    // Off the agents, for the reason [`router_running_sessions`] takes it off
    // them: one configured set, said once.
    let binds = agents.binds().clone();
    let gate = key::Gate::keyed(key);

    routed(
        pool,
        updates::watching(releases),
        binds,
        data_dir,
        sessions::Sessions::under(agents),
        gh,
        remote,
        gate.clone(),
        onboarding::Machine::here(),
    )
    .fallback_service(guarded_viewer::<viewer::Built>(&gate))
}

/// The same, over a site named by the caller, which is how the tests ask what the
/// server does with one without waiting on `pnpm build` to produce it.
pub fn router_with_viewer<V: Embed + 'static>(pool: SqlitePool) -> Router {
    router(pool).fallback(viewer::serve::<V>)
}

/// [`router`], keyed: what the suite that asks about the gate itself stands up.
///
/// A constructor of its own rather than a flag on the others, because the others
/// are what several hundred requests across the suites are built on and every
/// one of them assumes an answer rather than a 401 — see [`key::Gate`].
pub fn router_keyed(pool: SqlitePool, key: key::WorkbenchKey) -> Router {
    routed(
        pool,
        updates::Updates::nothing_learned(),
        nothing_bound(),
        nowhere(),
        sessions::Sessions::none(),
        Gh::on_path(),
        tailnet().keyed(key.clone()),
        key::Gate::keyed(key),
        onboarding::Machine::here(),
    )
}

/// And the same with a site behind it, which is what the workbench's own pages
/// are asked for through: the gate's second attachment is over the fallback, so
/// a suite asking whether a page is gated needs a router that has one.
pub fn router_keyed_with_viewer<V: Embed + 'static>(
    pool: SqlitePool,
    key: key::WorkbenchKey,
) -> Router {
    let gate = key::Gate::keyed(key.clone());

    router_keyed(pool, key).fallback_service(guarded_viewer::<V>(&gate))
}

/// The viewer's fallback with the gate over it.
///
/// A router of its own rather than a layer on the outer one: the two routes the
/// gate is not over — the health check and a session's own Conversation-scoped
/// API — are on the outer router, and a layer there would cover them too.
fn guarded_viewer<V: Embed + 'static>(gate: &key::Gate) -> Router {
    gate.guarding(Router::new().fallback(viewer::serve::<V>))
}

/// Take the address, open the database, and serve until the process is stopped.
///
/// **The socket is taken before anything is made.** Everything below makes
/// something — the Data Directory, the Skills written into it, the Build Cache,
/// the database — and an address somebody else is already listening on is no
/// reason to have made any of it. So the bind is the first thing that can fail,
/// and a second Verkstead is refused by the socket rather than after it has
/// written over the first one's directory.
///
/// [`run_on`] is the same thing on a socket the caller bound, which is where the
/// desktop binary starts: a taken address is the one failure it draws a dialog
/// for, and a dialog wants the failure before the side effects rather than after
/// them.
pub async fn run(config: Config) -> Result<()> {
    let listener = std::net::TcpListener::bind(config.listen)
        .with_context(|| format!("binding {}", config.listen))?;

    run_on(listener, config).await
}

/// The same, on a socket that is already bound.
///
/// The listener is the standard library's rather than tokio's, because a caller
/// that has one bound it before there was a runtime to bind it on — see [`run`]
/// for why the address is settled first.
///
/// A bare `verkstead serve` is configured by nobody and comes up all the same:
/// there is nothing here that has to be said before the server can be reached,
/// and everything that *was* said is resolved before it is served over.
pub async fn run_on(listener: std::net::TcpListener, config: Config) -> Result<()> {
    // The Data Directory resolved and made, and the key in it read or written,
    // before anything else this start does — see [`Config::workbench_key`], and
    // [`run_on_keyed`] for the caller that arrives having already made this call.
    let key = config.workbench_key()?;

    // And nothing to escalate with: a server started this way was started from a
    // shell or a unit file, where there is nobody at the machine to put a
    // password dialog in front of — see [`remote::Elevate`].
    run_on_keyed(listener, config, key, None).await
}

/// The same again, with the Workbench Key already in hand — and with whatever
/// way of escalating the caller has.
///
/// Which is the desktop app's way in. The browser it opens is opened on the
/// login link, so it has to hold the key before there is a server to ask one of
/// — and what it hands over here is therefore the key the workbench is gated on,
/// by being the same handle rather than by being read a second time. See
/// `verkstead_desktop::Desktop::run`.
///
/// `escalation` is the other thing only that caller has: the operator grant the
/// Remote access pane asks for is a command run with a privilege this process
/// has not got, and the desktop app can ask the platform for one where a daemon
/// cannot. `None` is every other way in, and is what the pane behaved as before
/// there was an app to hand one over — see [`remote::Elevate`].
pub async fn run_on_keyed(
    listener: std::net::TcpListener,
    config: Config,
    key: key::WorkbenchKey,
    escalation: Option<Arc<dyn remote::Elevate>>,
) -> Result<()> {
    // Resolved at startup: a bind that names nothing, and a HOME the unit never
    // said, are misconfigurations to report now rather than sessions that fail
    // to start weeks later with nobody watching. The home is where a sandbox
    // reads who git commits as, and it is what `~` means inside one, so a server
    // without one can run no session at all.
    let binds = sandbox::SandboxConfig::resolve(&config.sandbox_binds)?;

    // The same directory the key was kept in, resolved again rather than
    // threaded through: what it comes to is the flag or the platform's own place
    // for it, and neither of those changes between two calls a moment apart.
    // Where the flag said nothing this is the platform's own directory — see
    // [`platform::data_dir`] — so the startup line below is the only place a
    // human finds out which one that turned out to be.
    let data_dir = config.data_directory()?;

    // And where a session's HOME comes from, which wants the Data Directory
    // above on the platform that makes a real one under it — see
    // [`sandbox::Homes`]. Refused for the reason the binds are: a HOME the unit
    // never said is a misconfiguration to report now rather than a session that
    // fails to start weeks later with nobody watching.
    let homes = sandbox::Homes::of_the_server(&data_dir).with_context(|| {
        format!(
            "no {} is set: a session's `~` is the home directory of whoever runs Verkstead, \
             and the machine's git identity is read out of it, so whatever starts the \
             server has to say what it is",
            platform::home_variable(platform::Platform::HERE),
        )
    })?;

    // And the skills written out into it, before anything can ask for a session:
    // they are what a grilling session is pointed at, and this binary's are what
    // every sandbox gets, whatever an earlier one left there.
    let skills = skills::Skills::installed(platform::Platform::HERE, &data_dir)
        .context("installing the skills every sandbox is given")?;

    // And the shared build cache, which is resolved for the reason the binds
    // above are and *made* here, which they never are — see
    // [`build_cache::BuildCache::resolve`] for why this one directory is
    // Verkstead's to create. After the Data Directory, because it wants the
    // Worktrees directory inside it: that is what the shared compile server is
    // given, and a bind of nothing will not start. An sccache that could not be
    // found is not a failure: what is left still shares the downloads, and the
    // log line says so.
    let cache = build_cache::BuildCache::resolve(config.build_cache_dir.as_deref(), &data_dir)?;

    // And the executable every sandbox asks with, which is this one: `verkstead
    // serve` and `verkstead ask` are two verbs of one binary, so a session's CLI
    // is the running server's own build and cannot disagree with it about a
    // schema, a Guide or a wire format — see [`sandbox::Executable`].
    //
    // Found and then *run*, here and once: a `guide` in the environment a
    // session would get, which is what says the file will run for somebody
    // other than whatever launched this process — see [`Executable::probed`],
    // and the AppImage it is written for.
    //
    // Not a reason to refuse to start, unlike the two above. A server with no
    // image to hand over has nothing to equip a session with, and which session
    // that costs is the thing worth reporting — so *which* is said as one is
    // started rather than here, where there is nothing to name.
    //
    // And *why* is said here, by whichever of the two steps found out: neither
    // an image that could not be found nor one that will not run leaves anything
    // to look at by the time a session is refused, so each says so as it
    // happens. See [`Executable::of_the_server`] and [`Executable::probed`].
    let verkstead =
        sandbox::Executable::of_the_server(&data_dir).and_then(sandbox::Executable::probed);

    // And where a Conversation's handoff document is written, which is a root
    // under the same directory: each Conversation's own is made as its first
    // session starts.
    let handoffs = handoffs::Handoffs::under(&data_dir);

    // And where the files the human attaches to a Conversation are kept, which
    // is a root under the same directory again: each Conversation's own is made
    // as its first file lands in it, and read-only inside every session it has
    // after that — see [`attachments`].
    let attachments = attachments::Attachments::under(&data_dir);

    // And where the credentials are read from, which is the same directory
    // again — both the ones a session runs with and the one the server's own
    // `gh` authenticates as. Nothing is read here: the files are read as each
    // session is spawned and as each `gh` is run, so what the human saves
    // through the settings page applies without a restart — see [`settings`].
    let settings = settings::Settings::in_data_dir(&data_dir);

    // With one exception, read here and held for the run: the directories
    // Verkstead has installed into, which a session's `PATH` leads with. It is
    // a startup value because the `PATH` it composes with is one — see
    // [`sandbox::machine_path`] — and the one thing that moves it afterwards is
    // an install landing in a directory, which appends to both the held list and
    // the file at once. Before the router below, whose probes read what this
    // held.
    sandbox::hold_session_path(&settings);

    let pool = open_database(&database(&data_dir)).await?;

    listener
        .set_nonblocking(true)
        .context("putting the listening socket into the mode the runtime reads it in")?;
    let listener = tokio::net::TcpListener::from_std(listener)
        .context("handing the listening socket to the runtime")?;

    // The syntax definitions built on a blocking thread while the server comes
    // up, rather than under the first Diff somebody opens. Nothing waits on it:
    // it is spawned and left, so serving starts when the bind does, and a
    // request that arrives before it finishes simply waits where it would have
    // waited anyway.
    tokio::task::spawn_blocking(verkstead_render::warm_highlighter);

    // And the pipe beside the socket, which is what a sandboxed Windows session
    // asks through. Here rather than with the bind, because its name comes off
    // the Data Directory — see [`pipe`] — and that is only settled above.
    //
    // Granting the account this installation's sessions run as, beside the one
    // this server runs as — and nobody where there is no such account, which is
    // a machine the elevated verb has never been run on and is a machine that
    // will refuse every session anyway. There is one account for the whole
    // installation, so this is settled here rather than added to later; see
    // [`pipe`], where the whole of that is.
    #[cfg(windows)]
    let session_account = sandbox::account::machine::sid_of(homes.session_account());

    #[cfg(windows)]
    if let Err(why) = &session_account {
        tracing::warn!(
            account = homes.session_account(),
            why,
            "there is no local account for this Data Directory to run sessions as, so the \
             named pipe is granting nobody beyond this server — and no session will start \
             until `{}` has been run from an elevated terminal",
            sandbox::account::MAKE_IT,
        );
    }

    #[cfg(windows)]
    let pipe = pipe::Listener::open(
        &data_dir,
        session_account.as_ref().ok().map(|sid| sid.text()),
    )
    .context("opening the named pipe a Windows session asks through")?;

    // The one line an operator reads as Verkstead comes up, and so the daemon's
    // whole way of handing the login link over: the address with the key on it,
    // which is what a browser has to be pointed at to be let in at all
    // (ADR-0015). A machine started from a unit file has no tray to press Open
    // in, and this is what somebody reading the journal can paste.
    //
    // The secret is in the log, therefore, and that is the point of it. The
    // journal is read by whoever the machine lets read it, which is where every
    // other credential this server was started with is too — and re-issuing the
    // key is what takes a link back off somebody who has read one.
    tracing::info!(
        listen = %config.listen,
        workbench = %key::login_link(config.listen, &key),
        data_dir = %data_dir.display(),
        update_check = config.releases().is_some(),
        home = %homes.servers().display(),
        sandbox_binds = binds.count(),
        build_cache = ?cache.dir(),
        caches_compiles = cache.caches_compiles(),
        skills = %skills.path().display(),
        verkstead = ?verkstead.as_ref().map(sandbox::Executable::path),
        "verkstead is listening",
    );

    // The pipe on a line of its own rather than as a field on the one above:
    // the other platforms have no pipe, and a field saying so at every startup
    // there would be a line about nothing. In the spelling a client is given
    // rather than the one Win32 takes, so that a human can paste what they read
    // straight into `--server` — see `pipe::Listener::asked_through`.
    #[cfg(windows)]
    tracing::info!(pipe = %pipe.asked_through(), "verkstead is listening on a named pipe too");

    // Where a session is told Verkstead is: the socket everywhere, and the pipe
    // beside it on the platform that opened one — a Windows session is headed
    // for a container refused the loopback interface, and the pipe is what an
    // identity can be granted instead. Which of the two a session is handed is
    // [`sandbox::Reachable`]'s, off the Platform it runs on.
    let reachable = sandbox::Reachable::at(config.listen);

    #[cfg(windows)]
    let reachable = reachable.piped(pipe.asked_through());

    let app = router_with_ui(
        pool,
        config.releases(),
        data_dir,
        Agents::new(
            homes,
            reachable,
            binds,
            cache,
            skills,
            verkstead,
            handoffs,
            attachments,
            settings.clone(),
        ),
        // Whatever `gh` this machine has, authenticating as the configured
        // token — the same one the sessions get, so one token is the whole
        // of Verkstead's GitHub auth.
        Gh::on_path().authenticated_by(settings),
        // And whatever `tailscale` it has, asked about the port this server just
        // bound: a serve is this workbench's when it proxies there, and an
        // install told to listen somewhere else is one whose serve has to point
        // somewhere else too — see [`remote`]. With whatever this process was
        // started with a way to escalate through, where it was started with one.
        tailnet_over(config.listen.port(), escalation, key.clone()),
        // And the key this Data Directory holds, which is what the workbench
        // and the viewer's own namespace are behind.
        key,
    );

    // Two listeners over one router here, so that everything a request can ask
    // for over the socket it can ask for over the pipe. Either one ending is
    // the server ending: there is no graceful shutdown — the process stopping
    // is the whole of stopping — so a half that has stopped answering is a
    // Verkstead that has stopped serving.
    #[cfg(windows)]
    {
        tokio::select! {
            served = axum::serve(listener, app.clone()) => served.context("serving Verkstead"),
            served = axum::serve(pipe, app) => served.context("serving Verkstead over its named pipe"),
        }
    }

    // And the socket on its own everywhere else, there being no pipe to serve.
    #[cfg(not(windows))]
    {
        axum::serve(listener, app)
            .await
            .context("serving Verkstead")
    }
}
