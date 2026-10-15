//! Whether a terminal has something running in it: the judgement a × on a tab
//! asks for ([ADR 0019](../../../../docs/adr/0019-the-code-pane.md), *Tabs and
//! groups*). Not the only reading of a terminal's processes any more — what
//! they listen on is read too, for the device that forwards them, in
//! [`super::ports`] — but the one that decides anything about the shell itself.
//!
//! **Busy is the foreground of the pseudo-terminal being something other than
//! the shell.** The kernel keeps, for every terminal, which process group is in
//! front of it — the one a keystroke goes to and the one Ctrl-C would interrupt
//! — and a shell at a prompt puts itself there and puts whatever it runs there
//! instead for as long as that runs. So a terminal whose foreground is the
//! shell is one nobody is in the middle of anything in, and one whose
//! foreground is anything else has a build, an editor or a `sleep` in it.
//!
//! **Read by name rather than by number**, which is the part worth saying out
//! loud. The obvious check is the foreground group against the shell's own pid,
//! and there is no such number on this side: the child the register's terminal
//! was spawned as is the sandbox wrapper — `bwrap` under `--unshare-all` — so
//! the shell is a grandchild inside a pid namespace of its own, and behind the
//! worktree's dev shell as well where its flake has one. The `setsid` in
//! [`crate::terminal::Terminal::spawn`] lands on that wrapper too, so the pty's
//! session leader is the wrapper rather than the shell. What is left is the
//! name: [`Terminal::foreground`] answers with a pid this side numbers, and
//! `/proc/<pid>/comm` says what it is running — on a Mac, whose Sandbox is a
//! `sandbox-exec` that execs the shell in place, the name `proc_name` gives
//! the same pid.
//!
//! **What a shell is called is the path it was started as.** The kernel takes
//! `comm` from the last component of the path handed to `execve`, so a terminal
//! started on `/bin/sh` reads `sh` whatever `/bin/sh` is a symlink to — which is
//! why the name here is the basename of the path as it was spawned rather than
//! of anything resolved. A Mac's kernel names a process the same way. `comm` is
//! also cut to fifteen characters, and a Mac's name to thirty-two, so the
//! comparison is against as much of the name as would have survived.
//!
//! **And where it cannot tell, it says busy.** A terminal that will not answer,
//! a pid with no name to read, and the whole of Windows — a pseudoconsole
//! has no foreground process group to read — are all *I do not know whether
//! somebody is working in this*, and the close that follows asks first. The
//! cost of being wrong that way is a confirm nobody needed; the other way it is
//! a shell ended under somebody's hands.
//!
//! Which is also what the first moments of a terminal read as: until the shell
//! has come up inside the Sandbox and taken the terminal's foreground for
//! itself, the foreground is the wrapper that started it and the name is the
//! wrapper's. A tab closed in that half-second asks, and asking about a shell
//! that has not finished starting is the harmless half of the same rule.

use crate::terminal::Terminal;

/// How much of a name the platform's own answer keeps: on Linux
/// `TASK_COMM_LEN` less the terminator the kernel counts in it, on a Mac the
/// `2 * MAXCOMLEN` of the name `proc_name` reads, and the whole of it where
/// there is no such answer and nothing is ever compared.
///
/// A shell whose name is longer than this — nothing common, but a wrapper
/// somebody has named at length is one — would never match its own `comm`, and
/// a terminal whose shell can never be recognised is one that asks before every
/// close. So the comparison is against as much of the name as `comm` could have
/// held.
#[cfg(target_os = "linux")]
const COMM: usize = 15;

/// A Mac's — see [`running`].
#[cfg(target_os = "macos")]
const COMM: usize = 2 * libc::MAXCOMLEN;

/// And where no name is ever read back — see [`running`].
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
const COMM: usize = usize::MAX;

/// Whether this platform can ever read a terminal as *not* busy: a foreground
/// to read and a name to read off it, which is Linux and macOS.
///
/// What a terminal opened for a Preview waits on before its command is typed —
/// see [`crate::terminals`] — and where it is `false` there is nothing to wait
/// for, so the line is typed at once and held by the terminal until the shell
/// reads it.
pub const TELLS: bool = cfg!(any(target_os = "linux", target_os = "macos"));

