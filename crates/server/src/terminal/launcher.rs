//! The launcher: the verb of Verkstead's own binary that makes a session's
//! console on the far side of the account boundary, and the channel Verkstead
//! goes on speaking to it over.
//!
//! **Why anything is on the far side at all.** A pseudoconsole is an attribute
//! on an extended startup info, and `CreateProcessWithLogonW` — the one call
//! that starts a process as somebody else without a privilege a per-user
//! install has not got — refuses one outright with *The parameter is
//! incorrect*. So a console cannot be handed to a process started as somebody
//! else, and it is made by somebody who is already the account instead: a
//! launcher, started as it with the console's two pipes as its plain standard
//! handles, which calls `CreatePseudoConsole` over those and starts the session
//! on it with an ordinary `CreateProcessW` (ADR-0014, *Amended: the Sandbox is
//! an account*).
//!
//! **Nothing is duplicated across the boundary.** The handles the console needs
//! are the ones the launcher was given as its own standard handles, which is
//! why the launcher takes no handle on its command line and Verkstead has no
//! process to duplicate one into before there is one.
//!
//! **Verkstead keeps the launcher's process handle**, so the Job Object that
//! already kills a session's whole tree holds the launcher and everything under
//! it. An ended session leaves no launcher for the same reason it leaves no
//! agent.
//!
//! **And two things still have to reach it after it has started**, so it has a
//! channel of its own: a resize goes down, and the exit code of what it
//! launched comes back up. The second is reported rather than inferred, and
//! that is the whole reason there is a channel — a launcher that exits does not
//! mean the program it launched exited well, and the two are different numbers.
//!
//! **The channel is a named pipe of Verkstead's**, created granting the session
//! account and nothing wider, and dialled by the launcher as the one word it is
//! told. Overlapped on the far side, because a synchronous handle serialises
//! its I/O: the thread parked on a read would be the thread a write waited
//! behind, and the read there is parked for as long as the session runs.
//!
//! **It is a verb nobody types.** It is hidden from the CLI's help the way the
//! rest of Verkstead's internal surface is, and it refuses plainly when its
//! standard handles are not the pipes it expects — which is what running it by
//! hand looks like.

use std::ffi::{OsStr, OsString, c_void};
use std::io;
use std::path::Path;
use std::ptr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};
use windows_sys::Win32::Foundation::{
    ERROR_IO_PENDING, GENERIC_READ, GENERIC_WRITE, HANDLE, INVALID_HANDLE_VALUE, WAIT_FAILED,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_FLAG_OVERLAPPED, FILE_TYPE_PIPE, GetFileType, OPEN_EXISTING, ReadFile,
    WriteFile,
};
use windows_sys::Win32::System::Console::{
    COORD, ClosePseudoConsole, CreatePseudoConsole, GetStdHandle, HPCON, ResizePseudoConsole,
    STD_INPUT_HANDLE, STD_OUTPUT_HANDLE,
};
use windows_sys::Win32::System::IO::{GetOverlappedResult, OVERLAPPED};
use windows_sys::Win32::System::Threading::{
    CreateEventW, CreateProcessW, EXTENDED_STARTUPINFO_PRESENT, GetExitCodeProcess, INFINITE,
    PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE, PROCESS_INFORMATION, STARTUPINFOEXW, WaitForSingleObject,
};

use crate::pipe::Descriptor;
use crate::sandbox::Rendering;
use crate::sandbox::account::machine::sid_of;
use crate::sandbox::starting::{Attributes, Handle, command_line, wide};

/// The verb the launcher is, as it is spelled on the command line nobody types.
///
/// Read from both ends: the CLI names its hidden subcommand this, and the words
/// `asking` builds put it where clap will read it back.
pub const VERB: &str = "session-launcher";

/// How long the console is left open after the program on it has exited.
///
/// The console host writes what a session printed on its own schedule, and the
/// last of it can still be on the way when the process it came from is already
/// gone — see `super::conpty`, whose own `FLUSHING` is this said on the other
/// side of the boundary about the console it closes there.
const FLUSHING: Duration = Duration::from_millis(250);

/// How much of the channel is read at once. What is on it is a line at a time,
/// and a line is a word and two numbers.
const MOUTHFUL: usize = 256;

/// What a resize says on the channel, on its way down.
const RESIZE: &str = "resize";

/// And what the exit of the program the launcher started says, on its way up.
const EXIT: &str = "exit";

