//! Installing what the wizard's first step is missing: what the ticked rows
//! come to as commands, and the run that raises them.
//!
//! **The wizard used to draw a command per row and wait.** Every tab of that is
//! still written down — see `web/src/setup/instructions.ts` — because a machine
//! Verkstead cannot install on has to say what to type instead. What this
//! module is, is the other answer: the human ticks the rows they want, presses
//! Next once, and Verkstead installs them.
//!
//! **One press is one dialog.** Every package this distribution carries goes
//! into one command — `apt-get install -y bubblewrap git nodejs npm`, with the
//! `npm install -g` for a ticked npm harness joined onto the same shell line —
//! raised once through the [`Elevate`] handle whatever started this server
//! handed over. A second password dialog for a second package would be a wizard
//! nobody finishes.
//!
//! **The privilege is the app's rather than this server's.** Nothing here runs
//! `sudo` and nothing here is installed setuid: the handle is the platform's own
//! password dialog put in front of one command, and a server handed none — a
//! unit file's, with nobody at that machine to ask — installs nothing at all and
//! says so on every ticked row. See [`crate::remote::Elevate`], which is the
//! same seam the Tailscale operator grant is taken through.
//!
//! **A run is a sequence of units, and a unit is one command.** The elevated
//! batch is the first of them, and a unit covers the rows it installs — so a
//! refusal lands on every row of that unit and on nothing else, and a row this
//! machine has no command for is failed before anything is raised, with a
//! sentence saying so where the hint screen draws it.
//!
//! **And the two rows that are not a package run as the user, after it.** Claude
//! Code is Anthropic's own installer and Grok Build is xAI's — see [`CLAUDE`]
//! and [`GROK`] — and each of them installs under the home of whoever ran it, so
//! raising either behind the password dialog would leave a harness in root's
//! home where no session could reach it. Neither needs the privilege in the
//! first place. What they land in is written to `session_path` instead, which is
//! what puts the row on the next probe and the program on the next session's
//! `PATH` with no shell profile edited and nothing restarted — see
//! [`crate::sandbox::installed_into`].
//!
//! **A Mac is Homebrew's, and Homebrew refuses to run as root.** So there the
//! order is the other way up: every ticked row is a `brew install` of its own,
//! run as the user, and the one thing that is ever raised is the step that
//! makes Homebrew's prefix on a Mac that has no `brew` yet. That is the whole of
//! what Homebrew's own installer would have called `sudo` for, so the installer
//! after it runs as the user too and asks for nothing — see [`homebrew`], and
//! [`Chain`], which is what makes a prefix nobody could make fail every `brew`
//! line behind it rather than each of them separately.
//!
//! **And a Windows machine raises everything.** There the dialog is UAC and
//! what is behind it is the same user with their administrator token, so
//! `%USERPROFILE%` is still the human's own: a vendor's installer raised there
//! lands where a session looks, which is what made the Unix arm keep those two
//! out of the dialog in the first place. So every ticked row is one raised unit
//! — `winget` for the packages, `npm install -g` for the two harnesses that
//! come off npm, and the vendors' PowerShell installers — and the sandbox row is
//! the one thing here that is not an install at all: it is the local account
//! this Data Directory's sessions run as, made by Verkstead's own elevated verb
//! and then granted on the named pipe without anything being restarted. See
//! [`windows`], and [`crate::pipe::granted`].
//!
//! **Nothing runs while nobody is looking.** A run is started by a press and
//! ends by itself; between presses the wizard is the probe it always was — a
//! `PATH` walked and one `bwrap` run — which is also what says a row that was
//! installing has landed. There is no sweep here and nothing is written down: a
//! run belongs to the life of this server, and a restart has none.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use base64::Engine;
use verkstead_render::{Dependency, Distro, InstallState, RunPhase, RunView};

use super::{Machine, SHELL};
use crate::platform::Platform;
use crate::remote::{Elevate, Raised};
use crate::settings::Settings;

/// Every row of the step, in the order the wizard draws them.
///
/// Which is the order a run works through them in and the order the packages
/// are named in. The same order [`Machine::rows`] probes in, held to it by
/// `the_rows_are_worked_in_the_order_the_wizard_draws_them`: a run installing in
/// one order under a page listing in another would be two accounts of one
/// press.
const ROWS: &[Dependency] = &[
    Dependency::Sandbox,
    Dependency::Git,
    Dependency::Claude,
    Dependency::Codex,
    Dependency::Grok,
    Dependency::OpenCode,
    Dependency::Gh,
];

/// The program an npm harness is installed with, and the name whose absence
/// puts node into the elevated batch.
const NPM: &str = "npm";

/// What that absence adds, which is the same two packages on all three package
/// managers.
const NODE: &[&str] = &["nodejs", "npm"];

/// What a skipped row reads.
const CANCELLED: &str = "cancelled";

/// Anthropic's own installer, which is what a ticked Claude row runs.
///
/// **The command the wizard already shows**, and the install that stays
/// current: a distribution's `claude` can be too old to connect at all, which is
/// why the tab leads with this one and why the npm package — `npm install -g
/// @anthropic-ai/claude-code` — is not what the run installs. See
/// `web/src/setup/instructions.ts`, where the same line is written for the human
/// who has to type it on a machine Verkstead cannot install on.
const CLAUDE: Vendor = Vendor {
    who: "Anthropic",
    line: "curl -fsSL https://claude.ai/install.sh | bash",
    lands: ".local/bin",
};

/// And xAI's, which is what a ticked Grok Build row runs.
///
/// The line and the directory are the ones the script its install page serves
/// really uses: `$HOME/.grok/bin` where `GROK_BIN_DIR` says nothing, with the
/// binary linked from a downloads directory beside it. Anywhere outside that
/// home is somewhere an unelevated run could not write in any case, which is why
/// the directory under it is the one worth putting on a session's `PATH`.
const GROK: Vendor = Vendor {
    who: "xAI",
    line: "curl -fsSL https://x.ai/cli/install.sh | bash",
    lands: ".grok/bin",
};

/// Anthropic's installer on Windows, which is the PowerShell one.
///
/// The same install and the same landing place said the way this platform
/// spells them: `%USERPROFILE%\.local\bin`, which is what a ticked row writes to
/// `session_path`. The line is the one the wizard's own Windows tab shows — see
/// `web/src/setup/instructions.ts`.
const CLAUDE_ON_WINDOWS: Vendor = Vendor {
    who: "Anthropic",
    line: "irm https://claude.ai/install.ps1 | iex",
    lands: r".local\bin",
};

/// And xAI's, which is the PowerShell one too.
const GROK_ON_WINDOWS: Vendor = Vendor {
    who: "xAI",
    line: "irm https://x.ai/cli/install.ps1 | iex",
    lands: r".grok\bin",
};

/// What installs a package on Windows: the word in front of every id, and what
/// goes after it.
///
/// **Answering no questions**, which is the same rule the three Linux package
/// managers' `-y` is here for: an install behind a password dialog has nobody
/// at a terminal to say yes. `--exact` because `--id` is a substring match
/// without it and two matches is a refusal; the two agreements because winget
/// stops for the source's on its first run and for a package's whenever there
/// is one; and `--silent` because an installer drawing its own windows on a
/// machine nobody is looking at is an install that never finishes.
///
/// The hint screen's Windows tab shows the bare line instead — a human at a
/// terminal is exactly who can answer what this has to answer in advance. See
/// `web/src/setup/instructions.ts`.
const WINGET: &str = "winget install --id";
const UNATTENDED: &str = "--exact --silent --accept-package-agreements --accept-source-agreements";

/// The node winget carries, which is what an npm harness is installed under
/// where this machine has no `npm`.
const NODE_ON_WINDOWS: &str = "OpenJS.NodeJS";

/// And the verb that makes the account a Windows session runs as, which is the
/// whole of what the sandbox row there installs.
///
/// Verkstead's own binary and an administrator's call (ADR-0014, *Amended: the
/// Sandbox is an account*) — which is what the runas arm is: the elevated
/// terminal the doc used to send somebody to, raised for them instead.
const MAKE_THE_ACCOUNT: &str = "session-account create --data-dir";

/// The program every install on a Mac goes through, and the name whose absence
/// puts Homebrew's own two units in front of them.
const BREW: &str = "brew";

/// Where Homebrew installs, worked out by the machine the line runs on.
///
/// **Which prefix it is, is that machine's own word.** `/opt/homebrew` is Apple
/// silicon's and `/usr/local` is Intel's, and nothing on this side of the dialog
/// knows which Mac it is talking to: a server built for one architecture may be
/// the one running under Rosetta on the other. So the line asks `uname` where it
/// lands, which is what Homebrew's own installer does.
const WHERE: &str = "prefix=/usr/local; [ \"$(uname -m)\" = arm64 ] && prefix=/opt/homebrew";

/// And Homebrew's own installer, run as the user over the prefix the step in
/// front of it made.
///
/// `NONINTERACTIVE=1` because there is nobody at that machine to press return:
/// the human who pressed Next is on a phone on the tailnet, and the one thing
/// the installer would stop to ask about — the `sudo` for its prefix — has been
/// answered by the step in front of it.
///
/// The same script the hint screen's macOS tab hands a human to run, where this
/// could not be run for them — with the interactive line Homebrew publishes, a
/// human at a terminal being who that one is for. See
/// `web/src/setup/instructions.ts`.
const HOMEBREW: &str = "NONINTERACTIVE=1 bash -c \"$(curl -fsSL \
                        https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)\"";

/// What a unit that could not even be started says, a command with no output to
/// quote being a row with nothing else to carry.
const FAILED_SILENTLY: &str = "the installer failed and said nothing";

/// What a server with no way to raise a dialog says on every ticked row.
const NO_DIALOG: &str = "Verkstead has no way to ask this machine for a password: the desktop \
                         app is what raises the dialog, and this server was not started by it.";

/// And what a Mac with no Homebrew and nobody to hand a prefix to says on every
/// ticked row Homebrew would have installed.
const NO_USER: &str = "Homebrew's prefix has to belong to somebody, and this server was started \
                       without a name for whoever is running it.";

/// And what the Windows sandbox row says where this process cannot name the
/// image the elevated verb is a verb of.
///
/// Which is a server that cannot say what it is running — see
/// [`Machine::verkstead`]. The account is still Verkstead's own binary's to
/// make, so the row goes to the hint screen with the command a human runs.
const NO_IMAGE: &str = "Verkstead cannot say which program it is running, so it has no binary to \
                        make the account with.";

/// And where it does not know which Data Directory the account would belong to,
/// which is a stated machine nobody pointed at one.
const NO_DATA_DIRECTORY: &str = "Verkstead was not told which Data Directory it keeps, so it \
                                 cannot say which account to make.";

