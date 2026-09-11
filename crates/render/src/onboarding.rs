//! What the wizard is drawn from: whether this Verkstead can do anything yet,
//! what machine it is standing on, and what is missing from it.
//!
//! Nothing here is a setting. Every field is what the server found a moment
//! ago — a `PATH` walked, a `bwrap` run, `/etc/os-release` read — which is why
//! there is no stored half to reconcile against: an install that lands while
//! the page is open reads here on the next re-read, exactly as one that was
//! there all along.
//!
//! **Except the one field that is not read at all.** [`OnboardingView::mode`]
//! is the verdict the server reached at startup and has held ever since, and it
//! is what says whether the wizard is the only page there is. The steps beside
//! it are read afresh like everything else, so the two can disagree — a Profile
//! deleted while the server runs is a step that stands unmet under a mode that
//! is off — and that disagreement is the decision rather than a gap in it: the
//! wizard is a first run rather than a state to fall back into, and what says
//! so is the settings page's own empty state.
//!
//! **The distro is one of eight**, and they are the wizard's tabs: the two
//! platforms that have no distro to speak of, the five Linux distributions
//! whose install commands are written down, and everything else. The detected
//! one opens and the other seven stay reachable, because a detection read off
//! `ID_LIKE` is a guess about a derivative and the human is the one looking at
//! the machine.
//!
//! **The accounts are the machine's too.** What is found in the server's own
//! home is one account per harness at most — each shape is a fixed path under a
//! home — offered as the Profile it would be saved as, with whether that
//! harness is on the machine carried beside it. See [`AccountView`].
//!
//! **The git step's prefills are a read of their own.** What `git config
//! --global` and the environment can tell a Verkstead that has been told
//! nothing is [`PrefillView`], asked for by the step that has those fields
//! rather than carried on every reading above — see that type, where the
//! reasoning is.
//!
//! **A row is present, absent or neither.** Neither is the Windows sandbox
//! row, which is not a thing to install there — see [`DependencyState`]. A
//! present one carries the file a session would run, and an absent one carries
//! whatever the machine said about it: the failed `bwrap` run's own standard
//! error on Linux, and — for a name that is on this machine somewhere a session
//! cannot use it — where it was seen. See [`Seen`], which is the difference
//! between a program to install and a `PATH` to fix.
//!
//! **And the install run is the one thing here that is neither.** What the
//! wizard's first step does about a missing row is install it, and a run of
//! that is a thing that is happening rather than a thing that was read — see
//! [`RunView`] and [`InstallState`]. It is still not a setting: a run belongs
//! to this server's life and nothing about it is written down, so a Verkstead
//! that was restarted has none.
//!
//! **And where a session looks is the list itself.** A session's `PATH` is
//! composed out of the one the server was started with, behind the directories
//! Verkstead has itself installed into, so which directories those are is a fact
//! about this machine rather than about the platform — see
//! [`OnboardingView::path`].

use serde::{Deserialize, Serialize};

#[cfg(feature = "typescript")]
use ts_rs::TS;

/// Whether a fresh Verkstead can do anything yet, and what it would take.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct OnboardingView {
    /// Whether onboarding mode is on: the verdict of the objective, reached
    /// once at startup and standing until the wizard finishes.
    pub mode: bool,

    /// Whose rules this machine plays by, which is what the sandbox row and the
    /// wording around it are about.
    pub platform: Platform,

    /// And which install commands it takes, which is the tab that opens.
    pub distro: Distro,

    /// Every row of the dependencies step, in the order it is drawn.
    pub dependencies: Vec<DependencyView>,

    /// And where a session looks for a program, in the order it looks: the
    /// `PATH` a session is given, as this server composed it out of its own.
    ///
    /// **The list rather than a sentence about one.** What a session searches
    /// is what Verkstead installed into ahead of the server's own `PATH`, and
    /// that ahead of the platform's floor — see `sandbox::composed` — so which
    /// directories those are is a fact about *this* machine rather than about
    /// the platform, and a tab of written-down prose could not say it. A wizard
    /// telling somebody where to put a binary has to name the directories a
    /// session really looks in, which is the whole of why this is on the wire.
    ///
    /// Verkstead's own directory is not on it, that being the one entry
    /// holding nothing a human installs.
    pub path: Vec<String>,

    /// And every agent account already on this machine, in the order the
    /// harnesses above are drawn. Empty on a machine that has none, which is
    /// the step saying what to run rather than what to tick.
    pub accounts: Vec<AccountView>,

    /// And whether each of the three steps stands met, at this moment.
    pub steps: StepsView,

    /// And the install run, where one has been started in this server's life —
    /// going, or over and the last thing that happened here. See [`RunView`].
    ///
    /// Nothing until the first Next is pressed on the dependencies step, which
    /// is every reading a wizard nobody has pressed anything on draws.
    pub run: Option<RunView>,
}

