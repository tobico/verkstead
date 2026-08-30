//! The Verkstead server: the agents' HTTP API and the human's web UI, over one
//! SQLite store and out of one binary.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result};
use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::routing::{get, post};
use sqlx::SqlitePool;
use verkstead_store::{Settlements, Waits};

/// The shared Rust build cache every sandbox is given: where it is, and whether
/// there is an sccache to compile through.
///
/// Public for the reason [`sandbox`] is — what a session builds into is part of
/// the surface it runs on rather than an implementation detail of an endpoint,
/// and standing a router up that runs sessions means saying where it is.
pub mod build_cache;
mod capture;
mod checklist;
mod checks;
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
mod limits;
mod nudge;
mod profiles;
/// Putting a share where a link reaches it, which is Verkstead's own write to
/// GitHub.
mod publishing;
mod push;
/// Following a Conversation's branch to the name a session renamed it to,
/// rather than repairing a checkout that has not come adrift after all.
mod renames;
mod reply;
mod repos;
/// Speaking to a session that has gone idle without asking anything.
mod rescues;
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
mod transcript;
mod ui;
mod updates;
mod viewer;
mod watched;
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

/// The security boundary every filesystem path is decided against. Public
/// because starting the server is choosing what it may touch, and a caller
/// standing up a router has to say so.
pub use watched::{Admission, WatchedPaths};

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

    /// The watcher each Conversation's latest pick armed — see [`followers`].
    /// Beside the sessions rather than inside them, because a watcher is a task
    /// of Verkstead's own and a session is an agent's process.
    followers: followers::Followers,

    /// And what is driving each of them, which is the other half of the same
    /// question: a session is one agent running, and a driver is the task that
    /// keeps starting them — see [`drivers`].
    drivers: drivers::Drivers,

    updates: updates::Updates,
    watched: WatchedPaths,

    /// How Verkstead itself asks GitHub about a pull request — the host's `gh`,
    /// authenticating as the configured token.
    github: Gh,

    /// The two files the human tells Verkstead their credentials and their
    /// identity in. A handle rather than what is in them: the files are read at
    /// the moment they are wanted, so the settings page and the next session to
    /// spawn see the same thing — see [`settings`].
    settings: settings::Settings,

    /// Where Verkstead keeps what it makes — the worktrees, for now — which is
    /// not a Watched Path and is not meant to be: the Watched Paths bound what
    /// the human may point Verkstead at, and this is the directory Verkstead was
    /// given for its own things.
    data_dir: PathBuf,
}

/// The one name the database is ever kept under, inside the Data Directory.
/// Fixed rather than configurable: the directory is what an operator points
/// Verkstead at, and a file inside it is Verkstead's own business.
const DATABASE_NAME: &str = "verkstead.db";

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
    /// This is the Data Directory. Not a Watched Path and not one to point at a
    /// directory the human works in: the Watched Paths bound what Verkstead may
    /// be pointed at, and this is Verkstead's own. It defaults to the working
    /// directory, which is what running the server out of a checkout means
    /// everywhere else here.
    #[arg(
        long,
        env = "VERKSTEAD_DATA_DIR",
        default_value = ".",
        value_name = "DIR"
    )]
    pub data_dir: PathBuf,

    /// Address and port to bind. Bind a tailnet address to reach the server
    /// from other devices.
    #[arg(long, env = "VERKSTEAD_LISTEN", default_value = "127.0.0.1:8422")]
    pub listen: SocketAddr,

    /// A directory Verkstead may operate inside. Repeat the flag, or separate
    /// several with `:` in the environment variable, as `PATH` is written.
    ///
    /// This is a security boundary and not a convenience: nothing outside these
    /// directories is ever touched, and a Repo is registered only from within
    /// one. There is no default and no scan — guessing at what a machine's owner
    /// meant to expose is not a guess worth making.
    ///
    /// Nor is there a requirement. The workbench settings say Watched Paths too,
    /// and the boundary is the union of the two — see [`WatchedPaths`] — so a
    /// standalone install comes up with none of these, admits nothing at all,
    /// and is pointed at its first directory from its own settings page. A
    /// service unit goes on saying them here, where a directory that is not
    /// there still refuses to start.
    #[arg(
        long = "watched-path",
        env = "VERKSTEAD_WATCHED_PATHS",
        value_delimiter = ':',
        value_name = "DIR"
    )]
    pub watched_paths: Vec<PathBuf>,

    /// An extra read-write bind every sandbox gets, or `name=DIR` for one only
    /// the Repo registered under that name gets. Repeat the flag, or separate
    /// several with `:` in the environment variable.
    ///
    /// This is the Sandbox Configuration: the package registries and the caches
    /// a session needs beyond its own worktree that Verkstead does not provide
    /// itself. Each names a directory of somebody else's and is a hole in the
    /// boundary a sandbox is, which is why they are configured here beside the
    /// Watched Paths rather than anywhere a session or a browser could reach —
    /// and why a bind that is not there refuses startup rather than being
    /// skipped.
    ///
    /// A Rust build cache is not one of them: the server provides that one — see
    /// `--build-cache-dir` — and the switch that turns it off is in the
    /// workbench settings.
    #[arg(
        long = "sandbox-bind",
        env = "VERKSTEAD_SANDBOX_BINDS",
        value_delimiter = ':',
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
    /// Unlike a Sandbox Configuration bind this is not a hole the installer
    /// opened in the boundary: it is the server's own directory, holding
    /// nothing but build output, and the switch that closes it is in the
    /// workbench settings beside the size it may grow to.
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

    /// The SQLite file, which is [`DATABASE_NAME`] inside the Data Directory
    /// and is never anywhere else: one directory is what an operator says, and
    /// everything in it is Verkstead's to name.
    pub fn database(&self) -> PathBuf {
        self.data_dir.join(DATABASE_NAME)
    }
}

