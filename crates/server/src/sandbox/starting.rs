//! What a [`Rendering`] comes to on Windows: the Win32 half of starting one,
//! shared by the two places that start anything.
//!
//! **Because the standard library cannot start either of them.** A session runs
//! on a pseudoconsole, which is an attribute on a `CreateProcessW` that
//! `std::process::Command` has no way to carry — see [`crate::terminal`] — and a
//! Windows session also runs **as** a local account of Verkstead's own
//! (ADR-0014, *Amended: the Sandbox is an account*), which is
//! `CreateProcessWithLogonW` — a call it does not make at all. So the words a
//! rendering becomes — the command line, the environment block — and the
//! attribute list one of the two carries them on are here, where both callers
//! can reach them, rather than copied into each.
//!
//! **The two do not compose, which is why there is a launcher.**
//! `CreateProcessWithLogonW` refuses an extended startup info outright, so a
//! process started as somebody else can be given no console: what it is given is
//! the same four things a rendering comes to, and the console it is to run on is
//! made on the far side of the boundary by a launcher of Verkstead's own — see
//! [`crate::terminal::launcher`], which is what `as_the_account` starts.
//!
//! And beside them, the other way of starting a rendering: [`off_a_console`],
//! for everything that reads what a process printed rather than watching it —
//! the boundary suite, the ask test, the Compile Server. A rendering that names
//! an account cannot go through `Command` at all, so it goes through here.

use std::collections::BTreeMap;
use std::ffi::{OsStr, c_void};
use std::fs::File;
use std::io::{self, Read, Write};
use std::os::windows::ffi::OsStrExt;
use std::os::windows::io::{FromRawHandle, RawHandle};
use std::os::windows::process::ExitStatusExt;
use std::process::{Command, ExitStatus, Output, Stdio};
use std::ptr;
use std::sync::{Mutex, PoisonError};
use std::time::Duration;

use windows_sys::Win32::Foundation::{
    CloseHandle, HANDLE, HANDLE_FLAG_INHERIT, INVALID_HANDLE_VALUE, SetHandleInformation,
    WAIT_FAILED,
};
use windows_sys::Win32::Security::SECURITY_ATTRIBUTES;
use windows_sys::Win32::System::Pipes::CreatePipe;
use windows_sys::Win32::System::Threading::{
    CREATE_UNICODE_ENVIRONMENT, CreateProcessWithLogonW, DeleteProcThreadAttributeList,
    GetExitCodeProcess, INFINITE, InitializeProcThreadAttributeList, LOGON_WITH_PROFILE,
    LPPROC_THREAD_ATTRIBUTE_LIST, PROCESS_INFORMATION, STARTF_USESTDHANDLES, STARTUPINFOW,
    UpdateProcThreadAttribute, WaitForSingleObject,
};

use super::account::{Logon, MAKE_IT};
use super::rendering::Rendering;

/// `rendering` run to its end with nothing watching it, and everything it
/// printed read back.
///
/// The other half of the seam from [`crate::terminal::Terminal::spawn`]: a
/// session is started on a console and read as it draws, and this is for
/// everything that wants what a process *said* — a probe inside the boundary
/// suite, the `verkstead ask` a session makes, the Compile Server. What comes
/// back is what `Command::output` hands back, because that is what a caller
/// here already knows how to read.
///
/// `typed` is put in at its standard input and the input is then closed, which
/// is what says *that is the whole of it* to a program reading one. Nothing at
/// all is the ordinary case; a Set on the way to `verkstead ask` is the case
/// that is not.
///
/// **A rendering that names no account is an ordinary `Command`**, which is
/// what the two Unix platforms and the Compile Server are. One that names an
/// account is the whole reason this exists: starting a process as somebody else
/// is `CreateProcessWithLogonW`, which is a call the standard library does not
/// make.
///
/// It waits, so a caller that has to answer the process it started — the ask
/// test, whose Set has to be answered before the process will exit — runs this
/// on a thread of its own.
pub fn off_a_console(rendering: &Rendering, typed: &[u8]) -> io::Result<Output> {
    match rendering.account() {
        Some(logon) => over_pipes(rendering, logon, typed),
        None => ordinarily(rendering, typed),
    }
}

