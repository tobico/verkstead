//! `verkstead` — the command coding agents run to put a Question Set to the
//! human and block until it is answered.
//!
//! The CLI is the agent-facing compatibility surface (ADR-0001): agents run it
//! as a background shell command, so the wait outlives any harness tool
//! timeout, and nothing but a shell is needed to integrate. It also derives
//! `project` and `branch` from the working directory, so neither is ever at the
//! mercy of what an agent claims. The Diff is the same rule answered by the
//! other end: the server reads it off the Worktrees the Set was asked from.
//!
//! **And it is the only binary** (ADR-0004, and ADR-0012 as amended). `serve`
//! runs the server out of the same file the agent asks with, and `desktop` runs
//! that server with a tray icon over it — so the image a sandboxed session is
//! handed is the image its server is running, and the two halves of an ask
//! cannot skew. The tray half is a default-on `desktop` cargo feature, and the
//! headless artifacts are the same binary built with the feature off.

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

mod answers;
mod ask;
mod client;
#[cfg(feature = "desktop")]
mod desktop;
mod guide;
pub mod repo;
mod serve;

/// Where the server lives when nothing says otherwise. The tailnet is the
/// perimeter, so the default stays on the loopback interface.
const DEFAULT_SERVER: &str = "http://127.0.0.1:8422";

#[derive(Debug, Parser)]
#[command(
    name = "verkstead",
    // What the usage lines say this command is called, said rather than taken
    // from the file that was run. Clap otherwise names it after `argv[0]`,
    // which on Windows is `verkstead.exe` — so `verkstead ask --help` there
    // would print a usage line no document in this repository spells, the
    // Guide's quoted CLI contract included, and a Windows human types
    // `verkstead` at the prompt either way.
    bin_name = "verkstead",
    version,
    about = "Put a Question Set to the human and wait for the answer.\n\n\
             Run `verkstead guide` — or `verkstead` with no arguments — for \
             the Guide: everything an agent needs in order to ask well."
)]
pub struct Cli {
    /// No subcommand is the Guide: an agent that runs the binary to see what it
    /// is gets the instructions rather than clap's usage error.
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Submit a Question Set and wait for the human to answer it.
    ///
    /// Whether waiting means blocking here until the Response comes back, or
    /// storing the Set and ending the turn to be told later, is this backend's
    /// own — run `verkstead guide` for which it is and how to run one. Either
    /// way the human answers in their own time, and that may be hours.
    ///
    /// Prints the Response as YAML on stdout and exits 0 where it blocked, and
    /// the stored Set where it did not. Nothing else is ever written to stdout,
    /// so the agent can parse what comes back as it stands.
    Ask {
        /// The Question Set, as YAML. Read from stdin when absent.
        file: Option<PathBuf>,

        /// Nobody is to wait on it: store the Set and carry straight on.
        ///
        /// Prints the stored Set as YAML instead of a Response — its `id` and
        /// when the server took it — and exits 0, on every backend. The human
        /// answers it in their own time, and their Answers reach a later
        /// session of this Conversation and never this one, so `verkstead
        /// answers` refuses one. Wait only on Questions whose Answers affect
        /// the work about to be done.
        #[arg(long)]
        deferred: bool,

        /// Base URL of the Verkstead server.
        #[arg(long, env = "VERKSTEAD_SERVER", default_value = DEFAULT_SERVER)]
        server: String,
    },

    /// Fetch the Response to a Question Set stored earlier, by id.
    ///
    /// Prints the Response as YAML on stdout and exits 0 — byte for byte what
    /// a blocking `verkstead ask` prints for the same Set, so an agent parses
    /// the two the same way. Nothing else is ever written to stdout.
    ///
    /// A fetch rather than a wait: it polls once and comes back, so a Set
    /// nobody has answered yet is a non-zero exit rather than something to
    /// idle on. Run it when something has said the Answers are there.
    ///
    /// A Set sent with `--deferred` is refused: its Answers go into the prompt
    /// of a later session of this Conversation and are not this session's to
    /// take.
    Answers {
        /// The id of the stored Set, as the ask that stored it printed it.
        id: i64,

        /// Base URL of the Verkstead server.
        #[arg(long, env = "VERKSTEAD_SERVER", default_value = DEFAULT_SERVER)]
        server: String,
    },

    /// Run the Verkstead server: the agents' API and the human's viewer.
    ///
    /// The flags are the server's own, and the one verb here that is not an
    /// agent's — everything else in this binary talks *to* a server.
    Serve(verkstead_server::Config),

    /// Run Verkstead on the desktop: the server, and a tray icon over it.
    ///
    /// `serve` with a screen in front of it — the viewer opened in the default
    /// browser at startup unless `--no-open` says otherwise, an icon in the
    /// system tray, and the log written to a file rather than to a terminal
    /// nobody started this from. The flags are `serve`'s own beneath that one.
    ///
    /// Here rather than in a binary of its own so that the image the server is
    /// running out of is an image that can also `ask` (ADR-0012, amended). A
    /// build made with `--no-default-features` has no tray half and no verb for
    /// it.
    #[cfg(feature = "desktop")]
    Desktop(verkstead_desktop::Desktop),

    /// Print the Guide: everything an agent needs in order to ask well.
    ///
    /// Markdown on stdout, exit 0. With no topic, the core Guide — the same
    /// one bare `verkstead` prints. The Guide has no Topics at present, so
    /// that is the whole of it.
    Guide {
        /// A Topic of the Guide, required reading when its task is at hand.
        /// Omit for the core Guide.
        topic: Option<String>,
    },
}

impl Cli {
    pub fn run(self) -> Result<()> {
        match self.command {
            Some(Command::Ask {
                file,
                deferred,
                server,
            }) => ask::ask(file.as_deref(), deferred, &server),
            Some(Command::Answers { id, server }) => answers::answers(id, &server),
            Some(Command::Serve(config)) => serve::serve(config),
            #[cfg(feature = "desktop")]
            Some(Command::Desktop(app)) => desktop::desktop(app),
            Some(Command::Guide { topic }) => guide::guide(topic.as_deref()),
            None => guide::guide(None),
        }
    }
}
