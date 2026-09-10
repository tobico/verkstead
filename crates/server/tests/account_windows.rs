//! A process really started **as the session account**, and read back off its
//! standard handles.
//!
//! The first half of what stage 04 turns on: a Windows session runs as a local
//! account of Verkstead's own rather than as the human (ADR-0014, *Amended: the
//! Sandbox is an account*), and this is that account being somebody a process
//! can be. The console half is `crates/cli/tests/launcher_windows.rs`, which is
//! where the binary a launcher is a verb of gets built.
//!
//! **It needs the account, and says so rather than passing.** Creating a local
//! account is an administrator's call, so this suite cannot make one: what it
//! asks for is the account this machine's Data Directory already has, and a
//! machine where the elevated verb has never been run fails here with the line
//! that names the verb. The same rule the two build-cache tests in
//! `tests/sessions_windows.rs` follow about an sccache — a suite that quietly
//! passed for want of what it is testing has said nothing.
//!
//! **What the account can reach is not this file's question.** The entries a
//! Surface comes to are a later task's; everything a probe here touches is a
//! path any account on the machine can already read — `cmd.exe` in the system
//! directory, and the system directory itself to start in. So a refusal in this
//! file is a refusal about the *logon*, which is what is being asked about.
#![cfg(windows)]

use std::path::PathBuf;

use verkstead_server::platform;
use verkstead_server::sandbox::Rendering;
use verkstead_server::sandbox::account::Logon;
use verkstead_server::sandbox::account::machine::Account;
use verkstead_server::sandbox::off_a_console;
use verkstead_server::settings::Settings;

/// The shell every Windows machine has, in the directory every account can read
/// it out of.
///
/// Named with its path rather than found on one: what is being asked here is
/// whether a process runs as the account at all, and a `PATH` lookup that
/// failed would be an answer to a different question.
const CMD: &str = r"C:\Windows\System32\cmd.exe";

/// Where the probe starts, which has to be a directory the account can reach:
/// a process started with nowhere to be is one the machine refuses before it
/// has run.
const SOMEWHERE: &str = r"C:\Windows";

/// What a probe prints, so that what comes back is this test's rather than
/// anything the shell says for itself.
const MARKER: &str = "the-account-printed-this";

/// The account this machine's Verkstead runs its sessions as, or a failure
/// saying what to run.
fn the_account() -> Logon {
    let data_dir =
        platform::data_dir(None).expect("this machine has somewhere for a Data Directory");
    let settings = Settings::in_data_dir(&data_dir);

    match Account::on_this_machine(&data_dir, &settings.secrets()) {
        Ok(account) => Logon::of(account.name(), account.password()),
        Err(missing) => panic!(
            "this suite runs a probe as the session account and there is not one: {missing}\n\
             \n\
             The Data Directory it asked about is {}.",
            data_dir.display(),
        ),
    }
}

/// And the one that says who a process is, which it asks the token rather than
/// the environment.
const WHOAMI: &str = r"C:\Windows\System32\whoami.exe";

/// A rendering of `program`, with the little any process needs to start at all.
///
/// Said rather than inherited, because a rendering is the whole of what a
/// process is handed — see [`Rendering`] — and the server's own environment is
/// full of paths this account is refused.
fn running(program: &str) -> Rendering {
    let mut rendering = Rendering::running(program);

    rendering
        .set("SystemRoot", SOMEWHERE)
        .set("SystemDrive", "C:")
        .set("PATH", format!(r"{SOMEWHERE}\System32;{SOMEWHERE}"))
        .set("PATHEXT", ".COM;.EXE;.BAT;.CMD")
        .starting_in(PathBuf::from(SOMEWHERE));

    rendering
}

/// And one of `cmd.exe` running `command`.
fn probe(command: &str) -> Rendering {
    let mut rendering = running(CMD);

    rendering.arg("/d").arg("/c").arg(command);

    rendering
}