/// A rendering that names no account, started as anything else is started.
fn ordinarily(rendering: &Rendering, typed: &[u8]) -> io::Result<Output> {
    let mut running = Command::try_from(rendering)?
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    if let Some(mut typing) = running.stdin.take() {
        typing.write_all(typed)?;
    }

    running.wait_with_output()
}

/// The three pipes a process started as the account runs over, and everything
/// it printed read back off two of them.
///
/// The same four things a rendering comes to, said to Win32 by hand: the
/// command line, the environment block, where it starts, and the logon that is
/// the boundary.
///
/// Each of the three standard handles is a pipe of its own, with the child's
/// end inheritable and this process's end not — see [`piped`]. Both of the ends
/// this holds are read on threads of their own and the input is written on a
/// third, so that a process printing more than a pipe holds is not one waiting
/// on a reader that is waiting on it.
fn over_pipes(rendering: &Rendering, logon: &Logon, typed: &[u8]) -> io::Result<Output> {
    let (given, mut typing) = piped(Reads::TheChild)?;
    let (printing, printed) = piped(Reads::ThisProcess)?;
    let (complaining, complained) = piped(Reads::ThisProcess)?;

    let mut line = command_line(rendering);
    let environment = environment(rendering);
    let chdir = rendering.chdir().map(|chdir| wide(chdir.as_os_str()));
    let standard = [given.0, printing.0, complaining.0];

    let information = as_the_account(
        logon,
        &mut line,
        &environment,
        chdir.as_deref(),
        standard,
        0,
        &rendering.program().to_string_lossy(),
    )?;

    let process = Handle(information.hProcess);
    drop(Handle(information.hThread));

    // The child's ends of all three, let go of here: a pipe whose write end
    // this process is still holding is one the read below would wait on for
    // ever.
    drop(given);
    drop(printing);
    drop(complaining);

    let typed = typed.to_vec();

    // Whatever it would not take is nothing to report: a program that read none
    // of its input, or exited before reading the rest of it, has answered the
    // caller in what it printed rather than in a broken pipe here.
    let putting = std::thread::spawn(move || {
        let _ = typing.write_all(&typed);
    });

    let saying = std::thread::spawn(move || drained(printed));
    let complaining = std::thread::spawn(move || drained(complained));

    let said = saying
        .join()
        .map_err(|_| lost("reading what it printed"))??;
    let complained = complaining
        .join()
        .map_err(|_| lost("reading what it complained about"))??;
    putting
        .join()
        .map_err(|_| lost("writing what it was given"))?;

    if unsafe { WaitForSingleObject(process.0, INFINITE) } == WAIT_FAILED {
        return Err(io::Error::last_os_error());
    }

    let mut code = 0u32;

    if unsafe { GetExitCodeProcess(process.0, &mut code) } == 0 {
        return Err(io::Error::last_os_error());
    }

    Ok(Output {
        status: ExitStatus::from_raw(code),
        stdout: said,
        stderr: complained,
    })
}

