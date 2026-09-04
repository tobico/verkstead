//! The sandbox a session runs in: a surface built around one Conversation's
//! worktree, and nothing else.
//!
//! **What is inside is one description, and the mechanism under it is the
//! platform's.** [`Sandbox::surface`] says what a session may reach, once; a
//! renderer turns that into bubblewrap's flags on Linux — see [`bwrap`] — or
//! into a deny-by-default policy on a Mac — see [`seatbelt`]. No rendering is
//! the description, no two of them are the same boundary, and nothing above
//! this module learns which one it got (ADR-0012).
//!
//! **And on Windows into the process itself, with no boundary at all** — see
//! [`open`]. That arm sets the environment, starts in the Worktree and runs the
//! vector, which is the whole of what a rendering with nothing to hide behind
//! comes to: a session there runs with the reach of the account running the
//! server, and the workbench says so in words rather than pretending otherwise
//! (ADR-0014). The description is the same description, which is what lets the
//! stage that gives that platform a boundary change the renderer and nothing
//! above it.
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
// rendering is a description going in and a process coming out, so the arm this
// machine will never run is still an arm its tests call — the same reason
// `crates/desktop`'s startup registrations are all built here. Not on a Windows
// build outside one: there is no namespace to unshare there, and the renderer a
// Windows session gets is the third of them below.
#[cfg(any(not(any(target_os = "macos", target_os = "windows")), test))]
mod bwrap;
// The Apple rendering is built for its tests on a Unix and nowhere else: what
// it renders is a policy about a filesystem where every path is a Unix path,
// and the tests that read one back make symlinks with a call Windows has not
// got. There is no Mac to be tested for on a Windows machine, so the arm is
// left out rather than made portable for a platform it says nothing about.
#[cfg(any(target_os = "macos", all(test, unix)))]
mod seatbelt;
// And the third, built everywhere and gated by nothing. What it renders is the
// process the description describes — the environment said, the directory
// started in, the vector run — with no wrapper in front of it, so there is
// nothing there for a `cfg` to be about. The one call it makes that one
// platform has to itself is beside it, in a module with an arm for each.
//
// Reachable from the rest of the crate rather than from here alone, for one
// thing in it: how a Windows machine resolves the name of a program is also
// what the terminals module has to ask to find the shell it opens on — see
// [`open::found`] and [`crate::terminals::shell`], its one caller outside this
// module.
mod junction;
pub(crate) mod open;
// And the three ends of what a renderer is: the description going in, the
// process coming out, and what is left to see to once that process has gone.
// The last of those is nothing on the two platforms whose links follow their
// own target, which is why it is a value here rather than a `cfg` — see
// [`closing`].
mod closing;
mod rendering;
mod surface;

// And what a rendering cannot say on the platform it is for: how long what it
// started lives. Not a `cfg` at all — the platform is a value here, and both
// arms are built and called wherever the tests are.
pub(crate) mod outliving;

use std::ffi::{OsStr, OsString};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

// And the description itself, which is not this module's alone: the compile
// server outside every session is composed of the same vocabulary and rendered
// by the same renderer — see [`crate::build_cache`], and [`on_the_machine`]
// for the part of it the two share.
pub(crate) use surface::{Access, Reach, Surface};

// And what a renderer hands back, which is not this module's alone either: it
// is what a session is started from, on whichever of the two arms of
// [`crate::terminal`] this machine has — and beside it what is left to see to
// once that session has gone, which is held by whoever is holding the session.
pub use closing::Closing;
pub use rendering::Rendering;

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
/// **The cryptex is under two names and the dynamic linker uses the second**,
/// which is why both are here. Apple ships the shared cache — and Safari, and
/// the rest of what arrives out of band — on a cryptex mounted at
/// `/System/Volumes/Preboot/Cryptexes`, and firmlinked back in at
/// `/System/Cryptexes`, which is what dyld actually opens. A firmlink is not a
/// symlink and resolves to itself, so a policy naming only the mount point is a
/// policy that says nothing about the path being used: every process started
/// under one dies on `SIGABRT` before it runs a line, because a dyld that
/// cannot map the cache has nowhere left to load libSystem from. It says
/// nothing while it does it, either — which is the whole of what made this cost
/// a CI run to find rather than a read.
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
    "/System/Cryptexes",
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
/// Windows is the Mac's answer for the Mac's reason, arrived at from the other
/// end: there are no binds there either, so a path nothing makes is a path
/// nothing is at. What a session is told to read is therefore where the file
/// really is, under the Data Directory — and what keeps it Verkstead's is that
/// Verkstead wrote it, rather than a boundary, there being none yet.
pub fn own_directory(platform: Platform, data_dir: &Path) -> PathBuf {
    match platform {
        Platform::MacOs | Platform::Windows => data_dir.to_owned(),
        Platform::Linux => PathBuf::from(OWN_DIRECTORY),
    }
}

/// What that directory is where a bind makes it.
const OWN_DIRECTORY: &str = "/verkstead";

/// A path under one of the directories a session reaches, composed the way the
/// session that reads it will read it.
///
/// [`Path::join`] is the wrong tool for this and looks like the right one: it
/// composes with the *host's* separator, and these are paths a session opens
/// inside its sandbox rather than paths this process opens. On the two Unixes
/// the two characters are the same one, so nothing has ever turned on the
/// difference — but a Windows host composing a Linux path gets a backslash in
/// the middle of a POSIX one, and these are the paths the skills' own text is
/// written out of and that a session is told in prose.
///
/// A forward slash on Windows itself is what that costs, and it costs nothing:
/// every call there opens the path with Win32, which reads both separators, and
/// what a session is told in prose is a path it can open.
pub(crate) fn under(directory: &Path, name: &str) -> PathBuf {
    PathBuf::from(format!("{}/{name}", directory.display()))
}

/// And the two things inside it: the directory the executables are in, which
/// goes first on a session's `PATH`, and what the server's own image is called
/// there.
///
/// A directory rather than a name inside one of the system binds: those are the
/// host's and read-only, so there is nowhere in them to put a file. It holds
/// the one executable the server put there and nothing else.
const BIN: &str = "bin";
const VERKSTEAD: &str = "verkstead";

/// And the two more that are there only where the image carries the libraries
/// it runs over — see [`Bundled`]: the libraries themselves, and the image
/// behind the launcher that stands at `bin/verkstead` in front of them.
///
/// `lib` beside `bin` because that is where a reader looks for it, and
/// `libexec` because the image is no longer what a session runs directly — what
/// is on the `PATH` is what a session types, and the file behind the launcher
/// is not that.
const LIB: &str = "lib";
const LIBEXEC: &str = "libexec";

/// Where the launcher is written on the host: under the Data Directory, beside
/// everything else the server makes for a session to read.
///
/// Not under the `bin` of the server's own directory, which on a Mac *is* a
/// directory under the Data Directory: what goes there is what a session finds,
/// and a host file standing where the bind goes would be one path trying to be
/// two files.
const LAUNCHER: &str = "verkstead-launcher";

/// What the AppImage runtime says about where it mounted the image, which is
/// how the server finds the libraries it was bundled with — see
/// [`Executable::bundling`].
const APPDIR: &str = "APPDIR";

/// And where the libraries are under that, which is the layout
/// `tools/build-appimage.sh` packs.
const BUNDLED_LIBRARIES: &str = "usr/lib";

/// What the loader reads a library path off, which is what the launcher sets
/// and what the probe is run with — see [`Bundled`].
const LD_LIBRARY_PATH: &str = "LD_LIBRARY_PATH";

/// And the verb the server runs its own image with before it equips anybody
/// with it — see [`Executable::probe`], which is where the choice of this one
/// is argued.
const GUIDE: &str = "guide";

/// How much of what a refused image said for itself is carried into the log
/// line that refuses it.
///
/// A loader naming the library it could not find says it in a few words; an
/// image that failed some other way could say anything at all, and a log record
/// is no place to find out how much.
const COMPLAINT: usize = 500;

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
///
/// **A session on Windows leads its `PATH` with something else**, and this is
/// the compile server's alone there: nothing is bound and nothing is linked, so
/// what a session asks with is the running image where it really is — see
/// [`Executable::at`] — and the directory holding *that* is what leads. The
/// sccache is not in it either, and for the same reason — see
/// [`sccache_inside`], which is where a session finds one on each platform.
pub(crate) fn own_bin(platform: Platform, data_dir: &Path) -> PathBuf {
    under(&own_directory(platform, data_dir), BIN)
}

/// And where the sccache the shared build cache compiles through is found from
/// inside: in that directory on the platforms that join it in, and where it
/// really is on the one that joins in nothing.
///
/// One answer for the two things that ask — a session's sandbox and the compile
/// server's own surface — because they are one fact: what is on the far end of
/// a `RUSTC_WRAPPER` and what the compile server runs are the same file, and
/// two readings of where it is would be two ways for them to disagree.
///
/// **Windows joins in nothing here, and needs to join in nothing.** There is no
/// boundary on that platform yet, so the path outside *is* the path inside —
/// and a hard link into Verkstead's own directory would drop the extension the
/// name is found by, which is to say it would make a file nothing on that
/// platform can start. What the description then says of it collapses to
/// nothing at all: a path bound onto itself is a path already where it is — see
/// [`Surface::elsewhere`].
pub(crate) fn sccache_inside(platform: Platform, ours: &Path, sccache: &Path) -> PathBuf {
    match platform {
        Platform::Linux | Platform::MacOs => ours.join(build_cache::SCCACHE),
        Platform::Windows => sccache.to_owned(),
    }
}

