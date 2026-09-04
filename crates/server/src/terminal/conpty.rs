//! The pseudoconsole itself, which is what Windows has where the other
//! platforms have a pseudo-terminal: opening one, starting a process on it,
//! sizing it, and the two directions of reading and writing it. See [`super`]
//! for what it is all for.
//!
//! **A console and two pipes.** `CreatePseudoConsole` takes a size and the two
//! ends a console is spoken to through, and hands back a handle that a process
//! can be started against. Verkstead holds the other end of each pipe: one is
//! what the relay reads, and one is what a keystroke is written into. The pair
//! the console got are let go of the moment it has them — the API duplicates
//! them into the console host — which is what makes reading end at all: a copy
//! left open here would be a writer nothing would ever close, and a session
//! long gone would read as one still running.
//!
//! **Named pipes rather than anonymous ones**, for the reason the Unix end is
//! non-blocking: an anonymous pipe on Windows cannot be read without a thread
//! parked on the read, and a named one opened for overlapped I/O is what the
//! runtime knows how to watch. They are the standard library's own trick for
//! the same problem, and the name is nobody's but this process's — see
//! [`pipe`].
//!
//! **Started by hand.** Rust's `Command` cannot attach a pseudoconsole: the
//! attribute list that would is behind an unstable extension, so the process is
//! `CreateProcessW` with the console in a `PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE`
//! and the command line quoted by the rules `CommandLineToArgvW` reads one back
//! with. What comes back is a [`Child`] of Verkstead's own rather than tokio's.
//!
//! **The Job is what `--die-with-parent` is on Linux.** Every child here is
//! created suspended, put in a Job Object that kills everything in it when the
//! last handle to it closes, and only then resumed — so a server that dies
//! takes its sessions with it, and a [`Child`] that is dropped takes the whole
//! tree the session started with it. Nothing is left to a keeper the way a Mac
//! leaves it: `outliving::keep` has nothing to add here, and says so.
//!
//! **The console is closed when what ran on it has gone**, which is the one
//! thing this arm does that the Unix one has no need to. A pseudo-terminal
//! reports end-of-file when the last process holding the far end exits; a
//! pseudoconsole does not — its host is alive until it is closed, and a relay
//! reading it would wait for output from a session that ended minutes ago. So
//! the far end here is the console, and a task started beside every child
//! closes it once that child has gone.

use std::collections::BTreeMap;
use std::ffi::{OsStr, c_void};
use std::io;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::io::RawHandle;
use std::os::windows::process::ExitStatusExt;
use std::process::ExitStatus;
use std::ptr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use tokio::net::windows::named_pipe::NamedPipeServer;
use tokio::sync::watch;
use windows_sys::Win32::Foundation::{
    CloseHandle, GENERIC_READ, GENERIC_WRITE, HANDLE, INVALID_HANDLE_VALUE, WAIT_FAILED,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_FLAG_FIRST_PIPE_INSTANCE, FILE_FLAG_OVERLAPPED, OPEN_EXISTING,
    PIPE_ACCESS_DUPLEX,
};
use windows_sys::Win32::System::Console::{
    COORD, ClosePseudoConsole, CreatePseudoConsole, HPCON, ResizePseudoConsole,
};
use windows_sys::Win32::System::Pipes::{
    CreateNamedPipeW, PIPE_READMODE_BYTE, PIPE_TYPE_BYTE, PIPE_WAIT,
};
use windows_sys::Win32::System::Threading::{
    CREATE_SUSPENDED, CREATE_UNICODE_ENVIRONMENT, CreateProcessW, DeleteProcThreadAttributeList,
    EXTENDED_STARTUPINFO_PRESENT, GetExitCodeProcess, INFINITE, InitializeProcThreadAttributeList,
    LPPROC_THREAD_ATTRIBUTE_LIST, PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE, PROCESS_INFORMATION,
    ResumeThread, STARTUPINFOEXW, TerminateProcess, UpdateProcThreadAttribute, WaitForSingleObject,
};