/// The three platforms, as the viewer receives one.
///
/// Its own type beside [`Distro`], which carries the same fact for two of its
/// eight values: the distro is which set of commands to draw, and this is which
/// machine they are for — a sandbox row that ticks, one that is run, and one
/// that is nothing to install.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum Platform {
    Linux,
    MacOs,
    Windows,
}

/// And which of the wizard's eight tabs this machine is.
///
/// The five Linux distributions whose commands are written down, everything
/// else that is a Linux, and the two platforms whose answer is the platform's
/// own. Read off `/etc/os-release` — `ID` first and then `ID_LIKE`, so that a
/// derivative gets its parent's commands rather than the generic list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum Distro {
    MacOs,
    Windows,
    NixOs,
    Ubuntu,
    Fedora,
    Debian,
    Arch,

    /// A Linux naming none of them, which gets the generic list of what is
    /// needed rather than a command that would be wrong.
    OtherLinux,
}

/// One row of the dependencies step: a thing a session needs, and whether this
/// machine has it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct DependencyView {
    pub dependency: Dependency,
    pub state: DependencyState,

    /// And what the install run has made of it, where one has been started at
    /// all — see [`InstallState`], which is *installing*, *failed with why*, or
    /// nothing.
    ///
    /// Beside the state above rather than folded into it, because the two are
    /// different questions asked at the same moment: whether this machine has
    /// the thing is probed on every read, and whether Verkstead is in the
    /// middle of putting it there is what the run says. A row that is
    /// installing is a row that was absent — which is why it could be ticked —
    /// and the same row a moment later is present with nothing under it.
    pub install: InstallState,
}

/// What the install run has made of one row.
///
/// Flat on the wire — `{"install": "Failed", "why": "…"}` — the way
/// [`DependencyState`] is, so the viewer narrows on a field rather than
/// unwrapping a variant name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "install")]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum InstallState {
    /// Nothing. No run has been started, or this row was not ticked, or its
    /// unit has landed and the state beside this is what says how that went.
    ///
    /// A row still waiting for a unit further down the run reads this too: how
    /// far the run has got is the bar's question — see [`RunView::done`] — and
    /// a row saying *queued* would be a second answer to it.
    Idle,

    /// Its unit is running now.
    Installing,

    /// Its unit is over, and the thing is not installed.
    Failed {
        /// In the machine's own words: the first line the run printed on
        /// standard error — the package manager's or the vendor installer's,
        /// whichever unit this row was — or whatever a dismissed password
        /// dialog was refused in, and *cancelled* for a unit that was skipped
        /// rather than run.
        why: String,
    },
}

/// What a row is about.
///
/// Flat rather than a harness variant carrying an [`crate::AgentType`]: the
/// step draws seven rows with an instruction apiece, and which of them are
/// harnesses is a fact about the objective rather than about the drawing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum Dependency {
    /// What a session is run inside: `bwrap` on Linux, the seatbelt on macOS,
    /// and nothing at all on Windows.
    Sandbox,

    /// Which is not optional: every Conversation is a branch and a worktree.
    Git,

    Claude,
    Codex,
    Grok,
    OpenCode,

    /// The one row that never gates, GitHub being a choice rather than a
    /// dependency — see the objective in ADR-0016.
    Gh,
}

