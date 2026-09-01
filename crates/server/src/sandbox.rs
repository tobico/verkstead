//! The sandbox a session runs in: a surface built around one Conversation's
//! worktree, and nothing else.
//!
//! **What is inside is one description, and the mechanism under it is the
//! platform's.** [`Sandbox::surface`] says what a session may reach, once; a
//! renderer turns that into bubblewrap's flags on Linux — see [`bwrap`] — or
//! into a deny-by-default policy on a Mac — see [`seatbelt`]. Neither rendering
//! is the description, the two are not the same boundary, and nothing above
//! this module learns which one it got (ADR-0012).
//!
//! Evolved from `tobico-scripts/bin/sandbox`, which is the working reference —
//! but narrowed where it matters. That script binds the whole of `~/src`
//! read-write, so every session it starts can reach every repository on the
//! machine; here the only checkout inside is the Conversation's own. The
//! filesystem is the boundary, and a boundary with everything behind it is not
//! one.
//!
//! What is inside:
//!
//! - **read-write** — the Conversation's worktree, the Repo's common `.git`
//!   directory, and the Profile's pair at `~/.claude` and `~/.claude.json`
//! - **read-only** — `/nix` and the system paths, the bundled skills in a
//!   directory of Verkstead's own — see [`own_directory`] — nothing at all
//!   where the account's own skills would be found, and the executable serving
//!   all this, as `verkstead`
//! - **tmpfs** — `/tmp`, and everything else in HOME simply absent
//! - **by mode** — each companion repo the Conversation was configured with,
//!   its worktree and its git directory together, read-only or read-write as
//!   the human said — see [`companion_binds`]
//! - **the shared Rust build cache** — one directory of Verkstead's own,
//!   writable, plus the sccache it compiles through read-only beside the
//!   `verkstead` binary, so a crate is downloaded and compiled once for the
//!   machine rather than once per Conversation — see [`crate::build_cache`]
//!
//! That last one has a process outside every sandbox to go with it: the sccache
//! *server*, which is what actually runs `rustc`, and which Verkstead runs in a
//! sandbox of its own rather than leaving each session to start one. What the
//! sandbox composed here holds is only the client's half. See
//! [`crate::build_cache::BuildCache::compiling`].
//!
//! Credentials are on none of those lists, and neither is who a session commits
//! as. Both arrive in the environment out of the settings files the human filled
//! in — see [`crate::settings`] — GitHub auth as `GH_TOKEN`, and the whole of
//! git's configuration as `GIT_CONFIG_COUNT` and the pairs it counts. So there
//! are no gh files inside a sandbox and no `.gitconfig` either, and no question
//! of which account a session turned out to be running as.
//!
//! The network is not a boundary here: it is shared with the host, whole and
//! unfiltered. An agent has to reach GitHub, the npm registry, the model's API
//! and whatever a build downloads, and an allowlist that has to hold all of that
//! is one nobody will keep honest. What stops a session doing damage is that
//! there is nothing to damage: it can push the branch it is on and write the
//! worktree it is in. A proxy allowlist can come later, in front of this, and
//! the seam for it is that nothing here reads the network's absence.

// Built wherever the tests are rather than on its own platform alone: a
// rendering is a description going in and a command coming out, so the arm this
// machine will never run is still an arm its tests call — the same reason
// `crates/desktop`'s startup registrations are all built here.
#[cfg(any(not(target_os = "macos"), test))]
mod bwrap;
#[cfg(any(target_os = "macos", test))]
mod seatbelt;
mod surface;

// And which of them a session actually gets, which is the one thing about the
// seam that is a `cfg` rather than a description: bubblewrap wherever there is
// a kernel with namespaces to unshare, and Apple's own on a Mac.
#[cfg(not(target_os = "macos"))]
use bwrap::command as render;
#[cfg(target_os = "macos")]
use seatbelt::command as render;

use std::ffi::OsStr;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

// And the description itself, which is not this module's alone: the compile
// server outside every session is composed of the same vocabulary and rendered
// by the same renderer — see [`crate::build_cache`], and [`on_the_machine`]
// for the part of it the two share.
pub(crate) use surface::{Access, Reach, Surface};

use crate::build_cache::{self, BuildCache};
use crate::handoffs::{self, Handoffs};
use crate::platform::Platform;
use crate::settings::{Config, GitAuthor, Secrets};
use crate::skills::{self, Skills};
use crate::store;
use crate::terminal;

/// The system directories a sandbox gets read-only on this machine, in the
/// order they are said.
///
/// A `cfg!` rather than a `cfg`, for the reason [`crate::platform::Platform`]
/// is a value rather than one: what the system *is* differs by platform and the
/// list that says so is a fact either way, so both are here and both are
/// readable by a test on whichever machine is running it.
pub(crate) const SYSTEM: &[&str] = if cfg!(target_os = "macos") {
    APPLE_SYSTEM
} else {
    LINUX_SYSTEM
};

/// What that is on Linux.
///
/// `/nix` is the whole of what makes the box usable — every binary a session
/// runs is under it — and the rest is what a dynamic linker, a certificate store
/// and a shell need to exist at all. `/run/current-system` is NixOS's own, and
/// is what `PATH` below points at.
///
/// One that is not there is skipped rather than refused: `/lib64` is an x86
/// fact, and a machine without `/run/current-system` is one that is not NixOS,
/// which is a thing to notice elsewhere than in a mount table.
///
/// `/lib` and `/lib64` are both here because a shell off NixOS needs both and
/// says so in a way that reads as something else entirely: Ubuntu's `/bin/sh` is
/// `dash`, whose interpreter is under `/lib64` but whose `libc.so.6` is under
/// `/lib`, and `execvp` reports a library it cannot find as `No such file or
/// directory` — naming the binary rather than the library. On a merged-`/usr`
/// distribution neither is a widening: both are symlinks into `/usr`, which is
/// bound above, so this reaches the same files by their other name. It is on
/// NixOS that they are nothing, which is why their absence went unnoticed —
/// there, `/nix` is the whole answer.
///
/// The compile server gets the same list, because what makes a machine usable
/// is the same whichever of the two is reading it — see
/// [`crate::build_cache::BuildCache::compiling`].
const LINUX_SYSTEM: &[&str] = &[
    "/nix",
    "/usr",
    "/bin",
    "/lib",
    "/lib64",
    "/etc",
    "/run/current-system",
];

/// And what it is on a Mac, which shares not one entry with it.
///
/// `/System/Library` rather than `/System`, and deliberately: the data volume
/// is firmlinked in under `/System/Volumes/Data`, so a session given the whole
/// of `/System` would reach every home directory on the machine by its other
/// name. What is here is the frameworks, the shared cache the dynamic linker
/// maps out of the cryptex, the machine-wide `/Library`, and the tools —
/// Apple's own under `/usr` and `/bin`, Homebrew's where somebody installed it,
/// and nix's where a Mac is running nix-darwin.
///
/// `/var/select` is the one that reads as nothing: `/bin/sh` on a Mac reads
/// `/private/var/select/sh` to decide which shell it is being run as, and a
/// session without it has a shell that will not start.
///
/// One that is not there is skipped, as on Linux and for the same reason: a
/// Mac without Homebrew is a Mac without Homebrew, which is a thing to notice
/// elsewhere than in a policy.
///
/// `/run/current-system` is nix-darwin's, and it is the one entry here that is
/// about a Mac somebody has been to some trouble over: that machine's tools are
/// under it, [`APPLE_PATH`] looks there, and a Mac without nix-darwin skips it
/// the way a machine without Homebrew skips `/opt/homebrew`.
const APPLE_SYSTEM: &[&str] = &[
    "/System/Library",
    "/System/Volumes/Preboot/Cryptexes",
    "/Library",
    "/usr",
    "/bin",
    "/sbin",
    "/etc",
    "/var/select",
    "/opt/homebrew",
    "/nix",
    "/run/current-system",
];

/// Where a session finds the product itself: the skills it is grilled by, and
/// under `bin` the executable it asks with.
///
/// `/verkstead` on Linux, and it is nowhere on the host — the binds make it,
/// which is what says everything in it is the server's own rather than whatever
/// the machine happened to have under that name.
///
/// A Mac has neither half of that. There are no binds to make a directory with,
/// and nothing can be made at `/` to be found in one: the root volume is
/// read-only and sealed, and the one supported way past that is
/// `/etc/synthetic.conf`, which is root's and wants a reboot — neither of which
/// an app a human dragged out of a dmg has. So there it is the Data Directory.
/// The skills are already written out inside it, the binary is linked in beside
/// them, and no other part of it is reachable from a session at all — so what
/// is in it is still the server's own, said by the policy rather than by the
/// directory being nobody's. What a session is then told to read is the path
/// the file is really at.
///
/// Windows keeps the Linux spelling, having no sandbox at all yet: the stage
/// that gives it one is the stage that decides what a directory of Verkstead's
/// own is there.
pub fn own_directory(platform: Platform, data_dir: &Path) -> PathBuf {
    match platform {
        Platform::MacOs => data_dir.to_owned(),
        Platform::Linux | Platform::Windows => PathBuf::from(OWN_DIRECTORY),
    }
}