use super::{COLUMNS, ROWS};
use crate::sandbox::Rendering;
use crate::sandbox::outliving::job::Job;

/// How much of each direction the console host may get ahead by, in bytes.
///
/// The relay reads as fast as it is given, so this is only what covers a burst
/// between two reads — a screen's worth of a session redrawing itself many
/// times over.
const BUFFER: u32 = 64 * 1024;

/// How long the console is left open after the process on it has exited.
///
/// The console host writes what a session printed on its own schedule, and the
/// last of it can still be on the way when the process it came from is already
/// gone. Closing the console at that instant is what loses a session's final
/// line, so the close waits — long enough for a host that is behind, short
/// enough that nobody watching sees a terminal that will not end.
const FLUSHING: Duration = Duration::from_millis(250);

/// What a killed child is said to have exited with — see [`Child::start_kill`].
const KILLED: u32 = 1;

/// How a child ended, as the one thread waiting on it saw it: the code it
/// exited with, or what went wrong asking.
///
/// Said as a value rather than as an `io::Error` because more than one place
/// waits for it — whoever is driving the session, and the task that closes the
/// console behind it — and an error is not something two of them can share.
type Ended = Result<u32, String>;

/// One session's terminal: the console, and the end Verkstead holds of each
/// pipe.
///
/// Opened before the session is started and held for as long as it runs.
pub struct Terminal {
    /// The console itself, shared with the task that closes it when the process
    /// on it has gone — see this module's own documentation.
    console: Arc<Console>,

    /// What the session prints arrives here. Registered with the runtime, so
    /// reading it costs no thread.
    output: NamedPipeServer,

    /// And what is written here arrives at the session.
    input: NamedPipeServer,

    /// Whether something has been started on it already. A console can host
    /// more than one process; a terminal of Verkstead's is one session's, and
    /// the second start is a mistake rather than a second session.
    started: bool,
}

impl Terminal {
    /// Open a console, [`COLUMNS`] by [`ROWS`], for a session about to start.
    pub fn open() -> io::Result<Terminal> {
        let (output, printing) = pipe()?;
        let (input, typing) = pipe()?;

        let mut console: HPCON = 0;

        // The size is the console's own from the first byte it draws: there is
        // no resize to send afterwards, and a session that started on a window
        // of nothing would have drawn one frame for it.
        let opened = unsafe {
            CreatePseudoConsole(
                COORD {
                    X: i16::try_from(COLUMNS).unwrap_or(i16::MAX),
                    Y: i16::try_from(ROWS).unwrap_or(i16::MAX),
                },
                typing.0,
                printing.0,
                0,
                &mut console,
            )
        };

        if opened < 0 {
            return Err(io::Error::other(format!(
                "a pseudoconsole could not be opened: CreatePseudoConsole said {opened:#010x}"
            )));
        }

        // The console has its own copies now, and these are the copies that
        // would keep reading alive forever — see this module's own
        // documentation.
        drop(printing);
        drop(typing);

        Ok(Terminal {
            console: Arc::new(Console(Mutex::new(Some(console)))),
            output,
            input,
            started: false,
        })
    }

    /// Start `rendering` on this console, and watch what it started.
    ///
    /// The process is created suspended so that it is inside the Job before it
    /// has run an instruction: a child that started first could have started a
    /// child of its own outside the Job, and outside the Job is outside every
    /// promise about what an ended session leaves behind.
    ///
    /// Two things are set going beside it. One thread waits for the process,
    /// which is the whole of what waiting on a Windows process is and is shared
    /// by everything that asks how it ended; and one task closes the console
    /// once it has, which is what makes [`Terminal::read`] end.
    pub fn spawn(&mut self, rendering: &Rendering) -> io::Result<Child> {
        if self.started {
            return Err(io::Error::other(
                "this terminal has already had a session started on it",
            ));
        }

        let Some(console) = self.console.held() else {
            return Err(io::Error::other(
                "this terminal's console has already been closed",
            ));
        };

        let mut attributes = Attributes::carrying(console)?;

        let mut startup: STARTUPINFOEXW = unsafe { std::mem::zeroed() };
        startup.StartupInfo.cb = u32::try_from(size_of::<STARTUPINFOEXW>()).unwrap_or(u32::MAX);
        startup.lpAttributeList = attributes.list();

        let mut line = command_line(rendering);
        let environment = environment(rendering);
        let chdir = rendering.chdir().map(|chdir| wide(chdir.as_os_str()));

        let mut information: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };

        // Nothing of the server's is inherited: what the process has of this
        // one is the console it was given, and every handle here is this
        // process's own — see [`pipe`], where neither end is inheritable.
        let started = unsafe {
            CreateProcessW(
                ptr::null(),
                line.as_mut_ptr(),
                ptr::null(),
                ptr::null(),
                0,
                EXTENDED_STARTUPINFO_PRESENT | CREATE_UNICODE_ENVIRONMENT | CREATE_SUSPENDED,
                environment.as_ptr().cast::<c_void>(),
                chdir.as_ref().map_or(ptr::null(), |chdir| chdir.as_ptr()),
                &raw const startup.StartupInfo,
                &mut information,
            )
        };

        if started == 0 {
            return Err(io::Error::last_os_error());
        }

        self.started = true;

        let process = Arc::new(Handle(information.hProcess));
        let thread = Handle(information.hThread);

        // A failure from here on is a failure with a suspended child on the
        // other side of it, and a suspended child nobody resumed is one that
        // would sit there for as long as the machine is up — so what cannot be
        // held properly is ended instead.
        let job = match held_in_a_job(&process, &thread) {
            Ok(job) => job,
            Err(error) => {
                unsafe { TerminateProcess(process.0, KILLED) };

                return Err(error);
            }
        };

        // The one wait on this process, and what everything else that asks how
        // it ended is reading — see [`Ended`]. A thread rather than a
        // registration with the runtime, which has no way to watch a handle
        // that is not a pipe or a socket: it is one thread per running session,
        // parked on the one call that answers.
        let (over, exited) = watch::channel(None);

        tokio::task::spawn_blocking({
            let process = process.clone();

            move || {
                let _ = over.send(Some(awaited(&process)));
            }
        });

        tokio::spawn(closing(self.console.clone(), exited.clone()));

        Ok(Child {
            id: information.dwProcessId,
            job,
            exited,
        })
    }

    /// Make the window `columns` by `rows`, and tell whatever is running on it.
    ///
    /// The console host's own notification rather than anything of Verkstead's:
    /// a program on a console asks the console how big it is, and this is what
    /// changes the answer.
    ///
    /// A console that has already been closed takes it and says nothing: the
    /// session it belonged to has ended, and a window nobody is drawing in is
    /// not a failure to report to whoever resized it.
    pub fn resize(&self, columns: u16, rows: u16) -> io::Result<()> {
        self.console.resize(columns, rows)
    }

    /// Put `keys` in at this end, where the session reads them as typing.
    ///
    /// The other direction of the same console, and the whole of what a Hold
    /// does to one: a keystroke from a watcher is written here, and the session
    /// cannot tell it from a human at a keyboard of its own.
    ///
    /// Written to the end rather than once, because a pipe takes what fits in
    /// its buffer and says how much that was. Nothing is echoed back from here:
    /// what the session makes of a keystroke comes round the ordinary way, off
    /// [`Terminal::read`], which is what keeps the Screen and the Capture the
    /// one account of what happened.
    pub async fn write(&self, keys: &[u8]) -> io::Result<()> {
        let mut left = keys;

        while !left.is_empty() {
            self.input.writable().await?;

            match self.input.try_write(left) {
                // A pipe that has said it is writable and then taken nothing is
                // one there is no progress to be made against.
                Ok(0) => {
                    return Err(io::Error::new(
                        io::ErrorKind::WriteZero,
                        "this terminal would take no more",
                    ));
                }
                Ok(put) => left = &left[put..],
                // Which is the pipe saying it was not ready after all, and is
                // the one error there is nothing to report about.
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => continue,
                Err(error) => return Err(error),
            }
        }

        Ok(())
    }

    /// Take what the session has printed, waiting until there is some.
    ///
    /// `Ok(0)` is the session gone: the console host closes its end of this
    /// pipe as it goes, and it goes when the console is closed — which is what
    /// the task beside every child does once that child has exited.
    pub async fn read(&self, buffer: &mut [u8]) -> io::Result<usize> {
        loop {
            match self.output.readable().await {
                Ok(()) => {}
                // The far end having gone, arriving while waiting for it to
                // say something rather than on the read itself. The same
                // answer either way.
                Err(error) if error.kind() == io::ErrorKind::BrokenPipe => return Ok(0),
                Err(error) => return Err(error),
            }

            match self.output.try_read(buffer) {
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => continue,
                // The runtime already reads a pipe whose writer has gone as the
                // end of what there is to read; this is the same answer said
                // again for the case where it arrives as an error, and it is
                // what `EIO` on a pseudo-terminal means on the other arm.
                Err(error) if error.kind() == io::ErrorKind::BrokenPipe => return Ok(0),
                read => return read,
            }
        }
    }
}