/// The words that start a launcher: Verkstead's own image, the verb, the
/// channel to dial, the size to open the console at, and then — after the `--`
/// that stops clap reading any further — the program it is to start and
/// everything that program is given.
///
/// The environment and the directory are *not* among them: they are the
/// session's own and are given to the launcher itself, which passes both on by
/// starting the program with neither of its own. So a session runs in the
/// environment its rendering described, one process further down than it used
/// to.
pub(crate) fn asking(
    image: &Path,
    channel: &str,
    size: (u16, u16),
    rendering: &Rendering,
) -> Vec<u16> {
    let mut launcher = Rendering::running(image);

    launcher
        .arg(VERB)
        .arg("--channel")
        .arg(channel)
        .arg("--columns")
        .arg(size.0.to_string())
        .arg("--rows")
        .arg(size.1.to_string())
        .arg("--")
        .arg(rendering.program())
        .args(rendering.argv());

    command_line(&launcher)
}

/// Verkstead's end of a launcher's channel: the named pipe it will dial,
/// waiting for it.
///
/// Created before the launcher is started, because a launcher dialling a name
/// nothing was listening at would be refused — and named after this process and
/// a number that is never given out twice, the way a terminal's own pipes are.
pub(crate) struct Channel {
    name: String,
    pipe: NamedPipeServer,
}

impl Channel {
    /// One channel, granting `account` and the account the server runs as, and
    /// nothing wider.
    ///
    /// The descriptor is the server's own pipe's — see [`Descriptor::granting`]
    /// — which grants this process everything and the identity beside it the
    /// read and the write a client needs. The session account is a *different*
    /// account, so a pipe created with the default descriptor would be one the
    /// launcher could not open at all.
    pub(crate) fn open(account: &str) -> io::Result<Channel> {
        /// What makes one channel's name different from the next one's.
        static NAMED: AtomicU64 = AtomicU64::new(0);

        let sid = sid_of(account).map_err(|why| {
            io::Error::other(format!(
                "the local account {account} would not resolve, so no channel could be opened \
                 for a launcher running as it: {why}"
            ))
        })?;

        let name = format!(
            r"\\.\pipe\verkstead-launcher-{}-{}",
            std::process::id(),
            NAMED.fetch_add(1, Ordering::Relaxed),
        );

        let granting = Descriptor::granting(&[sid.text().to_owned()])?;
        let mut attributes = granting.attributes();

        // SAFETY: the attributes are valid for the length of the call and point
        // at `granting`, which outlives it. One instance, because one launcher
        // dials this and a second client on the name would be somebody else.
        let pipe = unsafe {
            ServerOptions::new()
                .first_pipe_instance(true)
                .max_instances(1)
                .create_with_security_attributes_raw(&name, ptr::from_mut(&mut attributes).cast())
        }?;

        Ok(Channel { name, pipe })
    }

    /// The name the launcher is told to dial.
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    /// And the pipe itself, once there is nothing left to do out here but wait
    /// for the launcher on it.
    pub(crate) fn waiting(self) -> NamedPipeServer {
        self.pipe
    }
}

/// A resize, as it goes down the channel.
pub(crate) fn resized(columns: u16, rows: u16) -> String {
    format!("{RESIZE} {columns} {rows}\n")
}

/// And the exit code of the program a launcher started, read back off a line it
/// wrote — and nothing where the line says anything else.
///
/// Verkstead's end of the one thing this channel exists to carry: what comes
/// back is the *session's* exit rather than the launcher's, which is a
/// different number and is the whole reason it is said rather than inferred.
pub(crate) fn ended(line: &str) -> Option<u32> {
    line.trim()
        .strip_prefix(EXIT)?
        .trim_start()
        .parse::<u32>()
        .ok()
}