/// And one **as** `logon`: `CreateProcessWithLogonW`, which is the one call
/// that starts a process as somebody else without a privilege a per-user
/// install has not got.
///
/// **It will not carry an attribute list.** Handed an extended startup info it
/// refuses outright with *The parameter is incorrect*, which is why a
/// pseudoconsole cannot be given to a process started as somebody else, and why
/// the console a session runs on is made on the far side of the boundary
/// instead — see [`crate::terminal::launcher`]. What goes to it here is a plain
/// `STARTUPINFOW` and the same four things a rendering already comes to.
///
/// **Its standard handles are inherited.** There is no `bInheritHandles` to
/// pass — the secondary logon service duplicates the three named in `standard`
/// into the process it starts — and they are marked inheritable here so that
/// every caller's are, whether they came from [`piped`] or from a terminal's
/// own pipes.
///
/// `also` is whatever creation flags the caller wants beside the environment's:
/// `CREATE_SUSPENDED` for a process that is to be in a Job before it has run an
/// instruction, and nothing for one that is not. `what` is what is being
/// started, said in the refusal so that a person reading a log is told which of
/// the two ends of this failed.
pub(crate) fn as_the_account(
    logon: &Logon,
    line: &mut [u16],
    environment: &[u16],
    chdir: Option<&[u16]>,
    standard: [HANDLE; 3],
    also: u32,
    what: &str,
) -> io::Result<PROCESS_INFORMATION> {
    // Refused here rather than by the call, which would take it: an account
    // with an empty password is either a logon that fails with a number, or —
    // on a machine whose policy allows one — a boundary made with no secret at
    // all. Neither is something to start a session on.
    if logon.password().is_empty() {
        return Err(io::Error::other(format!(
            "nothing holds a password for the local account {}, so {what} cannot be started as \
             it — the account end of this, rather than the program",
            logon.name(),
        )));
    }

    // Inheritable, which is what the far side reads as its standard handles.
    // Left inheritable rather than put back: every one of these is let go of by
    // its caller as soon as the process is started, and a handle that is closed
    // is a handle nothing else can inherit.
    for handle in standard {
        if unsafe { SetHandleInformation(handle, HANDLE_FLAG_INHERIT, HANDLE_FLAG_INHERIT) } == 0 {
            return Err(io::Error::last_os_error());
        }
    }

    let mut startup: STARTUPINFOW = unsafe { std::mem::zeroed() };
    startup.cb = u32::try_from(size_of::<STARTUPINFOW>()).unwrap_or(u32::MAX);
    startup.dwFlags = STARTF_USESTDHANDLES;
    startup.hStdInput = standard[0];
    startup.hStdOutput = standard[1];
    startup.hStdError = standard[2];

    let name = wide(OsStr::new(logon.name()));
    let password = wide(OsStr::new(logon.password()));

    let mut information: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };

    // One of Verkstead's at a time — see [`ONE_AT_A_TIME`]. Held across the
    // retries as well as across the call, so that two sessions starting
    // together take their turns rather than both backing off into each other.
    let _turn = ONE_AT_A_TIME.lock().unwrap_or_else(PoisonError::into_inner);

    for left in (0..AGAIN).rev() {
        // No domain, which is what says a local account of this machine's; and
        // LOGON_WITH_PROFILE, which loads the account's own profile and is what
        // makes the directory under C:\Users at the first session — see
        // `sandbox::account::machine::remove`, which is what takes it away
        // again.
        let started = unsafe {
            CreateProcessWithLogonW(
                name.as_ptr(),
                ptr::null(),
                password.as_ptr(),
                LOGON_WITH_PROFILE,
                ptr::null(),
                line.as_mut_ptr(),
                CREATE_UNICODE_ENVIRONMENT | also,
                environment.as_ptr().cast::<c_void>(),
                chdir.map_or(ptr::null(), |chdir| chdir.as_ptr()),
                &raw const startup,
                &mut information,
            )
        };

        if started != 0 {
            return Ok(information);
        }

        let said = io::Error::last_os_error();

        // Somebody else's logon, still going. A blocking sleep in what may be a
        // runtime's thread, and deliberately: this is the retry path of a call
        // that is a blocking Win32 call the whole way down, it is two seconds at
        // the very worst, and the alternative is a session refused for a reason
        // that has nothing to do with it.
        if said.raw_os_error() == Some(ALREADY_BUSY) && left > 0 {
            std::thread::sleep(AFTER);

            continue;
        }

        return Err(which_end(logon, what, said));
    }

    unreachable!("the loop above returns on its last turn")
}