/// What that directory is where a bind makes it.
const OWN_DIRECTORY: &str = "/verkstead";

/// And the two things inside it: the directory the executables are in, which
/// goes first on a session's `PATH`, and what the server's own image is called
/// there.
///
/// A directory rather than a name inside one of the system binds: those are the
/// host's and read-only, so there is nowhere in them to put a file. It holds
/// the one executable the server put there and nothing else.
const BIN: &str = "bin";
const VERKSTEAD: &str = "verkstead";

/// Where each agent type's account lands in HOME.
///
/// Claude's pair, and the one directory each backend after it keeps its whole
/// account under. Written out rather than derived, because they are the paths
/// those programs look in and not a scheme any of them follows — and named
/// apart from [`skills::CLAUDE_INSIDE_HOME`], which is the directory *inside*
/// the first of them that a sandbox covers.
const CLAUDE_DIR_INSIDE_HOME: &str = ".claude";
const CLAUDE_CONFIG_INSIDE_HOME: &str = ".claude.json";
const CODEX_INSIDE_HOME: &str = ".codex";
const GROK_INSIDE_HOME: &str = ".grok";

/// And OpenCode's, which is two of them rather than one: it keeps no
/// dot-directory of its own and reads the XDG base directories instead,
/// appending `opencode` to each — read off opencode 1.18.25, which is the
/// release this backend is pinned at, and which makes all four at startup.
///
/// **These are the paths inside the Profile's home as well as inside HOME.**
/// The sandbox's HOME is made fresh and empty, so the XDG defaults resolve
/// inside it and nothing has to be set in the environment (ADR-0011 allowed
/// either shape, and this is the cheaper one); and a Profile's home is an
/// opencode home too, being the directory left behind by running
/// `HOME=<it> opencode` once to log the account in. So one relative path
/// serves both ends of each bind, which is what says the two homes are the
/// same shape.
///
/// **The config and the data, and not the other two.** The data directory is
/// the account — `auth.json` and the session store are in it — and the config
/// directory is what the human configures that account with, so both travel
/// with the Profile. The cache and the state directories are neither: the
/// cache holds what opencode downloaded for this machine and the state holds
/// the TUI's own furniture, and both are derived things that a session is
/// welcome to make fresh in the HOME it is thrown away with. What that costs
/// is a re-download of whatever tooling opencode fetches, once per session.
pub(crate) const OPENCODE_CONFIG_INSIDE_HOME: &str = ".config/opencode";
pub(crate) const OPENCODE_DATA_INSIDE_HOME: &str = ".local/share/opencode";

/// What opencode is told to call the store it writes under the data directory.
///
/// Said rather than left to opencode, which names the file after the release
/// channel the install came from — a beta build writes `opencode-beta.db`
/// beside a stable build's `opencode.db`. Whichever channel the host installed,
/// a session under Verkstead writes the one file Verkstead named, so the reader
/// that follows a session's Transcript opens a path this chose rather than
/// guessing which of several is this session's — see [`crate::records`], which
/// reads this very constant.
///
/// A bare filename rather than a path: opencode resolves a relative
/// `OPENCODE_DB` against its own data directory, which is the account.
///
/// Named for the reason the idle signature and the usage-limit phrase are: it
/// is somebody else's spelling, and moving it costs one edit here. Read off
/// opencode 1.18.25, which is the release this backend is pinned at: a stable
/// install of that names the file `opencode.db` on its own, so what this is
/// worth is the day the host installs a beta.
const OPENCODE_DB: &str = "OPENCODE_DB";
pub(crate) const OPENCODE_DB_FILE: &str = "opencode.db";

/// And how long a command opencode's shell tool will run before it kills it,
/// where the model passed no timeout of its own.
///
/// **This is what keeps a blocking ask from being killed under a session that
/// ignored its Guide.** `verkstead ask` blocks until the human answers and the
/// human is on a phone, so the wait is measured in hours; the shell tool's own
/// default is two minutes, and its description tells the model that a command
/// with no timeout is killed after it. So the Guide tells an OpenCode session
/// to pass a large one — see the CLI's `running-opencode.md` — and this raises
/// what a session that passed nothing gets. Both, because the instruction is
/// the mechanism and this is what stands under a drifted instruction: an ask
/// killed two minutes in is an answer the human gave to nobody.
///
/// A day, which is the same order as the wait itself: a Set asked at the end of
/// an evening is answered the next morning, and nothing about an ask expires
/// before that — see ADR-0001. What it costs is the tool's own guard against a
/// command that hangs for reasons of its own: with the default in place such a
/// command is killed after two minutes and the model carries on, and with this
/// it is not. Nothing else here would catch one either — opencode repaints its
/// at-work label for as long as the tool holds a command, so the session reads
/// at work and the byte-quiet long-stop behind the screen never comes — so
/// what is left of that case is the Stop the human presses. Taken deliberately:
/// a hung command is rare and is drawn on the Screen while it hangs, and an ask
/// killed under a session waiting on the human loses an answer that was given.
///
/// Read off opencode 1.18.25, the release this backend is pinned at, and named
/// for the reason the store's name above is: the spelling is opencode's, it
/// says `EXPERIMENTAL` on it, and moving it costs one edit here. A value it
/// cannot read as a positive integer is ignored rather than refused, so the
/// number is written out in milliseconds rather than computed into one.
const OPENCODE_BASH_DEFAULT_TIMEOUT: &str = "OPENCODE_EXPERIMENTAL_BASH_DEFAULT_TIMEOUT_MS";
const OPENCODE_BASH_DEFAULT_TIMEOUT_MS: &str = "86400000";

/// Which backend a session is running, in its own environment.
///
/// Set for the Guide alone. Nothing else inside a sandbox needs to know — the
/// asking channel and the idle judgement are both decided server-side — but
/// `verkstead guide` prints the asking instructions for the backend reading it,
/// and there is nothing else inside a sandbox that says which one that is.
///
/// A Guide printed where this is unset is the blocking one, which is what a
/// `verkstead` run outside a sandbox altogether gets.
pub const AGENT_TYPE: &str = "VERKSTEAD_AGENT";

/// Where the executables of Verkstead's own are inside a sandbox: `bin` under
/// [`own_directory`].
///
/// Two things are in it and neither is the host's — the binary a session asks
/// with, and the sccache the shared build cache compiles through — so it is
/// what leads a `PATH` inside, in a session's sandbox and in the compile
/// server's alike. Made by the bind on Linux and really there on a Mac, which
/// is why it is a function of where the Data Directory is rather than a name.
/// See [`path`], [`Executable`] and [`crate::build_cache`].
pub(crate) fn own_bin(platform: Platform, data_dir: &Path) -> PathBuf {
    own_directory(platform, data_dir).join(BIN)
}

/// What a session's `PATH` is inside: `ours` first, and then the machine's own.
///
/// `ours` is the directory of Verkstead's own that the binary a session asks
/// with is in — see [`Executable`] for why a session asks with the server's own
/// build rather than with whatever the machine has installed. It goes first on
/// both platforms, and it is passed in rather than said here because where it
/// is, is a fact about the machine too: `/verkstead/bin` where a bind makes it,
/// and a directory under the Data Directory where nothing can — see
/// [`own_directory`].
///
/// Nothing here is inherited from the server's own environment: what a session
/// can run should be a fact about the sandbox rather than about however the
/// unit that started the orchestrator happened to be launched.
///
/// And what the compile server has, for the same reason: it is a fact about
/// what a sandbox holds rather than about either process.
pub(crate) fn path(ours: &Path) -> String {
    format!("{}:{MACHINE_PATH}", ours.display())
}

/// The floor every sandbox Verkstead makes stands on, whatever it is for: the
/// system read-only, a process table of its own, the device nodes a program
/// opens by name, and somewhere to write a temporary file.
///
/// Said here rather than twice, because there are two sandboxes and they
/// share this much of what they hold — a session's own, and the compile
/// server every session's `rustc` goes through (see
/// [`crate::build_cache::BuildCache::compiling`]). What each adds on top of
/// it is its own, and so is where it starts: `chdir` is a Conversation's
/// Worktree for one of them and the compile server's own HOME for the other.
///
/// What is not on the machine is skipped rather than said: a rule about a
/// path that does not exist is a rule about nothing, and on Linux a bind of
/// one is a sandbox that will not start.
pub(crate) fn on_the_machine(chdir: PathBuf) -> Surface {
    let mut surface = Surface::starting_in(chdir);

    for path in SYSTEM.iter().map(Path::new).filter(|path| path.exists()) {
        surface.own(path, Reach::ReadOnly);
    }

    surface
        .made(Access::ProcessTable)
        .made(Access::Devices)
        .made(Access::Temporary(PathBuf::from(TMP)));

    surface
}