/// The launcher itself, running **as the session account**: the console it
/// makes, the program it starts on it, and the two things it goes on saying to
/// Verkstead over its channel.
///
/// Everything here is on the far side of the boundary — this function runs in a
/// process Verkstead started as somebody else — so nothing about it is a
/// server's: no runtime, no tracing, and the one channel is the whole of what it
/// can say. Its complaints go to the standard error it was given, which is the
/// console's own pipe, so a launcher that would not start says so in the Capture
/// of the session that failed.
pub fn launch(
    channel: &str,
    columns: u16,
    rows: u16,
    program: &OsStr,
    argv: &[OsString],
) -> io::Result<()> {
    // Its own standard handles, which are the two ends of the console it is
    // about to make. Nothing else crosses the boundary — see this module's own
    // documentation.
    let typed_at = unsafe { GetStdHandle(STD_INPUT_HANDLE) };
    let printing = unsafe { GetStdHandle(STD_OUTPUT_HANDLE) };

    for (which, handle) in [("input", typed_at), ("output", printing)] {
        if handle.is_null() || handle == INVALID_HANDLE_VALUE {
            return Err(io::Error::other(format!(
                "`verkstead {VERB}` is Verkstead's own and takes its console's {which} as its \
                 standard {which}, and it was given none"
            )));
        }

        if unsafe { GetFileType(handle) } != FILE_TYPE_PIPE {
            return Err(io::Error::other(format!(
                "`verkstead {VERB}` is Verkstead's own and takes a pipe as its standard {which}: \
                 it is what a session's console is made over, and is not something to run this \
                 by hand with"
            )));
        }
    }

    let channel = Arc::new(Dialled::to(channel)?);
    let console = Arc::new(Console::over(typed_at, printing, columns, rows)?);
    let started = on_the_console(&console, program, argv)?;

    let process = Handle(started.hProcess);
    drop(Handle(started.hThread));

    // The resizes, on a thread of its own for as long as this process lives:
    // what arrives is a window a watcher's browser is drawing, and it arrives
    // while the wait below is parked.
    std::thread::spawn({
        let channel = channel.clone();
        let console = console.clone();

        move || listening(&channel, &console)
    });

    let code = awaited(&process)?;

    // Said before the console is closed and before this process exits, which is
    // what makes it *reported*: everything after this line is the launcher's
    // own ending, and the launcher's own ending says nothing about the session.
    channel.say(&format!("{EXIT} {code}\n"))?;

    std::thread::sleep(FLUSHING);
    console.close();

    Ok(())
}

/// The program the launcher was told to start, on the console it just made.
///
/// An ordinary `CreateProcessW` with the console on its attribute list, which
/// carries one happily: it is starting a process as the account it already is.
/// Its environment and its directory are this process's own — which are the
/// session's, handed to the launcher by whoever started it — so neither is said
/// again here.
fn on_the_console(
    console: &Console,
    program: &OsStr,
    argv: &[OsString],
) -> io::Result<PROCESS_INFORMATION> {
    let Some(held) = console.held() else {
        return Err(io::Error::other("this launcher's console has been closed"));
    };

    let mut attributes = Attributes::of(1)?;

    attributes.carrying(
        PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE as usize,
        held as *const c_void,
        size_of::<HPCON>(),
    )?;

    let mut startup: STARTUPINFOEXW = unsafe { std::mem::zeroed() };
    startup.StartupInfo.cb = u32::try_from(size_of::<STARTUPINFOEXW>()).unwrap_or(u32::MAX);
    startup.lpAttributeList = attributes.list();

    let mut running = Rendering::running(program);
    running.args(argv);

    let mut line = command_line(&running);
    let mut information: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };

    let started = unsafe {
        CreateProcessW(
            ptr::null(),
            line.as_mut_ptr(),
            ptr::null(),
            ptr::null(),
            0,
            EXTENDED_STARTUPINFO_PRESENT,
            ptr::null(),
            ptr::null(),
            &raw const startup.StartupInfo,
            &mut information,
        )
    };

    if started == 0 {
        return Err(io::Error::last_os_error());
    }

    Ok(information)
}

/// Every resize that arrives on the channel, applied to the console, until the
/// channel says no more.
///
/// Which it will not until Verkstead lets go of its end, so this is a thread
/// that ends when the process does. Nothing it can do about a line it cannot
/// read is worth doing: a channel carrying a word this does not know is a
/// Verkstead newer than this launcher, and the launcher a session was started
/// with is the image that session was started from.
fn listening(channel: &Dialled, console: &Console) {
    let mut said = String::new();
    let mut buffer = [0u8; MOUTHFUL];

    loop {
        let read = match channel.heard(&mut buffer) {
            Ok(0) | Err(_) => return,
            Ok(read) => read,
        };

        said.push_str(&String::from_utf8_lossy(&buffer[..read]));

        while let Some(end) = said.find('\n') {
            let line = said[..end].trim().to_owned();
            said = said[end + 1..].to_owned();

            if let Some((columns, rows)) = a_resize(&line) {
                let _ = console.resize(columns, rows);
            }
        }
    }
}

/// A line off the channel read as a resize, and nothing where it is not one.
fn a_resize(line: &str) -> Option<(u16, u16)> {
    let mut words = line.strip_prefix(RESIZE)?.split_whitespace();
    let columns = words.next()?.parse().ok()?;
    let rows = words.next()?.parse().ok()?;

    Some((columns, rows))
}

