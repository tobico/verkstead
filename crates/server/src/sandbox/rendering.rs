//! What a rendering hands back: the process a sandbox comes down to.
//!
//! A [`Surface`](super::surface::Surface) is what a session may reach and a
//! rendering is how this machine makes that true — bubblewrap's flags, Apple's
//! policy, or Windows' own identity for a process — and what each of them ends
//! at is the same four things: a program, its arguments, the environment it is
//! handed and the directory it starts in. This is those four, said once, and it
//! is what every renderer returns.
//!
//! **And a fifth on the platform whose boundary is not a wrapper: who the
//! process is.** A Linux rendering runs `bwrap` and a Mac's runs `sandbox-exec`,
//! so what makes the boundary is in the vector; a Windows one runs the session's
//! own program as a **local account of Verkstead's own** rather than as the
//! human (ADR-0014, *Amended: the Sandbox is an account*), which is an identity
//! on its token rather than a word on its command line. So a rendering names the
//! account it is started as — see [`Rendering::as_account`] — and the two
//! platforms with a wrapper name none.
//!
//! **And a sixth beside it**, which is how a process started as somebody else
//! comes up on a console at all: the image whose launcher verb makes one on the
//! far side of the boundary — see [`Rendering::launched_by`]. Both of the last
//! two are Windows' alone, and both are fields rather than `cfg`s: a description
//! is portable, so the suite builds and reads a Windows rendering on any
//! machine.
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

use super::account::Logon;
use crate::unseen::Unseen;

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

    /// The account it is started as, where it is not started as the account
    /// the server is — see [`Rendering::as_account`].
    ///
    /// `None` everywhere but the Windows rendering, and a field rather than a
    /// `cfg` for the reason [`crate::platform::Platform`] is a value: a
    /// description is portable and only the boundary is not, so a Windows
    /// rendering is a thing the suite can build and read on any machine.
    as_account: Option<Logon>,

    /// And the image whose launcher verb makes the console it comes up on,
    /// where it comes up on one at all — see [`Rendering::launched_by`].
    launched_by: Option<PathBuf>,
}

