//! What a Mac has where Linux has `--die-with-parent`, and what Windows has
//! where neither is on offer: a keeper beside every sandbox with a process
//! group for it to end, and a Job Object holding what it started.
//!
//! One promise, three ways of keeping it, and this module is where the two
//! Verkstead has to keep for itself live. The promise is the same on all
//! three: **nothing Verkstead started outlives the server that started it** —
//! see [`keep`] for the Mac's, [`held`] for the Windows one, and [`job`] for
//! what that is made of.
//!
//! On Linux a sandbox outlives nothing because bubblewrap says so. Every
//! session and the compile server are started `--die-with-parent` — see the
//! `bwrap` rendering — and ADR-0012 leans on it: the tray's **Exit** is a stop
//! where it stands, with no shutdown path anywhere in the server, precisely
//! because what it leaves behind is nothing.
//!
//! Apple's sandbox has no such flag to keep. A process under `sandbox-exec` is
//! an ordinary child of whoever started it, so a server that is gone leaves one
//! running with a Worktree open and an agent still talking to a model. The
//! lifetime is therefore Verkstead's own on that platform, which is this
//! module — the arm for the platform with no flag, rather than a second way of
//! doing what the flag already does.
//!
//! **A keeper, and a group for it to end.** Every sandbox on a Mac is started
//! in a process group of its own — see [`in_its_own_group`] — and a keeper is
//! started beside it: a small shell that asks, once a second, whether the
//! server is still there. When it is not, the keeper kills that group, which is
//! the sandbox and everything a session started inside it, and goes.
//!
//! **A shell, and asking rather than waiting.** There is nothing to block on
//! the end of a process that is not your child, so a keeper asks: `kill -0`,
//! once a second, which is a signal that is never sent and an answer about
//! whether there is anybody left to send it to. And it is a shell because it
//! has to be a program already on the machine — Verkstead's own image would
//! have to be startable as something that is not Verkstead, and a second binary
//! beside it in the bundle would be a thing to build, lipo and ship for a loop
//! of four lines.
//!
//! **Nothing of the server's runs at that moment**, which is the whole of why
//! it is out here rather than on some path through the shutdown the server does
//! not have. **Exit** off the tray, a `kill -9` of the process and a crash all
//! end the same way, because there is nothing to end them differently: the
//! promise is kept from outside the process.
//!
//! **It watches the group rather than the process that leads it.** A group with
//! anything left in it is a group there is still something to end — a session's
//! agent may have left a build running and gone — and an empty one is a keeper
//! with nothing to keep. Which is also what makes the kill safe to make: a
//! process group id is a pid, pids are reused, and a group nothing is in is one
//! whose number may be somebody else's by the time it is used. While anything
//! is in it, it is nobody else's.
//!
//! **The keeper is nobody's.** It is started in a session of its own, so that
//! what reaches the server's own process group — a Ctrl-C on `verkstead serve`
//! — takes the server and leaves the keeper standing, which is exactly the
//! moment it is for. And it is orphaned rather than held: the shell Verkstead
//! starts forks the loop and exits, so the server has one shell to reap and no
//! keeper of its own to remember.

/// And the third platform's answer, which is a handle rather than a keeper —
/// see [`job::Job`], and [`held`], which is where the Compile Server gets one.
///
/// A `cfg` rather than an arm compiled everywhere, for the reason
/// [`crate::terminal`] is one: a Job Object is not a thing a machine that is
/// not Windows has any way to make, so there is no value to be had here and
/// nothing for a test elsewhere to call.
#[cfg(windows)]
pub(crate) mod job;

use std::io;
use std::process::{Child, Command, Stdio};

use crate::platform::Platform;

/// The shell a keeper is written in, by its whole path.
///
/// Absolute for the reason the `sandbox-exec` a policy is applied by is named
/// absolutely: what the server's own `PATH` holds is a fact about however the
/// app was launched, and this is a promise about what a session leaves behind.
const SH: &str = "/bin/sh";

/// How often a keeper looks, in seconds — and so the longest a sandbox goes on
/// running after the server that started it has gone.
///
/// A second, because there is nothing to be gained by it being shorter and the
/// question is one `kill -0` against a pid the shell already holds. What it
/// costs is a `sleep` per second per sandbox, which is the price of the
/// platform having nothing to hang this on.
const EVERY: u32 = 1;

