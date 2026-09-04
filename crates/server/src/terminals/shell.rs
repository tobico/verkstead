//! Which shell a terminal runs: the server user's own, where the machine has
//! given it a usable one.
//!
//! A terminal is the human at the machine rather than an agent on it, so what
//! comes up in it should be the shell they would have got had they sat down at
//! it — their prompt, their aliases, their completions
//! ([ADR 0013](../../../../docs/adr/0013-conversation-terminals.md)). There is
//! no setting for it: the packaged install's answer is the nix module giving the
//! service user a shell, which is where a machine's shells are said already, and
//! a second place to configure one would be a place for the two to disagree.
//!
//! **Read out of the passwd database, and checked before it is believed.** The
//! entry is the machine's own word about the account the server runs as, and
//! three kinds of answer are no use to a human at a keyboard: an account with no
//! entry at all, one whose shell is not inside the Sandbox to be run, and a
//! system user's `nologin` or `false`, which are the two ways an account says it
//! is not for logging into. Each of them falls back to `/bin/sh` — the one path
//! every Sandbox is certain to have a shell at — because a terminal that opened
//! on nothing would be worse than one that opened on a plain shell.
//!
//! **Whichever is chosen is reachable inside.** `/nix`, `/usr`, `/bin` and
//! `/run/current-system` are bound into every Sandbox on every platform — see
//! [`crate::sandbox::SYSTEM`] — so a shell out of the store, out of the system
//! profile or out of `/usr/bin` is at the same path inside as it is out here,
//! which is what makes the passwd answer usable as it stands.
//!
//! **The rules are a function and the lookup is not**, so that what is tested is
//! the answer to each kind of passwd entry rather than the account the suite
//! happens to run under — see [`usable`], which is the whole of the deciding,
//! and [`login_shell`], which is the one call to the machine.

use std::path::Path;

/// The shell a terminal opens on where the machine has no usable one to name.
///
/// `/bin/sh` is NixOS's and a Mac's alike, and it is inside every Sandbox for
/// the reason [`crate::sandbox::SHELL`] says: it is the one path a shell can be
/// counted on to be at. A terminal that fell back to it is a plainer terminal
/// rather than a broken one.
pub(crate) const FALLBACK: &str = "/bin/sh";

/// The names an account uses to say it is not for logging into.
///
/// A system user is given one of these as its shell precisely so that anything
/// starting a shell for it starts nothing — and the server may well be running
/// as one, since a packaged install runs it under a system account. Both are
/// matched on the file's own name rather than on a path, because where the
/// distribution keeps them is its own business: `/run/current-system/sw/bin`,
/// `/usr/sbin` and `/sbin` are all somebody's answer.
const REFUSED: &[&str] = &["nologin", "false"];

/// What a terminal on this server runs.
///
/// The passwd answer put through [`usable`], with the filesystem as the witness
/// to whether the shell is really there.
pub fn of_the_server() -> String {
    let chosen = usable(login_shell().as_deref(), |shell| shell.is_file());

    tracing::debug!(shell = chosen, "a terminal runs this shell");

    chosen
}

/// `passwd`'s answer where it is usable, and [`FALLBACK`] where it is not.
///
/// `there` is whether the file is on the machine, which the caller answers off
/// the filesystem and a test answers with a value — the point of the split being
/// that every arm below can be asked about without the suite being run under an
/// account that would provoke it.
///
/// Absolute, because a bare word is a `PATH` lookup and what the sandbox is
/// handed is a command rather than a shell line; there, because a path naming
/// nothing is a terminal that would not start; and not one of [`REFUSED`],
/// because those are an account saying there is no shell to give.
fn usable(passwd: Option<&str>, there: impl Fn(&Path) -> bool) -> String {
    let Some(shell) = passwd.filter(|shell| !shell.is_empty()) else {
        return FALLBACK.to_owned();
    };

    let path = Path::new(shell);

    let named = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();

    if !path.is_absolute() || REFUSED.contains(&named) || !there(path) {
        return FALLBACK.to_owned();
    }

    shell.to_owned()
}