/// And `surface` as the command that runs it, by whichever mechanism this
/// machine has one.
///
/// The one place either rendering is reached from, so that a sandbox is a
/// description going in and a command coming out wherever one is made.
pub(crate) fn rendered(surface: &Surface) -> Command {
    render(surface)
}

/// The machine's own half of that, which is one list or the other and never
/// both: a NixOS box has nothing under `/opt/homebrew` and a Mac has nothing
/// under `/run/current-system/sw` unless somebody put it there.
const MACHINE_PATH: &str = if cfg!(target_os = "macos") {
    APPLE_PATH
} else {
    LINUX_PATH
};

/// What that is on Linux: the system profile, then the Nix default profile,
/// then the paths a non-NixOS `/usr` would put things in.
const LINUX_PATH: &str =
    "/run/current-system/sw/bin:/nix/var/nix/profiles/default/bin:/usr/bin:/bin";

/// And on a Mac, which has none of NixOS in it until somebody installs one.
///
/// Homebrew first — both the Apple-silicon prefix and the Intel one, which is
/// under `/usr/local` — because a Mac used for development has its actual
/// toolchain there and Apple's own `/usr/bin` holds older copies of half of it.
/// Then the system, which is what is there on a Mac nobody has touched.
///
/// Then nix's, last and present at all: a Mac running nix-darwin has tools
/// under `/run/current-system/sw/bin` that exist nowhere else on it, and a Mac
/// without one has an entry on its `PATH` that resolves to nothing, which costs
/// a session nothing. So neither kind of machine is made to do without the
/// other's — see [`APPLE_SYSTEM`], which lets the same directory be reached.
const APPLE_PATH: &str = "/opt/homebrew/bin:/opt/homebrew/sbin:/usr/local/bin:/usr/bin:/bin:\
                          /usr/sbin:/sbin:/run/current-system/sw/bin:\
                          /nix/var/nix/profiles/default/bin";

/// And what a session's `SHELL` is: the one path every platform this runs on is
/// certain to have a shell at.
///
/// The one thing about the environment with no arm of its own. `/bin/sh` is
/// NixOS's and a Mac's alike — Apple's is what reads `/private/var/select/sh`
/// on its way up, which is why that path is in [`APPLE_SYSTEM`] — so the answer
/// that was right for one machine is right for the other.
const SHELL: &str = "/bin/sh";

/// Where a session writes a temporary file.
///
/// The one thing in the description whose two renderings differ in what they
/// leave behind rather than in how they are spelled: on Linux this is a
/// filesystem of the session's own, holding nothing of the host's and gone when
/// the session is, and on a Mac it is the machine's own directory of that name
/// with a rule about it — see [`seatbelt`].
const TMP: &str = "/tmp";

/// GitHub over HTTPS, which is the one host a sandbox is given credentials for
/// and the one every SSH remote is rewritten to — see [`Sandbox::git_config`].
///
/// Without a trailing slash, because a credential scope has none and the URL
/// rewrite says its own.
const GITHUB: &str = "https://github.com";

/// Where a session's HOME comes from, on whichever platform is asking.
///
/// **On Linux it is the server's own directory, at the same path inside.**
/// Nothing of what is in it comes through: an empty directory is made over it,
/// and the Profile's account mounted into that — which is what makes one
/// account's sessions invisible to another's. Nothing is read out of it either.
/// It used to give up two things, what `gh` was authenticated as and who git
/// committed as, and both are now said rather than found — a token and an
/// author in the settings files, handed to the session in its environment. What
/// is left is a path, not a place credentials are collected from.
///
/// **On a Mac there is nothing to make a directory *over*.** A policy can
/// refuse a path and cannot empty one, so the directory a session gets has to
/// really be one: Verkstead's own, under the Data Directory beside the
/// handoffs, one per Conversation and made fresh as each session starts. The
/// server's own home is then no session's HOME at all, and is refused along
/// with the rest of the machine. What keeps one account out of another's is the
/// policy rather than the absence — everything outside this session's own is
/// denied — and what keeps one Conversation out of another's is that they are
/// different directories.
///
/// One is emptied as each of that Conversation's sessions starts rather than
/// removed when the Conversation ends. What a session left in it is nothing
/// anything reads — the account is linked in rather than copied, so what is
/// there is the session's own leavings — and a Conversation's id is never
/// handed out twice, so the only thing a directory left behind can ever be
/// given to is the Conversation it already belonged to.
#[derive(Debug, Clone)]
pub struct Homes {
    /// The home of whoever is running the server, which is what `~` means
    /// inside on the platform that can make an empty one over it.
    servers: PathBuf,

    /// And the root a Conversation's own real one is made under, on the
    /// platform that cannot.
    root: PathBuf,

    /// Which of those a session gets. A value rather than a `cfg`, for the
    /// reason [`Platform`] is one: the arm this machine will never run is still
    /// an arm a test on it can ask for.
    platform: Platform,
}

impl Homes {
    /// The homes this server hands out.
    ///
    /// The server's own is read from the environment rather than from the
    /// passwd database: a service unit says what HOME is, and that is the answer
    /// that should count — under the packaged unit it is what the module sets,
    /// and in a development shell it is the human's own. `None` where nothing
    /// says, which on Linux is a server that can run no session; a Mac needs it
    /// for nothing here, and is refused with the rest for the sake of one
    /// answer rather than two.
    pub fn of_the_server(data_dir: &Path) -> Option<Homes> {
        Some(Homes::on(
            Platform::HERE,
            PathBuf::from(std::env::var_os("HOME")?),
            data_dir,
        ))
    }

    /// The same on `platform`, out of the two directories it decides between.
    pub fn on(platform: Platform, servers: PathBuf, data_dir: &Path) -> Homes {
        Homes {
            servers,
            root: data_dir.join("homes"),
            platform,
        }
    }

    /// The home of whoever is running the server, for the startup line that
    /// says which machine this is running on.
    pub fn servers(&self) -> &Path {
        &self.servers
    }

    /// And one Conversation's own.
    fn for_conversation(&self, conversation_id: i64) -> Home {
        let path = match self.platform {
            Platform::MacOs => self.root.join(conversation_id.to_string()),
            Platform::Linux | Platform::Windows => self.servers.clone(),
        };

        Home { path }
    }
}

/// The directory `~` means inside one sandbox.
///
/// Made rather than constructed: it is [`Homes`] that decides where a session's
/// own is, and a HOME the caller chose is a directory a renderer would empty.
#[derive(Debug, Clone)]
pub struct Home {
    path: PathBuf,
}

impl Home {
    /// The directory itself, which is what `~` resolves to inside.
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}

/// The Verkstead executable a session is given, which is the one serving it.
///
/// One binary carries both verbs — `verkstead serve` and `verkstead ask` — so a
/// running server has on disk exactly what a session needs, and handing it that
/// one is what keeps the two halves of an ask the same build. They share a
/// schema, a Guide and a wire format, and two builds that have drifted apart
/// cannot put a Question Set through together at all: the installed CLI this
/// replaces validated a `proposal` locally against a field the running server
/// refused as unknown, so no grilling could reach its closing move.
///
/// A machine's own install is therefore not a fallback. A session asking with a
/// binary nobody chose is the failure this removes, and where the server cannot
/// find its own image the session is not started at all, and what is logged is
/// which session that cost.
#[derive(Debug, Clone)]
pub struct Executable {
    path: PathBuf,

    /// And where a session finds it, which is `bin/verkstead` under the
    /// directory of Verkstead's own — see [`own_directory`]. Made by the bind
    /// on Linux and really there on a Mac, and first on a session's `PATH`
    /// either way.
    inside: PathBuf,
}

impl Executable {
    /// The running server's own image, as a session started against `data_dir`
    /// finds it.
    ///
    /// `None` where the process cannot say what it is running, and `None` too
    /// where what it names is no longer a file: a binary replaced under a
    /// running server is exactly that, and `/proc` answers for it with a path
    /// marked `(deleted)` that no bind can be made from.
    pub fn of_the_server(data_dir: &Path) -> Option<Executable> {
        Executable::at(std::env::current_exe().ok()?, data_dir)
    }

