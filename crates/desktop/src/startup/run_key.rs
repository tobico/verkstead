//! The Run key, which is what [Launch on Startup](super) is on Windows.
//!
//! A value under `HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\
//! Run`, which Windows starts everything named in when this user signs in. The
//! value is named for the app id — `net.tobico.Verkstead` — as the Linux entry
//! and the macOS agent are named for it, so that the value this writes is the
//! value this reads and never anybody else's, and what it holds is the command
//! line Windows runs.
//!
//! **The value is the whole of the state.** There is nothing else to keep: it
//! is there and Verkstead starts with the sign-in, or it is not and Verkstead
//! does not — the shape both other arms already have, in the one place Windows
//! keeps this kind of answer.
//!
//! **The current user's own key**, rather than the machine's. What the box on
//! the menu is about is this human's sign-in; the key under
//! `HKEY_LOCAL_MACHINE` would be every account on the machine, and writing it
//! wants an elevation nobody asked for to answer a question nobody asked.
//!
//! **A registry value is not a file**, which is the one place this arm is
//! shorter than the two beside it: there is no directory to make on the way and
//! no quoting rule beyond the one Windows itself has for a command line, and
//! the value is written whole rather than a document being formatted around it.
//!
//! **What this cannot see is the Startup tab.** A human who turns Verkstead off
//! in Task Manager is recorded in a key of Explorer's own — `StartupApproved` —
//! that is nothing to do with this value, so the box goes on showing what the
//! value says. That is the honest answer to what is written here rather than a
//! claim about what Windows will do, and it is the same answer the macOS arm
//! gives about Login Items; checking the box again rewrites the value either
//! way.
//!
//! **Built on Linux too, under `cfg(test)`** — the half of it that is text.
//! What a Run value holds is a command line, and what makes it right is the
//! same thing that makes the Linux `Exec` line right: the executable that is
//! running, quoted so that a path with a space in it is one argument, and
//! `--no-open` after it. So [`written`] and [`quoted`] are compiled and tested
//! wherever the suite runs, and what waits for a Windows machine is the
//! registry underneath them — which is [`Entry`], and which the suite tests
//! there against a key of its own.

#[cfg(windows)]
use anyhow::{Context, Result};

/// Where Windows keeps what it starts at sign-in, under the current user.
#[cfg(windows)]
const RUN: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

/// Verkstead's own value under a Run key: which key it is in, and the reading
/// and writing of it.
///
/// The key is held as a path rather than as an open handle: a handle would be
/// opened once at launch and read from for as long as the app runs, and what
/// this asks the registry is what somebody may have changed since.
#[cfg(windows)]
#[derive(Debug, Clone)]
pub(super) struct Entry {
    /// The key the value goes in, under `HKEY_CURRENT_USER`.
    key: String,
}

#[cfg(windows)]
impl Entry {
    /// Where this machine keeps Verkstead's registration, which on Windows is
    /// the one key there is.
    ///
    /// Never `None`: every Windows account has this key or can have it made,
    /// which is what makes this the arm with no such answer — the greyed menu
    /// item [`super::Startup`] draws is for a machine that names nowhere, and
    /// Windows names somewhere without being asked.
    pub(super) fn here() -> Option<Entry> {
        Some(Entry::under(RUN))
    }

    /// Verkstead's value in `key`, whatever is or is not there.
    pub(super) fn under(key: &str) -> Entry {
        Entry {
            key: key.to_owned(),
        }
    }

    /// Whether Verkstead is registered to start at sign-in.
    ///
    /// The value being there is the whole of it. A key that cannot be opened
    /// and a value that cannot be read are both read as no registration, for
    /// the reason the other two arms read an unreadable file that way: what
    /// this answers is whether Verkstead *is* started by this value, and one
    /// nothing here can read is one nothing here can answer for.
    pub(super) fn on(&self) -> bool {
        windows_registry::CURRENT_USER
            .open(&self.key)
            .and_then(|key| key.get_string(crate::APP_ID))
            .is_ok()
    }

