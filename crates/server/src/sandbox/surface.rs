//! What a session may reach, said once.
//!
//! One description, and two renderings of it: bubblewrap's flags on Linux — see
//! [`super::bwrap`] — and a deny-by-default policy on macOS — see
//! [`super::seatbelt`]. Neither rendering is the description, and nothing above
//! [`super::Sandbox`] learns which one it got.
//!
//! **The order is part of it.** A bind is applied in the order bwrap is given
//! it, so a directory said twice is the second one, and a temporary filesystem
//! made over a path is what a bind said before it lands under. So this is one
//! ordered list rather than a set of lists by kind, and a renderer walks it as
//! it was written.
//!
//! **The vocabulary is what both mechanisms can answer to**, which is narrower
//! than either of their own. A path is reachable at its own place or somewhere
//! else, and a session may read it or write it; a few things are made rather
//! than reached, being nobody's on the host. Everything a mechanism does about
//! *how* — a mount namespace, a policy expression, a directory really made and
//! a link really written — is the renderer's own business and none of this
//! file's.

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

/// How far into a path a session may reach.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Reach {
    ReadOnly,
    ReadWrite,
}

/// One thing about what a session may reach, in the order it was said.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Access {
    /// A path of the host's, reachable at its own place: the Worktree, the git
    /// directory behind it, a companion's checkout, a configured bind, the
    /// system directories.
    Own { path: PathBuf, reach: Reach },

    /// A path of the host's that a session finds somewhere else — a fresh HOME
    /// with the Profile's account inside it, the skills and the binary a
    /// session asks with under a directory of Verkstead's own.
    ///
    /// The one part of the vocabulary the two mechanisms answer differently in
    /// kind rather than in spelling: a bind is a path being somewhere it is
    /// not, and Apple's sandbox has no such thing to offer.
    Elsewhere {
        host: PathBuf,
        inside: PathBuf,
        reach: Reach,
    },

    /// A path a session must not reach, whatever is under it: the account's own
    /// skills, which are hidden rather than merged with.
    ///
    /// Said as an intention of its own rather than as one more
    /// [`Access::Elsewhere`], because it is the one place the two mechanisms
    /// answer a description in opposite directions. A mount makes something not
    /// be there by putting an empty directory of Verkstead's own over it —
    /// which is what `empty` is for — and a policy makes it not be there by
    /// refusing it, having nothing to put anywhere. Rendered as a bind, the
    /// second of them would write into the account it is standing on.
    Nothing { inside: PathBuf, empty: PathBuf },

    /// The process table, which is the sandbox's own where the platform keeps
    /// one in the filesystem.
    ProcessTable,

    /// The device nodes a program expects to be able to open: the null, the
    /// random, the terminal it is on.
    Devices,

    /// Somewhere to put a temporary file, holding nothing of the host's.
    Temporary(PathBuf),

    /// A directory that is really there and really empty, which is what a
    /// session's HOME is before the account lands in it.
    Empty(PathBuf),
}

/// The whole of what a command run inside a sandbox is given: what it may
/// reach, the environment it is handed, where it starts and what it runs.
#[derive(Debug, Clone)]
pub(crate) struct Surface {
    reaches: Vec<Access>,

    /// The environment in full. Nothing of the server's own comes through — a
    /// renderer clears it — so what is here is what a session has.
    env: Vec<(String, OsString)>,

    /// The directory the command starts in, which is the Conversation's
    /// Worktree.
    chdir: PathBuf,

    /// And the command itself, as an argument vector rather than a line: what
    /// runs inside is what the orchestrator built, and a shell between the two
    /// would be one more thing to quote for.
    argv: Vec<OsString>,
}

impl Surface {
    /// An empty surface, for a command that starts in `chdir`.
    pub(crate) fn starting_in(chdir: PathBuf) -> Surface {
        Surface {
            reaches: Vec::new(),
            env: Vec::new(),
            chdir,
            argv: Vec::new(),
        }
    }

    /// A path of the host's, at its own place.
    pub(crate) fn own(&mut self, path: impl Into<PathBuf>, reach: Reach) -> &mut Surface {
        self.reaches.push(Access::Own {
            path: path.into(),
            reach,
        });

        self
    }

    /// And one a session finds somewhere else.
    ///
    /// A path already where it is wanted is [`Access::Own`] rather than a bind
    /// of itself onto itself. Which is not a tidying: it is what becomes of
    /// these on the platform with no mounts to offer, where what a session
    /// finds is where the thing already is — and saying it here is what keeps
    /// both renderers from having to notice.
    pub(crate) fn elsewhere(
        &mut self,
        host: impl Into<PathBuf>,
        inside: impl Into<PathBuf>,
        reach: Reach,
    ) -> &mut Surface {
        let (host, inside) = (host.into(), inside.into());

        if host == inside {
            return self.own(host, reach);
        }

        self.reaches.push(Access::Elsewhere {
            host,
            inside,
            reach,
        });

        self
    }

    /// And a path a session must not reach, with the empty directory a mount
    /// puts over it — see [`Access::Nothing`].
    pub(crate) fn nothing(
        &mut self,
        inside: impl Into<PathBuf>,
        empty: impl Into<PathBuf>,
    ) -> &mut Surface {
        self.reaches.push(Access::Nothing {
            inside: inside.into(),
            empty: empty.into(),
        });

        self
    }

    /// Something that is inside without being anybody's on the host.
    pub(crate) fn made(&mut self, made: Access) -> &mut Surface {
        self.reaches.push(made);

        self
    }

    /// One variable of the environment a session is handed.
    pub(crate) fn set(&mut self, key: &str, value: impl AsRef<OsStr>) -> &mut Surface {
        self.env
            .push((key.to_owned(), value.as_ref().to_os_string()));

        self
    }

    /// What the command is.
    pub(crate) fn running<S: AsRef<OsStr>>(&mut self, argv: &[S]) -> &mut Surface {
        self.argv
            .extend(argv.iter().map(|word| word.as_ref().to_os_string()));

        self
    }

    pub(crate) fn reaches(&self) -> &[Access] {
        &self.reaches
    }

    pub(crate) fn env(&self) -> &[(String, OsString)] {
        &self.env
    }

    pub(crate) fn chdir(&self) -> &Path {
        &self.chdir
    }

    pub(crate) fn argv(&self) -> &[OsString] {
        &self.argv
    }
}
