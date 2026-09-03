//! The XDG autostart entry, which is what [Launch on Startup](super) is on
//! Linux.
//!
//! A `.desktop` file in the user's autostart directory, which every desktop
//! that follows the specification reads when a session begins. It is named for
//! the app id — `net.tobico.Verkstead.desktop` — so that Verkstead's entry
//! stands among the other applications' the way the specification means it to,
//! and so that the file this writes is the file this reads and never anybody
//! else's.
//!
//! **The file is the whole of the state.** There is nothing else to keep: it is
//! there and Verkstead starts with the session, or it is not and Verkstead does
//! not, and a desktop's own settings turning it off is read here as the human
//! having unchecked the box.

use std::io;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::APP_ID;

/// What the entry calls Verkstead, which is what a Startup Applications list
/// draws beside its checkbox.
const NAME: &str = "Verkstead";

/// And what it says underneath, where the desktop shows one.
const COMMENT: &str = "The workbench, in the system tray";

/// Where autostart entries go under a configuration directory.
const AUTOSTART: &str = "autostart";

/// Verkstead's own autostart entry: where the file is, and the reading and
/// writing of it.
#[derive(Debug, Clone)]
pub(super) struct Entry {
    /// The `.desktop` file itself.
    file: PathBuf,
}

impl Entry {
    /// Where this machine keeps Verkstead's entry, or `None` where it names
    /// nowhere to keep one.
    ///
    /// The one read of the process environment here, made where the answer is
    /// asked for and passed down from there — see [`autostart_dir`], which is
    /// the same question of the values themselves.
    pub(super) fn here() -> Option<Entry> {
        let dir = autostart_dir(
            std::env::var_os("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .as_deref(),
            std::env::var_os("HOME").map(PathBuf::from).as_deref(),
        )?;

        Some(Entry::in_dir(&dir))
    }

    /// Verkstead's entry in `dir`, whatever is or is not there.
    pub(super) fn in_dir(dir: &Path) -> Entry {
        Entry {
            file: dir.join(format!("{APP_ID}.desktop")),
        }
    }

    /// Whether Verkstead is registered to start with the session.
    ///
    /// The file being there is most of it, and what it says is the rest — see
    /// [`says_on`]. A file that cannot be read at all is read as no
    /// registration: what this answers is whether Verkstead *is* started by
    /// this entry, and an entry nothing here can read is one nothing here can
    /// answer for.
    pub(super) fn on(&self) -> bool {
        std::fs::read_to_string(&self.file).is_ok_and(|entry| says_on(&entry))
    }

    /// Write the entry, naming what is running and — where what is running is
    /// not already a way into the app — the `verb` it was entered through, see
    /// [`Entered`](super::Entered).
    ///
    /// The autostart directory is made where it is not there: a machine that
    /// has never registered anything has never needed one, and the
    /// specification's answer is to make it.
    pub(super) fn write(&self, verb: &str) -> Result<()> {
        let named = running()?;
        let exe = named.path.to_str().with_context(|| {
            format!(
                "{} is not a path an autostart entry can name",
                named.path.display()
            )
        })?;

        if let Some(dir) = self.file.parent() {
            std::fs::create_dir_all(dir).with_context(|| format!("making {}", dir.display()))?;
        }

        std::fs::write(
            &self.file,
            written(exe, named.says_the_verb.then_some(verb)),
        )
        .with_context(|| format!("writing the autostart entry at {}", self.file.display()))
    }

    /// Take the entry away.
    ///
    /// One that is already gone is this having nothing to do rather than
    /// anything to report: what was asked for is that Verkstead not start with
    /// the session, and it does not.
    pub(super) fn remove(&self) -> Result<()> {
        match std::fs::remove_file(&self.file) {
            Ok(()) => Ok(()),
            Err(why) if why.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(why) => Err(why).with_context(|| {
                format!("removing the autostart entry at {}", self.file.display())
            }),
        }
    }
}

/// Where autostart entries go: `$XDG_CONFIG_HOME/autostart` where that is set
/// to an absolute path, and `~/.config/autostart` otherwise — the
/// specification's own reading, and the reading
/// [`verkstead_server::platform`] gives the directories Verkstead keeps its own
/// things in.
///
/// Not shared with that module, and deliberately: this is not one of
/// Verkstead's directories. It belongs to the desktop, what goes in it is a
/// registration with the desktop rather than anything of Verkstead's to keep,
/// and no flag of the server's has anything to say about where it is.
fn autostart_dir(config: Option<&Path>, home: Option<&Path>) -> Option<PathBuf> {
    let config = match config.filter(|dir| dir.is_absolute()) {
        Some(dir) => dir.to_owned(),
        None => home.filter(|dir| dir.is_absolute())?.join(".config"),
    };

    Some(config.join(AUTOSTART))
}

/// The entry as it is written, for a Verkstead at `exe` entered through `verb`
/// — or through nothing at all, where `exe` is its own way in.
///
/// **`--no-open` is in it**, which is the one decision the entry makes: a
/// startup launch is an ordinary launch of this app in every other way, and a
/// browser window arriving over whatever the human is doing at every login is
/// the thing that gets the box unchecked.
///
/// The verb goes between the two, unquoted: it is a word of the binary's own
/// grammar rather than a path, so there is nothing in it a desktop could read
/// as two arguments — see [`Entered::verb`](super::Entered::verb). And it is
/// left out where the entry point says it itself, which is [`running`]'s to
/// answer: an AppImage execs `verkstead desktop` from inside, so a `desktop`
/// here as well would start `verkstead desktop desktop`, which clap refuses.
///
/// The icon is named rather than pointed at, the way the specification means:
/// what a desktop draws beside this entry is whatever it has installed under
/// the app id, and a desktop that has none draws none.
fn written(exe: &str, verb: Option<&str>) -> String {
    let start = match verb {
        Some(verb) => format!("{} {verb}", quoted(exe)),
        None => quoted(exe),
    };

    format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name={NAME}\n\
         Comment={COMMENT}\n\
         Exec={start} --no-open\n\
         Icon={APP_ID}\n\
         Terminal=false\n"
    )
}