/// How often the marker is glanced at while a dialog is up — see [`STARTED`].
const GLANCE: Duration = Duration::from_millis(200);

/// The file an elevated command touches before it does anything else, which is
/// how this side finds out the password dialog has been answered.
///
/// **There is nothing else to read.** Raising a command is one blocking call
/// that returns when the command is over, so between the press and that return
/// this server cannot otherwise tell a human staring at a password prompt from a
/// package manager half way through unpacking. That difference is the whole of
/// what the status line is for, and the human reading it may be on a phone whose
/// screen the dialog is *not* on — so it is worth one file: the command writes
/// it as its first act, and a glance every [`GLANCE`] moves the line on.
const STARTED: &str = "started";

/// The install run, and the way this process asks for a privilege.
///
/// One run at a time, and it is this server's: a press while a run is going is
/// refused rather than queued — see [`Refusal`].
#[derive(Debug)]
pub(crate) struct Installer {
    /// How one command is raised, where whatever started this server handed a
    /// way over.
    ///
    /// `None` is every server but the desktop app's: there is nobody at that
    /// machine to put a dialog in front of, so a run started there installs
    /// nothing and every ticked row says why.
    escalation: Option<Arc<dyn Elevate>>,

    /// What is going on, or the last thing that was. `None` until the first
    /// press.
    run: Mutex<Option<Arc<Run>>>,
}

/// One run: what each ticked row has come to, and what the status line is
/// about.
///
/// Shared between the thread working through the units and every read of the
/// wizard, which is what the lock is for. The cancel flag is beside that lock
/// rather than under it, being the one thing written from outside while the run
/// is holding it.
#[derive(Debug)]
struct Run {
    going: Mutex<Going>,

    /// What this machine calls itself, for the one sentence that names it.
    hostname: String,

    /// Whether Cancel has been pressed. Read between units and never inside
    /// one: what is already running is left to finish.
    cancelled: AtomicBool,
}

/// The half of a run a reading is drawn from.
#[derive(Debug)]
struct Going {
    /// Every ticked row and where it has got to, in [`ROWS`] order.
    rows: Vec<(Dependency, Progress)>,

    phase: RunPhase,
    status: String,
}

/// Where one ticked row has got to.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Progress {
    /// Its unit has not started.
    Waiting,

    /// Its unit is running.
    Installing,

    /// Its unit is over and said it worked. What the row reads from here is the
    /// probe's business, which is the point of leaving it there: a command that
    /// exited zero and left no program behind is an absent row rather than a
    /// tick.
    Installed,

    /// Its unit is over and the thing is not installed.
    Failed(String),
}

/// Why a press was not taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Refusal {
    /// The wizard is not the page there is: this server came up with the
    /// objective met, or the wizard has already finished in this run.
    Over,

    /// A run is going. A press that would start a second one is refused, and
    /// what the page does about it is read the run it already has.
    Going,
}

/// One command of a run, and the rows it is for.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Unit {
    /// What to run, as one shell line.
    line: String,

    /// The rows it installs: what it comes to lands on every one of them, and
    /// on nothing else.
    covers: Vec<Dependency>,

    /// And what the status line says while it is running.
    doing: String,

    /// Whether it goes behind the password dialog or runs as the user.
    how: How,

    /// And the directory whatever it installs lands in, where it is an installer
    /// that has one of its own.
    ///
    /// Written to `session_path` once the unit has run and left something there,
    /// which is what puts the row on the next probe — see [`landing`]. Nothing
    /// for the elevated batch, a package manager installing onto the machine's
    /// own floor, which every session's `PATH` already ends with.
    lands: Option<PathBuf>,

    /// And what this unit is part of, where it is part of anything: a unit that
    /// fails takes every later unit of its own chain with it — see [`Chain`].
    chain: Option<Chain>,

    /// And the local account it makes, where what it makes is one.
    ///
    /// **The Windows sandbox row and nothing else.** The named pipe a sandboxed
    /// Windows session asks through is opened granting the account this Data
    /// Directory's sessions run as, and a server that came up before there was
    /// one is granting nobody — so the unit that makes the account says which
    /// one it made, and the pipe is opened again granting it. See
    /// [`crate::pipe::granted`], which is what makes a session started a minute
    /// later able to ask without anything having restarted.
    regrants: Option<String>,
}

/// A run of units that stand or fall together.
///
/// **A unit that installs the program the units behind it are run with.** Every
/// install on a Mac is a `brew install`, so a Mac without `brew` has two units
/// in front of the ticked rows — the prefix, and the installer that fills it —
/// and a row installed by a `brew` that was never installed is a command that
/// would fail saying nothing anybody can act on. So the first of the chain that
/// fails fails the rest of it, in the words it failed in, and nothing after it
/// is run. A unit outside the chain is untouched: Grok Build's installer wants
/// no Homebrew and is nobody's business but its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Chain {
    /// Homebrew: the prefix, the installer, and every `brew install` after
    /// them.
    Homebrew,

    /// And node on Windows: the `winget` that puts `npm` on this machine, and
    /// every `npm install -g` behind it — which would otherwise each fail
    /// saying the machine has no `npm`, which is the failure in front of them
    /// said worse.
    Node,
}

/// How a unit is run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum How {
    /// Behind the platform's own password dialog, which is where a package
    /// manager writing into `/usr` has to go.
    Raised,

    /// Or as the user this server runs as, which is where an installer writing
    /// under that user's home belongs — see [`Vendor`].
    AsTheUser,
}

/// A vendor's own installer: the command a ticked row runs, and where it puts
/// what it installs.
///
/// **Two of the seven rows are one of these rather than a package.** Neither
/// Claude Code nor Grok Build is carried by a distribution under a name that is
/// really the vendor's, so what a tick runs is the script the vendor publishes —
/// the same one the hint screen tells a human to run where Verkstead cannot run
/// it for them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Vendor {
    /// Whose installer it is, which is the whole of what the status line says
    /// while it runs: *Running Anthropic's installer*.
    who: &'static str,

    /// The line, as one shell command.
    line: &'static str,

    /// And where it lands what it installs, under the home it ran as.
    lands: &'static str,
}

/// What a unit that runs as the user is given: the server's own environment,
/// with the two values the machine is the word on put back.
///
/// Read off the machine at the press rather than out of the process, for the
/// reason everything else here is: a stated machine's home is a test's own
/// directory and its `PATH` a test's own list, and an installer run against this
/// box's would be a suite installing a harness on whoever ran it.
#[derive(Debug, Clone)]
struct AsTheUser {
    /// The home the installers land under — `None` on a machine that names
    /// none, which is a machine no vendor installer is planned for at all.
    home: Option<PathBuf>,

    /// And the `PATH` a session searches, which is what the line reaches for:
    /// the directories Verkstead has installed into lead it, so an installer
    /// that looks for what it is upgrading finds what a session would.
    path: OsString,
}

/// What a press comes to: the commands to raise, and the rows nothing here can
/// install.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Plan {
    units: Vec<Unit>,

    /// Each with the sentence that goes under it on the hint screen. They fail
    /// before anything is raised: a machine Verkstead has no command for is one
    /// to say so about rather than one to put a password dialog on.
    beyond: Vec<(Dependency, String)>,

    /// And whose shell a unit's line is one of, which is the one thing about
    /// running it that is not written on the unit: `/bin/sh` on the two Unixes
    /// and PowerShell on Windows — see [`raising`].
    platform: Platform,
}

impl Installer {
    /// An installer raising through `escalation`, with nothing running.
    pub(crate) fn raising(escalation: Option<Arc<dyn Elevate>>) -> Installer {
        Installer {
            escalation,
            run: Mutex::new(None),
        }
    }

    /// Start a run over `ticked`, and return as soon as it is going.
    ///
    /// **Blocking, and briefly.** What happens here is a `PATH` walk for `npm`
    /// and a thread spawned; the waiting on a human reading a dialog is that
    /// thread's.
    ///
    /// `settings` is what a unit that lands in a directory writes `session_path`
    /// to — see [`landing`].
    pub(crate) fn start(
        &self,
        machine: &Machine,
        settings: &Settings,
        ticked: &[Dependency],
    ) -> Result<(), Refusal> {
        let mut held = self.run.lock().expect("the install run's lock");

        if held.as_ref().is_some_and(|run| run.going()) {
            return Err(Refusal::Going);
        }

        let ticked = ordered(ticked);

        let plan = match self.escalation.as_ref() {
            Some(_) => plan(machine, &ticked),
            None => nothing(machine, &ticked, NO_DIALOG),
        };

        let run = Arc::new(Run::of(&plan, machine.hostname()));
        *held = Some(run.clone());

        // The lock goes back before the run starts: what is under it is read by
        // every wizard poll, and a run holding it across a password dialog would
        // be a page that stopped answering the moment it had something to draw.
        drop(held);

        // A plan with no command in it is over before it started — every row it
        // has is already failed — so there is nothing for a thread to do, and a
        // thread that ran would only be a race with the reading that answers
        // this press.
        let Some(escalation) = self.escalation.clone().filter(|_| !plan.units.is_empty()) else {
            run.over();
            return Ok(());
        };

        let user = AsTheUser::of(machine);
        let settings = settings.clone();

        std::thread::spawn(move || work(&run, &plan, escalation.as_ref(), &settings, &user));

        Ok(())
    }

    /// Cancel whatever is going: the unit under way finishes, and the units
    /// after it are skipped.
    ///
    /// Nothing at all where nothing is going, which is a press the page made
    /// about a run that ended while it was in flight.
    pub(crate) fn cancel(&self) {
        let held = self.run.lock().expect("the install run's lock");

        if let Some(run) = held.as_ref().filter(|run| run.going()) {
            run.cancelled.store(true, Ordering::SeqCst);
        }
    }

    /// The run as the wizard reads it, and what each row it holds is doing.
    ///
    /// Nothing where no press has been made in this server's life.
    pub(crate) fn reading(&self) -> Option<(RunView, Vec<(Dependency, InstallState)>)> {
        let held = self.run.lock().expect("the install run's lock");

        held.as_ref().map(|run| run.reading())
    }
}

impl Run {
    /// A run of `plan`, with every row it cannot install already failed.
    fn of(plan: &Plan, hostname: &str) -> Run {
        let mut rows: Vec<(Dependency, Progress)> = plan
            .units
            .iter()
            .flat_map(|unit| unit.covers.iter().copied())
            .map(|dependency| (dependency, Progress::Waiting))
            .chain(
                plan.beyond
                    .iter()
                    .map(|(dependency, why)| (*dependency, Progress::Failed(why.clone()))),
            )
            .collect();

        rows.sort_by_key(|(dependency, _)| place(*dependency));

        Run {
            going: Mutex::new(Going {
                rows,
                phase: RunPhase::Asking,
                status: asking(hostname),
            }),
            hostname: hostname.to_owned(),
            cancelled: AtomicBool::new(false),
        }
    }

