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
//!
//! **The one thing under a session that a platform has to answer for itself.**
//! Everything above this — the sessions module, the runner, the Screen, the
//! Capture — is ordinary portable Rust and is compiled wherever Verkstead is.
//! A pseudo-terminal is not: it is a pair of file descriptors and a controlling
//! terminal on Unix, and a pseudoconsole and two pipes on Windows. So there are
//! two arms here, and this is a `cfg` where the rest of the codebase would
//! reach for a value — see [`crate::platform::Platform`] for why that is the
//! preference — because there is no value to be had. The type on one platform
//! is a descriptor the runtime is watching, and on the other it is a console
//! handle with a pipe at each end of it.
//!
//! **What the two arms agree on is the whole of what is above them.** A
//! [`Terminal`] is opened, something is started on it, it is resized, written
//! into and read; a [`Child`] has an id, is waited for and can be killed, and
//! goes away taking whatever it started with it. Nothing above here knows which
//! arm it got.
//!
//! And what is started is a [`crate::sandbox::Rendering`] rather than a command
//! of the standard library's: a `Command` has already decided how a process is
//! spawned, and on Windows that decision is Verkstead's own — a
//! `CreateProcessW` carrying the pseudoconsole in an attribute list. So the
//! seam carries what the Sandbox described and each arm starts it the way its
//! platform starts anything.

/// The terminal on the platforms with a pseudo-terminal — see
/// [`pty::Terminal`].
#[cfg(unix)]
mod pty;

/// And on the one with a pseudoconsole instead — see [`conpty::Terminal`].
#[cfg(windows)]
mod conpty;

#[cfg(windows)]
pub use conpty::{Child, Terminal};
#[cfg(unix)]
pub use pty::{Child, Terminal};

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