/// Everything the server answers in a serialised format: the agents' contract
/// under `/api/v1/`, and the viewer's own namespace under `/api/ui/`.
///
/// Both live under `/api/`, which is also the one prefix the viewer's fallback
/// refuses to answer with the document — see [`viewer`].
///
/// Watching nothing, which is the closed state: no path is inside a Watched
/// Path, so no Repo can be registered. That is what everything but the server
/// itself and the Repo tests wants — see [`router_watching`] for the other one.
pub fn router(pool: SqlitePool) -> Router {
    routed(
        pool,
        updates::Updates::nothing_learned(),
        WatchedPaths::none(),
        nowhere(),
        sessions::Sessions::none(),
        Gh::on_path(),
    )
}

/// The same, permitted inside `watched` — the directories a Repo may be
/// registered from — and keeping what it makes in `data_dir`.
///
/// It runs no sessions: starting a grilling makes the branch and the worktree
/// and records that it did, and there is nothing here to launch inside them.
/// See [`router_running_sessions`] for the one that does.
pub fn router_watching(pool: SqlitePool, watched: WatchedPaths, data_dir: PathBuf) -> Router {
    routed(
        pool,
        updates::Updates::nothing_learned(),
        watched,
        data_dir,
        sessions::Sessions::none(),
        Gh::on_path(),
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
    watched: WatchedPaths,
    data_dir: PathBuf,
    agents: Agents,
    gh: Gh,
) -> Router {
    routed(
        pool,
        updates::Updates::nothing_learned(),
        watched,
        data_dir,
        sessions::Sessions::under(agents),
        gh,
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
        WatchedPaths::none(),
        data_dir,
        sessions::Sessions::none(),
        gh,
    )
}

/// The data directory of a router that has no use for one.
///
/// The empty path, which nothing is created in — and nothing tries: a router
/// watching nothing can register no Repo, so it has no Conversation to start and
/// no worktree to put anywhere.
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
        WatchedPaths::none(),
        nowhere(),
        sessions::Sessions::none(),
        Gh::on_path(),
    )
}

fn routed(
    pool: SqlitePool,
    updates: updates::Updates,
    watched: WatchedPaths,
    data_dir: PathBuf,
    sessions: sessions::Sessions,
    github: Gh,
) -> Router {
    let settings = settings::Settings::in_data_dir(&data_dir);

    let state = AppState {
        pool,
        settings: settings.clone(),
        nudges: nudge::Nudges::new(),
        settlements: Settlements::new(SETTLEMENT_BACKLOG),
        waits: Waits::new(),
        sessions,
        followers: followers::Followers::new(),
        drivers: drivers::Drivers::new(),
        updates,

        // The boundary the installation drew, widened by whatever the human has
        // put in `config.yaml` — read at each admission rather than here, so a
        // directory added on the settings page admits from the next request on.
        watched: watched.reading(settings),

        github,
        data_dir,
    };

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
        .merge(ui::routes())
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
pub fn router_with_ui(
    pool: SqlitePool,
    releases: Option<&str>,
    watched: WatchedPaths,
    data_dir: PathBuf,
    agents: Agents,
    gh: Gh,
) -> Router {
    routed(
        pool,
        updates::watching(releases),
        watched,
        data_dir,
        sessions::Sessions::under(agents),
        gh,
    )
    .fallback(viewer::serve::<viewer::Built>)
}

/// The same, over a site named by the caller, which is how the tests ask what the
/// server does with one without waiting on `pnpm build` to produce it.
pub fn router_with_viewer<V: Embed + 'static>(pool: SqlitePool) -> Router {
    router(pool).fallback(viewer::serve::<V>)
}

