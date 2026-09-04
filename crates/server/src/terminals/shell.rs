//! Which shell a terminal runs: the server user's own where the machine keeps
//! such an answer, and PowerShell where it keeps none.
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
//! **Whichever is chosen is reachable inside**, which is asked rather than
//! assumed. A Sandbox binds the machine's own directories in by a list of its
//! own — see [`crate::sandbox::SYSTEM`] — and a shell out of the store, out of
//! the system profile or out of `/usr/bin` is at the same path inside as it is
//! out here, which is most of the shells any machine hands out. A shell
//! anywhere else is not: `/opt/somebody/bin` and a home-manager
//! `~/.nix-profile/bin/fish` are both on the machine and both nowhere inside,
//! so a terminal started on one would open and die on its first line, over and
//! over, with nothing to say why. So the roots are what the path is asked
//! about, and one outside them falls back like every other unusable answer —
//! see [`reachable`].
//!
//! **And on Windows there is no passwd database to read**, which is the other
//! arm. An account there has no login shell recorded against it, and none of
//! the reasoning above survives the crossing: `/bin/sh` is not a path, and the
//! roots every Sandbox binds are a fact about a mount namespace, so a Windows
//! answer put through [`reachable`] would fall back for every shell there is.
//! What a human at that machine opens instead is `pwsh` where somebody has
//! installed PowerShell 7, and Windows PowerShell where nobody has — the one
//! every Windows machine carries. See [`installed`], which is that whole
//! choosing, and [`on_the_path`], which is its one call to the machine.
//!
//! One function with two arms rather than two notions: what a Terminal *is* is
//! the same word on both platforms, and only the machine it asks differs.
//!
//! **The rules are a function and the lookup is not**, so that what is tested is
//! the answer to each kind of passwd entry rather than the account the suite
//! happens to run under, and the answer to each kind of Windows machine rather
//! than whichever one the suite is on — see [`usable`] and [`installed`], which
//! are the whole of the deciding, and [`login_shell`] and [`on_the_path`],
//! which are the two calls to the machine.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use crate::platform::Platform;

/// The shell a terminal opens on where the passwd database has no usable one to
/// name.
///
/// `/bin/sh` is NixOS's and a Mac's alike, and it is inside every Sandbox for
/// the reason [`crate::sandbox::SHELL`] says: it is the one path a shell can be
/// counted on to be at. A terminal that fell back to it is a plainer terminal
/// rather than a broken one.
///
/// The Unix arm's, that database being the only thing this is ever the answer
/// to: what a Windows machine falls back to is [`WINDOWS_POWERSHELL`].
pub(crate) const FALLBACK: &str = "/bin/sh";

/// What a Windows terminal opens on where somebody has installed it: PowerShell
/// 7 and after, which is the one a human working on that machine has and the
/// one every piece of advice written this decade is about.
///
/// A bare name rather than a path, because where its installer put it is not
/// something to write down: it is on the `PATH` its installer added, and that is
/// what makes it *installed* rather than merely present. Looked for with no
/// extension on purpose — `PATHEXT` is what says which of `pwsh.exe` and a
/// `pwsh.cmd` beside it is the one to start, and that is the machine's answer
/// rather than this module's.
const PWSH: &str = "pwsh";

