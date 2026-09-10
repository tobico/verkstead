//! The verb nobody types: the launcher a Windows session's console is made by,
//! on the far side of the account boundary.
//!
//! **Why the binary has a verb for this at all** (ADR-0014, *Amended: the
//! Sandbox is an account*). A session runs as a local account of Verkstead's
//! own, and a pseudoconsole cannot be handed to a process started as somebody
//! else: `CreateProcessWithLogonW` refuses an extended startup info outright.
//! So the console is made by somebody who already *is* the account — this —
//! started with the console's two pipes as its plain standard handles. It is
//! Verkstead's own image because a session already has that bound read-only,
//! and because one binary is the whole of what this product ships (ADR-0012).
//!
//! **It is hidden**, as hard to reach by accident as the rest of the internal
//! surface: no help entry, and a refusal in words when its standard handles are
//! not the two pipes it exists to build a console over. What it does is
//! [`verkstead_server::terminal::launcher::launch`]'s; what is here is the
//! words it is told in.
//!
//! Windows' own. The other platforms hand a session a pseudo-terminal the
//! kernel made and have nothing for a launcher to do.

use std::ffi::OsString;

use anyhow::{Context, Result, bail};
use clap::Args;
use verkstead_server::terminal::launcher;

/// What a launcher is told: the channel to speak to Verkstead over, the size to
/// open the console at, and the program to start on it.
#[derive(Debug, Args)]
pub struct Launcher {
    /// The named pipe Verkstead is waiting on, which a resize arrives down and
    /// the session's own exit code goes back up.
    #[arg(long, value_name = "PIPE")]
    channel: String,

    /// How wide to open the console.
    #[arg(long, value_name = "N")]
    columns: u16,

    /// And how tall.
    #[arg(long, value_name = "N")]
    rows: u16,

    /// The program to start on the console, and everything it is given.
    ///
    /// After a `--`, so that nothing a session is run with is read as a flag of
    /// this verb's — an agent's own arguments are nobody here's to parse.
    #[arg(last = true, required = true, value_name = "PROGRAM")]
    program: Vec<OsString>,
}

/// Make the console, start the program on it, and report how that ended.
pub fn launcher(asked: Launcher) -> Result<()> {
    let Some((program, argv)) = asked.program.split_first() else {
        bail!(
            "`verkstead {}` was given no program to start",
            launcher::VERB
        );
    };

    launcher::launch(&asked.channel, asked.columns, asked.rows, program, argv)
        .with_context(|| format!("running `verkstead {}`", launcher::VERB))
}