    /// A named one, which is how a test puts the real CLI where the server's own
    /// image goes — a test harness being its own executable.
    ///
    /// `None` for a path with nothing behind it, for the reason above: what this
    /// is for is a bind, and a bind of nothing is a session that will not start.
    pub fn at(path: PathBuf, data_dir: &Path) -> Option<Executable> {
        let path = unwrapped(&path);

        path.is_file().then_some(Executable {
            path,
            inside: own_bin(Platform::HERE, data_dir).join(VERKSTEAD),
        })
    }

    /// Where it is on the host, which is what a sandbox binds.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Where a session finds it, which is what a sandbox puts it at.
    fn inside(&self) -> &Path {
        &self.inside
    }

    /// And the directory it is in inside, which is what goes first on a
    /// session's `PATH` — see [`path`].
    fn bin(&self) -> &Path {
        self.inside
            .parent()
            .expect("a path built by joining two names onto a directory has one")
    }
}

/// The wrapper beside `path` where `path` is what nix's `wrapProgram` left
/// behind one, and `path` itself everywhere else.
///
/// The packaged `verkstead` is a wrapper script that puts `git` on the CLI's
/// `PATH`, with the real executable beside it under a name that begins with a
/// dot and ends in `-wrapped`. A packaged server *is* that second one, because
/// that is what the wrapper execed — so binding what the process says it is
/// running would hand a session the binary without the wrapper's doing, and the
/// CLI shells out to git for a Set's project, its branch and its Diff.
fn unwrapped(path: &Path) -> PathBuf {
    let Some(wrapped) = path.file_name().and_then(OsStr::to_str) else {
        return path.to_owned();
    };

    let Some(name) = wrapped
        .strip_prefix('.')
        .and_then(|name| name.strip_suffix("-wrapped"))
    else {
        return path.to_owned();
    };

    let wrapper = path.with_file_name(name);

    if wrapper.is_file() {
        wrapper
    } else {
        path.to_owned()
    }
}

/// Where Verkstead is reachable from inside a sandbox, before the Conversation a
/// session is asking from.
///
/// The one thing a session is given that is not a directory. The network inside a
/// sandbox is the host's own — see this module's own documentation — so what a
/// session dials is what anything else on the machine would.
///
/// An address the server was told to listen on for the sake of the tailnet is
/// still that address. An unspecified one — `0.0.0.0`, `[::]` — is every
/// interface at once, and there is no such thing to put in a URL: the loopback
/// is the one of them certain to answer, and it is the one a session shares.
#[derive(Debug, Clone)]
pub struct Reachable {
    base: String,
}

impl Reachable {
    /// Where a server listening on `listen` can be reached.
    pub fn at(listen: SocketAddr) -> Reachable {
        let host = match listen.ip() {
            IpAddr::V4(ip) if ip.is_unspecified() => IpAddr::V4(Ipv4Addr::LOCALHOST),
            IpAddr::V6(ip) if ip.is_unspecified() => IpAddr::V6(Ipv6Addr::LOCALHOST),
            named => named,
        };

        Reachable {
            // Through `SocketAddr` rather than by hand, because that is what puts
            // the brackets round an IPv6 address — `[::1]:8422` is a URL and
            // `::1:8422` is not a number anything could parse.
            base: format!("http://{}", SocketAddr::new(host, listen.port())),
        }
    }

    /// The base URL a session on `conversation_id` is given, which is what makes
    /// its Question Sets that Conversation's.
    ///
    /// Explicit rather than inferred: the CLI derives the project and the branch
    /// from the working directory, and two Conversations against one Repo would
    /// be indistinguishable by either.
    pub(crate) fn asking_from(&self, conversation_id: i64) -> String {
        format!("{}{}/{conversation_id}", self.base, crate::ASKING_FROM)
    }
}

/// One directory bound into a sandbox beyond the surface every one of them has,
/// and how far into it a session may reach.
///
/// The reach travels with the path because the two are one decision. Everything
/// bound out here used to be read-write — a build cache is written to or it is
/// no cache at all — and a companion repo added read-only is a checkout a
/// session is meant to read and leave alone. A list of bare paths could not say
/// which of the two a directory is, and a second list beside it would be two
/// things to keep in step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bind {
    path: PathBuf,

    reach: Reach,
}

impl Bind {
    /// One a session may write in: a build cache, a read-write companion's
    /// checkout.
    pub fn writable(path: PathBuf) -> Bind {
        Bind {
            path,
            reach: Reach::ReadWrite,
        }
    }

    /// And one it may only read: a read-only companion's checkout, and the git
    /// directory behind it.
    pub fn readable(path: PathBuf) -> Bind {
        Bind {
            path,
            reach: Reach::ReadOnly,
        }
    }
}

/// Sandbox Configuration: the extra read-write binds a sandbox gets beyond the
/// surface every one of them has.
///
/// Two sets, composed. The global one every sandbox gets, and a per-Repo one so
/// that a repository needing a build cache can say so without every repository
/// getting it. This type is the *installation's* half of them: what
/// `--sandbox-bind` was given, resolved once and kept. The settings file says
/// binds too, in the same two grammars and composed the same way — see
/// [`SandboxConfig::settings_binds`], which is where the two part company, in
/// what a bind that will not resolve costs.
///
/// A Repo is named by its *name*, which is the directory's own: the human writes
/// what they call the repository rather than a path they would then have to keep
/// in step with the registration. Two Repos of one name in different places
/// therefore share what is configured for the name, which is a collision the
/// human can see in their own configuration and rename their way out of.
#[derive(Debug, Clone, Default)]
pub struct SandboxConfig {
    /// What every sandbox gets.
    global: Vec<PathBuf>,

    /// And what only the Repo of that name does.
    per_repo: std::collections::BTreeMap<String, Vec<PathBuf>>,
}

impl SandboxConfig {
    /// The binds as configured, checked once at startup.
    ///
    /// Each is either an absolute path — a global bind — or `name=path` for the
    /// Repo called `name`. The two are told apart by the leading `/`, so a Repo
    /// can be called anything without a spelling of it turning into a path.
    ///
    /// A bind that is not there is refused rather than skipped, which is where
    /// this parts company with the settings files: a setting nobody has filled in
    /// is an installation part-way through being set up, and a missing configured
    /// bind is a typo that would otherwise take every session in that repository
    /// down with it, weeks later, with nobody watching. Not created either: the
    /// path is the human's own word, so a directory made where they meant
    /// another one is an empty cache that looks like a working one.
    ///
    /// Which is the whole of what makes [`crate::build_cache`] the exception it
    /// is. That directory is Verkstead's own choice on a fresh install rather
    /// than something typed, there is nothing in it for a typo to hide, and a
    /// feature that is on by default cannot ask the human to `mkdir` first — so
    /// it is made, and only failing to make it refuses startup.
    pub fn resolve(binds: &[String]) -> anyhow::Result<SandboxConfig> {
        let mut config = SandboxConfig::default();

        for bind in binds {
            let (repo, path) = read_bind(bind)?;

            if !path.exists() {
                anyhow::bail!(
                    "the sandbox bind {} is not there: a bind Verkstead cannot make is one \
                     every session it applies to would fail to start on",
                    path.display()
                );
            }

            match repo {
                Some(repo) => config.per_repo.entry(repo).or_default().push(path),
                None => config.global.push(path),
            }
        }

        Ok(config)
    }

    /// And what `config.yaml` asks for, resolved afresh for `conversation` at
    /// the moment its session spawns.
    ///
    /// The same grammar and the same composition as [`SandboxConfig::resolve`]
    /// and [`SandboxConfig::binds_for`] — one bind is one bind, however it was
    /// said — and the opposite answer to a bind that will not resolve. The two
    /// sets compose: what is here is added to what the installation configured
    /// rather than standing in for it.
    ///
    /// **Nothing here is ever an error.** An entry that is neither a path nor
    /// `name=path`, and one naming a directory the server cannot see — never
    /// made, or outside what a hardened unit's namespace holds — is skipped with
    /// a line in the log naming it, and the session starts without it. That is
    /// the settings side of the line the whole of [`crate::settings`] is on: the
    /// file is edited from a phone, a save lands whatever it was told, and a
    /// typo in it is a bind that is missing rather than every session in that
    /// Repo failing to start. The flag keeps the other answer, because a flag is
    /// the installation's own word and nobody is watching when it is wrong.
    ///
    /// The ones that are not there are dropped after the composition rather than
    /// before it, so what is logged is what *this* session would have been
    /// given: a bind configured for some other Repo has nothing to say to a
    /// session that was never going to get it. An entry that will not read at
    /// all is the exception, and unavoidably: an entry nothing can tell the Repo
    /// of is one there is no composition to drop it out of.
    pub fn settings_binds(binds: &[String], conversation: &store::Conversation) -> Vec<Bind> {
        let mut config = SandboxConfig::default();

        for bind in binds {
            match read_bind(bind) {
                Ok((Some(repo), path)) => config.per_repo.entry(repo).or_default().push(path),
                Ok((None, path)) => config.global.push(path),
                Err(error) => tracing::warn!(
                    conversation_id = conversation.id,
                    bind,
                    error = ?error,
                    "a sandbox bind in the settings could not be read, so the session was \
                     started without it"
                ),
            }
        }

        config
            .binds_for(conversation)
            .into_iter()
            .filter(|bind| {
                let there = bind.path.exists();

                if !there {
                    tracing::warn!(
                        conversation_id = conversation.id,
                        bind = %bind.path.display(),
                        "a sandbox bind in the settings is not there, so the session was \
                         started without it"
                    );
                }

                there
            })
            .collect()
    }

