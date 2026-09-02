//! The loop the tray lives on: the toolkit that draws it, the thread that runs
//! it, and the two ways it ends.
//!
//! **The main thread is the loop's**, whichever platform this is, and that is
//! why there is a module for it at all. GTK holds the thread it was started on
//! for as long as `gtk::main` is running; AppKit will not put anything in the
//! menu bar from any thread but the main one. So [`start`] is called on the main
//! thread, [`run`] blocks it, and everything drawn afterwards is spoken to from
//! there — the tray's menu, and the dialogs a menu item raises. The server runs
//! beside all of it on a runtime of its own threads; see [`crate::Desktop::run`],
//! which is where these four are called from.
//!
//! **The loop ends two ways, and they arrive on different threads.** Exit picked
//! off the menu is handled on the loop's own thread; the server stopping
//! underneath the app is a task on the runtime. Ending a loop is something its
//! own thread does, so the second of those has to ask rather than do — which is
//! [`stop`] and [`stop_from_elsewhere`], one for each caller.
//!
//! **One toolkit for the whole binary on each platform**, which is what makes
//! the dialogs [`crate::dialog`] draws the same toolkit's as the menu: GTK on
//! Linux, which is also what `muda` draws the menu with, and AppKit on macOS,
//! which is what `tray-icon` puts the status item in. A binary with two would be
//! two answers to what a machine has to carry to build it, and a dialog that
//! could not be raised from inside the loop's own dispatch.

use anyhow::Result;

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
compile_error!(
    "verkstead-desktop draws its tray with GTK on Linux and with AppKit on macOS, and this is \
     neither — the Windows arm is stage 05's."
);

/// Start the toolkit, on the thread the loop is to run on.
///
/// Called on the main thread and nowhere else. Idempotent: the tray starts the
/// toolkit long before anything else needs it, and [`crate::dialog`] asks again
/// because the one caller that has not is the caller it exists for — a Verkstead
/// that could not take its address, and has nothing but the message to say.
pub fn start() -> Result<()> {
    platform::start()
}

/// Block this thread on the loop until [`stop`] or [`stop_from_elsewhere`] ends
/// it.
///
/// Called on the thread [`start`] was called on, which is the main thread.
/// Anywhere else there is no loop to be running, because [`start`] refused
/// there, and this returns at once.
pub fn run() {
    platform::run();
}

/// End the loop, from the thread it is running on.
///
/// What **Exit** does: a menu item's handler runs on the loop's own thread, so
/// this is the loop being told to stop by the thread that is holding it.
pub fn stop() {
    platform::stop();
}

/// End the loop, from a thread that is not the one it is running on.
///
/// What the server stopping does: it is awaited on the runtime, and a loop can
/// only be ended by its own thread — so the ending is handed to that thread
/// rather than done here.
pub fn stop_from_elsewhere() {
    platform::stop_from_elsewhere();
}

/// GTK: the toolkit on Linux, and the appindicator the icon is drawn as.
#[cfg(target_os = "linux")]
mod platform {
    use anyhow::{Context, Result};

    pub(super) fn start() -> Result<()> {
        gtk::init().context("the desktop toolkit would not start")
    }

    pub(super) fn run() {
        gtk::main();
    }

    pub(super) fn stop() {
        gtk::main_quit();
    }

    /// Put on the loop's own list of things to do next, which is the one thing
    /// GTK may be asked for from another thread.
    pub(super) fn stop_from_elsewhere() {
        gtk::glib::idle_add_once(gtk::main_quit);
    }
}

/// AppKit: the toolkit on macOS, and the status item the icon is drawn as.
#[cfg(target_os = "macos")]
mod platform {
    use anyhow::{Result, bail};
    use dispatch2::DispatchQueue;
    use objc2::MainThreadMarker;
    use objc2::rc::Retained;
    use objc2_app_kit::{
        NSApplication, NSApplicationActivationPolicy, NSEvent, NSEventModifierFlags, NSEventType,
    };
    use objc2_foundation::NSPoint;

    /// The application object, and the app saying what kind of app it is.
    ///
    /// `sharedApplication` is what makes one where there is none, so this is
    /// AppKit starting as much as it is Verkstead's own answer — and it is
    /// idempotent for the same reason: every call after the first is handed the
    /// application the first one made.
    pub(super) fn start() -> Result<()> {
        let Some(main) = MainThreadMarker::new() else {
            bail!("the tray has to be drawn from the main thread, and this is not the main thread");
        };

        let app = NSApplication::sharedApplication(main);

        // A menu-bar app rather than a windowed one. There is no window behind
        // the icon (ADR-0012), so a Dock tile would be a tile that opens
        // nothing and a Command-Tab entry would be an app with nothing to
        // switch to — and this is the answer whether or not the binary is
        // inside a bundle with an `Info.plist` saying the same thing, which is
        // what makes it the answer for a binary run from a shell as well.
        app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);

        Ok(())
    }

    pub(super) fn run() {
        let Some(main) = MainThreadMarker::new() else {
            // `start` has already refused off the main thread, so there is no
            // loop here to be running.
            return;
        };

        NSApplication::sharedApplication(main).run();
    }

    pub(super) fn stop() {
        match MainThreadMarker::new() {
            Some(main) => end(main),
            // Not this thread's to end, so it is asked for rather than done —
            // which is the other function, and no reason to refuse the caller.
            None => stop_from_elsewhere(),
        }
    }

    pub(super) fn stop_from_elsewhere() {
        DispatchQueue::main().exec_async(|| {
            // SAFETY: the main queue runs what it is given on the main thread,
            // which is what this closure is being run by.
            end(unsafe { MainThreadMarker::new_unchecked() });
        });
    }

    /// Tell the application to stop, and then give it the event it has to be
    /// finishing with to notice.
    ///
    /// `stop:` raises a flag rather than an ending: the loop reads it once it
    /// has finished with the event it is on, and an app whose icon is sitting in
    /// the menu bar with nobody touching it is not on one. So the flag is
    /// followed by an event of Verkstead's own, put at the head of the queue —
    /// nothing handles it and nothing has to, because being handled is not what
    /// it is for.
    fn end(main: MainThreadMarker) {
        let app = NSApplication::sharedApplication(main);

        app.stop(None);

        if let Some(nudge) = nudge() {
            app.postEvent_atStart(&nudge, true);
        }
    }

    /// An event that means nothing, for the loop to end on.
    ///
    /// `ApplicationDefined` is the kind AppKit reserves for exactly this: an
    /// event the framework will carry and never interpret. Nothing about where
    /// or when it happened is read by anybody, so it happened at the origin, at
    /// no time, with no modifiers held.
    fn nudge() -> Option<Retained<NSEvent>> {
        NSEvent::otherEventWithType_location_modifierFlags_timestamp_windowNumber_context_subtype_data1_data2(
            NSEventType::ApplicationDefined,
            NSPoint::ZERO,
            NSEventModifierFlags::empty(),
            0.0,
            0,
            None,
            0,
            0,
            0,
        )
    }
}
