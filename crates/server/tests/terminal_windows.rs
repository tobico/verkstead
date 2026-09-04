//! The console Verkstead opens a session on where the terminal is a
//! pseudoconsole, asked the way `terminal.rs` asks the other arm: by running
//! something on one and reading back what it says about the console it is on.
//!
//! The probe is `mode con`, which is how a program on Windows asks the console
//! underneath it how big it is — `stty` on the other arm — and `cmd.exe` is the
//! shortest way to it.
//!
//! **The width is what is asserted about, rather than the height.** A console
//! has a window and a buffer behind it, and `mode con` reports the buffer; how
//! tall a pseudoconsole's buffer is, is the console host's own business and not
//! something Verkstead asks for. How wide it is, is exactly what was asked for
//! — so a hundred columns is both the proof that the process is on Verkstead's
//! console rather than on whatever console the tests were started from, and the
//! proof that a resize reached it.
//!
//! And two things a pseudo-terminal needs no test for, because on Unix the
//! kernel is what does them: that reading ends when the session has gone, and
//! that a session that is dropped takes what it started with it. Both are this
//! arm's own work — a console closed behind the process on it, and a Job Object
//! — so both are asked here.
#![cfg(windows)]

use std::process::Command;
use std::time::{Duration, Instant};

use verkstead_server::sandbox::Rendering;
use verkstead_server::terminal::{COLUMNS, Terminal};

/// How long to wait for something the probe says. Generously long: what is
/// being waited on is a process starting, and a first `powershell.exe` on a
/// cold runner is not quick.
const PATIENCE: Duration = Duration::from_secs(60);

/// The shell every Windows machine has, by name: `CreateProcessW` finds it in
/// the system directory whatever the environment says.
const CMD: &str = "cmd.exe";

/// And the one every Windows machine has that can start a process and say what
/// its id was — see [`a_child_that_is_dropped_takes_what_it_started_with_it`].
const POWERSHELL: &str = "powershell.exe";

/// What the environment of a probe is: the names nothing on Windows runs
/// without.
///
/// Said rather than inherited, because a rendering is the whole of what a
/// process is handed — see [`Rendering`] — and `cmd.exe` with no `SystemRoot`
/// is a shell that will not start. The test's own values, because what is being
/// asked about here is the console rather than what a session may reach.
const NEEDED: [&str; 10] = [
    "ComSpec",
    "PATH",
    "PATHEXT",
    "SystemDrive",
    "SystemRoot",
    "TEMP",
    "TMP",
    "USERPROFILE",
    "APPDATA",
    "LOCALAPPDATA",
];

/// What a session is on, said by the session.
///
/// A process with no console at all cannot answer `mode con` — there is no
/// `CON` device to ask — and one on the console the tests were started from
/// would answer with that console's width. So an answer of [`COLUMNS`] is a
/// console of this terminal's own, opened at the size Verkstead opens one at.
#[tokio::test]
async fn a_session_is_started_on_a_console_of_its_own() {
    let mut terminal = Terminal::open().expect("this machine has pseudoconsoles");

    let mut child = terminal
        .spawn(&shell("mode con & echo asked"))
        .expect("a shell to run on it");

    let said = until(&terminal, |said| said.contains("asked")).await;

    assert_eq!(
        widths(&said).last().copied(),
        Some(COLUMNS),
        "the session should be on a console of its own, {COLUMNS} columns across, \
         and it said: {said:?}"
    );

    let _ = child.wait().await;
}

/// The size a session starts at, and the size it is at afterwards.
///
/// Both read from inside, because the size a console is set to and the size the
/// program on it is told about are two different things: a resize nothing was
/// told about is a window that changed for the watcher and for nobody else.
///
/// The probe asks again and again rather than once, because there is no signal
/// on this platform to wait for: a program on a console learns that its window
/// changed by asking the console.
#[tokio::test]
async fn resizing_a_console_is_something_the_session_on_it_is_told() {
    let mut terminal = Terminal::open().expect("this machine has pseudoconsoles");

    let mut child = terminal
        .spawn(&shell(
            "for /l %i in (1,0,1) do @(mode con & ping -n 2 127.0.0.1 >nul)",
        ))
        .expect("a shell to run on it");

    let said = until(&terminal, |said| widths(said).contains(&COLUMNS)).await;
    assert!(
        widths(&said).contains(&COLUMNS),
        "a session should start on a console {COLUMNS} columns across, \
         and it said: {said:?}"
    );

    terminal.resize(132, 43).expect("the window to be resized");

    let said = until(&terminal, |said| widths(said).contains(&132)).await;
    assert!(
        widths(&said).contains(&132),
        "the session should have been told its window is now 132 across, \
         and it said: {said:?}"
    );

    let _ = child.start_kill();
    let _ = child.wait().await;
}