    /// What a sandbox for `conversation` binds beyond the decided surface: the
    /// global set, then its own Repo's, then each of its companions' own.
    ///
    /// In that order, and all of them kept: a Repo's set composes over the
    /// global one rather than replacing it, because the global set is what the
    /// machine gives every session and a repository asking for a cache of its
    /// own is not asking to give the rest up.
    ///
    /// A companion brings what is configured for its own name and nothing else.
    /// Its builds need its caches like any other repository's — the checkout is
    /// half of what building in a companion takes and this is the other half —
    /// and the global set is already in by way of the Conversation's own Repo,
    /// so binding it again per companion would say something this does not mean.
    ///
    /// Writable, every one of them, and a companion's whatever its mode. A
    /// configured bind is a build cache or a package registry — somewhere a
    /// build writes — and whoever configured it opened the hole on purpose.
    /// They sit outside the repository besides, so a read-only companion whose
    /// cache could not be written to would fail on a cold cache for nothing
    /// gained.
    pub fn binds_for(&self, conversation: &store::Conversation) -> Vec<Bind> {
        let mut binds: Vec<Bind> = self.global.iter().cloned().map(Bind::writable).collect();

        binds.extend(self.own_binds(&conversation.repo.name));

        for companion in &conversation.companions {
            binds.extend(self.own_binds(&companion.repo.name));
        }

        binds
    }

    /// What only the Repo of that name asked for, without the global set in
    /// front of it.
    pub fn own_binds(&self, repo: &str) -> Vec<Bind> {
        self.per_repo
            .get(repo)
            .into_iter()
            .flatten()
            .cloned()
            .map(Bind::writable)
            .collect()
    }

    /// Every bind configured, for the line the server logs about what it will
    /// hand out.
    pub fn count(&self) -> usize {
        self.global.len() + self.per_repo.values().map(Vec::len).sum::<usize>()
    }

    /// And every one of them as the Repo it is for — `None` for every Repo — and
    /// the directory it binds: what the settings page draws the installation's
    /// half of its list from.
    ///
    /// The global ones first and each Repo's after them, which is the order they
    /// are composed in and the order the page reads them in. A pair rather than
    /// the entry as it was written, because what was written is gone by here:
    /// this is the parsed set, and the page draws the two halves apart anyway.
    pub fn entries(&self) -> Vec<(Option<&str>, &Path)> {
        let global = self.global.iter().map(|path| (None, path.as_path()));

        let per_repo = self.per_repo.iter().flat_map(|(repo, paths)| {
            paths
                .iter()
                .map(move |path| (Some(repo.as_str()), path.as_path()))
        });

        global.chain(per_repo).collect()
    }
}

/// One `--sandbox-bind`, as the Repo it belongs to — `None` for every Repo — and
/// the directory it binds.
pub(crate) fn read_bind(bind: &str) -> anyhow::Result<(Option<String>, PathBuf)> {
    if bind.starts_with('/') {
        return Ok((None, PathBuf::from(bind)));
    }

    let Some((repo, path)) = bind.split_once('=') else {
        anyhow::bail!(
            "the sandbox bind {bind:?} is neither an absolute path nor `name=path`: \
             a global bind is a directory, and a Repo's own is the name it is \
             registered under and then the directory"
        );
    };

    if repo.is_empty() {
        anyhow::bail!("the sandbox bind {bind:?} names no Repo before its `=`");
    }

    if !path.starts_with('/') {
        anyhow::bail!(
            "the sandbox bind {bind:?} is relative: a bind has to name one directory, \
             whichever directory the server was started in"
        );
    }

    Ok((Some(repo.to_owned()), PathBuf::from(path)))
}

/// One session's sandbox: everything that decides what a command run inside can
/// see.
///
/// Built rather than run — [`Sandbox::command`] hands back a [`Command`] for the
/// caller to spawn, because what a session needs around it (a pty, a Capture
/// being written) is the next stage's business and none of it belongs in a
/// mount table.
#[derive(Debug, Clone)]
pub struct Sandbox {
    /// The Conversation's worktree, read-write, and the directory the command
    /// starts in.
    worktree: PathBuf,

    /// The Repo's common `.git` directory, read-write.
    ///
    /// Read-write because a session commits, and read *common* because a
    /// worktree's own git directory lives inside the repository's rather than
    /// beside the checkout: the `.git` in the worktree is a file pointing back
    /// into this one. The Repo's working files are not bound, so the checkout
    /// the worktree was made from stays invisible — what is shared is the object
    /// database and the refs, which is what a commit and a push need.
    git_dir: PathBuf,

    /// The Profile's account, in the shape its agent type keeps one — what is
    /// mounted into HOME, and where.
    ///
    /// Claude's pair goes over `~/.claude` and `~/.claude.json`, and travels
    /// together or not at all: the pair is what keeps accounts apart. Every type
    /// after it is one home over the one directory that backend keeps its whole
    /// account under. Which arm this is also says which backend a session is
    /// running, which is what [`AGENT_TYPE`] carries inside.
    account: store::Account,

    /// The bundled skills, read-only where a session is told to read them —
    /// see [`crate::skills`] for why they are Verkstead's rather than the
    /// account's, why they are read-only, and why the path is nobody's. And the
    /// empty directory that goes over `~/.claude/skills`, which comes with them
    /// — see [`skills::Skills::nothing`].
    skills: Skills,

    /// The executable a session runs as `verkstead`, read-only where a session
    /// finds it and first on `PATH`.
    ///
    /// The server's own — see [`Executable`] — and read-only for the reason the
    /// skills are: what a session asks with is the product's, and not a file the
    /// session can rewrite mid-run.
    verkstead: Executable,

    /// The Conversation's own directory outside the worktree, read-write, at
    /// [`handoffs::INSIDE`].
    ///
    /// Where the handoff document is written, and the one writable place a
    /// session has that git will never see — see [`crate::handoffs`]. Every
    /// session of the Conversation gets it, not only the grilling one that
    /// writes the handoff: it is somewhere to put what is Verkstead's rather
    /// than the project's, and which session is doing that does not change what
    /// the surface is.
    handoff_dir: PathBuf,

    home: Home,

    /// What `gh` inside authenticates as, or `None` where nothing is configured.
    ///
    /// Taken as the sandbox is built rather than held from startup, so a token
    /// the human rotates applies from the next session — and a session already
    /// running keeps the one it started with, which is the only thing an
    /// environment variable could mean.
    github_token: Option<String>,

    /// And who it commits as, taken at the same moment and for the same reason.
    ///
    /// Either half may be missing, and a missing half is left unsaid rather than
    /// filled in: git's own "tell me who you are" is the failure worth having,
    /// because it says what to go and configure.
    git_author: GitAuthor,

    /// Where the session inside reaches Verkstead: this Conversation's own base
    /// URL, which is what `verkstead ask` puts its Sets to.
    server: String,

    /// Everything bound beyond that surface: the Conversation's companion repos
    /// first, each by its own mode, then what the installation's Sandbox
    /// Configuration asked for, and then what the settings file adds to it.
    ///
    /// The companions first because a bind is applied in the order it is given,
    /// so a configured cache inside a read-only companion's tree lands over the
    /// read-only bind rather than under it — and a cache that could not be
    /// written to is no cache. The settings-held set last for no stronger reason
    /// than that the two sets compose and one of them has to be second; what
    /// says it is this one is that the other was resolved at startup and this
    /// one is read here.
    binds: Vec<Bind>,

    /// The shared Rust build cache this session gets, or `None` where the human
    /// switched it off or this server has none to give.
    ///
    /// Decided as the sandbox is built rather than held from startup, for the
    /// reason the token and the author are: the switch is in `config.yaml`, the
    /// file is read at every spawn, so flipping it in the workbench applies to
    /// the next session and a running one keeps what it started with.
    build_cache: Option<build_cache::Shared>,
}

