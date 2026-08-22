//! The pseudo-terminal a session runs on, allocated and held by Verkstead.
//!
//! A session's agent is a terminal application, so it needs a terminal — and it
//! used to get one from `script` running inside the sandbox, whose stdin was
//! `/dev/null` and whose window nothing could resize. Verkstead opens the pair
//! itself instead: the sandbox is started with the far end as its stdin, stdout
//! and stderr, and this end is what the relay reads, what a resize is applied
//! to, and — once there is somebody watching — what a keystroke is written into
//! ([ADR 0007](../../../docs/adr/0007-server-held-terminal.md)).
//!
//! One terminal rather than a terminal and a pipe beside it. What the session
//! prints and what the sandbox complains about now arrive together and in
//! order, which is what a real terminal does and what makes a sandbox that
//! refused to start say so in the Capture of the session that failed.

use std::io;
use std::os::fd::{AsFd, BorrowedFd, OwnedFd};
use std::process::Stdio;

use rustix::fs::{Mode, OFlags};
use rustix::pty::OpenptFlags;
use rustix::termios::Winsize;
use tokio::io::Interest;
use tokio::io::unix::AsyncFd;
use tokio::process::{Child, Command};

/// How wide a session's terminal is until somebody watching says otherwise, and
/// how tall.
///
/// Fixed rather than inherited: the server has no terminal of its own to take a
/// size from, and a session started with none at all draws for a window zero
/// columns across. A hundred by thirty is a comfortable reading width for the
/// one thing that will resize it — a browser attaching later — to start from.
pub const COLUMNS: u16 = 100;

/// How tall — see [`COLUMNS`].
pub const ROWS: u16 = 30;

/// What a session is told its terminal is.
///
/// Nothing said before, and what an agent's interface draws depends on what it
/// thinks it has: with no `TERM` at all it falls back to the dumbest terminal
/// it knows. This is the one every emulator this will be watched through
/// answers to, and the one the server-side screen is written against.
pub const TERM: &str = "xterm-256color";

/// One session's terminal: the end Verkstead holds.
///
/// Opened before the session is spawned and held for as long as it runs. The
/// far end goes into the sandbox as its stdio and is let go of here the moment
/// it has — see [`Terminal::spawn`].
pub struct Terminal {
    /// What the session prints arrives here, and what is written here arrives
    /// at the session. Non-blocking and registered with the runtime, so reading
    /// it costs no thread.
    held: AsyncFd<OwnedFd>,

    /// The end the session gets, kept only until it has one.
    ///
    /// The reason it is let go of: reading [`Terminal::held`] reports
    /// end-of-file when the last of these is closed, and a copy left open here
    /// is one that never would be — so a session that had long since exited
    /// would read as one still running.
    inside: Option<OwnedFd>,
}

impl Terminal {
    /// Open a terminal, [`COLUMNS`] by [`ROWS`], for a session about to start.
    pub fn open() -> io::Result<Terminal> {
        // `NOCTTY` on both ends because neither is the server's: the process
        // that will take this as its controlling terminal is the sandbox, and
        // it says so for itself in [`Terminal::spawn`].
        let held =
            rustix::pty::openpt(OpenptFlags::RDWR | OpenptFlags::NOCTTY | OpenptFlags::CLOEXEC)?;

        // Read without waiting, which is what the runtime below requires of
        // anything it is asked to watch.
        rustix::io::ioctl_fionbio(&held, true)?;

        rustix::pty::grantpt(&held)?;
        rustix::pty::unlockpt(&held)?;

        let name = rustix::pty::ptsname(&held, Vec::new())?;
        let inside = rustix::fs::open(
            name.as_c_str(),
            OFlags::RDWR | OFlags::NOCTTY,
            Mode::empty(),
        )?;

        let terminal = Terminal {
            held: AsyncFd::new(held)?,
            inside: Some(inside),
        };

        terminal.resize(COLUMNS, ROWS)?;

        Ok(terminal)
    }

