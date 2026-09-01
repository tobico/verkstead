//! The macOS rendering: a [`Surface`] as a deny-by-default sandbox policy, and
//! the `sandbox-exec` that starts a command under it.
//!
//! Seatbelt is what Apple's sandbox is called by the people who work on it, and
//! this is the whole of what Verkstead can reach of it: `sandbox-exec` takes a
//! policy and execs a command inside it. The supported way to sandbox a Mac
//! program is an entitlement on a signed bundle, applied to the app itself
//! rather than to a child it spawns — which is no use to an app that sandboxes
//! sessions rather than itself, and no use at all to an unsigned one
//! (ADR-0012). So this is a deprecated command with no replacement, used with
//! open eyes.
//!
//! **The boundary refuses where bubblewrap's hides.** There are no mounts here:
//! every path a session reaches is the path it really is, and everything else
//! is denied rather than absent. A session on a Mac can therefore see that the
//! machine has a home directory full of somebody's work, and cannot read a byte
//! of it — where the same session on Linux is in a namespace the directory was
//! never in. The metadata is deliberately left readable, because that is what
//! the machine looks like from inside a policy and pretending otherwise would
//! take a rule per path for nothing gained.
//!
//! **What this does not render yet.** [`Access::Elsewhere`] and
//! [`Access::Empty`] are a path being somewhere it is not, and Apple's sandbox
//! has no such thing: HOME, the Profile's account, the skills and the binary a
//! session asks with have to be made real on this platform, which is the next
//! task of this stage rather than this one. Until it lands, what those describe
//! is left out of the policy — which is a session that cannot reach its own
//! account, rather than one that can reach anything more. Everything skipped
//! here narrows the sandbox; nothing skipped widens it.

use std::path::{Path, PathBuf};
use std::process::Command;

use super::surface::{Access, Reach, Surface};

/// The command that applies a policy and execs what comes after it.
///
/// By its whole path rather than by name: what the server's own `PATH` holds is
/// a fact about however the app was launched, and the one thing a session's
/// boundary should not be resolved through.
const SANDBOX_EXEC: &str = "/usr/bin/sandbox-exec";

/// What every policy starts with: nothing at all, and then the handful of
/// things that are true of a session whatever it may reach.
///
/// - **Metadata everywhere**, which is what this boundary refusing rather than
///   hiding comes to — and what makes a denied path answer `stat` and then
///   refuse to open, which is what the probes read.
/// - **Forking, signalling and posix IPC inside the sandbox**, because a
///   session is a shell that runs programs, and a program that cannot fork is
///   not one.
/// - **`sysctl` reads**, which is how a program on a Mac asks how many cores
///   the machine has.
/// - **The network, whole and unfiltered**, exactly as `--share-net` leaves it
///   on Linux and for the reason the sandbox module gives: an agent has to
///   reach GitHub, a registry and a model's API, and an allowlist that has to
///   hold all of that is one nobody will keep honest.
/// - **Mach lookups**, unrestricted. Everything on a Mac that resolves a name,
///   finds a font or asks who the user is goes through one, and the narrow
///   allowlist that would cover a coding session is a list nobody can write
///   without watching a real one fail. What it costs is a session that can talk
///   to the machine's own services — which is a narrowing worth making once
///   there is a probe to say what a session actually needs.
/// - **The devices**, which are named rather than a directory: `/dev` on a Mac
///   is the machine's own and holds every disk on it.
const FLOOR: &str = r#"(version 1)

(deny default)

(allow file-read-metadata)

(allow process-fork)
(allow process-info* (target self))
(allow sysctl-read)
(allow ipc-posix*)
(allow signal (target same-sandbox))

(allow system-socket)
(allow network*)
(allow mach-lookup)
"#;