/// The process a session is, and the Job that holds everything it started.
///
/// What the sessions module asks of one is what it asks of tokio's on the other
/// arm — an id, an exit, a kill — and the promise that dropping it ends the
/// session is kept here by the Job rather than by the runtime.
pub struct Child {
    /// What the process is called in Task Manager, which is the whole of what
    /// anything above here does with it.
    id: u32,

    /// The Job everything this session starts is in. Dropping it closes the
    /// last handle to the Job, and a Job with no handles left kills what is
    /// inside it — see this module's own documentation.
    job: Job,

    /// How it ended, once it has — see [`Ended`].
    exited: watch::Receiver<Option<Ended>>,
}

impl Child {
    /// The process id, which a Windows child always has: nothing is reaped
    /// here, so there is no moment after which it has none.
    pub fn id(&self) -> Option<u32> {
        Some(self.id)
    }

    /// Wait for the session to end, and say how it did.
    ///
    /// Off the one thread waiting on the process rather than a wait of its own,
    /// so that asking twice — the relay reaping it, and the console being
    /// closed behind it — is asking one waiter twice.
    pub async fn wait(&mut self) -> io::Result<ExitStatus> {
        let ended = {
            let seen = self
                .exited
                .wait_for(|ended| ended.is_some())
                .await
                .map_err(|_| io::Error::other("nothing waited for this session"))?;

            // Off the shared word rather than held: what is being held while
            // this borrows is everything else's reading of how it ended.
            (*seen).clone()
        };

        match ended {
            Some(Ok(code)) => Ok(ExitStatus::from_raw(code)),
            Some(Err(error)) => Err(io::Error::other(error)),
            None => Err(io::Error::other("nothing waited for this session")),
        }
    }

    /// End the session: the Job rather than the process, so that what the
    /// session started goes with it.
    ///
    /// Asked for rather than waited on, the way tokio's is — what says it is
    /// over is [`Child::wait`].
    pub fn start_kill(&mut self) -> io::Result<()> {
        self.job.terminate(KILLED)
    }
}

/// Put a suspended `process` in a Job of its own and let it run — the two
/// halves of what makes a session's whole tree Verkstead's to end.
fn held_in_a_job(process: &Handle, thread: &Handle) -> io::Result<Job> {
    let job = Job::killing_everything_in_it()?;

    job.take(process.0)?;

    if unsafe { ResumeThread(thread.0) } == u32::MAX {
        return Err(io::Error::last_os_error());
    }

    Ok(job)
}

