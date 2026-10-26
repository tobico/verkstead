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
    going: Arc<Mutex<Option<ThisUpdating>>>,
}

impl Updating {
    /// Take a press: the row reads *updating…* from here, and the app is told.
    /// `false` where one is already going, which a second press reads rather
    /// than starts again.
    pub(crate) fn begin(&self) -> bool {
        {
            let mut going = self
                .going
                .lock()
                .unwrap_or_else(|poison| poison.into_inner());
            if going.is_some() {
                return false;
            }
            *going = Some(ThisUpdating::default());
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
        let mut going = self
            .going
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let Some(update) = going.as_mut() else {
            return false;
        };
        update.percent = Some(percent.min(100));
        true
    }

    /// What the row is drawn with: the Update going, or `None`.
    pub(crate) fn reading(&self) -> Option<ThisUpdating> {
        *self
            .going
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
    }
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
