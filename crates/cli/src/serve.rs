//! `verkstead serve` — the server, run out of the binary the agent already has.
//!
//! One binary carries both halves (ADR-0004), so this is where the two parts
//! the server needs and no other verb does are assembled: the async runtime,
//! and somewhere for the server's `tracing` to go.
//!
//! **And the one flag that is this verb's rather than the server's.** The
//! desktop app starts this very binary as its sidecar (ADR-0020), and
//! `--desktop` is how it says so — a fact about who is at the machine rather
//! than a setting, which is why it sits here with the server's own
//! configuration flattened beneath it instead of on that configuration. The
//! same `Config` is flattened under `verkstead desktop`, where a flag saying
//! the caller is the app would be a flag saying nothing.

use anyhow::{Context, Result};
use tracing_subscriber::EnvFilter;
use verkstead_server::{Config, StartedBy};

/// What the server logs when `RUST_LOG` says nothing: its own startup line and
/// whatever else it has to report, and nothing from the crates beneath it.
const DEFAULT_FILTER: &str = "verkstead_server=info";

/// How the server is started from a command line: who started it, and the
/// server's own configuration under that.
///
/// [`clap::Args`] rather than [`clap::Parser`], because this is what a verb
/// takes rather than what a binary is: what names the command and describes it
/// is this crate's own enum — see `Command::Serve`.
#[derive(Debug, clap::Args)]
pub struct Serve {
    /// Run as the desktop app's sidecar: the server the app starts beside
    /// itself, told who started it.
    ///
    /// **The desktop app's flag and nobody else's** — there is nothing here for
    /// a unit file or a shell to want.
    ///
    /// The same server in every other respect — the same API, the same
    /// workbench, the same Data Directory. What changes is the one line an
    /// operator reads as Verkstead comes up: it names the workbench address
    /// alone rather than the login link, because the app has read the
    /// **Workbench Key** out of the Data Directory and opened its own window on
    /// that link already, and the log under the app is a file on somebody's
    /// desk that a menu item opens (ADR-0015, as amended).
    ///
    /// The key is not hidden, only kept off that line: it stays in
    /// `workbench.key` inside the Data Directory the line names, at the mode it
    /// has always had. A `--desktop` run started by hand is read from there.
    ///
    /// And where this machine has nowhere to draw a window at all — over SSH,
    /// in a container — the line carries the whole login link after all: an app
    /// that could not have started is an app that opened no window on one.
    #[arg(long, action = clap::ArgAction::SetTrue)]
    pub desktop: bool,

    #[command(flatten)]
    pub server: Config,
}

impl Serve {
    /// Who started this, as the server is told it: the whole of what the flag
    /// says, and what the server reads to do anything differently at all.
    fn started_by(&self) -> StartedBy {
        match self.desktop {
            true => StartedBy::TheDesktopApp,
            false => StartedBy::AnOperator,
        }
    }
}

/// Serve until the process is stopped.
///
/// The runtime is built here rather than around `main`: `ask` and `guide` are
/// blocking calls, and an agent running one should not be paying for threads
/// nothing is going to use.
pub fn serve(asked: Serve) -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| DEFAULT_FILTER.into()),
        )
        .init();

    let started_by = asked.started_by();

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("starting the async runtime")?
        .block_on(verkstead_server::run(asked.server, started_by))
}