/// Close `console` once whatever was started on it has gone — see this module's
/// own documentation for why anything has to.
async fn closing(console: Arc<Console>, mut exited: watch::Receiver<Option<Ended>>) {
    // Whether it exited well or the wait itself failed: either way there is
    // nothing running on this console any more.
    let _ = exited.wait_for(|ended| ended.is_some()).await;

    tokio::time::sleep(FLUSHING).await;

    // On a thread of its own: closing a console waits for the console host to
    // go, and a wait of unknown length is not something to do on a thread the
    // server is answering requests on.
    let _ = tokio::task::spawn_blocking(move || console.close()).await;
}

/// The one wait on a process, made on a thread of its own — see
/// [`Terminal::spawn`].
fn awaited(process: &Handle) -> Ended {
    if unsafe { WaitForSingleObject(process.0, INFINITE) } == WAIT_FAILED {
        return Err(format!(
            "this session could not be waited for: {}",
            io::Error::last_os_error()
        ));
    }

    let mut code = 0u32;

    if unsafe { GetExitCodeProcess(process.0, &mut code) } == 0 {
        return Err(format!(
            "this session's exit could not be read: {}",
            io::Error::last_os_error()
        ));
    }

    Ok(code)
}

/// The pseudoconsole handle, closed once and by whoever gets there first.
///
/// Shared because two things end it: the task that closes it behind a session
/// that has exited, and the terminal being dropped — a session that was never
/// started, or a server going down under one that was.
struct Console(Mutex<Option<HPCON>>);

impl Console {
    /// The handle while it is open, and nothing once it is not.
    ///
    /// Read and let go of, which is only safe where nothing could be closing
    /// the console meanwhile: its one caller is [`Terminal::spawn`], and what
    /// closes a console is the task started beside a child that does not exist
    /// yet there. Everything else does its work under the lock — see
    /// [`Console::resize`].
    fn held(&self) -> Option<HPCON> {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Make the window `columns` by `rows`, under the lock the close takes.
    ///
    /// Under it rather than around it, because the two are different threads
    /// and a console read out of the lock is one that can be closed before it
    /// is used: what resizes a window is a watcher's browser, and what closes a
    /// console is the session on it ending.
    ///
    /// A console that is already closed takes it and says nothing: the session
    /// it belonged to has ended, and a window nobody is drawing in is not a
    /// failure to report to whoever resized it.
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
                "this terminal could not be resized: ResizePseudoConsole said {resized:#010x}"
            )));
        }

        Ok(())
    }

    /// Close it, if it is not closed already.
    ///
    /// This is what ends the console host, and with it the pipe the relay is
    /// reading — see [`Terminal::read`]. It waits for the host to go, which is
    /// why the task that closes a console behind a session does it on a thread
    /// of its own.
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

/// One handle of the process's own, closed when it is let go of.
///
/// A handle is a pointer as far as the bindings are concerned and therefore
/// neither `Send` nor `Sync` by itself, and it is both in fact: it is a number
/// the kernel looks up in a table this whole process shares, and nothing about
/// which thread holds it means anything.
struct Handle(HANDLE);

unsafe impl Send for Handle {}
unsafe impl Sync for Handle {}

impl Drop for Handle {
    fn drop(&mut self) {
        if !self.0.is_null() && self.0 != INVALID_HANDLE_VALUE {
            unsafe { CloseHandle(self.0) };
        }
    }
}

/// The attribute list a process is started with, holding the one attribute
/// there is to say: the console it comes up on.
///
/// A list is a block of memory Windows lays out itself, so this is a buffer of
/// pointer-sized words — the alignment a list wants — with the list written
/// into it, and it is deleted when it is dropped.
struct Attributes(Vec<usize>);