/// What the secondary logon service says when it is already busy with one of
/// these: `ERROR_SERVICE_ALREADY_RUNNING`.
///
/// **It does one logon at a time**, machine-wide, which is not documented
/// anywhere and is what two sessions starting together find out. It is neither
/// end of a logon — the account is fine and the program is fine — so it is
/// waited out rather than reported: see [`ONE_AT_A_TIME`] and [`AGAIN`].
const ALREADY_BUSY: i32 = 1056;

/// How many times a logon that collided with another is tried again, and how
/// long is left between two tries.
///
/// Two seconds all told, which is far longer than a logon takes and far shorter
/// than a session is worth. A machine where something else is holding the
/// service for longer than that is one where the refusal says what happened,
/// which is better than a wait nobody can see the end of.
const AGAIN: usize = 20;

/// How long between two tries — see [`AGAIN`].
const AFTER: Duration = Duration::from_millis(100);

/// Verkstead's own logons, one at a time.
///
/// The service is machine-wide and so is the collision, so this does not remove
/// the retry above it — what it removes is Verkstead colliding with *itself*,
/// which is two sessions starting together and is the common case by a long
/// way.
static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

/// Every way this machine says *the account or its password*, as against every
/// other way a `CreateProcessWithLogonW` can fail.
///
/// Win32's own numbers, said here as numbers: what a refusal has to tell apart
/// is a list, and a list of ten `use` lines for constants nothing else in this
/// crate reads is a worse way to keep one. The spellings are
/// `ERROR_NO_SUCH_USER`, `ERROR_LOGON_TYPE_NOT_GRANTED`,
/// `ERROR_PASSWORD_MUST_CHANGE`, `ERROR_ACCOUNT_LOCKED_OUT` and the run from
/// `ERROR_LOGON_FAILURE` to `ERROR_ACCOUNT_DISABLED`.
const THE_ACCOUNT_END: &[i32] = &[
    1317, // there is no such user
    1326, // the user name or the password is wrong
    1327, // the account is restricted from logging on
    1328, // not at this hour
    1329, // not from this workstation
    1330, // the password has expired
    1331, // the account is disabled
    1385, // this logon type is not granted to it
    1907, // the password must be changed before it may be used
    1909, // the account is locked out
];

/// A logon that would not happen, said as which end of it was refused.
///
/// **Which is the whole of what a person can act on.** A session refused with
/// error 1326 is a session nobody can do anything about; one refused with *the
/// local account vk-… or its password was refused* is one somebody runs the
/// elevated verb again for. So the account end and the program end are told
/// apart by what the machine said, and the answer says which it was either way.
fn which_end(logon: &Logon, what: &str, said: io::Error) -> io::Error {
    let account = said
        .raw_os_error()
        .is_some_and(|code| THE_ACCOUNT_END.contains(&code));

    if account {
        return io::Error::other(format!(
            "this machine refused the local account {} or the password Verkstead holds for it, \
             so {what} was never started: {said} — run `{MAKE_IT}` from an elevated terminal",
            logon.name(),
        ));
    }

    io::Error::other(format!(
        "the local account {} was accepted and {what} would not start as it: {said}",
        logon.name(),
    ))
}

/// Everything there is to read at one end of a pipe, read until there is no
/// more of it.
fn drained(mut end: File) -> io::Result<Vec<u8>> {
    let mut read = Vec::new();

    end.read_to_end(&mut read)?;

    Ok(read)
}

/// What a thread that went away without answering is reported as.
fn lost(what: &str) -> io::Error {
    io::Error::other(format!(
        "the thread {what} of a process started off a console went away"
    ))
}

/// Which end of a pipe the child gets.
enum Reads {
    /// Its standard input: the child reads, and this process writes.
    TheChild,

    /// Its output or its complaints: the child writes, and this process reads.
    ThisProcess,
}

