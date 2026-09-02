//! Whether this process has a screen to draw on.
//!
//! **Asked rather than risked**, and asked before anything is drawn — the tray
//! as much as the dialogs. GTK will not start without a screen, and a Verkstead
//! run over SSH, in a container or under a test has none: what starting the
//! toolkit anyway would buy is GTK's own complaint on the standard error nobody
//! launched from an icon is reading, printed underneath the very message that
//! was being reported.
//!
//! What says there is a screen on Linux is `$DISPLAY` or `$WAYLAND_DISPLAY`, so
//! that is what is read. A screen that is *named* and is not there is a
//! different question and one only GTK can answer — see [`crate::tray::show`],
//! which is where the answer arrives.
//!
//! Windows says it nowhere in the environment and is asked instead: a process
//! belongs to a **window station**, and only one of a session's stations is the
//! one a human is looking at. A Verkstead started by a service or by a
//! scheduled task with nobody logged in is on one of the others, where a window
//! can be made and nothing can be seen or dismissed — which is the same
//! misfortune as a Linux session with no `$DISPLAY`, and the same answer.

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

/// The same question where the answer is always yes: macOS draws through the
/// window server every logged-in session has, and says so nowhere.
#[cfg(not(any(target_os = "linux", windows)))]
pub fn there_is_one() -> bool {
    true
}

/// And on Windows, whether this process's window station is one somebody can
/// see.
///
/// `WSF_VISIBLE` is what the flag means: the station has a desktop on the
/// screen, which is every logged-in session's `WinSta0` and none of the
/// stations a service is given. Windows will make a window on either — what it
/// will not do is show one on the second, so a message box there is a wait for
/// a dismissal that cannot arrive, and a tray icon is an icon in a
/// notification area no shell is drawing.
///
/// **A question that could not be asked is a screen.** Every reason this call
/// fails is about this process rather than about the station, and the answer
/// that costs least when it is wrong is the one that leaves the tray to
/// [`crate::tray::show`] to fail at — which is where a tray that cannot be
/// raised is already handled, and already logged.
#[cfg(windows)]
pub fn there_is_one() -> bool {
    use windows_sys::Win32::System::StationsAndDesktops::{
        GetProcessWindowStation, GetUserObjectInformationW, UOI_FLAGS, USEROBJECTFLAGS,
    };

    /// The station has a desktop that is drawn on a screen. Windows names the
    /// flag in a header rather than in the metadata `windows-sys` is generated
    /// from, so it is written out here.
    const WSF_VISIBLE: u32 = 0x0001;

    let mut flags = USEROBJECTFLAGS {
        fInherit: 0,
        fReserved: 0,
        dwFlags: 0,
    };
    let mut answered = 0;

    // SAFETY: the station handle is this process's own and is not ours to
    // close, and the call is handed a buffer of exactly the size it is told.
    let asked = unsafe {
        GetUserObjectInformationW(
            GetProcessWindowStation(),
            UOI_FLAGS,
            (&raw mut flags).cast(),
            size_of::<USEROBJECTFLAGS>() as u32,
            &mut answered,
        )
    };

    if asked == 0 {
        return true;
    }

    flags.dwFlags & WSF_VISIBLE != 0
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
