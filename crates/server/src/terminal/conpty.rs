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
//! The list itself, the quoting and the environment block are
//! [`crate::sandbox::starting`]'s: they are what a rendering comes to on this
//! platform wherever it is started, and this is one of the two places that
//! starts one.
//!
//! **And the account instead of the console, where a rendering names one.** A
//! session that runs as a local account of Verkstead's own — which is every
//! Windows session (ADR-0014, *Amended: the Sandbox is an account*) — cannot be
//! handed a console at all: `CreateProcessWithLogonW` refuses an attribute list
//! outright. So the console for one is made on the far side by a launcher — see
//! [`launcher`] — and this arm hands that launcher the two pipes as its plain
//! standard handles rather than making anything over them itself. Which is why a
//! terminal is *opened* as a pair of pipes and a size, and why the console
//! appears at [`Terminal::spawn`]: it is the rendering that says which side of
//! the boundary one is made on.
//!
//! **The console made here is therefore the unsandboxed one**: a Conversation
//! Terminal or a probe that names no account, started as Verkstead itself. No
//! session takes that arm.

//!
//! **And a launcher's console is the launcher's to close**, so the two things
//! this arm does about an ended session are done on the far side for one: the
//! launcher waits for the program, closes the console behind it, and exits —
//! which closes the last handle to each pipe and ends the reading here. What
//! comes back over its channel is the *session's* exit rather than its own.
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

use std::ffi::{OsStr, c_void};
use std::io;
use std::os::windows::io::RawHandle;
use std::os::windows::process::ExitStatusExt;
use std::process::ExitStatus;
use std::ptr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::windows::named_pipe::NamedPipeServer;
use tokio::sync::{mpsc, watch};
use windows_sys::Win32::Foundation::{
    GENERIC_READ, GENERIC_WRITE, INVALID_HANDLE_VALUE, WAIT_FAILED,
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
    CREATE_SUSPENDED, CREATE_UNICODE_ENVIRONMENT, CreateProcessW, EXTENDED_STARTUPINFO_PRESENT,
    GetExitCodeProcess, INFINITE, PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE, PROCESS_INFORMATION,
    ResumeThread, STARTF_USESTDHANDLES, STARTUPINFOEXW, TerminateProcess, WaitForSingleObject,
};

