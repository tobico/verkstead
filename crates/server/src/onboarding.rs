//! Whether this Verkstead can do anything yet: the objective, the probes it is
//! answered by, and the mode a server that cannot enters at startup.
//!
//! **The objective is three things** (ADR-0016): a sandbox, `git` and at least
//! one of the four harnesses present; at least one Agent Profile; a git author.
//! The GitHub token is not among them — GitHub may not be in use at all, while
//! git is not optional and its author is what git asks for.
//!
//! **The verdict is reached once, at startup**, and what it settles is the
//! mode: a server short of the objective is a server whose every URL is the
//! wizard until the wizard finishes. It never re-enters inside a run. Deleting
//! the last Profile mid-run leaves the mode where it was, and the step beside
//! it reading unmet — which is the decision rather than a gap in it: a page
//! that came back over work somebody has is what a live predicate would give
//! them, and the settings page's own empty state is where a Profile that has
//! gone is said. See [`Mode`].
//!
//! **Everything else is probed on every read.** The probes are a `PATH` walked,
//! one `bwrap` run, a home looked in and a settings file read, and probing on
//! read is what leaves nothing running while nobody is looking: the wizard
//! re-reads while a step is unmet, so an install that lands is ticked within ten
//! seconds of landing, an account that appears is offered as quickly, and a
//! closed workbench asks the machine nothing at all.
//!
//! **Present means a session would find it.** A session resolves its binaries
//! on the `PATH` inside the Sandbox, so every probe here resolves on that same
//! list — [`crate::sandbox::machine_path`], walked by that module's own lookup.
//! That list is composed out of the `PATH` the server itself was started with,
//! read once at startup and shared with the sandbox builder, so a row and a
//! session cannot come to disagree: a harness found somewhere no session looks
//! would be a row that ticked and a session that could not start, which is
//! exactly the failure the wizard exists to move forward in time. The names are
//! [`crate::sandbox::PROGRAMS`]'s, which is the harness names
//! [`crate::sessions::binary`] gives with `git` and `gh` beside them: what a row
//! is about is a program a session is launched as or reaches for, and every one
//! of them is followed into its install by the sandbox and by the probe alike —
//! see [`crate::sandbox::opened`]. So a name that is a link into somewhere no
//! session can reach reads absent here, a row promising a session that starts
//! rather than a file that happens to be on a list.
//!
//! **A row says where, and not only whether.** Which `claude` a session found
//! is the whole of what this is for: a distribution's, too old to connect, and
//! the current one under `~/.local/bin` are the same tick and two different
//! programs, so a row that is there carries the path it resolved to and the
//! file that path lands on. And a row that is *not* there says where the name
//! was seen, where it was seen at all — on an entry of the server's own `PATH`
//! that the composing dropped, or at the end of a link a session could not
//! follow. Both of those are a `PATH` to fix rather than a program to install.
//! See [`crate::sandbox::standing`], which decides all four, and
//! [`Machine::looks_in`], which is the list a session searches put in front of
//! whoever is reading the row.
//!
//! **The accounts are found in the server's own home**, which is where an
//! agent that has been logged into once wrote one. What a shape is made of is
//! [`crate::sandbox::account_in_home`]'s — the same list a session's account is
//! mounted from — so a home holds at most one account per harness, and each is
//! offered as the Profile the wizard would save it as. Which home that is, is
//! the platform's: `$HOME` on the two Unixes and `%USERPROFILE%` on Windows,
//! read the one way [`crate::platform::home_dir`] reads it.
//!
//! **The sandbox row is a run rather than a lookup** on Linux, because a
//! `bwrap` that is installed is not yet a `bwrap` that works: unprivileged user
//! namespaces can be switched off, and an AppImage cannot carry one. So the row
//! runs the most trivial sandbox there is and keeps what the failure said. On
//! macOS `sandbox-exec` is on every Mac and the row ticks; on Windows a
//! session's boundary is an identity rather than something to install, and the
//! row is not applicable at all.
//!
//! **The git step's prefills are a read apart.** What `git config --global`
//! says the machine commits as, and whatever GitHub token it is already
//! holding, are asked for by the step that has those fields rather than
//! carried on every reading — see [`Onboarding::prefill`]. Two processes and
//! an environment read are not something to run every ten seconds while
//! somebody waits for an install, and a token is not something to hand a page
//! that is drawing a sidebar.
//!
//! **And one thing here is done rather than read**: installing what the first
//! step is missing. The human ticks the absent rows they want and presses Next
//! once, and what that starts is a sequence of commands raised through the
//! [`crate::remote::Elevate`] handle the desktop app hands the server — see
//! [`install`], which is the whole of the run. It is still nothing written
//! down: a run belongs to the life of this server, and the rows under it go on
//! being probed at every read, which is what says a row that was installing has
//! landed.
//!
//! **All of it follows [`crate::platform`]'s discipline**: the platform is a
//! value rather than a `cfg`, and the machine is a set of values read at the
//! edge and passed down — see [`Machine`]. That is what leaves every arm,
//! including the two this runner will never be, a unit test on this one.

use std::borrow::Cow;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::Result;
use sqlx::SqlitePool;
use tokio::sync::OnceCell;
use verkstead_render::{
    AccountView, Dependency, DependencyState, DependencyView, Distro, InstallState, OnboardingView,
    PrefillView, Prefilled, RunView, Seen, Source, StepsView,
};

use crate::github::Gh;
use crate::platform::{Environment, Platform};
use crate::remote::Elevate;
use crate::settings::Settings;
use crate::unseen::Unseen;
use crate::{github, profiles, sandbox, sessions, store};

mod install;

pub(crate) use install::Refusal;

/// Where a Linux machine says which distribution it is.
///
/// The one file, rather than the several a distribution may also have: it is
/// what systemd standardised and what every distribution the wizard writes a
/// command for ships.
const OS_RELEASE: &str = "/etc/os-release";

/// What the sandbox row runs on Linux: a mount namespace holding the machine
/// read-only, running the one program every machine really has.
///
/// The smallest thing that is a sandbox. What it proves is not that this
/// argument vector works but that `bwrap` can make a namespace at all, which is
/// the thing a machine with unprivileged user namespaces switched off cannot
/// do.
///
/// **`/bin/sh` because it is the one path a Linux is obliged to have.** POSIX
/// names it and every distribution honours it — NixOS keeps `sh` there and
/// nothing else, which is exactly the case a `/bin/true` got wrong: a machine
/// with a working `bwrap` answered `execvp /bin/true: No such file or
/// directory`, and the row it drew held the wizard on a dependency the human
/// had already installed. `-c :` is the shell's own do-nothing, so what is run
/// inside the namespace is still nothing at all.
const TRIVIALLY: &[&str] = &["--ro-bind", "/", "/", SHELL, "-c", ":"];

/// The program run inside it, which is the half of that vector that is a claim
/// about the machine rather than about `bwrap`.
///
/// Its own constant so that the claim can be asked of a real machine — see
/// `what_the_sandbox_row_runs_inside_is_a_program_this_machine_has`, which is
/// the test a stub `bwrap` cannot be.
const SHELL: &str = "/bin/sh";

/// The program the Linux sandbox row is about.
const BWRAP: &str = "bwrap";

/// What a machine calls itself when it will not say, and what a stated one is
/// called until a test says otherwise — see [`Machine::called`].
///
/// The sentence it goes in is *waiting for the password dialog on …*, so what
/// stands in for a name is the thing that sentence is pointing at.
const NAMELESS: &str = "this machine";

/// And what a stated machine is called, which is a name nobody's box has: what
/// a test asserts about is what the server made of what it was told, and a
/// hostname read off the box the suite is on would be a golden fixture nobody
/// could commit.
const STATED: &str = "a-machine";

/// What this box calls itself, or [`NAMELESS`] where it will not say.
///
/// Read at the edge with everything else about the machine — see
/// [`Machine::here`] — and never again: a hostname is a fact about the box
/// rather than about a request, and the one sentence that needs it is written
/// while somebody is waiting for a dialog.
fn hostname() -> String {
    hostname::get()
        .ok()
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| NAMELESS.to_owned())
}

/// Which harness row is which agent's, so that the name each is probed under is
/// the program a session of that type is launched as.
///
/// The names themselves are the sandbox module's — [`sandbox::PROGRAMS`], which
/// holds these four with the `git` and `gh` a row is drawn for beside them —
/// because what is done with a name is that module's: every name here is
/// followed into its install and granted where that install is the human's own.
/// What holds the two lists together is
/// `every_name_the_wizard_has_a_row_for_is_a_name_a_session_is_followed_into`.
const HARNESSES: &[(Dependency, store::AgentType)] = &[
    (Dependency::Claude, store::AgentType::Claude),
    (Dependency::Codex, store::AgentType::Codex),
    (Dependency::Grok, store::AgentType::Grok),
    (Dependency::OpenCode, store::AgentType::OpenCode),
];

/// The machine the probes are made against: whose rules a name is read by,
/// where a session would look for one, and what this box calls itself.
///
/// A value rather than a set of reads, for the reason [`crate::platform`] holds
/// one: the environment is read once, at the edge, and every arm below is then
/// a function of what was read. That is what lets the Windows lookup and each
/// distribution's mapping be asked about on the Linux runner, without a test
/// mutating a process environment its neighbours are sharing.
#[derive(Debug, Clone)]
pub struct Machine {
    /// Whose rules a name is read by, and which sandbox row this is.
    platform: Platform,