/// Wait for the program the launcher started, and say how it ended.
fn awaited(process: &Handle) -> io::Result<u32> {
    if unsafe { WaitForSingleObject(process.0, INFINITE) } == WAIT_FAILED {
        return Err(io::Error::last_os_error());
    }

    let mut code = 0u32;

    if unsafe { GetExitCodeProcess(process.0, &mut code) } == 0 {
        return Err(io::Error::last_os_error());
    }

    Ok(code)
}

/// The console the launcher made, closed once and by whoever gets there first.
///
/// Shared between the wait and the thread applying resizes, so that a resize
/// arriving as the session ends is one that finds the console gone rather than
/// one that reaches a handle nothing owns any more.
struct Console(Mutex<Option<HPCON>>);

impl Console {
    /// One console `columns` by `rows`, over the two pipes the launcher was
    /// given as its standard handles.
    fn over(typed_at: HANDLE, printing: HANDLE, columns: u16, rows: u16) -> io::Result<Console> {
        let mut console: HPCON = 0;

        let opened = unsafe {
            CreatePseudoConsole(
                COORD {
                    X: i16::try_from(columns).unwrap_or(i16::MAX),
                    Y: i16::try_from(rows).unwrap_or(i16::MAX),
                },
                typed_at,
                printing,
                0,
                &mut console,
            )
        };

        if opened < 0 {
            return Err(io::Error::other(format!(
                "a pseudoconsole could not be opened as the session account: \
                 CreatePseudoConsole said {opened:#010x}"
            )));
        }

        Ok(Console(Mutex::new(Some(console))))
    }

    /// The handle while it is open, and nothing once it is not.
    fn held(&self) -> Option<HPCON> {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Make the window `columns` by `rows`, under the lock the close takes —
    /// see `super::conpty`, whose own console is held the same way and for the
    /// same reason.
    fn resize(&self, columns: u16, rows: u16) -> io::Result<()> {
        let held = self.0.lock().unwrap_or_else(PoisonError::into_inner);

        let Some(console) = *held else {
            return Ok(());
        };

        let resized = unsafe {
            ResizePseudoConsole(
                console,
                COORD {
                    X: i16::try_from(columns).unwrap_or(i16::MAX),
                    Y: i16::try_from(rows).unwrap_or(i16::MAX),
                },
            )
        };

        if resized < 0 {
            return Err(io::Error::other(format!(
                "this console could not be resized: ResizePseudoConsole said {resized:#010x}"
            )));
        }

        Ok(())
    }

    /// Close it, if it is not closed already.
    fn close(&self) {
        let console = self.0.lock().unwrap_or_else(PoisonError::into_inner).take();

        if let Some(console) = console {
            unsafe { ClosePseudoConsole(console) };
        }
    }
}

impl Drop for Console {
    fn drop(&mut self) {
        self.close();
    }
}

/// The launcher's end of the channel: the pipe Verkstead opened, dialled.
///
/// **Overlapped**, which is the one thing about it worth a paragraph. A
/// synchronous handle serialises every operation on itself, so the thread
/// parked on the read that waits for resizes would be the thread the exit line
/// waited behind — and the exit line is written at the one moment the read is
/// certainly still parked. Each call below therefore carries an event of its
/// own and waits on that.
struct Dialled(Handle);

/// A handle is a pointer as far as the bindings are concerned and therefore
/// neither `Send` nor `Sync` by itself, and this one is both in fact: it is a
/// number the kernel looks up in a table this whole process shares, and the two
/// threads that use it use it for opposite directions.
unsafe impl Send for Dialled {}
unsafe impl Sync for Dialled {}

impl Dialled {
    /// Dial the channel called `name`.
    fn to(name: &str) -> io::Result<Dialled> {
        let name = wide(OsStr::new(name));

        let held = unsafe {
            CreateFileW(
                name.as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                0,
                ptr::null(),
                OPEN_EXISTING,
                FILE_FLAG_OVERLAPPED,
                ptr::null_mut(),
            )
        };

        if held == INVALID_HANDLE_VALUE {
            return Err(io::Error::other(format!(
                "`verkstead {VERB}` could not reach the channel it was told to speak to \
                 Verkstead over: {}",
                io::Error::last_os_error(),
            )));
        }

        Ok(Dialled(Handle(held)))
    }

