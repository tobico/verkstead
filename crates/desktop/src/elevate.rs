//! Asking this desktop for a privilege the app has not got, the way this
//! desktop asks for one.
//!
//! One command is ever raised: the operator grant, which is what Tailscale wants
//! before it will take a `tailscale serve` from anybody but root (ADR-0015). The
//! press is made in a browser, the server that runs it has no privilege to
//! raise, and the line it hands back — `sudo tailscale set --operator=<user>` —
//! was a line somebody had to find a terminal for. What this module is, is the
//! app answering that press with the platform's own password dialog instead.
//!
//! **The seam is the server's.** The desktop crate depends on the server crate
//! rather than the other way round, so what crosses is a handle handed in as the
//! server starts — [`verkstead_server::remote::Elevate`] — and a server this app
//! did not start is handed none and shows the line as it always did.
//!
//! **Three arms, and each of them a command.** `pkexec` on Linux, `osascript`
//! running the command *with administrator privileges* on macOS, and a
//! UAC-elevated process through PowerShell's `Start-Process -Verb RunAs` on
//! Windows. Nothing here is a dialog this app draws: each of the three is the
//! platform's own asking, put in front of a command by starting a program, so
//! this crate's own toolkit is not involved and neither is the thread it holds
//! — see [`crate::dialog`], which is the opposite case.
//!
//! **Which arm is a value rather than a `cfg`**, for the reason
//! [`Platform`] is one: the arm a machine will never run is still an arm a test
//! on that machine can build the command of, and what these three are is
//! precisely a command each.
//!
//! **A cancelled dialog exits non-zero**, which is the one thing all three agree
//! on: `pkexec` exits 126 for a dismissal, `osascript` fails on the AppleScript
//! error a cancelled authorisation raises, and PowerShell throws where UAC was
//! refused. So there is nothing here that has to tell a dismissal from a
//! failure — both are the privilege not taken, which is the whole of what the
//! caller does anything about.

use std::process::{Command, Output, Stdio};

use verkstead_server::platform::Platform;
use verkstead_server::remote::{Elevate, Raised};
use verkstead_server::unseen::Unseen;

/// The platform's own password dialog, in front of one command.
#[derive(Debug, Clone, Copy)]
pub struct Graphical {
    /// Whose way of asking — see the module's own note on why this is a value.
    asks: Platform,
}

impl Graphical {
    /// The asking this machine does.
    pub fn here() -> Graphical {
        Graphical {
            asks: Platform::HERE,
        }
    }
}

impl Elevate for Graphical {
    /// Run `command` behind the platform's own asking, and wait for it.
    ///
    /// Waiting is the point: what is being waited on is a human reading a
    /// password dialog, and the answer is not knowable until they have. The
    /// server calls this on a thread that can be waited on.
    ///
    /// **Nothing is inherited.** A program started to ask for a password has no
    /// business reading this process's standard input, and both of the streams
    /// it writes are what the refusal is worded out of.
    fn raise(&self, command: &[String]) -> Raised {
        let asking = asking(self.asks, command);

        let (program, arguments) = asking
            .split_first()
            .expect("every arm builds a command with a program to run");

        // And nothing drawn for the asking itself: the dialog is the platform's,
        // and the program that raises it is a console program this app would
        // otherwise put a black window behind it — see
        // [`verkstead_server::unseen`].
        let told = Command::new(program)
            .args(arguments)
            .unseen()
            .stdin(Stdio::null())
            .output();

        match told {
            Ok(told) => ran(&told),

            // Which is a machine with no `pkexec` on it, or no `osascript`, or
            // no `powershell`: the privilege was not taken, and this is why.
            Err(trouble) => Raised::Refused {
                why: format!("{program} could not be run: {trouble}"),
            },
        }
    }
}

/// What the asking said, read off how it exited.
///
/// Zero is the command having run with the privilege. Everything else is the
/// privilege not taken, in whatever words there were — the first line the asking
/// printed on standard error, which is where all three of them put the sentence
/// naming what happened.
fn ran(told: &Output) -> Raised {
    if told.status.success() {
        return Raised::Done;
    }

    let why = String::from_utf8_lossy(&told.stderr)
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| format!("the asking exited with {}", told.status));

    Raised::Refused { why }
}

