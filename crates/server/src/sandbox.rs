//! The sandbox a session runs in: a bwrap surface built around one
//! Conversation's worktree, and nothing else.
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
//! - **read-only** — `/nix` and the system paths, the bundled skills at
//!   `/verkstead/skills`, an empty directory over the `~/.claude/skills` they
//!   used to hide, and the executable serving all this, as `verkstead`
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

use std::ffi::OsStr;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::build_cache::{self, BuildCache};
use crate::handoffs::{self, Handoffs};
use crate::settings::{Config, GitAuthor, Secrets};
use crate::skills::{self, Skills};
use crate::store;
use crate::terminal;

/// The system directories a sandbox gets read-only, in the order bwrap is told
/// about them.
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
pub(crate) const SYSTEM: [&str; 7] = [
    "/nix",
    "/usr",
    "/bin",
    "/lib",
    "/lib64",
    "/etc",
    "/run/current-system",
];

/// Where the server's own executable is mounted, which is what a session runs
/// as `verkstead`.
///
/// In a directory of Verkstead's own rather than under a name inside one of the
/// system binds: those are the host's and read-only, so there is nowhere in them
/// to put a file. The directory is made by the bind itself, holds this one
/// executable and nothing else, and goes first on [`PATH`].
const VERKSTEAD_INSIDE: &str = "/verkstead/bin/verkstead";

/// And where the sccache the shared build cache compiles through is mounted,
/// beside it.
///
/// The same trick, for the same reason: the binary is the server's own to
/// choose, the directory is made by the bind and holds nothing the host put
/// there, and an absolute `RUSTC_WRAPPER` is one that works whatever a project's
/// dev shell does to `PATH`. See [`crate::build_cache`].
pub(crate) const SCCACHE_INSIDE: &str = "/verkstead/bin/sccache";

/// What a session's `PATH` is inside.
///
/// Verkstead's own directory first — see [`Executable`] for why a session asks
/// with the server's own build rather than with whatever the machine has
/// installed — then the system profile, then the Nix default profile, then the
/// paths a non-NixOS `/usr` would put things in. Not inherited from the server's
/// own environment: what a session can run should be a fact about the sandbox
/// rather than about however the unit that started the orchestrator happened to
/// be launched.
///
/// And what the compile server has, for the same reason: it is a fact about
/// what a sandbox holds rather than about either process.
pub(crate) const PATH: &str =
    "/verkstead/bin:/run/current-system/sw/bin:/nix/var/nix/profiles/default/bin:/usr/bin:/bin";

/// And what a session's `SHELL` is: the one path the system bind is certain to
/// have a shell at, on NixOS and everywhere else.
const SHELL: &str = "/bin/sh";

/// GitHub over HTTPS, which is the one host a sandbox is given credentials for
/// and the one every SSH remote is rewritten to — see [`Sandbox::git_config`].
///
/// Without a trailing slash, because a credential scope has none and the URL
/// rewrite says its own.
const GITHUB: &str = "https://github.com";

/// The host directory `~` means inside a sandbox.
///
/// A session's HOME is the server's own, at the same path inside — and that is
/// the whole of what this is for. Nothing is read out of it any more: it used to
/// give up two things, what `gh` was authenticated as and who git committed as,
/// and both are now said rather than found — a token and an author in the
/// settings files, handed to the session in its environment. What is left is a
/// path, not a place credentials are collected from.
///
/// Nothing of the directory comes through either. HOME inside is an empty
/// directory with the Profile's pair mounted into it, which is what makes one
/// account's sessions invisible to another's.
#[derive(Debug, Clone)]
pub struct Home {
    /// The directory itself, which is what `~` resolves to inside.
    pub path: PathBuf,
}

impl Home {
    /// The home of whoever is running the server.
    ///
    /// Read from the environment rather than from the passwd database: a service
    /// unit says what HOME is, and that is the answer that should count — under
    /// the packaged unit it is what the module sets, and in a development shell
    /// it is the human's own.
    pub fn of_the_server() -> Option<Home> {
        Some(Home {
            path: PathBuf::from(std::env::var_os("HOME")?),
        })
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
}

impl Executable {
    /// The running server's own image.
    ///
    /// `None` where the process cannot say what it is running, and `None` too
    /// where what it names is no longer a file: a binary replaced under a
    /// running server is exactly that, and `/proc` answers for it with a path
    /// marked `(deleted)` that no bind can be made from.
    pub fn of_the_server() -> Option<Executable> {
        Executable::at(std::env::current_exe().ok()?)
    }