    /// Write the value, naming the executable that is running and the `verb` it
    /// was entered through — see [`Entered`](super::Entered).
    ///
    /// The key is created where it is not there, which is what `create` does to
    /// one that is: a Windows account has this key from the start, and asking
    /// for it either way is one call rather than two.
    pub(super) fn write(&self, verb: &str) -> Result<()> {
        let exe = std::env::current_exe().context("finding the path of the running executable")?;
        let exe = exe
            .to_str()
            .with_context(|| format!("{} is not a path a Run value can name", exe.display()))?;

        windows_registry::CURRENT_USER
            .create(&self.key)
            .and_then(|key| key.set_string(crate::APP_ID, written(exe, verb)))
            .with_context(|| format!(r"writing HKEY_CURRENT_USER\{}", self.key))
    }

    /// Take the value away.
    ///
    /// One that is already gone is this having nothing to do rather than
    /// anything to report — the reading both other arms give a file that is
    /// already not there: what was asked for is that Verkstead not start at
    /// sign-in, and it does not. A key that is not there is the same answer for
    /// the same reason.
    pub(super) fn remove(&self) -> Result<()> {
        // Opened for writing as well as reading, which is what taking a value
        // away is: the reading above asks for no more than it needs, and this
        // asks for exactly as much.
        let Ok(key) = windows_registry::CURRENT_USER
            .options()
            .read()
            .write()
            .open(&self.key)
        else {
            // No key, so no registration in it. Made where one is written and
            // not made here: there is nothing to put in it.
            return Ok(());
        };

        match key.remove_value(crate::APP_ID) {
            Ok(()) => Ok(()),
            // Read back rather than told apart by its code: what was asked for
            // is that Verkstead not start at sign-in, and a value that is not
            // there is that.
            Err(_) if !self.on() => Ok(()),
            Err(why) => {
                Err(why).with_context(|| format!(r"removing from HKEY_CURRENT_USER\{}", self.key))
            }
        }
    }
}

/// The command line the value holds, for a Verkstead at `exe` entered through
/// `verb`.
///
/// **`--no-open` is in it**, which is the one decision the registration makes
/// and the one both other arms make: a sign-in start is an ordinary launch of
/// this app in every other way, and a browser window arriving over whatever the
/// human is doing at every sign-in is the thing that gets the box unchecked.
///
/// The verb goes between the two, unquoted for the reason the Linux entry
/// leaves it unquoted: it is a word of the binary's own grammar rather than a
/// path, so there is nothing in it Windows could read as two arguments.
#[cfg(any(windows, test))]
fn written(exe: &str, verb: &str) -> String {
    format!("{} {verb} --no-open", quoted(exe))
}

