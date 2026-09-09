//! The launcher: a session's console really made on the far side of the account
//! boundary, by a verb of Verkstead's own binary running as the session
//! account.
//!
//! **Why here rather than in the server crate.** The launcher is a verb of the
//! `verkstead` binary, and the server crate builds no binary at all — its test
//! harness is the only executable a test there could point at, and there is no
//! launcher verb in one. This crate builds the real thing, and the
//! `windows-2025` job runs the whole workspace. `crates/cli/tests/sandbox_windows.rs`
//! is here for the same reason.
//!
//! **What it needs of the machine, and what it says when it has not got it.**
//! The session account, which is an elevated verb's to create — see
//! `crates/server/tests/account_windows.rs`, whose refusal this one repeats. And
//! a copy of the binary somewhere the account can read: the entries a Surface
//! comes to are a later task's, so this file puts one under `C:\Users\Public`,
//! whose own access-control entries grant an interactive logon what it needs and
//! whose contents are nobody's secret. Every copy is taken away again.
//!
//! **What is asserted is the console.** A `mode con` on the far side reports a
//! width, and the width is the one Verkstead asked for rather than whatever
//! console the tests were started from — which is the same probe
//! `crates/server/tests/terminal_windows.rs` puts to a console made on *this*
//! side, asked of one made on the other. Beside it: that the exit code coming
//! back is the *session's* rather than the launcher's, and that the Job holds
//! both.
#![cfg(windows)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

use verkstead_server::platform;
use verkstead_server::sandbox::Rendering;
use verkstead_server::sandbox::account::Logon;
use verkstead_server::sandbox::account::machine::Account;
use verkstead_server::settings::Settings;
use verkstead_server::terminal::{COLUMNS, Terminal};

/// How long to wait for something the probe says. Generously long: what is
/// being waited on is a logon, a profile that may be loading for the first time,
/// a launcher, a console and a shell.
const PATIENCE: Duration = Duration::from_secs(120);

/// The shell every Windows machine has, in the directory every account can read
/// it out of.
const CMD: &str = r"C:\Windows\System32\cmd.exe";

/// And where a probe starts and where the copies go: a directory every
/// interactive logon on this machine may read, which is what the boundary's own
/// entries will be doing for a session's Worktree a task from now.
const PUBLIC: &str = r"C:\Users\Public";

/// And the shell this side asks the machine its questions with — Windows
/// PowerShell by its own path, which every machine has wherever `pwsh` is.
const POWERSHELL: &str = r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe";

/// The environment a probe is handed. Said rather than inherited, because a
/// rendering is the whole of what a process is handed and the server's own
/// environment is full of paths this account is refused.
const NEEDED: [(&str, &str); 4] = [
    ("SystemRoot", r"C:\Windows"),
    ("SystemDrive", "C:"),
    ("PATH", r"C:\Windows\System32;C:\Windows"),
    ("PATHEXT", ".COM;.EXE;.BAT;.CMD"),
];

/// The account this machine's Verkstead runs its sessions as, or a failure
/// saying what to run.
fn the_account() -> Logon {
    let data_dir =
        platform::data_dir(None).expect("this machine has somewhere for a Data Directory");
    let settings = Settings::in_data_dir(&data_dir);

    match Account::on_this_machine(&data_dir, &settings.secrets()) {
        Ok(account) => Logon::of(account.name(), account.password()),
        Err(missing) => panic!(
            "this suite starts a launcher as the session account and there is not one: \
             {missing}\n\nThe Data Directory it asked about is {}.",
            data_dir.display(),
        ),
    }
}

/// Verkstead's own binary, put where the session account can read it, and taken
/// away again.
///
/// **Which is what a rendering will be doing for itself a task from now**: a
/// session is handed the server's image bound read-only inside its own reach,
/// because the path the server is really running out of is under the human's
/// profile and the account is refused that. This is that, done by hand and with
/// no entries written — see this file's own documentation.
struct Bound(PathBuf);

impl Bound {
    /// One copy, under a name nothing else is at.
    fn made() -> Bound {
        /// What makes one test's copy different from the next one's: these run
        /// side by side, and a name they shared would be a file one of them was
        /// copying while another was running it.
        static MADE: AtomicU32 = AtomicU32::new(0);

        let at = PathBuf::from(PUBLIC).join(format!(
            "verkstead-launcher-test-{}-{}",
            std::process::id(),
            MADE.fetch_add(1, Ordering::Relaxed),
        ));

        std::fs::create_dir_all(&at).unwrap_or_else(|error| {
            panic!("{PUBLIC} should be a directory this test can make one under: {error}")
        });

        let image = at.join("verkstead.exe");

        std::fs::copy(env!("CARGO_BIN_EXE_verkstead"), &image)
            .expect("the binary this crate builds should be copyable");

        Bound(at)
    }

