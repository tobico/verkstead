//! **Launch on Startup**: whether Verkstead comes up when the machine's desktop
//! session does.
//!
//! **The platform's own registration is the source of truth**, and nothing
//! duplicates it. On Linux that is an XDG autostart entry — see [`xdg`] — on
//! macOS a launch agent in the user's own agents directory — see [`launchd`] —
//! and on Windows a value under this user's Run key — see [`run_key`]. The
//! checkbox on the tray menu is drawn from whether that registration is there
//! and says yes, read afresh every time it is asked. There is no entry in
//! either settings file for this and none is added: a human who takes the
//! registration away with the platform's own tools has unchecked the box, and
//! Verkstead agrees with them rather than argues.
//!
//! **Every launch rewrites the registration while it is there.** A binary that
//! was moved — downloaded again into another directory, an AppImage put
//! somewhere else — leaves a registration naming a path that is no longer
//! anything, and the next launch by hand is exactly the moment that can be
//! healed. Unregistered stays unregistered: nothing here ever writes a
//! registration that was not already there.
//!
//! **A launched-at-startup Verkstead is an ordinary launch**, and nothing in
//! this binary knows which one it was. The one thing the registration decides
//! is the browser, and it decides it the way anybody else does — with
//! `--no-open` on the command it starts, because a browser window arriving over
//! whatever the human is doing at every login is the thing that gets the box
//! unchecked.
//!
//! **The platform half is a module of its own**, and everything else in this
//! file is the same wherever it is built. Linux registers with an XDG autostart
//! entry, which is the `xdg` module; macOS with a launch agent plist, which is
//! the `launchd` one — one file that is there or is not, either way; and
//! Windows with a value under the Run key, which is the `run_key` module and
//! the same shape again in the one place Windows keeps this answer. Those are
//! the three, and there is no fourth arm behind them: a platform that is none
//! of them is one [`crate::toolkit`] refuses to compile at all, having no
//! toolkit there to draw a menu with.
//!
//! **A machine with nowhere to keep a registration is a state this file draws**
//! all the same — the item greyed rather than a box that ticks and does
//! nothing. It is an arm's own answer about a machine rather than a platform
//! without an arm: a Linux machine that names no configuration directory to put
//! an entry in has nowhere, and that is [`Startup`] carrying `None`.

// Built wherever the tests are rather than on macOS alone: everything in it is
// path and text, so the arm this Linux runner will never run is still an arm
// its tests call — see the module's own docs.
#[cfg(any(target_os = "macos", test))]
mod launchd;
// And the same, for the half of the Windows arm that is text: what a Run value
// holds is a command line, and a command line is written and read on any
// machine — see that module's own docs for what waits for a Windows one.
#[cfg(any(windows, test))]
mod run_key;
#[cfg(target_os = "linux")]
mod xdg;

use anyhow::{Context, Result, bail};

#[cfg(target_os = "macos")]
use launchd::Entry;
#[cfg(windows)]
use run_key::Entry;
#[cfg(target_os = "linux")]
use xdg::Entry;

/// Verkstead's startup registration on this machine, or the machine having
/// nowhere to keep one.
///
/// Read once at launch, because where the registration goes does not move while
/// the app runs; what it *says* is read from the platform every time, because
/// that does.
#[derive(Debug, Clone)]
pub struct Startup(Option<Entry>);

impl Startup {
    /// Where this machine keeps the registration, asked of the platform.
    pub fn here() -> Startup {
        Startup(Entry::here())
    }

    /// Whether this machine can be registered with at all.
    ///
    /// What the menu item is drawn from: a machine that names nowhere to keep a
    /// registration gets the item greyed rather than a box that ticks and does
    /// nothing.
    pub fn possible(&self) -> bool {
        self.0.is_some()
    }

    /// Whether Verkstead starts with the desktop session, read from the
    /// registration itself.
    pub fn on(&self) -> bool {
        self.0.as_ref().is_some_and(Entry::on)
    }