    /// The `PATH` a session resolves its binaries on — the machine's own half
    /// of it, which is the half a human installs anything into.
    ///
    /// **`None` on the machine this server is running on**, where it is
    /// composed afresh at every read instead: an install that landed inside
    /// this run has grown `session_path` — see [`sandbox::installed_into`] —
    /// and a value held here since startup would be the wizard going on saying
    /// a row is absent after the install that filled it. A stated machine is a
    /// test's own word and holds whatever it was told.
    path: Option<OsString>,

    /// And the `PATH` the server itself was started with, which the one above
    /// was composed out of.
    ///
    /// What it answers is *where was it then*: a `claude` in a directory the
    /// composing dropped is a program on this machine that no session can open,
    /// and a row that said only *absent* would send somebody to install what
    /// they already have. See [`sandbox::standing`], which is where the two
    /// lists are read against each other, and [`Seen`], which is what a row
    /// carries about it.
    servers: OsString,

    /// And `%PATHEXT%`, which is what says a name is a program on the one
    /// platform where a bare one is not. Nothing on the two Unixes reads it.
    pathext: Option<OsString>,

    /// What `/etc/os-release` said, where the machine keeps one — the whole
    /// file, because which line answers is [`distro`]'s business rather than
    /// the reading's.
    os_release: Option<String>,

    /// And the home of whoever is running this server, which is where the
    /// accounts are looked for. Whichever variable the platform keeps it in —
    /// see [`crate::platform::home_dir`] — and `None` on a machine that names
    /// none, which is a machine with no account to be found.
    home: Option<PathBuf>,

    /// And the two variables a GitHub token is prefilled out of, in the order
    /// `gh` itself reads them: `GH_TOKEN` and then `GITHUB_TOKEN`. Read at the
    /// edge with everything else here, so the arm that prefers one to the other
    /// is a unit test rather than a process environment a suite has to mutate.
    gh_token: Option<String>,
    github_token: Option<String>,

    /// And what this box calls itself, which is the one thing here that is
    /// neither a probe nor a setting: it is what the install run's status line
    /// names while a password dialog is up — see [`install`].
    ///
    /// Stated like everything else rather than read where it is wanted, and for
    /// the same reason: a status line that had this box's own hostname in it
    /// would be a golden fixture that read differently on every machine.
    hostname: String,
}

impl Machine {
    /// The machine this server is running on: the one read of it, made where a
    /// router is stood up and passed down from there.
    ///
    /// **With no `PATH` of its own held**, which is the one thing this has that
    /// a stated machine has not: what a session searches is composed at every
    /// read instead, so that a directory an install landed in inside this run
    /// is on the very next probe — see [`Machine::path`].
    pub fn here() -> Machine {
        Machine::of(
            Platform::HERE,
            None,
            sandbox::servers_path(),
            std::env::var_os("PATHEXT"),
            std::fs::read_to_string(OS_RELEASE).ok(),
            &Environment::of_the_process(),
            hostname(),
        )
    }

    /// A machine stated rather than read, which is what a test stands a server
    /// up over.
    ///
    /// The whole of what a probe can see, so a suite can put every platform,
    /// every distribution and a `bwrap` that will not run in front of the same
    /// code the served router runs — see [`crate::router_onboarding`], which is
    /// the same seam [`crate::remote::Tailscale::running`] is for the machine's
    /// Tailscale.
    pub fn stated(
        platform: Platform,
        path: OsString,
        servers: OsString,
        pathext: Option<OsString>,
        os_release: Option<String>,
        env: &Environment,
    ) -> Machine {
        Machine::of(
            platform,
            Some(path),
            servers,
            pathext,
            os_release,
            env,
            STATED.to_owned(),
        )
    }

    /// The same, called something in particular — which is what the one
    /// sentence that names this machine reads: the status line of an install
    /// run, while a password dialog is up on its screen.
    pub fn called(self, hostname: String) -> Machine {
        Machine { hostname, ..self }
    }

    /// The two ways of making one, said once: a `PATH` a caller stated, or none
    /// at all for the machine that composes its own.
    #[allow(clippy::too_many_arguments)]
    fn of(
        platform: Platform,
        path: Option<OsString>,
        servers: OsString,
        pathext: Option<OsString>,
        os_release: Option<String>,
        env: &Environment,
        hostname: String,
    ) -> Machine {
        Machine {
            platform,
            path,
            servers,
            pathext,
            os_release,
            // Read here rather than taken as a path, because which variable
            // holds it is one of the things this platform decides: a `HOME` on
            // a Windows machine was set by somebody's shell, and the account
            // the wizard is looking for is under the profile.
            home: crate::platform::home_dir(platform, env),
            gh_token: env.gh_token.clone(),
            github_token: env.github_token.clone(),
            hostname,
        }
    }

    /// What this machine calls itself.
    fn hostname(&self) -> &str {
        &self.hostname
    }

    /// And which of the wizard's eight tabs it is, which is also which package
    /// manager an install run raises — see [`install`].
    fn distro(&self) -> Distro {
        distro(self.platform, self.os_release.as_deref())
    }

    /// Everything a reading of this machine asks it, made in one hop off the
    /// runtime: the rows, and the accounts whose harnesses those rows are.
    fn probed(&self) -> Probed {
        let dependencies = self.rows();
        let accounts = self.accounts(&dependencies);

        Probed {
            dependencies,
            path: self.looks_in(),
            accounts,
        }
    }

    /// Every row of the dependencies step, in the order it is drawn: the
    /// sandbox, `git`, the four harnesses, and `gh`.
    ///
    /// Blocks: a `PATH` walk apiece, and one `bwrap` run.
    fn rows(&self) -> Vec<DependencyView> {
        let mut rows = vec![row(Dependency::Sandbox, self.sandbox())];

        rows.push(row(Dependency::Git, self.installed(sandbox::GIT)));

        rows.extend(HARNESSES.iter().map(|(dependency, agent_type)| {
            row(*dependency, self.installed(sessions::binary(*agent_type)))
        }));

        rows.push(row(Dependency::Gh, self.installed(sandbox::GH)));

        rows
    }

    /// Every agent account already in this server's home, in the order the
    /// harness rows are drawn.
    ///
    /// A home holds at most one account per harness — see
    /// [`sandbox::account_in_home`], which is where the shapes are — so this is
    /// four looks and never a search. Each carries whether its harness is
    /// there, read off the row that was already probed rather than probed
    /// again: one answer, so the tick and the row cannot disagree.
    ///
    /// Nothing at all where the platform names no home, which is a machine
    /// there is nowhere to look in.
    fn accounts(&self, dependencies: &[DependencyView]) -> Vec<AccountView> {
        let Some(home) = self.home.as_deref() else {
            return Vec::new();
        };

        HARNESSES
            .iter()
            .filter_map(|(dependency, agent_type)| {
                Some(AccountView {
                    account: profiles::account(&sandbox::account_in_home(*agent_type, home)?),
                    harness: there(dependencies, *dependency),
                })
            })
            .collect()
    }

    /// Whether a session would find `program`, said as a row's state — and
    /// where, which is the half of the answer this feature is about.
    ///
    /// **A row that is there says which file it is.** A distribution's `claude`
    /// too old to connect and the current one under `~/.local/bin` are the same
    /// tick and two different programs, and which of them a session got is the
    /// one thing the human cannot read off a tick.
    ///
    /// **And a row that is not says where it was seen**, where it was seen at
    /// all — on a `PATH` entry the composing dropped, or at the end of a link a
    /// session could not follow. Each of those is somebody's `PATH` to fix
    /// rather than a program to install, and a row that said only *absent*
    /// would send them to install what they have. See [`sandbox::standing`],
    /// which is where all four answers are decided.
    fn installed(&self, program: &str) -> DependencyState {
        match self.reaches(program) {
            sandbox::Standing::Found { at, landed, .. } => DependencyState::Present {
                at: Some(shown_path(&at)),
                // The link's target, and only where it is another file: a
                // program that is no link has one path and would read as two.
                target: (landed != at).then(|| shown_path(&landed)),
            },
            sandbox::Standing::Beyond { at } => absent(Seen::Beyond {
                at: shown_path(&at),
            }),
            sandbox::Standing::Leading { at, target } => absent(Seen::Leading {
                at: shown_path(&at),
                target: shown_path(&target),
            }),
            sandbox::Standing::Dangling { at } => absent(Seen::Dangling {
                at: shown_path(&at),
            }),
            sandbox::Standing::Nowhere => DependencyState::Absent {
                trouble: None,
                seen: None,
            },
        }
    }

