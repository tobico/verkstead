//! `verkstead-desktop` — the Verkstead a human starts from an icon. See the
//! crate docs for the shape of it; this is the order the two failures come in.

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