/// What a session's `PATH` is inside: `ours` first, and then the machine's own.
///
/// `ours` is the directory of Verkstead's own that the binary a session asks
/// with is in — see [`Executable`] for why a session asks with the server's own
/// build rather than with whatever the machine has installed. It goes first on
/// every platform, and it is passed in rather than said here because where it
/// is, is a fact about the machine too: `/verkstead/bin` where a bind makes it,
/// a directory under the Data Directory where nothing can, and the directory
/// the running image is really in where nothing is linked either — see
/// [`own_directory`] and [`Executable::at`].
///
/// Nothing on the two Unixes is inherited from the server's own environment:
/// what a session can run should be a fact about the sandbox rather than about
/// however the unit that started the orchestrator happened to be launched. On
/// Windows the machine's own half *is* that environment, which is the one place
/// this gives that up and is argued at [`servers_path`].
///
/// And what the compile server has, for the same reason: it is a fact about
/// what a sandbox holds rather than about either process.
///
/// **On Windows the machine's own half is the server's own `PATH`** rather than
/// a list written down here — see [`servers_path`], which is where that is
/// argued — and the two halves are joined by that platform's separator rather
/// than by a colon, which there is a drive letter's own punctuation. The
/// separator is this one's own: `crate::PATH_LIST_SEPARATOR` is what a flag is
/// parsed with, and a session's `PATH` is not a flag.
pub(crate) fn path(platform: Platform, ours: &Path) -> OsString {
    let mut path = ours.as_os_str().to_owned();

    path.push(separator(platform));
    path.push(machine_path(platform));

    path
}

/// How `PATH` is written on `platform` where it holds more than one directory.
fn separator(platform: Platform) -> &'static str {
    match platform {
        Platform::Linux | Platform::MacOs => ":",
        Platform::Windows => ";",
    }
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
///
/// The one thing in it that is `platform`'s rather than the machine's is where
/// a temporary file goes. `/tmp` is a path both Unixes have and Windows has
/// not, and nothing there reaches for one: what a Windows program reads is
/// `TEMP`, which points inside the session's own profile — so that one is said
/// with the rest of the profile rather than here. See [`temporary_inside`].
pub(crate) fn on_the_machine(platform: Platform, chdir: PathBuf) -> Surface {
    let mut surface = Surface::starting_in(chdir);

    for path in SYSTEM.iter().map(Path::new).filter(|path| path.exists()) {
        surface.own(path, Reach::ReadOnly);
    }

    surface.made(Access::ProcessTable).made(Access::Devices);

    match platform {
        Platform::Linux | Platform::MacOs => {
            surface.made(Access::Temporary(PathBuf::from(TMP)));
        }
        Platform::Windows => {}
    }

    surface
}

/// A directory that is really there and really empty.
///
/// What [`Access::Empty`] comes to on the two platforms that have to make one
/// rather than mount one — see [`seatbelt`] and [`open`], which are both of
/// them. Emptied rather than left where something is already in it: a HOME is
/// one Conversation's and every session of it is given the same one, so what a
/// session finds there should be what *it* was given rather than what the last
/// one left — which is what the other platform's tmpfs does for nothing.
///
/// Nothing under it is followed on the way out. A junction and a symbolic link
/// are both names for somewhere else and both are removed as the names they
/// are, which is what keeps emptying a profile from emptying the account joined
/// into it.
///
/// The path is Verkstead's own and nothing else is ever passed here: see
/// [`Homes`], which is the only thing that says where one goes.
pub(crate) fn emptied(path: &Path) -> std::io::Result<()> {
    match std::fs::remove_dir_all(path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }

    std::fs::create_dir_all(path)
}

/// And `surface` as the process that runs it, by whichever mechanism
/// `platform` has one: bubblewrap wherever there is a kernel with namespaces to
/// unshare, Apple's own on a Mac, and the open one where there is no boundary
/// yet — see [`bwrap`], [`seatbelt`] and [`open`].
///
/// The one place any rendering is reached from, so that a sandbox is a
/// description going in and a [`Rendering`] coming out wherever one is made.
///
/// **A value rather than the `cfg` this used to be**, for the reason
/// [`Platform`] is a value everywhere else: the arm this machine will never run
/// is still an arm its tests call. The arms are `cfg`-ed all the same, because
/// a renderer is compiled where there is a machine to run it or a test to ask
/// it — see the modules at the top of this file — and the last arm is what a
/// build with none for the platform it was handed has to say. Nothing outside a
/// test passes anything but [`Platform::HERE`], whose arm is always here.
pub(crate) fn rendered(platform: Platform, surface: &Surface) -> (Rendering, Closing) {
    match platform {
        #[cfg(any(not(any(target_os = "macos", target_os = "windows")), test))]
        Platform::Linux => (bwrap::command(surface), Closing::nothing()),
        #[cfg(any(target_os = "macos", all(test, unix)))]
        Platform::MacOs => (seatbelt::command(surface), Closing::nothing()),
        Platform::Windows => open::command(surface),
        #[allow(unreachable_patterns)]
        elsewhere => {
            unreachable!("this build carries no rendering for {elsewhere:?}")
        }
    }
}

/// The machine's own half of a session's `PATH`, on the platform whose answer
/// is a list rather than a lookup — see [`path`], and [`servers_path`] for the
/// one whose answer is neither.
///
/// One list or the other and never both: a NixOS box has nothing under
/// `/opt/homebrew` and a Mac has nothing under `/run/current-system/sw` unless
/// somebody put it there.
fn machine_path(platform: Platform) -> OsString {
    match platform {
        Platform::Linux => OsString::from(LINUX_PATH),
        Platform::MacOs => OsString::from(APPLE_PATH),
        Platform::Windows => servers_path(),
    }
}

/// And what it is on Windows, which is not a list here at all: the `PATH` the
/// server itself was started with.
///
/// **There is no `WINDOWS_PATH` beside [`LINUX_PATH`] and [`APPLE_PATH`]**, and
/// that is the decision rather than an omission. Those two are lists of where a
/// packaged system puts its tools, and they are worth writing down because a
/// session should reach the machine's own toolchain whatever the unit that
/// started the orchestrator was handed. A Windows machine has no such list:
/// where `git`, `node` and an agent live there is wherever their installers put
/// them and wherever the human added, which is written down in one place and
/// that place is the machine's `PATH`. So a session gets that, behind
/// Verkstead's own directory.
///
/// What it costs is the one thing the other two arms buy: a session's `PATH` is
/// a fact about how the server was launched. Which is the same trade the
/// unsandboxed note is about — there is no boundary on this platform yet, so
/// there is nothing here for a narrower `PATH` to protect.
fn servers_path() -> OsString {
    std::env::var_os("PATH").unwrap_or_default()
}

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
///
/// And on Windows it is a name nothing reads: what a program shelling out there
/// runs is `ComSpec` — see [`MACHINE_NAMES`], which is where that is said — and
/// the shell a human types into is the terminals' to name, exactly as it is on
/// the two Unixes. So this stays the one answer rather than growing an arm for
/// a platform that asks the question elsewhere.
///
/// What every sandbox says unless its maker says otherwise — see
/// [`Sandbox::shelled`], which is a Conversation's own terminal naming the shell
/// it is about to run instead.
pub(crate) const SHELL: &str = "/bin/sh";

/// The names a Windows session is told about the machine it is on, which are
/// the machine's own to say.
///
/// Nothing on that platform runs without them. `cmd.exe` with no `SystemRoot`
/// does not start; a program that looks for a system library resolves it
/// against `SystemDrive`; `ComSpec` is what anything shelling out runs, and
/// [`open`] is one of them — see that module, which runs a batch file through
/// it; and `PATHEXT` is half of what finding a program on this platform means,
/// which is the other thing that module reads.
///
/// Read off the server's own environment because that is what they are: facts
/// about the machine, not choices a sandbox makes. One that is somehow unset is
/// left unsaid rather than invented — a Windows machine has all four, and a
/// value guessed here would be a wrong answer standing where a missing one
/// would have said which.
const MACHINE_NAMES: [&str; 4] = ["SystemRoot", "SystemDrive", "ComSpec", "PATHEXT"];

/// Where the halves of a Windows profile are under it, which is where Windows
/// itself puts them: the roaming application data, the local application data,
/// and the temporary directory inside the second.
const APP_DATA: &str = "AppData";
const ROAMING: &str = "Roaming";
const LOCAL: &str = "Local";
const TEMPORARY: &str = "Temp";

/// And where a Windows session writes a temporary file: that last one, inside
/// the profile the session was given.
///
/// **Inside rather than the machine's own**, which is the one place this
/// platform's answer parts company with the Mac's: a Mac session writes into
/// the `/tmp` everybody on the box shares, and there is nothing about a Windows
/// profile that has to be shared — `TEMP` is already a variable, so pointing it
/// inside costs nothing and what a session throws away is thrown away with the
/// profile.
///
/// Said in two places off this one function: it is what [`windows_names`] sets
/// the variable to, and what the description says a session is given somewhere
/// to write — see [`Access::Temporary`], which is what really makes it.
pub(crate) fn temporary_inside(home: &Path) -> PathBuf {
    home.join(APP_DATA).join(LOCAL).join(TEMPORARY)
}