/// Keep the sandbox running in process group `sandbox` from outliving the
/// server running as `server`.
///
/// A no-op on the platforms whose sandbox says this for itself. Linux is the
/// one that does — see this module's own documentation — and on Windows what
/// says it is the Job Object a process is put in: a session's terminal makes
/// one around what it starts (see [`crate::terminal`]), and everything else
/// gets one from [`held`], which is a value the caller holds rather than
/// something started beside it.
///
/// **The platform is a value and the server is said**, for the reason
/// [`Platform`] is a value at all: the arm this machine will never run is still
/// an arm a test can call, and a promise about one process is one a test can
/// only prove by making that process something it may kill. Everything outside
/// a test passes [`Platform::HERE`] and its own pid.
///
/// Nothing is refused for. A keeper that will not start is said in the log
/// naming what it was to have kept, and what it was started beside goes on
/// running: a sandbox that may outlive the server is worth less than one that
/// cannot, and more than one that was never started at all.
pub(crate) fn keep(platform: Platform, sandbox: u32, server: u32) {
    match platform {
        // Linux, whose sandboxes are `bwrap --die-with-parent` children; and
        // Windows, whose are inside a Job — see [`held`]. Neither has anything
        // for a keeper to add.
        Platform::Linux | Platform::Windows => {}

        Platform::MacOs => {
            if let Err(error) = kept(sandbox, server) {
                tracing::error!(
                    %error,
                    sandbox,
                    server,
                    "no keeper could be started, so this sandbox would outlive the server",
                );
            }
        }
    }
}

/// And the other half of the same promise, on the platform that keeps it with a
/// handle: `child` put in a Job that kills everything left in it when the last
/// handle to that Job closes.
///
/// **What comes back is held rather than read.** A Job is a promise for exactly
/// as long as somebody has a handle to it, so this hands one back and the
/// caller keeps it beside the child it is about — see [`Held`]. Dropping it, or
/// the server going down however it goes down, closes it and ends the tree.
///
/// **Its one caller is the Compile Server**, for the reason
/// [`in_its_own_group`] has the same one: a session's sandbox is already inside
/// a Job, because it runs on a pseudoconsole and
/// [`crate::terminal::Terminal::spawn`] makes one there. The Compile Server
/// runs on no terminal, so it says this for itself.
///
/// **The child is running by the time it is put in**, which is a window this
/// cannot close: `CreateProcessW` can start a process suspended and the
/// standard library's spawn will not hand back the thread to resume, so there
/// is no way from here to have the Job before the first instruction. What is at
/// risk in that window is a grandchild started in the microseconds between the
/// spawn and this call, and what runs there is an sccache server that starts
/// compilers on demand minutes later.
///
/// Nothing is refused for, as nothing is for a keeper: a Job that could not be
/// made is said in the log, and the Compile Server it was to have held goes on
/// running.
pub(crate) fn held(platform: Platform, child: &Child) -> Held {
    match platform {
        // Whose lifetimes are the sandbox's own to say — `--die-with-parent`
        // on one and a keeper on the other.
        Platform::Linux | Platform::MacOs => Held::nothing(),

        Platform::Windows => holding(child),
    }
}

/// What holds a child to the life of the server that started it, where the
/// platform's answer is something to hold.
///
/// Nothing at all on the two platforms whose answer is said elsewhere, which is
/// why this is a value rather than an `Option` of a handle: what a caller does
/// with one is keep it, and that is the same on all three.
#[derive(Debug)]
pub(crate) struct Held {
    #[cfg(windows)]
    #[expect(
        dead_code,
        reason = "\
        held rather than read: what ends the tree is this handle closing, so \
        there is nothing to ask it and nothing to do with it but keep it"
    )]
    job: Option<job::Job>,
}

impl Held {
    /// Nothing to hold, which is what the two platforms that say this elsewhere
    /// hand back — and what a Job that could not be made comes to.
    fn nothing() -> Held {
        Held {
            #[cfg(windows)]
            job: None,
        }
    }
}

/// `child` in a Job of its own — see [`held`], which is the whole of the why.
#[cfg(windows)]
fn holding(child: &Child) -> Held {
    use std::os::windows::io::AsRawHandle;

    let made = job::Job::killing_everything_in_it()
        .and_then(|job| job.take(child.as_raw_handle().cast()).map(|()| job));

    match made {
        Ok(job) => Held { job: Some(job) },
        Err(error) => {
            tracing::error!(
                %error,
                child = child.id(),
                "no Job could be made for this process, so it would outlive the server",
            );

            Held::nothing()
        }
    }
}