    /// The `PATH` a session searches, which on the machine this server is
    /// running on is composed now rather than read off a field — see
    /// [`Machine::path`] for why that one has none held, and
    /// [`sandbox::machine_path`], which is the value a session is really given.
    fn path(&self) -> Cow<'_, OsStr> {
        match &self.path {
            Some(stated) => Cow::Borrowed(stated.as_os_str()),
            None => Cow::Owned(sandbox::machine_path(self.platform)),
        }
    }

    /// Where `program` really is for a session: resolved on the `PATH` a
    /// session gets and followed into whatever it links into — see
    /// [`sandbox::standing`], which is the rule and is the same one the sandbox
    /// grants by.
    ///
    /// So a `~/.local/bin/claude` linking into the versions directory the
    /// native installer keeps is a row that ticks, and one linking into
    /// somewhere no session can reach is a row that says absent: what a row
    /// promises is a session that can start.
    ///
    /// The server's own `PATH` goes in beside a session's because the answer
    /// runs to more than yes: where a name was seen is read off the entries the
    /// composing dropped.
    fn reaches(&self, program: &str) -> sandbox::Standing {
        sandbox::standing(
            self.platform,
            program,
            Some(&self.path()),
            Some(self.servers.as_os_str()),
            self.pathext.as_deref(),
            self.home.as_deref(),
        )
    }

    /// And where a session looks, in the order it looks: a session's own `PATH`
    /// as the directories it names.
    fn looks_in(&self) -> Vec<String> {
        sandbox::entries(self.platform, &self.path())
            .iter()
            .map(|directory| shown_path(directory))
            .collect()
    }

    /// And `program` resolved on that same list and nothing more asked of it:
    /// no link followed, no reach checked, no second list read.
    ///
    /// The shorter question, for the two rows that go and *run* something out
    /// here rather than say whether a session could. The Linux sandbox row
    /// starts a `bwrap` in this process, and the git step reads a `git config`
    /// out of the machine's own — see [`Machine::sandbox`] and
    /// [`Machine::configured`], which are both of them. What either needs is a
    /// file this process can start, and following a link into an install a
    /// *session* could not reach would be answering somebody else's question
    /// with it.
    ///
    /// A session's `PATH` rather than the server's all the same, so that what a
    /// row reports having run is a program on the list the row is about — see
    /// [`Machine::reaches`], which is that list asked the whole question.
    fn found(&self, program: &str) -> Option<PathBuf> {
        sandbox::on_the_path(
            self.platform,
            program,
            Some(&self.path()),
            self.pathext.as_deref(),
        )
    }

    /// And what the sandbox row says, which is the one row that is a different
    /// question on each platform.
    fn sandbox(&self) -> DependencyState {
        match self.platform {
            // Every Mac has `sandbox-exec`; it is Apple's own and there is no
            // version of macOS without it, so there is nothing to probe and
            // nothing anybody could install — and so no file to name under the
            // row either.
            Platform::MacOs => DependencyState::Present {
                at: None,
                target: None,
            },

            // And on Windows there is no sandbox to have: what holds a session
            // to its own work there is the identity it runs as, which Verkstead
            // makes for itself.
            Platform::Windows => DependencyState::NotApplicable,

            Platform::Linux => match self.found(BWRAP) {
                Some(bwrap) => trivially(&bwrap),
                None => DependencyState::Absent {
                    trouble: None,
                    seen: None,
                },
            },
        }
    }

    /// What this machine can offer the git step, for each field of it Verkstead
    /// has not been told — see [`Wanted`].
    ///
    /// Blocks: a `git config` apiece for the two fields that are wanted, and at
    /// most one `gh` for the third.
    fn prefilled(&self, gh: &Gh, wanted: Wanted) -> PrefillView {
        PrefillView {
            name: self.configured(wanted.name, "user.name"),
            email: self.configured(wanted.email, "user.email"),
            token: wanted.token.then(|| self.token(gh)).flatten(),
        }
    }

    /// What `git config --global` says `key` is, where it is wanted at all.
    ///
    /// The `git` a session would run, resolved the way every other row here is:
    /// what the wizard is about is the machine a session stands on, and the
    /// author it commits as comes off the same one.
    ///
    /// Nothing where there is no `git`, where it would not run, or where it
    /// printed nothing — all of which are the same thing to a field: there is
    /// nothing to offer, so it stays empty.
    fn configured(&self, wanted: bool, key: &str) -> Option<Prefilled> {
        if !wanted {
            return None;
        }

        let git = self.found(sandbox::GIT)?;
        let run = Command::new(git)
            .args(["config", "--global", "--get", key])
            .unseen()
            .stdin(Stdio::null())
            .output()
            .ok()?;

        run.status
            .success()
            .then(|| words(&run.stdout))
            .flatten()
            .map(|value| prefilled(value, Source::GitConfig))
    }

    /// And a GitHub token this machine is already holding: the server's own
    /// environment first, in the order `gh` itself reads the two variables, and
    /// the host `gh`'s own login after them.
    ///
    /// The environment before the process, because reading a variable costs
    /// nothing and running `gh` is a process — and because a token put in the
    /// environment of the thing that is running is the more deliberate of the
    /// two.
    fn token(&self, gh: &Gh) -> Option<Prefilled> {
        let said = |held: &Option<String>| {
            held.as_deref()
                .map(str::trim)
                .filter(|token| !token.is_empty())
                .map(str::to_owned)
        };

        if let Some(token) = said(&self.gh_token) {
            return Some(prefilled(token, Source::GhToken));
        }

        if let Some(token) = said(&self.github_token) {
            return Some(prefilled(token, Source::GithubToken));
        }

        github::host_token(gh).map(|token| prefilled(token, Source::HostGh))
    }
}

/// Which of the git step's three fields there is anything to prefill.
///
/// **A field Verkstead has been told is not one to prefill**: what the human is
/// looking at is then what is written down, and a value found on the machine
/// drawn over it would be the wizard proposing to overwrite the settings with
/// the environment. So this is read off the settings and the probes are made
/// for what is left — which is also what keeps a configured token from being
/// handed back to a browser that had no business being sent one.
#[derive(Debug, Clone, Copy)]
struct Wanted {
    name: bool,
    email: bool,
    token: bool,
}

/// One field's prefill.
fn prefilled(value: String, source: Source) -> Prefilled {
    Prefilled { value, source }
}

/// What one look at the machine found: the rows, and the accounts beside them.
///
/// The two together because they are one hop off the runtime and one answer:
/// whether a harness is there is a row's state and an account's tick both, and
/// asking twice would be two `PATH` walks that could disagree.
struct Probed {
    dependencies: Vec<DependencyView>,

    /// And where those rows were looked for, which is where a session looks.
    path: Vec<String>,

    accounts: Vec<AccountView>,
}

/// Onboarding mode: the verdict this server reached at startup, and the machine
/// every read of it probes.
///
/// A handle rather than a reading, the way [`crate::settings::Settings`] is:
/// what a state holds is where the answer comes from, and the answer itself is
/// made at the moment somebody asks for it. The one thing that is *not* made
/// afresh is the mode, which is the whole point of [`Mode`].
#[derive(Debug, Clone)]
pub struct Onboarding {
    machine: Machine,
    mode: Arc<Mode>,

    /// And the install run, which is the one thing the wizard *does* to this
    /// machine rather than reads off it — see [`install`].
    ///
    /// Held beside the mode for the same reason the mode is held: a run belongs
    /// to the life of this server, so what is here is what has happened since it
    /// came up, and a restart has none of it.
    installer: Arc<install::Installer>,
}

/// Whether the wizard is the only page there is, held for the length of a run.
///
/// **Settled once.** The cell is what makes it once and the startup read is
/// what makes it *at startup*: whichever gets there first computes the verdict,
/// and every reader afterwards is handed what it decided. A first read that
/// beat the startup task to it would reach the same answer off the same
/// machine, so there is no window in which two callers could disagree.
///
/// **And cleared once.** The wizard's last Continue is the one thing inside a
/// run that takes the mode off, and it is a flag beside the verdict rather than
/// a rewrite of it: the verdict is what was true at startup and stays said,
/// while this is what has happened since.
#[derive(Debug, Default)]
struct Mode {
    /// The verdict: whether the objective was unmet when it was reached.
    settled: OnceCell<bool>,

    /// And whether the wizard has finished since.
    finished: AtomicBool,
}

impl Onboarding {
    /// A server that probes `machine`, and installs what its wizard is missing
    /// by raising a command through `escalation`.
    ///
    /// `None` is a server nothing handed a way to ask: it probes and draws
    /// exactly as the other does, and a run started on it installs nothing and
    /// says why on every ticked row — see [`install`].
    pub fn probing(machine: Machine, escalation: Option<Arc<dyn Elevate>>) -> Onboarding {
        Onboarding {
            machine,
            mode: Arc::new(Mode::default()),
            installer: Arc::new(install::Installer::raising(escalation)),
        }
    }

    /// The whole of what the wizard is drawn from, read now.
    ///
    /// The probes and the store together, because the steps are the one thing
    /// both halves answer: what is on the machine is a `PATH` and a `bwrap`,
    /// and what is configured is a Profile and an author.
    pub(crate) async fn read(
        &self,
        pool: &SqlitePool,
        settings: &Settings,
    ) -> Result<OnboardingView> {
        // Off the runtime: a `PATH` walk is a handful of `stat` calls and the
        // sandbox row is a process, and neither belongs on a thread that is
        // meant to be answering requests. The accounts are looked for in the
        // same hop, being more of the same `stat` calls.
        let machine = self.machine.clone();
        let probed = tokio::task::spawn_blocking(move || machine.probed()).await?;

        let steps = steps(&probed.dependencies, pool, settings).await?;

        // And the run over the top of them, where one has been started: what the
        // probe answers is whether the machine has the thing, and what the run
        // answers is whether Verkstead is in the middle of putting it there.
        let (run, dependencies) = installing(probed.dependencies, self.installer.reading());

        Ok(OnboardingView {
            mode: self.mode(steps).await,
            platform: shown(self.machine.platform),
            distro: self.machine.distro(),
            dependencies,
            path: probed.path,
            accounts: probed.accounts,
            steps,
            run,
        })
    }

    /// Start an install run over `ticked`, and answer as soon as it is going.
    ///
    /// **Refused while the wizard is not the page there is.** A run installs
    /// software on the machine this server is on, behind a password dialog, and
    /// the one thing that says somebody asked for it is the wizard being up:
    /// see [`Refusal::Over`], which is a Verkstead that came up with the
    /// objective met or one whose wizard has already finished.
    ///
    /// **And while one is going**, which is the same press twice — see
    /// [`Refusal::Going`].
    pub(crate) async fn install(&self, ticked: Vec<Dependency>) -> Result<(), Refusal> {
        if !self.wizarding() {
            return Err(Refusal::Over);
        }

        // Off the runtime, being a `PATH` walk and a thread spawned. It answers
        // as soon as the run is going rather than when it is over: what is on
        // the other side of it is a human reading a dialog.
        let installing = self.clone();

        tokio::task::spawn_blocking(move || {
            installing.installer.start(&installing.machine, &ticked)
        })
        .await
        .unwrap_or(Ok(()))
    }