/// Everything a Windows session is told beyond what every session is told: the
/// profile it keeps its own things under, and the machine it is on.
///
/// **A Windows program does not read `HOME`.** What it reads is `USERPROFILE`
/// for the account's own directory, `APPDATA` and `LOCALAPPDATA` for the two
/// halves of where a program keeps its settings, and `TEMP` and `TMP` for
/// somewhere to write a file it will throw away. All five are pointed inside
/// the HOME this sandbox was given, so that all five follow it — which is what
/// makes the fresh profile fresh: what a session caches, what a tool writes
/// down and what either of them throws away are under a directory of this
/// Conversation's own, and none of it in the human's. `HOME` is said beside
/// them and not for Windows' sake: `git` and every agent that grew up on a Unix
/// read it, and a session's is the one this sandbox chose.
///
/// And then [`MACHINE_NAMES`], which are the machine's own to say.
pub(crate) fn windows_names(home: &Path) -> Vec<(&'static str, OsString)> {
    let local = home.join(APP_DATA).join(LOCAL);
    let temporary = temporary_inside(home);

    let mut named = vec![
        ("USERPROFILE", home.as_os_str().to_owned()),
        (
            "APPDATA",
            home.join(APP_DATA).join(ROAMING).into_os_string(),
        ),
        ("LOCALAPPDATA", local.into_os_string()),
        ("TEMP", temporary.as_os_str().to_owned()),
        ("TMP", temporary.into_os_string()),
    ];

    named.extend(
        MACHINE_NAMES
            .into_iter()
            .filter_map(|name| Some((name, std::env::var_os(name)?))),
    );

    named
}

/// Where each shape of account is joined into a session's HOME: what the
/// Profile names on the host, and what it is called inside.
///
/// **One place says what an account is made of**, because two things ask: the
/// description a session is rendered from — see [`Sandbox::surface`] — and
/// whether the account can be joined in at all on the platform whose links are
/// hard ones — see [`across_volumes`]. A second reading of the four shapes
/// would be a backend arriving with an account of its own and only one of them
/// learning about it.
///
/// Claude's pair is a directory and a file; the three after it are directories,
/// one of them twice over. Which is exactly the distinction the Windows arm
/// turns on: a directory is joined in by a junction and a file by a hard link,
/// and only the second of those cares what volume anything is on.
fn account_inside(account: &store::Account, home: &Path) -> Vec<(PathBuf, PathBuf)> {
    match account {
        store::Account::Claude {
            claude_dir,
            config_file,
        } => vec![
            (claude_dir.clone(), home.join(CLAUDE_DIR_INSIDE_HOME)),
            (config_file.clone(), home.join(CLAUDE_CONFIG_INSIDE_HOME)),
        ],
        store::Account::Codex { home: account } => {
            vec![(account.clone(), home.join(CODEX_INSIDE_HOME))]
        }
        store::Account::Grok { home: account } => {
            vec![(account.clone(), home.join(GROK_INSIDE_HOME))]
        }
        // Two rather than one, and the same relative path on both sides of
        // each: an OpenCode Profile's home is an opencode home, and the XDG
        // defaults resolve inside the fresh HOME — see
        // [`OPENCODE_CONFIG_INSIDE_HOME`].
        store::Account::OpenCode { home: account } => {
            [OPENCODE_CONFIG_INSIDE_HOME, OPENCODE_DATA_INSIDE_HOME]
                .into_iter()
                .map(|inside| (account.join(inside), home.join(inside)))
                .collect()
        }
    }
}

/// Whichever of `account`'s own paths cannot be joined into a profile at
/// `home`, and `None` where every one of them can.
///
/// **The whole of this is a Windows question**, and it is a hard link's. On the
/// two Unixes a path is mounted or symlinked into the fresh home and neither
/// cares what filesystem the other end is on; on Windows a file symlink wants a
/// privilege a per-user install has not got, so the link is a hard one — and a
/// hard link is two names for one file, which is a thing only one volume can
/// hold. See ADR-0014.
///
/// **The account is the end that can differ.** The profile is made under the
/// Data Directory and so is wherever that is; an account is wherever the Agent
/// Profile points inside a Watched Path, which on a machine with a second drive
/// may well be that drive. So this asks the account's paths against the
/// profile's, and what comes back is the first that could not be joined in —
/// which is a session refused before it starts rather than one started into a
/// profile with no account in it. See [`Sandbox::for_conversation`], which is
/// where it is refused and where both paths are said.
///
/// Directories are left out, and that is the rule rather than an omission: a
/// junction is a path rather than a file, it crosses volumes, and it needs no
/// privilege. A name with nothing at it yet is taken as a file, that being the
/// one shape of account path that may not be there — an agent that has not
/// written its config out yet.
fn across_volumes(platform: Platform, home: &Path, account: &store::Account) -> Option<PathBuf> {
    if platform != Platform::Windows {
        return None;
    }

    let ours = volume(home);

    account_inside(account, home)
        .into_iter()
        .map(|(host, _)| host)
        .filter(|host| !host.is_dir())
        .find(|host| volume(host) != ours)
}

/// Which volume `path` is on: the machine's own answer where there is a machine
/// to ask, and what the name itself says where there is not.
///
/// Asked rather than read wherever it can be, because the two are not the same
/// question on this platform: a volume mounted into a directory rather than
/// given a letter of its own is under some drive letter and is another volume,
/// which is exactly the case a rule about hard links exists for. [`written`] is
/// what stands where the call cannot answer — a name for a volume that is not
/// mounted, and every machine that is not this one, which is where the tests
/// ask.
///
/// Compared rather than shown, so nothing here needs a spelling of its own: it
/// is bytes, upper-cased, and the only question ever asked of two of them is
/// whether they are the same.
fn volume(path: &Path) -> Option<Vec<u8>> {
    asked(path).or_else(|| written(path))
}

/// What Windows says the volume of `path` is: the mount point it is under, or
/// `None` where the call could not answer.
#[cfg(windows)]
fn asked(path: &Path) -> Option<Vec<u8>> {
    use std::os::windows::ffi::{OsStrExt, OsStringExt};

    // The documented size for this, and the one every ordinary answer fits in:
    // a mount point is a drive letter or the directory a volume was mounted
    // into. A path that overflows it is one [`written`] answers instead.
    const MOUNT_POINT: usize = 260;

    let name: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let mut under = [0u16; MOUNT_POINT];

    // Safety: the name is NUL-terminated, and what is passed as the buffer's
    // length is the buffer's length.
    let answered = unsafe {
        windows_sys::Win32::Storage::FileSystem::GetVolumePathNameW(
            name.as_ptr(),
            under.as_mut_ptr(),
            under.len() as u32,
        )
    };

    if answered == 0 {
        return None;
    }

    let end = under.iter().position(|word| *word == 0)?;

    Some(
        OsString::from_wide(&under[..end])
            .as_encoded_bytes()
            .to_ascii_uppercase(),
    )
}

/// And what a machine that is not Windows can be asked, which is nothing: the
/// name itself is the whole of the answer there.
#[cfg(not(windows))]
fn asked(_: &Path) -> Option<Vec<u8>> {
    None
}

/// What `path` itself says about the volume it is on: the drive letter it
/// begins with, or the share of a UNC name.
///
/// Read by hand rather than through [`Path::components`], whose idea of a
/// prefix is the one the *server* was compiled with: this is a Windows path
/// wherever it is being read, and a test on a Linux machine asking whether two
/// of them are on one volume is asking about names written with drive letters.
///
/// The share rather than the server, because two shares on one server are two
/// volumes. Upper-cased, `c:` and `C:` being one drive. `None` for a name with
/// neither — a relative path, or a POSIX one, which is what every path on the
/// machines that ask this in a test is.
fn written(path: &Path) -> Option<Vec<u8>> {
    let name = path.as_os_str().as_encoded_bytes();

    // The two prefixes that say only that what follows goes to the filesystem
    // unchanged, so what they are in front of is what says which volume.
    let name = match name {
        [one, two, b'?' | b'.', three, rest @ ..]
            if a_separator(*one) && a_separator(*two) && a_separator(*three) =>
        {
            rest
        }
        _ => name,
    };

    match name {
        // What a UNC name is left as once that prefix has been taken off it,
        // which is the server and the share behind one more word.
        [u, n, c, four, share @ ..]
            if u.eq_ignore_ascii_case(&b'U')
                && n.eq_ignore_ascii_case(&b'N')
                && c.eq_ignore_ascii_case(&b'C')
                && a_separator(*four) =>
        {
            named(share)
        }
        // And the same name written by hand, which is two separators and then
        // the two of them.
        [one, two, share @ ..] if a_separator(*one) && a_separator(*two) => named(share),
        [letter, b':', ..] if letter.is_ascii_alphabetic() => {
            Some(vec![letter.to_ascii_uppercase(), b':'])
        }
        _ => None,
    }
}

/// The server and share at the front of a UNC name, as the one value they are.
///
/// `None` where there is no share after the server, which is a name for a
/// machine rather than for somewhere on one.
fn named(share: &[u8]) -> Option<Vec<u8>> {
    let mut pieces = share
        .split(|byte| a_separator(*byte))
        .filter(|piece| !piece.is_empty());

    let server = pieces.next()?;
    let share = pieces.next()?;

    let mut whole = vec![b'\\', b'\\'];
    whole.extend_from_slice(server);
    whole.push(b'\\');
    whole.extend_from_slice(share);

    Some(whole.to_ascii_uppercase())
}