    /// Whether this run is still working.
    fn going(&self) -> bool {
        self.held().phase != RunPhase::Done
    }

    /// The rows of `unit` are this run's business now, and the dialog is up.
    fn asking(&self, unit: &Unit) {
        let mut going = self.held();

        going.phase = RunPhase::Asking;
        going.status = asking(&self.hostname);
        going.set(&unit.covers, Progress::Installing);
    }

    /// The rows of `unit` are this run's business now, and there is nothing to
    /// wait for: it runs as the user, so there is no dialog between the press
    /// and the install.
    fn running(&self, unit: &Unit) {
        self.held().set(&unit.covers, Progress::Installing);
        self.installing(&unit.doing);
    }

    /// The dialog has been answered and the command is running.
    fn installing(&self, doing: &str) {
        let mut going = self.held();

        going.phase = RunPhase::Installing;
        going.status = doing.to_owned();
    }

    /// A unit is over: every row it covered reads `landed`.
    fn landed(&self, unit: &Unit, landed: Progress) {
        self.held().set(&unit.covers, landed);
    }

    /// And the run is over: whatever is still waiting was skipped, and the
    /// status line says what the whole of it came to.
    fn over(&self) {
        let mut going = self.held();

        for (_, progress) in going.rows.iter_mut() {
            if *progress == Progress::Waiting {
                *progress = Progress::Failed(CANCELLED.to_owned());
            }
        }

        let failed = going
            .rows
            .iter()
            .filter(|(_, progress)| matches!(progress, Progress::Failed(_)))
            .count();

        going.phase = RunPhase::Done;
        going.status = match failed {
            0 => "Everything that was ticked is installed".to_owned(),
            failed => format!("{failed} of {} could not be installed", going.rows.len()),
        };
    }

    /// What the wizard draws of it.
    fn reading(&self) -> (RunView, Vec<(Dependency, InstallState)>) {
        let going = self.held();

        let done = going
            .rows
            .iter()
            .filter(|(_, progress)| matches!(progress, Progress::Installed | Progress::Failed(_)))
            .count();

        let view = RunView {
            phase: going.phase,
            status: going.status.clone(),
            done: done as u32,
            total: going.rows.len() as u32,

            // Only while there is still something for it to stop: a run that is
            // over is not cancelling anything, whatever was pressed on the way.
            cancelling: self.cancelled.load(Ordering::SeqCst) && going.phase != RunPhase::Done,
        };

        let rows = going
            .rows
            .iter()
            .map(|(dependency, progress)| (*dependency, progress.shown()))
            .collect();

        (view, rows)
    }

    /// The half a reading is drawn from, locked.
    fn held(&self) -> std::sync::MutexGuard<'_, Going> {
        self.going.lock().expect("a run's lock")
    }
}

impl AsTheUser {
    /// What `machine` says a unit running as the user is given.
    fn of(machine: &Machine) -> AsTheUser {
        AsTheUser {
            home: machine.home().map(Path::to_owned),
            path: machine.path().into_owned(),
        }
    }
}

impl Going {
    /// Every row in `covers` reads `progress`.
    fn set(&mut self, covers: &[Dependency], progress: Progress) {
        for (dependency, held) in self.rows.iter_mut() {
            if covers.contains(dependency) {
                *held = progress.clone();
            }
        }
    }
}

impl Progress {
    /// The row's install state as the viewer receives it.
    ///
    /// A row that has landed reads as nothing, the way one still waiting does:
    /// what says whether the program is there is the probe beside it, made
    /// afresh on every read — see [`verkstead_render::InstallState`].
    fn shown(&self) -> InstallState {
        match self {
            Progress::Waiting | Progress::Installed => InstallState::Idle,
            Progress::Installing => InstallState::Installing,
            Progress::Failed(why) => InstallState::Failed { why: why.clone() },
        }
    }
}

/// Work through `plan`, one unit at a time.
///
/// On a thread of its own, because the first thing the elevated unit does is
/// wait for somebody to read a password dialog, and how long that takes is
/// theirs.
fn work(
    run: &Arc<Run>,
    plan: &Plan,
    escalation: &dyn Elevate,
    settings: &Settings,
    user: &AsTheUser,
) {
    // The one directory a run makes, and it goes when the run does: what is in
    // it is the marker each raised unit touches, which is how the status line
    // finds out the dialog was answered — see [`STARTED`]. A machine with
    // nowhere to put one is a run whose status line stays on the dialog, and
    // nothing worse.
    let marking = tempfile::Builder::new()
        .prefix("verkstead-install")
        .tempdir()
        .ok();

    // The chain that has already failed, and what it failed with: every unit of
    // it after this is that failure's too — see [`Chain`].
    let mut broken: Option<(Chain, String)> = None;

    for (number, unit) in plan.units.iter().enumerate() {
        // Between units and never inside one: a package manager that is already
        // running is left to finish, a machine half way through an unpack being
        // worse than one that finished the unpack nobody wanted.
        if run.cancelled.load(Ordering::SeqCst) {
            break;
        }

        // And a unit whose chain is broken is not run at all: what it needed was
        // the unit that failed, so what it would say is the first failure said
        // better.
        if let Some((_, why)) = broken
            .as_ref()
            .filter(|(chain, _)| unit.chain == Some(*chain))
        {
            run.landed(unit, Progress::Failed(why.clone()));
            continue;
        }

        let landed = match unit.how {
            How::Raised => {
                let marker = marking
                    .as_ref()
                    .map(|dir| dir.path().join(format!("{STARTED}-{number}")));

                run.asking(unit);
                raised(run, plan.platform, unit, marker.as_deref(), escalation)
            }

            How::AsTheUser => {
                run.running(unit);
                as_the_user(&unit.line, user)
            }
        };

        if landed == Progress::Installed {
            landing(unit, settings);
            regranted(unit);
        }

        if let (Progress::Failed(why), Some(chain)) = (&landed, unit.chain) {
            broken = Some((chain, why.clone()));
        }

        run.landed(unit, landed);
    }

    run.over();
}

/// Put `unit` behind the platform's password dialog, moving the status line on
/// where `marker` appears.
fn raised(
    run: &Arc<Run>,
    platform: Platform,
    unit: &Unit,
    marker: Option<&Path>,
    escalation: &dyn Elevate,
) -> Progress {
    let watching = marker.map(|marker| watch(run, marker.to_owned(), unit.doing.clone()));

    let raised = escalation.raise(&raising(platform, &unit.line, marker));

    if let Some((over, watching)) = watching {
        over.store(true, Ordering::SeqCst);
        let _ = watching.join();
    }

    match raised {
        Raised::Done => Progress::Installed,
        Raised::Refused { why } => Progress::Failed(why),
    }
}

/// And run `line` as the user this server runs as, which is what a vendor's own
/// installer is.
///
/// **The server's own environment, with `HOME` and `PATH` as the machine reads
/// them** — see [`AsTheUser`]. What it exited with is the whole of the verdict,
/// and what it printed on standard error is what the row carries: an installer
/// that could not reach the network has said why in one line, and a sentence of
/// Verkstead's own over the top of it would be a worse account of the same
/// thing.
fn as_the_user(line: &str, user: &AsTheUser) -> Progress {
    let mut running = std::process::Command::new(SHELL);

    running.arg("-c").arg(line).env("PATH", &user.path);

    if let Some(home) = &user.home {
        running.env("HOME", home);
    }

    match running.output() {
        Ok(told) if told.status.success() => Progress::Installed,
        Ok(told) => Progress::Failed(first_line(&told.stderr)),
        Err(error) => Progress::Failed(error.to_string()),
    }
}

/// The directory `unit` installed into goes on `session_path`, where it really
/// landed something.
///
/// **Which is what makes the row go present without a restart**: a session's
/// `PATH` is composed with the list this grows, so the very next probe walks the
/// directory the installer wrote into and finds the program there — no shell
/// profile to edit, and no second start of the server. See
/// [`crate::sandbox::installed_into`], which writes `config.yaml` and the held
/// list together.
///
/// **Only a directory that is there**, because a command that exited zero and
/// left nothing behind has installed nothing: writing its intended landing place
/// down would be `config.yaml` carrying a directory nobody ever made.
fn landing(unit: &Unit, settings: &Settings) {
    let Some(directory) = unit.lands.as_deref().filter(|lands| lands.is_dir()) else {
        return;
    };

    if let Err(error) = crate::sandbox::installed_into(settings, directory) {
        tracing::warn!(
            error = ?error,
            directory = %directory.display(),
            "an install landed in a directory that could not be written to \
             `session_path` — a session's `PATH` will not name it until it is",
        );
    }
}

/// And the named pipe is opened again granting the account `unit` made.
///
/// **Which is what makes the row worth pressing at all.** A Windows server that
/// came up before there was an account opened its pipe granting nobody, and a
/// session of the account just made would be refused by it — so the SID is
/// resolved here, the moment the verb has run, and the listener opens the pipe
/// again with it. Nothing is restarted and no session has started yet.
///
/// A name the machine will not resolve a moment after the verb reported making
/// it is a log line and nothing else: the account is there, so the row ticks
/// and sessions start, and it is the pipe alone that goes on granting nobody
/// until this server is next started.
#[cfg(windows)]
fn regranted(unit: &Unit) {
    let Some(account) = unit.regrants.as_deref() else {
        return;
    };

    match crate::sandbox::account::machine::sid_of(account) {
        Ok(sid) => crate::pipe::granted(sid.text()),
        Err(why) => tracing::warn!(
            account,
            why,
            "the session account was made and this machine will not say what its SID is, so \
             the named pipe goes on granting nobody until Verkstead is started again",
        ),
    }
}

/// And nothing anywhere else: no other platform opens a pipe, and no other
/// platform's plan makes an account.
#[cfg(not(windows))]
fn regranted(_: &Unit) {}

/// The first thing a command that failed printed on standard error, which is
/// what its rows carry.
fn first_line(stderr: &[u8]) -> String {
    String::from_utf8_lossy(stderr)
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or(FAILED_SILENTLY)
        .to_owned()
}

/// Watch for `marker` until the unit is over, moving the status line on where
/// it appears.
///
/// A thread rather than a task, because what it sits beside is a blocking call
/// on a thread of its own: there is no runtime in reach here to spawn anything
/// onto.
fn watch(run: &Arc<Run>, marker: PathBuf, doing: String) -> (Arc<AtomicBool>, JoinHandle<()>) {
    let over = Arc::new(AtomicBool::new(false));

    let watching = std::thread::spawn({
        let over = over.clone();
        let run = run.clone();

        move || {
            while !over.load(Ordering::SeqCst) {
                if marker.exists() {
                    run.installing(&doing);
                    return;
                }

                std::thread::sleep(GLANCE);
            }
        }
    });

    (over, watching)
}