    /// Cancel whatever is going: the unit under way finishes, and the units
    /// after it are skipped.
    ///
    /// Nothing where nothing is going, and refused for nothing: a press that
    /// stops something is not one to hold up over whether the wizard is still
    /// the page.
    pub(crate) fn cancel(&self) {
        self.installer.cancel();
    }

    /// What this machine can offer the git step, for whatever Verkstead has
    /// not been told.
    ///
    /// **Its own read rather than a part of [`Onboarding::read`]**, and the
    /// reasoning is the cost of the probes and what one of them carries: the
    /// reading above is made every ten seconds while a step is unmet and again
    /// by the workbench's own gate at every start, and neither of those has any
    /// business running `git config` twice and `gh` once — or handing a GitHub
    /// token to a page that is drawing a sidebar. This is asked for by the step
    /// that has the fields, while they stand empty.
    pub(crate) async fn prefill(&self, settings: &Settings, gh: &Gh) -> Result<PrefillView> {
        // What is wanted is decided here, off the settings, and the probes are
        // made for that alone — see [`Wanted`].
        let wanted = {
            let config = settings.config();
            let author = config.git_author();

            Wanted {
                name: author.name().is_none(),
                email: author.email().is_none(),
                token: settings.secrets().github_token().is_none(),
            }
        };

        // Off the runtime, for the reason the probes above are: every one of
        // these is a process.
        let machine = self.machine.clone();
        let gh = gh.clone();

        Ok(tokio::task::spawn_blocking(move || machine.prefilled(&gh, wanted)).await?)
    }

    /// The wizard is over: the mode is off for the rest of this run.
    ///
    /// Pressed by the wizard's last Continue, which is the one thing inside a
    /// run that takes the mode off.
    ///
    /// Nothing about the verdict is rewritten and nothing is written down —
    /// the mode is a fact about this process, and the next start reaches it
    /// again off a machine that now has what it was missing.
    pub fn finished(&self) {
        self.mode.finished.store(true, Ordering::SeqCst);
    }

    /// Whether the mode is on: the verdict, settled off `steps` where nothing
    /// has settled it yet, and off for good once the wizard has finished.
    async fn mode(&self, steps: StepsView) -> bool {
        let unmet = self
            .mode
            .settled
            .get_or_init(|| async { !met(steps) })
            .await;

        *unmet && !self.mode.finished.load(Ordering::SeqCst)
    }

    /// The same question asked without a reading in hand, which is what a press
    /// has: whether this server is still the wizard.
    ///
    /// **A verdict nothing has settled yet reads as the wizard.** The startup
    /// read settles it within a moment of the server coming up — see
    /// [`at_startup`] — so what this is about is the sliver before that, and a
    /// press inside it came from a page that is drawing the wizard. Refusing it
    /// would be refusing the one thing the human is there to do.
    fn wizarding(&self) -> bool {
        self.mode.settled.get().copied().unwrap_or(true)
            && !self.mode.finished.load(Ordering::SeqCst)
    }
}

/// The rows with whatever a run has made of them written over the top, and the
/// run itself.
///
/// Two lists rather than one, because they are answers to different questions
/// asked a moment apart: the rows come off a probe of the machine and the run
/// off what this server is doing to it. A row a run says nothing about is left
/// exactly as the probe found it.
fn installing(
    dependencies: Vec<DependencyView>,
    run: Option<(RunView, Vec<(Dependency, InstallState)>)>,
) -> (Option<RunView>, Vec<DependencyView>) {
    let Some((run, installing)) = run else {
        return (None, dependencies);
    };

    let dependencies = dependencies
        .into_iter()
        .map(
            |row| match installing.iter().find(|(of, _)| *of == row.dependency) {
                Some((_, install)) => DependencyView {
                    install: install.clone(),
                    ..row
                },
                None => row,
            },
        )
        .collect();

    (Some(run), dependencies)
}

/// Settle the verdict, as early in a run as there is a runtime to settle it on.
///
/// The read is what settles it — see [`Onboarding::mode`] — so this is that
/// read, made once with nobody asking for the answer. It is here rather than in
/// the first request for the reason every other sweep in [`crate::routed`] is:
/// what it decides is about the machine as this server came up on it, and a
/// first request an hour later would be deciding about a different one.
pub(crate) fn at_startup(state: &crate::AppState) {
    let state = state.clone();

    tokio::spawn(async move {
        match state.onboarding.read(&state.pool, &state.settings).await {
            Ok(reading) => tracing::info!(
                mode = reading.mode,
                dependencies = reading.steps.dependencies,
                accounts = reading.steps.accounts,
                git = reading.steps.git,
                "the onboarding objective was read at startup"
            ),
            Err(trouble) => {
                tracing::warn!(?trouble, "the onboarding objective could not be read");
            }
        }
    });
}

/// Whether each of the three steps stands met, at this moment.
async fn steps(
    dependencies: &[DependencyView],
    pool: &SqlitePool,
    settings: &Settings,
) -> Result<StepsView> {
    // Every reading of a Profile is this list, so there is nothing cheaper to
    // ask: a machine has a handful of them.
    let profiles = store::profiles(pool).await?;
    let author = settings.config();
    let author = author.git_author();

    Ok(StepsView {
        dependencies: dependencies_met(dependencies),
        accounts: !profiles.is_empty(),
        git: author.name().is_some() && author.email().is_some(),
    })
}

/// The whole objective, which is the three steps together.
fn met(steps: StepsView) -> bool {
    steps.dependencies && steps.accounts && steps.git
}

/// And the first of them: a sandbox, `git`, and at least one of the four
/// harnesses.
///
/// *At least one*, because a session runs under one Profile and a Profile is of
/// one agent type: three rows left unticked hold nothing up. And `gh` holds
/// nothing up at all, GitHub being a choice rather than a dependency.
fn dependencies_met(dependencies: &[DependencyView]) -> bool {
    there(dependencies, Dependency::Sandbox)
        && there(dependencies, Dependency::Git)
        && HARNESSES
            .iter()
            .any(|(dependency, _)| there(dependencies, *dependency))
}

/// Whether one row of a reading says the machine has that thing.
///
/// The one reading of a row's state, so that the objective and an account's
/// tick are answering off the same list.
fn there(dependencies: &[DependencyView], dependency: Dependency) -> bool {
    dependencies
        .iter()
        .any(|row| row.dependency == dependency && present(&row.state))
}

/// Whether a row is one the objective can be met with: it is there, or it is
/// nothing this platform has to have.
fn present(state: &DependencyState) -> bool {
    matches!(
        state,
        DependencyState::Present { .. } | DependencyState::NotApplicable
    )
}

/// Which of the wizard's eight tabs this machine is.
///
/// `ID` first, and `ID_LIKE` after it, which is what makes a derivative get its
/// parent's commands: Linux Mint says `ID=linuxmint` and `ID_LIKE="ubuntu
/// debian"`, and the first of those two is the one whose `apt` line is right.
/// A machine naming none of them is *other Linux*, which the wizard answers
/// with the generic list rather than with a command that would be wrong.
fn distro(platform: Platform, os_release: Option<&str>) -> Distro {
    match platform {
        Platform::MacOs => Distro::MacOs,
        Platform::Windows => Distro::Windows,
        Platform::Linux => linux(os_release.unwrap_or_default()),
    }
}

/// And what a Linux calls itself, out of the file it says so in.
fn linux(os_release: &str) -> Distro {
    let named = |word: &str| {
        NAMED
            .iter()
            .find(|(id, _)| id.eq_ignore_ascii_case(word))
            .map(|(_, distro)| *distro)
    };

    said(os_release, "ID")
        .and_then(named)
        .or_else(|| {
            said(os_release, "ID_LIKE")?
                .split_whitespace()
                .find_map(named)
        })
        .unwrap_or(Distro::OtherLinux)
}

/// The five distributions the wizard writes a command for, under the `ID` each
/// of them takes.
const NAMED: &[(&str, Distro)] = &[
    ("nixos", Distro::NixOs),
    ("ubuntu", Distro::Ubuntu),
    ("fedora", Distro::Fedora),
    ("debian", Distro::Debian),
    ("arch", Distro::Arch),
];

/// What an os-release file says `key` is, unquoted, or nothing where it says
/// nothing.
///
/// The file is shell-syntax and its values are optionally quoted — `ID=fedora`
/// beside `ID_LIKE="ubuntu debian"` — so the quotes come off whichever pair
/// they were written with. Nothing else about the syntax is read: what is
/// wanted is two keys, and a parser for the rest would be a parser for a
/// language nothing here evaluates.
fn said<'a>(os_release: &'a str, key: &str) -> Option<&'a str> {
    os_release
        .lines()
        .filter_map(|line| line.split_once('='))
        .find(|(name, _)| name.trim() == key)
        .map(|(_, value)| value.trim().trim_matches(['"', '\'']))
}

/// One row, as the probe alone answers it: whatever an install run has made of
/// it is written over the top afterwards — see [`installing`].
fn row(dependency: Dependency, state: DependencyState) -> DependencyView {
    DependencyView {
        dependency,
        state,
        install: InstallState::Idle,
    }
}