impl Attributes {
    /// A list of one, carrying `console`.
    fn carrying(console: HPCON) -> io::Result<Attributes> {
        let mut wanted = 0usize;

        // The first call always fails: what it is for is the size, which is
        // what it writes on its way out.
        unsafe { InitializeProcThreadAttributeList(ptr::null_mut(), 1, 0, &mut wanted) };

        if wanted == 0 {
            return Err(io::Error::last_os_error());
        }

        // The buffer before the list rather than after it: an
        // [`Attributes`] deletes the list as it is dropped, and there is no list
        // to delete until the call below has written one.
        let mut buffer = vec![0usize; wanted.div_ceil(size_of::<usize>())];

        let made = unsafe {
            InitializeProcThreadAttributeList(
                buffer.as_mut_ptr().cast::<c_void>(),
                1,
                0,
                &mut wanted,
            )
        };

        if made == 0 {
            return Err(io::Error::last_os_error());
        }

        let mut attributes = Attributes(buffer);

        let carried = unsafe {
            UpdateProcThreadAttribute(
                attributes.list(),
                0,
                PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE as usize,
                console as *const c_void,
                size_of::<HPCON>(),
                ptr::null_mut(),
                ptr::null(),
            )
        };

        if carried == 0 {
            return Err(io::Error::last_os_error());
        }

        Ok(attributes)
    }

    /// The list itself, as everything that takes one wants it.
    fn list(&mut self) -> LPPROC_THREAD_ATTRIBUTE_LIST {
        self.0.as_mut_ptr().cast::<c_void>()
    }
}

impl Drop for Attributes {
    fn drop(&mut self) {
        unsafe { DeleteProcThreadAttributeList(self.list()) };
    }
}

/// One direction of a terminal: the end Verkstead holds, watched by the
/// runtime, and the end the console gets.
///
/// A named pipe rather than an anonymous one — see this module's own
/// documentation — under a name nobody else can be at: this process's own id
/// and a number that is never given out twice, with
/// `FILE_FLAG_FIRST_PIPE_INSTANCE` to say that a name already taken is a
/// failure rather than somebody else's pipe.
///
/// Neither end is inheritable, so nothing the server spawns afterwards holds a
/// copy of a terminal that has nothing to do with it.
fn pipe() -> io::Result<(NamedPipeServer, Handle)> {
    /// What makes one pipe's name different from the next one's.
    static NAMED: AtomicU64 = AtomicU64::new(0);

    let name = wide(OsStr::new(&format!(
        r"\\.\pipe\verkstead-terminal-{}-{}",
        std::process::id(),
        NAMED.fetch_add(1, Ordering::Relaxed),
    )));

    let held = unsafe {
        CreateNamedPipeW(
            name.as_ptr(),
            PIPE_ACCESS_DUPLEX | FILE_FLAG_OVERLAPPED | FILE_FLAG_FIRST_PIPE_INSTANCE,
            PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT,
            1,
            BUFFER,
            BUFFER,
            0,
            ptr::null(),
        )
    };

    if held == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }

    let held = Handle(held);

    // Which connects: a pipe with a client on it is a connected pipe, whether
    // or not the end that made it ever asked to wait for one.
    let inside = unsafe {
        CreateFileW(
            name.as_ptr(),
            GENERIC_READ | GENERIC_WRITE,
            0,
            ptr::null(),
            OPEN_EXISTING,
            0,
            ptr::null_mut(),
        )
    };

    if inside == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }

    let inside = Handle(inside);

    // And the held end goes to the runtime, handle and all: from here it is
    // watched rather than read.
    let watched = held.0;
    std::mem::forget(held);

    let held = unsafe { NamedPipeServer::from_raw_handle(watched as RawHandle) }?;

    Ok((held, inside))
}

/// `rendering` as the command line `CreateProcessW` takes, program first.
///
/// Windows has no argument vector to hand over: a process is given one string
/// and takes it apart again, so this is the taking-apart run backwards — see
/// [`quoted`].
fn command_line(rendering: &Rendering) -> Vec<u16> {
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
fn environment(rendering: &Rendering) -> Vec<u16> {
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
fn wide(text: &OsStr) -> Vec<u16> {
    text.encode_wide().chain(std::iter::once(0)).collect()
}
