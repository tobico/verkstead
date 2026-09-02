//! The terminal a platform without pseudo-terminals has not got, which today
//! is Windows.
//!
//! Everything above this compiles and is compiled here — the sessions module,
//! the runner, the Screen, the Capture — because all of it is ordinary Rust
//! about bytes and records. What is missing is the one thing underneath: the
//! terminal a session's agent would run on. Windows has a console API that
//! could be made into one, and making it is a later stage's; until then a
//! machine that would start a session gets a terminal that cannot be opened,
//! and the session is not started.
//!
//! **Uninhabited rather than empty.** [`Terminal::open`] is the only way to one
//! and it never returns one, so there is no `Terminal` on this platform for
//! anything to do anything with — which is why every method below is a match
//! with no arms rather than a body that panics or quietly does nothing. The
//! compiler is what says so, and nothing at runtime has to.
//!
//! What a human reads is not this. A refusal worded as `io::Error` is what the
//! log says of a session that could not be started; the answer the viewer draws
//! where a session would start says Windows and says not yet, and it is said
//! above the spawn rather than under it.

use std::io;

use tokio::process::{Child, Command};

/// One session's terminal, on a platform that has none to give.
///
/// See this module's own documentation: there is no value of this type, and
/// that is the whole of what it says.
pub enum Terminal {}

impl Terminal {
    /// Open a terminal — which here is to refuse to, in a line naming why.
    ///
    /// The one method with a body. What reads it is
    /// [`crate::sessions`], which logs the refusal and starts nothing.
    pub fn open() -> io::Result<Terminal> {
        Err(io::Error::other(
            "this Verkstead runs no sessions: a session's agent needs a \
             pseudo-terminal, and Windows has none to open",
        ))
    }

    /// Start `command` on this terminal — see [`Terminal::open`], which is the
    /// only thing that could have produced one.
    pub fn spawn(&mut self, _command: &mut Command) -> io::Result<Child> {
        match *self {}
    }

    /// Make the window `columns` by `rows` — see [`Terminal::open`].
    pub fn resize(&self, _columns: u16, _rows: u16) -> io::Result<()> {
        match *self {}
    }

    /// Put `keys` in at this end — see [`Terminal::open`].
    pub async fn write(&self, _keys: &[u8]) -> io::Result<()> {
        match *self {}
    }

    /// Take what the session has printed — see [`Terminal::open`].
    pub async fn read(&self, _buffer: &mut [u8]) -> io::Result<usize> {
        match *self {}
    }
}