    /// Register the running executable, or take the registration away.
    ///
    /// What checking and unchecking the box does, and the whole of what either
    /// does: no settings file is touched, because the registration is the state
    /// rather than a copy of it.
    pub fn set(&self, on: bool) -> Result<()> {
        let Some(entry) = &self.0 else {
            bail!(
                "Verkstead has nowhere to keep a startup registration on this machine, so it \
                 cannot start itself with your desktop session."
            );
        };

        // Worded for a human under whatever the platform said underneath,
        // because a human is who reads it: this is picked off a menu, and a
        // failure is put on the screen — see [`crate::dialog::refusal`].
        if on {
            entry.write().context(
                "Verkstead could not ask this machine to start it with your desktop session",
            )
        } else {
            entry.remove().context(
                "Verkstead could not ask this machine to stop starting it with your desktop session",
            )
        }
    }

    /// Rewrite the registration with the running executable's path, while there
    /// is one.
    ///
    /// Called at every launch. A registration naming a binary that has moved
    /// heals itself here; an unregistered machine stays unregistered, because
    /// what this rewrites is what somebody asked for rather than something it
    /// decided for them.
    ///
    /// **Nothing here fails.** A registration that could not be rewritten is
    /// the one that was already there — which is what the launch before this
    /// one left, and no reason to stop a Verkstead that is otherwise about to
    /// serve.
    pub fn refresh(&self) {
        if !self.on() {
            return;
        }

        if let Err(error) = self.set(true) {
            tracing::warn!("the startup registration could not be rewritten: {error:#}");
        }
    }
}

/// Linux's, because what these write and read back is the platform arm's own
/// registration: a machine with nowhere to keep one has nothing to put in a
/// temporary directory and nothing to find there.
#[cfg(all(test, target_os = "linux"))]
mod tests {
    use std::path::Path;

    use super::*;

    /// A registration kept in `dir`, which is what the tests have instead of
    /// the machine's own.
    fn in_dir(dir: &Path) -> Startup {
        Startup(Some(Entry::in_dir(dir)))
    }

    /// The box, checked and unchecked: the registration follows and nothing
    /// else is kept anywhere.
    #[test]
    fn checking_the_box_registers_and_unchecking_it_takes_that_back() {
        let dir = tempfile::tempdir().unwrap();
        let startup = in_dir(dir.path());

        assert!(startup.possible());
        assert!(!startup.on(), "nothing has been registered yet");

        startup.set(true).unwrap();
        assert!(startup.on());

        startup.set(false).unwrap();
        assert!(!startup.on());
    }

    /// The state is the registration's, so a registration taken away by
    /// somebody else is a box that comes back unchecked rather than one
    /// Verkstead argues about.
    #[test]
    fn a_registration_removed_outside_verkstead_is_an_unchecked_box() {
        let dir = tempfile::tempdir().unwrap();
        let startup = in_dir(dir.path());

        startup.set(true).unwrap();
        for entry in std::fs::read_dir(dir.path()).unwrap() {
            std::fs::remove_file(entry.unwrap().path()).unwrap();
        }

        assert!(!startup.on());
    }

    /// The launch of a binary that has moved: the registration it left behind
    /// names where it was, and this is where that is put right.
    #[test]
    fn a_launch_rewrites_a_registration_that_names_somewhere_else() {
        let dir = tempfile::tempdir().unwrap();
        let startup = in_dir(dir.path());
        let entry = dir.path().join("net.tobico.Verkstead.desktop");

        std::fs::write(
            &entry,
            "[Desktop Entry]\nType=Application\nName=Verkstead\n\
             Exec=\"/somewhere/it/used/to/be/verkstead-desktop\" --no-open\n",
        )
        .unwrap();

        startup.refresh();

        let registered = std::fs::read_to_string(&entry).unwrap();
        let here = std::env::current_exe().unwrap();

        assert!(
            registered.contains(here.to_str().unwrap()),
            "the registration should name the executable that is running, got:\n{registered}"
        );
        assert!(
            !registered.contains("/somewhere/it/used/to/be/"),
            "and not the one it was written for, got:\n{registered}"
        );
    }

    /// And a launch of a machine nobody asked to be started on registers
    /// nothing: what a launch rewrites is what somebody asked for.
    #[test]
    fn a_launch_registers_nothing_that_was_not_registered() {
        let dir = tempfile::tempdir().unwrap();
        let startup = in_dir(dir.path());

        startup.refresh();

        assert!(!startup.on());
        assert_eq!(
            std::fs::read_dir(dir.path()).unwrap().count(),
            0,
            "nothing should have been written"
        );
    }

