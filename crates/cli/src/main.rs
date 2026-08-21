//! `verkstead` — the command coding agents run to put a Question Set to the
//! human and block until it is answered. See the crate docs for the shape of
//! it; this is only the exit code.

use std::process::ExitCode;

use clap::Parser;
use verkstead_cli::Cli;

fn main() -> ExitCode {
    match Cli::parse().run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("verkstead: {error:#}");
            ExitCode::FAILURE
        }
    }
}
