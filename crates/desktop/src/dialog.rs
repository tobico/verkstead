//! The two things this binary draws that carry words.
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
//! **Drawn with GTK, on the thread the toolkit was started on**, which is the
//! main thread and — while there is a tray — the thread its loop runs on. That
//! is the whole of what these two functions are careful about. A dialog handed
//! to a toolkit thread of its own cannot be drawn from inside the loop's own
//! dispatch: the loop's thread holds the main context for as long as a menu
//! item's handler runs, so nothing else can take it to draw with, and a menu
//! item that raised one that way would never come back. Drawn here, the nested
//! loop [`gtk::prelude::DialogExt::run`] starts belongs to the thread that
//! already owns the context, which is what a menu handler and a dying `main`
//! both are.
//!
//! One toolkit for the whole binary, therefore — the same GTK the tray icon is
//! drawn with, and one answer for the packages a machine has to carry to build
//! it.

use gtk::prelude::*;

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
    draw(gtk::MessageType::Error, message);
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
    draw(gtk::MessageType::Info, message);
}

/// The dialog itself, at whichever level it is drawn as.
///
/// Called from the main thread and nowhere else, which is where both of this
/// binary's callers are: `main`, before there is a tray, and a menu item's
/// handler, which runs on the loop's own thread.
fn draw(level: gtk::MessageType, message: &str) {
    // Asked before the toolkit is, because a session with no screen is one that
    // has already had these words somewhere it can read them, and starting GTK
    // to find that out would only put its own complaint on the same stderr.
    if !crate::screen::there_is_one() {
        return;
    }

    // Idempotent, and asked here rather than owed by the caller: the tray has
    // usually started the toolkit long before this, and the one caller that has
    // not — the address that could not be taken — is the caller this module
    // exists for.
    if gtk::init().is_err() {
        return;
    }

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