    /// Where the copy is.
    fn image(&self) -> PathBuf {
        self.0.join("verkstead.exe")
    }

    /// And the directory it is in, which is also where a probe starts: an
    /// account with nowhere it may be is one a process will not start for.
    fn directory(&self) -> &Path {
        &self.0
    }
}

/// Taken away again, waited for.
///
/// **Waited for rather than tried once**, which is what a first run of this
/// suite taught it: dropping the [`Child`] kills the Job the launcher is in,
/// and a kill is something the machine gets round to — so for a moment after
/// the test has finished the image is still open and the directory still
/// refuses to go. A copy left behind is litter under a directory that is not
/// this suite's, on somebody's own machine.
///
/// [`Child`]: verkstead_server::terminal::Child
impl Drop for Bound {
    fn drop(&mut self) {
        let deadline = Instant::now() + PATIENCE;

        while Instant::now() < deadline {
            if std::fs::remove_dir_all(&self.0).is_ok() || !self.0.exists() {
                return;
            }

            std::thread::sleep(Duration::from_millis(100));
        }

        // Said rather than asserted: a copy that would not go is worth knowing
        // about and is not what any test here is asking.
        eprintln!("the copy at {} would not be removed", self.0.display());
    }
}

/// A rendering a launcher will start, of `cmd.exe` running `script`.
fn probe(bound: &Bound, script: &str) -> Rendering {
    let mut rendering = Rendering::running(CMD);

    rendering
        .arg("/d")
        .arg("/c")
        .arg(script)
        .starting_in(bound.directory())
        .as_account(the_account())
        .launched_by(bound.image());

    for (name, value) in NEEDED {
        rendering.set(name, value);
    }

    rendering
}

/// The whole of what a launcher is for: a console made on the far side of the
/// boundary, and a marker printed on it by the program the launcher started
/// coming back off the console's pipe.
#[tokio::test]
async fn a_marker_printed_on_a_console_the_launcher_made_comes_back() {
    let bound = Bound::made();
    let mut terminal = Terminal::open().expect("this machine has pseudoconsoles");

    let _child = terminal
        .spawn(&probe(&bound, "echo the-launcher-made-this"))
        .expect("a launcher to start as the session account");

    let said = until(&terminal, |said| said.contains("the-launcher-made-this")).await;

    assert!(said.contains("the-launcher-made-this"), "it said {said:?}");
}

/// And it is a console rather than a pipe: `mode con` answers, which a process
/// with no console cannot, and the width it answers with is the one Verkstead
/// opened the terminal at rather than whatever the tests were started from.
///
/// Then the same question again after a resize, which is what proves a resize
/// crossed the boundary: there is no console in this process to resize at all,
/// so what changed the answer is a line on the launcher's own channel.
#[tokio::test]
async fn a_resize_reaches_the_console_the_launcher_made() {
    const NARROWER: u16 = 61;

    let bound = Bound::made();
    let mut terminal = Terminal::open().expect("this machine has pseudoconsoles");

    // Asked over and over rather than once and once again, the way the same
    // question is asked of a console made on this side: a session that reports
    // its window on a loop needs no keystroke to be got at, and a keystroke is
    // one more thing that would have had to cross the boundary before this
    // question could be answered.
    let mut child = terminal
        .spawn(&probe(
            &bound,
            "for /l %i in (1,0,1) do @(mode con & ping -n 2 127.0.0.1 >nul)",
        ))
        .expect("a launcher to start as the session account");

    let said = until(&terminal, |said| widths(said).contains(&COLUMNS)).await;

    assert!(
        widths(&said).contains(&COLUMNS),
        "the console the launcher made should be {COLUMNS} columns across, and it said: {said:?}",
    );

    terminal
        .resize(NARROWER, 24)
        .expect("a resize to reach the launcher");

    let said = until(&terminal, |said| widths(said).contains(&NARROWER)).await;

    assert!(
        widths(&said).contains(&NARROWER),
        "the resize should have reached the console on the far side of the boundary, \
         and it said: {said:?}",
    );

    let _ = child.start_kill();
    let _ = child.wait().await;
}

