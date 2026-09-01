//! Whether this process has a screen to draw on.
//!
//! **Asked rather than risked**, and asked before anything is drawn — the tray
//! as much as the one dialog. GTK will not start without a screen, and what
//! sits on top of it does not always fail politely: rfd's dialog waits on a
//! toolkit thread that never began rather than coming back, so a Verkstead run
//! over SSH, or under a test, would hang on the very failure it was trying to
//! report.
//!
//! What says there is a screen on Linux is `$DISPLAY` or `$WAYLAND_DISPLAY`, so
//! that is what is read. A screen that is *named* and is not there is a
//! different question and one only GTK can answer — see [`crate::tray::show`],
//! which is where the answer arrives.

#[cfg(target_os = "linux")]
use std::ffi::OsStr;

/// Whether there is a screen, read from the process environment.
///
/// The read is made where drawing is asked for and nowhere below it — see
/// [`there_is_one_with`], which is the same question of the values themselves.
#[cfg(target_os = "linux")]
pub fn there_is_one() -> bool {
    there_is_one_with(
        std::env::var_os("DISPLAY").as_deref(),
        std::env::var_os("WAYLAND_DISPLAY").as_deref(),
    )
}

/// The same question where the answer is always yes: macOS and Windows draw
/// through the window server every session has, and neither says so in the
/// environment.
#[cfg(not(target_os = "linux"))]
pub fn there_is_one() -> bool {
    true
}

/// Whether `display` and `wayland` — `$DISPLAY` and `$WAYLAND_DISPLAY` as they
/// were read — name a screen.
///
/// Set and empty is unset: a shell that exported the name without a value has
/// no more of a screen than one that never mentioned it, and GTK is no happier
/// with it.
#[cfg(target_os = "linux")]
fn there_is_one_with(display: Option<&OsStr>, wayland: Option<&OsStr>) -> bool {
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
        assert!(there_is_one_with(Some(OsStr::new(":0")), None));
        assert!(there_is_one_with(None, Some(OsStr::new("wayland-0"))));
    }

    /// The case the dialog would hang on, which is the whole reason this is
    /// asked at all: a session with no screen — over SSH, in a container, under
    /// a test — where the message has already gone to stderr and the tray has
    /// nowhere to be.
    #[test]
    fn nothing_said_and_nothing_in_it_is_no_screen() {
        assert!(!there_is_one_with(None, None));
        assert!(!there_is_one_with(
            Some(OsStr::new("")),
            Some(OsStr::new(""))
        ));
    }
}