    /// Start `command` on this terminal, and let go of the end it now has.
    ///
    /// The three streams are the one terminal, which is what puts the sandbox's
    /// own complaints in among what the session printed. Before the sandbox is
    /// replaced by what it runs it takes a session of its own and makes this its
    /// controlling terminal — a terminal application asks its controlling
    /// terminal about itself, and one whose window nothing owns is one that
    /// hears about no resize.
    pub fn spawn(&mut self, command: &mut Command) -> io::Result<Child> {
        let Some(inside) = self.inside.take() else {
            return Err(io::Error::other(
                "this terminal has already had a session started on it",
            ));
        };

        command
            .stdin(Stdio::from(inside.try_clone()?))
            .stdout(Stdio::from(inside.try_clone()?))
            .stderr(Stdio::from(inside.try_clone()?));

        // Between the fork and the exec, where the three streams above are
        // already in place: file descriptor 0 is this terminal, and both calls
        // are a syscall and nothing else, which is the whole of what is allowed
        // in here.
        unsafe {
            command.pre_exec(|| {
                rustix::process::setsid()?;
                rustix::process::ioctl_tiocsctty(BorrowedFd::borrow_raw(0))?;
                Ok(())
            });
        }

        let child = command.spawn();

        // Whether it started or not: the copy that was to be handed over has
        // been, or there is nothing to hand it to.
        drop(inside);

        child
    }

    /// Make the window `columns` by `rows`, and tell whatever is running on it.
    ///
    /// The kernel's own notification rather than anything of Verkstead's: a
    /// terminal application asks its terminal how big it is when it is told the
    /// size changed, and this is what tells it.
    pub fn resize(&self, columns: u16, rows: u16) -> io::Result<()> {
        rustix::termios::tcsetwinsize(
            self.held.get_ref(),
            Winsize {
                ws_row: rows,
                ws_col: columns,
                ws_xpixel: 0,
                ws_ypixel: 0,
            },
        )?;

        Ok(())
    }

    /// Put `keys` in at this end, where the session reads them as typing.
    ///
    /// The other direction of the same terminal, and the whole of what a Hold
    /// does to one: a keystroke from a watcher is written here, and the session
    /// cannot tell it from a human at a keyboard of its own — which is the
    /// point, an agent that behaved differently for being driven from elsewhere
    /// being no use to drive.
    ///
    /// Written to the end rather than once, because a terminal takes what fits
    /// in its buffer and says how much that was. Nothing is echoed back from
    /// here: what the session makes of a keystroke comes round the ordinary
    /// way, off [`Terminal::read`], which is what keeps the Screen and the
    /// Capture the one account of what happened.
    pub async fn write(&self, keys: &[u8]) -> io::Result<()> {
        let mut left = keys;

        while !left.is_empty() {
            let chunk = left;

            let put = self
                .held
                .async_io(Interest::WRITABLE, move |held| {
                    rustix::io::write(held.as_fd(), chunk).map_err(io::Error::from)
                })
                .await?;

            // A terminal that has said it is writable and then taken nothing is
            // one there is no progress to be made against.
            if put == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::WriteZero,
                    "this terminal would take no more",
                ));
            }

            left = &left[put..];
        }

        Ok(())
    }

    /// Take what the session has printed, waiting until there is some.
    ///
    /// `Ok(0)` is the session gone. A terminal whose far end has closed answers
    /// a read with `EIO` rather than with nothing — there is no end-of-file on
    /// one — and the two mean the same thing to whoever is reading.
    pub async fn read(&self, buffer: &mut [u8]) -> io::Result<usize> {
        self.held
            .async_io(Interest::READABLE, |held| take(held.as_fd(), buffer))
            .await
    }
}

/// One read of the terminal, with the far end having closed reported as the
/// end of what there is to read — see [`Terminal::read`].
fn take(held: BorrowedFd<'_>, buffer: &mut [u8]) -> io::Result<usize> {
    match rustix::io::read(held, buffer) {
        Ok(read) => Ok(read),
        Err(rustix::io::Errno::IO) => Ok(0),
        Err(error) => Err(error.into()),
    }
}