/// One pipe, as the child's end and this process's own.
///
/// The child's end is inheritable and this one's is not, which is the whole of
/// what makes a read here end: a copy of the write end left inheritable would
/// go to the next process this server starts, and a pipe with a writer is a
/// pipe with more to come.
fn piped(reads: Reads) -> io::Result<(Handle, File)> {
    let mut reading: HANDLE = ptr::null_mut();
    let mut writing: HANDLE = ptr::null_mut();

    let attributes = SECURITY_ATTRIBUTES {
        nLength: u32::try_from(size_of::<SECURITY_ATTRIBUTES>()).unwrap_or(u32::MAX),
        lpSecurityDescriptor: ptr::null_mut(),
        bInheritHandle: 1,
    };

    if unsafe { CreatePipe(&mut reading, &mut writing, &attributes, 0) } == 0 {
        return Err(io::Error::last_os_error());
    }

    let (theirs, ours) = match reads {
        Reads::TheChild => (reading, writing),
        Reads::ThisProcess => (writing, reading),
    };

    if unsafe { SetHandleInformation(ours, HANDLE_FLAG_INHERIT, 0) } == 0 {
        let failed = io::Error::last_os_error();

        drop(Handle(theirs));
        drop(Handle(ours));

        return Err(failed);
    }

    // Safety: the handle is this process's own, made a moment ago and held by
    // nothing else, so the file is the one owner of it from here.
    Ok((Handle(theirs), unsafe {
        File::from_raw_handle(ours as RawHandle)
    }))
}

/// The attribute list a process is started with, sized for as many attributes
/// as it is going to be given.
///
/// A list is a block of memory Windows lays out itself, so this is a buffer of
/// pointer-sized words — the alignment a list wants — with the list written
/// into it, and it is deleted when it is dropped.
pub(crate) struct Attributes(Vec<usize>);

impl Attributes {
    /// A list with room for `count` attributes and nothing on it yet.
    pub(crate) fn of(count: usize) -> io::Result<Attributes> {
        let count = u32::try_from(count).unwrap_or(u32::MAX);
        let mut wanted = 0usize;

        // The first call always fails: what it is for is the size, which is
        // what it writes on its way out.
        unsafe { InitializeProcThreadAttributeList(ptr::null_mut(), count, 0, &mut wanted) };

        if wanted == 0 {
            return Err(io::Error::last_os_error());
        }

        // The buffer before the list rather than after it: an [`Attributes`]
        // deletes the list as it is dropped, and there is no list to delete
        // until the call below has written one.
        let mut buffer = vec![0usize; wanted.div_ceil(size_of::<usize>())];

        let made = unsafe {
            InitializeProcThreadAttributeList(
                buffer.as_mut_ptr().cast::<c_void>(),
                count,
                0,
                &mut wanted,
            )
        };

        if made == 0 {
            return Err(io::Error::last_os_error());
        }

        Ok(Attributes(buffer))
    }

    /// One more attribute on it: `value` is a pointer the list keeps rather
    /// than a value it copies, so whatever is at the other end of it has to
    /// outlive the `CreateProcessW` this list is passed to.
    pub(crate) fn carrying(
        &mut self,
        attribute: usize,
        value: *const c_void,
        size: usize,
    ) -> io::Result<()> {
        let list = self.list();

        let carried = unsafe {
            UpdateProcThreadAttribute(
                list,
                0,
                attribute,
                value,
                size,
                ptr::null_mut(),
                ptr::null(),
            )
        };

        if carried == 0 {
            return Err(io::Error::last_os_error());
        }

        Ok(())
    }

    /// The list itself, as everything that takes one wants it.
    pub(crate) fn list(&mut self) -> LPPROC_THREAD_ATTRIBUTE_LIST {
        self.0.as_mut_ptr().cast::<c_void>()
    }
}

impl Drop for Attributes {
    fn drop(&mut self) {
        unsafe { DeleteProcThreadAttributeList(self.list()) };
    }
}

/// One handle of the process's own, closed when it is let go of.
///
/// A handle is a pointer as far as the bindings are concerned and therefore
/// neither `Send` nor `Sync` by itself, and it is both in fact: it is a number
/// the kernel looks up in a table this whole process shares, and nothing about
/// which thread holds it means anything.
pub(crate) struct Handle(pub(crate) HANDLE);