/// And what [`Access::Devices`] renders as: the nodes a program opens by name,
/// and the pseudo-terminal a session is on.
///
/// `/dev/tty` and the `ttys` numbers are the session's own terminal — Verkstead
/// opens the pair and hands one end over, and a program that reads the width of
/// its terminal opens it again by name.
const DEVICES: &str = r#"
(allow file*
       (literal "/dev/null")
       (literal "/dev/zero")
       (literal "/dev/random")
       (literal "/dev/urandom")
       (literal "/dev/tty")
       (literal "/dev/dtracehelper")
       (literal "/dev/ptmx")
       (subpath "/dev/fd")
       (regex #"^/dev/ttys[0-9]+"))
"#;

/// `surface` as the `sandbox-exec` invocation that runs it under its own
/// policy.
pub(crate) fn command(surface: &Surface) -> Command {
    let mut sandbox = Command::new(SANDBOX_EXEC);

    // Nothing of the server's environment, for the reason the Linux rendering
    // clears it: what a session has is the description's to say.
    sandbox.env_clear();

    for (key, value) in surface.env() {
        sandbox.env(key, value);
    }

    sandbox.current_dir(surface.chdir());
    sandbox.arg("-p").arg(policy(surface));
    sandbox.args(surface.argv());

    sandbox
}

/// The policy `surface` describes, as `sandbox-exec` reads one.
///
/// Every path is resolved to what it really is first. A policy is matched
/// against the resolved path, and a Mac is made of symlinks that matter —
/// `/tmp` is `/private/tmp`, `/etc` is `/private/etc`, and a temporary
/// directory is under `/private/var/folders` however it was handed over — so a
/// rule written about the name would hold about nothing. One that resolves to
/// nothing is written as it stands, which is a rule about a path that is not
/// there.
fn policy(surface: &Surface) -> String {
    let mut policy = String::from(FLOOR);

    for access in surface.reaches() {
        match access {
            Access::Own { path, reach } => policy.push_str(&reaching(path, *reach)),

            // Somewhere to write a temporary file. The host's own on this
            // platform rather than a filesystem of the session's, which is
            // where a tmpfs and a policy part company: what a session writes
            // there is visible to whoever else is on the machine, and what they
            // left there is visible to it.
            Access::Temporary(path) => policy.push_str(&reaching(path, Reach::ReadWrite)),

            Access::Devices => policy.push_str(DEVICES),

            // There is no `/proc` on a Mac, so the process table is not a path
            // and nothing here is about one — what a session can learn about
            // the processes around it is the floor's `process-info*`.
            Access::ProcessTable => {}

            // And the two that want a real path on this platform, which is the
            // next task's — see this module's own documentation.
            Access::Elsewhere { .. } | Access::Empty(_) => {}
        }
    }

    policy
}

/// The rules that make one path reachable: readable and runnable, and writable
/// where the description said so.
///
/// Runnable with readable, rather than as a decision of its own. Every path in
/// a description is either the system a session runs programs out of or a
/// directory of the project's, and a checkout a session may read is one it may
/// build and run — which is what a coding session is for.
fn reaching(path: &Path, reach: Reach) -> String {
    let path = quoted(&real(path));

    let mut rules =
        format!("\n(allow file-read* file-map-executable process-exec* (subpath {path}))\n");

    if reach == Reach::ReadWrite {
        rules.push_str(&format!("(allow file-write* (subpath {path}))\n"));
    }

    rules
}

/// What `path` really is, or `path` itself where it resolves to nothing.
fn real(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_owned())
}

/// `path` as a policy's own string literal.
///
/// A path is somebody's directory name and a policy is a program, so the two
/// characters that would end the string or escape the next one are escaped
/// here. A path that is not valid UTF-8 is written as the lossy reading of it,
/// which is a rule about a path that does not exist — a policy has no other
/// spelling to offer, and the alternative is a session started with the rule
/// silently missing.
fn quoted(path: &Path) -> String {
    let escaped = path
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('"', "\\\"");

    format!("\"{escaped}\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A surface with one writable directory and one readable one, which is the
    /// floor a session stands on.
    fn surface(writable: &Path, readable: &Path) -> Surface {
        let mut surface = Surface::starting_in(writable.to_owned());

        surface
            .own(readable, Reach::ReadOnly)
            .own(writable, Reach::ReadWrite)
            .made(Access::Devices)
            .set("HOME", "/nowhere")
            .running(&["/bin/sh", "-c", "true"]);

        surface
    }

    #[test]
    fn nothing_is_reachable_before_the_description_says_so() {
        let policy = policy(&Surface::starting_in(PathBuf::from("/")));

        assert!(
            policy.starts_with("(version 1)\n\n(deny default)"),
            "the first thing a policy says is that nothing is allowed, and \
             every rule after it is an exception to that:\n{policy}",
        );
    }

    #[test]
    fn a_writable_path_is_readable_and_a_readable_one_is_not_writable() {
        let dir = tempfile::tempdir().unwrap();
        let writable = dir.path().join("worktree");
        let readable = dir.path().join("system");
        std::fs::create_dir_all(&writable).unwrap();
        std::fs::create_dir_all(&readable).unwrap();

        let policy = policy(&surface(&writable, &readable));
        let (writable, readable) = (quoted(&real(&writable)), quoted(&real(&readable)));

        assert!(
            policy.contains(&format!("(allow file-write* (subpath {writable}))")),
            "the Worktree is what a session writes:\n{policy}",
        );
        assert!(
            policy.contains(&format!(
                "(allow file-read* file-map-executable process-exec* (subpath {readable}))"
            )),
            "and the system is what it reads and runs:\n{policy}",
        );
        assert!(
            !policy.contains(&format!("(allow file-write* (subpath {readable}))")),
            "read-only is the whole of what read-only means:\n{policy}",
        );
    }

    #[test]
    fn a_path_is_written_as_the_one_it_resolves_to() {
        let dir = tempfile::tempdir().unwrap();
        let real_dir = dir.path().join("worktree");
        let through_a_link = dir.path().join("link");
        std::fs::create_dir_all(&real_dir).unwrap();
        std::os::unix::fs::symlink(&real_dir, &through_a_link).unwrap();

        let mut surface = Surface::starting_in(through_a_link.clone());
        surface.own(&through_a_link, Reach::ReadWrite);

        let policy = policy(&surface);

        assert!(
            policy.contains(&quoted(&std::fs::canonicalize(&real_dir).unwrap())),
            "a policy is matched against the path a name resolves to, and \
             `/tmp` on a Mac is `/private/tmp`:\n{policy}",
        );
        assert!(
            !policy.contains(&quoted(&through_a_link)),
            "so the name itself is not what the rule is about:\n{policy}",
        );
    }

    /// The two the next task makes real — see this module's own documentation.
    /// Nothing about them is in the policy, and what that costs is a session
    /// that cannot reach its account rather than one that can reach more.
    #[test]
    fn what_only_a_mount_could_do_is_left_out_rather_than_guessed_at() {
        let dir = tempfile::tempdir().unwrap();
        let account = dir.path().join("account");
        let home = dir.path().join("home");
        std::fs::create_dir_all(&account).unwrap();

        let mut surface = Surface::starting_in(dir.path().to_owned());
        surface.made(Access::Empty(home.clone())).elsewhere(
            &account,
            home.join(".claude"),
            Reach::ReadWrite,
        );

        let policy = policy(&surface);

        assert!(
            !policy.contains(&account.to_string_lossy().to_string())
                && !policy.contains(&home.to_string_lossy().to_string()),
            "neither the account nor the HOME it would be mounted into is a \
             path this platform can reach yet:\n{policy}",
        );
    }

    #[test]
    fn a_directory_name_cannot_write_a_rule_of_its_own() {
        assert_eq!(
            quoted(Path::new(r#"/Users/you/a "quoted" \ directory"#)),
            r#""/Users/you/a \"quoted\" \\ directory""#,
            "a path is somebody's directory name and a policy is a program",
        );
    }

    #[test]
    fn the_command_starts_in_the_worktree_with_the_environment_it_was_given() {
        let dir = tempfile::tempdir().unwrap();
        let command = command(&surface(dir.path(), dir.path()));

        assert_eq!(command.get_program(), SANDBOX_EXEC);
        assert_eq!(command.get_current_dir(), Some(dir.path()));

        let argv: Vec<_> = command.get_args().collect();
        assert_eq!(
            argv[argv.len() - 3..],
            ["/bin/sh", "-c", "true"],
            "what a session runs comes after the policy, whole:\n{argv:?}",
        );

        let env: Vec<_> = command.get_envs().collect();
        assert_eq!(
            env,
            [(
                std::ffi::OsStr::new("HOME"),
                Some(std::ffi::OsStr::new("/nowhere"))
            )],
            "and its environment is the description's rather than the server's",
        );
    }
}