impl Rendering {
    /// A process that runs `program`, with nothing said about it yet.
    pub fn running(program: impl Into<OsString>) -> Rendering {
        Rendering {
            program: program.into(),
            argv: Vec::new(),
            env: Vec::new(),
            chdir: None,
            as_account: None,
            launched_by: None,
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

    /// The account it is started **as**, where that is not the account the
    /// server itself is running as — see [`Logon`], which is the name and the
    /// password together.
    ///
    /// The Windows rendering's own, and the whole of its boundary: a session is
    /// a local account of Verkstead's rather than the human (ADR-0014,
    /// *Amended: the Sandbox is an account*), and what it may reach is what that
    /// account has been granted — see [`super::granting`]. Starting a process as
    /// somebody else is `CreateProcessWithLogonW`, which is not a call the
    /// standard library makes at all: see the [`TryFrom`] below, which refuses a
    /// rendering that names one rather than quietly running it as the human.
    pub fn as_account(&mut self, logon: Logon) -> &mut Rendering {
        self.as_account = Some(logon);

        self
    }

    /// And the image whose launcher verb makes the console this comes up on.
    ///
    /// **The one thing a rendering says about how it is started rather than
    /// about what is started**, and it is here because it is a *path* the
    /// boundary has to reach: a console cannot be handed to a process started
    /// as another account, so one is made on the far side by a verb of
    /// Verkstead's own binary — see [`crate::terminal::launcher`] — and what
    /// that binary is, is whatever the session has bound read-only rather than
    /// wherever the server's own image happens to be. Which are two different
    /// paths, and only one of them is a path the session account can read.
    ///
    /// Read only where a rendering also names an account, and only by the arm
    /// that puts a process on a console: nothing off a console needs a launcher
    /// at all.
    pub fn launched_by(&mut self, image: impl Into<PathBuf>) -> &mut Rendering {
        self.launched_by = Some(image.into());

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

    /// The account it is started as, and nothing where it is started as the
    /// account the server is — see [`Rendering::as_account`].
    pub fn account(&self) -> Option<&Logon> {
        self.as_account.as_ref()
    }

    /// The image whose launcher verb makes its console — see
    /// [`Rendering::launched_by`].
    pub fn launcher(&self) -> Option<&Path> {
        self.launched_by.as_deref()
    }
}

/// And the same thing as the standard library starts one, for everything that
/// is not a session: the compile server, and the tests that run a probe inside
/// a sandbox and read what it printed.
///
/// The one direction of the seam that does decide how to spawn. What a session
/// gets instead is [`crate::terminal::Terminal::spawn`], which decides
/// differently on each platform and is the whole reason the description exists.
///
/// **It can refuse, which is what the `Try` is for.** A rendering that names an
/// account is a process to be started as somebody else, which is
/// `CreateProcessWithLogonW` — not a call the standard library makes at all. A
/// conversion that quietly dropped the account would hand back a command that
/// runs the same session as the *human*, which is precisely the identity the
/// boundary exists to be other than, and it would say nothing about it — which
/// is the one thing ADR-0014 refuses. So it is an error here, it names what was
/// asked for, and what starts such a rendering is `sandbox::off_a_console` or
/// the terminal's own spawn.
///
/// **And nothing is drawn for it**, which is [`crate::unseen`]: a process the
/// server starts to read something back off has no window to put in front of
/// the human, and on Windows it is given one unless it says otherwise.
impl TryFrom<&Rendering> for Command {
    type Error = std::io::Error;

    fn try_from(rendering: &Rendering) -> Result<Command, std::io::Error> {
        if let Some(logon) = rendering.account() {
            return Err(std::io::Error::other(format!(
                "this rendering runs as the local account {}, which the standard library has \
                 no way to start a process as — see `sandbox::off_a_console`",
                logon.name(),
            )));
        }

        let mut command = Command::new(rendering.program());

        command.args(rendering.argv()).unseen();

        // Nothing of the server's environment comes through — see this module's
        // own documentation.
        command.env_clear();

        for (key, value) in rendering.env() {
            command.env(key, value);
        }

        if let Some(chdir) = rendering.chdir() {
            command.current_dir(chdir);
        }

        Ok(command)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A rendering naming none, which is every rendering on the platforms with
    /// a wrapper to hide behind.
    #[test]
    fn a_rendering_as_nobody_in_particular_is_the_command_it_describes() {
        let mut rendering = Rendering::running("echo");
        rendering.arg("hello").set("HOME", "/nowhere");

        let command = Command::try_from(&rendering).expect("a rendering naming no account");

        assert_eq!(command.get_program(), "echo");
        assert_eq!(command.get_args().collect::<Vec<_>>(), ["hello"]);
    }

    /// And the refusal at the seam, which is what says a boundary is never lost
    /// by a conversion — see the [`TryFrom`] above. A rendering naming an
    /// account is a `CreateProcessWithLogonW`, which the standard library has
    /// no vocabulary for at all.
    ///
    /// Asked on every platform, because the description is portable: a Windows
    /// rendering built on a Linux machine names the same account, and what the
    /// standard library will do with it is the same nothing.
    #[test]
    fn a_rendering_as_an_account_is_not_a_command_the_standard_library_can_start() {
        let mut rendering = Rendering::running("cmd.exe");
        rendering.as_account(Logon::of("vk-0123456789ab", "a password nobody typed"));

        let refused = Command::try_from(&rendering)
            .expect_err("a rendering naming an account should not convert to a command");

        assert!(
            refused.to_string().contains("vk-0123456789ab"),
            "the refusal should say which account was asked for, and it said: {refused}"
        );
    }

    /// And it should not say the password while it is at it: a refusal is a log
    /// line, and this one carries the one secret under the Data Directory.
    #[test]
    fn no_refusal_and_no_debug_of_a_rendering_says_the_password() {
        let mut rendering = Rendering::running("cmd.exe");
        rendering.as_account(Logon::of("vk-0123456789ab", "Vk1-nobodyshouldreadthis"));

        let refused = Command::try_from(&rendering).expect_err("a rendering naming an account");

        assert!(!refused.to_string().contains("nobodyshouldreadthis"));
        assert!(
            !format!("{rendering:?}").contains("nobodyshouldreadthis"),
            "a rendering printed for a log should not carry the account's password",
        );
    }
}