/// And where there is no Job to make — see the arm above.
///
/// Reached only by a test asking what a Windows build answers, on a machine
/// that is not one: there is no kernel object here to stand for the promise, so
/// what it answers is that it holds nothing.
#[cfg(not(windows))]
fn holding(_child: &Child) -> Held {
    Held::nothing()
}

/// Start `command` in a process group of its own, so that ending it is ending
/// everything it started.
///
/// The other half of what a keeper needs: a group is what it kills, and a
/// sandbox left in the server's own group is one that could not be killed
/// without killing the server with it.
///
/// A session's sandbox has one already and is not passed here — the terminal it
/// runs on is a session of its own, which [`crate::terminal::Terminal::spawn`]
/// makes with `setsid` so that the agent inside has a controlling terminal to
/// ask about itself. The compile server runs on no terminal, so it says this
/// for itself.
pub(crate) fn in_its_own_group(platform: Platform, command: &mut Command) {
    if platform != Platform::MacOs {
        return;
    }

    leading_a_group(command);
}

/// Ask for that group between the fork and the exec, where a syscall and
/// nothing else is allowed — the same window `Terminal::spawn` makes a
/// session's own terminal in.
///
/// One of the handful of calls in the server that a platform has to have its
/// own answer to, and Windows' answer is that it has none: there are no process
/// groups of this kind there, no `pre_exec` to ask for one in, and nothing on a
/// Windows machine is a Mac — so the arm below is compiled and never reached.
#[cfg(unix)]
fn leading_a_group(command: &mut Command) {
    use std::os::unix::process::CommandExt;

    unsafe {
        command.pre_exec(|| {
            rustix::process::setpgid(None, None)?;
            Ok(())
        });
    }
}

/// And where there is no group to lead — see the arm above.
#[cfg(not(unix))]
fn leading_a_group(_command: &mut Command) {}

/// Start one, and reap the shell that forked it.
fn kept(sandbox: u32, server: u32) -> io::Result<()> {
    let mut keeper = Command::new(SH);

    keeper
        .arg("-c")
        .arg(watching(sandbox, server))
        // A keeper has nothing to say and nothing to read. Said here rather
        // than in the script because it is the one thing about it that must
        // hold whatever the shell does with the loop: a copy of a session's
        // terminal held open out here would be one the server never reads the
        // end of, and a session that had long since exited would read as one
        // still running.
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    // In a session of its own — see this module's own documentation for what
    // that is for.
    in_a_session_of_its_own(&mut keeper);

    // Waited for, and what is waited for is not the keeper: the shell started
    // here forks the loop and exits at once, so this reaps a shell that has
    // already gone and the loop goes on with no parent to be reaped by.
    //
    // How it exited is worth reading all the same. It is the one thing that
    // would say the loop was never forked — a shell that would not parse what
    // it was handed — and the alternative is a keeper reported as started that
    // was never there.
    let forked = keeper.spawn()?.wait()?;

    if !forked.success() {
        return Err(io::Error::other(format!(
            "the shell that was to fork a keeper exited {forked}"
        )));
    }

    Ok(())
}

/// Put the keeper in a session of its own, in the same window between the fork
/// and the exec that [`leading_a_group`] asks in — and gated for the same
/// reason, which is written there.
#[cfg(unix)]
fn in_a_session_of_its_own(keeper: &mut Command) {
    use std::os::unix::process::CommandExt;

    unsafe {
        keeper.pre_exec(|| {
            rustix::process::setsid()?;
            Ok(())
        });
    }
}

/// And where there are no sessions to be put in — see the arm above.
#[cfg(not(unix))]
fn in_a_session_of_its_own(_keeper: &mut Command) {}

/// The keeper itself: a loop that ends when the server does, or when there is
/// nothing left to keep.
///
/// It says what it is in its first line, because the one place anybody will
/// ever read it is a process listing on their own Mac, where an unexplained
/// shell loop holding a pid is a thing to worry about.
fn watching(sandbox: u32, server: u32) -> String {
    format!(
        "\
# verkstead: nothing in process group {sandbox} outlives the server at {server}
{{
    while kill -0 {server} 2>/dev/null; do
        kill -0 -{sandbox} 2>/dev/null || exit 0
        sleep {EVERY}
    done

    kill -9 -{sandbox} 2>/dev/null
}} &
"
    )
}

