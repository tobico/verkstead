//! The two things this app draws that carry words.
//!
//! Everything a human ever sees of Verkstead is the viewer in their browser and
//! the icon in their tray, and what is left over is here. A Verkstead that could
//! not take its address has no viewer to say so in and no tray to say it from —
//! and it was started from an icon rather than from a shell, so the message has
//! nowhere else to go (ADR-0012). A Verkstead that stopped on its own account
//! has neither either, and the same is true of it. And a menu item that has
//! nothing to do, or could not do what it was picked for, says so rather than
//! being picked and appearing to do nothing.
//!
//! **Drawn on the loop's own thread**, which is the main thread and — while
//! there is a tray — the thread [`crate::toolkit::run`] is holding. That is the
//! whole of what these two functions are careful about, and it is the same
//! obligation on all three platforms. A dialog handed to a toolkit thread of
//! its own cannot be drawn from inside the loop's own dispatch: GTK's loop
//! holds the main context for as long as a menu item's handler runs, so nothing
//! else can take it to draw with, and a menu item that raised one that way
//! would never come back; AppKit will not be spoken to from anywhere but the
//! main thread at all; and a message box on Windows runs a loop of its own over
//! a thread's message queue, which has to be the queue the tray's own messages
//! arrive in. Drawn here, the nested loop each platform starts for a modal
//! belongs to the thread that is already entitled to it — which is what a menu
//! handler and a dying `main` both are.
//!
//! One toolkit for the whole binary, therefore — the same GTK the tray icon is
//! drawn with on Linux, the same AppKit it is drawn with on macOS, and the same
//! Win32 it is drawn with on Windows. See [`crate::toolkit`], which is where any
//! of them is started.

/// Put `message` on the screen as an error, and wait for it to be dismissed.
///
/// The address that could not be taken, the server that stopped, and the Launch
/// on Startup box that could not be written: a failure the human has to be told
/// about, wherever it happened.
///
/// **Nothing here is reported and nothing here fails.** A machine with no
/// screen to draw on has already had the same words on stderr and in the log,
/// and a failure to tell somebody something is not itself something to tell
/// them.
pub fn refusal(message: &str) {
    draw(Level::Refusal, message);
}

/// Put `message` on the screen as a remark, and wait for it to be dismissed.
///
/// What a menu item says when there is nothing for it to do — **View Logs** on
/// a machine with nowhere to keep a log file, which is the whole of the list so
/// far. Not an error: the app is running, and it was asked for something it
/// happens not to have.
///
/// Nothing here is reported and nothing here fails either, for [`refusal`]'s
/// reasons.
pub fn note(message: &str) {
    draw(Level::Info, message);
}

/// Which of the two a message is, said without a toolkit in it.
///
/// The two platforms have their own names for the same distinction, so what
/// crosses [`draw`] is this rather than either of theirs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Level {
    /// Something went wrong.
    Refusal,
    /// Something is worth saying, and nothing went wrong.
    Info,
}

/// The dialog itself, at whichever level it is drawn as.
///
/// Called from the main thread and nowhere else, which is where both of this
/// binary's callers are: `main`, before there is a tray, and a menu item's
/// handler, which runs on the loop's own thread.
fn draw(level: Level, message: &str) {
    // Asked before the toolkit is, because a session with no screen is one that
    // has already had these words somewhere it can read them, and starting a
    // toolkit to find that out would only put its own complaint on the same
    // stderr.
    if !crate::screen::there_is_one() {
        return;
    }

    // Idempotent, and asked here rather than owed by the caller: the tray has
    // usually started the toolkit long before this, and the one caller that has
    // not — the address that could not be taken — is the caller this module
    // exists for.
    if crate::toolkit::start().is_err() {
        return;
    }

    put(level, message);
}

/// GTK's message dialog, run on the thread that started GTK.
#[cfg(target_os = "linux")]
fn put(level: Level, message: &str) {
    use gtk::prelude::*;

    let level = match level {
        Level::Refusal => gtk::MessageType::Error,
        Level::Info => gtk::MessageType::Info,
    };

    let dialog = gtk::MessageDialog::new(
        None::<&gtk::Window>,
        gtk::DialogFlags::MODAL,
        level,
        gtk::ButtonsType::Ok,
        "Verkstead",
    );
    dialog.set_title("Verkstead");
    dialog.set_secondary_text(Some(message));

    dialog.run();

    // A dialog that was run is still a window until it is told otherwise, and
    // the tray's own loop would go on drawing it.
    unsafe { dialog.destroy() };
}

/// AppKit's alert, run on the main thread.
///
/// **Brought to the front first**, which GTK's dialog does not have to be: a
/// menu-bar app is an accessory rather than an application somebody switched to
/// — see [`crate::toolkit`] — and an accessory's alert opens behind whatever
/// they were doing unless the app asks for the front. The deprecated spelling of
/// the asking is the one used, because the replacement arrived in macOS 14 and
/// this app runs on the macOS people have.
#[cfg(target_os = "macos")]
fn put(level: Level, message: &str) {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSAlert, NSAlertStyle, NSApplication};
    use objc2_foundation::NSString;

    let Some(main) = MainThreadMarker::new() else {
        // `toolkit::start` has already refused off the main thread, so this is
        // unreachable by the two callers there are — and drawing anyway is the
        // one thing that must not happen here.
        return;
    };

    let alert = NSAlert::new(main);
    alert.setAlertStyle(match level {
        Level::Refusal => NSAlertStyle::Critical,
        Level::Info => NSAlertStyle::Informational,
    });
    alert.setMessageText(&NSString::from_str("Verkstead"));
    alert.setInformativeText(&NSString::from_str(message));

    #[allow(deprecated)]
    NSApplication::sharedApplication(main).activateIgnoringOtherApps(true);

    alert.runModal();
}

/// Win32's message box, run on the thread the loop is on.
///
/// **Brought to the front and kept there**, which is the same courtesy the
/// macOS alert asks AppKit for: this process has no window a human ever
/// switched to, so a box left to Windows' own ordering opens behind whatever
/// they were doing and waits there. `MB_SETFOREGROUND` is what asks for the
/// front and `MB_TOPMOST` is what keeps it there, and neither is a claim about
/// the rest of the app — there is no rest of the app to raise.
///
/// **Owned by nothing**, which is the other half of having no window: the box
/// is given a null owner, so it is a window of its own and there is nothing for
/// it to be modal over. What it stops is this thread, which is what the two
/// callers want — a menu item that has said something, and a `main` that is
/// about to give up.
#[cfg(windows)]
fn put(level: Level, message: &str) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        MB_ICONERROR, MB_ICONINFORMATION, MB_OK, MB_SETFOREGROUND, MB_TOPMOST, MessageBoxW,
    };

    let level = match level {
        Level::Refusal => MB_ICONERROR,
        Level::Info => MB_ICONINFORMATION,
    };

    let message = wide(message);
    let title = wide("Verkstead");

    // SAFETY: two null-terminated wide strings that outlive the call, a null
    // owner window, and a call made on the thread that draws.
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            message.as_ptr(),
            title.as_ptr(),
            MB_OK | level | MB_SETFOREGROUND | MB_TOPMOST,
        );
    }
}

/// `said` as Windows takes a string: UTF-16, with the zero on the end that says
/// where it stops.
#[cfg(windows)]
fn wide(said: &str) -> Vec<u16> {
    said.encode_utf16().chain(std::iter::once(0)).collect()
}