    /// A machine with nowhere to keep one has an item to grey rather than a box
    /// to tick, and the refusal it would give says what it is about.
    #[test]
    fn nowhere_to_keep_a_registration_is_an_item_that_cannot_be_picked() {
        let nowhere = Startup(None);

        assert!(!nowhere.possible());
        assert!(!nowhere.on());
        nowhere.refresh();

        let refused = nowhere.set(true).unwrap_err().to_string();
        assert!(refused.contains("nowhere"), "got: {refused}");
    }
}

/// And Windows's, for the reason the Linux module above is Linux's: what these
/// write and read back is the platform arm's own registration, and here that is
/// a value in the registry rather than a file in a directory.
///
/// Under a key of the test's own — `Software\net.tobico.Verkstead\tests` —
/// rather than under the Run key, so that a suite run on somebody's own machine
/// registers nothing and unregisters nothing. See
/// [`run_key`](super::run_key), whose own tests are the format's; these are
/// the checkbox's.
#[cfg(all(test, windows))]
mod tests {
    use super::*;

    /// The key a test keeps its registration in, named for what the test is
    /// about so that the tests in this module do not read each other's.
    fn key(about: &str) -> String {
        format!(r"Software\{}\tests\startup\{about}", crate::APP_ID)
    }

    /// A registration kept in that key, which is what the tests have instead of
    /// the machine's own.
    fn under(about: &str) -> Startup {
        Startup(Some(Entry::under(&key(about))))
    }

    /// The key this test wrote, taken away again so that a suite leaves the
    /// registry as it found it.
    fn forget(about: &str) {
        let _ = windows_registry::CURRENT_USER.remove_tree(key(about));
    }

    /// The box, checked and unchecked: the registration follows and nothing
    /// else is kept anywhere.
    #[test]
    fn checking_the_box_registers_and_unchecking_it_takes_that_back() {
        let startup = under("checking");

        assert!(startup.possible());
        assert!(!startup.on(), "nothing has been registered yet");

        startup.set(true).unwrap();
        assert!(startup.on());

        startup.set(false).unwrap();
        assert!(!startup.on());

        forget("checking");
    }

    /// The state is the registration's, so a registration taken away by
    /// somebody else — Task Manager's Startup tab, or the registry editor — is
    /// a box that comes back unchecked rather than one Verkstead argues about.
    #[test]
    fn a_registration_removed_outside_verkstead_is_an_unchecked_box() {
        let startup = under("removed");

        startup.set(true).unwrap();
        windows_registry::CURRENT_USER
            .remove_tree(key("removed"))
            .unwrap();

        assert!(!startup.on());
    }

    /// The launch of an exe that has moved: the registration it left behind
    /// names where it was, and this is where that is put right.
    #[test]
    fn a_launch_rewrites_a_registration_that_names_somewhere_else() {
        let key = key("moved");
        let startup = under("moved");

        windows_registry::CURRENT_USER
            .create(&key)
            .unwrap()
            .set_string(
                crate::APP_ID,
                r#""D:\where\it\used\to\be\verkstead-desktop.exe" --no-open"#,
            )
            .unwrap();

        startup.refresh();

        let registered = windows_registry::CURRENT_USER
            .open(&key)
            .unwrap()
            .get_string(crate::APP_ID)
            .unwrap();
        let here = std::env::current_exe().unwrap();

        assert!(
            registered.contains(here.to_str().unwrap()),
            "the registration should name the executable that is running, got: {registered}"
        );
        assert!(
            !registered.contains(r"D:\where\it\used\to\be"),
            "and not the one it was written for, got: {registered}"
        );

        forget("moved");
    }

    /// And a launch on a machine nobody asked to be started on registers
    /// nothing: what a launch rewrites is what somebody asked for.
    #[test]
    fn a_launch_registers_nothing_that_was_not_registered() {
        let startup = under("unasked");

        startup.refresh();

        assert!(!startup.on());
        assert!(
            windows_registry::CURRENT_USER.open(key("unasked")).is_err(),
            "nothing should have been written"
        );
    }
}
