//! The open rendering: a [`Surface`] as the process it describes, with nothing
//! in front of it.
//!
//! The third of the three, and the one with no boundary in it. Where
//! [`super::bwrap`] hides the rest of the machine behind a mount namespace and
//! [`super::seatbelt`] refuses it with a policy, this does neither: it sets the
//! environment, starts in the directory the description said, and runs the
//! argument vector. A session rendered here reaches whatever the account
//! running the server reaches, and the workbench says so in words — see the
//! unsandboxed note on the Conversation view.
//!
//! **What it is still worth.** The environment is Verkstead's rather than
//! whatever the service was launched with, the working directory is the
//! Conversation's Worktree, and what runs is the vector the orchestrator built.
//! Those are the three things every rendering says, and they are what makes a
//! session on this platform a session at all rather than a shell inheriting the
//! server's world.
//!
//! **What it does not do is what the description's other half is for.** A path
//! a session was to find somewhere else — [`super::Access::Elsewhere`] — is nothing
//! this can make: there are no mounts here and no links written yet. Every one
//! of those that matters is a path whose two sides are already one directory,
//! which is what the Windows arms of [`super::own_directory`],
//! [`crate::skills::Skills::inside`], [`crate::handoffs::inside`] and
//! [`super::Executable`] are for — see [`super::Surface::elsewhere`], which
//! collapses a bind of a path onto itself. What is left is the Profile's
//! account and the handoff directory, and the stage after this one is what
//! joins those in.
//!
//! **Finding the program is this rendering's own work.** The two Unix
//! renderings hand a vector to a wrapper and the wrapper's own `execvp` finds
//! the first word of it; there is no wrapper here, and `CreateProcessW` finds
//! rather less than a Windows human expects — it appends `.exe` to a name with
//! no extension and knows nothing of `PATHEXT`, so an npm-installed
//! `claude.cmd` would not be found and, found, would not start. So the name is
//! resolved here the way the machine resolves one — see [`found`] — and a batch
//! file is run by the shell that can run one, which is the one wrapper in this
//! module and is the operating system's rather than the sandbox's.

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use super::rendering::Rendering;
use super::surface::{Access, Surface};

/// Where a program is looked for, and what a name with no extension may turn
/// out to be: the two variables a Windows machine resolves a command with.
///
/// Read out of the description rather than off the server, because what a
/// session can run is what the description says it can run — see
/// [`super::Sandbox::surface`], which is where both are set.
const PATH: &str = "PATH";
const PATHEXT: &str = "PATHEXT";

/// What a name with no extension may be where nothing said.
///
/// Windows' own default, and what a machine that has somehow lost the variable
/// still resolves a command with. Nothing a session gets reaches this — the
/// description sets `PATHEXT` off the machine — but the compile server's
/// surface says no such thing, and a lookup with no extensions at all would
/// find nothing at all.
const EXTENSIONS: &str = ".COM;.EXE;.BAT;.CMD";

/// How several directories or several extensions are written in one value.
///
/// A semicolon because this is a Windows value, wherever it is being read: a
/// colon there is a drive letter's own punctuation. Which is why nothing here
/// goes through [`std::env::split_paths`], whose separator is the one the
/// *server* was compiled for.
const BETWEEN: u8 = b';';

/// And the three characters that say a name is a place rather than something to
/// go looking for: both separators Windows accepts, and the colon a drive
/// letter ends with.
const A_PLACE: [u8; 3] = [b'\\', b'/', b':'];

/// The variable naming the shell a batch file is run by, and the shell where
/// nothing named one.
const COMSPEC: &str = "ComSpec";
const CMD: &str = "cmd.exe";

/// What a batch file is: the two extensions that shell runs and
/// `CreateProcessW` cannot. A `.cmd` is what npm writes for a command-line
/// program, so this is the ordinary way an agent is installed rather than a
/// corner.
const BATCH: [&str; 2] = ["cmd", "bat"];

/// How one is run: with the machine's `AutoRun` command left out of it, as the
/// one command this shell is being started for, and through `call` so that what
/// follows is read as arguments.
///
/// **`call` is what keeps the arguments whole**, and is the whole reason it is
/// here. `cmd /c` strips the first quote on its line and the last one on it
/// when the line begins with a quote — so a script path in quotes followed by
/// an argument in quotes comes apart in the middle. A line beginning with a
/// word instead is one that shell reads as it was written.
const AUTORUN_OFF: &str = "/d";
const COMMAND: &str = "/c";
const CALL: &str = "call";