    /// A named one, which is how a test puts the real CLI where the server's own
    /// image goes — a test harness being its own executable.
    ///
    /// `None` for a path with nothing behind it, for the reason above: what this
    /// is for is a bind, and a bind of nothing is a session that will not start.
    pub fn at(path: PathBuf) -> Option<Executable> {
        let path = unwrapped(&path);

        path.is_file().then_some(Executable { path })
    }

    /// Where it is on the host, which is what a sandbox binds.
    pub fn path(&self) -> &Path {
        &self.path
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

    /// The bwrap flag that makes it what it is.
    fn flag(&self) -> &'static str {
        match self.reach {
            Reach::ReadOnly => "--ro-bind",
            Reach::ReadWrite => "--bind",
        }
    }
}

/// How far into a bind a session may reach.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Reach {
    ReadOnly,
    ReadWrite,
}

/// Sandbox Configuration: the extra read-write binds a sandbox gets beyond the
/// surface every one of them has.
///
/// Two sets, composed. The global one every sandbox gets, and a per-Repo one so
/// that a repository needing a build cache can say so without every repository
/// getting it. A bind here is a hole in the one boundary a sandbox is, which is
/// why they are configured where the Watched Paths are — in the environment, at
/// installation — rather than anywhere a session or a browser could reach.
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
    /// build writes — and the installer opened the hole on purpose. They sit
    /// outside the repository besides, so a read-only companion whose cache
    /// could not be written to would fail on a cold cache for nothing gained.
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

    /// The Profile's claude directory, mounted over `~/.claude`.
    claude_dir: PathBuf,

    /// And its config file, over `~/.claude.json`. The pair is what keeps
    /// accounts apart, so the two travel together or not at all.
    config_file: PathBuf,

    /// The bundled skills, mounted read-only at [`skills::INSIDE`] — see
    /// [`crate::skills`] for why they are Verkstead's rather than the account's,
    /// why they are read-only, and why the path is nobody's.
    skills: PathBuf,

    /// And the empty directory that goes over `~/.claude/skills` in their place,
    /// read-only, so what the account keeps there stays hidden now that the
    /// mount doing the hiding has moved away — see [`skills::Skills::nothing`].
    nothing: PathBuf,

    /// The executable a session runs as `verkstead`, mounted read-only at
    /// [`VERKSTEAD_INSIDE`] and first on `PATH`.
    ///
    /// The server's own — see [`Executable`] — and read-only for the reason the
    /// skills are: what a session asks with is the product's, and not a file the
    /// session can rewrite mid-run.
    verkstead: PathBuf,

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
        home: Home,
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

        // One arm per agent type: what is bound over where is that type's own
        // shape, and a backend arriving with an account of its own lands here
        // rather than in whatever the pair happened to mean.
        let store::Account::Claude {
            claude_dir,
            config_file,
        } = &profile.account;

        Some(Sandbox {
            worktree,
            git_dir,
            claude_dir: claude_dir.clone(),
            config_file: config_file.clone(),
            skills: skills.path().to_owned(),
            nothing: skills.nothing().to_owned(),
            verkstead: verkstead.path().to_owned(),
            handoff_dir,
            home,
            github_token: secrets.github_token().map(str::to_owned),
            git_author: config.git_author().clone(),
            server: reachable.asking_from(conversation.id),
            binds,
            build_cache: cache.shared(config.rust_build_cache()),
        })
    }

    /// `argv` as it will be run inside the sandbox.
    ///
    /// The command is not put through a shell: what runs inside is an argument
    /// vector the orchestrator built, and a shell between it and bwrap would be
    /// one more thing to quote for.
    pub fn command<S: AsRef<OsStr>>(&self, argv: &[S]) -> Command {
        let mut bwrap = Command::new("bwrap");

        // Nothing of the server's environment comes through. What the sandbox
        // holds is decided here, and a variable the unit happened to be started
        // with — where the database is, what the server listens on — is not part
        // of that decision.
        bwrap.env_clear();

        bwrap.args([
            // A session outlives nothing: if the orchestrator goes, so does
            // whatever it left running.
            "--die-with-parent",
            // Every namespace, and then the network back — see the module's own
            // documentation for why that one.
            "--unshare-all",
            "--share-net",
            "--hostname",
            "verkstead",
        ]);

        for path in SYSTEM.iter().map(Path::new).filter(|path| path.exists()) {
            bwrap.arg("--ro-bind").arg(path).arg(path);
        }

        // `/proc` and `/dev` are made rather than bound: they are the sandbox's
        // own, which is what makes the unshared pid namespace mean anything.
        bwrap.args(["--proc", "/proc", "--dev", "/dev", "--tmpfs", "/tmp"]);

        // HOME before anything mounted into it: the directory has to be there
        // for the pair to land in, and everything else about it stays absent.
        bwrap.arg("--dir").arg(&self.home.path);

        bwrap
            .arg("--bind")
            .arg(&self.worktree)
            .arg(&self.worktree)
            .arg("--bind")
            .arg(&self.git_dir)
            .arg(&self.git_dir)
            .arg("--bind")
            .arg(&self.claude_dir)
            .arg(self.home.path.join(".claude"))
            .arg("--bind")
            .arg(&self.config_file)
            .arg(self.home.path.join(".claude.json"));

        // After `/tmp` is made, and inside it: the tmpfs above would otherwise
        // land over this and leave the session writing its handoff into memory
        // nothing outside will ever read.
        bwrap
            .arg("--bind")
            .arg(&self.handoff_dir)
            .arg(handoffs::INSIDE);

        // The skills, at a path of Verkstead's own outside HOME entirely — the
        // bind makes the directory, so what a session reads there is what this
        // binary ships. Read-only, because what a session is grilled by is the
        // product's and not a file the session can rewrite mid-run.
        bwrap.arg("--ro-bind").arg(&self.skills).arg(skills::INSIDE);

        // And nothing at all where the account's own skills would otherwise be
        // found: after the Profile's directory and inside it, because a bind is
        // applied in the order it is given and the one that lands second is the
        // one that wins. Read-only as the mount that used to stand here was, so
        // a session cannot fill the directory in and then read from it.
        bwrap
            .arg("--ro-bind")
            .arg(&self.nothing)
            .arg(self.home.path.join(skills::CLAUDE_INSIDE_HOME));

        // And the binary the session asks with, in a directory of its own that
        // goes first on `PATH` — see [`Executable`]. The bind makes the
        // directory, so what is on that `PATH` entry is this one file and
        // nothing the host put beside it.
        bwrap
            .arg("--ro-bind")
            .arg(&self.verkstead)
            .arg(VERKSTEAD_INSIDE);

        // And the shared build cache: the directory writable at the same path
        // inside, and the sccache that compiles into it read-only in the
        // directory the binary above just made. After the `--dir` on HOME, so
        // that a cache under the server's own home — which is where it is when
        // nobody has configured one — lands inside the fresh HOME rather than
        // being wiped by it. See [`crate::build_cache`].
        if let Some(cache) = &self.build_cache {
            bwrap.arg("--bind").arg(cache.dir()).arg(cache.dir());

            if let Some(sccache) = cache.sccache() {
                bwrap.arg("--ro-bind").arg(sccache).arg(SCCACHE_INSIDE);
            }
        }

        for bind in &self.binds {
            bwrap.arg(bind.flag()).arg(&bind.path).arg(&bind.path);
        }

        bwrap
            .arg("--chdir")
            .arg(&self.worktree)
            .arg("--setenv")
            .arg("HOME")
            .arg(&self.home.path)
            .arg("--setenv")
            .arg("PATH")
            .arg(PATH)
            // Which shell is inside, for the same reason `PATH` is said here:
            // the environment is cleared, so a tool that shells out reaches for
            // whatever this holds — and with nothing in it, it would fall back
            // to whatever login shell the passwd file gives the user the server
            // happens to run as.
            .arg("--setenv")
            .arg("SHELL")
            .arg(SHELL)
            // And what kind of terminal a session is on, which is a fact about
            // the pseudo-terminal Verkstead opened for it rather than about the
            // sandbox — see [`crate::terminal`]. Said because nothing else
            // would: the environment is cleared, and an interface told nothing
            // draws for the dumbest terminal it knows about.
            .arg("--setenv")
            .arg("TERM")
            .arg(terminal::TERM)
            // What makes a session's Question Sets its own Conversation's. The
            // variable the bundled CLI reads, scoped to one Conversation, so
            // nothing is inferred from the project or the branch — two
            // Conversations against one Repo would be indistinguishable by
            // either.
            .arg("--setenv")
            .arg("VERKSTEAD_SERVER")
            .arg(&self.server);

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
            bwrap
                .arg("--setenv")
                .arg("CARGO_HOME")
                .arg(cache.cargo_home());

            // Only where there is an sccache to point at. Without one this is a
            // cache of downloads and nothing else — see [`crate::build_cache`]
            // — and a `RUSTC_WRAPPER` naming a path that is not mounted would
            // be every Rust build inside failing rather than one running
            // uncached.
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
                bwrap
                    .arg("--setenv")
                    .arg("RUSTC_WRAPPER")
                    .arg(SCCACHE_INSIDE)
                    .arg("--setenv")
                    .arg("SCCACHE_DIR")
                    .arg(cache.sccache_dir())
                    .arg("--setenv")
                    .arg("SCCACHE_CACHE_SIZE")
                    .arg(cache.size());
            }
        }

        // What `gh` inside authenticates as, which it reads from here without
        // being told to and without a file anywhere. Set only where there is one
        // to set: `GH_TOKEN` present and empty is a login `gh` fails on obscurely
        // where its absence is a login it says plainly it does not have.
        if let Some(token) = &self.github_token {
            bwrap.arg("--setenv").arg("GH_TOKEN").arg(token);
        }

        // And the whole of git's configuration, in the environment for the same
        // reason: there is no file inside for it to be in.
        let git_config = self.git_config();

        for (n, (key, value)) in git_config.iter().enumerate() {
            bwrap
                .arg("--setenv")
                .arg(format!("GIT_CONFIG_KEY_{n}"))
                .arg(key)
                .arg("--setenv")
                .arg(format!("GIT_CONFIG_VALUE_{n}"))
                .arg(value);
        }

        bwrap
            .arg("--setenv")
            .arg("GIT_CONFIG_COUNT")
            .arg(git_config.len().to_string())
            // And nothing git cannot answer for itself is asked of anybody.
            // Nobody is at this terminal: a push with no usable credentials has
            // to come back saying so, where a prompt for a username would be a
            // session sitting on a pty until something noticed.
            .arg("--setenv")
            .arg("GIT_TERMINAL_PROMPT")
            .arg("0");

        bwrap.args(argv);

        bwrap
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

    /// The bind puts the executable in one directory and `PATH` sends a session
    /// looking in another, and the two have to be the same place — a `verkstead`
    /// mounted somewhere nothing searches is a session back on the machine's
    /// install without anything saying so.
    #[test]
    fn the_directory_the_binary_is_mounted_in_is_the_first_on_the_path() {
        let mounted = Path::new(VERKSTEAD_INSIDE);

        assert_eq!(
            mounted.file_name().and_then(OsStr::to_str),
            Some("verkstead"),
            "the name on `PATH` is the name the skills and the Guide tell a session to run"
        );
        assert_eq!(
            PATH.split(':').next().map(Path::new),
            mounted.parent(),
            "the server's own build has to be found before the machine's install"
        );
    }

    /// A packaged binary is a wrapper and a dotted file beside it, and a
    /// packaged server is the second of the two — see [`unwrapped`].
    #[test]
    fn a_wrapped_executable_resolves_to_the_wrapper_beside_it() {
        let bin = tempfile::tempdir().unwrap();

        std::fs::write(bin.path().join("verkstead"), "#!/bin/sh\n").unwrap();
        std::fs::write(bin.path().join(".verkstead-wrapped"), "an ELF\n").unwrap();

        let executable = Executable::at(bin.path().join(".verkstead-wrapped"))
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

        let executable = Executable::at(path.clone()).expect("the file is there");

        assert_eq!(executable.path(), path);
    }

    /// A binary replaced under a running server, which is what an upgrade is:
    /// there is nothing left to bind, and saying so is what stops a session
    /// being equipped with the machine's install instead.
    #[test]
    fn an_executable_that_is_not_there_equips_nobody() {
        let bin = tempfile::tempdir().unwrap();

        assert!(Executable::at(bin.path().join("verkstead")).is_none());
    }
}
