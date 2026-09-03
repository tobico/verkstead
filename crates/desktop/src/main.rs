//! `verkstead-desktop.exe` — the shim that starts the app for a launcher that
//! cannot say a verb.
//!
//! Verkstead is one binary again (ADR-0012, amended): the tray app is
//! `verkstead desktop`, and the library beside this file is the whole of it. A
//! Start-menu shortcut and a pinned taskbar icon name a *file*, though, and the
//! file they name has to be the app rather than a binary that prints the Guide
//! — so on Windows there is one more exe, and this is it. It starts `verkstead`
//! from beside its own image, hands it the verb and whatever it was given
//! itself, and exits with what the app exited with. Linux has no such gap — an
//! autostart entry and a `.desktop` file both hold a command line — and macOS
//! closes its own with a launcher script inside the bundle.
//!
//! **Windows alone**, therefore, which is also what the `shim` feature says:
//! the crate is a library on the other two platforms and this file is not built
//! there at all — see `Cargo.toml`.
//!
//! **No console**, which is what the subsystem below says: an exe of the console
//! kind opens a black window when it is double-clicked in Explorer, and that
//! window stays in front of the human for as long as the app runs. This is the
//! one place that attribute lives, so `verkstead` itself stays an ordinary
//! console program on every platform — which is what a shell, a session and a
//! test all want of it. The app it starts is a console program too, and would
//! be given a console of its own by Windows if nothing said otherwise, so
//! [`CREATE_NO_WINDOW`] says otherwise.
//!
//! **Beside its own image and never off the `PATH`.** The whole reason the two
//! halves were made one binary is that a session must ask the server that
//! spawned it, and a shim that resolved `verkstead` through the `PATH` would
//! hand that back: whatever the human installed last would answer instead of
//! the file this one was installed with.
//!
//! **And nothing of the library is linked in.** The one failure this has to
//! draw is drawn with `MessageBoxW` here rather than with
//! [`verkstead_desktop::dialog`], because reaching for that would put the
//! server, the tray and GTK's Windows cousin inside a file whose whole job is
//! to start another file. See [`refusal`].
#![cfg_attr(windows, windows_subsystem = "windows")]

// Said here as well as in `Cargo.toml`, where `required-features` is what
// actually keeps this file out of a Linux or macOS build: a build that turns
// the feature on somewhere else is told what the feature is for, rather than
// being told that `std::os::windows` is not a module. Nothing below is gated a
// second time — this is the whole gate, and every line under it is Windows's.
#[cfg(not(windows))]
compile_error!(
    "the `shim` feature builds verkstead-desktop.exe, which is Windows's alone: the desktop app \
     is `verkstead desktop` on every platform, and a shim in front of it is only wanted where a \
     launcher cannot say a verb."
);

use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

/// The unified binary this starts, which is the file beside this one.
const CLI: &str = "verkstead.exe";

/// The verb of it that is the app.
const VERB: &str = "desktop";

/// Start the app without giving it a console — see this file's own docs.
///
/// Named here rather than taken from `windows-sys`, which does not carry the
/// process-creation flags: it is one constant, and this is the whole of what
/// the shim asks Windows for.
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

fn main() -> ExitCode {
    match start() {
        Ok(code) => code,
        Err(why) => {
            refusal(&format!(
                "Verkstead could not start.\n\n{why}\n\n\
                 The app is started by verkstead.exe, which belongs in the same \
                 folder as this one. Installing Verkstead again puts it back."
            ));

            ExitCode::FAILURE
        }
    }
}

/// Run the app to its end, and hand back what it ended with.
///
/// The arguments are forwarded whole, this file's own name aside: everything
/// the app takes is the CLI's to parse, and a shim that read any of them would
/// be a second grammar to keep in step with the first.
fn start() -> Result<ExitCode, String> {
    let here = std::env::current_exe().map_err(|why| format!("{why}"))?;
    let cli = beside(&here);

    let status = Command::new(&cli)
        .arg(VERB)
        .args(std::env::args_os().skip(1))
        .creation_flags(CREATE_NO_WINDOW)
        .status()
        .map_err(|why| format!("{} could not be started ({why}).", cli.display()))?;

    // Truncated to what an `ExitCode` carries, which is what a process exit
    // code is on Windows in every case that means anything: the app is this
    // crate's own `main`, which exits 0 or 1. A code with more in it than a
    // byte is one nothing here put there, and reporting it as failure is the
    // honest reading — see the `unwrap_or` beside it, which is the app killed
    // by something rather than exiting at all.
    match status.code() {
        Some(0) => Ok(ExitCode::SUCCESS),
        Some(code) => Ok(ExitCode::from(u8::try_from(code).unwrap_or(1))),
        None => Ok(ExitCode::FAILURE),
    }
}

/// The unified binary beside `shim`.
///
/// A file in the same directory rather than a name for Windows to resolve: see
/// this file's own docs for what resolving it would cost. A shim with no parent
/// directory at all is not a thing `current_exe` returns, and the empty path it
/// would come to fails at the spawn with the message that names it.
fn beside(shim: &Path) -> PathBuf {
    shim.parent().unwrap_or(Path::new("")).join(CLI)
}

/// Put `message` on the screen and wait for it to be dismissed.
///
/// The one thing this file draws, and the reason it draws anything: a shim
/// started from an icon has no console for a message to be printed in, so a
/// `verkstead.exe` that is not there would otherwise be an icon that was
/// double-clicked and did nothing at all.
///
/// Owned by nothing, brought to the front and kept there, for the reasons
/// [`verkstead_desktop::dialog`] gives — this is that dialog's shape without
/// that dialog's crate behind it.
fn refusal(message: &str) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        MB_ICONERROR, MB_OK, MB_SETFOREGROUND, MB_TOPMOST, MessageBoxW,
    };

    let message = wide(message);
    let title = wide("Verkstead");

    // SAFETY: two null-terminated wide strings that outlive the call, a null
    // owner window, and a call made on the thread that started.
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            message.as_ptr(),
            title.as_ptr(),
            MB_OK | MB_ICONERROR | MB_SETFOREGROUND | MB_TOPMOST,
        );
    }
}

/// `said` as Windows takes a string: UTF-16, with the zero on the end that says
/// where it stops.
fn wide(said: &str) -> Vec<u16> {
    said.encode_utf16().chain(std::iter::once(0)).collect()
}
