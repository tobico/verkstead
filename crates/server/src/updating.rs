//! This device's own Update: the press on its row, the line the desktop app
//! acts on, and the progress the app reports back (ADR-0027, *A device updates
//! itself*).
//!
//! **The server asks and the app does the whole of it.** A press marks this
//! device updating and says [`ASKING`] on stdout, which the app already reads
//! line by line; the app drives electron-updater on it — check, download,
//! `quitAndInstall` — and posts how far it has got back over the viewer's own
//! API with the Workbench Key, so the row's state is this server's to draw and
//! a phone sees it too. No new channel in either direction.
//!
//! **Held in memory and nowhere else**, for the reason the state ends where it
//! does: an Update ends in this server being stopped and a new one started, so
//! what a start reads is no Update going, and the row then shows the version
//! it came up on. *restarting…*, the moment between, is the viewer's to say —
//! the server that would say it is the one that has gone.
//!
//! **And an Update that failed is the row's trouble line** (ADR-0027: no
//! rollback — the line, and a higher release). The app reports it the way it
//! reports progress: a download or a check that failed while this server is
//! up, and a relaunch that came up on another version than the one it handed
//! over to, which is the new server being told what the old one never saw. The
//! line stands until the next press, or the next start.

use std::sync::{Arc, Mutex};

use verkstead_render::{InstallKind, ThisUpdating};

/// What the server says on stdout when its row's Update is pressed — the line
/// `desktop/src/updating.ts` waits for, as `ASKING` there too. A whole line in a
/// fixed shape, said apart from `tracing`, so that no field the logging puts
/// around a message can move it.
pub(crate) const ASKING: &str = "verkstead-desktop: update this device";

/// This device's Update, shared by every clone of the Devices handle.
#[derive(Debug, Clone, Default)]
pub(crate) struct Updating {
    held: Arc<Mutex<Held>>,
}

/// The Update going, and the trouble line the last one that failed left.
#[derive(Debug, Default)]
struct Held {
    going: Option<ThisUpdating>,
    trouble: Option<String>,
}

impl Updating {
    fn held(&self) -> std::sync::MutexGuard<'_, Held> {
        self.held
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
    }

    /// Take a press: the row reads *updating…* from here, and the app is told.
    /// `false` where one is already going, which a second press reads rather
    /// than starts again.
    pub(crate) fn begin(&self) -> bool {
        {
            let mut held = self.held();
            if held.going.is_some() {
                return false;
            }
            held.going = Some(ThisUpdating::default());
            held.trouble = None;
        }

        tracing::info!(
            "this device's Update was pressed, so the desktop app is asked to update it"
        );
        println!("{ASKING}");

        true
    }

    /// The app's report of how much of the release is down. Nothing where no
    /// Update is going: a report from an app that outlived its press is not
    /// one to start drawing *updating…* again on.
    pub(crate) fn progressed(&self, percent: u8) -> bool {
        let mut held = self.held();
        let Some(update) = held.going.as_mut() else {
            return false;
        };
        update.percent = Some(percent.min(100));
        true
    }

    /// The app's report that the Update failed, with why: the row leaves
    /// *updating…* for its trouble line, and the device stays on its version.
    /// Taken with no Update going as well, which is the relaunch that came up
    /// on another version telling a server that never saw the press.
    pub(crate) fn failed(&self, why: &str) {
        let line = sentence("This device couldn't be updated", why);
        tracing::warn!(%why, "the desktop app said this device's Update failed");

        let mut held = self.held();
        held.going = None;
        held.trouble = Some(line);
    }

    /// What the row is drawn with: the Update going, or `None`.
    pub(crate) fn reading(&self) -> Option<ThisUpdating> {
        self.held().going
    }

    /// And the trouble line the last Update that failed left, or `None`.
    pub(crate) fn trouble(&self) -> Option<String> {
        self.held().trouble.clone()
    }
}

/// The trouble line: `what`, and the first line of `why` as a sentence after
/// it — the Annex row's shape. The first line alone, because what the updater
/// says can carry a stack or a dump of headers under it, which is the log's.
fn sentence(what: &str, why: &str) -> String {
    let why = why.lines().next().unwrap_or_default().trim();

    if why.is_empty() {
        return format!("{what}.");
    }

    let stop = if why.ends_with(['.', '!', '?']) {
        ""
    } else {
        "."
    };

    format!("{what}: {why}{stop}")
}

/// Why this device can't be updated from its row, where it can't — each
/// install that isn't the desktop app's by what it is, the way its by-hand line
/// names it. `None` for the three electron-updater can replace.
pub(crate) fn refusal(install: InstallKind) -> Option<&'static str> {
    match install {
        InstallKind::AppImage | InstallKind::MacApp | InstallKind::WindowsInstall => None,
        InstallKind::Nix => Some("This device was installed with nix, so it's updated by hand."),
        InstallKind::Checkout => {
            Some("This device was built from a checkout, so it's updated by hand.")
        }
        InstallKind::Binary => {
            Some("This device wasn't started by the desktop app, so it can't update itself.")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A press is taken once: the second reads the first.
    #[test]
    fn a_second_press_reads_the_first() {
        let updating = Updating::default();

        assert!(updating.begin());
        assert!(!updating.begin());
        assert_eq!(updating.reading(), Some(ThisUpdating { percent: None }));
    }

    /// Progress lands on an Update going, and on nothing otherwise.
    #[test]
    fn progress_lands_only_on_an_update_going() {
        let updating = Updating::default();

        assert!(!updating.progressed(40));
        assert_eq!(updating.reading(), None);

        updating.begin();
        assert!(updating.progressed(40));
        assert_eq!(updating.reading(), Some(ThisUpdating { percent: Some(40) }));

        updating.progressed(250);
        assert_eq!(
            updating.reading(),
            Some(ThisUpdating { percent: Some(100) })
        );
    }

    /// A failure ends the Update going in the trouble line, and the next press
    /// takes the line away.
    #[test]
    fn a_failure_is_the_trouble_line_until_the_next_press() {
        let updating = Updating::default();

        updating.begin();
        updating.progressed(30);
        updating.failed("sha512 checksum mismatch, expected abc, got def\n    at stack");

        assert_eq!(updating.reading(), None);
        assert_eq!(
            updating.trouble().as_deref(),
            Some(
                "This device couldn't be updated: sha512 checksum mismatch, expected abc, got def."
            )
        );

        updating.begin();
        assert_eq!(updating.trouble(), None);
    }

    /// A failure is taken with no Update going: the relaunch's report.
    #[test]
    fn a_failure_with_no_update_going_is_still_the_line() {
        let updating = Updating::default();

        updating.failed("Verkstead 0.0.2 was installed, but this device started on 0.0.1.");

        assert_eq!(
            updating.trouble().as_deref(),
            Some(
                "This device couldn't be updated: Verkstead 0.0.2 was installed, but this \
                 device started on 0.0.1."
            )
        );
    }

    /// The desktop app's three artifacts are pressed; nothing else is.
    #[test]
    fn only_the_desktop_apps_installs_are_pressed() {
        for install in [
            InstallKind::AppImage,
            InstallKind::MacApp,
            InstallKind::WindowsInstall,
        ] {
            assert_eq!(refusal(install), None, "{install:?}");
            assert!(install.updates_itself());
        }
        for install in [InstallKind::Nix, InstallKind::Checkout, InstallKind::Binary] {
            assert!(refusal(install).is_some(), "{install:?}");
        }
    }
}
