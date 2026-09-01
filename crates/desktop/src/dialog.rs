//! The one thing this binary draws that carries words.
//!
//! Everything a human ever sees of Verkstead is the viewer in their browser and
//! the icon in their tray, with one exception: a Verkstead that could not take
//! its address has no viewer to say so in and no tray to say it from — and it
//! was started from an icon rather than from a shell, so the message has
//! nowhere else to go (ADR-0012).

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