/// The whole of what this task's first criterion asks: a process started as the
/// account runs, and what it printed is read back off its standard handles.
#[test]
fn a_process_started_as_the_account_runs_and_what_it_printed_comes_back() {
    let mut rendering = probe(&format!("echo {MARKER}"));
    rendering.as_account(the_account());

    let said = off_a_console(&rendering, b"").expect("a probe started as the session account");

    assert!(
        said.status.success(),
        "the probe should have exited well and it exited {:?}, saying {:?} and complaining {:?}",
        said.status.code(),
        String::from_utf8_lossy(&said.stdout),
        String::from_utf8_lossy(&said.stderr),
    );
    assert!(
        String::from_utf8_lossy(&said.stdout).contains(MARKER),
        "what the account printed should have come back and it said {:?}",
        String::from_utf8_lossy(&said.stdout),
    );
}

/// And it is really the account rather than the human: `whoami` inside says a
/// name, and the name is not this test's own.
///
/// Which is the claim the whole boundary rests on — every entry a later task
/// writes is written for that account's SID, and a session that turned out to
/// be the human would be granted the human's reach whatever the entries said.
#[test]
fn the_process_started_as_the_account_is_the_account() {
    let account = the_account();

    // `whoami` rather than a variable, because it asks the *token* rather than
    // the environment: a rendering says what a process is handed, and what this
    // one is handed says nothing about who it is.
    let mut rendering = running(WHOAMI);

    rendering.as_account(account.clone());

    let said = off_a_console(&rendering, b"").expect("a probe started as the session account");
    let printed = String::from_utf8_lossy(&said.stdout).to_lowercase();

    assert!(
        printed.contains(&account.name().to_lowercase()),
        "the probe should have said it was {} and it said {printed:?}",
        account.name(),
    );
    assert!(
        !printed.contains(&whoami().to_lowercase()),
        "the probe should not have been this test's own account, {}",
        whoami(),
    );
}

/// A password that is wrong is the *account* end of a logon, and the refusal
/// says so rather than handing back a number.
///
/// Asked of an account that is not on this machine, because an account that is
/// answers the same way — `CreateProcessWithLogonW` will not say which of the
/// name and the password it did not like, and neither will this. What is being
/// asserted is that the refusal names the account and the verb rather than the
/// program.
#[test]
fn a_password_that_is_wrong_is_refused_at_the_account_end() {
    let mut rendering = probe(&format!("echo {MARKER}"));
    rendering.as_account(Logon::of("vk-nobody-at-all", "Vk1-not-the-password"));

    let refused = off_a_console(&rendering, b"")
        .expect_err("a logon with a password that is wrong should not start anything");

    assert!(
        refused.to_string().contains("vk-nobody-at-all"),
        "the refusal should name the account, and it said: {refused}"
    );
    assert!(
        refused
            .to_string()
            .contains("verkstead session-account create"),
        "the refusal should say what to run about it, and it said: {refused}"
    );
    assert!(
        !refused.to_string().contains("not-the-password"),
        "and it should not say the password, and it said: {refused}"
    );
}

/// And an account nothing holds a password for is refused before anything is
/// started at all.
///
/// Refused rather than attempted, because an empty password is either a logon
/// that fails with a number or — on a machine whose policy allows one — a
/// boundary made with no secret. Neither is a session.
#[test]
fn an_account_with_no_password_is_refused_before_anything_starts() {
    let mut rendering = probe(&format!("echo {MARKER}"));
    rendering.as_account(Logon::of("vk-nobody-at-all", ""));

    let refused = off_a_console(&rendering, b"")
        .expect_err("a logon with no password should not start anything");

    assert!(
        refused.to_string().contains("vk-nobody-at-all"),
        "the refusal should name the account, and it said: {refused}"
    );
    assert!(
        refused.to_string().contains("the account end"),
        "the refusal should say which end of it this was, and it said: {refused}"
    );
}

/// What this test's own process is running as, for the assertion that a session
/// is not it.
fn whoami() -> String {
    std::env::var("USERNAME").unwrap_or_else(|_| String::from("(nobody this test could name)"))
}