impl Sandbox {
    /// The sandbox a session for `conversation` runs in, under the account
    /// `profile` names.
    ///
    /// Which Profile is the caller's to choose: a Conversation fixes two, and
    /// which of them a session runs under is what the session is for rather than
    /// anything the sandbox can tell.
    ///
    /// `None` where the Conversation has nowhere to run yet — no worktree, or
    /// one git will not own — which is every Conversation before grilling starts
    /// and every one that has been closed. Its handoff directory is made here
    /// where it is not already there, and failing to make one is the same
    /// answer: a bind with nothing behind it is a sandbox that will not start.
    /// A companion whose checkout git will not own is that answer too: it was
    /// made at grill start and the session was told about it, so a sandbox
    /// missing it is not a smaller sandbox but a wrong one.
    ///
    /// Git is asked here and the filesystem is written to, so this blocks.
    ///
    /// `extra` is what the installation's Sandbox Configuration asked for,
    /// resolved once at startup. What the settings file asks for is not a
    /// parameter beside it: `config` is already here, it is already the thing
    /// read afresh at this moment, and a caller passing binds it read out of the
    /// same file would be a second way to say one thing.
    ///
    /// The parameter list is the surface: every one of these is something the
    /// session gets, and spelling them out is what lets a reader of a call site
    /// see the whole of what one is given. A struct grouping them would hide
    /// exactly the thing worth reading. The companions are the one thing not in
    /// it — they come off the Conversation, as its own worktree does, and by the
    /// same rule: the mode on the row decides the bind, and no caller is in a
    /// position to decide otherwise.
    #[allow(clippy::too_many_arguments)]
    pub fn for_conversation(
        conversation: &store::Conversation,
        profile: &store::Profile,
        homes: &Homes,
        reachable: &Reachable,
        skills: &Skills,
        verkstead: &Executable,
        handoffs: &Handoffs,
        secrets: &Secrets,
        config: &Config,
        cache: &BuildCache,
        extra: Vec<Bind>,
    ) -> Option<Sandbox> {
        let worktree = conversation.worktree.clone()?;
        let git_dir = crate::worktrees::common_git_dir(&worktree)?;
        let handoff_dir = handoffs.directory(conversation.id)?;

        let mut binds = companion_binds(conversation)?;
        binds.extend(extra);

        // And what `config.yaml` asks for on top of it, read here for the reason
        // the author and the build cache below are read here: the settings side
        // of Sandbox Configuration is the human's to change from a phone, so a
        // bind added there reaches the next session without the server being
        // restarted. See [`SandboxConfig::settings_binds`] for why an entry that
        // will not resolve is dropped rather than refused.
        binds.extend(SandboxConfig::settings_binds(
            config.sandbox_binds(),
            conversation,
        ));

        Some(Sandbox {
            worktree,
            git_dir,
            account: profile.account.clone(),
            skills: skills.clone(),
            verkstead: verkstead.clone(),
            handoff_dir,
            home: homes.for_conversation(conversation.id),
            github_token: secrets.github_token().map(str::to_owned),
            git_author: config.git_author().clone(),
            server: reachable.asking_from(conversation.id),
            binds,
            build_cache: cache.shared(config.rust_build_cache()),
        })
    }

    /// Where a session finds the sccache it compiles through, which is beside
    /// the binary it asks with — see [`own_bin`].
    ///
    /// Read off the executable rather than said again, so that the one answer
    /// about where a directory of Verkstead's own is on this machine serves
    /// both of the things in it.
    fn sccache_inside(&self) -> PathBuf {
        self.verkstead.bin().join(build_cache::SCCACHE)
    }

    /// `argv` as it will be run inside the sandbox, by whichever mechanism this
    /// machine has one.
    ///
    /// The command is not put through a shell: what runs inside is an argument
    /// vector the orchestrator built, and a shell between it and the sandbox
    /// would be one more thing to quote for.
    pub fn command<S: AsRef<OsStr>>(&self, argv: &[S]) -> Command {
        render(&self.surface(argv))
    }