/// What `ticked` comes to on this machine: the commands, and the rows nothing
/// here installs.
///
/// **Three shapes, and which one a machine is, is the platform's answer rather
/// than a preference.** A Linux installs out of the archive its distribution
/// carries, raised once; a Mac installs out of Homebrew, raised never — see
/// [`homebrew`]; and a Windows raises every ticked row on its own — see
/// [`windows`].
fn plan(machine: &Machine, ticked: &[Dependency]) -> Plan {
    let distro = machine.distro();

    if distro == Distro::MacOs {
        return homebrew(machine, ticked);
    }

    if distro == Distro::Windows {
        return windows(machine, ticked);
    }

    let Some(packager) = packager(distro) else {
        return nothing(machine, ticked, &no_command(distro));
    };

    packages(machine, ticked, packager)
}

/// Every ticked row on a machine whose archive carries what it is asking for:
/// the one raised command, and the vendors' own installers after it.
fn packages(machine: &Machine, ticked: &[Dependency], packager: Packager) -> Plan {
    let mut packages: Vec<&str> = Vec::new();
    let mut from_npm: Vec<&str> = Vec::new();
    let mut covers: Vec<Dependency> = Vec::new();
    let mut vendors: Vec<(Dependency, Vendor, PathBuf)> = Vec::new();
    let mut beyond: Vec<(Dependency, String)> = Vec::new();

    for row in ticked {
        match installs(packager, *row) {
            Installs::Package(package) => {
                packages.push(package);
                covers.push(*row);
            }

            Installs::FromNpm(package) => {
                from_npm.push(package);
                covers.push(*row);
            }

            // Where the installer would land it is under the home this server
            // runs as, so a machine that names none is one neither of these can
            // be planned for: what it installed would be somewhere nobody could
            // say, and a directory nobody can name is one no session's `PATH`
            // could be composed with.
            Installs::Vendor(vendor) => match machine.home() {
                Some(home) => vendors.push((*row, vendor, home.join(vendor.lands))),
                None => beyond.push((*row, no_home(*row))),
            },
        }
    }

    // Node where an npm harness was ticked and this machine has no `npm`, which
    // is the one thing the elevated batch installs that nobody asked for: an
    // `npm install -g` on a machine with no npm is a line that fails, and a row
    // of its own for something no session ever runs would be a row about
    // plumbing.
    if !from_npm.is_empty() && machine.found(NPM).is_none() {
        packages.extend(NODE);
    }

    let mut lines: Vec<String> = Vec::new();

    if !packages.is_empty() {
        lines.push(format!(
            "{} {}",
            packager.install.join(" "),
            packages.join(" ")
        ));
    }

    if !from_npm.is_empty() {
        lines.push(format!("{NPM} install -g {}", from_npm.join(" ")));
    }

    // Joined rather than raised apart, so that one press is one dialog — and
    // with `&&`, so that a package manager that failed is not followed by an
    // `npm` that was going to fail too.
    let batch = (!lines.is_empty()).then(|| Unit {
        line: lines.join(" && "),
        covers,
        doing: format!(
            "Installing {}",
            packages
                .iter()
                .chain(from_npm.iter())
                .copied()
                .collect::<Vec<&str>>()
                .join(", ")
        ),
        how: How::Raised,
        lands: None,
        chain: None,
        regrants: None,
    });

    // And the vendors' own installers after it, one unit apiece and none of them
    // elevated: the dialog is the package manager's business, and a run that put
    // a second one in front of a `curl | bash` writing under this user's home
    // would be asking for a privilege to do something that needs none.
    let units = batch
        .into_iter()
        .chain(
            vendors
                .into_iter()
                .map(|(row, vendor, lands)| vendors_own(row, vendor, lands)),
        )
        .collect();

    Plan {
        units,
        beyond,
        platform: machine.platform,
    }
}

/// One unit running `vendor`'s own installer for `row`, as the user, landing in
/// `lands`.
///
/// The same unit on every platform that has one: what a vendor's installer is
/// is a line to run under this user's home, and neither the archive beside it
/// nor the Homebrew beside it has anything to say about that.
fn vendors_own(row: Dependency, vendor: Vendor, lands: PathBuf) -> Unit {
    Unit {
        line: vendor.line.to_owned(),
        covers: vec![row],
        doing: format!("Running {}'s installer", vendor.who),
        how: How::AsTheUser,
        lands: Some(lands),
        chain: None,
        regrants: None,
    }
}

/// And every ticked row on a Mac, which is Homebrew's.
///
/// **One unit per row, and each of them as the user**, Homebrew refusing to run
/// as root at all. Nothing lands anywhere worth writing down: both prefixes are
/// on the floor a Mac session's `PATH` is composed from — see
/// `sandbox::APPLE_PATH` — so the row goes present off the very next probe
/// without `session_path` being touched.
///
/// **And where there is no `brew` yet, two units in front of them.** The
/// elevated one makes the prefix and hands it over, which is the whole of what
/// Homebrew's installer would have raised a dialog for; the installer itself
/// runs as the user over a prefix it finds writable. Both are the same [`Chain`]
/// as the `brew` lines behind them, so a prefix nobody could make fails every
/// ticked row at once, in the words it failed in, and no installer is run at
/// all.
///
/// **The sandbox row is neither.** `sandbox-exec` is Apple's own and on every
/// Mac, so the row is ticked by the probe rather than by anybody — and a press
/// that named it anyway has nothing to install and nothing to say about it.
fn homebrew(machine: &Machine, ticked: &[Dependency]) -> Plan {
    let getting = getting(machine, ticked);

    let mut units: Vec<Unit> = Vec::new();
    let mut beyond: Vec<(Dependency, String)> = Vec::new();

    for row in ticked {
        match on_a_mac(*row) {
            OnAMac::Nothing => {}

            OnAMac::Brew(brew) => match &getting {
                Getting::Beyond(why) => beyond.push((*row, why.clone())),
                _ => units.push(brew.unit(*row)),
            },

            // The one row here that is not Homebrew's, and it is the one row
            // Homebrew has no cask for: xAI's own installer, under this user's
            // home the way it is on a Linux — see [`GROK`].
            OnAMac::Vendor(vendor) => match machine.home() {
                Some(home) => units.push(vendors_own(*row, vendor, home.join(vendor.lands))),
                None => beyond.push((*row, no_home(*row))),
            },
        }
    }

    if let Getting::First(first) = getting {
        units.splice(0..0, first);
    }

    Plan {
        units,
        beyond,
        platform: machine.platform,
    }
}

/// What a `brew install` on this machine wants before it would work.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Getting {
    /// Nothing: `brew` is there.
    Nothing,

    /// The prefix and the installer, in that order, in front of everything
    /// ticked.
    First(Vec<Unit>),

    /// Or nothing doing, in the words of why — which every ticked `brew` row
    /// carries to the hint screen instead.
    Beyond(String),
}

/// Which of the three this machine is.
///
/// Asked once per press rather than per row: what it comes to is a `PATH` walk
/// for `brew`, and seven walks for one answer would be six too many.
fn getting(machine: &Machine, ticked: &[Dependency]) -> Getting {
    // A press with no `brew` line in it wants no Homebrew: a ticked Grok Build
    // by itself is xAI's installer and nothing else, and a Mac that has never
    // heard of Homebrew is not a machine to install one on over it.
    if machine.found(BREW).is_some()
        || !ticked
            .iter()
            .any(|row| matches!(on_a_mac(*row), OnAMac::Brew(_)))
    {
        return Getting::Nothing;
    }

    // The prefix is made as root and handed to somebody, so a machine whose
    // environment names nobody is one there is no handing it to: a prefix left
    // owned by root is a Homebrew that asks for a password at every install,
    // from a server that has nobody to ask.
    let Some(user) = machine.user() else {
        return Getting::Beyond(NO_USER.to_owned());
    };

    Getting::First(vec![
        Unit {
            line: format!(
                "{WHERE}; mkdir -p \"$prefix\" && chmod ug=rwx \"$prefix\" && \
                 chgrp admin \"$prefix\" && chown {} \"$prefix\"",
                quoted(user),
            ),
            covers: Vec::new(),
            doing: "Making Homebrew's prefix".to_owned(),
            how: How::Raised,
            lands: None,
            chain: Some(Chain::Homebrew),
            regrants: None,
        },
        Unit {
            line: HOMEBREW.to_owned(),
            covers: Vec::new(),
            doing: "Installing Homebrew".to_owned(),
            how: How::AsTheUser,
            lands: None,
            chain: Some(Chain::Homebrew),
            regrants: None,
        },
    ])
}

/// What Homebrew calls one row, and which of its two shelves it is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Brew {
    /// The name to install.
    name: &'static str,

    /// And whether it is a cask — an application rather than a formula, which
    /// is how Homebrew carries the two harnesses that ship as one.
    cask: bool,
}

impl Brew {
    /// The unit that installs it for `row`.
    fn unit(&self, row: Dependency) -> Unit {
        Unit {
            line: format!(
                "{BREW} install {}{}",
                if self.cask { "--cask " } else { "" },
                self.name,
            ),
            covers: vec![row],
            doing: format!("Installing {}", self.name),
            how: How::AsTheUser,

            // Both Homebrew prefixes are on a Mac session's floor already, so
            // there is nothing here to write to `session_path` — see
            // [`homebrew`].
            lands: None,
            chain: Some(Chain::Homebrew),
            regrants: None,
        }
    }
}

/// How one row is installed on a Mac: Homebrew's, the vendor's own, or nothing
/// at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OnAMac {
    /// The sandbox row, and only it: `sandbox-exec` is Apple's own.
    Nothing,

    /// A formula or a cask — the same names the hint screen's macOS tab shows,
    /// which is where the human who has to type one reads them.
    Brew(Brew),

    /// Or the vendor's own installer, there being no Homebrew name that is
    /// really xAI's grok — see [`GROK`].
    Vendor(Vendor),
}

/// Which of the three one row is.
fn on_a_mac(dependency: Dependency) -> OnAMac {
    match dependency {
        Dependency::Sandbox => OnAMac::Nothing,

        Dependency::Git => OnAMac::Brew(Brew {
            name: "git",
            cask: false,
        }),
        Dependency::OpenCode => OnAMac::Brew(Brew {
            name: "opencode",
            cask: false,
        }),
        Dependency::Gh => OnAMac::Brew(Brew {
            name: "gh",
            cask: false,
        }),

        // The two that ship as applications rather than as formulae. Claude
        // Code's cask is the install a Mac session finds whichever way
        // Verkstead was started, which is why it is the row here and the
        // native installer is not: an app started from the Dock has launchd's
        // `PATH` rather than a shell's, and that one never names
        // `~/.local/bin`.
        Dependency::Claude => OnAMac::Brew(Brew {
            name: "claude-code",
            cask: true,
        }),
        Dependency::Codex => OnAMac::Brew(Brew {
            name: "codex",
            cask: true,
        }),

        Dependency::Grok => OnAMac::Vendor(GROK),
    }
}