/// And whether the machine has it.
///
/// Flat on the wire — `{"state": "Absent", "trouble": "…"}` — so the viewer
/// narrows on a field rather than unwrapping a variant name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state")]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum DependencyState {
    /// A session would find it, and this is the file it would run.
    ///
    /// **Which file it is, is the point.** A distribution's `claude` too old to
    /// connect, shadowing the current one the human installed under their own
    /// home, is a row that ticks either way and two entirely different
    /// programs — so a row that is there says which one it found.
    Present {
        /// The path the name resolved to on a session's `PATH`: the entry it
        /// was found in with the name on the end of it.
        ///
        /// Nothing on the sandbox row of the two platforms where a sandbox is
        /// no program to find — Apple's own, and the identity a Windows session
        /// runs under. Every other present row has one.
        at: Option<String>,

        /// And the file that path finally lands on, where it is a link and the
        /// two are not the same file. Claude's native installer leaves
        /// `~/.local/bin/claude` pointing into its versions directory, and
        /// which version is about to run is the half worth reading.
        target: Option<String>,
    },

    /// It would not.
    Absent {
        /// What the machine said about it, where anything was said at all: the
        /// standard error of a `bwrap` that is installed and would not run,
        /// which is where an unprivileged user namespace that is switched off
        /// says so in its own words. Nothing where the answer was simply that
        /// no such program is on the sandbox's `PATH`.
        trouble: Option<String>,

        /// And where the name *was* seen, where it was seen somewhere a session
        /// cannot use it — see [`Seen`]. Nothing where it is on no `PATH` at
        /// all, which is a row with nothing to say beyond *install one*.
        seen: Option<Seen>,
    },

    /// It is not a thing on this platform: the Windows sandbox row, where a
    /// session's boundary is an identity rather than something to install.
    NotApplicable,
}

/// Where a program was seen that a session still cannot run.
///
/// The half of *absent* that is worth a sentence. A name is missing in three
/// ways that are not the same thing to do anything about, and a row saying only
/// *absent* would send somebody to install what they have already got: a
/// program on the server's own `PATH` and not on a session's is a shell profile
/// and a restart rather than an install.
///
/// Flat on the wire — `{"seen": "Beyond", "at": "…"}` — the way
/// [`DependencyState`] is, so the viewer narrows on a field rather than
/// unwrapping a variant name. The wording is the viewer's own, like the install
/// commands beside it: what is here is what the machine is, and what to say
/// about it is the same three sentences on every Verkstead.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "seen")]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum Seen {
    /// On the `PATH` the *server* was started with, in a directory a session's
    /// own does not hold: an `/opt/foo/bin`, or one of the `/mnt/c/…` entries
    /// WSL appends. The program is on this machine and no session can open it.
    Beyond {
        /// Where it was seen, with the name on the end of it.
        at: String,
    },

    /// Where a session looks, and a link into somewhere a session cannot
    /// reach: an install under `/opt` that no sandbox binds.
    Leading {
        /// The link, on a `PATH` entry a session has.
        at: String,

        /// And what it points at, which is the part a session cannot open.
        target: String,
    },

    /// Where a session looks, and a link with nothing at the end of it — what
    /// an uninstall leaves behind.
    Dangling {
        /// The link that leads nowhere.
        at: String,
    },
}

/// One account this machine already has, offered as the Agent Profile it would
/// be saved as.
///
/// **Found rather than configured.** The server looks in its own home for the
/// shapes a session mounts an account from, so what is here is an account some
/// agent wrote there by being logged into once. At most one per harness, each
/// shape being a fixed path under a home — which is the same fact an unnamed
/// Profile's uniqueness is per harness for.
///
/// **And it is the whole account**, in the shape the profile form sends one:
/// the wizard saves a ticked row by handing it straight back to the profile
/// create, with no name and the models this build knows for its harness, rather
/// than by naming paths of its own.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct AccountView {
    /// What was found, ready to be saved as it stands.
    pub account: crate::ProfileAccount,

    /// And whether the harness that runs it is on this machine — the same
    /// answer that harness's [`Dependency`] row carries, so a row is offered
    /// ticked or drawn greyed without the viewer pairing the two lists up. An
    /// account whose binary is missing is not one to make a Profile of yet.
    pub harness: bool,
}

/// Whether each of the wizard's three steps stands met, read at the moment the
/// endpoint is asked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct StepsView {
    /// A sandbox, `git`, and at least one of the four harnesses.
    pub dependencies: bool,

    /// At least one Agent Profile, however it was made.
    pub accounts: bool,

    /// And a git author: both halves of one, because that is what git asks for.
    /// The GitHub token is not in this — see ADR-0016.
    pub git: bool,
}