    /// And what that mechanism is given: everything a session may reach, said
    /// once.
    ///
    /// **The order is the description**, because a path said twice is the
    /// second one — see [`surface`]. Which is why the account lands after the
    /// directory it goes inside, why what covers the account's own skills is
    /// after the account, and why the handoff directory is after the temporary
    /// filesystem that would otherwise be over it.
    fn surface<S: AsRef<OsStr>>(&self, argv: &[S]) -> Surface {
        // The floor every sandbox of Verkstead's stands on — see
        // [`on_the_machine`], which is where the compile server outside every
        // session gets the same one.
        let mut surface = on_the_machine(self.worktree.clone());

        // HOME before anything that goes inside it: the directory has to be
        // there for the account to land in, and everything else about it stays
        // absent.
        surface.made(Access::Empty(self.home.path().to_owned()));

        surface
            .own(&self.worktree, Reach::ReadWrite)
            .own(&self.git_dir, Reach::ReadWrite);

        // And the account, in the shape its agent type keeps one: what goes
        // where is that type's own business, and a backend arriving with an
        // account of its own lands here rather than in whatever the pair
        // happened to mean.
        match &self.account {
            store::Account::Claude {
                claude_dir,
                config_file,
            } => {
                surface
                    .elsewhere(
                        claude_dir,
                        self.home.path().join(CLAUDE_DIR_INSIDE_HOME),
                        Reach::ReadWrite,
                    )
                    .elsewhere(
                        config_file,
                        self.home.path().join(CLAUDE_CONFIG_INSIDE_HOME),
                        Reach::ReadWrite,
                    );
            }
            store::Account::Codex { home } => {
                surface.elsewhere(
                    home,
                    self.home.path().join(CODEX_INSIDE_HOME),
                    Reach::ReadWrite,
                );
            }
            store::Account::Grok { home } => {
                surface.elsewhere(
                    home,
                    self.home.path().join(GROK_INSIDE_HOME),
                    Reach::ReadWrite,
                );
            }
            // Two rather than one, and the same relative path on both sides of
            // each: an OpenCode Profile's home is an opencode home, and the XDG
            // defaults resolve inside the fresh HOME — see
            // [`OPENCODE_CONFIG_INSIDE_HOME`].
            store::Account::OpenCode { home } => {
                for inside in [OPENCODE_CONFIG_INSIDE_HOME, OPENCODE_DATA_INSIDE_HOME] {
                    surface.elsewhere(
                        home.join(inside),
                        self.home.path().join(inside),
                        Reach::ReadWrite,
                    );
                }
            }
        }

        // After the temporary filesystem and inside it: that would otherwise
        // land over this and leave the session writing its handoff into memory
        // nothing outside will ever read.
        surface.elsewhere(&self.handoff_dir, handoffs::INSIDE, Reach::ReadWrite);

        // The skills, at a path of Verkstead's own outside HOME entirely — what
        // a session reads there is what this binary ships. Read-only, because
        // what a session is grilled by is the product's and not a file the
        // session can rewrite mid-run.
        //
        // Where a bind can make that path, this is one; where none can, the
        // path a session is told to read is the one they are really written at
        // and the two sides of this are the same directory — see
        // [`skills::Skills::inside`].
        surface.elsewhere(self.skills.path(), self.skills.inside(), Reach::ReadOnly);

        // And nothing at all where the account's own skills would otherwise be
        // found: after the Profile's directory and inside it, because what is
        // said second is what a session gets — an empty directory of
        // Verkstead's own standing over them where a mount can do that, and a
        // refusal of the path where none can.
        //
        // Claude's, and so far Claude's alone. Each backend after it has a
        // discovery path of its own, covered the same way by the stage that
        // lands it — except where that path is *inside* the account home
        // itself, as Codex's `~/.codex/skills` and Grok Build's `~/.grok/skills`
        // both are: covering one would hide the skills those programs ship as
        // well as the ones the account added, and the home is the whole of what
        // such a Profile names, so it is left as the account keeps it
        // (ADR-0011).
        //
        // OpenCode adds none either, and for a reason of its own. Its two
        // global paths — `~/.claude/skills` and `~/.agents/skills` — are
        // Claude-shaped and sit under HOME rather than under its account, and
        // an OpenCode Profile puts neither of them inside: HOME inside is
        // fresh, so there is nothing at either to hide and an empty directory
        // over one would cover nothing. Its own is inside the config directory
        // the Profile names, which is the exception above. So every backend
        // that has landed adds nothing here, and a stage that adds nothing is
        // following the rule rather than forgetting one.
        //
        // And the account's type at all, rather than every home there is,
        // because covering a home no session is running under would cover
        // nothing and make a directory the account never had.
        if matches!(self.account, store::Account::Claude { .. }) {
            surface.nothing(
                self.home.path().join(skills::CLAUDE_INSIDE_HOME),
                self.skills.nothing(),
            );
        }

        // And the binary the session asks with, in a directory of its own that
        // goes first on `PATH` — see [`Executable`]. What is on that `PATH`
        // entry is this one file and nothing the host put beside it.
        surface.elsewhere(
            self.verkstead.path(),
            self.verkstead.inside(),
            Reach::ReadOnly,
        );

        // And the shared build cache: the directory writable at its own place,
        // and the sccache that compiles into it read-only in the directory the
        // binary above just made. After the empty HOME, so that a cache under
        // the server's own home — which is where it is when nobody has
        // configured one — is inside it rather than wiped by it. See
        // [`crate::build_cache`].
        if let Some(cache) = &self.build_cache {
            surface.own(cache.dir(), Reach::ReadWrite);

            if let Some(sccache) = cache.sccache() {
                surface.elsewhere(sccache, self.sccache_inside(), Reach::ReadOnly);
            }
        }

        for bind in &self.binds {
            surface.own(&bind.path, bind.reach);
        }

        surface
            .set("HOME", self.home.path())
            .set("PATH", path(self.verkstead.bin()))
            // Which shell is inside, for the same reason `PATH` is said here:
            // the environment is cleared, so a tool that shells out reaches for
            // whatever this holds — and with nothing in it, it would fall back
            // to whatever login shell the passwd file gives the user the server
            // happens to run as.
            .set("SHELL", SHELL)
            // And what kind of terminal a session is on, which is a fact about
            // the pseudo-terminal Verkstead opened for it rather than about the
            // sandbox — see [`crate::terminal`]. Said because nothing else
            // would: the environment is cleared, and an interface told nothing
            // draws for the dumbest terminal it knows about.
            .set("TERM", terminal::TERM)
            // What makes a session's Question Sets its own Conversation's. The
            // variable the bundled CLI reads, scoped to one Conversation, so
            // nothing is inferred from the project or the branch — two
            // Conversations against one Repo would be indistinguishable by
            // either.
            .set("VERKSTEAD_SERVER", &self.server)
            // And which backend this session is, which is what tailors the
            // Guide it reads — see [`AGENT_TYPE`]. Off the account's own shape,
            // so nothing has to be plumbed through to say which agent is being
            // launched.
            .set(AGENT_TYPE, self.account.agent_type().word());

        // The two an OpenCode session is told about itself: where its store
        // goes, because opencode names the file after the release channel the
        // install came from and the reader that follows a Transcript opens the
        // path this chose; and how long its shell tool holds a command the
        // model gave no timeout of its own, which is what a blocking ask under
        // this backend stands on — see [`OPENCODE_BASH_DEFAULT_TIMEOUT`].
        if matches!(self.account, store::Account::OpenCode { .. }) {
            surface.set(OPENCODE_DB, OPENCODE_DB_FILE).set(
                OPENCODE_BASH_DEFAULT_TIMEOUT,
                OPENCODE_BASH_DEFAULT_TIMEOUT_MS,
            );
        }

        // Where a Rust build inside puts what it downloads and what it
        // compiles. Nothing but a Rust build ever reads any of them, which is
        // what makes them safe to set for every session whatever the repository
        // holds.
        //
        // `CARGO_INCREMENTAL` is deliberately not among them. Cargo compiles
        // dependencies non-incrementally already, which is exactly what sccache
        // can cache; the workspace's own crates stay incremental in the
        // worktree's `target/`, and turning that off to feed the cache would
        // trade the fast half of every build for the slow half of one.
        //
        // Two things this loses to, both on purpose. A project that sets
        // `build.rustc-wrapper` in its own `.cargo/config.toml` is overridden,
        // because cargo gives the environment precedence — rare, and accepted. A
        // project whose dev shell exports these itself wins instead, because
        // `nix develop` layers its environment over this one — which is a
        // project saying what its own build needs, and is working as intended.
        if let Some(cache) = &self.build_cache {
            surface.set("CARGO_HOME", cache.cargo_home());

            // Only where there is an sccache to point at. Without one this is a
            // cache of downloads and nothing else — see [`crate::build_cache`]
            // — and a `RUSTC_WRAPPER` naming a path that is not inside would be
            // every Rust build inside failing rather than one running uncached.
            //
            // What this reaches is the compile server Verkstead is running
            // outside, over the host's network — see
            // [`crate::build_cache::BuildCache::compiling`], which is what puts
            // one up before a session that builds Rust starts. `SCCACHE_DIR` is
            // said here all the same, and it is not redundant: it is what the
            // client would start a server of its own into if Verkstead's were
            // somehow missing, and that server should write into the machine's
            // one cache like every other.
            if cache.sccache().is_some() {
                surface
                    .set("RUSTC_WRAPPER", self.sccache_inside())
                    .set("SCCACHE_DIR", cache.sccache_dir())
                    .set("SCCACHE_CACHE_SIZE", cache.size());
            }
        }

        // What `gh` inside authenticates as, which it reads from here without
        // being told to and without a file anywhere. Set only where there is one
        // to set: `GH_TOKEN` present and empty is a login `gh` fails on obscurely
        // where its absence is a login it says plainly it does not have.
        if let Some(token) = &self.github_token {
            surface.set("GH_TOKEN", token);
        }

        // And the whole of git's configuration, in the environment for the same
        // reason: there is no file inside for it to be in.
        let git_config = self.git_config();

        for (n, (key, value)) in git_config.iter().enumerate() {
            surface
                .set(&format!("GIT_CONFIG_KEY_{n}"), key)
                .set(&format!("GIT_CONFIG_VALUE_{n}"), value);
        }

        surface
            .set("GIT_CONFIG_COUNT", git_config.len().to_string())
            // And nothing git cannot answer for itself is asked of anybody.
            // Nobody is at this terminal: a push with no usable credentials has
            // to come back saying so, where a prompt for a username would be a
            // session sitting on a pty until something noticed.
            .set("GIT_TERMINAL_PROMPT", "0");

        surface.running(argv);

        surface
    }

    /// Every `key = value` git is configured with inside, in the order the
    /// environment will number them.
    ///
    /// Three things, and the last two are the same thing said twice over: who
    /// the commits are by, how a push proves who it is, and which URLs that
    /// proof is any use for.
    ///
    /// The credential helper is `gh`'s own, which answers out of `GH_TOKEN` — so
    /// a `git push` over HTTPS authenticates as whoever the settings file says,
    /// with nothing stored anywhere and nothing to log in to. It is scoped to
    /// `https://github.com` rather than left open: a helper on every host is one
    /// asked about hosts it has no business being asked about.
    ///
    /// And the rewrites are what make that reachable at all. A repository cloned
    /// over SSH has an SSH remote, and there are no keys inside a sandbox for it
    /// to offer — so a push would fail on a missing key rather than fall back to
    /// anything. `insteadOf` turns the two spellings of a GitHub SSH remote into
    /// the HTTPS one as git resolves the URL, leaving what the repository has
    /// written down untouched: a session pushes, and `.git/config` still says
    /// what the human cloned.
    fn git_config(&self) -> Vec<(String, String)> {
        let mut config = Vec::new();

        // Each half on its own: a name configured and no address is a name
        // configured, and git says for itself what is still missing.
        if let Some(name) = self.git_author.name() {
            config.push(("user.name".to_owned(), name.to_owned()));
        }

        if let Some(email) = self.git_author.email() {
            config.push(("user.email".to_owned(), email.to_owned()));
        }

        config.push((
            format!("credential.{GITHUB}.helper"),
            // The `!` is what makes git run it as a command rather than look for
            // a `git-credential-` binary of that name.
            "!gh auth git-credential".to_owned(),
        ));

        // Both spellings of the same remote, as two values of the one
        // multi-valued key — which is what repeating it in the environment
        // means, the counted pairs being read as one more configuration file.
        for ssh in ["git@github.com:", "ssh://git@github.com/"] {
            config.push((format!("url.{GITHUB}/.insteadOf"), ssh.to_owned()));
        }

        config
    }
}