/// And every ticked row on a Windows machine, which is one raised unit apiece.
///
/// **Everything goes behind the dialog here**, which is the other way up from
/// both Unixes and is what that dialog is: `Start-Process -Verb RunAs` raises
/// the *same* user with their administrator token, so `%USERPROFILE%` on the
/// far side of it is still the human's own. A vendor's installer raised there
/// lands where a session looks, which is the whole of what kept those two out
/// of the dialog on a Linux — see [`vendors_own`]. And a package manager
/// needs it: `winget` writing into Program Files is an administrator's call.
///
/// **One unit each rather than one batch.** `winget` takes one id at a time,
/// and the two harnesses that come off npm are their own line apiece, so
/// there is no batch to be made: what one press comes to is a dialog per
/// ticked row, which is what this platform has instead.
///
/// **And node in front of the npm rows** where this machine has no `npm`,
/// chained to them so that a `winget` nobody allowed fails them in its own
/// words rather than leaving each to fail for want of a program — see
/// [`Chain::Node`].
///
/// **The sandbox row is not an install at all.** It is the local account this
/// Data Directory's sessions run as, and what it runs is Verkstead's own
/// elevated verb — see [`the_account`]. A machine this server cannot name
/// itself on, or one it was never told the Data Directory of, is a row for the
/// hint screen with the line a human runs instead.
fn windows(machine: &Machine, ticked: &[Dependency]) -> Plan {
    let mut units: Vec<Unit> = Vec::new();
    let mut beyond: Vec<(Dependency, String)> = Vec::new();
    let mut from_npm = false;

    for row in ticked {
        match on_windows(*row) {
            OnWindows::Account => match the_account(machine) {
                Ok(unit) => units.push(unit),
                Err(why) => beyond.push((*row, why)),
            },

            OnWindows::Winget(id) => units.push(winget(vec![*row], id, None)),

            OnWindows::FromNpm(package) => {
                from_npm = true;

                units.push(Unit {
                    line: format!("{NPM} install -g {package}"),
                    covers: vec![*row],
                    doing: format!("Installing {package}"),
                    how: How::Raised,
                    lands: None,
                    chain: Some(Chain::Node),
                    regrants: None,
                });
            }

            // Under the home this machine names, which is `%USERPROFILE%` —
            // see [`crate::platform::home_dir`]. A server started without one
            // is a machine neither installer has anywhere to land in, exactly
            // as it is on a Linux.
            OnWindows::Vendor(vendor) => match machine.home() {
                Some(home) => units.push(Unit {
                    how: How::Raised,
                    ..vendors_own(*row, vendor, home.join(vendor.lands))
                }),
                None => beyond.push((*row, no_home(*row))),
            },
        }
    }

    // The node winget carries, where an npm harness was ticked and this machine
    // has no `npm`. In front of every ticked row rather than only in front of
    // those two, which is where Homebrew's own units go and for the same
    // reason: what is being installed first is what the rest is installed with.
    //
    // A row of nobody's, like Homebrew's two: what a session runs is the
    // harness, and a row for the plumbing under it would be a row about
    // plumbing.
    if from_npm && machine.found(NPM).is_none() {
        units.insert(0, winget(Vec::new(), NODE_ON_WINDOWS, Some(Chain::Node)));
    }

    Plan {
        units,
        beyond,
        platform: machine.platform,
    }
}

/// One `winget install` of `id`, for the rows it installs.
///
/// `covers` is empty for the one that installs node, which is a unit nobody's
/// row is about.
fn winget(covers: Vec<Dependency>, id: &str, chain: Option<Chain>) -> Unit {
    Unit {
        line: format!("{WINGET} {id} {UNATTENDED}"),
        covers,
        doing: format!("Installing {id}"),
        how: How::Raised,

        // A winget install lands on the machine's own floor, which every
        // session's `PATH` already ends with — so there is nothing here to
        // write to `session_path`.
        lands: None,
        chain,
        regrants: None,
    }
}

/// And the unit that makes the account a Windows session runs as, which is what
/// the sandbox row installs.
///
/// **Verkstead's own verb, raised.** Creating a local account is an
/// administrator's call (ADR-0014, *Amended: the Sandbox is an account*), which
/// is why it was a line to type in an elevated terminal — and the runas arm is
/// that terminal, opened for the human by the press. The Data Directory is said
/// outright rather than left to the environment, the account's name being a
/// fingerprint of it: a verb that resolved a different directory would make a
/// different account.
///
/// And it says which account it made, so that the named pipe is re-opened
/// granting it — see [`Unit::regrants`].
fn the_account(machine: &Machine) -> Result<Unit, String> {
    let Some(verkstead) = machine.verkstead() else {
        return Err(NO_IMAGE.to_owned());
    };

    let Some(data_dir) = machine.data_dir() else {
        return Err(NO_DATA_DIRECTORY.to_owned());
    };

    let account = crate::sandbox::account::named(data_dir);

    Ok(Unit {
        line: format!(
            "& {} {MAKE_THE_ACCOUNT} {}",
            said(&verkstead.to_string_lossy()),
            said(&data_dir.to_string_lossy()),
        ),
        covers: vec![Dependency::Sandbox],
        doing: format!("Making the local account {account} sessions run as"),
        how: How::Raised,
        lands: None,
        chain: None,
        regrants: Some(account),
    })
}

/// How one row is installed on Windows: the account itself, a package winget
/// carries, a package npm carries, or the vendor's own installer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OnWindows {
    /// The sandbox row, which on this platform is the local account this Data
    /// Directory's sessions run as — see [`the_account`].
    Account,

    /// A package winget carries, under the id it carries it as — the same ids
    /// the hint screen's Windows tab shows.
    Winget(&'static str),

    /// A package npm carries, installed globally under the node the unit in
    /// front of it made sure of.
    FromNpm(&'static str),

    /// Or the vendor's own installer, which is the PowerShell one here — see
    /// [`CLAUDE_ON_WINDOWS`] and [`GROK_ON_WINDOWS`].
    Vendor(Vendor),
}

/// Which of the four one row is.
///
/// **The same three-way split the other platforms have, with the account in
/// front of it.** Claude Code is the native installer here as it is on a Linux
/// — winget carries no Anthropic package — and the two npm harnesses are npm's
/// for want of anything else that is really theirs.
fn on_windows(dependency: Dependency) -> OnWindows {
    match dependency {
        Dependency::Sandbox => OnWindows::Account,

        Dependency::Git => OnWindows::Winget("Git.Git"),
        Dependency::Gh => OnWindows::Winget("GitHub.cli"),

        Dependency::Codex => OnWindows::FromNpm("@openai/codex"),
        Dependency::OpenCode => OnWindows::FromNpm("opencode-ai"),

        Dependency::Claude => OnWindows::Vendor(CLAUDE_ON_WINDOWS),
        Dependency::Grok => OnWindows::Vendor(GROK_ON_WINDOWS),
    }
}

/// A plan that installs nothing, every ticked row failed with the same
/// sentence.
fn nothing(machine: &Machine, ticked: &[Dependency], why: &str) -> Plan {
    Plan {
        units: Vec::new(),
        beyond: ticked
            .iter()
            .map(|dependency| (*dependency, why.to_owned()))
            .collect(),
        platform: machine.platform,
    }
}

/// One distribution's package manager: the words that install a list, and the
/// one name the three of them disagree about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Packager {
    /// What goes in front of the package names, answering no questions: an
    /// install behind a password dialog has nobody at a terminal to say yes.
    install: &'static [&'static str],

    /// What the GitHub CLI is called in this archive. `bubblewrap`, `git`,
    /// `nodejs` and `npm` are the same word on all three.
    gh: &'static str,
}

/// How one row is installed on a machine whose package manager is known.
///
/// **Every row has one of the three**, which is what makes the hint screen a
/// screen about machines rather than about rows: on a distribution Verkstead has
/// a command for, nothing that was ticked is left with nowhere to go — see
/// [`no_command`], which is the whole of the other arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Installs {
    /// A package this distribution carries, under the name it calls it.
    Package(&'static str),

    /// A package npm carries, installed globally under the node the elevated
    /// batch has just made sure of.
    FromNpm(&'static str),

    /// Or the vendor's own installer, run as the user — see [`Vendor`].
    Vendor(Vendor),
}

/// Which of the three one row is, on a machine whose `packager` is this one.
///
/// **Claude Code's npm package is not what a ticked row installs.** The native
/// installer is the one that stays current, which is why the wizard's own tab
/// leads with it and why the two npm harnesses here are the other two. And the
/// `grok-cli` in nixpkgs and the one on npm are other people's projects rather
/// than xAI's grok, so neither is a package this could name.
fn installs(packager: Packager, dependency: Dependency) -> Installs {
    match dependency {
        // `bubblewrap`, `git`, `nodejs` and `npm` are the same word in all three
        // archives; the GitHub CLI is the one they disagree about.
        Dependency::Sandbox => Installs::Package("bubblewrap"),
        Dependency::Git => Installs::Package("git"),
        Dependency::Gh => Installs::Package(packager.gh),

        Dependency::Codex => Installs::FromNpm("@openai/codex"),
        Dependency::OpenCode => Installs::FromNpm("opencode-ai"),

        Dependency::Claude => Installs::Vendor(CLAUDE),
        Dependency::Grok => Installs::Vendor(GROK),
    }
}

/// And which package manager a distribution has.
///
/// The three Linux families whose command is written down, and nothing for the
/// rest: NixOS installs from its configuration rather than from a command, a
/// Linux naming none of the five is one no line here would be right on, a Mac
/// never reaches this at all — Homebrew is no archive of the machine's and has
/// its own arm, see [`homebrew`] — and Windows is its own task.
fn packager(distro: Distro) -> Option<Packager> {
    match distro {
        Distro::Ubuntu | Distro::Debian => Some(Packager {
            install: &["apt-get", "install", "-y"],
            gh: "gh",
        }),
        Distro::Fedora => Some(Packager {
            install: &["dnf", "install", "-y"],
            gh: "gh",
        }),
        Distro::Arch => Some(Packager {
            install: &["pacman", "-S", "--noconfirm"],
            gh: "github-cli",
        }),
        Distro::NixOs | Distro::OtherLinux | Distro::MacOs | Distro::Windows => None,
    }
}

/// What goes under a row on a machine there is no command for at all.
fn no_command(distro: Distro) -> String {
    match distro {
        Distro::NixOs => {
            "NixOS installs what it has from its configuration rather than from a command."
        }
        Distro::OtherLinux => "Verkstead has no install command for this distribution.",
        _ => "Verkstead has no install command for this machine.",
    }
    .to_owned()
}

/// And what goes under a row whose installer would land under a home this
/// machine does not name.
///
/// The two vendors' installers both write under `$HOME`, so a server started
/// with none — a unit file's environment with the variable unset — has nowhere
/// to put what they install and nothing to write to `session_path` afterwards.
fn no_home(dependency: Dependency) -> String {
    format!(
        "{} installs under a home directory, and this server was started \
         without one.",
        named(dependency)
    )
}

/// What a row is called in a sentence.
fn named(dependency: Dependency) -> &'static str {
    match dependency {
        Dependency::Sandbox => "bubblewrap",
        Dependency::Git => "git",
        Dependency::Claude => "Claude Code",
        Dependency::Codex => "Codex",
        Dependency::Grok => "Grok Build",
        Dependency::OpenCode => "OpenCode",
        Dependency::Gh => "The GitHub CLI",
    }
}

