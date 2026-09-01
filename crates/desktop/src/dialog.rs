//! The two things this binary draws that carry words.
//!
//! Everything a human ever sees of Verkstead is the viewer in their browser and
//! the icon in their tray, with two exceptions. A Verkstead that could not take
//! its address has no viewer to say so in and no tray to say it from — and it
//! was started from an icon rather than from a shell, so the message has
//! nowhere else to go (ADR-0012). And a menu item that has nothing to do says
//! so, rather than being picked and appearing to do nothing.

/// Put `message` on the screen as an error, and wait for it to be dismissed.
///
/// GTK draws it, which is the toolkit the tray icon is drawn with too: one
/// toolkit for the whole binary rather than one per thing on the screen, and
/// one answer for the packages a machine has to carry to build it.
///
/// **Nothing here is reported and nothing here fails.** A machine with no
/// screen to draw on has already had the same words on stderr, and a failure to
/// tell somebody something is not itself something to tell them.
pub fn refusal(message: &str) {
    if !crate::screen::there_is_one() {
        return;
    }

    rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Error)
        .set_title("Verkstead")
        .set_description(message)
        .set_buttons(rfd::MessageButtons::Ok)
        .show();
}

/// Put `message` on the screen as a remark, and wait for it to be dismissed.
///
/// What a menu item says when there is nothing for it to do — **View Logs** on
/// a machine with nowhere to keep a log file, which is the whole of the list so
/// far. Not an error: the app is running, and it was asked for something it
/// happens not to have.
///
/// Nothing here is reported and nothing here fails either, for
/// [`refusal`]'s reasons. A machine with no screen has no tray to have picked a
/// menu item from in the first place.
pub fn note(message: &str) {
    if !crate::screen::there_is_one() {
        return;
    }

    rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Info)
        .set_title("Verkstead")
        .set_description(message)
        .set_buttons(rfd::MessageButtons::Ok)
        .show();
}
