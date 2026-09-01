//! The one thing this binary draws for itself.
//!
//! Everything a human ever sees of Verkstead is the viewer in their browser,
//! with one exception: a Verkstead that could not take its address has no viewer
//! to say so in, and was started from an icon rather than from a shell, so the
//! message has nowhere else to go (ADR-0012).

#[cfg(target_os = "linux")]
use std::ffi::OsStr;

/// Put `message` on the screen as an error, and wait for it to be dismissed.
///
/// GTK draws it, which is the toolkit a tray icon is drawn with too: one
/// toolkit for the whole binary rather than one per thing on the screen, and
/// one answer for the packages a machine has to carry to build it.
///
/// **Nothing here is reported and nothing here fails.** A machine with no
/// screen to draw on has already had the same words on stderr, and a failure to
/// tell somebody something is not itself something to tell them.
pub fn refusal(message: &str) {
    if !on_a_screen() {
        return;
    }

    rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Error)
        .set_title("Verkstead")
        .set_description(message)
        .set_buttons(rfd::MessageButtons::Ok)
        .show();
}

/// Whether this process has a screen to draw on.
///
/// **Asked rather than risked.** GTK will not start without one, and rfd's
/// dialog then waits on a toolkit thread that never began rather than coming
/// back — so a Verkstead run over SSH, or under a test, would hang on the very
/// failure it was trying to report. What says there is a screen on Linux is
/// `$DISPLAY` or `$WAYLAND_DISPLAY`, so that is what is read.
///
/// The read of the process environment, made where the dialog is asked for and
/// nowhere below it — see [`on_a_screen_with`], which is the same question of
/// the values themselves.
#[cfg(target_os = "linux")]
fn on_a_screen() -> bool {
    on_a_screen_with(
        std::env::var_os("DISPLAY").as_deref(),
        std::env::var_os("WAYLAND_DISPLAY").as_deref(),
    )
}

/// The same question where the answer is always yes: macOS and Windows draw
/// through the window server every session has, and neither says so in the
/// environment.
#[cfg(not(target_os = "linux"))]
fn on_a_screen() -> bool {
    true
}

/// Whether `display` and `wayland` — `$DISPLAY` and `$WAYLAND_DISPLAY` as they
/// were read — name a screen.
///
/// Set and empty is unset: a shell that exported the name without a value has
/// no more of a screen than one that never mentioned it, and GTK is no happier
/// with it.
#[cfg(target_os = "linux")]
fn on_a_screen_with(display: Option<&OsStr>, wayland: Option<&OsStr>) -> bool {
    [display, wayland]
        .into_iter()
        .flatten()
        .any(|named| !named.is_empty())
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    #[test]
    fn a_screen_is_either_variable_naming_one() {
        assert!(on_a_screen_with(Some(OsStr::new(":0")), None));
        assert!(on_a_screen_with(None, Some(OsStr::new("wayland-0"))));
    }

    /// The case the dialog would hang on, which is the whole reason this is
    /// asked at all: a session with no screen — over SSH, in a container, under
    /// a test — where the message has already gone to stderr.
    #[test]
    fn nothing_said_and_nothing_in_it_is_no_screen() {
        assert!(!on_a_screen_with(None, None));
        assert!(!on_a_screen_with(
            Some(OsStr::new("")),
            Some(OsStr::new(""))
        ));
    }
}
