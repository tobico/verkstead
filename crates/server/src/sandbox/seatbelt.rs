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
//! **What a mount would have made, this makes.** [`Access::Empty`] and
//! [`Access::Elsewhere`] are a directory out of nothing and a path being
//! somewhere it is not, and Apple's sandbox has neither to offer — so before a
//! policy is written at all, the directory is really made and the path is
//! really linked, under [`realise`]. A session's HOME is then a real directory
//! of Verkstead's own with the Profile's account symlinked into it, and what
//! keeps one account out of another's is the policy: the account this session
//! runs under is reachable and every other path on the machine is not.
//!
//! And [`Access::Nothing`] is the one the two mechanisms answer in opposite
//! directions. A mount hides the account's own skills by standing an empty
//! directory on them; there is nothing to stand anywhere here, and a link
//! written at that path would be written *into* the account itself — so it is
//! the path being kept out of the rule that grants the account, in that rule's
//! own words. See [`reaching`] and [`refusing`].
//!
//! **Not a `deny` written after it, which is what this used to be.** A mount
//! table takes the last bind that landed and a policy does not read the same
//! way: a probe inside a real sandbox found the account's own skills writable
//! while the policy that made them so said, in order, that they were refused.
//! So the exclusion is `require-not` inside the one rule, where no reading of
//! the order can come out differently, and the outright `deny` stays beside it
//! as the statement of intent rather than as the mechanism.

use std::path::{Path, PathBuf};

use super::rendering::Rendering;
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
/// - **The root directory itself**, which is the one rule here that reads as
///   nothing and is the whole of whether a policy runs at all. A description is
///   rendered as `subpath` rules, and a `subpath` of `/usr` — or of every
///   top-level directory on the machine at once — never matches `/`. A process
///   started under a policy that does not match it dies in `dyld` on `SIGABRT`
///   before `main`, with nothing on either stream: there is no stderr to
///   complain to yet. Which is exactly what it cost to find, so it is written
///   down here rather than left to be found again. `literal` rather than
///   `subpath`, because what is wanted is the directory node and not the
///   machine behind it — and it is no widening, `file-read-metadata` above
///   having already made every name on the box `stat`-able.
const FLOOR: &str = r#"(version 1)

(deny default)

(allow file-read-metadata)
(allow file-read* (literal "/"))

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
pub(crate) fn command(surface: &Surface) -> Rendering {
    realise(surface);

    let mut sandbox = Rendering::running(SANDBOX_EXEC);

    // Nothing of the server's environment, for the reason the Linux rendering
    // says none of it either: what a session has is the description's to say,
    // and a [`Rendering`] holds the whole of what the process is handed.
    for (key, value) in surface.env() {
        sandbox.set(key, value);
    }

    sandbox.starting_in(surface.chdir());
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

    // What the description says a session must not reach, resolved once and
    // read by every rule below — see [`refusing`]. A rule that would grant
    // something one of these sits under has to say so in the same breath rather
    // than leave a later `deny` to take it back.
    let refused: Vec<PathBuf> = surface
        .reaches()
        .iter()
        .filter_map(|access| match access {
            Access::Nothing { inside, .. } => Some(real(inside)),
            _ => None,
        })
        .collect();

    for access in surface.reaches() {
        match access {
            Access::Own { path, reach } => policy.push_str(&reaching(path, *reach, &refused)),

            // Somewhere to write a temporary file. The host's own on this
            // platform rather than a filesystem of the session's, which is
            // where a tmpfs and a policy part company: what a session writes
            // there is visible to whoever else is on the machine, and what they
            // left there is visible to it.
            Access::Temporary(path) => policy.push_str(&reaching(path, Reach::ReadWrite, &refused)),

            Access::Devices => policy.push_str(DEVICES),

            // A directory that is really there and really empty by the time
            // this is written — see [`realise`] — so what is left to say about
            // it is that a session may read and write it, which is what a HOME
            // is for.
            Access::Empty(path) => policy.push_str(&reaching(path, Reach::ReadWrite, &refused)),

            // And a path a session finds somewhere else, which by now is a link
            // to the path it really is. A policy is matched against what a name
            // resolves to, so what is said about it is said about the host's
            // path — and the directory the link is *in* is said too, and by
            // itself: a session looks in the directory its own binary is in,
            // and nothing under that directory is reachable for its being
            // listed.
            Access::Elsewhere {
                host,
                inside,
                reach,
            } => {
                if let Some(holding) = inside.parent() {
                    policy.push_str(&format!(
                        "\n(allow file-read* (literal {}))\n",
                        quoted(&real(holding))
                    ));
                }

                policy.push_str(&reaching(host, *reach, &refused));
            }

            // And what a mount would have covered, refused instead — said
            // twice, and neither saying is the other's spelling.
            //
            // **The rule that grants the account excludes this path in the same
            // breath**, which is [`reaching`]'s doing and is what actually
            // holds: a `deny` written after an `allow` the path sits under does
            // not take it back, and a probe inside a real sandbox is what said
            // so — the account's own skills came back writable while a policy
            // that read in order said they could not be.
            //
            // **And it is denied outright as well**, which costs nothing and is
            // what says the intention rather than the arithmetic: a path
            // reached by some route nobody thought of is still one a session
            // must not have.
            Access::Nothing { inside, .. } => policy.push_str(&format!(
                "\n(deny file* (subpath {}))\n",
                quoted(&real(inside))
            )),

            // There is no `/proc` on a Mac, so the process table is not a path
            // and nothing here is about one — what a session can learn about
            // the processes around it is the floor's `process-info*`.
            Access::ProcessTable => {}
        }
    }

    policy
}