/// `exe` as the first word of a command line.
///
/// Quoted always rather than where it happens to need it, which is the rule the
/// Linux entry follows: one rule is worth more than a table of which characters
/// would have forced it, and `C:\Program Files\` is where half of Windows
/// lives. Nothing inside the quotes is escaped and nothing has to be — Windows
/// will not have a quote, an angle bracket or a pipe in a filename at all, so
/// the one character that would end this early cannot be in what it is quoting.
#[cfg(any(windows, test))]
fn quoted(exe: &str) -> String {
    format!("\"{exe}\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What the value starts, and the one decision it makes about how: the app
    /// as anybody else starts it — through the verb, because the exe it names
    /// has other verbs and only one of them is the app — with the browser left
    /// alone.
    #[test]
    fn the_value_starts_this_executable_through_the_verb_without_opening_a_browser() {
        assert_eq!(
            written(r"C:\Program Files\Verkstead\verkstead.exe", "desktop"),
            r#""C:\Program Files\Verkstead\verkstead.exe" desktop --no-open"#,
        );
    }

    /// A path Windows would otherwise read as two arguments, which is what the
    /// quoting is there for — and where a portable exe actually ends up.
    #[test]
    fn a_path_with_a_space_in_it_is_still_one_argument() {
        assert_eq!(
            quoted(r"C:\Program Files\Verkstead\verkstead.exe"),
            r#""C:\Program Files\Verkstead\verkstead.exe""#,
        );
    }

    /// The registry itself, which only a Windows machine has: a key of the
    /// test's own under this user's, written, read back and taken away again —
    /// the three things the checkbox ever does to it.
    ///
    /// Under `Software\net.tobico.Verkstead` rather than under the real Run
    /// key, so that a suite run on somebody's own machine registers nothing and
    /// unregisters nothing.
    #[cfg(windows)]
    #[test]
    fn writing_the_value_and_taking_it_away_again() {
        let key = scratch("writing");
        let entry = Entry::under(&key);

        assert!(!entry.on(), "nothing has been registered yet");

        entry.write("desktop").unwrap();
        assert!(entry.on(), "the value should be there");
        assert!(
            value(&key).contains(std::env::current_exe().unwrap().to_str().unwrap()),
            "the value should name the executable that wrote it, got: {}",
            value(&key)
        );
        assert!(
            value(&key).contains("--no-open"),
            "a sign-in should not be handed a browser window, got: {}",
            value(&key)
        );

        entry.remove().unwrap();
        assert!(!entry.on(), "the value should be gone");
        entry
            .remove()
            .expect("one that is already gone is nothing to do");

        forget(&key);
    }

    /// The launch of an exe that has moved: the value it left behind names
    /// where it was, and writing over it is where that is put right — see
    /// [`super::super::Startup::refresh`], which is what calls this at every
    /// launch while the box is checked, and only while it is.
    #[cfg(windows)]
    #[test]
    fn writing_over_a_value_that_names_somewhere_else() {
        let key = scratch("moved");
        let entry = Entry::under(&key);

        windows_registry::CURRENT_USER
            .create(&key)
            .unwrap()
            .set_string(
                crate::APP_ID,
                written(r"D:\where\it\used\to\be\verkstead.exe", "desktop"),
            )
            .unwrap();

        entry.write("desktop").unwrap();

        let registered = value(&key);
        assert!(
            registered.contains(std::env::current_exe().unwrap().to_str().unwrap()),
            "the value should name the executable that is running, got: {registered}"
        );
        assert!(
            !registered.contains(r"D:\where\it\used\to\be"),
            "and not the one it was written for, got: {registered}"
        );

        forget(&key);
    }

    /// A registration is one value, and the key it goes in holds nothing else
    /// of Verkstead's: there is no settings file behind it and nothing beside
    /// it.
    #[cfg(windows)]
    #[test]
    fn the_value_is_the_only_thing_writing_it_makes() {
        let key = scratch("alone");

        Entry::under(&key).write("desktop").unwrap();

        let written: Vec<_> = windows_registry::CURRENT_USER
            .open(&key)
            .unwrap()
            .values()
            .unwrap()
            .map(|(name, _)| name)
            .collect();

        assert_eq!(written, [crate::APP_ID.to_owned()]);

        forget(&key);
    }

    /// A key of this test's own, named for what it is about so that the tests
    /// in this file do not read each other's.
    #[cfg(windows)]
    fn scratch(about: &str) -> String {
        format!(r"Software\{}\tests\{about}", crate::APP_ID)
    }

    /// What Verkstead's value in `key` says.
    #[cfg(windows)]
    fn value(key: &str) -> String {
        windows_registry::CURRENT_USER
            .open(key)
            .unwrap()
            .get_string(crate::APP_ID)
            .unwrap()
    }

    /// And the key taken away again, so that a suite leaves the registry as it
    /// found it.
    #[cfg(windows)]
    fn forget(key: &str) {
        windows_registry::CURRENT_USER.remove_tree(key).unwrap();
    }
}
