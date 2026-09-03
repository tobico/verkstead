//! `verkstead desktop` — Verkstead started from an icon: the server in-process,
//! the viewer in the default browser, and an icon in the tray over the two of
//! them.
//!
//! The whole of the app is [`verkstead_desktop`]; this is the verb it is
//! reached through, and the order the two failures come in. It is a verb rather
//! than a binary of its own because of what the other verbs are: the sandbox
//! hands every session the image the server is running out of, and an image
//! that could serve but not ask is the skew ADR-0012 was amended to close. So
//! one binary answers `ask` and draws a tray, and a build that wants none of the
//! tray half says so with `--no-default-features` — see this crate's own
//! `desktop` feature.
//!
//! **Nothing here is the app's own doing.** Settling the address before the
//! server has made anything, the dialogs, the log file, `--no-open` — all of it
//! is the library's; this verb is the one door into it. The Windows shim that
//! stands in front of a Start-menu shortcut comes in the same way, running
//! `verkstead desktop` rather than reaching for the library itself.

use anyhow::{Result, anyhow};
use verkstead_desktop::{Desktop, dialog, startup::Entered};

/// What this verb is called, which is what a startup registration written from
/// inside it has to name after the executable's path — and what the Windows
/// shim says to reach it; see `crates/desktop/src/main.rs`.
///
/// Said here rather than read back off `argv`: a registration is a command line
/// the platform will run at every login for years, and what is in `argv` is
/// whatever a shell happened to hand this process on the one run that wrote it.
const VERB: &str = "desktop";

/// Run the tray app, and say what stopped it in each of the three places a
/// human might be looking.
///
/// **The address, before the server has made anything** — and before there is a
/// log file to write anything in, which is what makes that failure different
/// from the ones below rather than the only one worth drawing. Nobody who
/// started Verkstead from an icon is watching a terminal, so a refusal that was
/// only printed would be a window that never opened and no reason given.
///
/// **And everything after it**, which is the server's own startup and then the
/// serving. A Data Directory that cannot be written, a Watched Path that is not
/// there, no `HOME` for a session to have one — every one of them lands there,
/// and so does the machine with nowhere to resolve a Data Directory to, which
/// is refused at startup precisely because a refusal was supposed to have
/// somewhere to be worded.
///
/// So it is said three ways, none of them enough on its own: on the standard
/// error for whoever started this from a shell — which is `main`'s doing, as it
/// is for every other verb — in the log file for whoever goes looking
/// afterwards, and on the screen, because an icon that appeared and vanished is
/// otherwise the whole of what the human was told.
pub fn desktop(desktop: Desktop) -> Result<()> {
    let listener = match desktop.settle() {
        Ok(listener) => listener,
        Err(taken) => {
            dialog::refusal(&taken.to_string());

            // Its own words rather than the error itself: `main` prints an
            // error's whole chain, and what a taken address has under it is the
            // operating system's line — which this refusal already quotes, in
            // the sentence that says which address it was about.
            return Err(anyhow!("{taken}"));
        }
    };

    match desktop.run(listener, Entered::verb(VERB)) {
        Ok(()) => Ok(()),
        Err(error) => {
            let why = format!("{error:#}");

            tracing::error!("{why}");
            dialog::refusal(&format!("Verkstead has stopped: {why}."));

            Err(error)
        }
    }
}
