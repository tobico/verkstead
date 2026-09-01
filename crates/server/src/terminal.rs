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
//! terminal on Unix, and Windows has neither of those things. So there are two
//! arms here, and this is a `cfg` where the rest of the codebase would reach
//! for a value — see [`crate::platform::Platform`] for why that is the
//! preference — because there is no value to be had. The type on one platform
//! is a descriptor the runtime is watching, and on the other it is nothing at
//! all.

/// The terminal on the platforms that have one — see [`pty::Terminal`].
#[cfg(unix)]
mod pty;

/// And where there is none to open, which today is Windows — see
/// [`absent::Terminal`].
#[cfg(not(unix))]
mod absent;

#[cfg(not(unix))]
pub use absent::Terminal;
#[cfg(unix)]
pub use pty::Terminal;

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
