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

mod conversations;
mod nudge;
mod push;
mod reply;
mod repos;
mod responses;
mod sets;
mod ui;
mod updates;
mod viewer;
mod watched;

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

/// The longest a client may ask to have a wait held open. There is no expiry
/// on the waiting itself — the client owns retry (ADR-0001), so it picks the
/// hold length and the server only bounds it.
const MAX_HOLD: Duration = Duration::from_secs(60);

/// How many settlements a held wait can fall behind before it gives up
/// following along and goes back to the store instead. One notification per
/// Set settled, for a single human settling them: this is generous.
const SETTLEMENT_BACKLOG: usize = 64;

/// What the handlers share: the store, word of Sets that have just arrived or
/// have just been settled — so held waits need not poll for the one and open
/// pages hear about both — which Sets a wait is being held on, whether a newer
/// Verkstead has been released than this one, and which directories any of it
/// may touch.
#[derive(Clone)]
pub(crate) struct AppState {
    pool: SqlitePool,
    creations: nudge::Creations,
    settlements: Settlements,
    waits: Waits,
    updates: updates::Updates,
    watched: WatchedPaths,
}

/// How the server is pointed at its database and its socket. There is no
/// app-level auth: the tailnet is the perimeter, so the defaults keep the
/// server on the loopback interface until told otherwise.
#[derive(Debug, Clone, clap::Parser)]
#[command(name = "verkstead serve", version, about = "Verkstead server")]
pub struct Config {
    /// Path to the SQLite database. Created, with its parent directory, if
    /// it does not exist.
    #[arg(long, env = "VERKSTEAD_DATABASE", default_value = "verkstead.db")]
    pub database: PathBuf,

    /// Address and port to bind. Bind a tailnet address to reach the server
    /// from other devices.
    #[arg(long, env = "VERKSTEAD_LISTEN", default_value = "127.0.0.1:8422")]
    pub listen: SocketAddr,

    /// A directory Verkstead may operate inside. Repeat the flag, or separate
    /// several with `:` in the environment variable, as `PATH` is written.
    ///
    /// This is a security boundary and not a convenience: nothing outside these
    /// directories is ever touched, and a Repo is registered only from within
    /// one. There is no default and no scan — the server refuses to start
    /// without at least one, because guessing at what a machine's owner meant to
    /// expose is not a guess worth making.
    #[arg(
        long = "watched-path",
        env = "VERKSTEAD_WATCHED_PATHS",
        value_delimiter = ':',
        required = true,
        value_name = "DIR"
    )]
    pub watched_paths: Vec<PathBuf>,

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
    )
}

/// The same, permitted inside `watched` — the directories a Repo may be
/// registered from.
pub fn router_watching(pool: SqlitePool, watched: WatchedPaths) -> Router {
    routed(pool, updates::Updates::nothing_learned(), watched)
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
    routed(pool, updates::watching(releases), WatchedPaths::none())
}

fn routed(pool: SqlitePool, updates: updates::Updates, watched: WatchedPaths) -> Router {
    Router::new()
        .route("/api/v1/health", get(health))
        .route(
            "/api/v1/sets",
            post(sets::create_set).layer(DefaultBodyLimit::max(MAX_SET_BYTES)),
        )
        .route(
            "/api/v1/sets/{id}/response",
            post(responses::submit_response).get(responses::wait_for_response),
        )
        // The viewer's half. It shares this state rather than holding its own:
        // a submit or an archiving from the browser has to reach an agent
        // waiting on the endpoint above, and both halves have to agree about
        // which Sets a wait is being held on.
        .merge(ui::routes())
        .with_state(AppState {
            pool,
            creations: nudge::Creations::new(),
            settlements: Settlements::new(SETTLEMENT_BACKLOG),
            waits: Waits::new(),
            updates,
            watched,
        })
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
pub fn router_with_ui(pool: SqlitePool, releases: Option<&str>, watched: WatchedPaths) -> Router {
    routed(pool, updates::watching(releases), watched).fallback(viewer::serve::<viewer::Built>)
}

/// The same, over a site named by the caller, which is how the tests ask what the
/// server does with one without waiting on `pnpm build` to produce it.
pub fn router_with_viewer<V: Embed + 'static>(pool: SqlitePool) -> Router {
    router(pool).fallback(viewer::serve::<V>)
}

/// Open the database and serve until the process is stopped.
///
/// The Watched Paths are resolved before anything else: a server that cannot say
/// what it is permitted to touch has no business coming up, and a directory that
/// is not there is a misconfiguration to report at startup rather than one to
/// discover as a refusal weeks later.
pub async fn run(config: Config) -> Result<()> {
    let watched = WatchedPaths::resolve(&config.watched_paths)?;

    let pool = open_database(&config.database).await?;

    let listener = tokio::net::TcpListener::bind(config.listen)
        .await
        .with_context(|| format!("binding {}", config.listen))?;

    tracing::info!(
        listen = %config.listen,
        database = %config.database.display(),
        update_check = config.releases().is_some(),
        watched = ?watched.paths(),
        "verkstead is listening",
    );

    axum::serve(listener, router_with_ui(pool, config.releases(), watched))
        .await
        .context("serving Verkstead")
}