/// Whether `byte` is one of the two characters Windows separates a path with.
fn a_separator(byte: u8) -> bool {
    byte == b'\\' || byte == b'/'
}

/// What NixOS's own shell initialisation reads to decide whether it has already
/// run, said inside a sandbox that runs a shell.
///
/// **Every shell on a NixOS box rebuilds the environment as it starts, login
/// shell or not.** `/etc/bashrc`, `/etc/zshenv` and fish's own preinit each
/// source `/etc/set-environment` unless this variable is set, and that file
/// exports a `PATH` built out of the *host's* profiles — the system profile,
/// the user's `~/.nix-profile`, their flatpak exports. So a terminal that said
/// nothing would come up with the sandbox's `PATH` replaced a moment after the
/// shell started, and the invariant that the running server's own `verkstead`
/// is the first one found would hold for every session and for no terminal:
/// a `verkstead ask` typed into one would run whatever the machine had
/// installed. Starting the shell without `-l` is not enough, which is the thing
/// worth knowing here — a login shell is not what provokes it.
///
/// So a sandbox that runs a shell says the environment has been set, which is
/// true and is the whole of what the variable claims: it was set here, by the
/// description this sandbox is, and it is deliberately not the host's.
///
/// Somebody else's spelling, read off the `set-environment` a NixOS 25.11 box
/// generates, and named here for the reason opencode's are: moving it costs one
/// edit. Off NixOS it is a variable nothing reads, which costs a sandbox
/// nothing.
const NIXOS_ENVIRONMENT_DONE: &str = "__NIXOS_SET_ENVIRONMENT_DONE";

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
/// **And Windows is the Mac's answer**, arrived at from the other end: nothing
/// is mounted there either, so a session's profile has to be a directory that
/// really is one — Verkstead's own, under the Data Directory, one per
/// Conversation and made fresh as each session starts. It is the profile
/// `USERPROFILE` names as well as the `HOME` a tool that grew up on a Unix
/// reads, and the two halves of a Windows profile and the temporary directory
/// are inside it — see [`windows_names`] — so what npm caches, what a tool
/// writes down and what a session throws away all land there rather than in
/// the human's own. What keeps one Conversation out of another's is again
/// that they are different directories; what keeps a session out of the rest
/// of the machine is nothing yet, which is what the unsandboxed note says in
/// words.
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
    /// and in a development shell it is the human's own. Which variable holds
    /// it is the platform's — see [`crate::platform::home_dir`], which is where
    /// Windows' `%USERPROFILE%` is read. `None` where nothing says, which on
    /// Linux is a server that can run no session; a Mac needs it for nothing
    /// here, and is refused with the rest for the sake of one answer rather
    /// than two.
    pub fn of_the_server(data_dir: &Path) -> Option<Homes> {
        Some(Homes::on(
            Platform::HERE,
            crate::platform::home_dir(
                Platform::HERE,
                &crate::platform::Environment::of_the_process(),
            )?,
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

    /// And whose platform these are, which is the same answer to two more
    /// questions a sandbox built against them has to have: which rendering it
    /// gets, and which platform's names its environment holds.
    ///
    /// Read off here rather than passed beside it, so that a test building a
    /// sandbox for a platform it is not on says which platform once.
    pub fn platform(&self) -> Platform {
        self.platform
    }

    /// And one Conversation's own, with the handoff directory it reaches
    /// beside it.
    ///
    /// The two together because one platform decides both: where a HOME is,
    /// and — where nothing can be mounted anywhere — that the Conversation's
    /// handoff directory is under it rather than at a `/tmp` every Conversation
    /// on the machine would be sharing. See [`handoffs::inside`].
    pub(crate) fn for_conversation(&self, conversation_id: i64) -> Home {
        let path = match self.platform {
            Platform::MacOs | Platform::Windows => self.root.join(conversation_id.to_string()),
            Platform::Linux => self.servers.clone(),
        };

        Home {
            handoffs: handoffs::inside(self.platform, &path),
            path,
        }
    }
}

/// The directory `~` means inside one sandbox, and the handoff directory that
/// goes with it.
///
/// Made rather than constructed: it is [`Homes`] that decides where a session's
/// own is, and a HOME the caller chose is a directory a renderer would empty.
#[derive(Debug, Clone)]
pub struct Home {
    path: PathBuf,

    /// Where the Conversation's handoff directory is reached from inside, which
    /// is `/tmp/verkstead` wherever a mount makes that one directory per
    /// session and a directory under `path` where none does — see
    /// [`handoffs::inside`], which is where the whole of that difference is.
    handoffs: PathBuf,
}

impl Home {
    /// The directory itself, which is what `~` resolves to inside.
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    /// And where this Conversation's handoff directory is inside.
    pub(crate) fn handoffs(&self) -> &Path {
        &self.handoffs
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
/// binary nobody chose is the failure this removes, and where the server has no
/// image of its own to hand over the session is not started at all, and what is
/// logged is which session that cost.
///
/// **Having none is the wider of the two things that sounds like**: an image
/// the server cannot find, and one it found that will not run. The second is
/// asked at startup by running it — see [`Executable::probed`] — because the
/// invariant this type exists to keep is not one an existence check can stand
/// on. Both say *why* in the startup log as they happen, because by the time a
/// session is refused for want of an image there is nothing left to look at.
///
/// **And an image is sometimes more than a file.** The AppImage carries the
/// libraries it is drawn over beside it, because the machine it lands on may
/// have none of them, and a session given the file alone would be given a
/// binary that cannot load — see [`Bundled`], which is what goes in with it.
#[derive(Debug, Clone)]
pub struct Executable {
    path: PathBuf,

    /// And where a session finds it, which is `bin/verkstead` under the
    /// directory of Verkstead's own — see [`own_directory`]. Made by the bind
    /// on Linux and really there on a Mac, and first on a session's `PATH`
    /// either way.
    ///
    /// Where the image was bundled with libraries this is where the *launcher*
    /// goes instead, and the image itself is under `libexec` behind it — see
    /// [`Bundled`].
    inside: PathBuf,

    /// The libraries the image cannot run without, where it was packed with
    /// any.
    bundled: Option<Bundled>,
}

/// The libraries an image was packed with, and the launcher that points the
/// loader at them.
///
/// **An AppImage is the whole of why this is here.** It carries GTK and
/// everything under it in `usr/lib`, and `AppRun` points the loader there with
/// `LD_LIBRARY_PATH` before it execs the binary — so the file runs for the
/// human on a machine that has none of those installed. A session gets no
/// `AppRun` and no such variable: its environment is cleared, `/tmp` is a
/// tmpfs of its own so the mounted image is not even there, and what it is
/// handed is the one file. On a machine with the toolkit installed that file
/// loads off `/usr/lib` and all is well; on the machine the AppImage was made
/// for, it does not load at all. So the libraries go in beside it.
///
/// **Through a launcher rather than through the session's own environment.**
/// An `LD_LIBRARY_PATH` set for the session would be set for everything the
/// session runs — its agent, `git`, `cargo`, `rustc` — and what is in that
/// directory is not only GTK: `libz`, `libexpat`, `libpcre2` and forty more,
/// built against whatever the artifact was built on. Ahead of the machine's own
/// for every process in the sandbox, that is a session whose toolchain has been
/// quietly re-pointed. So `bin/verkstead` is two lines of `/bin/sh` that set
/// the variable and exec the image behind it, which is `AppRun`'s own trick at
/// the scope it belongs at: the one binary that needs those libraries.
#[derive(Debug, Clone)]
struct Bundled {
    /// Where they are on the host.
    libraries: PathBuf,

    /// And where a session finds them, which is `lib` beside the `bin` the
    /// launcher is in.
    libraries_inside: PathBuf,

    /// The launcher on the host, written under the Data Directory — see
    /// [`LAUNCHER`].
    launcher: PathBuf,

    /// And where the image goes, which is behind the launcher rather than on
    /// the `PATH` — see [`LIBEXEC`].
    image_inside: PathBuf,
}

impl Executable {
    /// The running server's own image, as a session started against `data_dir`
    /// finds it.
    ///
    /// `None` where the process cannot say what it is running, and `None` too
    /// where what it names is no longer a file: a binary replaced under a
    /// running server is exactly that, and `/proc` answers for it with a path
    /// marked `(deleted)` that no bind can be made from. **Both are said in the
    /// log here**, because neither is said anywhere else: the session that is
    /// refused for want of an image names which session it cost and cannot name
    /// why, there being nothing left by then to look at.
    ///
    /// And packed with whatever the runtime that started this process says it
    /// was packed with — see [`Executable::bundling`].
    pub fn of_the_server(data_dir: &Path) -> Option<Executable> {
        let running = match std::env::current_exe() {
            Ok(running) => running,
            Err(error) => {
                tracing::error!(
                    error = ?error,
                    "Verkstead cannot say what image it is running, so no session can be \
                     equipped to ask with it and none will be started"
                );
                return None;
            }
        };

        let Some(image) = Executable::at(Platform::HERE, running.clone(), data_dir) else {
            tracing::error!(
                verkstead = %running.display(),
                "Verkstead's own image is not a file any more — a binary replaced under a \
                 running server reads exactly like this — so no session can be equipped to \
                 ask with it and none will be started"
            );
            return None;
        };

        Some(image.bundling(
            data_dir,
            std::env::var_os(APPDIR).map(PathBuf::from).as_deref(),
        ))
    }

    /// The same image, carrying the libraries it was packed with, where
    /// `appdir` says it was packed with any — see [`Bundled`].
    ///
    /// `appdir` is the AppImage runtime's own account of where it mounted this
    /// run's image, and what makes it worth trusting is the two things asked of
    /// it here: that the image being equipped is *inside* it, so a stray
    /// variable cannot point a session's loader somewhere of its own, and that
    /// the libraries are really there.
    ///
    /// **A launcher that could not be written is an image without one**, and
    /// then an image that will not load for a session — which is what the probe
    /// is about to say next, in the log, in the loader's own words. Nothing is
    /// refused here: what this settles is what the image comes with rather than
    /// whether it runs.
    pub fn bundling(mut self, data_dir: &Path, appdir: Option<&Path>) -> Executable {
        let Some(libraries) = appdir
            .filter(|appdir| appdir.is_absolute() && self.path.starts_with(appdir))
            .map(|appdir| appdir.join(BUNDLED_LIBRARIES))
            .filter(|libraries| libraries.is_dir())
        else {
            return self;
        };

        let own = own_directory(Platform::HERE, data_dir);
        let bundled = Bundled {
            libraries,
            libraries_inside: under(&own, LIB),
            launcher: data_dir.join(LAUNCHER),
            image_inside: under(&under(&own, LIBEXEC), VERKSTEAD),
        };

        match bundled.write_the_launcher() {
            Ok(()) => self.bundled = Some(bundled),
            Err(error) => tracing::error!(
                launcher = %bundled.launcher.display(),
                error = ?error,
                "Verkstead could not write the launcher its own image is reached through, so \
                 the libraries it was packed with cannot be handed to a session with it"
            ),
        }

        self
    }

    /// A named one, which is how a test puts the real CLI where the server's own
    /// image goes — a test harness being its own executable.
    ///
    /// `None` for a path with nothing behind it, for the reason above: what this
    /// is for is a bind, and a bind of nothing is a session that will not start.
    ///
    /// **On Windows a session finds it where it already is.** The other two
    /// platforms put it at `bin/verkstead` under the directory of Verkstead's
    /// own — made by a bind on Linux and linked into the Data Directory on a
    /// Mac — and neither of those happens on this one: nothing is bound and
    /// nothing is linked yet, so a name invented here would be a name pointing
    /// at nothing. The real path of the running image is what a session is
    /// given, and the directory holding it is what leads its `PATH` — which is
    /// the same invariant said of a different directory: what a session asks
    /// with is the build that is serving it.
    ///
    /// **`platform` rather than [`Platform::HERE`]**, for the reason
    /// [`own_directory`] and [`Homes::for_conversation`] take one: the arm this
    /// machine will never run is still an arm a test builds a description on,
    /// and the three answers above are three different paths. An Executable
    /// built for one platform and rendered into another's description names a
    /// path that platform has not got — and the rendering that joins a path in
    /// by hand would then be clearing and linking a name off the *host's* root
    /// rather than one inside the session's own profile. Everything outside a
    /// test passes [`Platform::HERE`].
    pub fn at(platform: Platform, path: PathBuf, data_dir: &Path) -> Option<Executable> {
        let path = unwrapped(&path);

        let inside = match platform {
            Platform::Windows => path.clone(),
            Platform::Linux | Platform::MacOs => under(&own_bin(platform, data_dir), VERKSTEAD),
        };

        path.is_file().then_some(Executable {
            path,
            inside,
            bundled: None,
        })
    }

    /// The same image, having proved it runs — and `None`, with the reason in
    /// the log, where it does not.
    ///
    /// Run once, at startup, because that is when there is something to do
    /// about it: an image that will not run is a Verkstead nobody can grill
    /// with, and a human reading the startup line is the one who can replace
    /// it. Not a reason to refuse to start, for the reason a missing image is
    /// not — the workbench, the Timeline and every record in it are still
    /// there to read — so what is refused is sessions, one at a time and each
    /// of them named as it is refused (see [`crate::sessions`]).
    ///
    /// The *why* is therefore said here and the *which* is said there. Nothing
    /// probes at spawn: an image that ran at startup is the same file every
    /// session after it is handed, and running it once per session would be a
    /// second answer to a question already answered.
    pub fn probed(self) -> Option<Executable> {
        match self.probe() {
            Ok(()) => Some(self),
            Err(error) => {
                tracing::error!(
                    verkstead = %self.path.display(),
                    error = ?error,
                    "Verkstead's own image will not run in the environment a session runs it \
                     in, so no session can be equipped to ask with it and none will be started"
                );
                None
            }
        }
    }

    /// Whether it runs at all, asked by running it.
    ///
    /// `guide` is the verb, because it is the one that reaches for nothing: it
    /// prints a document compiled into the binary and opens no socket, reads no
    /// directory and asks no server. So a non-zero exit says the file itself
    /// would not run, which is the whole of what is being asked.
    ///
    /// **In the environment a session would get rather than the server's own**,
    /// which is the only reason the answer is worth having. A sandbox clears
    /// the environment and sets the handful of variables inside — see
    /// [`Sandbox::surface`] — so an image that runs only by grace of something
    /// its launcher exported runs for the server and for nobody the server
    /// starts. The AppImage is exactly that: `AppRun` points the loader at the
    /// libraries bundled beside it with `LD_LIBRARY_PATH` and execs the binary
    /// under it, so a probe inheriting this process's environment would pass on
    /// a machine where the same file, run any other way, would not start.
    ///
    /// **And the libraries a session *is* given are given here too**, for the
    /// same reason and read the same way round: a session reaches the image
    /// through a launcher that points the loader at them — see [`Bundled`] —
    /// so a probe that left them out would refuse every AppImage on every
    /// machine rather than the ones where sessions really cannot run. What is
    /// named is the host's copy of the directory the launcher names inside,
    /// those being one directory seen from two places.
    fn probe(&self) -> anyhow::Result<()> {
        let mut command = Command::new(&self.path);
        command.arg(GUIDE).env_clear().stdin(Stdio::null());

        if let Some(bundled) = &self.bundled {
            command.env(LD_LIBRARY_PATH, &bundled.libraries);
        }

        let output = command
            .output()
            .map_err(|error| anyhow::anyhow!("running {} {GUIDE}: {error}", self.path.display()))?;

        if output.status.success() {
            return Ok(());
        }

        // What it said before it gave up, which on the failure this is here to
        // catch is the loader naming the library it could not find. On one line
        // and no longer than a line, because where it is going is a field of a
        // log record rather than a terminal.
        let complaint: String = String::from_utf8_lossy(&output.stderr)
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .take(COMPLAINT)
            .collect();

        anyhow::bail!(
            "{} {GUIDE} failed ({}){}{complaint}",
            self.path.display(),
            output.status,
            if complaint.is_empty() { "" } else { ": " },
        )
    }

    /// Where the image is on the host, which is what the probe runs.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// What a sandbox binds and where, host path first.
    ///
    /// One pair for an ordinary image — the file at `bin/verkstead`, which is
    /// the whole of what a session is given. Three where it was packed with
    /// libraries: the launcher on the `PATH`, the image behind it, and the
    /// libraries the launcher points the loader at — see [`Bundled`].
    fn binds(&self) -> Vec<(&Path, &Path)> {
        let Some(bundled) = &self.bundled else {
            return vec![(self.path.as_path(), self.inside.as_path())];
        };

        vec![
            (bundled.launcher.as_path(), self.inside.as_path()),
            (self.path.as_path(), bundled.image_inside.as_path()),
            (
                bundled.libraries.as_path(),
                bundled.libraries_inside.as_path(),
            ),
        ]
    }

    /// And the directory a session finds it in, which is what goes first on a
    /// session's `PATH` — see [`path`].
    fn bin(&self) -> &Path {
        self.inside
            .parent()
            .expect("a path built by joining two names onto a directory has one")
    }
}

impl Bundled {
    /// Write the launcher, which is `bin/verkstead` as a session finds it.
    ///
    /// Two lines of `/bin/sh`: the library path, and the image behind it. Every
    /// path in it is the path *inside*, because that is the only place it is
    /// ever run — the probe runs the image itself and says the same thing with
    /// an environment variable, see [`Executable::probe`].
    ///
    /// The variable is **set rather than prepended**. A session's environment is
    /// cleared and this is not one of the handful put back — see
    /// [`Sandbox::surface`] — so there is nothing to keep, and reading one that
    /// the sandbox does not set would be reading whatever a future stage
    /// happened to add.
    ///
    /// Rewritten at every startup rather than written once: the paths in it
    /// follow the Data Directory, and a launcher left by a Verkstead that was
    /// pointed somewhere else is a launcher naming a directory this run does not
    /// bind.
    fn write_the_launcher(&self) -> std::io::Result<()> {
        let launcher = format!(
            "#!/bin/sh\n\
             {LD_LIBRARY_PATH}={}\n\
             export {LD_LIBRARY_PATH}\n\
             exec {} \"$@\"\n",
            self.libraries_inside.display(),
            self.image_inside.display(),
        );

        if let Some(dir) = self.launcher.parent() {
            std::fs::create_dir_all(dir)?;
        }

        std::fs::write(&self.launcher, launcher)?;

        // And executable, which is the whole of what makes it the thing on the
        // `PATH` rather than a file beside it. Unix alone: there is no AppImage
        // on the other platforms and so no launcher, and no mode bit to set if
        // there were.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            std::fs::set_permissions(&self.launcher, std::fs::Permissions::from_mode(0o755))?;
        }

        Ok(())
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

    /// The shell this sandbox was built to run, where it was built to run one.
    ///
    /// `None` is a session: it runs an agent, and `SHELL` is there for whatever
    /// that agent shells out with, so [`SHELL`] is the answer. `Some` is a
    /// Conversation's own terminal, where the shell *is* the command — so
    /// `SHELL` names it, and the machine is told its own environment has already
    /// been said. See [`Sandbox::shelled`].
    shell: Option<String>,

    /// The shared Rust build cache this session gets, or `None` where the human
    /// switched it off or this server has none to give.
    ///
    /// Decided as the sandbox is built rather than held from startup, for the
    /// reason the token and the author are: the switch is in `config.yaml`, the
    /// file is read at every spawn, so flipping it in the workbench applies to
    /// the next session and a running one keeps what it started with.
    build_cache: Option<build_cache::Shared>,

    /// And which of the three renderings this description is turned into, and
    /// which platform's names its environment holds.
    ///
    /// Taken off the [`Homes`] the sandbox was built against rather than read
    /// from a `cfg`, for the reason that type carries one: the arm this machine
    /// will never run is still an arm a test can build a sandbox on. Everything
    /// outside a test hands over a `Homes` made with [`Platform::HERE`].
    platform: Platform,
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
    /// missing it is not a smaller sandbox but a wrong one. And on the platform
    /// that joins an account in by hard link, an account on another volume from
    /// the Data Directory is the same answer for the same reason — see
    /// [`across_volumes`].
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
        let home = homes.for_conversation(conversation.id);

        // And whether the account can be joined into that profile at all, which
        // is a question one platform asks — see [`across_volumes`]. Refused
        // here rather than found out as the session is rendered: a hard link
        // that will not be made is a session started into a profile with no
        // account in it, logged out with nothing saying why.
        if let Some(elsewhere) = across_volumes(homes.platform(), home.path(), &profile.account) {
            tracing::error!(
                conversation_id = conversation.id,
                account = %elsewhere.display(),
                profile = %home.path().display(),
                "the Profile's account is on a different volume from the profile a session is \
                 given, and a file is joined into that profile by a hard link, which one volume \
                 is the whole of what it needs — so this session was not started. Move the \
                 account onto the Data Directory's volume, or the Data Directory onto the \
                 account's"
            );

            return None;
        }

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
            home,
            github_token: secrets.github_token().map(str::to_owned),
            git_author: config.git_author().clone(),
            server: reachable.asking_from(conversation.id),
            binds,
            shell: None,
            build_cache: cache.shared(config.rust_build_cache()),
            platform: homes.platform(),
        })
    }

    /// The same sandbox, built to run `shell`: `SHELL` names it inside, where a
    /// session's says [`SHELL`], and the machine's own shell initialisation is
    /// told it has nothing to do — see [`NIXOS_ENVIRONMENT_DONE`].
    ///
    /// Which shell is said by the caller rather than worked out here, because
    /// which one a human gets is the terminals' business and not the sandbox's —
    /// see [`crate::terminals`], the one caller, which runs that shell as the
    /// command as well as naming it here.
    pub fn shelled(mut self, shell: &str) -> Sandbox {
        self.shell = Some(shell.to_owned());
        self
    }

    /// Where a session finds the sccache it compiles through — see
    /// [`sccache_inside`], which is the whole of the answer and is shared with
    /// the compile server.
    ///
    /// Read off the executable rather than said again, so that the one answer
    /// about where a directory of Verkstead's own is on this machine serves
    /// both of the things in it.
    fn sccache_inside(&self, sccache: &Path) -> PathBuf {
        sccache_inside(self.platform, self.verkstead.bin(), sccache)
    }

    /// `argv` as it will be run inside the sandbox, by whichever mechanism this
    /// machine has one — and what is left to see to once it has gone.
    ///
    /// The command is not put through a shell: what runs inside is an argument
    /// vector the orchestrator built, and a shell between it and the sandbox
    /// would be one more thing to quote for.
    ///
    /// **The second half is held for as long as the first one runs**, and asked
    /// to close when it has gone: a rendering that joined the account into a
    /// session's profile by hard link is one whose ending has something left to
    /// do about a file the session replaced rather than wrote. Whoever started
    /// the process is who holds it — a session's relay and a Conversation
    /// Terminal's follow loop, the two things that know when what they started
    /// is over. Nothing at all on the two platforms whose links follow their own
    /// target; see [`Closing`].
    pub fn command<S: AsRef<OsStr>>(&self, argv: &[S]) -> (Rendering, Closing) {
        rendered(self.platform, &self.surface(argv))
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
        let mut surface = on_the_machine(self.platform, self.worktree.clone());

        // HOME before anything that goes inside it: the directory has to be
        // there for the account to land in, and everything else about it stays
        // absent.
        surface.made(Access::Empty(self.home.path().to_owned()));

        // And where a temporary file goes on the platform whose one is inside
        // the profile rather than shared with the machine — after the profile,
        // because it is under it and what emptied that would take this with it.
        // See [`temporary_inside`], and [`on_the_machine`] for the two
        // platforms that say this before ever reaching here.
        if self.platform == Platform::Windows {
            surface.made(Access::Temporary(temporary_inside(self.home.path())));
        }

        surface
            .own(&self.worktree, Reach::ReadWrite)
            .own(&self.git_dir, Reach::ReadWrite);

        // And the account, in the shape its agent type keeps one: what goes
        // where is that type's own business, and a backend arriving with an
        // account of its own lands here rather than in whatever the pair
        // happened to mean. Which shape that is, is [`account_inside`]'s — the
        // one place the four are written down, because the platform that joins
        // an account in by hand has to know the same thing.
        for (host, inside) in account_inside(&self.account, self.home.path()) {
            surface.elsewhere(host, inside, Reach::ReadWrite);
        }

        // After the temporary filesystem and the empty HOME alike, because on
        // one platform or the other it is inside each of them: a tmpfs would
        // otherwise land over it and leave the session writing its handoff into
        // memory nothing outside will ever read, and an emptied HOME would take
        // the link away again. Which of the two it is under is
        // [`handoffs::inside`]'s, and this is the one place a session reaches
        // it.
        surface.elsewhere(&self.handoff_dir, self.home.handoffs(), Reach::ReadWrite);

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
        //
        // Three things rather than one where the image was packed with the
        // libraries it runs over: the launcher on the `PATH`, the image behind
        // it, and the libraries themselves — see [`Bundled`], which is where the
        // whole of that is. Nothing about the `PATH` or the environment changes
        // either way, which is the point of its being a launcher.
        for (host, inside) in self.verkstead.binds() {
            surface.elsewhere(host, inside, Reach::ReadOnly);
        }

        // And the shared build cache: the directory writable at its own place,
        // and the sccache that compiles into it read-only in the directory the
        // binary above just made. After the empty HOME, so that a cache under
        // the server's own home — which is where it is when nobody has
        // configured one — is inside it rather than wiped by it. See
        // [`crate::build_cache`].
        if let Some(cache) = &self.build_cache {
            surface.own(cache.dir(), Reach::ReadWrite);

            if let Some(sccache) = cache.sccache() {
                surface.elsewhere(sccache, self.sccache_inside(sccache), Reach::ReadOnly);
            }
        }

        for bind in &self.binds {
            surface.own(&bind.path, bind.reach);
        }

        surface
            .set("HOME", self.home.path())
            .set("PATH", path(self.platform, self.verkstead.bin()))
            // Which shell is inside, for the same reason `PATH` is said here:
            // the environment is cleared, so a tool that shells out reaches for
            // whatever this holds — and with nothing in it, it would fall back
            // to whatever login shell the passwd file gives the user the server
            // happens to run as. A terminal says the shell the human is actually
            // typing into — see [`Sandbox::shelled`].
            .set("SHELL", self.shell.as_deref().unwrap_or(SHELL))
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

        // And the names nothing on Windows runs without, which the two Unixes
        // have no equivalent of — see [`windows_names`].
        if self.platform == Platform::Windows {
            for (name, value) in windows_names(self.home.path()) {
                surface.set(name, value);
            }
        }

        // And where the command *is* a shell, that this machine's own
        // environment has already been said — see [`NIXOS_ENVIRONMENT_DONE`],
        // which is what stops the shell rebuilding the `PATH` above out of the
        // host's profile the moment it starts.
        if self.shell.is_some() {
            surface.set(NIXOS_ENVIRONMENT_DONE, "1");
        }

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
            if let Some(sccache) = cache.sccache() {
                surface
                    .set("RUSTC_WRAPPER", self.sccache_inside(sccache))
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
///
/// **And nothing is asked at all on Windows**, which is why the platform is a
/// parameter. There is no `nix` on that machine to answer with, so the question
/// has one answer whatever the worktree holds — and asking it anyway would be a
/// process started, failed and read per session for a result already known.
/// Said as the first half of the condition, so that the arm the answer is known
/// on never reaches the arm that shells out.
pub fn under_dev_shell(platform: Platform, worktree: &Path, argv: &[String]) -> Vec<String> {
    if platform == Platform::Windows || !dev_shell(worktree) {
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

        Executable::at(Platform::HERE, bin.join(name), data_dir)
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
        // Read as *what it begins with* rather than by splitting on the
        // separator: the directory is this machine's own whichever platform is
        // being rendered for, and a Windows path split on a colon comes apart
        // at the drive letter.
        for platform in [Platform::Linux, Platform::MacOs, Platform::Windows] {
            let inside = path(platform, equipped.bin());
            let leads = {
                let mut leads = equipped.bin().as_os_str().to_owned();
                leads.push(separator(platform));

                leads
            };

            assert!(
                inside
                    .as_encoded_bytes()
                    .starts_with(leads.as_encoded_bytes()),
                "on {platform:?} the server's own build has to be found before the \
                 machine's install, and the `PATH` is {inside:?}"
            );
        }
    }

    /// What each of the three renderings leaves to be seen to once what it
    /// started has gone — see [`Closing`].
    ///
    /// The one that joins a file into a session's profile by hard link leaves
    /// that file, because a link stops being one file the moment something
    /// renames over it; the two whose links follow their own target leave
    /// nothing at all.
    ///
    /// **Of whichever renderings this build carries**, which is the one thing
    /// here that is not a value. A renderer is compiled where there is a
    /// machine to run it or a test to ask it — see the modules at the top of
    /// this file — and the seatbelt one is neither on a Windows build: what it
    /// renders is a policy about a filesystem where every path is a Unix path.
    /// So a Mac's rendering is asked for from a Unix, which is where the whole
    /// of that arm is asked about anyway.
    #[test]
    fn only_the_rendering_that_links_a_file_by_hand_leaves_anything_to_close() {
        let account = tempfile::tempdir().unwrap();
        let profile = tempfile::tempdir().unwrap();

        let host = account.path().join("config.json");
        let inside = profile.path().join("config.json");

        std::fs::write(&host, "the account's own\n").unwrap();

        let described = || {
            let mut surface = Surface::starting_in(profile.path().to_owned());

            surface.elsewhere(&host, &inside, Reach::ReadWrite);
            surface.running(&["the-agent"]);

            surface
        };

        let following: &[Platform] = if cfg!(unix) {
            &[Platform::Linux, Platform::MacOs]
        } else {
            &[Platform::Linux]
        };

        for platform in following {
            let (_, closing) = rendered(*platform, &described());

            assert_eq!(
                closing.linked().count(),
                0,
                "a bind and a symbolic link both follow whatever happens at the \
                 far end of them, so {platform:?} has nothing left to do"
            );
        }

        let (_, closing) = rendered(Platform::Windows, &described());

        assert_eq!(
            closing.linked().collect::<Vec<_>>(),
            [inside.as_path()],
            "and the file joined in by hard link is what a session's ending is \
             asked about"
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
            own_directory(Platform::Windows, data),
            data,
            "and neither can Windows, having no binds at all yet"
        );
        assert_eq!(
            own_directory(Platform::Linux, data),
            Path::new(OWN_DIRECTORY),
            "and a bind makes a directory that is nobody's on the host"
        );
    }

    /// What a session's `PATH` holds after the directory of Verkstead's own,
    /// which is where a Windows machine differs from the other two in kind: the
    /// two Unixes are told a list written down here, and Windows is handed the
    /// `PATH` the server itself was started with — see [`servers_path`].
    #[test]
    fn a_windows_session_is_given_the_machines_own_path_rather_than_a_list() {
        let ours = Path::new("C:/verkstead/bin");
        let inside = path(Platform::Windows, ours);
        let inside = inside.to_string_lossy();
        let servers = servers_path();

        assert_eq!(
            inside.split_once(';'),
            Some((
                ours.to_str().expect("a name this test wrote"),
                &*servers.to_string_lossy()
            )),
            "the whole of it is Verkstead's own directory, a semicolon because a \
             colon on this platform is a drive letter's own, and then the `PATH` \
             the server itself was started with — there is no list of Windows \
             paths written down anywhere"
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

    /// What a Windows path says about the volume it is on, which is what a hard
    /// link into a session's profile turns on.
    ///
    /// Read here rather than asked of the machine, which is the half of
    /// [`volume`] a test anywhere can ask: the machine's own answer is only
    /// ever the same answer said more exactly.
    #[test]
    fn a_windows_path_says_which_volume_it_is_on_whoever_is_reading_it() {
        let read = |name: &str| {
            written(Path::new(name)).map(|volume| String::from_utf8(volume).expect("ascii"))
        };

        assert_eq!(read(r"C:\Users\someone\.claude"), Some("C:".to_owned()));
        assert_eq!(
            read(r"c:/users/someone/.claude.json"),
            Some("C:".to_owned()),
            "one drive however it is written: the case is nothing and either \
             separator is the separator"
        );
        assert_eq!(
            read(r"\\?\D:\accounts\.claude"),
            Some("D:".to_owned()),
            "and the spelling that says only that what follows goes to the \
             filesystem unchanged"
        );

        assert_ne!(
            read(r"C:\data"),
            read(r"D:\accounts"),
            "two drives are two volumes, which is what refuses a session"
        );

        assert_eq!(
            read(r"\\workshop\accounts\someone\.claude"),
            Some(r"\\WORKSHOP\ACCOUNTS".to_owned()),
            "a UNC name is on the share rather than on the server"
        );
        assert_eq!(
            read(r"\\?\UNC\workshop\accounts\someone\.claude"),
            read(r"\\workshop\accounts\someone\.claude"),
            "however that name is spelled"
        );
        assert_ne!(
            read(r"\\workshop\accounts"),
            read(r"\\workshop\backups"),
            "and two shares on one server are two volumes"
        );

        assert_eq!(
            read("/home/someone/.claude"),
            None,
            "a POSIX name says nothing about a Windows volume, and every path \
             on the machine running this test is one"
        );
    }

    /// And what that comes to for a Profile: an account this platform cannot
    /// hard-link into a session's profile, and one it can.
    #[test]
    fn an_account_with_a_file_on_another_volume_is_one_that_cannot_be_joined_in() {
        // A directory that is really one, because that is the question this
        // asks of each of an account's paths: a directory is joined in by a
        // junction and everything else by a hard link.
        let account = tempfile::tempdir().unwrap();

        let claude = |config: &str| store::Account::Claude {
            claude_dir: account.path().to_owned(),
            config_file: PathBuf::from(config),
        };

        let profile = Path::new(r"C:\ProgramData\Verkstead\homes\7");
        let elsewhere = r"D:\accounts\someone\.claude.json";

        assert_eq!(
            across_volumes(Platform::Windows, profile, &claude(elsewhere)),
            Some(PathBuf::from(elsewhere)),
            "the file half is what a hard link is asked for, and it is on \
             another drive"
        );

        assert_eq!(
            across_volumes(
                Platform::Windows,
                profile,
                &claude(r"C:\Users\someone\.claude.json")
            ),
            None,
            "and one volume is the whole of what it needed"
        );

        assert_eq!(
            across_volumes(
                Platform::Windows,
                profile,
                &store::Account::Codex {
                    home: account.path().to_owned(),
                }
            ),
            None,
            "an account that is one directory is joined in by a junction, which \
             crosses volumes and asks nothing of either"
        );

        assert_eq!(
            across_volumes(
                Platform::Linux,
                Path::new("/home/verkstead"),
                &claude(elsewhere)
            ),
            None,
            "and the two platforms that mount or symlink a path in ask this of \
             nobody"
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

        for platform in [Platform::MacOs, Platform::Windows] {
            let homes = Homes::on(platform, servers.clone(), data);

            assert_eq!(
                homes.for_conversation(7).path(),
                Path::new("/data/homes/7"),
                "{platform:?} has nothing to mount, so the directory is really made"
            );
            assert_ne!(
                homes.for_conversation(8).path(),
                homes.for_conversation(7).path(),
                "and one Conversation's HOME is not another's"
            );
        }
    }

    /// And its handoff directory comes with it, which is what keeps two
    /// Conversations running at once out of each other's document.
    ///
    /// `/tmp` is a filesystem of the session's own where a mount makes one and
    /// the machine's one real directory where none does — so the handoff is
    /// reached under HOME on the second, and a HOME is already one directory
    /// per Conversation there. See [`handoffs`].
    #[test]
    fn the_handoff_directory_is_reached_wherever_that_home_makes_it_the_conversations_own() {
        let data = Path::new("/data");
        let servers = PathBuf::from("/home/verkstead");

        let linux = Homes::on(Platform::Linux, servers.clone(), data);
        assert_eq!(
            linux.for_conversation(7).handoffs(),
            Path::new(handoffs::INSIDE),
            "a tmpfs makes that one path a directory per session already"
        );

        for platform in [Platform::MacOs, Platform::Windows] {
            let homes = Homes::on(platform, servers.clone(), data);

            assert_eq!(
                homes.for_conversation(7).handoffs(),
                Path::new("/data/homes/7/verkstead"),
                "{platform:?} reaches it under the HOME it was given"
            );
            assert_ne!(
                homes.for_conversation(8).handoffs(),
                homes.for_conversation(7).handoffs(),
                "and the Conversation beside it is writing somewhere else entirely"
            );
        }
    }

    /// A packaged binary is a wrapper and a dotted file beside it, and a
    /// packaged server is the second of the two — see [`unwrapped`].
    #[test]
    fn a_wrapped_executable_resolves_to_the_wrapper_beside_it() {
        let bin = tempfile::tempdir().unwrap();

        std::fs::write(bin.path().join("verkstead"), "#!/bin/sh\n").unwrap();
        std::fs::write(bin.path().join(".verkstead-wrapped"), "an ELF\n").unwrap();

        let executable = Executable::at(
            Platform::HERE,
            bin.path().join(".verkstead-wrapped"),
            bin.path(),
        )
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

        let executable =
            Executable::at(Platform::HERE, path.clone(), bin.path()).expect("the file is there");

        assert_eq!(executable.path(), path);
    }

    /// A binary replaced under a running server, which is what an upgrade is:
    /// there is nothing left to bind, and saying so is what stops a session
    /// being equipped with the machine's install instead.
    #[test]
    fn an_executable_that_is_not_there_equips_nobody() {
        let bin = tempfile::tempdir().unwrap();

        assert!(Executable::at(Platform::HERE, bin.path().join("verkstead"), bin.path()).is_none());
    }

    /// An image that runs `script` as the whole of whatever verb it is given.
    ///
    /// A script rather than a binary because what the probe asks of an image is
    /// whether the machine will run the file at all — and a shell script that
    /// exits 127 with a loader's complaint on stderr is the same answer to that
    /// question as an ELF whose libraries are missing. Unix alone: the mode bit
    /// is what makes it an image, and Windows has no such thing.
    #[cfg(unix)]
    fn image(dir: &Path, script: &str) -> Executable {
        use std::os::unix::fs::PermissionsExt;

        let path = dir.join(VERKSTEAD);
        std::fs::write(&path, format!("#!/bin/sh\n{script}\n")).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();

        Executable::at(Platform::HERE, path, dir).expect("the file was just written")
    }

    /// The probe is a run of the image, and the verb is the one that reaches
    /// for nothing — see [`Executable::probe`].
    ///
    /// Once, and at startup: the count is what says a session's spawn asks the
    /// image nothing, because a probe per spawn would be a process started for
    /// every session to answer what was already known.
    #[test]
    #[cfg(unix)]
    fn an_image_that_answers_the_guide_equips_every_session_after_it() {
        let dir = tempfile::tempdir().unwrap();
        let ran = dir.path().join("ran");
        let image = image(dir.path(), &format!("echo \"$@\" >> {}", ran.display()));

        assert!(
            image.probed().is_some(),
            "an image that runs is one every session can ask with"
        );

        let runs = std::fs::read_to_string(&ran).unwrap();
        assert_eq!(
            runs.lines().collect::<Vec<_>>(),
            [GUIDE],
            "the image is run once, with the verb that opens nothing"
        );
    }

    /// And one that will not run equips nobody, the same as one that is not
    /// there — with what the machine said about it carried out for the startup
    /// log, which is the only place a human finds out what to replace.
    #[test]
    #[cfg(unix)]
    fn an_image_that_will_not_run_equips_nobody() {
        let dir = tempfile::tempdir().unwrap();
        let image = image(
            dir.path(),
            "echo 'libgtk-3.so.0: cannot open shared object file' >&2\nexit 127",
        );

        let refused = format!("{:?}", image.probe().expect_err("it exits 127"));

        assert!(
            refused.contains("libgtk-3.so.0"),
            "the log line has to say what would not load, got {refused:?}"
        );
        assert!(
            image.probed().is_none(),
            "an image that will not run is one no session can ask with"
        );
    }

    /// And it is run in the environment a session would get rather than in this
    /// process's own, which is the whole of what the probe is worth.
    ///
    /// `HOME` stands for the environment here: it is set for anything a human
    /// or a service manager started, a shell that inherits nothing does not
    /// invent one, and it is not a variable a sandbox passes through — see
    /// [`Sandbox::surface`], which sets a home of the session's own. An image
    /// that saw this process's would have seen its `LD_LIBRARY_PATH` too, which
    /// is the variable that makes a bundled AppImage pass a probe it should
    /// fail.
    #[test]
    #[cfg(unix)]
    fn the_probe_runs_the_image_in_a_session_environment_rather_than_the_servers() {
        assert!(
            std::env::var_os("HOME").is_some(),
            "this process has to have a HOME for its absence inside to mean anything"
        );

        let dir = tempfile::tempdir().unwrap();
        let saw = dir.path().join("saw");
        let image = image(
            dir.path(),
            &format!("echo \"${{HOME-nothing}}\" > {}", saw.display()),
        );

        image.probe().expect("the image runs");

        assert_eq!(
            std::fs::read_to_string(&saw).unwrap().trim(),
            "nothing",
            "the server's own environment is not what a session runs the image in"
        );
    }

    /// An AppDir at `dir` with `libraries` under it, and an image inside it
    /// where an AppImage's own is — which is the whole of what
    /// [`Executable::bundling`] asks of a runtime's claim.
    #[cfg(unix)]
    fn appdir(dir: &Path, script: &str) -> Executable {
        let bin = dir.join("usr/bin");
        std::fs::create_dir_all(&bin).unwrap();
        std::fs::create_dir_all(dir.join(BUNDLED_LIBRARIES)).unwrap();

        image(&bin, script)
    }

    /// An image packed with libraries is handed over as three things rather than
    /// one, and what is on the `PATH` is the launcher — see [`Bundled`].
    ///
    /// The libraries a session gets are the AppDir's own, at a path of
    /// Verkstead's own inside; the image is behind the launcher rather than
    /// beside it, so that nothing but the launcher is on the `PATH`.
    #[test]
    #[cfg(unix)]
    fn an_image_packed_with_libraries_hands_a_session_the_launcher_and_the_two_behind_it() {
        let dir = tempfile::tempdir().unwrap();
        let data_dir = tempfile::tempdir().unwrap();
        let image = appdir(dir.path(), "exit 0").bundling(data_dir.path(), Some(dir.path()));

        let own = own_directory(Platform::HERE, data_dir.path());
        let launcher = data_dir.path().join(LAUNCHER);
        let on_the_path = under(&under(&own, BIN), VERKSTEAD);
        let behind_it = under(&under(&own, LIBEXEC), VERKSTEAD);
        let libraries_inside = under(&own, LIB);
        let libraries = dir.path().join(BUNDLED_LIBRARIES);

        assert_eq!(
            image.binds(),
            vec![
                (launcher.as_path(), on_the_path.as_path()),
                (image.path(), behind_it.as_path()),
                (libraries.as_path(), libraries_inside.as_path()),
            ],
        );

        let written = std::fs::read_to_string(&launcher).unwrap();
        assert!(
            written.contains(&format!("LD_LIBRARY_PATH={}", libraries_inside.display()))
                && written.contains(&format!("exec {} \"$@\"", behind_it.display())),
            "the launcher should point the loader at the libraries and exec the image, got:\n\
             {written}"
        );
    }

    /// And an ordinary image is one thing, with no launcher and nothing said to
    /// the loader — which is every image but an AppImage's.
    ///
    /// The three ways a runtime's claim is refused are each one of these: no
    /// variable at all, one naming a directory the image is not under, and one
    /// with no libraries in it.
    #[test]
    #[cfg(unix)]
    fn an_image_that_was_packed_with_nothing_is_the_one_file_it_always_was() {
        let dir = tempfile::tempdir().unwrap();
        let data_dir = tempfile::tempdir().unwrap();
        let elsewhere = tempfile::tempdir().unwrap();
        let bare = tempfile::tempdir().unwrap();

        for said in [None, Some(elsewhere.path()), Some(bare.path())] {
            let image = appdir(dir.path(), "exit 0").bundling(data_dir.path(), said);
            let inside = image.inside.clone();

            assert_eq!(
                image.binds(),
                vec![(image.path(), inside.as_path())],
                "an image the runtime said nothing usable about is bound as itself, \
                 and {said:?} said nothing usable"
            );
            assert!(
                !data_dir.path().join(LAUNCHER).exists(),
                "and there is no launcher to write"
            );
        }
    }

    /// The probe runs the image the way a session reaches it, libraries and all:
    /// an image that needs what it was packed with passes, and the same image
    /// without them does not.
    ///
    /// Which is the whole of what the probe is worth on an AppImage. Read the
    /// other way round it is the same claim: a probe that left the libraries out
    /// would refuse every AppImage on every machine, rather than the ones where
    /// a session really could not run one.
    #[test]
    #[cfg(unix)]
    fn the_probe_gives_the_image_the_libraries_a_session_would_give_it() {
        let dir = tempfile::tempdir().unwrap();
        let data_dir = tempfile::tempdir().unwrap();
        let needs_them = "[ -n \"${LD_LIBRARY_PATH-}\" ] || { \
             echo 'libgtk-3.so.0: cannot open shared object file' >&2; exit 127; }";

        let packed = appdir(dir.path(), needs_them).bundling(data_dir.path(), Some(dir.path()));
        assert!(
            packed.probe().is_ok(),
            "an image packed with what it needs is one a session can ask with"
        );

        let bare = appdir(dir.path(), needs_them);
        assert!(
            bare.probe().is_err(),
            "and the same image with nothing packed beside it is not"
        );
    }
}