/// What `bwrap` at this path made of the most trivial sandbox there is.
///
/// A run rather than a lookup, because the file being there says nothing about
/// whether it works: a kernel with unprivileged user namespaces switched off
/// refuses every one of them, and so does an AppImage's own `bwrap` under a
/// sandbox it is already inside. What it printed is kept as it is — the line
/// naming the sysctl to set is the one worth reading, and no sentence written
/// here would be as useful.
fn trivially(bwrap: &Path) -> DependencyState {
    let run = Command::new(bwrap)
        .args(TRIVIALLY)
        .unseen()
        .stdin(Stdio::null())
        .output();

    match run {
        // The file that was run, named the way every other present row names
        // one: this row is the server's own `PATH` walked rather than a
        // session's — a `bwrap` is started out here — and which `bwrap` was
        // tried is what its words underneath are about.
        Ok(run) if run.status.success() => DependencyState::Present {
            at: Some(shown_path(bwrap)),
            target: None,
        },
        Ok(run) => DependencyState::Absent {
            trouble: words(&run.stderr),
            seen: None,
        },
        // A `bwrap` that was found and would not start at all: a file that is
        // not executable, or one that has gone between the walk and the run.
        Err(trouble) => DependencyState::Absent {
            trouble: Some(trouble.to_string()),
            seen: None,
        },
    }
}

/// A row that is not there because the name was seen somewhere a session
/// cannot use it — which is a `PATH` to fix rather than a program to install.
fn absent(seen: Seen) -> DependencyState {
    DependencyState::Absent {
        trouble: None,
        seen: Some(seen),
    }
}

/// A path as the wizard shows one.
///
/// Lossily, and deliberately: what this is for is somebody reading it off a
/// page, and a directory whose name is not valid UTF-8 is still a directory
/// worth naming. Nothing is resolved from what is drawn.
fn shown_path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

/// What a failed run said, where it said anything.
fn words(stderr: &[u8]) -> Option<String> {
    let said = String::from_utf8_lossy(stderr).trim().to_owned();

    (!said.is_empty()).then_some(said)
}