/// Make what a mount would otherwise have made: the empty directories, and the
/// links to the paths a session finds somewhere else.
///
/// In the order the description says them, because the order *is* the
/// description — a session's HOME is emptied before the account is linked into
/// it, and a link written before the directory holding it exists is a link
/// nothing can be written at.
///
/// **Failures are logged rather than raised.** What one costs is a session that
/// cannot reach the thing that could not be made — its account, its handoff
/// directory — which fails at the far end saying so; and a [`Command`] is not a
/// thing that can refuse. What is written here is the server's own to write:
/// the directories are Verkstead's, under the Data Directory, and the links are
/// inside them.
fn realise(surface: &Surface) {
    for access in surface.reaches() {
        let made = match access {
            Access::Empty(path) => super::emptied(path),
            Access::Elsewhere { host, inside, .. } => linked(host, inside),
            _ => Ok(()),
        };

        if let Err(error) = made {
            tracing::error!(
                error = ?error,
                access = ?access,
                "what a session was to find could not be made, so it will not find it"
            );
        }
    }
}

/// And a path a session finds somewhere else: a link at `inside` to `host`.
///
/// Whatever is at `inside` already goes first. It is the link a session before
/// this one was given — a Conversation's handoff directory is reached at one
/// path by every Conversation there is — and a link left pointing at somebody
/// else's directory is worse than no link at all.
fn linked(host: &Path, inside: &Path) -> std::io::Result<()> {
    if let Some(holding) = inside.parent() {
        std::fs::create_dir_all(holding)?;
    }

    // By what it is rather than by following it: a link to a directory is a
    // link, and removing what it points at is not what this is for.
    match std::fs::symlink_metadata(inside) {
        Ok(there) if there.is_dir() => std::fs::remove_dir_all(inside)?,
        Ok(_) => std::fs::remove_file(inside)?,
        Err(_) => {}
    }

    std::os::unix::fs::symlink(host, inside)
}

/// The rules that make one path reachable: readable and runnable, and writable
/// where the description said so — less whatever of `refused` sits under it.
///
/// Runnable with readable, rather than as a decision of its own. Every path in
/// a description is either the system a session runs programs out of or a
/// directory of the project's, and a checkout a session may read is one it may
/// build and run — which is what a coding session is for.
///
/// **What is refused is excluded here rather than denied afterwards.** The
/// account's own skills sit inside the account, so the rule that grants the
/// account is the rule that would otherwise grant them — and a `deny` written
/// after it does not take them back, which a probe inside a real sandbox is
/// what settled: the skills came back writable while a policy that read in
/// order said they could not be. `require-not` says it in the one rule instead,
/// where no reading of the order can come out differently. See [`refusing`].
fn reaching(path: &Path, reach: Reach, refused: &[PathBuf]) -> String {
    let matched = refusing(&real(path), refused);

    let mut rules = format!("\n(allow file-read* file-map-executable process-exec* {matched})\n");

    if reach == Reach::ReadWrite {
        rules.push_str(&format!("(allow file-write* {matched})\n"));
    }

    rules
}

/// `path` as the filter a rule about it matches on: the subpath itself, and
/// where anything in `refused` sits under it, that subpath with each of them
/// taken out of it.
///
/// A path in `refused` that is `path` itself is left alone — a rule granting
/// exactly what another one refuses is the description contradicting itself
/// rather than something to render, and the outright `deny` is what answers it.
fn refusing(path: &Path, refused: &[PathBuf]) -> String {
    let subpath = format!("(subpath {})", quoted(path));

    let under: String = refused
        .iter()
        .filter(|no| no.as_path() != path && no.starts_with(path))
        .map(|no| format!(" (require-not (subpath {}))", quoted(no)))
        .collect();

    if under.is_empty() {
        return subpath;
    }

    format!("(require-all {subpath}{under})")
}