/// The install run as a whole: what it is doing, and how far it has got.
///
/// **One run at a time, and it is this server's.** The wizard's first step is
/// ticked and pressed, and what that press starts is a sequence of commands —
/// the elevated batch that installs this distribution's packages, and a vendor
/// installer for each row no package manager carries. This is what the install
/// screen is drawn from while they run: a status line, a bar, and the Cancel
/// beside it.
///
/// **The rows say the rest.** Which row is installing and which one failed is
/// on the row — see [`InstallState`] — because that is where the human is
/// looking for it. What is here is the run's own half: what it is about at this
/// moment, and how much of it is behind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct RunView {
    /// What the status line is about — see [`RunPhase`].
    pub phase: RunPhase,

    /// And the line itself, in words: *Waiting for the password dialog on
    /// ada-box*, *Installing bubblewrap, git*, and what the run came to once it
    /// is over.
    ///
    /// Written by the server rather than composed by the viewer, unlike every
    /// other sentence about this step: the words name this machine and the
    /// packages this distribution calls them, neither of which the viewer
    /// knows.
    pub status: String,

    /// How many of the ticked rows are behind the run, whether they were
    /// installed, refused or skipped.
    pub done: u32,

    /// And how many were ticked, which is what `done` is out of.
    pub total: u32,

    /// Whether Cancel has been pressed and the unit under way has not finished
    /// yet.
    ///
    /// A press cannot stop a package manager that is already running — a
    /// half-installed machine is worse than a fully installed one — so what
    /// Cancel does is skip what has not started. This is the window between the
    /// press and the run reaching that point.
    pub cancelling: bool,
}

/// What the status line is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum RunPhase {
    /// A password dialog is up on the machine Verkstead is running on, and
    /// nothing else can happen until somebody answers it.
    ///
    /// Its own phase because of who is reading it: the human may be on a phone
    /// on the tailnet, and a wizard that sat saying *installing* while a dialog
    /// waited on a screen in another room would be a wizard that looked stuck.
    /// The status line names the machine for that reason.
    Asking,

    /// Something is being installed.
    Installing,

    /// The run is over: everything ticked is either installed or on the hint
    /// screen with a reason under it.
    Done,
}

/// The press that starts one: the rows that were ticked.
///
/// Every one of them is a row the reading said was absent. A row that is
/// present is not offered a checkbox to tick, and one this machine cannot
/// install is not known in advance — what cannot be done is reported by the run
/// rather than refused at the door, because what Verkstead can install is a fact
/// about the distribution rather than about the row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct InstallPress {
    /// What to install, in any order: the run puts them in its own.
    pub dependencies: Vec<Dependency>,
}

/// What this machine can offer the git step, for each field Verkstead has not
/// been told yet.
///
/// **Its own read, beside [`OnboardingView`] rather than in it.** The reading
/// above is probed every ten seconds while a step is unmet and again by the
/// workbench's own gate on every start, and neither of those has any business
/// running `git config` — or handing a GitHub token to a page that is drawing
/// a sidebar. This is asked for once, by the step that has the fields, and
/// only while they are still empty.
///
/// **Found rather than configured, and only where nothing is configured.** A
/// value Verkstead already holds is what the field shows, so it is not
/// prefilled over: what is here is what the machine could tell a Verkstead
/// that has been told nothing. A field nothing answered for is absent, which
/// is a field that stays empty until somebody types in it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct PrefillView {
    /// Who the machine's own git commits are by.
    pub name: Option<Prefilled>,

    /// And what address they carry.
    pub email: Option<Prefilled>,

    /// And a GitHub token this machine is already holding somewhere — which is
    /// the one optional field of the three, GitHub being a choice.
    pub token: Option<Prefilled>,
}

/// One field's prefill: what was found, and where.
///
/// The source travels with the value because the human is being asked to
/// confirm something they did not type: a name off a `git config` and a token
/// out of an environment variable are two different things to be sure about,
/// and a field that only showed the value would be asking them to trust it
/// blind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct Prefilled {
    pub value: String,
    pub source: Source,
}

/// And where a prefill came from.
///
/// Four values rather than a variable name carried as a string: the wording
/// around each of them is the viewer's, the way the install commands are, and
/// which environment variable answered is part of what there is to say.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum Source {
    /// `git config --global`, run by the server as whoever it is.
    GitConfig,

    /// `GH_TOKEN` in the server's own environment, which is where a token
    /// meant for everything on this machine is usually put.
    GhToken,

    /// And `GITHUB_TOKEN`, where `GH_TOKEN` said nothing.
    GithubToken,

    /// And the login the host's own `gh` is holding — `gh auth token`, asked
    /// last because it is the one of the three that is a process.
    HostGh,
}