/// Run wherever the suite is rather than on a Mac alone, for the reason each
/// test says: a keeper is a shell script and a process group, and both are the
/// same on either Unix. Not on Windows, which has neither — and no
/// [`in_its_own_group`] to make one with, so there would be nothing to assert.
#[cfg(all(test, unix))]
mod tests {
    use std::process::Child;
    use std::time::{Duration, Instant};

    use rustix::process::{Pid, test_kill_process};

    use super::*;

    /// How long a test waits for what a keeper does, which is a second's poll
    /// and whatever the machine running the suite is doing besides.
    const PATIENTLY: Duration = Duration::from_secs(20);

    /// And how long it waits to be sure a keeper has *not* done something,
    /// which only has to outlast two of its own polls.
    const LONG_ENOUGH: Duration = Duration::from_secs(4);

    /// A process a test started, killed and reaped when the test ends however
    /// it ends — so that a failed assertion leaves nothing running.
    struct Standing(Child);

    impl Standing {
        fn pid(&self) -> u32 {
            self.0.id()
        }

        /// Whether it has gone, asked the way the keeper's own loop asks: of
        /// the process itself where this is one of ours, and reaped first so
        /// that a zombie is not read as a process still running.
        fn gone(&mut self) -> bool {
            matches!(self.0.try_wait(), Ok(Some(_)))
        }
    }

    impl Drop for Standing {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }

    /// Something that will sit there until it is killed, standing in for the
    /// server a keeper is watching.
    fn standing(script: &str) -> Standing {
        Standing(
            Command::new(SH)
                .arg("-c")
                .arg(script)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .expect("a shell is on every machine this suite runs on"),
        )
    }

    /// And a stand-in sandbox: a process group of its own, with something in it
    /// that the process leading the group did not wait for — which is what a
    /// session's agent leaving a build running comes to.
    fn sandbox(started: &std::path::Path) -> Standing {
        let mut command = Command::new(SH);

        command
            .arg("-c")
            .arg(format!(
                "sleep 60 & echo $! > {}; exec sleep 60",
                started.display()
            ))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        in_its_own_group(Platform::MacOs, &mut command);

        Standing(command.spawn().expect("a shell is on every machine"))
    }

    /// The pid the stand-in sandbox wrote down for what it started, waited for
    /// — the file appears a moment after the shell does.
    fn what_it_started(at: &std::path::Path) -> Pid {
        let until = Instant::now() + PATIENTLY;

        while Instant::now() < until {
            if let Ok(said) = std::fs::read_to_string(at)
                && let Ok(pid) = said.trim().parse::<i32>()
                && let Some(pid) = Pid::from_raw(pid)
            {
                return pid;
            }

            std::thread::sleep(Duration::from_millis(20));
        }

        panic!("the stand-in sandbox never said what it started");
    }

    /// Whether a process nothing here is the parent of is still there.
    fn still_there(pid: Pid) -> bool {
        test_kill_process(pid).is_ok()
    }

    /// A keeper ends the whole group the moment the server is gone, however it
    /// went — and `kill -9` of the server is *however it went* at its worst:
    /// there is no path through the server's own code at all, which is what
    /// makes it the same answer as Exit off the tray.
    ///
    /// Run on whatever machine the suite is on rather than on a Mac alone. The
    /// keeper is a shell script and a process group, and both are the same on
    /// either platform; what is macOS-only is that anything starts one, which
    /// is [`keep`]'s own arm and is asserted below.
    #[test]
    fn a_sandbox_and_what_it_started_go_when_the_server_does() {
        let dir = tempfile::tempdir().unwrap();
        let started = dir.path().join("what-it-started");

        let server = standing("exec sleep 60");
        let mut sandbox = sandbox(&started);
        let inside = what_it_started(&started);

        keep(Platform::MacOs, sandbox.pid(), server.pid());

        // Nothing has happened to either of them while the server is up, which
        // is most of what a keeper does.
        std::thread::sleep(LONG_ENOUGH);
        assert!(
            !sandbox.gone(),
            "the sandbox is running and so is the server"
        );
        assert!(still_there(inside), "and so is what the sandbox started");

        drop(server);

        let until = Instant::now() + PATIENTLY;

        while Instant::now() < until && (!sandbox.gone() || still_there(inside)) {
            std::thread::sleep(Duration::from_millis(100));
        }

        assert!(
            sandbox.gone(),
            "the server has gone, so the sandbox it started goes with it",
        );
        assert!(
            !still_there(inside),
            "and so does everything else in its process group, which is what \
             the session left running",
        );
    }

