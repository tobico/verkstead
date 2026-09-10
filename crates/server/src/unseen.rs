//! Running a program without putting a window on the human's screen, which is
//! Windows' question and nobody else's.
//!
//! **A console program spawned by a process that has no console gets one.**
//! Windows makes it a new console, and a new console has a window, and that
//! window is drawn in front of whatever the human was looking at. It is
//! there for as long as the program runs — which for a `git rev-parse` is a
//! blink and for the Compile Server is all day — and either way it is an empty
//! black rectangle nobody asked for.
//!
//! **And the server has no console**, because the tray app is started by
//! `verkstead-desktop.exe` with `CREATE_NO_WINDOW` — see that binary, which is
//! where the same sentence is said one level up. So every program the server
//! runs to read something back — git, `gh`, `taskkill`, the Compile Server —
//! is one of those windows unless it says otherwise. On a machine where the
//! server was started from a shell instead, the child inherits *that* console
//! and there was never a window to suppress; the flag below is ignored in that
//! case, which is why it is set unconditionally rather than worked out.
//!
//! `CREATE_NO_WINDOW` is the whole of what it takes for a process the standard
//! library starts. It is **not** what a process started as the session account
//! takes — `CreateProcessWithLogonW` turns `CREATE_NEW_CONSOLE` on underneath
//! whatever it is handed and ignores this flag beside it — so that half of the
//! same problem is answered where that call is made, in
//! [`crate::sandbox::starting::as_the_account`], and this module is only about
//! the ordinary spawns.
//!
//! **The other two platforms have no such notion**, so the arms below are empty
//! there: a caller writes `.unseen()` once and every platform reads it, rather
//! than every caller carrying a `#[cfg]`.

/// Started with nothing drawn for it — see this module's own docs.
pub trait Unseen {
    /// This command, with no console window to be made for it.
    fn unseen(&mut self) -> &mut Self;
}

/// What Windows calls a console program run without a console window.
///
/// Named here rather than taken from `windows-sys`: the standard library's own
/// `creation_flags` takes a bare `u32`, this is the one flag Verkstead passes it,
/// and the two other platforms compile this file without a Win32 binding at all.
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[cfg(windows)]
impl Unseen for std::process::Command {
    fn unseen(&mut self) -> &mut Self {
        use std::os::windows::process::CommandExt;

        self.creation_flags(CREATE_NO_WINDOW)
    }
}

#[cfg(not(windows))]
impl Unseen for std::process::Command {
    fn unseen(&mut self) -> &mut Self {
        self
    }
}

#[cfg(windows)]
impl Unseen for tokio::process::Command {
    fn unseen(&mut self) -> &mut Self {
        self.creation_flags(CREATE_NO_WINDOW)
    }
}

#[cfg(not(windows))]
impl Unseen for tokio::process::Command {
    fn unseen(&mut self) -> &mut Self {
        self
    }
}
