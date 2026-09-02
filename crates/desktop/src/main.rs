//! `verkstead-desktop` — the Verkstead a human starts from an icon. See the
//! crate docs for the shape of it; this is the order the two failures come in.
//!
//! **No console on Windows**, which is what the subsystem below says: an exe of
//! the console kind opens a black window when it is double-clicked in Explorer,
//! and that window stays in front of the human for as long as the app runs. So
//! this is a windows-subsystem binary, which draws a tray icon and dialogs and
//! attaches to no console at all.
//!
//! What that costs is the standard streams when a *shell* starts it: `--help`
//! typed into a terminal prints where nobody is looking. It is the right trade
//! for this binary and not for the other one — `verkstead` is what a shell is
//! given, this is what an icon is — and it costs a redirected stream nothing,
//! which is what a test and a release leg hand it. The account of a run goes to
//! the log file either way; see [`logs`], which is there because a tray app has
//! no stdout worth reading on any platform.
//!
//! [`logs`]: verkstead_desktop::logs
#![cfg_attr(windows, windows_subsystem = "windows")]

use std::process::ExitCode;

use clap::Parser;
use verkstead_desktop::{Desktop, dialog};

fn main() -> ExitCode {
    let desktop = Desktop::parse();

    // The address, before the server has made anything — and before there is a
    // log file to write anything in, which is what makes this failure different
    // from the ones below rather than the only one worth drawing. Nobody who
    // started Verkstead from an icon is watching a terminal, so a refusal that
    // was only printed would be a window that never opened and no reason given.
    let listener = match desktop.settle() {
        Ok(listener) => listener,
        Err(taken) => {
            eprintln!("verkstead-desktop: {taken}");
            dialog::refusal(&taken.to_string());
            return ExitCode::FAILURE;
        }
    };

    // And everything after it, which is the server's own startup and then the
    // serving. A Data Directory that cannot be written, a Watched Path that is
    // not there, no `HOME` for a session to have one — every one of them lands
    // here, and so does the machine with nowhere to resolve a Data Directory
    // to, which is refused at startup precisely because a refusal was supposed
    // to have somewhere to be worded.
    //
    // So it is said three ways, none of them enough on its own: on the standard
    // error for whoever started this from a shell, in the log file for whoever
    // goes looking afterwards, and on the screen, because an icon that appeared
    // and vanished is otherwise the whole of what the human was told.
    match desktop.run(listener) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            let why = format!("{error:#}");

            eprintln!("verkstead-desktop: {why}");
            tracing::error!("{why}");
            dialog::refusal(&format!("Verkstead has stopped: {why}."));

            ExitCode::FAILURE
        }
    }
}
