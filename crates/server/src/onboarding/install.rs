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
//! refusal lands on every row of that unit and on nothing else, and a row no
//! archive carries is failed before anything is raised, with a sentence saying
//! so where the hint screen draws it.
//!
//! **Nothing runs while nobody is looking.** A run is started by a press and
//! ends by itself; between presses the wizard is the probe it always was — a
//! `PATH` walked and one `bwrap` run — which is also what says a row that was
//! installing has landed. There is no sweep here and nothing is written down: a
//! run belongs to the life of this server, and a restart has none.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use verkstead_render::{Dependency, Distro, InstallState, RunPhase, RunView};

use super::{Machine, SHELL};
use crate::remote::{Elevate, Raised};

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

/// What a server with no way to raise a dialog says on every ticked row.
const NO_DIALOG: &str = "Verkstead has no way to ask this machine for a password: the desktop \
                         app is what raises the dialog, and this server was not started by it.";

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
    pub(crate) fn start(&self, machine: &Machine, ticked: &[Dependency]) -> Result<(), Refusal> {
        let mut held = self.run.lock().expect("the install run's lock");

        if held.as_ref().is_some_and(|run| run.going()) {
            return Err(Refusal::Going);
        }

        let ticked = ordered(ticked);

        let plan = match self.escalation.as_ref() {
            Some(_) => plan(machine, &ticked),
            None => nothing(&ticked, NO_DIALOG),
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

        std::thread::spawn(move || work(&run, &plan, escalation.as_ref()));

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
/// On a thread of its own, because the first thing every unit does is wait for
/// somebody to read a password dialog, and how long that takes is theirs.
fn work(run: &Arc<Run>, plan: &Plan, escalation: &dyn Elevate) {
    // The one directory a run makes, and it goes when the run does: what is in
    // it is the marker each unit touches, which is how the status line finds out
    // the dialog was answered — see [`STARTED`]. A machine with nowhere to put
    // one is a run whose status line stays on the dialog, and nothing worse.
    let marking = tempfile::Builder::new()
        .prefix("verkstead-install")
        .tempdir()
        .ok();

    for (number, unit) in plan.units.iter().enumerate() {
        // Between units and never inside one: a package manager that is already
        // running is left to finish, a machine half way through an unpack being
        // worse than one that finished the unpack nobody wanted.
        if run.cancelled.load(Ordering::SeqCst) {
            break;
        }

        let marker = marking
            .as_ref()
            .map(|dir| dir.path().join(format!("{STARTED}-{number}")));

        run.asking(unit);

        let watching = marker
            .clone()
            .map(|marker| watch(run, marker, unit.doing.clone()));

        let raised = escalation.raise(&raising(&unit.line, marker.as_deref()));

        if let Some((over, watching)) = watching {
            over.store(true, Ordering::SeqCst);
            let _ = watching.join();
        }

        run.landed(
            unit,
            match raised {
                Raised::Done => Progress::Installed,
                Raised::Refused { why } => Progress::Failed(why),
            },
        );
    }

    run.over();
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
fn plan(machine: &Machine, ticked: &[Dependency]) -> Plan {
    let distro = machine.distro();

    let Some(packager) = packager(distro) else {
        return nothing(ticked, &no_command(distro));
    };

    let mut packages: Vec<&str> = Vec::new();
    let mut from_npm: Vec<&str> = Vec::new();
    let mut covers: Vec<Dependency> = Vec::new();
    let mut beyond: Vec<(Dependency, String)> = Vec::new();

    for row in ticked {
        if let Some(package) = packager.package(*row) {
            packages.push(package);
            covers.push(*row);
        } else if let Some(package) = npm_package(*row) {
            from_npm.push(package);
            covers.push(*row);
        } else {
            beyond.push((*row, not_packaged(*row)));
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
    let units = (!lines.is_empty())
        .then(|| Unit {
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
        })
        .into_iter()
        .collect();

    Plan { units, beyond }
}

/// A plan that installs nothing, every ticked row failed with the same
/// sentence.
fn nothing(ticked: &[Dependency], why: &str) -> Plan {
    Plan {
        units: Vec::new(),
        beyond: ticked
            .iter()
            .map(|dependency| (*dependency, why.to_owned()))
            .collect(),
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

impl Packager {
    /// What this archive calls the program one row is about, where it carries
    /// one at all.
    fn package(&self, dependency: Dependency) -> Option<&'static str> {
        match dependency {
            Dependency::Sandbox => Some("bubblewrap"),
            Dependency::Git => Some("git"),
            Dependency::Gh => Some(self.gh),
            Dependency::Claude | Dependency::Codex | Dependency::Grok | Dependency::OpenCode => {
                None
            }
        }
    }
}

/// And which package manager a distribution has.
///
/// The three Linux families whose command is written down, and nothing for the
/// rest: NixOS installs from its configuration rather than from a command, a
/// Linux naming none of the five is one no line here would be right on, and the
/// two other platforms are their own tasks.
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

/// The npm package a harness is published as, for the two that are published as
/// one.
///
/// Claude Code's npm package is not among them: what a ticked Claude row runs is
/// Anthropic's own installer, which is the install that stays current — see the
/// wizard's own tab, which leads with it for the same reason.
fn npm_package(dependency: Dependency) -> Option<&'static str> {
    match dependency {
        Dependency::Codex => Some("@openai/codex"),
        Dependency::OpenCode => Some("opencode-ai"),
        Dependency::Sandbox
        | Dependency::Git
        | Dependency::Claude
        | Dependency::Grok
        | Dependency::Gh => None,
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

/// And what goes under one this distribution simply does not carry.
fn not_packaged(dependency: Dependency) -> String {
    format!(
        "{} is not one of this distribution's packages.",
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
/// two commands joined and the marker is a third — see [`STARTED`].
fn raising(line: &str, marker: Option<&Path>) -> Vec<String> {
    let line = match marker {
        Some(marker) => format!(": > {} && {line}", quoted(&marker.to_string_lossy())),
        None => line.to_owned(),
    };

    vec![SHELL.to_owned(), "-c".to_owned(), line]
}

/// `word` as one word of a shell line.
fn quoted(word: &str) -> String {
    format!("'{}'", word.replace('\'', r"'\''"))
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

    /// A machine on `distro` with nothing on its `PATH`, which is a machine
    /// with no `npm`.
    fn machine(distro: Distro) -> Machine {
        stated(distro, &PathBuf::new())
    }

    /// The same, searching `dir` — which is how a machine that *has* an `npm`
    /// is stated.
    fn stated(distro: Distro, dir: &Path) -> Machine {
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

        Machine::stated(
            platform,
            OsString::from(dir.as_os_str()),
            OsString::from(dir.as_os_str()),
            None,
            os_release.map(str::to_owned),
            &Environment::default(),
        )
    }

    /// The one command a plan of one unit raises.
    fn line(plan: &Plan) -> &str {
        let [unit] = plan.units.as_slice() else {
            panic!("one press is one dialog: {plan:?}");
        };

        &unit.line
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

    /// And a row this distribution does not carry is a row the run says so
    /// about, the rest of the ticking going ahead without it.
    #[test]
    fn a_row_no_archive_carries_is_left_to_the_hint_screen() {
        let plan = plan(
            &machine(Distro::Ubuntu),
            &[Dependency::Git, Dependency::Claude],
        );

        assert_eq!(line(&plan), "apt-get install -y git");
        assert_eq!(beyond(&plan), [Dependency::Claude]);
        assert!(
            plan.beyond[0].1.contains("Claude Code"),
            "the row is named in its own words: {:?}",
            plan.beyond[0].1,
        );
    }

    /// A press on a server with nothing to raise a dialog with installs
    /// nothing, asks nothing, and every ticked row says why.
    #[test]
    fn a_server_with_no_way_to_ask_installs_nothing() {
        let installer = Installer::raising(None);

        installer.start(&machine(Distro::Ubuntu), TICKED).unwrap();

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
        };

        let run = Arc::new(Run::of(&plan, "a-machine"));

        // Pressed from inside the first unit, which is the window the button is
        // pressed in: a flag set before the run started would prove nothing
        // about a unit finishing.
        let escalation = Pressing {
            raised: Mutex::new(Vec::new()),
            cancel: run.clone(),
        };

        work(&run, &plan, &escalation);

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

        work(&run, &plan, &Refusing);

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

    /// One unit installing one row.
    fn unit(line: &str, covers: Dependency) -> Unit {
        Unit {
            line: line.to_owned(),
            covers: vec![covers],
            doing: format!("Installing {line}"),
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
