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
//! On the platforms that have a pseudo-terminal to open. What Windows has
//! instead is a pseudoconsole, and the same two questions asked of one are in
//! `terminal_windows.rs` — `mode con` there rather than `stty` here.
#![cfg(unix)]

use std::time::{Duration, Instant};

use verkstead_server::sandbox::Rendering;
use verkstead_server::terminal::{COLUMNS, ROWS, Terminal};
use verkstead_server::terminals::busy;

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
        .spawn(&shell(
            ": < /dev/tty && printf 'controlling %s\\n' \"$(tty)\"",
        ))
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
        .spawn(&shell(
            "trap 'stty size' WINCH; stty size; while :; do sleep 0.05; done",
        ))
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

/// The probe as a terminal is given one: a shell running `script`.
///
/// A rendering carries the whole of the environment a process is handed — see
/// [`Rendering`] — so a `PATH` said here is the difference between a probe that
/// can call `stty` and one that cannot. The test's own, because what is being
/// asked about is the terminal rather than what a session may reach.
fn shell(script: &str) -> Rendering {
    let mut probe = Rendering::running(SH);

    probe.arg("-c").arg(script);

    if let Some(path) = std::env::var_os("PATH") {
        probe.set("PATH", path);
    }

    probe
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

/// What the server reads off a terminal to know whether somebody is working in
/// it: the shell at an idle prompt, and whatever it is running while it runs
/// (ADR 0019, *Tabs and groups*).
///
/// Read here rather than of a Conversation's terminal because this is the layer
/// the reading is made at — a pseudo-terminal with a shell on it, which is what
/// a Conversation's terminal is once the Sandbox around it is taken away. The
/// whole of it end to end, sandbox and all, is in `sessions.rs`.
///
/// The probe is `/bin/sh -i`, because what puts a process group in front of a
/// terminal is a shell with job control, and an interactive shell is the
/// shortest way to one.
#[tokio::test]
async fn a_terminal_reads_its_foreground_as_the_shell_and_then_as_what_it_runs() {
    let mut terminal = Terminal::open().expect("this machine has pseudo-terminals");
    let mut child = terminal
        .spawn(&interactive())
        .expect("a shell to run on it");

    at_a_prompt(&terminal).await;

    assert!(
        !busy::of(&terminal, &busy::named(SH)),
        "a shell at its prompt is a terminal nobody is working in",
    );

    running_in(&terminal).await;

    assert!(
        busy::of(&terminal, &busy::named(SH)),
        "a shell running something is a terminal somebody is working in",
    );

    let _ = child.start_kill();
    let _ = child.wait().await;
}

/// And the same with a process between the shell and the terminal, which is
/// what a Conversation's terminal really has: the child the server holds is the
/// sandbox wrapper, and the shell is its grandchild — behind the worktree's dev
/// shell as well where its flake has one, which is a second wrapper of the same
/// shape.
///
/// This is the whole reason the reading is by name. The wrapper is the pty's
/// session leader and the process the server knows the pid of, and the shell's
/// own number never reaches this side — so what is asserted here is that the
/// answer does not change for the layers in between.
#[tokio::test]
async fn a_shell_behind_a_wrapper_is_read_exactly_as_one_without() {
    let mut terminal = Terminal::open().expect("this machine has pseudo-terminals");

    // The wrapper stays alive with the shell under it rather than exec'ing into
    // it, which is what `bwrap` and `nix develop --command` both do. In the
    // foreground, where the shell can take the terminal for itself — and with
    // something after it, which is what stops the wrapper exec'ing into its
    // last command the way every shell does.
    let mut child = terminal
        .spawn(&shell(&format!("{SH} -i; :")))
        .expect("a shell to run on it");

    at_a_prompt(&terminal).await;

    assert!(
        !busy::of(&terminal, &busy::named(SH)),
        "a shell behind a wrapper is still a shell at its prompt",
    );

    running_in(&terminal).await;

    assert!(
        busy::of(&terminal, &busy::named(SH)),
        "and is still read as busy by what it is running rather than by what \
         started it",
    );

    let _ = child.start_kill();
    let _ = child.wait().await;
}

/// The probe shell, interactive — which is how a terminal's own is started, and
/// what makes it take the terminal's foreground for itself.
fn interactive() -> Rendering {
    let mut probe = Rendering::running(SH);

    probe.arg("-i");

    if let Some(path) = std::env::var_os("PATH") {
        probe.set("PATH", path);
    }

    probe
}

/// Wait until the shell has run something, which is the moment it is known to
/// have taken the terminal's foreground for itself.
///
/// Asked by having it print rather than by waiting a while: a shell that has
/// answered has started, and a sleep long enough to be sure on a loaded machine
/// is a sleep every run pays.
///
/// The word is put together by the shell rather than written in the line,
/// because the line is echoed by the terminal the moment it is written — what
/// is waited for has to be something only the shell could have said.
async fn at_a_prompt(terminal: &Terminal) {
    terminal
        .write(b"printf 'at a %s\\n' prompt\n")
        .await
        .expect("the terminal to take what was typed");

    until(terminal, |said| said.contains("at a prompt")).await;
}

/// And then have it run something, waiting until the reading says so.
///
/// Polled rather than assumed: what is being waited for is the shell putting a
/// child in front of the terminal, which is a thing it does after it has read
/// the line. A `sleep` long enough to outlast the polling, because what it is
/// standing for is a build.
async fn running_in(terminal: &Terminal) {
    terminal
        .write(b"sleep 30\n")
        .await
        .expect("the terminal to take what was typed");

    let deadline = Instant::now() + PATIENCE;

    while !busy::of(terminal, &busy::named(SH)) {
        assert!(
            Instant::now() < deadline,
            "the shell never put anything in front of the terminal",
        );

        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}