/// What `path` really is: resolved whole where it is there, and resolved as far
/// as it goes with the rest of the name on the end where it is not.
///
/// The second is what a rule about a path that is not there wants, and there is
/// one: `~/.claude` inside is a link to the Profile's account, and what refuses
/// the account's own skills has to name them *under the account* whether that
/// account happens to keep any or not. A name resolved no further than itself
/// would be a rule about a path nothing will ever be checked against.
fn real(path: &Path) -> PathBuf {
    if let Ok(resolved) = std::fs::canonicalize(path) {
        return resolved;
    }

    match (path.parent(), path.file_name()) {
        (Some(holding), Some(name)) => real(holding).join(name),
        _ => path.to_owned(),
    }
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

    /// What only a mount could otherwise have made, made: a real HOME with the
    /// account really linked into it, and a policy about where those are.
    #[test]
    fn a_home_is_really_made_and_the_account_is_really_linked_into_it() {
        let dir = tempfile::tempdir().unwrap();
        let account = dir.path().join("account/.claude");
        let home = dir.path().join("home");
        std::fs::create_dir_all(&account).unwrap();

        // Something the session before this one left behind, which is what a
        // fresh HOME means on a platform where the directory is really there.
        std::fs::create_dir_all(home.join("the-last-session")).unwrap();

        let mut surface = Surface::starting_in(dir.path().to_owned());
        surface.made(Access::Empty(home.clone())).elsewhere(
            &account,
            home.join(".claude"),
            Reach::ReadWrite,
        );

        realise(&surface);

        assert!(
            !home.join("the-last-session").exists(),
            "a HOME is the session's own, and what the last one left is not in it",
        );
        assert_eq!(
            std::fs::read_link(home.join(".claude")).unwrap(),
            account,
            "and the Profile's account is where its backend looks for it",
        );

        let policy = policy(&surface);

        assert!(
            policy.contains(&format!(
                "(allow file-write* (subpath {}))",
                quoted(&real(&home))
            )),
            "HOME is the session's to write:\n{policy}",
        );
        assert!(
            policy.contains(&format!(
                "(allow file-write* (subpath {}))",
                quoted(&real(&account))
            )),
            "and so is the account it is logged in as, at the path it really \
             is — which is what a policy matches against:\n{policy}",
        );
    }

    /// A link left by whichever session was here before is this session's to
    /// replace: one Conversation's handoff directory is reached at the path
    /// every other Conversation's is.
    #[test]
    fn a_link_left_by_the_session_before_is_the_one_this_session_needs() {
        let dir = tempfile::tempdir().unwrap();
        let theirs = dir.path().join("somebody-elses");
        let ours = dir.path().join("ours");
        let inside = dir.path().join("inside/verkstead");
        std::fs::create_dir_all(&theirs).unwrap();
        std::fs::create_dir_all(&ours).unwrap();
        std::fs::create_dir_all(inside.parent().unwrap()).unwrap();
        std::os::unix::fs::symlink(&theirs, &inside).unwrap();

        let mut surface = Surface::starting_in(dir.path().to_owned());
        surface.elsewhere(&ours, &inside, Reach::ReadWrite);

        realise(&surface);

        assert_eq!(std::fs::read_link(&inside).unwrap(), ours);
        assert!(
            theirs.is_dir(),
            "and what it pointed at is somebody else's directory, not this one's to remove",
        );
    }

    /// And what a mount would have covered is refused instead — under the
    /// account, which is where a session would really find it, and after the
    /// rule that made the account reachable.
    #[test]
    fn the_accounts_own_skills_are_refused_where_they_really_are() {
        let dir = tempfile::tempdir().unwrap();
        let account = dir.path().join("account/.claude");
        let home = dir.path().join("home");
        std::fs::create_dir_all(account.join("skills/the-accounts-own")).unwrap();

        let mut surface = Surface::starting_in(dir.path().to_owned());
        surface
            .made(Access::Empty(home.clone()))
            .elsewhere(&account, home.join(".claude"), Reach::ReadWrite)
            .nothing(home.join(".claude/skills"), dir.path().join("nothing"));

        realise(&surface);

        let policy = policy(&surface);
        let skills = quoted(&real(&account.join("skills")));

        assert!(
            policy.contains(&format!("(deny file* (subpath {skills}))")),
            "what a session is grilled by is the product's, not whatever the \
             account keeps:\n{policy}",
        );

        // The one that actually holds it: the rule granting the account says in
        // its own words that this path is not part of what it grants, so
        // nothing about the order of the two decides it — see the module's own
        // documentation for the probe that settled that it has to.
        for granted in ["file-read*", "file-write*"] {
            assert!(
                policy.contains(&format!(
                    "(allow {granted}{} (require-all (subpath {}) (require-not (subpath {skills})))",
                    if granted == "file-read*" {
                        " file-map-executable process-exec*"
                    } else {
                        ""
                    },
                    quoted(&real(&account)),
                )),
                "the account is granted with its own skills taken out of what \
                 is granted, rather than added back and denied again:\n{policy}",
            );
        }

        assert!(
            !policy.contains(&format!(
                "(allow file-write* (subpath {}))",
                quoted(&real(&account))
            )),
            "and there is no rule anywhere granting the account whole:\n{policy}",
        );
        assert!(
            account.join("skills/the-accounts-own").is_dir(),
            "nothing of the account's own is written over to do it",
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

        assert_eq!(command.program(), SANDBOX_EXEC);
        assert_eq!(command.chdir(), Some(dir.path()));

        let argv = command.argv();
        assert_eq!(
            argv[argv.len() - 3..],
            ["/bin/sh", "-c", "true"],
            "what a session runs comes after the policy, whole:\n{argv:?}",
        );

        assert_eq!(
            command.env(),
            [(
                std::ffi::OsString::from("HOME"),
                std::ffi::OsString::from("/nowhere")
            )],
            "and its environment is the description's rather than the server's",
        );
    }
}
