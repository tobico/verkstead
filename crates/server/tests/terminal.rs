//! The terminal Verkstead opens a session on, asked by running something on one
//! and reading back what it says about the terminal it is on.
//!
//! Nothing here reads the flags the pair was opened with, for the reason the
//! sandbox's own tests read no bwrap arguments: the flags are what is being
//! tested. What settles whether a session is on a terminal is a process on it
//! asking, and what settles whether a resize reached the session is the session
//! being asked again afterwards.
//!
//! The probe is a shell, because `stty` is how a process asks the terminal
//! underneath it how big it is, and a shell is the shortest way to `stty`.
//!
//! On the platforms that have a pseudo-terminal to open. Windows has not, and
//! the whole of what it has instead is a `Terminal::open` that refuses — see
//! `terminal::absent`, where there is nothing here left to ask.
#![cfg(unix)]

use std::time::{Duration, Instant};

use verkstead_server::terminal::{COLUMNS, ROWS, Terminal};

/// How long to wait for something the probe says. Generously long: what is
/// being waited on is a process starting.
const PATIENCE: Duration = Duration::from_secs(30);

/// A shell every machine this runs on has, at the one path the sandbox's own
/// surface is certain to have one at.
const SH: &str = "/bin/sh";

/// What a session is on, said by the session.
///
/// `/dev/tty` is the controlling terminal and nothing else: a process that has
/// none cannot open it, whatever its three streams happen to be. So a probe
/// that can is one Verkstead handed a terminal of its own rather than a pipe
/// that happens to look like one.
#[tokio::test]
async fn a_session_is_started_on_a_terminal_of_its_own() {
    let mut terminal = Terminal::open().expect("this machine has pseudo-terminals");

    let mut child = terminal
        .spawn(
            tokio::process::Command::new(SH)
                .arg("-c")
                .arg(": < /dev/tty && printf 'controlling %s\\n' \"$(tty)\"")
                .kill_on_drop(true),
        )
        .expect("a shell to run on it");

    let said = until(&terminal, |said| said.contains("controlling ")).await;

    assert!(
        said.contains("controlling /dev/pts/"),
        "the session should be on a pseudo-terminal that is its own controlling \
         terminal, and it said: {said:?}"
    );

    let _ = child.wait().await;
}

/// The size a session starts at, and the size it is at afterwards.
///
/// Both read from inside, because the size a terminal is set to and the size a
/// process on it is told about are two different things: the second is a signal
/// the kernel sends, and a resize nothing was told about is a window that
/// changed for the watcher and for nobody else.
#[tokio::test]
async fn resizing_a_terminal_is_something_the_session_on_it_is_told() {
    let mut terminal = Terminal::open().expect("this machine has pseudo-terminals");

    // The trap before the first answer, and not the other way round: the size
    // arriving is what says the probe is ready to be resized, and a window
    // changed before anything was listening is a signal nothing hears.
    let mut child = terminal
        .spawn(
            tokio::process::Command::new(SH)
                .arg("-c")
                .arg("trap 'stty size' WINCH; stty size; while :; do sleep 0.05; done")
                .kill_on_drop(true),
        )
        .expect("a shell to run on it");

    let started = format!("{ROWS} {COLUMNS}");
    let said = until(&terminal, |said| said.contains(&started)).await;
    assert!(
        said.contains(&started),
        "a session should start on a terminal {COLUMNS} by {ROWS}, and it said: {said:?}"
    );

    terminal.resize(132, 43).expect("the window to be resized");

    let said = until(&terminal, |said| said.contains("43 132")).await;
    assert!(
        said.contains("43 132"),
        "the session should have been told its window is now 132 by 43, \
         and it said: {said:?}"
    );

    let _ = child.start_kill();
    let _ = child.wait().await;
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