use super::launcher::{self, Channel};
use super::{COLUMNS, ROWS};
use crate::sandbox::Rendering;
use crate::sandbox::account::Logon;
use crate::sandbox::outliving::job::Job;
use crate::sandbox::starting::{
    Attributes, Handle, as_the_account, command_line, environment, wide,
};

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
    /// Where the console is, shared with the task that closes it when the
    /// process on it has gone — see [`Where`], and this module's own
    /// documentation.
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
    /// Open the two pipes a session's console is spoken to through, at
    /// [`COLUMNS`] by [`ROWS`].
    ///
    /// **The console is not made here**, which is the one thing this arm defers
    /// and the whole shape of the account boundary. A session that runs as
    /// Verkstead itself comes up on a console this process makes; one that runs
    /// as the session account comes up on a console a *launcher* makes on the
    /// far side, because `CreateProcessWithLogonW` will not carry one — see
    /// [`launcher`]. Which of the two it is, is the rendering's to say, and
    /// there is no rendering until [`Terminal::spawn`]. So what is opened here
    /// is the pair either console is built over, and the size the one that gets
    /// made will be made at.
    pub fn open() -> io::Result<Terminal> {
        let (output, printing) = pipe()?;
        let (input, typing) = pipe()?;

        Ok(Terminal {
            console: Arc::new(Console(Mutex::new(Where::Waiting {
                typing,
                printing,
                size: (COLUMNS, ROWS),
            }))),
            output,
            input,
            started: false,
        })
    }

    /// Start `rendering` on this terminal, and watch what it started.
    ///
    /// One of two, and the rendering says which: a session that names no
    /// account comes up on a console made here — see `Terminal::on_a_console`
    /// — and one that names an account comes up on a console a launcher makes
    /// as that account, which is `Terminal::through_a_launcher`. What is the
    /// same either way is everything above here: the two pipes are read and
    /// written the same, and a [`Child`] is waited on and killed the same.
    pub fn spawn(&mut self, rendering: &Rendering) -> io::Result<Child> {
        if self.started {
            return Err(io::Error::other(
                "this terminal has already had a session started on it",
            ));
        }

        let (typing, printing, size) = self.console.ends()?;

        self.started = true;

        match rendering.account() {
            Some(logon) => self.through_a_launcher(rendering, logon, typing, printing, size),
            None => self.on_a_console(rendering, typing, printing, size),
        }
    }

    /// A session on a console this process makes and holds.
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
    fn on_a_console(
        &mut self,
        rendering: &Rendering,
        typing: Handle,
        printing: Handle,
        size: (u16, u16),
    ) -> io::Result<Child> {
        let mut console: HPCON = 0;

        // The size is the console's own from the first byte it draws: there is
        // no resize to send afterwards, and a session that started on a window
        // of nothing would have drawn one frame for it.
        let opened = unsafe {
            CreatePseudoConsole(
                COORD {
                    X: i16::try_from(size.0).unwrap_or(i16::MAX),
                    Y: i16::try_from(size.1).unwrap_or(i16::MAX),
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

        self.console.here(console);

        // A list of one: the console this process just made, which is the whole
        // of what a rendering naming no account is started with.
        let mut attributes = Attributes::of(1)?;

        attributes.carrying(
            PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE as usize,
            console as *const c_void,
            size_of::<HPCON>(),
        )?;

        let mut startup: STARTUPINFOEXW = unsafe { std::mem::zeroed() };
        startup.StartupInfo.cb = u32::try_from(size_of::<STARTUPINFOEXW>()).unwrap_or(u32::MAX);
        startup.lpAttributeList = attributes.list();

        // Three standard handles said to be nothing, which is what puts the
        // session on the console it was given rather than on whatever the
        // server was started with.
        //
        // **A server's own output is redirected**, always: a service writes to
        // a log file and a `cargo test` writes down a pipe. A child of one is
        // handed those same three handles unless its parent says otherwise —
        // even one attached to a pseudoconsole, which then sets the console's
        // title, draws nothing on it, reads end-of-file where a watcher typed,
        // and asks how wide the window is only to be told it has none. So the
        // flag is set with the handles left zero, which says *these three, and
        // they are nothing*: the console the process comes up on is what fills
        // them in, and what a session prints reaches the pipe this terminal
        // reads.
        startup.StartupInfo.dwFlags = STARTF_USESTDHANDLES;

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

    /// And a session on a console made **as the session account**, by a
    /// launcher that is already it.
    ///
    /// The two pipes this terminal opened go to the launcher as its plain
    /// standard handles and it makes the console over them, which is the whole
    /// of why nothing is duplicated across the boundary — see [`launcher`],
    /// where the reason `CreateProcessWithLogonW` leaves no other route is.
    ///
    /// **What the Job holds is the launcher**, and everything under it is under
    /// the launcher: the session, the console host, and whatever the session
    /// starts. So an ended session leaves no launcher, and a server that dies
    /// takes both.
    ///
    /// **And what is waited on is not.** The launcher's own exit says nothing
    /// about how the session ended, so the session's exit comes back up the
    /// channel — see [`reporting`], which is the one place the two are told
    /// apart.
    fn through_a_launcher(
        &mut self,
        rendering: &Rendering,
        logon: &Logon,
        typing: Handle,
        printing: Handle,
        size: (u16, u16),
    ) -> io::Result<Child> {
        let Some(image) = rendering.launcher() else {
            return Err(io::Error::other(format!(
                "this rendering runs as the local account {} and names no image for the \
                 launcher that would make its console — see `Rendering::launched_by`",
                logon.name(),
            )));
        };

        // Before the launcher, because a launcher dialling a name nothing was
        // listening at would be refused.
        let channel = Channel::open(logon.name())?;

        let mut line = launcher::asking(image, channel.name(), size, rendering);
        let environment = environment(rendering);
        let chdir = rendering.chdir().map(|chdir| wide(chdir.as_os_str()));

        // Its standard error is the console's own pipe as well, so that a
        // launcher which would not start says so in the Capture of the session
        // that failed — the one terminal this module's documentation is about.
        let information = as_the_account(
            logon,
            &mut line,
            &environment,
            chdir.as_deref(),
            [typing.0, printing.0, printing.0],
            CREATE_SUSPENDED,
            "this session's launcher",
        )?;

        let process = Arc::new(Handle(information.hProcess));
        let thread = Handle(information.hThread);

        let job = match held_in_a_job(&process, &thread) {
            Ok(job) => job,
            Err(error) => {
                unsafe { TerminateProcess(process.0, KILLED) };

                return Err(error);
            }
        };

        // The launcher has its own copies now, and these are the copies that
        // would keep reading alive forever — the same reason a console's are
        // let go of the moment it has them.
        drop(printing);
        drop(typing);

        // From here a resize is a line on the channel rather than a call on a
        // handle: there is no console in this process to resize.
        let (resizes, asked) = mpsc::unbounded_channel();

        self.console.over_there(resizes);

        let (launched, launcher) = watch::channel(None);

        tokio::task::spawn_blocking({
            let process = process.clone();

            move || {
                let _ = launched.send(Some(awaited(&process)));
            }
        });

        let (over, exited) = watch::channel(None);

        tokio::spawn(reporting(channel.waiting(), asked, launcher, over));

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

/// Verkstead's end of a launcher's channel, for as long as the launcher is
/// there: the resizes on their way down, and the session's own exit on its way
/// back up.
///
/// **The one place the launcher's ending and the session's are told apart.** A
/// launcher exits when the program it started has exited *and* it has said so,
/// so what comes back off the channel is the session's code; the launcher's own
/// is the answer only where nothing was said at all — a launcher killed with
/// the Job, or one that would not start.
async fn reporting(
    channel: NamedPipeServer,
    asked: mpsc::UnboundedReceiver<(u16, u16)>,
    mut launcher: watch::Receiver<Option<Ended>>,
    over: watch::Sender<Option<Ended>>,
) {
    // Whichever happens first: the launcher dials, or it goes away without
    // having dialled.
    let dialled = tokio::select! {
        dialled = channel.connect() => Some(dialled),
        // Which is a launcher that refused: its complaint is already on the
        // console's own pipe, where the Capture will have it.
        _ = launcher.wait_for(|ended| ended.is_some()) => None,
    };

    let Some(dialled) = dialled else {
        let _ = over.send(Some(its_own(&launcher)));

        return;
    };

    if let Err(error) = dialled {
        let _ = over.send(Some(Err(format!(
            "this session's launcher could not be spoken to: {error}"
        ))));

        return;
    }

    let (reading, writing) = tokio::io::split(channel);

    tokio::spawn(resizing(writing, asked));

    let said = said_on(reading).await;

    // The launcher's own ending as well, whatever the channel said: a Child
    // that reported an exit before the launcher had gone would be one whose
    // Job still held a process.
    let _ = launcher.wait_for(|ended| ended.is_some()).await;

    let ended = match said {
        Some(code) => Ok(code),
        None => its_own(&launcher),
    };

    let _ = over.send(Some(ended));
}

/// How the launcher itself ended, read for the one case that has nothing else
/// to go on.
fn its_own(launcher: &watch::Receiver<Option<Ended>>) -> Ended {
    match &*launcher.borrow() {
        Some(ended) => ended.clone(),
        None => Err(String::from(
            "this session's launcher was never waited for, so nothing knows how it ended",
        )),
    }
}

/// Every resize a watcher asked for, written down the channel as the launcher
/// reads them.
///
/// It ends when the terminal that holds the other half of this is dropped, or
/// when the launcher has gone and the pipe will take no more — neither of which
/// is a failure to report to anybody: what resizes a window is somebody's
/// browser, and a window nobody is drawing in is not one to complain about.
async fn resizing(
    mut writing: tokio::io::WriteHalf<NamedPipeServer>,
    mut asked: mpsc::UnboundedReceiver<(u16, u16)>,
) {
    while let Some((columns, rows)) = asked.recv().await {
        if writing
            .write_all(launcher::resized(columns, rows).as_bytes())
            .await
            .is_err()
        {
            return;
        }
    }
}

/// The exit the launcher reported, off everything it said before its end of the
/// channel closed.
///
/// The last one rather than the first, and `None` where it said none: a
/// launcher writes one line and exits, and a launcher that was killed writes
/// nothing at all.
async fn said_on(mut reading: tokio::io::ReadHalf<NamedPipeServer>) -> Option<u32> {
    let mut buffer = [0u8; 256];
    let mut said = String::new();
    let mut ended = None;

    loop {
        let read = match reading.read(&mut buffer).await {
            Ok(0) | Err(_) => return ended,
            Ok(read) => read,
        };

        said.push_str(&String::from_utf8_lossy(&buffer[..read]));

        while let Some(at) = said.find('\n') {
            let line = said[..at].to_owned();
            said = said[at + 1..].to_owned();

            if let Some(code) = launcher::ended(&line) {
                ended = Some(code);
            }
        }
    }
}

/// Where a session's console is, and therefore what a resize goes to.
///
/// Three of these are a console and one is the absence of one, which is what a
/// terminal is opened as: which side of the boundary the console is made on is
/// the rendering's to say, and there is no rendering until something is
/// started.
enum Where {
    /// Not made yet: the two ends one will be made over, and the size it is to
    /// be made at. A resize arriving here is remembered rather than sent, which
    /// is what makes a watcher who attached before the session started see the
    /// window they asked for from its first frame.
    Waiting {
        typing: Handle,
        printing: Handle,
        size: (u16, u16),
    },

    /// Made here and held here, which is every session that is not started as
    /// another account.
    Here(HPCON),

    /// Made by a launcher on the far side of the account boundary: a resize
    /// goes down its channel, and closing it is the launcher's own to do.
    OverThere(mpsc::UnboundedSender<(u16, u16)>),

    /// And no console at all: one that has been closed, or one whose ends have
    /// been taken and not yet made into anything. A resize takes either and
    /// says nothing.
    Nowhere,
}

/// The console a session is on, whichever side of the boundary it was made on,
/// ended once and by whoever gets there first.
///
/// Shared because two things end it: the task that closes it behind a session
/// that has exited, and the terminal being dropped — a session that was never
/// started, or a server going down under one that was.
struct Console(Mutex<Where>);

impl Console {
    /// The two ends a console is to be made over and the size to make it at,
    /// taken out for whichever arm of [`Terminal::spawn`] is going to make one.
    ///
    /// Taken rather than borrowed, because what happens to them next is
    /// different on the two sides: one arm gives them to `CreatePseudoConsole`
    /// and one gives them to a launcher. What is left behind is [`Where::Nowhere`]
    /// — a spawn that then fails leaves a terminal with no console, which is
    /// what it is.
    fn ends(&self) -> io::Result<(Handle, Handle, (u16, u16))> {
        let mut held = self.0.lock().unwrap_or_else(PoisonError::into_inner);

        match std::mem::replace(&mut *held, Where::Nowhere) {
            Where::Waiting {
                typing,
                printing,
                size,
            } => Ok((typing, printing, size)),
            was => {
                *held = was;

                Err(io::Error::other(
                    "this terminal's console has already been made or closed",
                ))
            }
        }
    }

    /// The console this process just made, held from here.
    fn here(&self, console: HPCON) {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner) = Where::Here(console);
    }

    /// And the channel a console made on the far side is resized down.
    fn over_there(&self, resizes: mpsc::UnboundedSender<(u16, u16)>) {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner) = Where::OverThere(resizes);
    }

    /// Make the window `columns` by `rows`, under the lock the close takes.
    ///
    /// Under it rather than around it, because the two are different threads
    /// and a console read out of the lock is one that can be closed before it
    /// is used: what resizes a window is a watcher's browser, and what closes a
    /// console is the session on it ending.
    ///
    /// A console that is already closed takes it and says nothing, and so does
    /// a launcher that has gone: the session it belonged to has ended, and a
    /// window nobody is drawing in is not a failure to report to whoever
    /// resized it.
    fn resize(&self, columns: u16, rows: u16) -> io::Result<()> {
        let mut held = self.0.lock().unwrap_or_else(PoisonError::into_inner);

        match &mut *held {
            Where::Waiting { size, .. } => {
                *size = (columns, rows);

                Ok(())
            }
            Where::Here(console) => {
                let resized = unsafe {
                    ResizePseudoConsole(
                        *console,
                        COORD {
                            X: i16::try_from(columns).unwrap_or(i16::MAX),
                            Y: i16::try_from(rows).unwrap_or(i16::MAX),
                        },
                    )
                };

                if resized < 0 {
                    return Err(io::Error::other(format!(
                        "this terminal could not be resized: ResizePseudoConsole said \
                         {resized:#010x}"
                    )));
                }

                Ok(())
            }
            Where::OverThere(resizes) => {
                let _ = resizes.send((columns, rows));

                Ok(())
            }
            Where::Nowhere => Ok(()),
        }
    }

    /// Close it, if there is one here to close.
    ///
    /// This is what ends the console host, and with it the pipe the relay is
    /// reading — see [`Terminal::read`]. It waits for the host to go, which is
    /// why the task that closes a console behind a session does it on a thread
    /// of its own.
    ///
    /// A console made on the far side is the launcher's to close and is closed
    /// by it; what ends the reading there is the launcher exiting, which closes
    /// the last handles to both pipes.
    fn close(&self) {
        let was = std::mem::replace(
            &mut *self.0.lock().unwrap_or_else(PoisonError::into_inner),
            Where::Nowhere,
        );

        if let Where::Here(console) = was {
            unsafe { ClosePseudoConsole(console) };
        }
    }
}

impl Drop for Console {
    fn drop(&mut self) {
        self.close();
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