/// Whether something other than `shell` is in the foreground of `terminal`.
///
/// `shell` is the name, not the path — see [`named`], which is what the register
/// keeps beside each terminal.
pub fn of(terminal: &Terminal, shell: &str) -> bool {
    front(terminal, shell) != Front::Shell
}

/// What is in the foreground of a terminal, as far as this side can say.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Front {
    /// The shell the terminal was started on.
    Shell,

    /// Some other process group, by the id of the process leading it — which
    /// is what a Preview's run is recorded as (see [`super::run`]).
    Group(u32),

    /// Nothing can be said: no foreground to read, or no name to read off it.
    Unreadable,
}

/// The foreground of `terminal`, measured against `shell` — the reading
/// [`of`] makes its judgement out of, kept whole for the run's reading, which
/// wants to know *which* group is in front and not only that it is not the
/// shell.
pub fn front(terminal: &Terminal, shell: &str) -> Front {
    let Some(foreground) = terminal.foreground() else {
        return Front::Unreadable;
    };

    let Some(running) = running(foreground) else {
        return Front::Unreadable;
    };

    match running == as_far_as_comm(shell) {
        true => Front::Shell,
        false => Front::Group(foreground),
    }
}

/// As much of `shell` as the platform's own answer could have carried, cut on a
/// character rather than in the middle of one.
fn as_far_as_comm(shell: &str) -> &str {
    let mut kept = shell.len().min(COMM);

    while kept > 0 && !shell.is_char_boundary(kept) {
        kept -= 1;
    }

    &shell[..kept]
}

/// What the process group led by `pid` is running, by name.
///
/// `/proc/<pid>/comm` is the kernel's own short name for it, which is the last
/// component of the path it was `execve`d as — so this is compared against the
/// shell's path read the same way, and neither end resolves a symlink.
///
/// A pid with nothing there has exited between the terminal being asked and
/// this being read, which is a race this cannot win and does not try to: it
/// reads as *cannot tell*, and the close asks.
#[cfg(target_os = "linux")]
fn running(pid: u32) -> Option<String> {
    let comm = std::fs::read_to_string(format!("/proc/{pid}/comm")).ok()?;

    Some(comm.trim_end_matches('\n').to_owned())
}

/// And on a Mac, which has a process table and no file to read it from: by
/// the name the kernel keeps for it, which `proc_name` reads — the executable's
/// own name as it was `execve`d, as `comm` is, and readable for any process of
/// the same account. Once not worth the call for a confirm alone; worth it for
/// a Preview's run, which is decided by what is in front (ADR-0024).
///
/// The buffer is twice what the name can hold and zeroed: `proc_name` copies
/// the kernel's field whole and measures it with `strlen`, and a name that
/// fills the field has no terminator of its own.
#[cfg(target_os = "macos")]
fn running(pid: u32) -> Option<String> {
    let mut name = [0u8; 4 * libc::MAXCOMLEN];

    let read = unsafe {
        libc::proc_name(
            libc::c_int::try_from(pid).ok()?,
            name.as_mut_ptr().cast(),
            u32::try_from(name.len()).ok()?,
        )
    };

    let read = usize::try_from(read).ok().filter(|&read| read > 0)?;

    Some(String::from_utf8_lossy(&name[..read]).into_owned())
}

/// And everywhere else, which is Windows: it never gets this far, having no
/// foreground process group to have answered with, and reads as *cannot
/// tell*, which is a close that asks first.
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn running(_pid: u32) -> Option<String> {
    None
}

/// What to call the shell a terminal was started on: the last component of the
/// path it is spawned as.
///
/// The same reading the kernel does for `comm` — the path as given, with
/// nothing resolved — so that `/bin/sh` is `sh` on a machine where that is
/// bash, dash or busybox, exactly as `comm` will say.
pub fn named(shell: &str) -> String {
    std::path::Path::new(shell)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| shell.to_owned())
}

#[cfg(test)]
mod tests {
    use super::named;

    /// The name is the path's last component and nothing cleverer, which is
    /// what `comm` will say on the other side of the comparison.
    #[test]
    fn a_shell_is_named_by_the_path_it_is_started_as() {
        assert_eq!(named("/bin/sh"), "sh");
        assert_eq!(named("/run/current-system/sw/bin/bash"), "bash");
        assert_eq!(named("bash"), "bash");
    }

