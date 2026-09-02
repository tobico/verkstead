//! The LaunchAgents plist, which is what [Launch on Startup](super) is on
//! macOS.
//!
//! A property list in the user's own agents directory —
//! `~/Library/LaunchAgents` — which `launchd` reads when that user's agent
//! domain comes up at login. It is named for the app id, as the Linux entry is,
//! so that the file this writes is the file this reads and never anybody
//! else's, and the `Label` inside it says the same id: an agent whose label
//! disagrees with its filename is one `launchd` has opinions about.
//!
//! **The file is the whole of the state**, which is the shape the Linux arm
//! already has: it is there and Verkstead starts with the login, or it is not
//! and Verkstead does not. Nothing here shells out to `launchctl` — an agent is
//! loaded when the domain it belongs to comes up, and the next login is the
//! only moment this box was ever about. There is nothing to load *now* that
//! would mean anything, and a `launchctl` that failed would be a second answer
//! to a question the file has already answered.
//!
//! **A plist works for a binary run from anywhere**, which is why it is this
//! rather than `SMAppService`: the modern registration is made by an app about
//! its own bundle, and Verkstead on a Mac is also a binary somebody built out
//! of a checkout and left in a directory of their own. What is registered here
//! is the executable that is running, wherever that is — inside
//! `Verkstead.app/Contents/MacOS` or not — and `launchd` starts it the same
//! way. The app asks AppKit for the Accessory activation policy as it comes up,
//! so a login start is a menu-bar app rather than a Dock tile whichever it was.
//!
//! **Built on Linux too, under `cfg(test)`.** Everything here is path and text:
//! nothing calls an Apple API, and the only thing that makes it macOS's is the
//! convention it follows. So the tests run on the Linux runner as
//! [`verkstead_server::platform`]'s three arms do, and the arm this machine
//! will never run is still an arm its tests call. What is macOS's alone is
//! [`Entry::here`], which asks *this* machine where its home is.

use std::io;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::APP_ID;

/// Where a user's own launch agents go, under the home directory.
const LAUNCH_AGENTS: &str = "Library/LaunchAgents";

/// Verkstead's own launch agent: where the plist is, and the reading and
/// writing of it.
#[derive(Debug, Clone)]
pub(super) struct Entry {
    /// The `.plist` file itself.
    file: PathBuf,
}

impl Entry {
    /// Where this machine keeps Verkstead's agent, or `None` where it names
    /// nowhere to keep one.
    ///
    /// The one read of the process environment here, made where the answer is
    /// asked for and passed down from there — see [`agents_dir`], which is the
    /// same question of the value itself.
    #[cfg(target_os = "macos")]
    pub(super) fn here() -> Option<Entry> {
        let dir = agents_dir(std::env::var_os("HOME").map(PathBuf::from).as_deref())?;

        Some(Entry::in_dir(&dir))
    }

    /// Verkstead's agent in `dir`, whatever is or is not there.
    pub(super) fn in_dir(dir: &Path) -> Entry {
        Entry {
            file: dir.join(format!("{APP_ID}.plist")),
        }
    }

    /// Whether Verkstead is registered to start at login.
    ///
    /// The file being there is most of it, and what it says is the rest — see
    /// [`says_on`]. A file that cannot be read at all is read as no
    /// registration, for the reason the Linux arm reads one that way: what this
    /// answers is whether Verkstead *is* started by this agent, and an agent
    /// nothing here can read is one nothing here can answer for.
    pub(super) fn on(&self) -> bool {
        std::fs::read_to_string(&self.file).is_ok_and(|plist| says_on(&plist))
    }

    /// Write the agent, naming the executable that is running.
    ///
    /// The agents directory is made where it is not there: a machine that has
    /// never registered anything has never needed one, and `launchd` reads the
    /// directory rather than requiring it.
    pub(super) fn write(&self) -> Result<()> {
        let exe = std::env::current_exe().context("finding the path of the running executable")?;
        let exe = exe
            .to_str()
            .with_context(|| format!("{} is not a path a launch agent can name", exe.display()))?;

        if let Some(dir) = self.file.parent() {
            std::fs::create_dir_all(dir).with_context(|| format!("making {}", dir.display()))?;
        }

        std::fs::write(&self.file, written(exe))
            .with_context(|| format!("writing the launch agent at {}", self.file.display()))
    }

    /// Take the agent away.
    ///
    /// One that is already gone is this having nothing to do rather than
    /// anything to report: what was asked for is that Verkstead not start at
    /// login, and it does not.
    pub(super) fn remove(&self) -> Result<()> {
        match std::fs::remove_file(&self.file) {
            Ok(()) => Ok(()),
            Err(why) if why.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(why) => Err(why)
                .with_context(|| format!("removing the launch agent at {}", self.file.display())),
        }
    }
}