/// `line` as the command a dialog is put in front of, touching `marker` before
/// anything else it does.
///
/// One shell line rather than an argument vector, because the elevated batch is
/// two commands joined and the marker is a third — see [`STARTED`]. Whose shell
/// is the platform's: `/bin/sh` on the two Unixes, and PowerShell on the one
/// whose installers are written in it.
fn raising(platform: Platform, line: &str, marker: Option<&Path>) -> Vec<String> {
    if platform == Platform::Windows {
        return powershell(line, marker);
    }

    let line = match marker {
        Some(marker) => format!(": > {} && {line}", quoted(&marker.to_string_lossy())),
        None => line.to_owned(),
    };

    vec![SHELL.to_owned(), "-c".to_owned(), line]
}

/// And the same line as PowerShell takes one: encoded, and in one word.
///
/// **Encoded because of what raises it.** The runas arm hands the command to
/// `Start-Process -ArgumentList`, which joins its arguments with spaces and
/// quotes none of them — so a script with a space in it would arrive as a
/// dozen arguments. `-EncodedCommand` takes base64 of UTF-16, which is one word
/// with nothing in it a command line has an opinion about. See
/// `verkstead_desktop::elevate`, which is the arm this is for.
///
/// **`Stop` because a cmdlet that fails is not a shell that failed.** A failing
/// native command sets `$LASTEXITCODE` and a failing cmdlet — which is what
/// `irm` is — writes an error and carries on, so without this an installer that
/// could not reach the network would be a unit that reported success. With it
/// the error is terminating, PowerShell exits non-zero, and the row fails.
fn powershell(line: &str, marker: Option<&Path>) -> Vec<String> {
    let mut script = String::from("$ErrorActionPreference = 'Stop'; ");

    if let Some(marker) = marker {
        script.push_str(&format!(
            "New-Item -Force -ItemType File -Path {} | Out-Null; ",
            said(&marker.to_string_lossy()),
        ));
    }

    // And out with what the line exited with, `Start-Process -PassThru` on the
    // other side of the dialog reporting the exit code of *this* process. A
    // line that ran no native command leaves `$LASTEXITCODE` unset, which exits
    // nought: nothing failed, and a cmdlet that did would have thrown above.
    script.push_str(line);
    script.push_str("; exit $LASTEXITCODE");

    let utf16: Vec<u8> = script
        .encode_utf16()
        .flat_map(|unit| unit.to_le_bytes())
        .collect();

    vec![
        "powershell".to_owned(),
        "-NoProfile".to_owned(),
        "-NonInteractive".to_owned(),
        "-EncodedCommand".to_owned(),
        base64::engine::general_purpose::STANDARD.encode(utf16),
    ]
}

/// `word` as one word of a shell line.
fn quoted(word: &str) -> String {
    format!("'{}'", word.replace('\'', r"'\''"))
}

/// And as one word of a PowerShell line, where the character that ends a
/// literal string is written twice to mean itself.
fn said(word: &str) -> String {
    format!("'{}'", word.replace('\'', "''"))
}

/// What the status line says while a dialog is up.
///
/// It names the machine because of who may be reading it: the human answering
/// this wizard can be on a phone on the tailnet, and a dialog is on the screen
/// of the machine Verkstead is running on.
fn asking(hostname: &str) -> String {
    format!("Waiting for the password dialog on {hostname}")
}

/// `ticked` in the order the step draws its rows, each of them once.
fn ordered(ticked: &[Dependency]) -> Vec<Dependency> {
    ROWS.iter()
        .copied()
        .filter(|dependency| ticked.contains(dependency))
        .collect()
}

