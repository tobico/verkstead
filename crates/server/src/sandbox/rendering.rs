//! What a rendering hands back: the process a sandbox comes down to.
//!
//! A [`Surface`](super::surface::Surface) is what a session may reach and a
//! rendering is how this machine makes that true — bubblewrap's flags, Apple's
//! policy, or nothing at all where there is no boundary yet — and what each of
//! them ends at is the same four things: a program,
//! its arguments, the environment it is handed and the directory it starts in.
//! This is those four, said once, and it is what every renderer returns.
//!
//! **A description rather than a way of spawning.** It used to be a
//! `std::process::Command`, which is a description with a decision already
//! taken inside it: how the process is started. On the platforms with a
//! pseudo-terminal that decision is the standard library's and there is nothing
//! to say about it — but a Windows pseudoconsole is attached by an attribute
//! list on a `CreateProcessW` of Verkstead's own, and a `Command` is precisely
//! the thing that cannot carry one. So what crosses the seam is what was
//! described, and each arm of [`crate::terminal`] starts it the way its
//! platform starts anything.
//!
//! **The environment is the whole of it.** Every rendering clears what the
//! server was started with and says what a session gets — which is why there is
//! no vocabulary here for a variable being removed or left alone. What is in
//! [`Rendering::env`] is what the process has, and nothing else is.

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::Command;

/// One process, as a rendering left it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rendering {
    program: OsString,
    argv: Vec<OsString>,
    env: Vec<(OsString, OsString)>,

    /// Where it starts, where that is not wherever the server happens to be.
    ///
    /// `None` on the Linux rendering and not an oversight: what starts there is
    /// `bwrap`, whose own `--chdir` is what puts the session in its Worktree,
    /// and a directory said out here as well would be one the wrapper was asked
    /// to be in rather than the session.
    chdir: Option<PathBuf>,
}

impl Rendering {
    /// A process that runs `program`, with nothing said about it yet.
    pub fn running(program: impl Into<OsString>) -> Rendering {
        Rendering {
            program: program.into(),
            argv: Vec::new(),
            env: Vec::new(),
            chdir: None,
        }
    }

    /// One more argument.
    pub fn arg(&mut self, arg: impl AsRef<OsStr>) -> &mut Rendering {
        self.argv.push(arg.as_ref().to_os_string());

        self
    }

    /// And several, in the order they were given.
    pub fn args<S: AsRef<OsStr>>(&mut self, argv: impl IntoIterator<Item = S>) -> &mut Rendering {
        self.argv
            .extend(argv.into_iter().map(|word| word.as_ref().to_os_string()));

        self
    }

    /// One variable of the environment the process is handed — see this
    /// module's own documentation for why that is all there is.
    pub fn set(&mut self, key: impl AsRef<OsStr>, value: impl AsRef<OsStr>) -> &mut Rendering {
        self.env
            .push((key.as_ref().to_os_string(), value.as_ref().to_os_string()));

        self
    }

    /// The directory it starts in.
    pub fn starting_in(&mut self, chdir: impl Into<PathBuf>) -> &mut Rendering {
        self.chdir = Some(chdir.into());

        self
    }

    /// What runs.
    pub fn program(&self) -> &OsStr {
        &self.program
    }

    /// What it is given, the program's own name not among it: an argument
    /// vector here is the arguments, the way `Command` means it, rather than
    /// the whole of what a Windows command line holds.
    pub fn argv(&self) -> &[OsString] {
        &self.argv
    }

    /// The environment in full.
    pub fn env(&self) -> &[(OsString, OsString)] {
        &self.env
    }

    /// Where it starts, or nowhere in particular — see [`Rendering::chdir`]'s
    /// field for what `None` means on the Linux rendering.
    pub fn chdir(&self) -> Option<&Path> {
        self.chdir.as_deref()
    }
}

/// And the same thing as the standard library starts one, for everything that
/// is not a session: the compile server, and the tests that run a probe inside
/// a sandbox and read what it printed.
///
/// The one direction of the seam that does decide how to spawn. What a session
/// gets instead is [`crate::terminal::Terminal::spawn`], which decides
/// differently on each platform and is the whole reason the description exists.
impl From<&Rendering> for Command {
    fn from(rendering: &Rendering) -> Command {
        let mut command = Command::new(rendering.program());

        command.args(rendering.argv());

        // Nothing of the server's environment comes through — see this module's
        // own documentation.
        command.env_clear();

        for (key, value) in rendering.env() {
            command.env(key, value);
        }

        if let Some(chdir) = rendering.chdir() {
            command.current_dir(chdir);
        }

        command
    }
}