/// The login shell the passwd database gives the user this server runs as, or
/// `None` where it holds no entry for it.
///
/// The database rather than the file: `getpwuid_r` is what asks whichever of
/// them this machine keeps its accounts in — `/etc/passwd` on NixOS, the
/// directory service on a Mac — and reading the file directly would find nothing
/// on a Mac for every account a human actually has.
///
/// The reentrant call, with a buffer that grows until the entry fits, because
/// the server is threaded and the plain `getpwuid` hands back a pointer into
/// storage the next caller reuses.
#[cfg(unix)]
fn login_shell() -> Option<String> {
    use std::ffi::CStr;

    // Where the buffer starts and how far it is allowed to grow. glibc suggests
    // a size of its own and this is comfortably past it; the ceiling is there so
    // that a database answering `ERANGE` forever is a fallback rather than a
    // server eating memory.
    const ROOM: usize = 1024;
    const MOST: usize = 64 * 1024;

    let mut room = ROOM;

    loop {
        let mut buffer = vec![0u8; room];
        let mut entry: libc::passwd = unsafe { std::mem::zeroed() };
        let mut found: *mut libc::passwd = std::ptr::null_mut();

        // SAFETY: `entry` and `found` are ours to write into, and the buffer the
        // strings are written into is the one whose length is passed beside it.
        // Nothing read out of them outlives this iteration: the shell is copied
        // into a `String` below, before the buffer goes.
        let answered = unsafe {
            libc::getpwuid_r(
                libc::getuid(),
                &mut entry,
                buffer.as_mut_ptr().cast(),
                buffer.len(),
                &mut found,
            )
        };

        if answered == libc::ERANGE && room < MOST {
            room *= 2;
            continue;
        }

        // A non-zero answer is the lookup failing and a null `found` is the
        // database holding no such user — neither is a shell, and both are a
        // machine that has nothing to say about the account the server runs as.
        if answered != 0 || found.is_null() || entry.pw_shell.is_null() {
            tracing::debug!(
                answered,
                "the passwd database named no login shell for this server's own user"
            );

            return None;
        }

        // SAFETY: the entry was filled in above and its strings point into the
        // buffer, which is alive until this function returns.
        let shell = unsafe { CStr::from_ptr(entry.pw_shell) };

        return shell.to_str().ok().map(str::to_owned);
    }
}

/// And where there is no passwd database to ask — which is Windows, where there
/// is no pseudo-terminal to open a shell on either.
#[cfg(not(unix))]
fn login_shell() -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A machine where every shell asked about is really there.
    fn on_the_machine(_: &Path) -> bool {
        true
    }

    /// And one where none of them is.
    fn nowhere(_: &Path) -> bool {
        false
    }

    /// The passwd entry is what runs, where the machine has the shell it names.
    #[test]
    fn the_login_shell_is_what_a_terminal_opens_on() {
        for shell in [
            "/run/current-system/sw/bin/zsh",
            "/nix/store/whatever-fish-4.0.0/bin/fish",
            "/bin/bash",
        ] {
            assert_eq!(
                usable(Some(shell), on_the_machine),
                shell,
                "a usable login shell is the one a terminal runs",
            );
        }
    }

    /// A shell the Sandbox has no file for falls back, because a terminal
    /// started on one would open on nothing at all.
    #[test]
    fn a_shell_that_is_not_there_falls_back() {
        assert_eq!(usable(Some("/opt/somebody/bin/eshell"), nowhere), FALLBACK);
    }

    /// So does a system user's own, which is the account saying it is not for
    /// logging into — and the server may well be running as one.
    #[test]
    fn a_system_users_shell_falls_back() {
        for refused in [
            "/run/current-system/sw/bin/nologin",
            "/usr/sbin/nologin",
            "/sbin/nologin",
            "/bin/false",
            "/usr/bin/false",
        ] {
            assert_eq!(
                usable(Some(refused), on_the_machine),
                FALLBACK,
                "{refused} is an account with no shell to give",
            );
        }
    }

    /// And an account the database holds nothing for, or holds nothing usable
    /// as a path for.
    #[test]
    fn no_answer_at_all_falls_back() {
        assert_eq!(usable(None, on_the_machine), FALLBACK);
        assert_eq!(usable(Some(""), on_the_machine), FALLBACK);
        assert_eq!(usable(Some("bash"), on_the_machine), FALLBACK);
    }
}