/// The platform as the viewer receives it.
fn shown(platform: Platform) -> verkstead_render::Platform {
    match platform {
        Platform::Linux => verkstead_render::Platform::Linux,
        Platform::MacOs => verkstead_render::Platform::MacOs,
        Platform::Windows => verkstead_render::Platform::Windows,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stand_ins::program;

    /// A machine on `platform` whose `PATH` is `dir`, which says nothing about
    /// itself and whose home is `dir` as well.
    ///
    /// One directory for both because the two questions are asked apart: a
    /// `PATH` walk looks for names this puts there and an account is a shape
    /// under a home, and no test here writes both.
    fn machine(platform: Platform, dir: &Path) -> Machine {
        Machine::stated(
            platform,
            dir.as_os_str().to_owned(),
            dir.as_os_str().to_owned(),
            None,
            None,
            &home(platform, dir),
        )
    }

    /// A Linux machine whose session `PATH` is `dir` and whose server is
    /// running under `servers_home` — the two the per-user rules are read
    /// against, which the one-directory machine above cannot say apart.
    fn looking_in(dir: &Path, servers_home: &Path) -> Machine {
        Machine::stated(
            Platform::Linux,
            dir.as_os_str().to_owned(),
            dir.as_os_str().to_owned(),
            None,
            None,
            &home(Platform::Linux, servers_home),
        )
    }

    /// An environment naming `dir` as the home, in whichever variable this
    /// platform keeps one in.
    fn home(platform: Platform, dir: &Path) -> Environment {
        let dir = Some(dir.to_owned());

        match platform {
            Platform::Windows => Environment {
                userprofile: dir,
                ..Environment::default()
            },
            Platform::Linux | Platform::MacOs => Environment {
                home: dir,
                ..Environment::default()
            },
        }
    }

    /// The four shapes an account comes in, made under `home` — the same paths
    /// a session's account is mounted from.
    fn an_account(home: &Path, agent_type: store::AgentType) {
        let dirs: Vec<PathBuf> = match agent_type {
            store::AgentType::Claude => vec![home.join(".claude")],
            store::AgentType::Codex => vec![home.join(".codex")],
            store::AgentType::Grok => vec![home.join(".grok")],
            store::AgentType::OpenCode => {
                vec![
                    home.join(".config/opencode"),
                    home.join(".local/share/opencode"),
                ]
            }
        };

        for dir in dirs {
            std::fs::create_dir_all(dir).unwrap();
        }

        if agent_type == store::AgentType::Claude {
            std::fs::write(home.join(".claude.json"), "{}\n").unwrap();
        }
    }

    /// Which harnesses a machine found an account for, in the order it offered
    /// them.
    fn offered(machine: &Machine) -> Vec<store::AgentType> {
        machine
            .probed()
            .accounts
            .iter()
            .map(|found| match found.account {
                verkstead_render::ProfileAccount::Claude { .. } => store::AgentType::Claude,
                verkstead_render::ProfileAccount::Codex { .. } => store::AgentType::Codex,
                verkstead_render::ProfileAccount::Grok { .. } => store::AgentType::Grok,
                verkstead_render::ProfileAccount::OpenCode { .. } => store::AgentType::OpenCode,
            })
            .collect()
    }

    /// A machine whose `PATH` and home are `dir`, holding whatever the two
    /// token variables were set to.
    fn holding(dir: &Path, gh_token: Option<&str>, github_token: Option<&str>) -> Machine {
        Machine::stated(
            Platform::Linux,
            dir.as_os_str().to_owned(),
            dir.as_os_str().to_owned(),
            None,
            None,
            &Environment {
                gh_token: gh_token.map(str::to_owned),
                github_token: github_token.map(str::to_owned),
                ..home(Platform::Linux, dir)
            },
        )
    }

    /// A `git` in `dir` answering `git config --global --get user.name` with
    /// `name` and the email likewise, and saying nothing where nothing was
    /// configured — which is what a machine nobody has set up does.
    #[cfg(unix)]
    fn a_git(dir: &Path, name: Option<&str>, email: Option<&str>) {
        let said = |value: Option<&str>| match value {
            Some(value) => format!("echo '{value}'"),
            None => "exit 1".to_owned(),
        };

        program(
            &dir.join(sandbox::GIT),
            &format!(
                r#"#!/bin/sh
test "$1 $2 $3" = 'config --global --get' || exit 2
case "$4" in
  user.name) {};;
  user.email) {};;
  *) exit 1;;
esac
"#,
                said(name),
                said(email),
            ),
        );
    }

    /// And a `gh` that is logged in as somebody, or is not.
    #[cfg(unix)]
    fn a_gh(dir: &Path, token: Option<&str>) -> Gh {
        let path = dir.join(sandbox::GH);

        program(
            &path,
            &match token {
                Some(token) => format!(
                    r#"#!/bin/sh
test "$*" = 'auth token' || exit 2
echo {token}
"#
                ),
                None => "#!/bin/sh\nexit 1\n".to_owned(),
            },
        );

        Gh::running(vec![path.to_string_lossy().into_owned()])
    }

    /// Everything wanted, which is a Verkstead that has been told nothing.
    const EVERYTHING: Wanted = Wanted {
        name: true,
        email: true,
        token: true,
    };

    /// A row that is there, where which file it is, is not what is being asked
    /// about: the objective is met by a row's standing rather than by a path.
    const THERE: DependencyState = DependencyState::Present {
        at: None,
        target: None,
    };

    /// And one that is not, with nothing seen anywhere on the machine.
    const NOT_THERE: DependencyState = DependencyState::Absent {
        trouble: None,
        seen: None,
    };

    /// What one row of a reading says.
    fn state(machine: &Machine, dependency: Dependency) -> DependencyState {
        machine
            .rows()
            .into_iter()
            .find(|row| row.dependency == dependency)
            .expect("every dependency is a row")
            .state
    }

    /// The five distributions the wizard writes commands for say so in `ID`,
    /// and each of them is its own tab.
    #[test]
    fn a_distribution_is_named_by_the_id_it_calls_itself() {
        for (id, expected) in [
            ("nixos", Distro::NixOs),
            ("ubuntu", Distro::Ubuntu),
            ("fedora", Distro::Fedora),
            ("debian", Distro::Debian),
            ("arch", Distro::Arch),
        ] {
            let os_release = format!("NAME=\"Something\"\nID={id}\nVERSION_ID=\"1\"\n");

            assert_eq!(
                distro(Platform::Linux, Some(&os_release)),
                expected,
                "{id} is its own tab",
            );
        }
    }

    /// And a derivative gets its parent's commands, which is the whole of what
    /// `ID_LIKE` is read for: Mint's `apt` line is Ubuntu's.
    #[test]
    fn a_derivative_takes_the_commands_of_the_first_parent_it_names() {
        for (os_release, expected) in [
            ("ID=linuxmint\nID_LIKE=\"ubuntu debian\"\n", Distro::Ubuntu),
            ("ID=raspbian\nID_LIKE=debian\n", Distro::Debian),
            ("ID=manjaro\nID_LIKE=arch\n", Distro::Arch),
            (
                "ID=\"rocky\"\nID_LIKE=\"rhel centos fedora\"\n",
                Distro::Fedora,
            ),
            ("ID=pop\nID_LIKE=ubuntu debian\n", Distro::Ubuntu),
        ] {
            assert_eq!(
                distro(Platform::Linux, Some(os_release)),
                expected,
                "{os_release:?} is a derivative of a distribution with a command",
            );
        }
    }

    /// `ID` beats `ID_LIKE`, so a distribution with commands of its own is
    /// never read as its parent.
    #[test]
    fn the_id_is_read_before_what_it_is_like() {
        assert_eq!(
            distro(Platform::Linux, Some("ID=ubuntu\nID_LIKE=debian\n")),
            Distro::Ubuntu,
            "Ubuntu's own tab rather than Debian's, which is what it says it is like",
        );
    }

    /// Everything else is *other Linux*: a distribution naming nothing the
    /// wizard knows, and a machine with no such file at all.
    #[test]
    fn a_linux_naming_nothing_the_wizard_knows_is_the_generic_tab() {
        for os_release in [
            None,
            Some(""),
            Some("ID=gentoo\n"),
            Some("ID=alpine\nID_LIKE=\"\"\n"),
            Some("NAME=\"Something\"\n"),
        ] {
            assert_eq!(
                distro(Platform::Linux, os_release),
                Distro::OtherLinux,
                "{os_release:?} names no distribution with a command written for it",
            );
        }
    }

    /// And the two platforms that have no distribution to be read never read
    /// one, whatever happens to be in a file of that name.
    #[test]
    fn the_platforms_that_are_not_a_linux_are_their_own_tab() {
        for (platform, expected) in [
            (Platform::MacOs, Distro::MacOs),
            (Platform::Windows, Distro::Windows),
        ] {
            assert_eq!(
                distro(platform, Some("ID=ubuntu\n")),
                expected,
                "{platform:?} is what it is whatever a file says",
            );
        }
    }

    /// An os-release value is shell syntax, so it may be quoted or bare and is
    /// the same value either way.
    #[test]
    fn a_quoted_os_release_value_is_the_value_inside_the_quotes() {
        assert_eq!(said("ID=fedora\n", "ID"), Some("fedora"));
        assert_eq!(said("ID=\"fedora\"\n", "ID"), Some("fedora"));
        assert_eq!(said("ID='fedora'\n", "ID"), Some("fedora"));
        assert_eq!(
            said("ID=debian\nID_LIKE=\"ubuntu debian\"\n", "ID_LIKE"),
            Some("ubuntu debian"),
            "and a value holding several is one string, split by whoever reads it",
        );
        assert_eq!(said("ID=debian\n", "ID_LIKE"), None);
    }

    /// A harness is probed under the name a session is launched as, on the
    /// `PATH` a session gets — so a program in a directory on that list is a
    /// row that ticks.
    #[cfg(unix)]
    #[test]
    fn a_program_on_the_path_a_session_gets_is_present() {
        let dir = tempfile::tempdir().unwrap();
        program(&dir.path().join("claude"), "#!/bin/sh\n");

        let machine = machine(Platform::Linux, dir.path());

        assert_eq!(
            state(&machine, Dependency::Claude),
            DependencyState::Present {
                at: Some(dir.path().join("claude").to_string_lossy().into_owned()),
                target: None,
            },
            "the name a session launches Claude Code under is on this PATH, and \
             the row says which file that is",
        );
        assert_eq!(
            state(&machine, Dependency::Codex),
            DependencyState::Absent {
                trouble: None,
                seen: None,
            },
            "and the three that are not there say so, with nothing to say about \
             it: a name on no `PATH` at all was seen nowhere",
        );
    }

    /// A harness the vendor's own installer left is a link into a versions
    /// directory, and the row that ticks for it is the row for the file that
    /// link lands on.
    ///
    /// The install this whole feature is about, asked of the wizard: the sandbox
    /// grants the directory it lands in — see [`sandbox::installs`] — so the row
    /// and the session are saying the one thing.
    #[cfg(unix)]
    #[test]
    fn a_harness_linked_into_its_own_install_under_the_home_is_present() {
        let servers_home = tempfile::tempdir().unwrap();
        let local = servers_home.path().join(".local/bin");
        let version = servers_home
            .path()
            .join(".local/share/claude/versions/0.0.0");

        std::fs::create_dir_all(&local).unwrap();
        std::fs::create_dir_all(&version).unwrap();
        program(&version.join("claude"), "#!/bin/sh\n");
        std::os::unix::fs::symlink(version.join("claude"), local.join("claude")).unwrap();

        let machine = looking_in(&local, servers_home.path());

        assert_eq!(
            state(&machine, Dependency::Claude),
            DependencyState::Present {
                at: Some(local.join("claude").to_string_lossy().into_owned()),
                target: Some(version.join("claude").to_string_lossy().into_owned()),
            },
            "a session would follow that link and run what is at the end of it, \
             and the row says both halves: the name on the `PATH` and the \
             version it is really running",
        );
    }

    /// And a link into somewhere no session can reach, or one that leads
    /// nowhere at all, reads absent: what a row promises is a session that can
    /// start, rather than a file that happens to be on a list.
    #[cfg(unix)]
    #[test]
    fn a_harness_linking_out_of_the_home_or_to_nothing_is_absent() {
        let servers_home = tempfile::tempdir().unwrap();
        let elsewhere = tempfile::tempdir().unwrap();
        let local = servers_home.path().join(".local/bin");

        std::fs::create_dir_all(&local).unwrap();

        // An install nothing binds — an `/opt/claude`, which a session opens
        // and finds absent.
        program(&elsewhere.path().join("claude"), "#!/bin/sh\n");
        std::os::unix::fs::symlink(elsewhere.path().join("claude"), local.join("claude")).unwrap();

        // And what an uninstall leaves behind.
        std::os::unix::fs::symlink(servers_home.path().join("gone/codex"), local.join("codex"))
            .unwrap();

        let machine = looking_in(&local, servers_home.path());

        assert_eq!(
            state(&machine, Dependency::Claude),
            DependencyState::Absent {
                trouble: None,
                seen: Some(Seen::Leading {
                    at: local.join("claude").to_string_lossy().into_owned(),
                    target: elsewhere
                        .path()
                        .join("claude")
                        .to_string_lossy()
                        .into_owned(),
                }),
            },
            "a link into somewhere a session cannot reach is not a harness a \
             session has — and the row says where it leads, that being what \
             there is to do something about",
        );
        assert_eq!(
            state(&machine, Dependency::Codex),
            DependencyState::Absent {
                trouble: None,
                seen: Some(Seen::Dangling {
                    at: local.join("codex").to_string_lossy().into_owned(),
                }),
            },
            "and neither is a link with nothing at the end of it, which is what \
             an uninstall leaves behind rather than a harness never installed",
        );
    }

    /// A harness on the `PATH` the *server* was started with, in a directory
    /// the composing dropped, is absent with the place it was seen: what is
    /// wanted there is a `PATH` and a restart rather than an install.
    ///
    /// The `/opt/foo/bin` case, and the `/mnt/c/...` entries WSL appends with
    /// it — a program the human really has, that no session can open.
    #[cfg(unix)]
    #[test]
    fn a_harness_on_an_entry_the_composing_dropped_is_absent_with_where_it_was_seen() {
        let servers_home = tempfile::tempdir().unwrap();
        let beyond = tempfile::tempdir().unwrap();
        let local = servers_home.path().join(".local/bin");

        std::fs::create_dir_all(&local).unwrap();
        program(&beyond.path().join("claude"), "#!/bin/sh\n");

        // A session's own `PATH` holds the entry under the home; the server's
        // holds that and the one the composing would drop.
        let machine = Machine::stated(
            Platform::Linux,
            local.as_os_str().to_owned(),
            OsString::from(format!("{}:{}", local.display(), beyond.path().display())),
            None,
            None,
            &home(Platform::Linux, servers_home.path()),
        );

        assert_eq!(
            state(&machine, Dependency::Claude),
            DependencyState::Absent {
                trouble: None,
                seen: Some(Seen::Beyond {
                    at: beyond.path().join("claude").to_string_lossy().into_owned(),
                }),
            },
            "the program is on this machine and no session is told about the \
             directory it is in, which is what the row has to say",
        );
        assert_eq!(
            state(&machine, Dependency::Codex),
            DependencyState::Absent {
                trouble: None,
                seen: None,
            },
            "while a name on neither list was seen nowhere at all",
        );
    }

    /// And where a session looks is the list a session was given, in order.
    #[cfg(unix)]
    #[test]
    fn the_reading_carries_the_path_a_session_is_given() {
        let dir = tempfile::tempdir().unwrap();
        let machine = Machine::stated(
            Platform::Linux,
            OsString::from(format!("{}:/usr/bin", dir.path().display())),
            OsString::new(),
            None,
            None,
            &home(Platform::Linux, dir.path()),
        );

        assert_eq!(
            machine.probed().path,
            vec![
                dir.path().to_string_lossy().into_owned(),
                "/usr/bin".to_owned()
            ],
            "the wizard says where a session looks by saying the directories it \
             looks in, in the order it looks",
        );
    }

    /// Every name the wizard draws a row for is a name the sandbox follows into
    /// its install, because they are the one list — see [`sandbox::PROGRAMS`].
    ///
    /// What this holds together is a rule and its reach: a name a row ticked for
    /// and nothing followed would be a session handed a dangling link, and a
    /// name followed with no row would be a program nobody is told to install.
    #[test]
    fn every_name_the_wizard_has_a_row_for_is_a_name_a_session_is_followed_into() {
        let mut names: Vec<&str> = std::iter::once(sandbox::GIT)
            .chain(
                HARNESSES
                    .iter()
                    .map(|(_, agent_type)| sessions::binary(*agent_type)),
            )
            .chain(std::iter::once(sandbox::GH))
            .collect();

        let mut followed: Vec<&str> = sandbox::PROGRAMS.to_vec();

        names.sort_unstable();
        followed.sort_unstable();

        assert_eq!(
            names, followed,
            "one list: a backend that lands is a row and a name followed, and \
             neither is a second place to say so",
        );
    }

    /// And a harness the human installed for themselves is one of them: the
    /// vendor's own installer puts Claude Code in `~/.local/bin`, which a
    /// session searches because the `PATH` the server was started with is what
    /// a session's is composed out of — see [`sandbox::composed`].
    ///
    /// Ahead of the system directories, which is the whole point: a
    /// distribution's package too old to connect is what a session would find
    /// otherwise, and the row would tick on the wrong one.
    #[cfg(unix)]
    #[test]
    fn a_harness_under_the_servers_own_home_is_present_and_leads_the_path() {
        let servers_home = tempfile::tempdir().unwrap();
        let local = servers_home.path().join(".local/bin");

        std::fs::create_dir_all(&local).unwrap();
        program(&local.join("claude"), "#!/bin/sh\n");

        let path = sandbox::composed(
            Platform::Linux,
            &[],
            &OsString::from(format!("{}:/usr/bin", local.display())),
            Some(servers_home.path()),
        );

        let machine = Machine::stated(
            Platform::Linux,
            path.clone(),
            OsString::from(format!("{}:/usr/bin", local.display())),
            None,
            None,
            &home(Platform::Linux, servers_home.path()),
        );

        assert_eq!(
            state(&machine, Dependency::Claude),
            DependencyState::Present {
                at: Some(local.join("claude").to_string_lossy().into_owned()),
                target: None,
            },
            "a session would find that install, so the row says so — and says \
             which file it is",
        );

        let path = path.to_string_lossy();
        let entries: Vec<&str> = path.split(':').collect();

        assert_eq!(
            entries.first().copied(),
            local.to_str(),
            "and it is the first place a session looks: {entries:?}",
        );
        assert!(
            entries.contains(&"/usr/bin"),
            "with the machine's own still under it: {entries:?}",
        );
    }

    /// On Windows a bare name is not a program at all, and what says which
    /// names are is `%PATHEXT%` — so a `claude.CMD`, which is what an npm
    /// install writes, is found and the extensionless file beside it is not.
    ///
    /// Named in the case the extension list is written in, for the reason the
    /// open rendering's own tests name one that way: the machine running this
    /// is one that tells `.CMD` and `.cmd` apart, and the machine it is about
    /// is not.
    #[test]
    fn a_windows_name_is_read_with_the_extensions_that_machine_names() {
        let dir = tempfile::tempdir().unwrap();
        program(&dir.path().join("claude.CMD"), "@echo off\n");
        program(
            &dir.path().join("codex"),
            "not a program on this platform\n",
        );

        let machine = Machine::stated(
            Platform::Windows,
            dir.path().as_os_str().to_owned(),
            dir.path().as_os_str().to_owned(),
            Some(OsString::from(".COM;.EXE;.BAT;.CMD")),
            None,
            &Environment::default(),
        );

        assert_eq!(
            state(&machine, Dependency::Claude),
            DependencyState::Present {
                at: Some(dir.path().join("claude.CMD").to_string_lossy().into_owned()),
                target: None,
            },
            "`claude.cmd` is what an npm install writes and what this platform runs",
        );
        assert_eq!(
            state(&machine, Dependency::Codex),
            DependencyState::Absent {
                trouble: None,
                seen: None,
            },
            "and a file with no extension is a shell script for a Unix, which \
             nothing on this platform can start",
        );
    }

    /// The sandbox row on a Mac ticks and on Windows is not a thing to have,
    /// whatever is on either machine's `PATH`.
    #[test]
    fn the_sandbox_row_is_answered_by_the_platform_where_it_is_not_a_program() {
        let dir = tempfile::tempdir().unwrap();

        assert_eq!(
            state(&machine(Platform::MacOs, dir.path()), Dependency::Sandbox),
            DependencyState::Present {
                at: None,
                target: None,
            },
            "every Mac has `sandbox-exec`, so there is nothing to install and no \
             file the row had to go and find",
        );
        assert_eq!(
            state(&machine(Platform::Windows, dir.path()), Dependency::Sandbox),
            DependencyState::NotApplicable,
            "and on Windows a session's boundary is an identity rather than a program",
        );
    }

    /// On Linux it is a run: a machine with no `bwrap` is absent with nothing
    /// said about it, because nothing was run to say anything.
    #[test]
    fn a_linux_with_no_bwrap_is_absent_and_says_nothing() {
        let dir = tempfile::tempdir().unwrap();

        assert_eq!(
            state(&machine(Platform::Linux, dir.path()), Dependency::Sandbox),
            DependencyState::Absent {
                trouble: None,
                seen: None,
            },
            "there is no such program, which is a thing to install rather than \
             a failure to report",
        );
    }

    /// And a `bwrap` that is installed and will not make a namespace is absent
    /// with what it said kept — which is the line naming the sysctl that would
    /// let it.
    #[cfg(unix)]
    #[test]
    fn a_bwrap_that_will_not_run_keeps_what_it_said() {
        let dir = tempfile::tempdir().unwrap();
        program(
            &dir.path().join(BWRAP),
            "#!/bin/sh\necho 'bwrap: No permissions to creating new namespace' >&2\nexit 1\n",
        );

        assert_eq!(
            state(&machine(Platform::Linux, dir.path()), Dependency::Sandbox),
            DependencyState::Absent {
                trouble: Some("bwrap: No permissions to creating new namespace".to_owned()),
                seen: None,
            },
            "the machine's own words, which are the ones worth reading",
        );
    }

    /// A `bwrap` that makes one ticks, and what it was asked to make is the
    /// most trivial sandbox there is.
    #[cfg(unix)]
    #[test]
    fn a_bwrap_that_makes_a_namespace_ticks() {
        let dir = tempfile::tempdir().unwrap();
        program(
            &dir.path().join(BWRAP),
            "#!/bin/sh\ntest \"$*\" = '--ro-bind / / /bin/sh -c :'\n",
        );

        assert_eq!(
            state(&machine(Platform::Linux, dir.path()), Dependency::Sandbox),
            DependencyState::Present {
                at: Some(dir.path().join(BWRAP).to_string_lossy().into_owned()),
                target: None,
            },
            "it was asked for a read-only bind of the machine and the shell's \
             own do-nothing, and the row names the `bwrap` that answered",
        );
    }

    /// And what it is asked to run is a program this machine has.
    ///
    /// The one thing the stub above cannot say. A stub agrees with whatever
    /// argument vector it is written beside, so a payload no Linux ships would
    /// pass every test here and fail on the machine — which is what `/bin/true`
    /// did: NixOS keeps `sh` in `/bin` and nothing else, so the row came back
    /// absent on a `bwrap` that worked, holding the wizard on a dependency that
    /// was already installed.
    ///
    /// Asked of the running machine rather than of a fixture, because the claim
    /// is about machines: `/bin/sh` is what POSIX obliges a Linux to have, and
    /// this is the assertion that notices the day something is chosen that is
    /// not.
    #[cfg(target_os = "linux")]
    #[test]
    fn what_the_sandbox_row_runs_inside_is_a_program_this_machine_has() {
        assert!(
            TRIVIALLY.contains(&SHELL),
            "the vector runs whatever this names, so the two have to agree",
        );
        assert!(
            Path::new(SHELL).is_file(),
            "the sandbox row runs {SHELL} inside the namespace, and this machine \
             has no such file — every Linux has /bin/sh and not every Linux has \
             anything else",
        );
    }

    /// Each of the four shapes in the server's home is offered as the Profile
    /// it would be saved as, in the order the harness rows are drawn.
    ///
    /// The paths are the account's own — the wizard hands them straight back to
    /// the profile create — and the tick beside each is that harness's row,
    /// which is what says an account there is one there is anything to run.
    #[cfg(unix)]
    #[test]
    fn every_account_in_the_home_is_offered_with_its_harness_beside_it() {
        let dir = tempfile::tempdir().unwrap();

        for agent_type in [
            store::AgentType::Claude,
            store::AgentType::Codex,
            store::AgentType::Grok,
            store::AgentType::OpenCode,
        ] {
            an_account(dir.path(), agent_type);
        }

        // And one harness on the machine, which is what tells the ticked row
        // from the greyed ones.
        program(&dir.path().join("claude"), "#!/bin/sh\n");

        let machine = machine(Platform::Linux, dir.path());
        let accounts = machine.probed().accounts;

        assert_eq!(
            offered(&machine),
            vec![
                store::AgentType::Claude,
                store::AgentType::Codex,
                store::AgentType::Grok,
                store::AgentType::OpenCode,
            ],
            "a home holds one account per harness, and all four are here",
        );

        assert_eq!(
            accounts[0].account,
            verkstead_render::ProfileAccount::Claude {
                claude_dir: dir.path().join(".claude").to_string_lossy().into_owned(),
                config_file: dir
                    .path()
                    .join(".claude.json")
                    .to_string_lossy()
                    .into_owned(),
            },
            "Claude's is the pair, which is what a session mounts and what the \
             profile create is handed",
        );
        assert_eq!(
            accounts[3].account,
            verkstead_render::ProfileAccount::OpenCode {
                home: dir.path().to_string_lossy().into_owned(),
            },
            "and opencode's is the home its XDG directories are under, which is \
             the home itself",
        );

        assert!(
            accounts[0].harness,
            "the account whose harness is on this machine is one to offer ticked",
        );
        assert!(
            accounts[1..].iter().all(|found| !found.harness),
            "and the three whose binaries are missing are not accounts to make a \
             Profile of yet",
        );
    }

    /// Half a shape is no account. Every path of it has to be there, because
    /// every one of them is mounted — and a row offered here that the profile
    /// create would refuse is a tick that saves nothing.
    #[cfg(unix)]
    #[test]
    fn a_shape_that_is_only_half_there_is_not_offered() {
        let dir = tempfile::tempdir().unwrap();

        // Claude's directory without the config file beside it, which is an
        // account the profile create refuses as `ConfigMissing`.
        std::fs::create_dir_all(dir.path().join(".claude")).unwrap();

        // And an opencode home with the config directory and no data
        // directory, which is the half of it the account is *not* in.
        std::fs::create_dir_all(dir.path().join(".config/opencode")).unwrap();

        assert_eq!(
            offered(&machine(Platform::Linux, dir.path())),
            Vec::new(),
            "neither is a whole account, so neither is offered",
        );
    }

    /// Where the home is, is the platform's own question: a Windows machine
    /// keeps it in `%USERPROFILE%`, and a `HOME` there was set by somebody's
    /// shell.
    #[test]
    fn a_windows_machine_looks_in_the_user_profile() {
        let dir = tempfile::tempdir().unwrap();
        an_account(dir.path(), store::AgentType::Codex);

        assert_eq!(
            offered(&machine(Platform::Windows, dir.path())),
            vec![store::AgentType::Codex],
            "the profile is where a Windows account is kept, and the shape under \
             it is the one a session mounts",
        );

        let shells_home = Machine::stated(
            Platform::Windows,
            OsString::new(),
            OsString::new(),
            None,
            None,
            &Environment {
                home: Some(dir.path().to_owned()),
                ..Environment::default()
            },
        );

        assert_eq!(
            offered(&shells_home),
            Vec::new(),
            "and a Windows machine naming no profile has no home to look in, \
             whatever a shell set HOME to",
        );
    }

    /// A machine that names no home at all is a machine with nowhere to look,
    /// which is nothing offered rather than a search.
    #[test]
    fn a_machine_with_no_home_offers_nothing() {
        for platform in [Platform::Linux, Platform::MacOs, Platform::Windows] {
            let nowhere = Machine::stated(
                platform,
                OsString::new(),
                OsString::new(),
                None,
                None,
                &Environment::default(),
            );

            assert_eq!(
                offered(&nowhere),
                Vec::new(),
                "{platform:?} says where a home is, and this one says nothing",
            );
        }
    }

    /// The objective wants a sandbox, `git` and *one* harness — so three rows
    /// left unticked hold nothing up, and `gh` holds nothing up at all.
    #[test]
    fn one_harness_meets_the_step_and_gh_never_gates() {
        let rows = |claude, gh| {
            vec![
                row(Dependency::Sandbox, THERE),
                row(Dependency::Git, THERE),
                row(Dependency::Claude, claude),
                row(Dependency::Codex, NOT_THERE),
                row(Dependency::Grok, NOT_THERE),
                row(Dependency::OpenCode, NOT_THERE),
                row(Dependency::Gh, gh),
            ]
        };

        assert!(
            dependencies_met(&rows(THERE, NOT_THERE)),
            "one harness is what a Profile runs under, and `gh` is a choice",
        );
        assert!(
            !dependencies_met(&rows(NOT_THERE, THERE)),
            "and no harness at all is a machine that can run no session",
        );
    }

    /// A sandbox row that is *not applicable* is a step that stands met: there
    /// is nothing on Windows to install, so a row that never ticks must not be
    /// a row that holds the wizard.
    #[test]
    fn a_windows_sandbox_row_does_not_hold_the_step() {
        let rows = vec![
            row(Dependency::Sandbox, DependencyState::NotApplicable),
            row(Dependency::Git, THERE),
            row(Dependency::Claude, THERE),
        ];

        assert!(dependencies_met(&rows));
    }

    /// The two fields git asks for come off the machine's own global config,
    /// labelled with where they were found.
    #[cfg(unix)]
    #[test]
    fn the_author_is_prefilled_from_the_machines_global_git_config() {
        let dir = tempfile::tempdir().unwrap();
        a_git(dir.path(), Some("Ada Lovelace"), Some("ada@example.com"));
        let gh = a_gh(dir.path(), None);

        let prefill = holding(dir.path(), None, None).prefilled(&gh, EVERYTHING);

        assert_eq!(
            prefill.name,
            Some(prefilled("Ada Lovelace".to_owned(), Source::GitConfig)),
        );
        assert_eq!(
            prefill.email,
            Some(prefilled("ada@example.com".to_owned(), Source::GitConfig)),
        );
    }

    /// And a machine whose git config says nothing leaves them empty, which is
    /// a field to type in rather than a wrong one to correct.
    #[cfg(unix)]
    #[test]
    fn a_machine_that_names_no_author_prefills_nothing() {
        let dir = tempfile::tempdir().unwrap();
        a_git(dir.path(), None, None);
        let gh = a_gh(dir.path(), None);

        let prefill = holding(dir.path(), None, None).prefilled(&gh, EVERYTHING);

        assert_eq!(prefill.name, None);
        assert_eq!(prefill.email, None);
        assert_eq!(prefill.token, None, "and no `gh` logged in is no token");
    }

    /// A machine with no `git` at all is the same nothing: the step above the
    /// git one is where that is put right, and this one has nothing to offer
    /// until it is.
    #[cfg(unix)]
    #[test]
    fn a_machine_with_no_git_prefills_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let gh = a_gh(dir.path(), None);

        let prefill = holding(dir.path(), None, None).prefilled(&gh, EVERYTHING);

        assert_eq!(prefill.name, None);
        assert_eq!(prefill.email, None);
    }

    /// The token comes out of the server's own environment first, in the order
    /// `gh` itself reads the two variables — and which of them held it is what
    /// the field is labelled with.
    #[cfg(unix)]
    #[test]
    fn the_token_is_prefilled_from_gh_token_before_github_token() {
        let dir = tempfile::tempdir().unwrap();
        let gh = a_gh(dir.path(), Some("ghp_thehostslogin"));

        assert_eq!(
            holding(dir.path(), Some("ghp_first"), Some("ghp_second"))
                .prefilled(&gh, EVERYTHING)
                .token,
            Some(prefilled("ghp_first".to_owned(), Source::GhToken)),
        );

        assert_eq!(
            holding(dir.path(), None, Some("ghp_second"))
                .prefilled(&gh, EVERYTHING)
                .token,
            Some(prefilled("ghp_second".to_owned(), Source::GithubToken)),
            "and the second variable where the first said nothing",
        );

        assert_eq!(
            holding(dir.path(), Some("   "), Some("ghp_second"))
                .prefilled(&gh, EVERYTHING)
                .token,
            Some(prefilled("ghp_second".to_owned(), Source::GithubToken)),
            "a variable set to nothing but spaces holds no token",
        );
    }

    /// And the host's own `gh` where the environment holds neither, which is
    /// the one of the three that is a process.
    #[cfg(unix)]
    #[test]
    fn the_token_falls_back_to_the_login_the_host_gh_is_holding() {
        let dir = tempfile::tempdir().unwrap();
        let gh = a_gh(dir.path(), Some("ghp_thehostslogin"));

        assert_eq!(
            holding(dir.path(), None, None)
                .prefilled(&gh, EVERYTHING)
                .token,
            Some(prefilled("ghp_thehostslogin".to_owned(), Source::HostGh)),
        );
    }

    /// A field Verkstead has already been told is not prefilled at all, and the
    /// probe behind it is not made: what the human is looking at is what is
    /// written down, and a token already configured is one this endpoint has no
    /// business handing back to a browser.
    #[cfg(unix)]
    #[test]
    fn a_field_that_is_already_configured_is_not_prefilled() {
        let dir = tempfile::tempdir().unwrap();
        a_git(dir.path(), Some("Ada Lovelace"), Some("ada@example.com"));
        let gh = a_gh(dir.path(), Some("ghp_thehostslogin"));

        let prefill = holding(dir.path(), Some("ghp_first"), None).prefilled(
            &gh,
            Wanted {
                name: false,
                email: true,
                token: false,
            },
        );

        assert_eq!(prefill.name, None, "the name is Verkstead's own already");
        assert_eq!(prefill.token, None, "and so is the token");
        assert_eq!(
            prefill.email,
            Some(prefilled("ada@example.com".to_owned(), Source::GitConfig)),
            "and the one field that is still missing is the one that is offered",
        );
    }

    /// The verdict is reached once and stands: a second reading of steps that
    /// say something else does not move it.
    #[tokio::test]
    async fn the_mode_is_settled_by_the_first_reading_and_by_no_other() {
        let unmet = StepsView {
            dependencies: true,
            accounts: false,
            git: true,
        };
        let met = StepsView {
            dependencies: true,
            accounts: true,
            git: true,
        };

        let onboarding = Onboarding::probing(
            Machine::stated(
                Platform::Linux,
                OsString::new(),
                OsString::new(),
                None,
                None,
                &Environment::default(),
            ),
            None,
        );

        assert!(onboarding.mode(unmet).await, "the objective was not met");
        assert!(
            onboarding.mode(met).await,
            "and a Profile made since is a step that ticks under a mode that stays on",
        );
    }

    /// And the wizard finishing is the one thing inside a run that takes it
    /// off.
    #[tokio::test]
    async fn the_wizard_finishing_takes_the_mode_off_for_the_rest_of_the_run() {
        let unmet = StepsView {
            dependencies: false,
            accounts: false,
            git: false,
        };

        let onboarding = Onboarding::probing(
            Machine::stated(
                Platform::Linux,
                OsString::new(),
                OsString::new(),
                None,
                None,
                &Environment::default(),
            ),
            None,
        );

        assert!(onboarding.mode(unmet).await);

        onboarding.finished();

        assert!(
            !onboarding.mode(unmet).await,
            "the steps still stand unmet, and the mode is off all the same: what \
             the wizard saved is what the next start will read",
        );
    }
}
