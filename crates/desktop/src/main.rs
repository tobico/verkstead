//! `verkstead-desktop` — the Verkstead a human starts from an icon. See the
//! crate docs for the shape of it; this is the order the two failures come in.

use std::process::ExitCode;

use clap::Parser;
use verkstead_desktop::{Desktop, dialog};

fn main() -> ExitCode {
    let desktop = Desktop::parse();

    // The address, before the server has made anything. This is the failure
    // there is a dialog for: nobody who started Verkstead from an icon is
    // watching a terminal, so a refusal that was only printed would be a window
    // that never opened and no reason given.
    let listener = match desktop.settle() {
        Ok(listener) => listener,
        Err(taken) => {
            eprintln!("verkstead-desktop: {taken}");
            dialog::refusal(&taken.to_string());
            return ExitCode::FAILURE;
        }
    };

    // Everything after it is the server's own startup, which reports the way the
    // server reports: through the log, which is where whoever is reading it will
    // be looking.
    match desktop.run(listener) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("verkstead-desktop: {error:#}");
            ExitCode::FAILURE
        }
    }
}