    /// And on the platform whose sandbox says this for itself, nothing is
    /// started and nothing happens: `--die-with-parent` is what ends a session
    /// there, and a keeper beside it would be a second mechanism for the one
    /// promise.
    #[test]
    fn linux_is_left_to_the_flag_it_already_has() {
        let dir = tempfile::tempdir().unwrap();
        let started = dir.path().join("what-it-started");

        let server = standing("exec sleep 60");
        let mut sandbox = sandbox(&started);
        let inside = what_it_started(&started);

        keep(Platform::Linux, sandbox.pid(), server.pid());

        drop(server);
        std::thread::sleep(LONG_ENOUGH);

        assert!(
            !sandbox.gone(),
            "nothing was started to notice the server going",
        );
        assert!(still_there(inside), "so nothing was ended either");
    }
}

/// And the other machine's own, which asks the one thing a Job is for: that
/// letting go of it ends what is inside it.
///
/// Asked of Windows alone because a Job is a Windows object — see [`job`],
/// which is why that module is a `cfg` rather than a value. The `windows-2025`
/// job is where these run.
#[cfg(all(test, windows))]
mod tests {
    use std::time::{Duration, Instant};

    use super::*;

    /// How long to wait for a kill the kernel makes on its own account, which
    /// is something it gets round to rather than something that has already
    /// happened when the handle closes.
    const PATIENTLY: Duration = Duration::from_secs(20);

    /// Something that will sit there until something ends it, standing in for
    /// the Compile Server this is really about.
    fn standing() -> Child {
        Command::new("cmd.exe")
            .args(["/c", "ping -n 120 127.0.0.1 >nul"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("cmd.exe is part of Windows")
    }

    /// Whether process `running` is no longer on this machine, waited for.
    ///
    /// Asked of Windows rather than of the [`Child`], because what is being
    /// tested is a kill nothing in this process made: `tasklist` is what says
    /// whether an id is anybody.
    fn gone(running: u32) -> bool {
        let deadline = Instant::now() + PATIENTLY;

        while Instant::now() < deadline {
            let listed = Command::new("tasklist.exe")
                .args(["/fi", &format!("PID eq {running}")])
                .stdin(Stdio::null())
                .output()
                .expect("tasklist is part of Windows");

            if !String::from_utf8_lossy(&listed.stdout).contains(&running.to_string()) {
                return true;
            }

            std::thread::sleep(Duration::from_millis(200));
        }

        false
    }

    /// The whole of what a Job buys: what was put in one goes when the last
    /// handle to it closes.
    ///
    /// Which is what a server exiting does, however it exits — the tray's
    /// **Exit**, a `kill`, a crash — so this is the promise the Compile Server
    /// is held to on this platform, asked by letting go on purpose.
    #[test]
    fn what_is_in_a_job_goes_when_the_last_handle_to_it_closes() {
        let mut child = standing();
        let running = child.id();

        let held = held(Platform::Windows, &child);

        assert!(
            !gone_already(running),
            "the process should still be running while the Job is held",
        );

        drop(held);

        assert!(
            gone(running),
            "letting go of the Job should have ended process {running}",
        );

        let _ = child.wait();
    }

    /// And the two platforms that say this elsewhere hold nothing here, so
    /// letting go of what they answered ends nothing.
    #[test]
    fn the_platforms_whose_sandbox_says_it_hold_nothing() {
        for platform in [Platform::Linux, Platform::MacOs] {
            let mut child = standing();
            let running = child.id();

            drop(held(platform, &child));

            assert!(
                !gone_already(running),
                "{platform:?} has its own answer to this, so nothing here should \
                 have ended process {running}",
            );

            let _ = child.kill();
            let _ = child.wait();
        }
    }

    /// Whether `running` has gone by now, asked once rather than waited for —
    /// which is what a test asserting that something has *not* happened wants.
    fn gone_already(running: u32) -> bool {
        let listed = Command::new("tasklist.exe")
            .args(["/fi", &format!("PID eq {running}")])
            .stdin(Stdio::null())
            .output()
            .expect("tasklist is part of Windows");

        !String::from_utf8_lossy(&listed.stdout).contains(&running.to_string())
    }
}