/// And what it opens on where nobody has: Windows PowerShell, which ships with
/// the operating system and is in the system directory on every `PATH` there is.
///
/// Named with its extension because that is its name — nothing has to be
/// resolved to know it is an executable — and it is what a machine that somehow
/// answers about neither is told to run, a name every Windows machine resolves
/// being a better last word than a path this module invented.
const WINDOWS_POWERSHELL: &str = "powershell.exe";

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
/// The passwd answer put through [`usable`] on the platforms that keep one, with
/// the filesystem as the witness to whether the shell is really there; and the
/// two PowerShells put through [`installed`] on the one that keeps none.
///
/// A value rather than a `cfg`, as everything but the pseudo-terminal itself is
/// here: the arm this machine will never run is still an arm its tests call.
pub fn of_the_server() -> String {
    let chosen = match Platform::HERE {
        Platform::Windows => installed(on_the_path),
        Platform::Linux | Platform::MacOs => {
            usable(login_shell().as_deref(), |shell| shell.is_file())
        }
    };

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
/// handed is a command rather than a shell line; [`reachable`], because a path
/// no Sandbox binds is one nothing inside can run; there, because a path naming
/// nothing is a terminal that would not start; and not one of [`REFUSED`],
/// because those are an account saying there is no shell to give.
///
/// **Absolute by the entry's own rules rather than by the machine reading it.**
/// A passwd shell is a Unix path whichever platform this is compiled for, and
/// `Path::is_absolute` on Windows says `/bin/bash` is relative — which would
/// make every rule below unreachable on a build that has no passwd database to
/// break anyway, and every test of them pass for the wrong reason.
fn usable(passwd: Option<&str>, there: impl Fn(&Path) -> bool) -> String {
    let Some(shell) = passwd.filter(|shell| !shell.is_empty()) else {
        return FALLBACK.to_owned();
    };

    let path = Path::new(shell);

    let named = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();

    if !shell.starts_with('/') || REFUSED.contains(&named) || !reachable(shell) || !there(path) {
        return FALLBACK.to_owned();
    }

    shell.to_owned()
}

/// Whether a Sandbox has that path in it, which is a different question from
/// whether the machine does.
///
/// Under one of the directories every Sandbox binds the machine in by — see
/// [`crate::sandbox::SYSTEM`], which is where the answer really lives, and which
/// is a different list on a Mac than on Linux. Bound at the path they are really
/// at, so a shell under one of them is the same path inside as out.
///
/// *Under*, which is why the rest of the path has to start with a separator:
/// `/opt/homebrew-something` is not inside `/opt/homebrew`, and a root by itself
/// is a directory rather than a shell.
fn reachable(shell: &str) -> bool {
    crate::sandbox::SYSTEM.iter().any(|root| {
        shell
            .strip_prefix(root)
            .is_some_and(|below| below.starts_with('/'))
    })
}

/// And the Windows rules: `pwsh` where the machine has one, and Windows
/// PowerShell where it has not.
///
/// `look` is the machine — [`on_the_path`] out here and a value in a test — so
/// that what is asked about is each kind of Windows machine rather than
/// whichever one happens to be running the suite. It is the same question both
/// times, which is why there is one of it: *is this program on the server's
/// `PATH`, by this platform's own rules for reading a name.*
///
/// **What comes back is where it was found**, so that `SHELL` inside names a
/// real file the way the passwd arm's answer does, and so that the shell that
/// was looked at is the shell that is started. Failing that — a machine that
/// answers about neither, or a path that will not go into a `String` — the bare
/// name of the one every Windows machine has, which the rendering resolves for
/// itself when it starts it.
///
/// Nothing here is a fallback in [`FALLBACK`]'s sense. Both of these are shells
/// a human types into; which one they get is which one the machine has.
fn installed(look: impl Fn(&str) -> Option<PathBuf>) -> String {
    look(PWSH)
        .or_else(|| look(WINDOWS_POWERSHELL))
        .and_then(|found| found.into_os_string().into_string().ok())
        .unwrap_or_else(|| WINDOWS_POWERSHELL.to_owned())
}

/// Where a program is on the server's own `PATH`, read the way a Windows
/// machine reads a name.
///
/// The rendering's own resolving, handed the server's environment rather than a
/// session's description — see [`crate::sandbox::open::found`], which is where
/// the rules are. `PATHEXT` is half of them: a name with no extension is not a
/// file on this platform, so a walk of `PATH` alone would find `pwsh` nowhere
/// it is actually installed.
///
/// The server's own `PATH` because that is what *installed* means here, and
/// because it is the `PATH` a Windows session is given — `servers_path` in
/// [`crate::sandbox`] is the Windows arm of that same question, and answers it
/// out of the same variable.
fn on_the_path(program: &str) -> Option<PathBuf> {
    crate::sandbox::open::found(
        OsStr::new(program),
        std::env::var_os("PATH").as_deref(),
        std::env::var_os("PATHEXT").as_deref(),
    )
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

/// And where there is no passwd database to ask — which is Windows, whose
/// accounts keep no login shell against them at all. What a terminal opens on
/// there is [`installed`]'s answer rather than this one's.
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
        assert_eq!(
            usable(Some("/nix/store/nothing/bin/eshell"), nowhere),
            FALLBACK
        );
    }

    /// And so does one that is on the machine but nowhere inside a Sandbox,
    /// which is the same failure by the other route: the shell would be run and
    /// would not be found, so the terminal would open and die on its first line
    /// with nothing to say why.
    #[test]
    fn a_shell_no_sandbox_binds_falls_back() {
        for outside in [
            "/opt/somebody/bin/eshell",
            "/home/you/.nix-profile/bin/fish",
            "/srv/shells/bash",
        ] {
            assert_eq!(
                usable(Some(outside), on_the_machine),
                FALLBACK,
                "{outside} is on the machine and nowhere inside a Sandbox",
            );
        }
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

    /// Where PowerShell 7 was installed, that is what a Windows terminal opens
    /// on — and what it opens on is where the lookup found it, so that the
    /// shell that was looked at is the shell that is started.
    #[test]
    fn pwsh_is_what_a_windows_terminal_opens_on() {
        const INSTALLED: &str = r"C:\Program Files\PowerShell\7\pwsh.exe";

        assert_eq!(
            installed(|program| (program == PWSH).then(|| PathBuf::from(INSTALLED))),
            INSTALLED,
        );
    }

    /// And where nobody installed it, Windows PowerShell — which every Windows
    /// machine has, so a terminal there opens on a shell either way.
    #[test]
    fn a_machine_without_it_opens_on_windows_powershell() {
        const SHIPPED: &str = r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe";

        assert_eq!(
            installed(|program| (program == WINDOWS_POWERSHELL).then(|| PathBuf::from(SHIPPED))),
            SHIPPED,
        );
    }

    /// And a machine that answers about neither is told the name of the one it
    /// has anyway, rather than a path this module made up: the rendering
    /// resolves a name for itself when it starts it.
    #[test]
    fn a_machine_that_answers_about_neither_gets_the_name() {
        assert_eq!(installed(|_| None), WINDOWS_POWERSHELL);
    }
}