/// The command that runs `command` with this platform's own password dialog in
/// front of it.
fn asking(platform: Platform, command: &[String]) -> Vec<String> {
    match platform {
        Platform::Linux => pkexec(command),
        Platform::MacOs => osascript(command),
        Platform::Windows => runas(command),
    }
}

/// Linux: polkit's own, which is the desktop's password dialog wherever there is
/// an authentication agent running and a text prompt wherever there is not.
///
/// The command is passed as arguments rather than as a shell line, so nothing
/// about it is quoted or unquoted on the way through.
fn pkexec(command: &[String]) -> Vec<String> {
    std::iter::once("pkexec".to_owned())
        .chain(command.iter().cloned())
        .collect()
}

/// macOS: the AppleScript that asks, which is how everything on this platform
/// asks — the dialog is the system's own and says which application wants it.
///
/// `do shell script` takes a shell line rather than an argument list, so the
/// command is quoted for the shell and then quoted again for the AppleScript
/// string it sits inside. Both are done here rather than assumed away: the only
/// argument that ever crosses is a user name, and a user name is one of the
/// places an apostrophe turns up.
fn osascript(command: &[String]) -> Vec<String> {
    let line = applescript(&shell(command));

    vec![
        "osascript".to_owned(),
        "-e".to_owned(),
        format!("do shell script \"{line}\" with administrator privileges"),
    ]
}

/// Windows: `Start-Process -Verb RunAs`, which is what raises the UAC prompt —
/// the elevation this platform has, asked for by starting the program elevated
/// rather than by running something in front of it.
///
/// **What it exits with is the elevated process's own.** `Start-Process` reports
/// nothing by itself, so it is asked for the process it started (`-PassThru`),
/// waited on (`-Wait`) and exited with what that came to. And `$ErrorActionPreference`
/// is what makes a refused UAC prompt an error rather than a warning PowerShell
/// walks past — a cancelled dialog has to be a non-zero exit here, the way it is
/// on the other two.
fn runas(command: &[String]) -> Vec<String> {
    let (program, arguments) = command
        .split_first()
        .expect("a command to raise has a program to run");

    let mut script = format!(
        "$ErrorActionPreference = 'Stop'; $ran = Start-Process -FilePath {}",
        powershell(program)
    );

    // Omitted rather than empty, because `Start-Process` refuses an argument
    // list with nothing in it.
    if !arguments.is_empty() {
        let listed: Vec<String> = arguments
            .iter()
            .map(|argument| powershell(argument))
            .collect();

        script.push_str(&format!(" -ArgumentList {}", listed.join(",")));
    }

    script.push_str(" -Verb RunAs -Wait -PassThru; exit $ran.ExitCode");

    vec![
        "powershell".to_owned(),
        "-NoProfile".to_owned(),
        "-NonInteractive".to_owned(),
        "-Command".to_owned(),
        script,
    ]
}

/// `command` as one line a POSIX shell runs, every word of it quoted.
fn shell(command: &[String]) -> String {
    command
        .iter()
        .map(|word| format!("'{}'", word.replace('\'', r"'\''")))
        .collect::<Vec<String>>()
        .join(" ")
}

/// And `said` as it is written inside an AppleScript string: the two characters
/// that would end it or escape the next one.
fn applescript(said: &str) -> String {
    said.replace('\\', r"\\").replace('"', "\\\"")
}