    /// Say `line` on it, and wait until it has been said.
    fn say(&self, line: &str) -> io::Result<()> {
        let said = line.as_bytes();
        let waiting = Awaited::new()?;
        let mut written = 0u32;

        let put = unsafe {
            WriteFile(
                (self.0).0,
                said.as_ptr(),
                u32::try_from(said.len()).unwrap_or(u32::MAX),
                ptr::null_mut(),
                waiting.overlapped(),
            )
        };

        if put == 0 {
            waiting.pending()?;
        }

        waiting.done(&self.0, &mut written)?;

        Ok(())
    }

    /// And take whatever is on it next, waiting until there is some.
    ///
    /// `Ok(0)` is Verkstead having let go of its end, which is the session being
    /// over by every route that matters here.
    fn heard(&self, buffer: &mut [u8]) -> io::Result<usize> {
        let waiting = Awaited::new()?;
        let mut read = 0u32;

        let taken = unsafe {
            ReadFile(
                (self.0).0,
                buffer.as_mut_ptr(),
                u32::try_from(buffer.len()).unwrap_or(u32::MAX),
                ptr::null_mut(),
                waiting.overlapped(),
            )
        };

        if taken == 0 {
            waiting.pending()?;
        }

        waiting.done(&self.0, &mut read)?;

        Ok(read as usize)
    }
}

/// One overlapped operation: the structure Win32 writes into and the event it
/// signals when it is finished with it.
struct Awaited {
    overlapped: Box<OVERLAPPED>,
    event: Handle,
}

impl Awaited {
    /// One waiting to be started.
    ///
    /// The structure is boxed so that its address is the address the call will
    /// still be writing to: what goes to Win32 is a pointer, and one held
    /// inline would sit at one address while it was written and another by the
    /// time the operation finished.
    fn new() -> io::Result<Awaited> {
        // A manual-reset event, unsignalled, with no name: the one thing an
        // overlapped operation needs beside the structure.
        let event = unsafe { CreateEventW(ptr::null(), 1, 0, ptr::null()) };

        if event.is_null() {
            return Err(io::Error::last_os_error());
        }

        let mut overlapped: Box<OVERLAPPED> = Box::new(unsafe { std::mem::zeroed() });
        overlapped.hEvent = event;

        Ok(Awaited {
            overlapped,
            event: Handle(event),
        })
    }

    /// What Win32 is given.
    fn overlapped(&self) -> *mut OVERLAPPED {
        ptr::from_ref(self.overlapped.as_ref()).cast_mut()
    }

    /// A call that said it had not finished: whether that is *not yet* or a
    /// failure to report.
    fn pending(&self) -> io::Result<()> {
        let said = io::Error::last_os_error();

        if said.raw_os_error() == Some(ERROR_IO_PENDING as i32) {
            return Ok(());
        }

        Err(said)
    }

    /// Wait for it to finish, and say how much it moved.
    fn done(&self, on: &Handle, moved: &mut u32) -> io::Result<()> {
        if unsafe { WaitForSingleObject(self.event.0, INFINITE) } == WAIT_FAILED {
            return Err(io::Error::last_os_error());
        }

        if unsafe { GetOverlappedResult(on.0, self.overlapped(), moved, 0) } == 0 {
            return Err(io::Error::last_os_error());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{a_resize, ended, resized};

    /// The channel has two ends and each writes what the other reads, so the
    /// one thing worth asserting about it is that they agree.
    #[test]
    fn a_resize_written_at_one_end_is_the_resize_read_at_the_other() {
        assert_eq!(a_resize(resized(132, 43).trim()), Some((132, 43)));
    }

    /// And the exit the launcher reports is the number Verkstead reads back.
    #[test]
    fn the_exit_a_launcher_says_is_the_code_verkstead_reads() {
        assert_eq!(ended("exit 7"), Some(7));
        assert_eq!(ended("exit 0\n"), Some(0));
    }

    /// Neither line is the other, which is what keeps one word from being read
    /// as the other's.
    #[test]
    fn neither_line_is_read_as_the_other() {
        assert_eq!(ended(resized(80, 25).trim()), None);
        assert_eq!(a_resize("exit 7"), None);
    }

    /// And a line off the channel that says nothing this knows is nothing to
    /// act on: a Verkstead newer than the launcher a session was started from
    /// is a Verkstead saying words that launcher has never heard.
    #[test]
    fn a_line_this_does_not_know_is_nothing() {
        assert_eq!(a_resize("resize wide tall"), None);
        assert_eq!(a_resize("resize 80"), None);
        assert_eq!(ended("exit soon"), None);
        assert_eq!(ended("the launcher said something else"), None);
    }
}