/// What the relay reads, and what says there is nothing left to read.
///
/// The whole of the first criterion in one loop, because on this platform the
/// two are one thing: a pseudoconsole's host keeps writing until the console is
/// closed, and the console is closed once the process on it has gone. So a read
/// that answers `Ok(0)` is a session that ended, and the text before it is what
/// the session printed.
#[tokio::test]
async fn what_a_session_printed_arrives_and_then_the_reading_ends() {
    let mut terminal = Terminal::open().expect("this machine has pseudoconsoles");

    let mut child = terminal
        .spawn(&shell("echo printed-on-a-console"))
        .expect("a shell to run on it");

    let said = until(&terminal, |said| said.contains("printed-on-a-console")).await;

    assert!(
        said.contains("printed-on-a-console"),
        "what the session printed should reach the read, and it said: {said:?}"
    );

    let ended = tokio::time::timeout(PATIENCE, drained(&terminal))
        .await
        .expect("the reading to end once the console has been closed behind the session");

    assert_eq!(
        ended, 0,
        "a read after the session has gone and the console is closed is the end \
         of what there is to read"
    );

    let _ = child.wait().await;
}

/// And what an ended session leaves behind, which is nothing.
///
/// A child that started a child of its own: dropping the [`Child`] closes the
/// last handle to the Job both are in, and a Job with no handles left kills
/// what is in it. Asked of Windows itself rather than of Verkstead — `tasklist`
/// is what says whether a process id is anybody — because what is being tested
/// is that the promise holds outside this process.
///
/// [`Child`]: verkstead_server::terminal::Child
#[tokio::test]
async fn a_child_that_is_dropped_takes_what_it_started_with_it() {
    let mut terminal = Terminal::open().expect("this machine has pseudoconsoles");

    let mut probe = probing(POWERSHELL);

    probe
        .arg("-NoProfile")
        .arg("-NonInteractive")
        .arg("-Command")
        .arg(
            "$started = Start-Process -FilePath powershell.exe \
             -ArgumentList '-NoProfile','-Command','Start-Sleep 600' -PassThru; \
             Write-Output ('grandchild ' + $started.Id); Start-Sleep 600",
        );

    let child = terminal.spawn(&probe).expect("a shell to run on it");

    let session = child.id().expect("a started session has a process id");

    let said = until(&terminal, |said| said.contains("grandchild ")).await;

    let grandchild = said
        .split_once("grandchild ")
        .and_then(|(_, rest)| {
            let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();

            digits.parse::<u32>().ok()
        })
        .unwrap_or_else(|| panic!("the probe should have said what it started: {said:?}"));

    drop(child);

    for running in [session, grandchild] {
        assert!(
            gone(running).await,
            "a dropped child should have taken process {running} with it, and \
             the probe said: {said:?}"
        );
    }
}

/// The probe as a terminal is given one: a shell running `script`, with the
/// environment nothing on Windows runs without — see [`NEEDED`].
fn shell(script: &str) -> Rendering {
    let mut probe = probing(CMD);

    probe.arg("/c").arg(script);

    probe
}

/// And what every probe here has in common: `program`, and the environment it
/// takes to run at all.
fn probing(program: &str) -> Rendering {
    let mut probe = Rendering::running(program);

    for name in NEEDED {
        if let Some(value) = std::env::var_os(name) {
            probe.set(name, value);
        }
    }

    probe
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

/// Read the terminal to its end, and hand back what the last read answered.
async fn drained(terminal: &Terminal) -> usize {
    let mut buffer = [0u8; 4096];

    loop {
        let read = terminal
            .read(&mut buffer)
            .await
            .expect("the terminal to be readable");

        if read == 0 {
            return read;
        }
    }
}

/// Whether process `running` is no longer on this machine, waited for.
///
/// Waited for rather than asked once: a Job kills what is in it, and a kill is
/// something the machine gets round to rather than something that has already
/// happened when the handle closes.
async fn gone(running: u32) -> bool {
    let deadline = Instant::now() + PATIENCE;

    while Instant::now() < deadline {
        let listed = Command::new("tasklist.exe")
            .arg("/fi")
            .arg(format!("PID eq {running}"))
            .output()
            .expect("tasklist is part of Windows");

        // Which is what `tasklist` says of a filter that matched nothing — and
        // the id itself is what it prints when it matched.
        if !String::from_utf8_lossy(&listed.stdout).contains(&running.to_string()) {
            return true;
        }

        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    false
}