/// What a Conversation's companion repos put inside the sandbox: each one's
/// worktree and the git directory behind it, both at the companion's own mode.
///
/// Both of them, for the reason the Conversation's own repository needs both: a
/// worktree's git directory lives inside the repository's rather than beside the
/// checkout, so a checkout bound without it has no object database behind it and
/// git inside would not call it a repository at all.
///
/// The mode is the row's, and it is the whole of the difference: a read-write
/// companion is somewhere the work is done, and a read-only one is somewhere to
/// read. Read-only reaches the git directory too — the checkout alone would
/// leave the history writable through the back door.
///
/// A companion with no checkout is skipped rather than refused: that is a
/// Conversation still drafting, which has no session to build a sandbox for
/// anyway. One with a checkout git will not own is `None`, which is no sandbox
/// and so no session — see [`Sandbox::for_conversation`].
fn companion_binds(conversation: &store::Conversation) -> Option<Vec<Bind>> {
    let mut binds = Vec::new();

    for companion in &conversation.companions {
        let Some(worktree) = companion.worktree.clone() else {
            continue;
        };

        let git_dir = crate::worktrees::common_git_dir(&worktree)?;

        let bind = match companion.mode {
            store::CompanionMode::ReadOnly => Bind::readable,
            store::CompanionMode::ReadWrite => Bind::writable,
        };

        binds.push(bind(worktree));
        binds.push(bind(git_dir));
    }

    Some(binds)
}

/// `argv` wrapped in `nix develop` where the worktree's flake actually provides
/// a shell, and `argv` as it stands where it does not.
///
/// A `flake.nix` alone is not enough, which is the whole reason this asks rather
/// than looking for the file: `nix develop` falls through a list of attributes
/// and errors out when the flake defines none of them, so a repository with a
/// flake that only builds a package would have every session in it fail to
/// start.
///
/// Asked on the host, before the sandbox exists, because the answer decides what
/// the sandbox is told to run. It is a `nix eval` per attribute and it is asked
/// once per session.
///
/// One worktree, and only the Conversation's own. A companion repo with a flake
/// of its own is not wrapped in a second shell — there is nowhere for one to go,
/// a session being one command — and it does not need to be: `nix` is on the
/// sandbox's `PATH`, so an agent that has to build in a companion enters its
/// shell there the way it would in any checkout it had walked into.
pub fn under_dev_shell(worktree: &Path, argv: &[String]) -> Vec<String> {
    if !dev_shell(worktree) {
        return argv.to_vec();
    }

    let mut wrapped = vec![
        "nix".to_owned(),
        "develop".to_owned(),
        "--command".to_owned(),
    ];
    wrapped.extend_from_slice(argv);
    wrapped
}

/// Whether `nix develop` in `worktree` would find a shell to enter.
fn dev_shell(worktree: &Path) -> bool {
    if !worktree.join("flake.nix").is_file() {
        return false;
    }

    let Some(system) = nix(
        worktree,
        &[
            "eval",
            "--impure",
            "--raw",
            "--expr",
            "builtins.currentSystem",
        ],
    ) else {
        return false;
    };

    let system = system.trim();

    // The attributes `nix develop` itself falls through, in its own order.
    [
        format!("devShells.{system}.default"),
        format!("devShell.{system}"),
        format!("packages.{system}.default"),
        format!("defaultPackage.{system}"),
    ]
    .iter()
    .any(|attr| {
        // `--apply` forces the attribute enough to say whether it is there,
        // without building whatever it evaluates to.
        nix(
            worktree,
            &["eval", &format!(".#{attr}"), "--apply", "x: true"],
        )
        .is_some()
    })
}

/// Run nix in `dir` and take its stdout, or `None` if it failed.
///
/// The experimental features are named rather than assumed: this runs as
/// whichever user the server does, and a flake command that works in the human's
/// shell and not in the service's would be a repository that mysteriously stops
/// having a dev shell.
fn nix(dir: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("nix")
        .args(["--extra-experimental-features", "nix-command flakes"])
        .args(args)
        .current_dir(dir)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;

    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An executable that is really there, wherever a test's temporary
    /// directory is.
    fn executable(bin: &Path, name: &str, data_dir: &Path) -> Option<Executable> {
        std::fs::write(bin.join(name), "an ELF\n").unwrap();

        Executable::at(bin.join(name), data_dir)
    }

    /// The bind puts the executable in one directory and `PATH` sends a session
    /// looking in another, and the two have to be the same place — a `verkstead`
    /// mounted somewhere nothing searches is a session back on the machine's
    /// install without anything saying so.
    #[test]
    fn the_directory_the_binary_is_found_in_is_the_first_on_the_path() {
        let dir = tempfile::tempdir().unwrap();
        let equipped = executable(dir.path(), "verkstead", dir.path()).expect("it is there");

        assert_eq!(
            equipped.inside.file_name().and_then(OsStr::to_str),
            Some("verkstead"),
            "the name on `PATH` is the name the skills and the Guide tell a session to run"
        );
        assert_eq!(
            path(equipped.bin()).split(':').next().map(Path::new),
            Some(equipped.bin()),
            "the server's own build has to be found before the machine's install"
        );
    }

    /// And what a session finds it under is a directory a mount can make, or
    /// the Data Directory where none can — see [`own_directory`].
    #[test]
    fn a_platform_with_no_mounts_finds_the_product_in_the_data_directory() {
        let data = Path::new("/Users/you/Library/Application Support/Verkstead");

        assert_eq!(
            own_directory(Platform::MacOs, data),
            data,
            "a Mac can make nothing at `/`, so what a session reads is where the \
             file really is"
        );
        assert_eq!(
            own_directory(Platform::Linux, data),
            Path::new(OWN_DIRECTORY),
            "and a bind makes a directory that is nobody's on the host"
        );
    }

    /// Every path on a session's `PATH` is one the policy also lets it reach,
    /// which is what says a Mac with nix-darwin gets nix's tools and one
    /// without gets Homebrew's and the system's.
    ///
    /// Whole directories rather than the paths themselves: `/usr/bin` is on the
    /// `PATH` and `/usr` is what the system list holds.
    #[test]
    fn nothing_on_a_macs_path_is_a_directory_its_policy_refuses() {
        for entry in APPLE_PATH.split(':') {
            assert!(
                APPLE_SYSTEM
                    .iter()
                    .any(|system| Path::new(entry).starts_with(system)),
                "{entry} is on a session's PATH and nothing in the system list \
                 makes it reachable"
            );
        }

        assert!(
            APPLE_PATH
                .split(':')
                .any(|entry| entry == "/opt/homebrew/bin"),
            "a Mac with Homebrew has its actual toolchain there"
        );
        assert!(
            APPLE_PATH
                .split(':')
                .any(|entry| entry == "/run/current-system/sw/bin"),
            "and a Mac running nix-darwin is not made to do without nix's"
        );
    }

    /// A Conversation's HOME is the server's own where a mount can be made
    /// empty over it, and a real directory of Verkstead's own where none can.
    #[test]
    fn a_platform_with_no_mounts_gives_each_conversation_a_home_of_its_own() {
        let data = Path::new("/data");
        let servers = PathBuf::from("/home/verkstead");

        let linux = Homes::on(Platform::Linux, servers.clone(), data);
        assert_eq!(
            linux.for_conversation(7).path(),
            servers,
            "a tmpfs over the server's own home is what makes it empty"
        );

        let mac = Homes::on(Platform::MacOs, servers, data);
        assert_eq!(mac.for_conversation(7).path(), Path::new("/data/homes/7"));
        assert_ne!(
            mac.for_conversation(8).path(),
            mac.for_conversation(7).path(),
            "and one Conversation's HOME is not another's"
        );
    }

    /// A packaged binary is a wrapper and a dotted file beside it, and a
    /// packaged server is the second of the two — see [`unwrapped`].
    #[test]
    fn a_wrapped_executable_resolves_to_the_wrapper_beside_it() {
        let bin = tempfile::tempdir().unwrap();

        std::fs::write(bin.path().join("verkstead"), "#!/bin/sh\n").unwrap();
        std::fs::write(bin.path().join(".verkstead-wrapped"), "an ELF\n").unwrap();

        let executable = Executable::at(bin.path().join(".verkstead-wrapped"), bin.path())
            .expect("the file is there to be equipped with");

        assert_eq!(
            executable.path(),
            bin.path().join("verkstead"),
            "the wrapper is what puts git on the CLI's PATH, so it is what a session gets"
        );
    }

    /// And an unpackaged one is itself: a `cargo build` leaves no wrapper, and
    /// neither does a dotted name with nothing beside it.
    #[test]
    fn an_unwrapped_executable_is_the_one_that_was_named() {
        let bin = tempfile::tempdir().unwrap();
        let path = bin.path().join("verkstead");
        std::fs::write(&path, "an ELF\n").unwrap();

        let executable = Executable::at(path.clone(), bin.path()).expect("the file is there");

        assert_eq!(executable.path(), path);
    }

    /// A binary replaced under a running server, which is what an upgrade is:
    /// there is nothing left to bind, and saying so is what stops a session
    /// being equipped with the machine's install instead.
    #[test]
    fn an_executable_that_is_not_there_equips_nobody() {
        let bin = tempfile::tempdir().unwrap();

        assert!(Executable::at(bin.path().join("verkstead"), bin.path()).is_none());
    }
}