/// `exe` as an argument of an `Exec` line.
///
/// Quoted always rather than where it happens to need it: the specification
/// allows an argument to be quoted whole, and one rule is worth more here than
/// a table of which characters would have forced it. What is escaped inside the
/// quotes is what the specification says must be — the backslash for the value
/// itself, and then the quote, the backslash, the dollar and the backtick for
/// the argument inside it.
fn quoted(exe: &str) -> String {
    let mut quoted = String::with_capacity(exe.len() + 2);

    quoted.push('"');
    for character in exe.chars() {
        match character {
            // Escaped for the argument and then escaped again for the value the
            // argument is written in, which is what makes four of one.
            '\\' => quoted.push_str("\\\\\\\\"),
            '"' | '$' | '`' => {
                quoted.push_str("\\\\");
                quoted.push(character);
            }
            _ => quoted.push(character),
        }
    }
    quoted.push('"');

    quoted
}

/// Whether `entry` — a `.desktop` file as it was read — says Verkstead starts
/// with the session.
///
/// Being there is most of what an entry says, and two keys can say otherwise.
/// Both are how a desktop's own settings turn an entry off rather than delete
/// it: `Hidden`, which the specification defines as an entry that has been
/// deleted by the user, and the key GNOME's own Startup Applications writes.
/// Turning it off *is* unchecking the box, so both are read and neither is
/// argued with.
fn says_on(entry: &str) -> bool {
    !entry.lines().any(|line| {
        let Some((key, value)) = line.split_once('=') else {
            return false;
        };
        let (key, value) = (key.trim(), value.trim());

        (key.eq_ignore_ascii_case("Hidden") && value.eq_ignore_ascii_case("true"))
            || (key.eq_ignore_ascii_case("X-GNOME-Autostart-enabled")
                && value.eq_ignore_ascii_case("false"))
    })
}

/// What an entry names: the file, and whether the entry has to say the verb
/// after it.
struct Named {
    path: PathBuf,