/// Where a user's launch agents go: `~/Library/LaunchAgents`, and nowhere at
/// all where the home directory is not an absolute path.
///
/// Apple names this directory rather than leaving it to a variable, so the home
/// directory is the whole of the question — the same reading
/// [`verkstead_server::platform`] gives the macOS directories Verkstead keeps
/// its own things in, and the same rule about a relative one being no answer.
///
/// Not shared with that module, and deliberately, for the reason the Linux arm
/// is not: this is not one of Verkstead's directories. It belongs to `launchd`,
/// what goes in it is a registration with the login session rather than
/// anything of Verkstead's to keep, and no flag of the server's has anything to
/// say about where it is.
fn agents_dir(home: Option<&Path>) -> Option<PathBuf> {
    Some(home.filter(|dir| dir.is_absolute())?.join(LAUNCH_AGENTS))
}

/// The agent as it is written, for a Verkstead at `exe`.
///
/// **`--no-open` is in it**, which is the one decision the agent makes and the
/// one the Linux entry makes: a login start is an ordinary launch of this app
/// in every other way, and a browser window arriving over whatever the human is
/// doing at every login is the thing that gets the box unchecked.
///
/// `RunAtLoad` is what makes it a startup registration at all — the agent is
/// loaded when the login session comes up, and this is what says to start
/// Verkstead then rather than to wait for something to ask. Nothing else is in
/// it: no `KeepAlive`, because Exit on the tray menu is a human stopping
/// Verkstead and an agent that restarted it would be arguing with them.
fn written(exe: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \
         \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
         <plist version=\"1.0\">\n\
         <dict>\n\
         \t<key>Label</key>\n\
         \t<string>{APP_ID}</string>\n\
         \t<key>ProgramArguments</key>\n\
         \t<array>\n\
         \t\t<string>{}</string>\n\
         \t\t<string>--no-open</string>\n\
         \t</array>\n\
         \t<key>RunAtLoad</key>\n\
         \t<true/>\n\
         </dict>\n\
         </plist>\n",
        escaped(exe)
    )
}