/// And `said` as PowerShell's own literal string, where the one character that
/// ends it is written twice to mean itself.
fn powershell(said: &str) -> String {
    format!("'{}'", said.replace('\'', "''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The one command this is ever asked to raise: the operator grant, without
    /// the `sudo` the pane shows — what raises it is the asking each arm builds.
    fn grant() -> Vec<String> {
        vec![
            "tailscale".to_owned(),
            "set".to_owned(),
            "--operator=ada".to_owned(),
        ]
    }

    /// Linux hands polkit the command as it stands.
    #[test]
    fn the_linux_arm_runs_pkexec() {
        assert_eq!(
            asking(Platform::Linux, &grant()),
            ["pkexec", "tailscale", "set", "--operator=ada"],
        );
    }

    /// macOS hands the same command to the system's own authorisation, as the
    /// shell line `do shell script` takes.
    #[test]
    fn the_macos_arm_runs_osascript() {
        assert_eq!(
            asking(Platform::MacOs, &grant()),
            [
                "osascript",
                "-e",
                "do shell script \"'tailscale' 'set' '--operator=ada'\" \
                 with administrator privileges",
            ],
        );
    }

    /// And Windows starts the command elevated, exiting with what the elevated
    /// process exited with.
    #[test]
    fn the_windows_arm_starts_an_elevated_process() {
        assert_eq!(
            asking(Platform::Windows, &grant()),
            [
                "powershell",
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "$ErrorActionPreference = 'Stop'; \
                 $ran = Start-Process -FilePath 'tailscale' \
                 -ArgumentList 'set','--operator=ada' \
                 -Verb RunAs -Wait -PassThru; exit $ran.ExitCode",
            ],
        );
    }

    /// A user name with an apostrophe in it is a user name, and each of the two
    /// arms that builds a string rather than an argument list has to survive
    /// one: the grant names whoever this machine's user is.
    #[test]
    fn a_name_that_would_end_a_quoted_string_is_quoted() {
        let grant = vec![
            "tailscale".to_owned(),
            "set".to_owned(),
            "--operator=o'hara".to_owned(),
        ];

        assert_eq!(
            asking(Platform::MacOs, &grant)[2],
            "do shell script \"'tailscale' 'set' '--operator=o'\\\\''hara'\" \
             with administrator privileges",
        );

        assert_eq!(
            asking(Platform::Windows, &grant)[4],
            "$ErrorActionPreference = 'Stop'; \
             $ran = Start-Process -FilePath 'tailscale' \
             -ArgumentList 'set','--operator=o''hara' \
             -Verb RunAs -Wait -PassThru; exit $ran.ExitCode",
        );
    }

    /// A command with nothing but a program in it is started with no argument
    /// list at all, because `Start-Process` will not take an empty one.
    #[test]
    fn a_command_with_no_arguments_is_started_without_a_list() {
        let asking = asking(Platform::Windows, &["tailscale".to_owned()]);

        assert!(
            !asking[4].contains("-ArgumentList"),
            "an empty list is no list: {}",
            asking[4],
        );
    }

    /// Zero is the privilege taken, and everything else is it not taken — in
    /// whatever the asking said about it, which is what a dismissal is told from
    /// a failure by wherever that distinction is wanted.
    #[test]
    fn what_the_asking_exited_with_is_the_answer() {
        assert_eq!(ran(&exited(true, "")), Raised::Done);

        assert_eq!(
            ran(&exited(false, "\nError: (-128) User canceled.\n")),
            Raised::Refused {
                why: "Error: (-128) User canceled.".to_owned(),
            },
        );
    }

    /// And an asking that failed silently is still a refusal, said in the only
    /// words there are for it.
    #[test]
    fn an_asking_that_said_nothing_is_still_a_refusal() {
        let Raised::Refused { why } = ran(&exited(false, "")) else {
            panic!("a non-zero exit is the privilege not taken");
        };

        assert!(why.contains("exited"), "got: {why}");
    }

    /// What a run of the asking came to, as this module reads one.
    fn exited(worked: bool, stderr: &str) -> Output {
        Output {
            status: status(worked),
            stdout: Vec::new(),
            stderr: stderr.as_bytes().to_vec(),
        }
    }

    /// An exit status of each kind, made the one way a test can make one: by
    /// running something that exits that way.
    #[cfg(unix)]
    fn status(worked: bool) -> std::process::ExitStatus {
        use std::os::unix::process::ExitStatusExt;

        std::process::ExitStatus::from_raw(if worked { 0 } else { 1 << 8 })
    }

    /// And the same on Windows, where an exit status is the code itself.
    #[cfg(windows)]
    fn status(worked: bool) -> std::process::ExitStatus {
        use std::os::windows::process::ExitStatusExt;

        std::process::ExitStatus::from_raw(if worked { 0 } else { 1 })
    }
}
