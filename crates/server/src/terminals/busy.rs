//! Whether a terminal has something running in it: the one judgement the server
//! makes about a shell it is holding, and the one a × on a tab asks for
//! ([ADR 0019](../../../../docs/adr/0019-the-code-pane.md), *Tabs and groups*).
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
//! `/proc/<pid>/comm` says what it is running.
//!
//! **What a shell is called is the path it was started as.** The kernel takes
//! `comm` from the last component of the path handed to `execve`, so a terminal
//! started on `/bin/sh` reads `sh` whatever `/bin/sh` is a symlink to — which is
//! why the name here is the basename of the path as it was spawned rather than
//! of anything resolved. `comm` is also cut to fifteen characters, so the
//! comparison is against as much of the name as would have survived.
//!
//! **And where it cannot tell, it says busy.** A terminal that will not answer,
//! a pid `/proc` has nothing for, and the whole of Windows — a pseudoconsole
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
/// `TASK_COMM_LEN` less the terminator the kernel counts in it, and the whole
/// of it where there is no such answer and nothing is ever compared.
///
/// A shell whose name is longer than this — nothing common, but a wrapper
/// somebody has named at length is one — would never match its own `comm`, and
/// a terminal whose shell can never be recognised is one that asks before every
/// close. So the comparison is against as much of the name as `comm` could have
/// held.
#[cfg(target_os = "linux")]
const COMM: usize = 15;

/// And where no name is ever read back — see [`running`].
#[cfg(not(target_os = "linux"))]
const COMM: usize = usize::MAX;

/// Whether something other than `shell` is in the foreground of `terminal`.
///
/// `shell` is the name, not the path — see [`named`], which is what the register
/// keeps beside each terminal.
pub fn of(terminal: &Terminal, shell: &str) -> bool {
    let Some(foreground) = terminal.foreground() else {
        return true;
    };

    let Some(running) = running(foreground) else {
        return true;
    };

    running != as_far_as_comm(shell)
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

/// And where there is no `/proc` to read it out of.
///
/// macOS has a process table and no file to read it from — `proc_pidinfo` is
/// the call, and it is not worth linking for a confirm — and Windows never gets
/// this far, having no foreground process group to have answered with. Both
/// read as *cannot tell*, which is a close that asks first.
#[cfg(not(target_os = "linux"))]
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
}