    /// And a path with nothing to take a name from is left as it stands rather
    /// than becoming an empty name that would match nothing.
    #[test]
    fn a_path_with_no_last_component_keeps_itself_as_its_name() {
        assert_eq!(named("/"), "/");
        assert_eq!(named(""), "");
    }

    /// And a name longer than the platform's own answer is compared as far as
    /// that answer would have gone, rather than never matching itself.
    #[cfg(target_os = "linux")]
    #[test]
    fn a_name_longer_than_comm_is_compared_as_far_as_comm_reaches() {
        assert_eq!(
            super::as_far_as_comm("a-shell-with-a-very-long-name"),
            "a-shell-with-a-"
        );
        assert_eq!(super::as_far_as_comm("bash"), "bash");
    }

    /// A shell on a real terminal, on the two platforms that read one: at its
    /// prompt it is the shell in front, a command it runs is a group of its own
    /// in front, read by that command's own name, and once that ends the shell
    /// is back. The Mac half of this runs on the `macOS sandbox` job alone,
    /// which is the one place `tcgetpgrp` on a Mac's master side and
    /// `proc_name` are asked anything.
    ///
    /// zsh on a Mac rather than `/bin/sh`, which there is a shim that execs
    /// whichever shell is selected and so is never named `sh` once running.
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[tokio::test]
    async fn a_shell_and_what_it_runs_are_read_in_front_of_a_real_terminal() {
        use super::{Front, front, running};
        use crate::sandbox::Rendering;
        use crate::terminal::Terminal;

        #[cfg(target_os = "linux")]
        const SHELL: (&str, &[&str]) = ("/bin/sh", &["-i"]);
        #[cfg(target_os = "macos")]
        const SHELL: (&str, &[&str]) = ("/bin/zsh", &["-f", "-i"]);

        let (path, args) = SHELL;
        let shell = named(path);

        // With the environment a terminal's shell is given, the terminal type
        // among it — a line editor with none may never take the line typed.
        let mut terminal = Terminal::open().unwrap();
        let mut rendering = Rendering::running(path);
        rendering
            .args(args)
            .set("PATH", std::env::var_os("PATH").unwrap_or_default())
            .set("TERM", crate::terminal::TERM);
        let _child = terminal.spawn(&rendering).unwrap();

        // And what it prints is read as it arrives, as the register's relay
        // reads it: a shell writing to a terminal nobody empties is one that
        // may stop, and what it showed is what a failure has to say.
        let terminal = std::sync::Arc::new(terminal);
        let shown = std::sync::Arc::new(std::sync::Mutex::new(Vec::<u8>::new()));
        let reading = tokio::spawn({
            let terminal = terminal.clone();
            let shown = shown.clone();

            async move {
                let mut buffer = [0u8; 4096];

                while let Ok(read @ 1..) = terminal.read(&mut buffer).await {
                    shown.lock().unwrap().extend_from_slice(&buffer[..read]);
                }
            }
        });
        let said = || String::from_utf8_lossy(&shown.lock().unwrap()).into_owned();

        let until = async |reached: &dyn Fn(Front) -> bool| {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);

            loop {
                let read = front(&terminal, &shell);

                if reached(read) {
                    return read;
                }

                assert!(
                    std::time::Instant::now() < deadline,
                    "{shell}'s terminal never got there, last reading {read:?}; it showed {:?}",
                    said()
                );
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }
        };

        until(&|read| read == Front::Shell).await;

        // In front is not yet at its prompt: a shell takes the terminal before
        // its line editor is ready, and a line typed in between may be thrown
        // away. A second is far more than any shell here takes.
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        terminal.write(b"sleep 30\r").await.unwrap();
        let Front::Group(group) = until(&|read| matches!(read, Front::Group(_))).await else {
            unreachable!("waited for a group");
        };
        assert_eq!(running(group).as_deref(), Some("sleep"), "{}", said());

        terminal.write(b"\x03").await.unwrap();
        until(&|read| read == Front::Shell).await;

        reading.abort();
    }
}