/// `surface` as the process it describes, run as it stands.
pub(crate) fn command(surface: &Surface) -> Rendering {
    said_of_what_is_not_made(surface);

    let named = surface.argv().split_first();

    // The name as the machine resolves it, and the word itself where nothing on
    // the `PATH` answers to it: a program that is not there is a spawn that
    // fails saying which program, and a guess made here would only change what
    // it is called in the failure.
    let program = named.map(|(program, _)| {
        found(program, said(surface, PATH), said(surface, PATHEXT))
            .unwrap_or_else(|| PathBuf::from(program))
    });

    let mut open = match &program {
        Some(program) if batch(program) => {
            let mut open = Rendering::running(
                said(surface, COMSPEC).map_or_else(|| OsString::from(CMD), OsStr::to_os_string),
            );

            open.args([
                OsStr::new(AUTORUN_OFF),
                OsStr::new(COMMAND),
                OsStr::new(CALL),
                program.as_os_str(),
            ]);

            open
        }
        Some(program) => Rendering::running(program),
        // A description with no command in it, which nothing builds: the
        // rendering is still the environment and the directory, and what it
        // runs is nothing.
        None => Rendering::running(OsString::new()),
    };

    open.args(named.map_or(&[][..], |(_, arguments)| arguments));

    // Nothing of the server's environment comes through, which is said by
    // there being none here but this — see [`Rendering`], which is the whole of
    // what the process is handed.
    for (key, value) in surface.env() {
        open.set(key, value);
    }

    open.starting_in(surface.chdir());

    open
}

/// Say what this rendering cannot make, where somebody debugging a session on
/// this platform would look for it.
///
/// A path a session was described as finding somewhere else is nothing here:
/// there are no mounts, and nothing is linked yet. Every one that matters is
/// already the same directory on both sides, which the description itself
/// collapses — see this module's own documentation — and what is left is the
/// Profile's account and the Conversation's handoff directory, which the stage
/// after this one joins in. So this is a record of the gap rather than a
/// failure: a session starts either way, and a session that could not find its
/// account has this to read.
fn said_of_what_is_not_made(surface: &Surface) {
    for access in surface.reaches() {
        if let Access::Elsewhere { host, inside, .. } = access {
            tracing::debug!(
                host = %host.display(),
                inside = %inside.display(),
                "nothing on this platform puts a path somewhere it is not, so a \
                 session will not find this one where it was told to",
            );
        }
    }
}

/// What `surface` says `name` is, or nothing where it says nothing.
///
/// Case-folded, and the last of a name is the one that stands: both are what an
/// environment on this platform means by a name, and the second is what setting
/// a variable twice means everywhere else in this codebase.
fn said<'a>(surface: &'a Surface, name: &str) -> Option<&'a OsStr> {
    surface
        .env()
        .iter()
        .rev()
        .find(|(key, _)| key.eq_ignore_ascii_case(name))
        .map(|(_, value)| value.as_os_str())
}

/// Where `program` really is, resolved the way the machine resolves a name.
///
/// A name holding a separator or a drive letter is a place and is taken as one;
/// anything else is looked for in each directory of `path`, in order. In either
/// case a name that already carries an extension is tried as it stands and
/// nothing is appended to it, and one that carries none is tried under each
/// extension of `pathext` — which is what `cmd.exe` does, and is what keeps an
/// npm install from being read wrong: that writes `claude.cmd` beside a
/// `claude` with no extension at all, and the second is a shell script for a
/// Unix that nothing on this platform can start.
///
/// `None` where nothing answers, which is the caller's to word.
fn found(program: &OsStr, path: Option<&OsStr>, pathext: Option<&OsStr>) -> Option<PathBuf> {
    let extensions = pathext.unwrap_or_else(|| OsStr::new(EXTENSIONS));

    if a_place(program) {
        return under(Path::new(program), extensions);
    }

    apart(path?)
        .map(Path::new)
        .find_map(|directory| under(&directory.join(program), extensions))
}

/// One name, tried as it stands where it has an extension and under each of
/// `extensions` where it has none.
fn under(named: &Path, extensions: &OsStr) -> Option<PathBuf> {
    if named.extension().is_some() {
        return named.is_file().then(|| named.to_owned());
    }

    apart(extensions)
        .map(|extension| {
            let mut candidate = named.as_os_str().to_owned();
            candidate.push(extension);

            PathBuf::from(candidate)
        })
        .find(|candidate| candidate.is_file())
}