unsafe impl Send for Handle {}
unsafe impl Sync for Handle {}

impl Drop for Handle {
    fn drop(&mut self) {
        if !self.0.is_null() && self.0 != INVALID_HANDLE_VALUE {
            unsafe { CloseHandle(self.0) };
        }
    }
}

/// `rendering` as the command line `CreateProcessW` takes, program first.
///
/// Windows has no argument vector to hand over: a process is given one string
/// and takes it apart again, so this is the taking-apart run backwards — see
/// [`quoted`].
pub(crate) fn command_line(rendering: &Rendering) -> Vec<u16> {
    let mut line = Vec::new();

    quoted(rendering.program(), &mut line);

    for argument in rendering.argv() {
        line.push(u16::from(b' '));
        quoted(argument, &mut line);
    }

    line.push(0);

    line
}

/// One word of a command line, written so that `CommandLineToArgvW` reads back
/// the word that went in.
///
/// Which is the rule everything on Windows that takes a command line apart
/// follows: a run of backslashes means itself unless a quote comes next, and
/// then it means half of itself and the quote is the word's rather than the
/// quoting's. So a run before a quote is doubled and the quote escaped, and a
/// run at the end of a quoted word is doubled because the closing quote comes
/// next.
fn quoted(word: &OsStr, line: &mut Vec<u16>) {
    const SPACE: u16 = b' ' as u16;
    const TAB: u16 = b'\t' as u16;
    const QUOTE: u16 = b'"' as u16;
    const BACKSLASH: u16 = b'\\' as u16;

    let word: Vec<u16> = word.encode_wide().collect();

    // A word with nothing in it to misread is written as it is — which is most
    // of them, and is what makes a command line readable in a log.
    if !word.is_empty() && !word.iter().any(|unit| matches!(*unit, SPACE | TAB | QUOTE)) {
        line.extend_from_slice(&word);

        return;
    }

    line.push(QUOTE);

    let mut backslashes = 0usize;

    for unit in word {
        match unit {
            BACKSLASH => backslashes += 1,
            QUOTE => {
                line.extend(std::iter::repeat_n(BACKSLASH, backslashes + 1));
                backslashes = 0;
            }
            _ => backslashes = 0,
        }

        line.push(unit);
    }

    line.extend(std::iter::repeat_n(BACKSLASH, backslashes));
    line.push(QUOTE);
}

/// And `rendering`'s environment as the block `CreateProcessW` takes: every
/// name and value in one run of text, sorted, and the whole ended by a second
/// nothing.
///
/// Sorted and case-folded because that is what Windows asks of a block, and
/// because an environment where `Path` and `PATH` are two variables is one no
/// program on this platform expects: the last of a name is the one that stands,
/// which is what setting a variable twice means everywhere else in this
/// codebase.
pub(crate) fn environment(rendering: &Rendering) -> Vec<u16> {
    let mut named: BTreeMap<Vec<u16>, (Vec<u16>, Vec<u16>)> = BTreeMap::new();

    for (key, value) in rendering.env() {
        let name: Vec<u16> = key.encode_wide().collect();
        let folded = name
            .iter()
            .map(|unit| match u8::try_from(*unit) {
                Ok(byte) => u16::from(byte.to_ascii_uppercase()),
                Err(_) => *unit,
            })
            .collect();

        named.insert(folded, (name, value.encode_wide().collect()));
    }

    let mut block = Vec::new();

    for (name, value) in named.into_values() {
        block.extend_from_slice(&name);
        block.push(u16::from(b'='));
        block.extend_from_slice(&value);
        block.push(0);
    }

    // An environment with nothing in it is still a block, and a block is a run
    // of strings ended by an empty one.
    if block.is_empty() {
        block.push(0);
    }

    block.push(0);

    block
}

/// A string as every one of these calls wants one: what it says, and then
/// nothing.
pub(crate) fn wide(text: &OsStr) -> Vec<u16> {
    text.encode_wide().chain(std::iter::once(0)).collect()
}