    /// **False for an AppImage**, which is the one file here that is already a
    /// way into the app rather than a binary with several. Its `AppRun` execs
    /// `verkstead desktop "$@"`, so an entry that said the verb as well would
    /// hand clap `verkstead desktop desktop --no-open` and get a login that
    /// starts nothing at all.
    says_the_verb: bool,
}

/// What this run should be registered as, asked of the environment.
fn running() -> Result<Named> {
    named(std::env::var_os("APPIMAGE").map(PathBuf::from))
}

/// And the same worked out from what `$APPIMAGE` said, which is where the whole
/// of the decision is.
///
/// `$APPIMAGE` before the process's own path, where the variable names an
/// absolute one: an AppImage runs out of a filesystem its runtime mounted for
/// this run alone, so `current_exe` there is a path under `/tmp` that will not
/// be anything at the next login, and the runtime sets that variable to say
/// where the file the human actually has is. The reading belongs here, beside
/// the writing that would otherwise put a path with a lifetime of one run into
/// a file meant to outlive every run.
///
/// Taken as an argument rather than read here, so that both answers are a test
/// rather than a variable a suite would have to set on the process it is running
/// in.
fn named(appimage: Option<PathBuf>) -> Result<Named> {
    match appimage.filter(|path| path.is_absolute()) {
        Some(appimage) => Ok(Named {
            path: appimage,
            says_the_verb: false,
        }),
        None => Ok(Named {
            path: std::env::current_exe().context("finding the path of the running executable")?,
            says_the_verb: true,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The specification's own directory, which is what nearly every machine
    /// has: no variable set, and the entry under the home directory.
    #[test]
    fn the_entry_goes_under_the_configuration_directory() {
        assert_eq!(
            autostart_dir(None, Some(Path::new("/home/you"))),
            Some(PathBuf::from("/home/you/.config/autostart")),
        );
    }

    #[test]
    fn an_absolute_xdg_config_home_is_where_it_says() {
        assert_eq!(
            autostart_dir(
                Some(Path::new("/etc/xdg-you")),
                Some(Path::new("/home/you"))
            ),
            Some(PathBuf::from("/etc/xdg-you/autostart")),
        );
    }

    /// The specification's own rule about its variables, and the one
    /// `verkstead_server::platform` reads them by: a relative path is no
    /// answer, because a directory resolved against wherever the app was
    /// started is the thing the default replaces.
    #[test]
    fn a_relative_configuration_directory_is_no_answer() {
        assert_eq!(
            autostart_dir(Some(Path::new("xdg")), Some(Path::new("/home/you"))),
            Some(PathBuf::from("/home/you/.config/autostart")),
        );
        assert_eq!(
            autostart_dir(Some(Path::new("xdg")), Some(Path::new("."))),
            None
        );
        assert_eq!(autostart_dir(None, None), None);
    }

    /// The file is Verkstead's own and says so in its name.
    #[test]
    fn the_entry_is_named_for_the_app_id() {
        assert_eq!(
            Entry::in_dir(Path::new("/home/you/.config/autostart")).file,
            PathBuf::from("/home/you/.config/autostart/net.tobico.Verkstead.desktop"),
        );
    }

    /// What the entry starts, and the one decision it makes about how: the app
    /// as anybody else starts it — through the verb, because the executable it
    /// names has other verbs and only one of them is the app — with the browser
    /// left alone.
    #[test]
    fn the_entry_starts_this_executable_through_the_verb_without_opening_a_browser() {
        let entry = written("/usr/local/bin/verkstead", Some("desktop"));

        assert!(
            entry.contains("Exec=\"/usr/local/bin/verkstead\" desktop --no-open"),
            "got:\n{entry}"
        );
        assert!(entry.starts_with("[Desktop Entry]\n"), "got:\n{entry}");
        assert!(entry.contains("Type=Application\n"), "got:\n{entry}");
        assert!(entry.contains("Name=Verkstead\n"), "got:\n{entry}");
    }

    /// And an AppImage is named without one, because it is a way into the app
    /// rather than a binary with several verbs: its `AppRun` execs
    /// `verkstead desktop "$@"`, so an entry that said the verb as well would
    /// start `verkstead desktop desktop` and clap would refuse it.
    #[test]
    fn an_appimage_is_started_without_a_verb_because_it_says_its_own() {
        let named = named(Some(PathBuf::from(
            "/home/you/Apps/Verkstead-x86_64.AppImage",
        )))
        .expect("a variable naming an absolute path answers without asking the process");

        assert_eq!(
            named.path,
            PathBuf::from("/home/you/Apps/Verkstead-x86_64.AppImage"),
            "the file the human has is the one that outlives this run"
        );
        assert!(
            !named.says_the_verb,
            "and the entry point inside it is what says the verb"
        );

        let entry = written(named.path.to_str().unwrap(), None);
        assert!(
            entry.contains("Exec=\"/home/you/Apps/Verkstead-x86_64.AppImage\" --no-open\n"),
            "got:\n{entry}"
        );
    }

    /// And anything else is the running executable, said with the verb: what a
    /// desktop starts then is one binary out of several verbs.
    #[test]
    fn anything_but_an_appimage_is_the_running_executable_and_its_verb() {
        for said in [None, Some(PathBuf::from("relative/Verkstead.AppImage"))] {
            let named = named(said.clone()).expect("this process knows where it is");

            assert_eq!(
                named.path,
                std::env::current_exe().unwrap(),
                "{said:?} names no file that will still be there at the next login"
            );
            assert!(named.says_the_verb, "so the entry has to say the verb");
        }
    }

    /// A path a desktop would otherwise read as two arguments, which is what
    /// the quoting is there for — a downloads directory with a space in it is
    /// exactly where a binary somebody moved ends up.
    #[test]
    fn a_path_with_a_space_in_it_is_still_one_argument() {
        assert_eq!(
            quoted("/home/you/My Apps/verkstead"),
            "\"/home/you/My Apps/verkstead\"",
        );
        assert_eq!(quoted("/home/$you/it`s"), "\"/home/\\\\$you/it\\\\`s\"");
        assert_eq!(quoted("/home/you/a\\b"), "\"/home/you/a\\\\\\\\b\"");
    }

    /// The whole of the state: the file is there, so Verkstead starts with the
    /// session.
    #[test]
    fn an_entry_that_is_there_is_a_checked_box() {
        assert!(says_on(&written(
            "/usr/local/bin/verkstead",
            Some("desktop")
        )));
    }

    /// And a desktop's own settings turning it off is the human unchecking the
    /// box, whichever of the two ways they did it.
    #[test]
    fn an_entry_a_desktop_turned_off_is_an_unchecked_one() {
        assert!(!says_on("[Desktop Entry]\nType=Application\nHidden=true\n"));
        assert!(!says_on(
            "[Desktop Entry]\nType=Application\nX-GNOME-Autostart-enabled=false\n"
        ));
        assert!(says_on(
            "[Desktop Entry]\nType=Application\nX-GNOME-Autostart-enabled=true\n"
        ));
    }

    /// Written, read back, and taken away again: the three things the checkbox
    /// ever does to the file.
    #[test]
    fn writing_the_entry_and_taking_it_away_again() {
        let dir = tempfile::tempdir().unwrap();
        let entry = Entry::in_dir(&dir.path().join("autostart"));

        assert!(!entry.on(), "nothing has been registered yet");

        entry.write("desktop").unwrap();
        assert!(entry.on(), "the entry should be there and say so");
        assert!(
            std::fs::read_to_string(&entry.file)
                .unwrap()
                .contains(std::env::current_exe().unwrap().to_str().unwrap()),
            "the entry should name the executable that wrote it"
        );

        entry.remove().unwrap();
        assert!(!entry.on(), "the entry should be gone");
        entry
            .remove()
            .expect("one that is already gone is nothing to do");
    }

    /// The registration is the whole of what is written: no settings file, and
    /// nothing beside the entry in the directory it goes in.
    #[test]
    fn the_entry_is_the_only_thing_writing_it_makes() {
        let dir = tempfile::tempdir().unwrap();
        let autostart = dir.path().join("autostart");

        Entry::in_dir(&autostart).write("desktop").unwrap();

        let made: Vec<_> = std::fs::read_dir(&autostart)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();

        assert_eq!(made, [format!("{APP_ID}.desktop")]);
    }
}