/// Whether `name` says where it is rather than what to go looking for.
fn a_place(name: &OsStr) -> bool {
    name.as_encoded_bytes()
        .iter()
        .any(|byte| A_PLACE.contains(byte))
}

/// And whether what was found is a batch file, which is the one thing on this
/// platform that is a program to a human and not one to `CreateProcessW`.
fn batch(program: &Path) -> bool {
    program
        .extension()
        .and_then(OsStr::to_str)
        .is_some_and(|extension| {
            BATCH
                .iter()
                .any(|batch| extension.eq_ignore_ascii_case(batch))
        })
}

/// The pieces of a value that holds several, in the order they were written,
/// with the empty ones left out — an empty `PATH` entry means the working
/// directory, which is not somewhere a sandbox goes looking for a program.
///
/// Split by hand rather than by [`std::env::split_paths`], which splits on the
/// separator of whatever platform the *server* was compiled for: this is a
/// Windows value wherever it is being read, and a test on a Linux machine
/// asking this arm what it resolves is asking about one written with
/// semicolons.
///
/// Putting the pieces back is what an `OsStr`'s own encoding documents as
/// allowed: it is self-synchronising, so an ASCII byte is never part of
/// anything else and a split on one lands on a boundary.
fn apart(value: &OsStr) -> impl Iterator<Item = &OsStr> {
    value
        .as_encoded_bytes()
        .split(|byte| *byte == BETWEEN)
        .filter(|piece| !piece.is_empty())
        .map(|piece| unsafe { OsStr::from_encoded_bytes_unchecked(piece) })
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::sandbox::surface::Surface;

    /// What a session would be started with: the directory it starts in, the
    /// `PATH` it looks for a program on, the extensions that `PATH` is read
    /// with, and the shell a batch file is run by.
    ///
    /// The machine's own names beside them where the tests are on the machine
    /// they are about, because a probe started with none of them is a program
    /// that does not load — see [`super::super::windows_names`].
    fn described(bin: &Path, argv: &[&str]) -> Surface {
        let mut surface = Surface::starting_in(bin.to_owned());

        surface
            .set(PATH, bin.as_os_str())
            .set(PATHEXT, EXTENSIONS)
            .set(COMSPEC, CMD);

        for (name, value) in super::super::windows_names(bin) {
            surface.set(name, value);
        }

        surface.running(argv);

        surface
    }

    /// A file that is really there, wherever a test's temporary directory is.
    ///
    /// Named with the case [`EXTENSIONS`] is written in, because the machine
    /// running these tests may be one that tells `.exe` and `.EXE` apart and the
    /// machine they are about is not.
    fn program(bin: &Path, name: &str) -> PathBuf {
        let path = bin.join(name);

        std::fs::write(&path, "a program\n").unwrap();

        path
    }

    /// The whole of what this rendering does with a name: finds it on the
    /// `PATH` under an extension the machine names, and runs it as it stands.
    #[test]
    fn a_program_is_found_on_the_path_under_an_extension_the_machine_names() {
        let bin = tempfile::tempdir().unwrap();
        let claude = program(bin.path(), "claude.EXE");

        let open = command(&described(bin.path(), &["claude", "--print"]));

        assert_eq!(
            open.program(),
            claude.as_os_str(),
            "an agent is named without its extension and found with one"
        );
        assert_eq!(
            open.argv(),
            [OsString::from("--print")],
            "and nothing stands in front of it"
        );
        assert_eq!(open.chdir(), Some(bin.path()));
    }

    /// And what an npm install leaves behind, which is the case this is for:
    /// a `.cmd` for this platform beside a `claude` with no extension at all,
    /// which is a shell script for a Unix and nothing `CreateProcessW` can
    /// start.
    #[test]
    fn a_batch_file_is_found_rather_than_the_unix_script_beside_it() {
        let bin = tempfile::tempdir().unwrap();
        program(bin.path(), "claude");
        let claude = program(bin.path(), "claude.CMD");

        let open = command(&described(bin.path(), &["claude", "--print"]));

        assert_eq!(
            open.program(),
            OsStr::new(CMD),
            "a batch file is a program to a human and not to the operating system"
        );
        assert_eq!(
            open.argv(),
            [
                OsString::from(AUTORUN_OFF),
                OsString::from(COMMAND),
                OsString::from(CALL),
                claude.into_os_string(),
                OsString::from("--print"),
            ],
            "so the shell that can run one runs it, and `call` is what keeps the \
             arguments after it whole"
        );
    }

    /// A name that says where it is, is taken at its word — and still resolved
    /// under an extension, which is what a `PATH` entry naming a program
    /// without one comes to.
    #[test]
    fn a_name_that_says_where_it_is_is_not_looked_for_anywhere() {
        let bin = tempfile::tempdir().unwrap();
        let claude = program(bin.path(), "claude.EXE");
        let elsewhere = tempfile::tempdir().unwrap();

        let mut surface = described(elsewhere.path(), &[]);
        surface.running(&[bin.path().join("claude")]);

        assert_eq!(
            command(&surface).program(),
            claude.as_os_str(),
            "nothing on the `PATH` was asked, the name having said where it is"
        );
    }

    /// And a name nothing answers to is the name itself, so that what fails is
    /// the spawn and what it names is what the description asked for.
    #[test]
    fn a_name_nothing_answers_to_is_left_as_it_was_written() {
        let bin = tempfile::tempdir().unwrap();

        let open = command(&described(bin.path(), &["claude"]));

        assert_eq!(open.program(), OsStr::new("claude"));
    }

    /// The environment is the description's and nothing else is in it, and the
    /// directory is the one the description said.
    #[test]
    fn the_environment_and_the_directory_are_the_descriptions_own() {
        let bin = tempfile::tempdir().unwrap();
        program(bin.path(), "claude.EXE");

        let mut surface = described(bin.path(), &["claude"]);
        surface.set("VERKSTEAD_SERVER", "http://127.0.0.1:8422/conversations/7");

        let open = command(&surface);

        assert!(
            open.env()
                .iter()
                .any(|(key, value)| key == "VERKSTEAD_SERVER"
                    && value == "http://127.0.0.1:8422/conversations/7"),
            "what the description set is what the process is handed"
        );
        assert_eq!(
            open.env().len(),
            surface.env().len(),
            "and nothing else is: {:?}",
            open.env()
        );
    }

    /// What the rendering comes to when it is run, asked on the machine it is
    /// for.
    ///
    /// A batch file that says what it was given, started through the rendering
    /// and answering — which is the half of this that cannot be asked anywhere
    /// else: a `.cmd` is a program only Windows has a shell for.
    #[cfg(windows)]
    #[test]
    fn a_batch_file_on_the_path_starts() {
        let bin = tempfile::tempdir().unwrap();

        std::fs::write(
            bin.path().join("claude.cmd"),
            "@echo off\r\necho started-as-a-batch-file %1\r\n",
        )
        .unwrap();

        let said = ran(&described(bin.path(), &["claude", "the-argument"]));

        assert!(
            said.contains("started-as-a-batch-file the-argument"),
            "an npm-installed agent should start and be given its arguments, \
             and it said: {said:?}"
        );
    }

    /// And an executable found the same way, given an argument that holds both
    /// of the things a command line is taken apart on.
    ///
    /// PowerShell under the agent's name, because a test cannot write an
    /// executable image and every Windows machine already has one — what is
    /// being asked is that the name resolved and that the argument arrived, and
    /// which image it resolved to says nothing about either.
    #[cfg(windows)]
    #[test]
    fn an_executable_is_found_the_same_way_and_its_arguments_arrive_whole() {
        const WHOLE: &str = r#"a "quoted" word"#;

        let bin = tempfile::tempdir().unwrap();
        let powershell = found(
            OsStr::new("powershell"),
            Some(OsStr::new(
                &std::env::var("PATH").expect("this machine has a PATH"),
            )),
            None,
        )
        .expect("every Windows machine has Windows PowerShell");

        std::fs::copy(powershell, bin.path().join("claude.exe")).unwrap();

        let say = bin.path().join("say.ps1");
        std::fs::write(&say, "[Console]::Out.Write($args[0])\r\n").unwrap();

        let said = ran(&described(
            bin.path(),
            &[
                "claude",
                "-NoProfile",
                "-NonInteractive",
                "-File",
                &say.display().to_string(),
                WHOLE,
            ],
        ));

        assert_eq!(
            said, WHOLE,
            "an argument holding a space and a quote should arrive as the one \
             argument it was"
        );
    }

    /// Run what `surface` describes and hand back what it printed.
    #[cfg(windows)]
    fn ran(surface: &Surface) -> String {
        let output = std::process::Command::from(&command(surface))
            .output()
            .expect("the rendering to be startable");

        assert!(
            output.status.success(),
            "the probe failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        String::from_utf8_lossy(&output.stdout).trim().to_owned()
    }
}