/// Open the database and serve until the process is stopped.
///
/// The installation's Watched Paths are resolved before anything else: a
/// directory that is not there is a misconfiguration to report at startup,
/// where it can be fixed, rather than one to discover as a refusal weeks later.
/// Being given none of them is not a misconfiguration — the settings file says
/// Watched Paths too, and a standalone install starts with nothing configured
/// anywhere and admits nothing until it is.
pub async fn run(config: Config) -> Result<()> {
    let watched = WatchedPaths::resolve(&config.watched_paths)?;

    // Both resolved at startup for the reason the Watched Paths are: a bind that
    // names nothing, and a HOME the unit never said, are misconfigurations to
    // report now rather than sessions that fail to start weeks later with nobody
    // watching. The home is where a sandbox reads who git commits as, and it is
    // what `~` means inside one, so a server without one can run no session at
    // all.
    let binds = sandbox::SandboxConfig::resolve(&config.sandbox_binds)?;

    let home = sandbox::Home::of_the_server().context(
        "no HOME is set: a session's `~` is the home directory of whoever runs Verkstead, \
         and the machine's git identity is read out of it, so the unit has to say what it is",
    )?;

    // Made at startup for the reason the Watched Paths are resolved at startup:
    // a directory Verkstead cannot write to is a misconfiguration to report now
    // rather than one to discover as a failed grilling weeks later.
    let data_dir = config.data_dir.clone();
    std::fs::create_dir_all(&data_dir)
        .with_context(|| format!("creating data directory {}", data_dir.display()))?;

    // And the skills written out into it, before anything can ask for a session:
    // they are what a grilling session is pointed at, and this binary's are what
    // every sandbox gets, whatever an earlier one left there.
    let skills = skills::Skills::installed(&data_dir)
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
    // Not a reason to refuse to start, unlike the two above. A server that
    // cannot say what it is running has nothing to equip a session with, and
    // which session that costs is the thing worth reporting — so it is said as
    // one is started rather than here, where there is nothing to name.
    let verkstead = sandbox::Executable::of_the_server();

    // And where a Conversation's handoff document is written, which is a root
    // under the same directory: each Conversation's own is made as its first
    // session starts.
    let handoffs = handoffs::Handoffs::under(&data_dir);

    // And where the credentials are read from, which is the same directory
    // again — both the ones a session runs with and the one the server's own
    // `gh` authenticates as. Nothing is read here: the files are read as each
    // session is spawned and as each `gh` is run, so what the human saves
    // through the settings page applies without a restart — see [`settings`].
    let settings = settings::Settings::in_data_dir(&data_dir);

    let pool = open_database(&config.database()).await?;

    let listener = tokio::net::TcpListener::bind(config.listen)
        .await
        .with_context(|| format!("binding {}", config.listen))?;

    // The syntax definitions built on a blocking thread while the server comes
    // up, rather than under the first Diff somebody opens. Nothing waits on it:
    // it is spawned and left, so serving starts when the bind does, and a
    // request that arrives before it finishes simply waits where it would have
    // waited anyway.
    tokio::task::spawn_blocking(verkstead_render::warm_highlighter);

    tracing::info!(
        listen = %config.listen,
        data_dir = %data_dir.display(),
        update_check = config.releases().is_some(),
        watched = ?watched.paths(),
        settings_watched = ?settings.config().watched_paths(),
        home = %home.path.display(),
        sandbox_binds = binds.count(),
        build_cache = ?cache.dir(),
        caches_compiles = cache.caches_compiles(),
        skills = %skills.path().display(),
        verkstead = ?verkstead.as_ref().map(sandbox::Executable::path),
        "verkstead is listening",
    );

    axum::serve(
        listener,
        router_with_ui(
            pool,
            config.releases(),
            watched,
            data_dir,
            Agents::new(
                home,
                sandbox::Reachable::at(config.listen),
                binds,
                cache,
                skills,
                verkstead,
                handoffs,
                settings.clone(),
            ),
            // Whatever `gh` this machine has, authenticating as the configured
            // token — the same one the sessions get, so one token is the whole
            // of Verkstead's GitHub auth.
            Gh::on_path().authenticated_by(settings),
        ),
    )
    .await
    .context("serving Verkstead")
}