/// The exit code that comes back is the **session's** rather than the
/// launcher's, which is the whole reason the launcher has a channel: a launcher
/// exits well having started a program that did not.
#[tokio::test]
async fn the_exit_code_that_comes_back_is_the_one_the_program_exited_with() {
    const PARTICULAR: i32 = 7;

    let bound = Bound::made();
    let mut terminal = Terminal::open().expect("this machine has pseudoconsoles");

    let mut child = terminal
        .spawn(&probe(&bound, &format!("exit {PARTICULAR}")))
        .expect("a launcher to start as the session account");

    let ended = tokio::time::timeout(PATIENCE, child.wait())
        .await
        .expect("the session to end")
        .expect("the session's exit to be readable");

    assert_eq!(
        ended.code(),
        Some(PARTICULAR),
        "the session's own exit should have come back up the channel rather than the \
         launcher's, which is 0",
    );
}

/// And the Job holds the launcher and the process under it: closing it takes
/// both.
#[tokio::test]
async fn the_job_holds_the_launcher_and_what_it_started() {
    let bound = Bound::made();
    let mut terminal = Terminal::open().expect("this machine has pseudoconsoles");

    let child = terminal
        .spawn(&probe(&bound, "echo the-session-is-running & pause"))
        .expect("a launcher to start as the session account");

    let launcher = child.id().expect("a started launcher has a process id");

    // The shell's own id, asked of the machine rather than printed by it: what
    // `cmd.exe` would echo is a variable this rendering does not set, and what
    // is wanted is the process the launcher started rather than the launcher.
    let _ = until(&terminal, |said| said.contains("the-session-is-running")).await;

    let session = under(launcher)
        .unwrap_or_else(|| panic!("the launcher {launcher} should have a process under it by now"));

    drop(child);

    for running in [launcher, session] {
        assert!(
            gone(running).await,
            "a dropped child should have taken process {running} with it",
        );
    }
}

/// The one process whose parent is `launcher`, asked of the machine.
///
/// Asked from out here rather than printed from in there, and asked with
/// PowerShell rather than `wmic`: this side runs as the human and may enumerate
/// what it likes, and `wmic` is not on a Windows 11 machine any more.
fn under(launcher: u32) -> Option<u32> {
    let listed = Command::new(POWERSHELL)
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            &format!(
                "(Get-CimInstance Win32_Process -Filter 'ParentProcessId={launcher}').ProcessId"
            ),
        ])
        .output()
        .ok()?;

    String::from_utf8_lossy(&listed.stdout)
        .lines()
        .filter_map(|line| line.trim().parse::<u32>().ok())
        .next()
}

/// Every width `mode con` has reported in `said`, in the order it reported
/// them.
///
/// Read out of the whole text rather than line by line, because what arrives
/// off a console is a drawing of one: the numbers are in it, with whatever the
/// console host wrote around them.
fn widths(said: &str) -> Vec<u16> {
    said.match_indices("Columns:")
        .filter_map(|(at, field)| {
            let rest = &said[at + field.len()..];
            let digits: String = rest
                .chars()
                .skip_while(|character| character.is_whitespace())
                .take_while(char::is_ascii_digit)
                .collect();

            digits.parse().ok()
        })
        .collect()
}

/// Read the terminal until what has arrived on it satisfies `enough`, and hand
/// back the whole of it — or give up, saying what did arrive.
async fn until(terminal: &Terminal, enough: impl Fn(&str) -> bool) -> String {
    let deadline = Instant::now() + PATIENCE;
    let mut said = String::new();
    let mut buffer = [0u8; 4096];

    while !enough(&said) {
        let read = tokio::time::timeout(
            deadline.saturating_duration_since(Instant::now()),
            terminal.read(&mut buffer),
        )
        .await
        .unwrap_or_else(|_| panic!("the session never said it. It said: {said:?}"))
        .expect("the terminal to be readable");

        assert!(read > 0, "the session ended having said: {said:?}");

        said.push_str(&String::from_utf8_lossy(&buffer[..read]));
    }

    said
}

/// Whether process `running` is no longer on this machine, waited for.
///
/// Waited for rather than asked once: a Job kills what is in it, and a kill is
/// something the machine gets round to rather than something that has already
/// happened when the handle closes.
async fn gone(running: u32) -> bool {
    let deadline = Instant::now() + PATIENCE;

    while Instant::now() < deadline {
        let listed = Command::new(r"C:\Windows\System32\tasklist.exe")
            .arg("/fi")
            .arg(format!("PID eq {running}"))
            .output()
            .expect("tasklist is part of Windows");

        if !String::from_utf8_lossy(&listed.stdout).contains(&running.to_string()) {
            return true;
        }

        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    false
}