/// Where one row is in that order.
fn place(dependency: Dependency) -> usize {
    ROWS.iter()
        .position(|row| *row == dependency)
        .unwrap_or(ROWS.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;
    use std::path::PathBuf;

    use crate::platform::{Environment, Platform};

    /// The sandbox, git and one npm harness, which is the ticking the whole of
    /// this is about: two packages, a package manager that has neither of the
    /// other two, and a harness that comes off npm.
    const TICKED: &[Dependency] = &[Dependency::Sandbox, Dependency::Git, Dependency::Codex];

    /// The home a stated machine runs under, which is what the two vendors'
    /// installers land under.
    const HOME: &str = "/home/you";

    /// And who it runs as, which is who a Mac hands Homebrew's prefix to.
    const USER: &str = "ada";

    /// A machine on `distro` with nothing on its `PATH`, which is a machine
    /// with no `npm` and no `brew`.
    fn machine(distro: Distro) -> Machine {
        stated(distro, &PathBuf::new())
    }

    /// The same, searching `dir` — which is how a machine that *has* one of
    /// them is stated.
    fn stated(distro: Distro, dir: &Path) -> Machine {
        under(distro, dir, Some(PathBuf::from(HOME)))
    }

    /// And the same again under `home`, which is `None` for the one machine
    /// this asks about that names none.
    fn under(distro: Distro, dir: &Path, home: Option<PathBuf>) -> Machine {
        named(distro, dir, home, Some(USER.to_owned()))
    }

    /// And the whole of it, for the other machine that names nobody.
    fn named(distro: Distro, dir: &Path, home: Option<PathBuf>, user: Option<String>) -> Machine {
        let (platform, os_release) = match distro {
            Distro::MacOs => (Platform::MacOs, None),
            Distro::Windows => (Platform::Windows, None),
            Distro::NixOs => (Platform::Linux, Some("ID=nixos\n")),
            Distro::Ubuntu => (Platform::Linux, Some("ID=ubuntu\n")),
            Distro::Fedora => (Platform::Linux, Some("ID=fedora\n")),
            Distro::Debian => (Platform::Linux, Some("ID=debian\n")),
            Distro::Arch => (Platform::Linux, Some("ID=arch\n")),
            Distro::OtherLinux => (Platform::Linux, None),
        };

        // Whichever variable this platform keeps a home in, so that a stated
        // Windows machine names one the way a stated Linux does — see
        // [`crate::platform::home_dir`].
        let environment = match platform {
            Platform::Windows => Environment {
                userprofile: home,
                user,
                ..Environment::default()
            },
            Platform::Linux | Platform::MacOs => Environment {
                home,
                user,
                ..Environment::default()
            },
        };

        Machine::stated(
            platform,
            OsString::from(dir.as_os_str()),
            OsString::from(dir.as_os_str()),
            None,
            os_release.map(str::to_owned),
            &environment,
        )
    }

    /// The profile a stated Windows machine runs under, which is where its
    /// vendors' installers land.
    const PROFILE: &str = r"C:\Users\ada";

    /// And the Data Directory it keeps, and the image it is running: the two
    /// the account row wants, and the two only the Windows arm asks for.
    const DATA_DIR: &str = r"C:\ProgramData\verkstead";
    const VERKSTEAD: &str = r"C:\Program Files\Verkstead\verkstead.exe";

    /// And what it calls a program, which is the one thing on this platform
    /// that says a name on the `PATH` is one: `npm` is `npm.cmd` here, so a
    /// machine asked whether it has one has to be told the extensions.
    const PATHEXT: &str = ".COM;.EXE;.BAT;.CMD";

    /// A stated Windows machine that knows all of them, searching `dir`.
    fn windows_machine(dir: &Path) -> Machine {
        Machine::stated(
            Platform::Windows,
            OsString::from(dir.as_os_str()),
            OsString::from(dir.as_os_str()),
            Some(OsString::from(PATHEXT)),
            None,
            &Environment {
                userprofile: Some(PathBuf::from(PROFILE)),
                user: Some(USER.to_owned()),
                ..Environment::default()
            },
        )
        .against(Path::new(DATA_DIR))
        .running(Some(PathBuf::from(VERKSTEAD)))
    }

    /// The one command a plan of one unit raises.
    fn line(plan: &Plan) -> &str {
        let [unit] = plan.units.as_slice() else {
            panic!("one press is one dialog: {plan:?}");
        };

        &unit.line
    }

    /// Nothing at all, which is what a run does on a machine it installs
    /// nothing on: the two arms that use it never reach a unit.
    fn as_nobody() -> AsTheUser {
        AsTheUser {
            home: None,
            path: OsString::new(),
        }
    }

    /// And a settings handle over a directory nothing here writes: the two
    /// plans worked below have no unit that lands anywhere.
    fn nowhere() -> Settings {
        Settings::in_data_dir(Path::new("/nonexistent"))
    }

    /// Which rows a plan says nothing here can install.
    fn beyond(plan: &Plan) -> Vec<Dependency> {
        plan.beyond.iter().map(|(row, _)| *row).collect()
    }

    /// The order the rows are worked in is the order the wizard draws them,
    /// which is the order the probe answers in.
    ///
    /// Two lists in two modules, held together here: a run that installed in
    /// one order under a page that listed in another would be two accounts of
    /// one press.
    #[test]
    fn the_rows_are_worked_in_the_order_the_wizard_draws_them() {
        let drawn: Vec<Dependency> = machine(Distro::Ubuntu)
            .rows()
            .iter()
            .map(|row| row.dependency)
            .collect();

        assert_eq!(drawn, ROWS);
    }

    /// Each of the three package managers is its own line, and every ticked
    /// package is on it.
    #[test]
    fn one_command_per_distribution_names_every_ticked_package() {
        assert_eq!(
            line(&plan(&machine(Distro::Ubuntu), TICKED)),
            "apt-get install -y bubblewrap git nodejs npm && npm install -g @openai/codex",
        );

        assert_eq!(
            line(&plan(&machine(Distro::Debian), TICKED)),
            "apt-get install -y bubblewrap git nodejs npm && npm install -g @openai/codex",
        );

        assert_eq!(
            line(&plan(&machine(Distro::Fedora), TICKED)),
            "dnf install -y bubblewrap git nodejs npm && npm install -g @openai/codex",
        );

        assert_eq!(
            line(&plan(&machine(Distro::Arch), TICKED)),
            "pacman -S --noconfirm bubblewrap git nodejs npm && npm install -g @openai/codex",
        );
    }

    /// And the GitHub CLI is the one name they disagree about.
    #[test]
    fn the_github_cli_is_named_as_the_archive_names_it() {
        let ticked = &[Dependency::Gh];

        assert_eq!(
            line(&plan(&machine(Distro::Ubuntu), ticked)),
            "apt-get install -y gh",
        );

        assert_eq!(
            line(&plan(&machine(Distro::Arch), ticked)),
            "pacman -S --noconfirm github-cli",
        );
    }

    /// Node goes in where an npm harness was ticked and the machine has no
    /// `npm`, and stays out where it has one.
    #[test]
    fn node_is_installed_only_where_it_is_missing() {
        let dir = tempfile::tempdir().unwrap();
        crate::stand_ins::program(&dir.path().join(NPM), "#!/bin/sh\nexit 0\n");

        assert_eq!(
            line(&plan(&stated(Distro::Ubuntu, dir.path()), TICKED)),
            "apt-get install -y bubblewrap git && npm install -g @openai/codex",
        );

        // And nothing about node where no npm harness was ticked, whatever the
        // machine has: what it is there for is the line beside it.
        assert_eq!(
            line(&plan(&machine(Distro::Ubuntu), &[Dependency::Git])),
            "apt-get install -y git",
        );
    }

    /// A distribution with no command of its own asks for nothing and fails
    /// every ticked row.
    #[test]
    fn a_distribution_with_no_command_raises_nothing() {
        for distro in [Distro::NixOs, Distro::OtherLinux] {
            let plan = plan(&machine(distro), TICKED);

            assert!(plan.units.is_empty(), "{distro:?}: {plan:?}");
            assert_eq!(beyond(&plan), TICKED);
        }

        assert!(
            plan(&machine(Distro::NixOs), TICKED).beyond[0]
                .1
                .contains("configuration"),
            "NixOS says what it installs from",
        );
    }

    /// The two rows that are not a package are a unit each, after the elevated
    /// batch and run as the user, landing where the vendor's own installer puts
    /// what it installs.
    #[test]
    fn the_vendor_installers_are_their_own_units_after_the_batch() {
        let plan = plan(
            &machine(Distro::Ubuntu),
            &[Dependency::Git, Dependency::Claude, Dependency::Grok],
        );

        assert!(plan.beyond.is_empty(), "{plan:?}");

        let [batch, claude, grok] = plan.units.as_slice() else {
            panic!("the batch and one unit per vendor: {plan:?}");
        };

        assert_eq!(batch.line, "apt-get install -y git");
        assert_eq!(batch.how, How::Raised);
        assert_eq!(batch.lands, None);

        assert_eq!(
            claude.line,
            "curl -fsSL https://claude.ai/install.sh | bash"
        );
        assert_eq!(claude.how, How::AsTheUser);
        assert_eq!(claude.doing, "Running Anthropic's installer");
        assert_eq!(claude.covers, [Dependency::Claude]);
        assert_eq!(claude.lands, Some(PathBuf::from(HOME).join(".local/bin")));

        assert_eq!(grok.line, "curl -fsSL https://x.ai/cli/install.sh | bash");
        assert_eq!(grok.how, How::AsTheUser);
        assert_eq!(grok.doing, "Running xAI's installer");
        assert_eq!(grok.lands, Some(PathBuf::from(HOME).join(".grok/bin")));
    }

    /// And on a machine that names no home there is nowhere for either of them
    /// to land, so both are the hint screen's, the packages going ahead without
    /// them.
    #[test]
    fn a_machine_with_no_home_installs_neither_of_them() {
        let plan = plan(
            &under(Distro::Ubuntu, &PathBuf::new(), None),
            &[Dependency::Git, Dependency::Claude, Dependency::Grok],
        );

        assert_eq!(line(&plan), "apt-get install -y git");
        assert_eq!(beyond(&plan), [Dependency::Claude, Dependency::Grok]);
        assert!(
            plan.beyond[0].1.contains("Claude Code") && plan.beyond[0].1.contains("home"),
            "the row is named in its own words: {:?}",
            plan.beyond[0].1,
        );
    }

    /// A Mac with Homebrew is one `brew` line per ticked row, every one of them
    /// run as the user and nothing raised at all.
    ///
    /// The sandbox row is neither a unit nor a sentence: `sandbox-exec` is on
    /// every Mac, so a press that named it has nothing to do about it.
    #[test]
    fn a_mac_with_homebrew_installs_every_ticked_row_with_it() {
        let dir = tempfile::tempdir().unwrap();
        crate::stand_ins::program(&dir.path().join(BREW), "#!/bin/sh\nexit 0\n");

        let plan = plan(
            &stated(Distro::MacOs, dir.path()),
            &[
                Dependency::Sandbox,
                Dependency::Git,
                Dependency::Claude,
                Dependency::Grok,
            ],
        );

        assert!(plan.beyond.is_empty(), "{plan:?}");

        let [git, claude, grok] = plan.units.as_slice() else {
            panic!("one unit per ticked row, and none for the sandbox: {plan:?}");
        };

        assert_eq!(git.line, "brew install git");
        assert_eq!(git.how, How::AsTheUser);
        assert_eq!(git.covers, [Dependency::Git]);
        assert_eq!(git.lands, None, "Homebrew's prefix is on the Apple floor");

        // A cask rather than a formula, which is the install a Mac session
        // finds whichever way Verkstead was started.
        assert_eq!(claude.line, "brew install --cask claude-code");
        assert_eq!(claude.how, How::AsTheUser);

        // And the one row Homebrew has no name for is xAI's own installer,
        // under this user's home the way it is everywhere else.
        assert_eq!(grok.line, "curl -fsSL https://x.ai/cli/install.sh | bash");
        assert_eq!(grok.lands, Some(PathBuf::from(HOME).join(".grok/bin")));
        assert_eq!(grok.chain, None, "xAI's installer wants no Homebrew");
    }

    /// And a Mac without it makes Homebrew's prefix and installs Homebrew
    /// first: one raised unit and one as the user, in front of every `brew`
    /// line and in the same chain as them.
    #[test]
    fn a_mac_without_homebrew_installs_it_before_anything_else() {
        let plan = plan(
            &machine(Distro::MacOs),
            &[Dependency::Git, Dependency::Grok, Dependency::Gh],
        );

        assert!(plan.beyond.is_empty(), "{plan:?}");

        let [prefix, installing, git, grok, gh] = plan.units.as_slice() else {
            panic!("the prefix, Homebrew, and a unit per ticked row: {plan:?}");
        };

        // The one thing a Mac ever raises, and it is a directory made and
        // handed over rather than an install.
        assert_eq!(prefix.how, How::Raised);
        assert!(prefix.covers.is_empty(), "a prefix is nobody's row");
        assert_eq!(
            prefix.line,
            "prefix=/usr/local; [ \"$(uname -m)\" = arm64 ] && prefix=/opt/homebrew; \
             mkdir -p \"$prefix\" && chmod ug=rwx \"$prefix\" && chgrp admin \"$prefix\" && \
             chown 'ada' \"$prefix\"",
            "both prefixes, told apart by the machine it runs on, and handed to the user",
        );

        assert_eq!(
            installing.how,
            How::AsTheUser,
            "Homebrew refuses to be root"
        );
        assert_eq!(installing.line, HOMEBREW);
        assert!(installing.covers.is_empty());

        assert_eq!(git.line, "brew install git");
        assert_eq!(gh.line, "brew install gh");

        for unit in [prefix, installing, git, gh] {
            assert_eq!(
                unit.chain,
                Some(Chain::Homebrew),
                "every one of them stands on the prefix: {unit:?}",
            );
        }

        assert_eq!(grok.chain, None, "and xAI's installer does not");
    }

    /// A refused prefix fails every row Homebrew would have installed, in the
    /// words the dialog was refused in, and nothing behind it is run.
    #[test]
    fn a_refused_prefix_fails_every_brew_row_and_runs_no_installer() {
        let dir = tempfile::tempdir().unwrap();
        let ran = dir.path().join("the-installer-ran");

        // The one program Homebrew's own line reaches for, as a stub that
        // leaves a mark: what is being asked is whether it is ever run.
        crate::stand_ins::program(
            &dir.path().join("bash"),
            &format!("#!/bin/sh\n: > '{}'\n", ran.display()),
        );

        let plan = plan(&machine(Distro::MacOs), &[Dependency::Git, Dependency::Gh]);
        let run = Arc::new(Run::of(&plan, "a-mac"));

        work(
            &run,
            &plan,
            &Refusing,
            &nowhere(),
            &AsTheUser {
                home: None,
                path: OsString::from(dir.path().as_os_str()),
            },
        );

        let (view, rows) = run.reading();

        assert_eq!(view.phase, RunPhase::Done);
        assert_eq!(view.status, "2 of 2 could not be installed");

        for (dependency, state) in rows {
            assert_eq!(
                state,
                InstallState::Failed {
                    why: DISMISSED.to_owned(),
                },
                "{dependency:?} stands on the prefix that was refused",
            );
        }

        assert!(
            !ran.exists(),
            "a prefix nobody made is a Homebrew nobody installs",
        );
    }

    /// And a Mac whose environment names nobody has nobody to hand a prefix to,
    /// so every `brew` row is the hint screen's and nothing is raised.
    #[test]
    fn a_mac_that_names_nobody_installs_nothing_with_brew() {
        let plan = plan(
            &named(
                Distro::MacOs,
                &PathBuf::new(),
                Some(PathBuf::from(HOME)),
                None,
            ),
            &[Dependency::Git, Dependency::Grok, Dependency::Gh],
        );

        assert_eq!(
            line(&plan),
            "curl -fsSL https://x.ai/cli/install.sh | bash",
            "the one row that is not Homebrew's goes ahead",
        );

        assert_eq!(beyond(&plan), [Dependency::Git, Dependency::Gh]);
        assert!(
            plan.beyond[0].1.contains("Homebrew"),
            "the row says what could not be done: {:?}",
            plan.beyond[0].1,
        );
    }

    /// A Windows machine raises a unit per ticked row, with the node winget
    /// carries in front of the npm ones where it has no `npm`.
    ///
    /// Which is this task's own criterion: git and Codex ticked is `winget` for
    /// git, `winget` for node, and the `npm install -g` — every one of them
    /// behind the dialog.
    #[test]
    fn a_windows_machine_raises_a_unit_for_every_ticked_row() {
        let plan = plan(
            &windows_machine(&PathBuf::new()),
            &[Dependency::Git, Dependency::Codex],
        );

        assert!(plan.beyond.is_empty(), "{plan:?}");

        let [node, git, codex] = plan.units.as_slice() else {
            panic!("node, and a unit per ticked row: {plan:?}");
        };

        assert_eq!(
            node.line,
            format!("winget install --id OpenJS.NodeJS {UNATTENDED}")
        );
        assert!(node.covers.is_empty(), "node is nobody's row");

        assert_eq!(
            git.line,
            format!("winget install --id Git.Git {UNATTENDED}")
        );
        assert_eq!(git.covers, [Dependency::Git]);
        assert_eq!(git.lands, None, "winget installs onto the machine's floor");
        assert_eq!(git.chain, None, "git wants no node");

        assert_eq!(codex.line, "npm install -g @openai/codex");
        assert_eq!(codex.covers, [Dependency::Codex]);

        for unit in [node, git, codex] {
            assert_eq!(
                unit.how,
                How::Raised,
                "every Windows install goes through the runas arm: {unit:?}",
            );
        }

        // And the two that stand together: a node nobody allowed fails the
        // `npm` line in its own words rather than leaving it to fail for want
        // of a program.
        assert_eq!(node.chain, Some(Chain::Node));
        assert_eq!(codex.chain, Some(Chain::Node));
    }

    /// And node stays out where the machine has `npm`, the way it does on a
    /// Linux.
    #[test]
    fn a_windows_machine_with_npm_installs_no_node() {
        let dir = tempfile::tempdir().unwrap();
        crate::stand_ins::program(&dir.path().join("npm.CMD"), "@exit 0\n");

        let plan = plan(&windows_machine(dir.path()), &[Dependency::OpenCode]);

        assert_eq!(line(&plan), "npm install -g opencode-ai");
    }

    /// The sandbox row there is the account this Data Directory's sessions run
    /// as, and what it runs is Verkstead's own elevated verb — said with the
    /// directory outright, the account's name being a fingerprint of it.
    #[test]
    fn the_windows_sandbox_row_makes_the_session_account() {
        let plan = plan(&windows_machine(&PathBuf::new()), &[Dependency::Sandbox]);

        let [account] = plan.units.as_slice() else {
            panic!("the one row, and one unit for it: {plan:?}");
        };

        assert_eq!(
            account.line,
            format!(
                "& 'C:\\Program Files\\Verkstead\\verkstead.exe' session-account create \
                 --data-dir '{DATA_DIR}'"
            ),
        );
        assert_eq!(account.how, How::Raised);
        assert_eq!(account.covers, [Dependency::Sandbox]);
        assert_eq!(account.lands, None, "an account lands on no `PATH`");

        // And it says which account it made, which is what the named pipe is
        // re-opened granting — see [`regranted`].
        assert_eq!(
            account.regrants.as_deref(),
            Some(crate::sandbox::account::named(Path::new(DATA_DIR)).as_str()),
        );
    }

    /// And a server that cannot name the image it is running has no binary to
    /// make one with, so that row is the hint screen's and the rest goes ahead.
    #[test]
    fn a_windows_machine_that_cannot_name_itself_makes_no_account() {
        let plan = plan(
            &windows_machine(&PathBuf::new()).running(None),
            &[Dependency::Sandbox, Dependency::Git],
        );

        assert_eq!(
            line(&plan),
            format!("winget install --id Git.Git {UNATTENDED}")
        );
        assert_eq!(beyond(&plan), [Dependency::Sandbox]);
        assert!(
            plan.beyond[0].1.contains("which program it is running"),
            "the row says what could not be done: {:?}",
            plan.beyond[0].1,
        );
    }

    /// The two vendors' installers are the PowerShell ones there, raised with
    /// everything else and landing under the profile.
    ///
    /// **Raised is right here and wrong on a Linux**, and the difference is
    /// what the dialog does: UAC raises the same user with their administrator
    /// token, so `%USERPROFILE%` on the far side of it is still the human's.
    #[test]
    fn the_windows_vendor_installers_are_raised_and_land_under_the_profile() {
        let plan = plan(
            &windows_machine(&PathBuf::new()),
            &[Dependency::Claude, Dependency::Grok],
        );

        let [claude, grok] = plan.units.as_slice() else {
            panic!("a unit per vendor: {plan:?}");
        };

        assert_eq!(claude.line, "irm https://claude.ai/install.ps1 | iex");
        assert_eq!(claude.how, How::Raised);
        assert_eq!(claude.doing, "Running Anthropic's installer");
        assert_eq!(
            claude.lands,
            Some(PathBuf::from(PROFILE).join(r".local\bin")),
            "which is what goes on `session_path`, so the row ticks off the next probe",
        );

        assert_eq!(grok.line, "irm https://x.ai/cli/install.ps1 | iex");
        assert_eq!(grok.how, How::Raised);
        assert_eq!(grok.lands, Some(PathBuf::from(PROFILE).join(r".grok\bin")));
    }

    /// And a Windows command crosses as one encoded word, because what raises
    /// it joins its arguments with spaces and quotes none of them.
    #[test]
    fn a_windows_command_is_one_encoded_word() {
        let raised = raising(
            Platform::Windows,
            "winget install --id Git.Git",
            Some(Path::new(r"C:\Temp\run\started")),
        );

        let [program, profile, interactive, encoded, script] = raised.as_slice() else {
            panic!("PowerShell, its switches and the script: {raised:?}");
        };

        assert_eq!(program, "powershell");
        assert_eq!(profile, "-NoProfile");
        assert_eq!(interactive, "-NonInteractive");
        assert_eq!(encoded, "-EncodedCommand");

        assert_eq!(
            decoded(script),
            "$ErrorActionPreference = 'Stop'; \
             New-Item -Force -ItemType File -Path 'C:\\Temp\\run\\started' | Out-Null; \
             winget install --id Git.Git; exit $LASTEXITCODE",
        );
    }

    /// What `-EncodedCommand` was handed, read back: base64 of UTF-16.
    fn decoded(encoded: &str) -> String {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .expect("base64, which is what that switch takes");

        let utf16: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .collect();

        String::from_utf16(&utf16).expect("UTF-16, which is what that switch takes")
    }

    /// A press on a server with nothing to raise a dialog with installs
    /// nothing, asks nothing, and every ticked row says why.
    #[test]
    fn a_server_with_no_way_to_ask_installs_nothing() {
        let installer = Installer::raising(None);

        installer
            .start(&machine(Distro::Ubuntu), &nowhere(), TICKED)
            .unwrap();

        let (run, rows) = installer.reading().expect("a press makes a run");

        assert_eq!(run.phase, RunPhase::Done);
        assert_eq!((run.done, run.total), (3, 3));

        for (dependency, state) in rows {
            let InstallState::Failed { why } = state else {
                panic!("{dependency:?} was ticked on a server that cannot ask: {state:?}");
            };

            assert!(why.contains("desktop app"), "{why}");
        }
    }

    /// Cancel finishes the unit under way and skips the rest.
    #[test]
    fn cancelling_finishes_what_is_running_and_skips_the_rest() {
        let plan = Plan {
            units: vec![
                unit("first", Dependency::Sandbox),
                unit("second", Dependency::Git),
                unit("third", Dependency::Gh),
            ],
            beyond: Vec::new(),
            platform: Platform::Linux,
        };

        let run = Arc::new(Run::of(&plan, "a-machine"));

        // Pressed from inside the first unit, which is the window the button is
        // pressed in: a flag set before the run started would prove nothing
        // about a unit finishing.
        let escalation = Pressing {
            raised: Mutex::new(Vec::new()),
            cancel: run.clone(),
        };

        work(&run, &plan, &escalation, &nowhere(), &as_nobody());

        assert_eq!(
            escalation.raised.lock().unwrap().len(),
            1,
            "the unit under way finishes and nothing after it starts",
        );

        let (view, rows) = run.reading();

        assert_eq!(view.phase, RunPhase::Done);
        assert_eq!((view.done, view.total), (3, 3));
        assert!(!view.cancelling, "a run that is over is cancelling nothing");

        assert_eq!(
            rows,
            [
                // It ran, so what it reads is the probe's business again.
                (Dependency::Sandbox, InstallState::Idle),
                (
                    Dependency::Git,
                    InstallState::Failed {
                        why: CANCELLED.to_owned(),
                    },
                ),
                (
                    Dependency::Gh,
                    InstallState::Failed {
                        why: CANCELLED.to_owned(),
                    },
                ),
            ],
        );
    }

    /// A refused dialog lands on every row of that unit, in the words the
    /// asking was refused in.
    #[test]
    fn a_refusal_lands_on_every_row_of_the_unit_that_was_refused() {
        let plan = plan(&machine(Distro::Ubuntu), TICKED);
        let run = Arc::new(Run::of(&plan, "a-machine"));

        work(&run, &plan, &Refusing, &nowhere(), &as_nobody());

        let (view, rows) = run.reading();

        assert_eq!(view.phase, RunPhase::Done);
        assert_eq!(view.status, "3 of 3 could not be installed");

        for (dependency, state) in rows {
            assert_eq!(
                state,
                InstallState::Failed {
                    why: DISMISSED.to_owned(),
                },
                "{dependency:?}",
            );
        }
    }

    /// The command a dialog is put in front of touches the marker before it
    /// does anything else, so that the status line can move on without waiting
    /// for the whole install.
    #[test]
    fn the_command_says_it_has_started_before_it_starts() {
        let raised = raising(
            Platform::Linux,
            "apt-get install -y git",
            Some(Path::new("/tmp/run/started")),
        );

        assert_eq!(
            raised,
            [
                SHELL,
                "-c",
                ": > '/tmp/run/started' && apt-get install -y git",
            ],
        );
    }

    /// One raised unit installing one row.
    fn unit(line: &str, covers: Dependency) -> Unit {
        Unit {
            line: line.to_owned(),
            covers: vec![covers],
            doing: format!("Installing {line}"),
            how: How::Raised,
            lands: None,
            chain: None,
            regrants: None,
        }
    }

    /// What a dismissed password dialog said.
    const DISMISSED: &str = "Error: (-128) User canceled.";

    /// An asking that records what it was handed, presses Cancel while it is
    /// holding the run, and says it worked.
    #[derive(Debug)]
    struct Pressing {
        raised: Mutex<Vec<Vec<String>>>,
        cancel: Arc<Run>,
    }

    impl Elevate for Pressing {
        fn raise(&self, command: &[String]) -> Raised {
            self.raised.lock().unwrap().push(command.to_vec());
            self.cancel.cancelled.store(true, Ordering::SeqCst);

            Raised::Done
        }
    }

    /// And one the human dismissed.
    #[derive(Debug)]
    struct Refusing;

    impl Elevate for Refusing {
        fn raise(&self, _: &[String]) -> Raised {
            Raised::Refused {
                why: DISMISSED.to_owned(),
            }
        }
    }
}