/// `exe` as the text of an element.
///
/// A plist is XML, so a path is escaped rather than quoted — the Linux entry's
/// question in the other format. The three characters that end an element or
/// start an entity are the three there are: a path with an ampersand in it is
/// unusual and a path with a space in it is not, and the space needs nothing
/// here.
fn escaped(exe: &str) -> String {
    exe.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Whether `plist` — a launch agent as it was read — says Verkstead starts at
/// login.
///
/// Being there is most of what an agent says, and two keys can say otherwise.
/// `Disabled` is `launchd`'s own way of keeping an agent that is not to be run,
/// and `RunAtLoad` set false is an agent that is loaded and waits for something
/// else to start it — neither of which is Verkstead coming up with the login.
/// Both are read as the box unchecked and neither is argued with, the way the
/// Linux arm reads a desktop's own settings turning its entry off.
///
/// **What this cannot see is Login Items.** A human who turns Verkstead off in
/// System Settings is recorded in a database of `launchd`'s that is nothing to
/// do with this file, so the box goes on showing what the file says. That is
/// the honest answer to what is written here rather than a claim about what
/// `launchd` will do, and checking the box again rewrites the file either way.
fn says_on(plist: &str) -> bool {
    flag(plist, "Disabled") != Some(true) && flag(plist, "RunAtLoad") != Some(false)
}

/// What `key` is set to in `plist`, where it is set to a boolean at all.
///
/// A reader for the two keys above rather than a plist parser: the file being
/// read is the file this module wrote, and what is being asked of it is whether
/// somebody has turned it off since. Anything else in it — a key that is not
/// there, a value that is not a boolean, a file that is not a plist — is this
/// having nothing to say, which [`says_on`] reads as nothing said.
fn flag(plist: &str, key: &str) -> Option<bool> {
    let after = plist
        .split_once(&format!("<key>{key}</key>"))?
        .1
        .trim_start();

    if after.starts_with("<true") {
        Some(true)
    } else if after.starts_with("<false") {
        Some(false)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Apple's own directory, which is what every Mac has: the home directory
    /// and the place under it launch agents go.
    #[test]
    fn the_agent_goes_under_the_home_directory() {
        assert_eq!(
            agents_dir(Some(Path::new("/Users/you"))),
            Some(PathBuf::from("/Users/you/Library/LaunchAgents")),
        );
    }

    /// The rule `verkstead_server::platform` reads a home directory by: a
    /// relative path is no answer, and a machine that says nothing about where
    /// its home is has nowhere to keep a registration — which is the greyed
    /// menu item [`super::Startup`] already draws.
    #[test]
    fn a_relative_home_is_no_answer() {
        assert_eq!(agents_dir(Some(Path::new("Users/you"))), None);
        assert_eq!(agents_dir(None), None);
    }

    /// The file is Verkstead's own and says so in its name, as the label inside
    /// it says the same thing.
    #[test]
    fn the_agent_is_named_for_the_app_id() {
        assert_eq!(
            Entry::in_dir(Path::new("/Users/you/Library/LaunchAgents")).file,
            PathBuf::from("/Users/you/Library/LaunchAgents/net.tobico.Verkstead.plist"),
        );
        assert!(
            written("/Applications/Verkstead.app/Contents/MacOS/verkstead-desktop")
                .contains("<key>Label</key>\n\t<string>net.tobico.Verkstead</string>"),
        );
    }

    /// What the agent starts, and the one decision it makes about how: the app
    /// as anybody else starts it, with the browser left alone.
    #[test]
    fn the_agent_starts_this_executable_without_opening_a_browser() {
        let agent = written("/usr/local/bin/verkstead-desktop");

        assert!(
            agent.contains(
                "\t<key>ProgramArguments</key>\n\
                 \t<array>\n\
                 \t\t<string>/usr/local/bin/verkstead-desktop</string>\n\
                 \t\t<string>--no-open</string>\n\
                 \t</array>\n"
            ),
            "got:\n{agent}"
        );
        assert!(agent.starts_with("<?xml version=\"1.0\""), "got:\n{agent}");
        assert!(agent.contains("<plist version=\"1.0\">\n"), "got:\n{agent}");
    }

    /// A path with something in it that XML reads as markup, which is what the
    /// escaping is there for — a downloads directory somebody named is exactly
    /// where a binary they moved ends up.
    #[test]
    fn a_path_xml_would_read_as_markup_is_still_a_path() {
        assert_eq!(
            escaped("/Users/you/Apps & Things/verkstead-desktop"),
            "/Users/you/Apps &amp; Things/verkstead-desktop",
        );
        assert_eq!(escaped("/Users/you/<a>/b"), "/Users/you/&lt;a&gt;/b");
        assert_eq!(
            escaped("/Users/you/My Apps/verkstead-desktop"),
            "/Users/you/My Apps/verkstead-desktop",
        );
    }

    /// The whole of the state: the file is there, so Verkstead starts at login.
    #[test]
    fn an_agent_that_is_there_is_a_checked_box() {
        assert!(says_on(&written("/usr/local/bin/verkstead-desktop")));
    }

    /// And an agent somebody turned off is the human unchecking the box,
    /// whichever of the two ways they did it.
    #[test]
    fn an_agent_that_is_turned_off_is_an_unchecked_one() {
        let off = |key: &str, value: &str| {
            format!(
                "<plist version=\"1.0\">\n<dict>\n\
                 \t<key>Label</key>\n\t<string>{APP_ID}</string>\n\
                 \t<key>RunAtLoad</key>\n\t<true/>\n\
                 \t<key>{key}</key>\n\t<{value}/>\n\
                 </dict>\n</plist>\n"
            )
        };

        assert!(!says_on(&off("Disabled", "true")));
        assert!(says_on(&off("Disabled", "false")));
        assert!(!says_on(
            "<plist version=\"1.0\">\n<dict>\n\
             \t<key>RunAtLoad</key>\n\t<false/>\n</dict>\n</plist>\n"
        ));
    }

    /// Written, read back, and taken away again: the three things the checkbox
    /// ever does to the file, and the registration naming the executable that
    /// is running is the whole of what checking the box does.
    #[test]
    fn writing_the_agent_and_taking_it_away_again() {
        let dir = tempfile::tempdir().unwrap();
        let entry = Entry::in_dir(&dir.path().join("Library/LaunchAgents"));

        assert!(!entry.on(), "nothing has been registered yet");

        entry.write().unwrap();
        assert!(entry.on(), "the agent should be there and say so");
        assert!(
            std::fs::read_to_string(&entry.file)
                .unwrap()
                .contains(std::env::current_exe().unwrap().to_str().unwrap()),
            "the agent should name the executable that wrote it"
        );

        entry.remove().unwrap();
        assert!(!entry.on(), "the agent should be gone");
        entry
            .remove()
            .expect("one that is already gone is nothing to do");
    }

    /// The launch of a binary that has moved: the agent it left behind names
    /// where it was, and writing over it is where that is put right — see
    /// [`super::Startup::refresh`], which is what calls this at every launch
    /// while the box is checked, and only while it is.
    #[test]
    fn writing_over_an_agent_that_names_somewhere_else() {
        let dir = tempfile::tempdir().unwrap();
        let entry = Entry::in_dir(dir.path());

        std::fs::write(
            &entry.file,
            written("/somewhere/it/used/to/be/verkstead-desktop"),
        )
        .unwrap();

        entry.write().unwrap();

        let registered = std::fs::read_to_string(&entry.file).unwrap();
        let here = std::env::current_exe().unwrap();

        assert!(
            registered.contains(here.to_str().unwrap()),
            "the agent should name the executable that is running, got:\n{registered}"
        );
        assert!(
            !registered.contains("/somewhere/it/used/to/be/"),
            "and not the one it was written for, got:\n{registered}"
        );
    }

    /// The registration is the whole of what is written: no settings file, and
    /// nothing beside the agent in the directory it goes in.
    #[test]
    fn the_agent_is_the_only_thing_writing_it_makes() {
        let dir = tempfile::tempdir().unwrap();
        let agents = dir.path().join("Library/LaunchAgents");

        Entry::in_dir(&agents).write().unwrap();

        let made: Vec<_> = std::fs::read_dir(&agents)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();

        assert_eq!(made, [format!("{APP_ID}.plist")]);
    }
}
