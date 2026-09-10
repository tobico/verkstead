//! The shared Rust build cache: one directory on the host that every sandbox
//! writes its cargo downloads and its compiled dependencies into, so a crate is
//! fetched once and compiled once for the machine rather than once per
//! Conversation.
//!
//! Every session used to start cold. A Conversation's `target/` is inside its
//! Worktree, which is deleted when the Conversation closes, and the cargo
//! registry landed in a HOME that is a fresh empty directory per sandbox — so
//! two Conversations against one repository downloaded and compiled the same
//! dependency tree twice, and the same Conversation did it again after a
//! wrap-up. Nothing about that is the human's to configure their way out of:
//! it is a fact about how a sandbox is built, so the fix is one the server
//! hands out.
//!
//! What a sandbox gets is a writable bind of this directory and three or four
//! environment variables — see [`Shared`]. Both are composed in
//! [`crate::sandbox`], because that is where the whole of what a session can
//! reach is decided; what is here is where the directory is, whether there is
//! an sccache to point at, and what the human left the switch on.
//!
//! And **one sccache server for the machine**, which is this module's other
//! half — see [`BuildCache::compiling`]. An sccache is a client and a server,
//! and the server is what actually runs `rustc`: the client in a sandbox only
//! hands it a command line. Left to itself each session's client starts a
//! server of its own, and because every sandbox shares the host's network they
//! all reach for one port — so the second Conversation to build Rust has its
//! compiles executed inside the *first* one's sandbox, which holds no bind for
//! the second one's Worktree, and the build fails outright. So Verkstead runs
//! the server itself, in a sandbox of its own that holds the Worktrees
//! directory and this cache and nothing else of the Data Directory.
//!
//! **On every platform**, which it was not always — see
//! [`compiles_through_an_sccache`]. The half of this that is directories has
//! always worked everywhere; the half that is a client talking to a server over
//! the loopback was off on Windows for as long as a session there ran inside an
//! AppContainer, which is refused the local machine. A session runs as a local
//! account of Verkstead's own now and an ordinary local account is refused
//! nothing of the sort, so the arm is a `true` with a history rather than a
//! difference between platforms.
//!
//! **And the server itself runs behind the boundary its platform has**, which
//! on Windows is the same account a session runs as, with entries of its own
//! written for the directories it compiles in — see [`compile_server`]. A
//! compile server on the host would be every Rust dependency's proc macro
//! running as whoever Verkstead runs as, which is the one thing this
//! arrangement exists not to be, whichever of the three renderings is making
//! the boundary.
//!
//! **Rust by name**, deliberately. Nothing here generalises over languages: a
//! node or a python cache would want its own directory, its own variables and
//! its own switch, and a sibling of this module is where one would go. Naming
//! this one for what it caches is what leaves room for that.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};

use crate::platform::Platform;
use crate::sandbox::account::Logon;
use crate::sandbox::outliving;
use crate::sandbox::{self, Access, Reach, Rendering};
use crate::settings::RustBuildCache;

/// What is looked for on the server's own `PATH`, and the name it is found
/// under inside a sandbox: in the directory of Verkstead's own that the binary
/// a session asks with is in — see [`crate::sandbox::own_bin`].
///
/// The same trick as that binary, for the same reason: which sccache a session
/// compiles through is the server's to choose rather than the machine's to
/// have installed, and an absolute `RUSTC_WRAPPER` is one that works whatever a
/// project's dev shell does to `PATH`. Made by a bind on Linux, where the
/// directory is nowhere on the host at all, and a link really written on a Mac,
/// where it is under the Data Directory.
pub(crate) const SCCACHE: &str = "sccache";

/// What `CARGO_HOME` is inside the cache directory: the registry index, the
/// `.crate` files downloaded into it, and the sources unpacked from them.
///
/// Shared by every session at once, which cargo has locked properly since 1.68:
/// two sessions resolving dependencies at the same moment queue on the registry
/// lock rather than tearing it up between them.
const CARGO: &str = "cargo";

/// And what `SCCACHE_DIR` is beside it: the compiled objects, keyed by the
/// hash of everything that went into producing them.
const SCCACHE_DIR: &str = "sccache";

/// How big the compiled half is allowed to get before sccache starts evicting,
/// where the human has not said. sccache's own default is 10G, which a few
/// Rust workspaces fill; this is a machine's own disk being spent on not
/// compiling things twice.
///
/// The cargo half has no eviction at all and no size to give it: what is in
/// there is what the projects on this machine depend on.
pub(crate) const SIZE: &str = "30G";

/// The `HOME` the compile server is given, which is a directory of Verkstead's
/// own holding nothing.
///
/// It has one because sccache looks for a config file under it, and a process
/// with no `HOME` at all looks for one under `/`. Beside the `bin` the sccache
/// itself is in, for the reason that is where it is: a directory of Verkstead's
/// own rather than a name inside one of the host's — `/verkstead/home` where a
/// bind makes it, and under the Data Directory where nothing can, which is why
/// it is a function of where that is. See [`crate::sandbox::own_directory`].
const COMPILING_HOME: &str = "home";

/// Where that is, for a server keeping its things under `data_dir`.
fn compiling_home(data_dir: &Path) -> PathBuf {
    sandbox::under(
        &sandbox::own_directory(Platform::HERE, data_dir),
        COMPILING_HOME,
    )
}

/// Whether a session on `platform` compiles through an sccache at all — which
/// is the whole of the difference between a cache of downloads and a cache of
/// compiled objects, and the one place it is decided.
///
/// **All three do**, and the third one did not for a while. An sccache is a
/// client and a server that talk over the loopback, and while a Windows session
/// ran inside an AppContainer there was no talking to be done: the probe's
/// connections to `127.0.0.1` and to the machine's own address both timed out
/// from inside one, and its client failed before the network at all, unable to
/// find a configuration directory in a profile the container was refused
/// (ADR-0014, *What the probe answered*). A session runs as a **local account
/// of Verkstead's own** now. An ordinary local account reaches the loopback
/// like anything else, and the profile a Windows session is handed is a real
/// one made under the Data Directory — both halves of it, because a program
/// asking the shell where its settings go is refused for a directory that is
/// not there (see [`crate::sandbox::windows_profile`]). So the arm falls the
/// other way from the container's answer, exactly as ADR-0014 said it would.
///
/// Kept as a function rather than dissolved into the `true` it now is, because
/// what it says is not *nothing here varies by platform*: it is where a
/// platform that cannot reach a compile server would be said, and it is what
/// every caller downstream is written against — the `PATH` walk below, the
/// `RUSTC_WRAPPER` in a session's environment, and the workbench's own answer
/// about whether compiles are cached.
///
/// **Read before the server's own `PATH` is walked**, which is what a platform
/// answering `false` here would come to: a [`BuildCache`] with no sccache is
/// the right answer to everything downstream already, whatever the machine has
/// installed.
///
/// A function of the platform rather than a `cfg!`, for the reason
/// [`crate::platform::Platform`] is a value: an arm this machine will never run
/// is still an arm its tests call.
pub fn compiles_through_an_sccache(platform: Platform) -> bool {
    match platform {
        Platform::Linux | Platform::MacOs | Platform::Windows => true,
    }
}

/// The build cache this server hands out: where it is, and what it can offer.
///
/// Resolved once at startup, like the Sandbox Configuration, and for the reason
/// that is: a cache directory that cannot be made is a misconfiguration to
/// report at startup rather than a session that fails to start weeks later with
/// nobody watching.
///
/// The switch that turns it off is *not* here. It is in `config.yaml` and is
/// read at every session spawn — see [`BuildCache::shared`] — so flipping it in
/// the workbench applies to the next session without a restart, which is what
/// every other setting does.
#[derive(Debug, Clone, Default)]
pub struct BuildCache {
    /// Where it is on the host, and the same path inside every sandbox.
    ///
    /// `None` is a server with no build cache to give, which the served router
    /// never is: [`BuildCache::resolve`] always produces a directory or refuses
    /// to start. It is what a router stood up for a test about something else
    /// carries — see [`BuildCache::none`].
    dir: Option<PathBuf>,

    /// The sccache binary this server found on its own `PATH`, or `None` where
    /// there is none — see [`BuildCache::resolve`] for why that is a smaller
    /// cache rather than a failure.
    sccache: Option<PathBuf>,

    /// The Data Directory, which the compile server's sandbox is read off two
    /// ways — see [`BuildCache::compiling`].
    ///
    /// The Worktrees directory under it is the whole of what that sandbox is
    /// *shown* of it: every Conversation's checkout is under that one directory
    /// and nothing else Verkstead keeps is, so one entry covers every
    /// Conversation there will ever be while leaving the database and the
    /// settings files outside. And where the sccache and the compile server's
    /// own HOME are found inside is read off it too, because on a Mac a
    /// directory of Verkstead's own is one under here rather than a name a bind
    /// invents — see [`crate::sandbox::own_directory`].
    data_dir: Option<PathBuf>,

    /// The one sccache server this machine compiles through, once something has
    /// asked for it.
    ///
    /// Behind a handle every clone of this shares, because every clone is the
    /// same machine's: [`BuildCache`] is cloned into the blocking thread each
    /// session is built on, and a server per clone would be a server per
    /// session, which is the thing this whole arrangement exists to stop.
    compiling: Arc<Mutex<Option<Compiling>>>,
}

/// The compile server as it is running: the process, and the size it was
/// started with.
///
/// The size is kept because sccache reads `SCCACHE_CACHE_SIZE` once, when the
/// server starts. The human changing it in the workbench would otherwise be a
/// setting that saves and does nothing — so a size that no longer matches is
/// what makes the next session start the server again.
#[derive(Debug)]
struct Compiling {
    server: Started,
    size: String,

    /// And what holds it to this server's life on the platform whose answer is
    /// something to hold: the Job Object it is in — see
    /// [`crate::sandbox::outliving::held`]. Nothing at all on the two whose
    /// answer is said elsewhere, and held rather than read either way: letting
    /// go of it is what ends the tree.
    _held: outliving::Held,
}

/// The Compile Server as a process this machine really started, whichever of
/// the two ways it was started.
///
/// **Because the standard library cannot start it on every platform.** A
/// rendering that names no account is an ordinary `Command` and hands back a
/// `Child`; a Windows one names the session account, which is
/// `CreateProcessWithLogonW` — see `sandbox::starting::left_running`, and
/// [`Rendering`], whose conversion into a `Command` refuses rather than
/// quietly starting a compile server as the human.
///
/// What [`Compiling`] does with one is the three things anybody does with a
/// child: ask whether it is still up, end it, and wait for it to go.
#[derive(Debug)]
enum Started {
    /// Started by the standard library, which is both of the platforms whose
    /// boundary is a wrapper in front of the process.
    Ordinarily(Child),

    /// And started as the local account of Verkstead's own that is the boundary
    /// on the third — see `sandbox::starting::Running`.
    #[cfg(windows)]
    AsTheAccount(sandbox::starting::Running),
}

impl Started {
    /// Whether it has stopped, without waiting to find out.
    fn stopped(&mut self) -> bool {
        match self {
            Started::Ordinarily(child) => !matches!(child.try_wait(), Ok(None)),

            #[cfg(windows)]
            Started::AsTheAccount(running) => !matches!(running.try_wait(), Ok(None)),
        }
    }

    /// Ended, and waited for: what [`Compiling::drop`] is.
    fn ended(&mut self) {
        match self {
            Started::Ordinarily(child) => {
                let _ = child.kill();
                let _ = child.wait();
            }

            #[cfg(windows)]
            Started::AsTheAccount(running) => {
                let _ = running.kill();
                let _ = running.wait();
            }
        }
    }
}

impl outliving::InAJob for Started {
    fn id(&self) -> u32 {
        match self {
            Started::Ordinarily(child) => outliving::InAJob::id(child),

            #[cfg(windows)]
            Started::AsTheAccount(running) => outliving::InAJob::id(running),
        }
    }

    #[cfg(windows)]
    fn handle(&self) -> std::os::windows::io::RawHandle {
        match self {
            Started::Ordinarily(child) => outliving::InAJob::handle(child),
            Started::AsTheAccount(running) => outliving::InAJob::handle(running),
        }
    }
}

impl Drop for Compiling {
    /// Stopping it is letting go of it.
    ///
    /// What makes the server Verkstead's rather than something left running on
    /// the machine is the platform's own answer to the case that matters — the
    /// server exiting: `--die-with-parent` on Linux, a keeper of Verkstead's
    /// own on a Mac, which has no such flag to be started with, and on Windows
    /// the Job Object above, whose promise is kept by the handle closing
    /// however this process ends (see [`crate::sandbox::outliving`]). This
    /// covers the other case: a size the human changed, where the old server is
    /// replaced while Verkstead carries on, and where nothing else would ever
    /// tell it to go.
    fn drop(&mut self) {
        self.server.ended();
    }
}

impl BuildCache {
    /// The cache at `configured`, or at the XDG cache directory where nothing
    /// was configured — `$XDG_CACHE_HOME/verkstead`, or `~/.cache/verkstead`
    /// where the machine leaves the XDG variable unset, which is the
    /// specification's own fallback and what most machines have.
    ///
    /// That reading is [`crate::platform::cache_dir`]'s rather than this
    /// module's: one place reads the environment for every directory of
    /// Verkstead's own, so a relative `XDG_CACHE_HOME` — or a relative `HOME`
    /// — is ignored here exactly as it is ignored for the Data Directory,
    /// because either would otherwise resolve against whatever directory the
    /// unit happened to start the server in. Nowhere to resolve to is a service
    /// unit that said nothing about either variable, and it refuses startup
    /// rather than picking somewhere: a cache in a directory nobody chose is
    /// one nobody will find to clear.
    ///
    /// The directory is **made** where it is not there, which is the one place
    /// Verkstead makes a directory outside its own Data Directory. Sandbox
    /// Configuration refuses a bind that is missing rather than creating one —
    /// a configured bind that is not there is a typo, and guessing at it would
    /// hand a session an empty directory where the human meant a full one. This
    /// is the other case: the path is Verkstead's own choice on a fresh install,
    /// there is nothing in it for a typo to hide, and a feature that is on by
    /// default cannot ask the human to `mkdir` first. Creation that fails
    /// refuses startup, because a bind of nothing is every session failing to
    /// start.
    ///
    /// The sccache underneath is looked for and not insisted on. Without one,
    /// what is left is still worth having — the downloads are still shared —
    /// so this says so in the log and carries on: a machine without sccache
    /// installed is a slower machine, never a broken one.
    pub fn resolve(configured: Option<&Path>, data_dir: &Path) -> anyhow::Result<BuildCache> {
        let dir = match configured {
            Some(dir) => dir.to_owned(),
            None => crate::platform::cache_dir().ok_or_else(|| {
                anyhow::anyhow!(
                    "there is nowhere to put the shared Rust build cache: {}, so say \
                     where it goes with --build-cache-dir",
                    crate::platform::nothing_says_where_to_cache(crate::platform::Platform::HERE),
                )
            })?,
        };

        std::fs::create_dir_all(&dir).map_err(|error| {
            anyhow::anyhow!(
                "the shared Rust build cache at {} could not be made ({error}): a bind \
                 Verkstead cannot make is every session failing to start",
                dir.display()
            )
        })?;

        let through_one = compiles_through_an_sccache(Platform::HERE);

        // Not looked for at all where a session does not compile through one,
        // rather than found and then ignored: the honest answer is that this
        // server has none — see [`compiles_through_an_sccache`].

        let sccache = through_one.then(|| on_the_path(SCCACHE)).flatten();

        match (&sccache, through_one) {
            (Some(_), _) => {}
            (None, true) => tracing::info!(
                cache = %dir.display(),
                "no sccache on the server's PATH, so compile caching is off: crate \
                 downloads are still shared between sessions, and dependencies are \
                 compiled once per session. Install sccache where the server can see \
                 it to cache the compiling too",
            ),
            (None, false) => tracing::info!(
                cache = %dir.display(),
                "compile caching is off on this platform, so nothing installed here is \
                 looked for. Crate downloads are still shared between sessions",
            ),
        }

        // Made rather than waited for, because the compile server binds it and
        // a bind of nothing will not start: on a fresh install nobody has
        // grilled anything yet, so there are no Worktrees and no directory to
        // hold them. It is inside the Data Directory, which is Verkstead's own
        // to fill.
        let worktrees = crate::worktrees::directory(data_dir);

        std::fs::create_dir_all(&worktrees).map_err(|error| {
            anyhow::anyhow!(
                "the worktrees directory at {} could not be made ({error}): it is what \
                 the shared compile server is given, and a bind of nothing will not start",
                worktrees.display()
            )
        })?;

        Ok(BuildCache {
            dir: Some(dir),
            sccache,
            data_dir: Some(data_dir.to_owned()),
            compiling: Arc::default(),
        })
    }

    /// One at `dir`, with `sccache` where the machine has one, compiling in a
    /// sandbox holding the Worktrees under `data_dir` — which is what a test
    /// builds when the cache is the thing under test.
    ///
    /// The platform's own answer is applied here as it is in
    /// [`BuildCache::resolve`], so a test that hands one in on a machine that
    /// cannot compile through it gets the cache that machine really has. There
    /// is one answer to *does a session compile through an sccache* rather than
    /// one per constructor.
    pub fn at(dir: PathBuf, sccache: Option<PathBuf>, data_dir: PathBuf) -> BuildCache {
        BuildCache {
            dir: Some(dir),
            sccache: sccache.filter(|_| compiles_through_an_sccache(Platform::HERE)),
            data_dir: Some(data_dir),
            compiling: Arc::default(),
        }
    }

    /// And no cache at all: no bind, no variables, every session cold.
    ///
    /// Never what the served router carries. It is for the routers a test
    /// stands up that are about something else entirely, the way
    /// [`crate::sandbox::SandboxConfig::default`] is an empty configuration.
    pub fn none() -> BuildCache {
        BuildCache::default()
    }

    /// Where it is, or `None` for a server that has none.
    pub fn dir(&self) -> Option<&Path> {
        self.dir.as_deref()
    }

    /// Whether compiling is cached as well as downloading — which is whether an
    /// sccache was found.
    ///
    /// What the workbench warns about when a repository is a Cargo workspace
    /// and this is false: the session will build, and it will build every
    /// dependency itself.
    ///
    /// **The same question on all three platforms.** It was false on Windows
    /// whatever was installed for as long as no session there could reach a
    /// compile server; now what it turns on is the machine — see
    /// [`compiles_through_an_sccache`], which is where a platform that could
    /// not would still be said.
    pub fn caches_compiles(&self) -> bool {
        self.dir.is_some() && self.sccache.is_some()
    }

    /// Make sure the one sccache server this machine compiles through is
    /// running, which is what a session about to build Rust needs before it
    /// starts.
    ///
    /// **Verkstead runs it rather than the sessions.** An sccache server is
    /// what executes `rustc`, and every sandbox shares the host's network, so
    /// clients left to start their own all reach for one port and whichever
    /// session lost the race has its compiles run inside the winner's sandbox —
    /// where its Worktree is not bound, and where the build therefore fails
    /// rather than merely missing the cache.
    ///
    /// **In a sandbox of its own**, holding the Worktrees directory, this cache
    /// and the system. That is the whole of what compiling needs: sources are
    /// under `worktrees/`, dependency sources are under `CARGO_HOME` inside the
    /// cache, and toolchains are under `/nix`. It is not a session's own
    /// sandbox — a compile server serves every session, so it cannot be inside
    /// any one of them — but neither is it the host: `rustc` runs proc macros
    /// while it compiles, so a server on the host would be every Rust
    /// dependency running as whoever Verkstead runs as, with the database and
    /// the settings files in reach. Those sit in the Data Directory's root,
    /// outside the one bind this gets.
    ///
    /// **Started here rather than at startup**, and only for a Conversation
    /// whose Repo builds Rust — see [`builds_rust`]. A machine that never
    /// builds Rust never runs one, and the switch and the size are the human's,
    /// read at this moment like everything else a session is built from.
    ///
    /// **And on Windows it is started as the session account**, which is what a
    /// boundary is on that platform: `session_account` is the local account of
    /// Verkstead's own that every session there runs as, and a compile server
    /// started as anybody else would be every Rust dependency's proc macro
    /// running as the human. The two Unixes pass `None` and read it nowhere —
    /// what makes their boundary is a wrapper in the vector.
    ///
    /// Nothing waits on it and nothing fails if it will not start: a session
    /// whose compile server is missing falls back to starting one of its own,
    /// which is what every session did before this existed.
    pub fn compiling(&self, settings: &RustBuildCache, session_account: Option<&Logon>) {
        let (Some(dir), Some(sccache), Some(data_dir)) = (&self.dir, &self.sccache, &self.data_dir)
        else {
            return;
        };

        if !settings.enabled() {
            return;
        }

        let mut running = self.held();

        if let Some(one) = running.as_mut() {
            // Still up and still the size the human asked for is nothing to do.
            // `try_wait` rather than a signal: a server that died is one to
            // start again, and asking is also what reaps it.
            if !one.server.stopped() && one.size == settings.size() {
                return;
            }

            // Dropped, which stops it where it is still up — see [`Compiling`].
            *running = None;
        }

        // Timed because a session waits on it and says nothing while it does:
        // the Compile Server is started before the first session that compiles
        // Rust, so every second here is a second of a start that looks stuck —
        // see [`crate::sandbox::granting::writing`], which is where the seconds
        // have been.
        let began = std::time::Instant::now();

        let started = compile_server(dir, sccache, data_dir, settings.size(), session_account)
            .and_then(|rendering| left_running(&rendering));

        let took = began.elapsed();

        match started {
            Ok(server) => {
                // A keeper beside it, where the sandbox it was started in has
                // nothing to say about outliving anybody — see
                // [`crate::sandbox::outliving`], and [`compile_server`] for the
                // process group of its own this is the other half of. Nothing
                // at all on Linux, where `--die-with-parent` is the whole of
                // it, and nothing on Windows either, where what says it is the
                // Job the [`Compiling`] below is holding.
                outliving::keep(
                    Platform::HERE,
                    outliving::InAJob::id(&server),
                    std::process::id(),
                );

                tracing::info!(
                    cache = %dir.display(),
                    size = settings.size(),
                    ?took,
                    "the shared compile server is up: every session's rustc goes through \
                     this one, in a sandbox holding the worktrees and the cache",
                );

                *running = Some(Compiling {
                    _held: outliving::held(Platform::HERE, &server),
                    server,
                    size: settings.size().to_owned(),
                });
            }
            Err(error) => {
                // Said and carried on. What a session does without one is start
                // a server inside its own sandbox, which builds perfectly well
                // — it is only *concurrent* Rust sessions that this is holding
                // together.
                tracing::warn!(
                    %error,
                    "the shared compile server would not start, so sessions will each \
                     start one of their own and two building Rust at once may fail",
                );
            }
        }
    }

    /// The compile server, locked.
    fn held(&self) -> std::sync::MutexGuard<'_, Option<Compiling>> {
        self.compiling
            .lock()
            .unwrap_or_else(|held| held.into_inner())
    }

    /// What one sandbox is given, or `None` where there is nothing to give:
    /// no cache on this server, or the human has switched it off.
    ///
    /// `settings` is read at every session spawn rather than held from startup,
    /// so a switch flipped in the workbench applies to the next session.
    pub fn shared(&self, settings: &RustBuildCache) -> Option<Shared> {
        if !settings.enabled() {
            return None;
        }

        Some(Shared {
            dir: self.dir.clone()?,
            sccache: self.sccache.clone(),
            size: settings.size().to_owned(),
        })
    }
}

/// What one sandbox is given of the build cache: a directory to bind, an
/// sccache to bind beside the `verkstead` binary where there is one, and the
/// paths the environment is built out of.
///
/// Decided as the sandbox is built rather than held from startup, because half
/// of it is the human's switch and their size — see [`BuildCache::shared`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shared {
    dir: PathBuf,
    sccache: Option<PathBuf>,
    size: String,
}

impl Shared {
    /// The directory bound writable into the sandbox, at the same path inside.
    ///
    /// The whole of it rather than the two halves separately: cargo makes what
    /// it needs under `CARGO_HOME` and sccache makes its own, and a bind per
    /// subdirectory would be two holes where one says the same thing.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// What `CARGO_HOME` is inside: the registry, shared by every session.
    pub fn cargo_home(&self) -> PathBuf {
        self.dir.join(CARGO)
    }

    /// The sccache binary on the host, or `None` where the server found none —
    /// which is the whole of the difference between a cache that shares
    /// downloads and one that shares compiled objects too.
    pub fn sccache(&self) -> Option<&Path> {
        self.sccache.as_deref()
    }

    /// What `SCCACHE_DIR` is, where there is an sccache to read it.
    pub fn sccache_dir(&self) -> PathBuf {
        self.dir.join(SCCACHE_DIR)
    }

    /// And what `SCCACHE_CACHE_SIZE` is: the human's, or [`SIZE`].
    pub fn size(&self) -> &str {
        &self.size
    }
}

/// Whether a Repo at `path` is one a session would build Rust in: a
/// `Cargo.toml` at its root.
///
/// Asked of the checkout rather than remembered against the Repo, because it is
/// a fact about what is on disk now and a repository gains and loses a manifest
/// like any other file. Two things read it — whether to start the compile
/// server for a Conversation, and whether to warn on the setup card that its
/// compiles will not be cached — and they have to agree, so they ask the same
/// question here.
pub fn builds_rust(repo: &Path) -> bool {
    repo.join("Cargo.toml").is_file()
}

/// The compile server as a command: one sccache, in the foreground, in a
/// sandbox that holds the Worktrees, the cache and the system.
///
/// **In the foreground on purpose.** `sccache --start-server` daemonises and
/// returns, which would leave Verkstead with nothing to hold — no way to know
/// whether it is still up, and no pid for either platform's answer to outliving
/// the server to be about. `SCCACHE_START_SERVER` with `SCCACHE_NO_DAEMON` is the server
/// *as* the process, so it is a child like any other: it dies when Verkstead
/// does, and [`BuildCache::compiling`] can ask whether it is alive.
///
/// `SCCACHE_IDLE_TIMEOUT` is nothing, because the default is ten minutes and
/// this one is meant to be there whenever a session wants it — an unattended
/// Conversation may go a long time between builds.
///
/// Not a [`crate::sandbox::Sandbox`], deliberately. That type is a
/// Conversation's — a Worktree, a Profile, a handoff directory, the
/// credentials a session commits with — and none of it applies to a process
/// that serves every Conversation and belongs to none. **It is a
/// [`crate::sandbox::Surface`] all the same**, and rendered by the renderer a
/// session's is: what a sandbox holds is one description on every platform, so
/// this is bubblewrap's flags on Linux and a deny-by-default policy on a Mac
/// without a word here saying which — see [`crate::sandbox::rendered`].
///
/// **And Windows is among them, which is the third rendering and is not a
/// wrapper at all**: what a description comes to there is an access-control
/// entry on each real path it names, written for the local account of
/// Verkstead's own that the process is then started as — see
/// [`crate::sandbox::granting`]. So this writes a boundary as well as
/// describing one, the way [`crate::sandbox::Sandbox::command`] does and for
/// the same reason: the entries have to be on the machine before the process
/// that runs behind them is started.
///
/// **The boundary is the Compile Server's own rather than any Conversation's.**
/// It is held for as long as the server runs and it is written under a record
/// of its own — see `sandbox::entries::Entries::of_the_compile_server` —
/// because the thing it belongs to is this process rather than a piece of work:
/// a compile server serves every Conversation and outlives each of them.
/// Most of what it names is granted by every session's own description anyway
/// — the cache, the sccache, the toolchain on the `PATH` — and an entry already
/// there is left alone, so what this really adds is its own profile and the
/// Worktrees directory whole.
///
/// What it gave up to be a [`crate::sandbox::Surface`] is the hostname it used
/// to be given inside. A name for the machine is something one of the
/// mechanisms can say and the others cannot, so it is no part of a description
/// any of them answers — and what it was worth was telling this sandbox apart
/// from a session's in a process listing.
fn compile_server(
    dir: &Path,
    sccache: &Path,
    data_dir: &Path,
    size: &str,
    session_account: Option<&Logon>,
) -> std::io::Result<Rendering> {
    let worktrees = crate::worktrees::directory(data_dir);
    let home = compiling_home(data_dir);

    // The directory of Verkstead's own inside, which is what leads the `PATH`
    // here as it leads a session's — and where the sccache below is found on
    // the platforms that join one in. See [`crate::sandbox::sccache_inside`],
    // which is the one place that difference is written.
    let ours = sandbox::own_bin(Platform::HERE, data_dir);
    let inside = sandbox::sccache_inside(Platform::HERE, &ours, sccache);

    // Started in its own HOME, which is a directory of Verkstead's own holding
    // nothing: a compile server has no checkout of its own to stand in, and
    // every path it is handed is absolute.
    // What this searches for a program in, said once: the same value the
    // environment below is set to, and the same one the grants beneath it are
    // read out of — see [`crate::sandbox::path`].
    let searches = sandbox::path(Platform::HERE, &ours);

    let mut surface = sandbox::on_the_machine(Platform::HERE, home.clone());

    surface.made(Access::Empty(home.clone()));

    // And the rest of the profile on the platform that has one, after the HOME
    // that would otherwise empty them: a Windows program keeps its settings
    // under `%APPDATA%` rather than in a dotfile, and asks the shell where that
    // is — which refuses to answer at all for a directory that is not really
    // there. Which is a thing an sccache does at its first instruction, and the
    // second half of why compile caching was ever off on that platform: the
    // probe's client failed before it reached the network, unable to find a
    // configuration directory in a profile it was refused (ADR-0014). See
    // [`crate::sandbox::windows_profile`], and the names beside it below.
    if Platform::HERE == Platform::Windows {
        for made in sandbox::windows_profile(&home) {
            surface.made(made);
        }
    }

    // And everything that `PATH` names which this has to be granted as well as
    // told about — see [`crate::sandbox::reaching`], which is the same rule a
    // session's own description goes through. That list leads with the `PATH`
    // the server was started with, so without this a compile server would be
    // told to look in directories of the human's own that are not inside it.
    //
    // After the empty HOME for that description's reason: on a machine whose
    // Data Directory is under the server's home, a bind said before it is one
    // the directory made over it takes away again.
    if let Some(servers_home) = sandbox::servers_home() {
        sandbox::reaching(Platform::HERE, &searches, servers_home, &mut surface);
    }

    // And where the toolchain it is about to run is, on a machine whose Rust is
    // rustup's — see [`crate::sandbox::toolchains`]. **This is the process that
    // really needs it**: an sccache client hands the server a command line, and
    // what is on the front of that line is the human's `rustc`, which is a shim
    // that has to find its own install. Granted a line above as part of what
    // the `PATH` implies, and said here because a compile server has a profile
    // of its own exactly as a session does.
    let toolchains = sandbox::servers_home()
        .and_then(|servers_home| sandbox::toolchains(Platform::HERE, &searches, servers_home));

    surface
        // Every Conversation's checkout, writable: a compile writes its output
        // into the Worktree's own `target/`. One entry rather than one per
        // Conversation, because a Worktree made after this started would
        // otherwise be one this cannot see.
        .own(&worktrees, Reach::ReadWrite)
        // Both of these stand for the installation rather than for this
        // process — see [`sandbox::Surface::standing`]. The Worktrees directory
        // and the cache are Verkstead's own, granted to Verkstead's own
        // account, and a compile server that came and went taking them apart
        // and putting them back is minutes of propagation for a grant that
        // never differs.
        .standing(&worktrees)
        .standing(dir)
        // And the cache, which holds both what it reads — the dependency
        // sources under `CARGO_HOME` — and what it writes.
        .own(dir, Reach::ReadWrite)
        .elsewhere(sccache, &inside, Reach::ReadOnly);

    surface
        .set("HOME", &home)
        // The same `PATH` a session gets, off the same directory: the sccache
        // this runs is beside where a session's `verkstead` goes, and what is
        // in front of the machine's own paths is that directory either way —
        // see [`crate::sandbox::path`]. The value the grants above were read
        // out of, so what this is told to search is what it can open.
        .set("PATH", &searches)
        .set("SCCACHE_DIR", dir.join(SCCACHE_DIR))
        .set("SCCACHE_CACHE_SIZE", size)
        .set("SCCACHE_START_SERVER", "1")
        .set("SCCACHE_NO_DAEMON", "1")
        .set("SCCACHE_IDLE_TIMEOUT", "0")
        .running(&[&inside]);

    if let Some(rustup) = &toolchains {
        surface.set(sandbox::RUSTUP_HOME, rustup);
    }

    // And the names nothing on Windows runs without, which the two Unixes have
    // no equivalent of: `USERPROFILE` and the two halves of the profile made
    // above, and somewhere to write what it throws away. The whole point of
    // saying them is that they point at the profile this description just made
    // rather than at whatever the account the logon loaded happens to have
    // under `C:\Users` — see [`crate::sandbox::windows_names`].
    if Platform::HERE == Platform::Windows {
        for (name, value) in sandbox::windows_names(&home) {
            surface.set(name, value);
        }
    }

    // And nothing to close after it, which is the one caller of a rendering
    // that has none: what a closing sees to is a file a session replaced rather
    // than wrote in place — see [`crate::sandbox::Closing`] — and the one file
    // this joins in is the sccache it is running, read-only. A compile server
    // outlives every session anyway, so there is no ending here to hang one on.
    #[allow(unused_mut)]
    let (mut rendering, _) = sandbox::rendered(Platform::HERE, &surface);

    // And on the platform whose rendering is a description and nothing else,
    // the boundary itself: the entries that make it true and the identity they
    // are written for — see [`crate::sandbox::Sandbox::command`], which does
    // the same three things in the same order for a session.
    //
    // **Which is where this can refuse.** On the two platforms with a wrapper
    // there is nothing here to refuse; on the third, an account this machine
    // has not got is a compile server that cannot be started at all, and it is
    // carried the way every other failure to start one is — said in the log,
    // with each session starting a server of its own.
    #[cfg(windows)]
    {
        let entries = sandbox::granting::entries(&surface, sandbox::servers_home());

        // Read before a word of it is written, for the reason a session's is:
        // a refusal cuts the inheritance on the path it refuses, so this is the
        // last moment at which the answer is the machine's own. The Compile
        // Server refuses nothing — it has no agent account to cover — so what
        // this comes back with is empty, and it is asked all the same rather
        // than assumed.
        let cut = sandbox::granting::writing::inheriting(&entries);

        let account = the_session_account(session_account)?;

        // Under a record of its own rather than a Conversation's: this boundary
        // belongs to the process rather than to a piece of work, and it is held
        // for as long as the server is up. The hold goes in the map that keeps
        // one boundary per name, so a compile server started again for a size
        // the human changed finds the entries it already has rather than
        // writing them afresh.
        let held = sandbox::entries::Entries::of_the_compile_server(
            data_dir,
            account.name(),
            account.sid().text(),
        )?;

        let standing = sandbox::granting::standing_of(&surface, sandbox::servers_home());

        held.wrote(
            sandbox::granting::written_down(&entries, &standing),
            cut.clone(),
        )?;

        // And the machine's own half — see this call in `sandbox::command`,
        // which is the same two records written for the same reason.
        sandbox::granting::remembering::standing_wrote(
            data_dir,
            account.name(),
            account.sid().text(),
            &sandbox::granting::standing_among(&entries, &standing),
        )?;

        sandbox::granting::writing::write(&entries, account.sid().text(), &cut)?;

        rendering.as_account(sandbox::account::Logon::of(
            account.name(),
            account.password(),
        ));
    }

    // Read by the block above and by nothing else, which the two platforms it
    // is not compiled on have to be told.
    let _ = session_account;

    Ok(rendering)
}

/// The account this machine really has, out of the name and password a caller
/// worked out — or a refusal saying which half of it is not there.
///
/// [`crate::sandbox::Sandbox::session_account`]'s question, asked for the
/// process that is nobody's session: the name and the password come off a Data
/// Directory and a settings file, and the SID every entry is written for is the
/// machine's own answer about that name.
#[cfg(windows)]
fn the_session_account(
    logon: Option<&Logon>,
) -> std::io::Result<sandbox::account::machine::Account> {
    let logon = logon.ok_or_else(|| {
        std::io::Error::other(
            "the shared compile server runs as this installation's own local account and \
             nothing said which account that is",
        )
    })?;

    sandbox::account::machine::Account::resolving(logon).map_err(|missing| {
        std::io::Error::other(format!(
            "the shared compile server runs as this installation's own local account and \
             there is not one: {missing}",
        ))
    })
}

/// `rendering` started, and left running for as long as this server is.
///
/// The one place the Compile Server crosses from a description into a process,
/// and it is two calls rather than one because the platforms start it two ways
/// — see [`Started`]. A rendering naming no account is the standard library's
/// own spawn, with the stdio and the process group a compile server wants said
/// here; one naming an account is `sandbox::starting::left_running`, which
/// says both for itself.
fn left_running(rendering: &Rendering) -> std::io::Result<Started> {
    #[cfg(windows)]
    if let Some(logon) = rendering.account() {
        return sandbox::starting::left_running(rendering, logon).map(Started::AsTheAccount);
    }

    let mut compiling = Command::try_from(rendering)?;

    // In a process group of its own where the platform needs one, which is what
    // a keeper ends when the server has gone — see
    // [`crate::sandbox::outliving`]. A session's sandbox has one already,
    // because it runs on a terminal; this runs on none, so it says so here.
    outliving::in_its_own_group(Platform::HERE, &mut compiling);

    // Nothing to read and nothing to say: what it prints in the ordinary case
    // is nothing at all. Its errors are left to the server's own, which is
    // where somebody would go looking for why compiling stopped being cached.
    compiling
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit());

    compiling.spawn().map(Started::Ordinarily)
}

/// Where `program` is on the server's own `PATH`, or `None` where it is on none
/// of it.
///
/// The server's environment rather than the sandbox's fixed `PATH`: what is
/// bound into a sandbox has to be a file on the host, and the packaged unit
/// puts sccache on the service's path precisely so that this finds it.
///
/// **Read on the two platforms that compile through one.** Nothing calls this
/// on Windows any more — see [`compiles_through_an_sccache`] — and the arm it
/// resolves under is kept for the reason every platform arm here is: it is what
/// a name means on that machine, rather than a claim that anything asks.
///
/// The lookup itself is [`crate::sandbox::on_the_path`], which is where the
/// platform's own rules for reading a name are, and which the onboarding probes
/// ask the same question of against the `PATH` a *session* is given. What is
/// left here is which `PATH` this caller means, which is the whole of the
/// difference between the two.
fn on_the_path(program: &str) -> Option<PathBuf> {
    crate::sandbox::on_the_path(
        Platform::HERE,
        program,
        std::env::var_os("PATH").as_deref(),
        std::env::var_os("PATHEXT").as_deref(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The switch is the human's and is read every time, so a cache that exists
    /// still hands out nothing while it is off.
    #[test]
    fn a_cache_that_is_switched_off_gives_a_sandbox_nothing() {
        let cache = BuildCache::at(
            PathBuf::from("/var/cache/verkstead"),
            None,
            PathBuf::from("/var/lib/verkstead"),
        );

        assert!(cache.shared(&RustBuildCache::of(false, None)).is_none());
    }

    /// And a server with no cache at all hands out nothing whatever the
    /// settings say.
    #[test]
    fn a_server_without_one_gives_a_sandbox_nothing_either() {
        assert!(
            BuildCache::none()
                .shared(&RustBuildCache::default())
                .is_none()
        );
        assert!(!BuildCache::none().caches_compiles());
    }

    /// The two halves are under the one directory, so one bind reaches both.
    #[test]
    fn the_two_halves_are_named_inside_the_one_directory() {
        let cache = BuildCache::at(
            PathBuf::from("/var/cache/verkstead"),
            Some(PathBuf::from("/nix/store/whatever/bin/sccache")),
            PathBuf::from("/var/lib/verkstead"),
        );
        let shared = cache
            .shared(&RustBuildCache::default())
            .expect("nothing configured is the feature on");

        assert_eq!(shared.dir(), Path::new("/var/cache/verkstead"));
        assert_eq!(
            shared.cargo_home(),
            Path::new("/var/cache/verkstead/cargo"),
            "the registry every session downloads into"
        );
        assert_eq!(
            shared.sccache_dir(),
            Path::new("/var/cache/verkstead/sccache"),
            "and the compiled objects beside it"
        );
        assert_eq!(shared.size(), SIZE, "the default where nobody has said");
    }

    /// An sccache the server never found is a cache that still shares the
    /// downloads — see [`BuildCache::resolve`].
    #[test]
    fn without_an_sccache_the_downloads_are_still_shared() {
        let cache = BuildCache::at(
            PathBuf::from("/var/cache/verkstead"),
            None,
            PathBuf::from("/var/lib/verkstead"),
        );
        let shared = cache.shared(&RustBuildCache::default()).unwrap();

        assert!(!cache.caches_compiles());
        assert_eq!(shared.sccache(), None);
        assert_eq!(shared.cargo_home(), Path::new("/var/cache/verkstead/cargo"));
    }

    /// The directory is made rather than insisted on, which is what lets the
    /// feature be on with nothing configured on a fresh install.
    #[test]
    fn a_cache_directory_that_is_not_there_yet_is_made() {
        let dir = tempfile::tempdir().unwrap();
        let cache = dir.path().join("never-made/verkstead");

        let resolved =
            BuildCache::resolve(Some(&cache), dir.path()).expect("it is made rather than refused");

        assert_eq!(resolved.dir(), Some(cache.as_path()));
        assert!(cache.is_dir());
        assert!(
            dir.path().join("worktrees").is_dir(),
            "and the worktrees directory with it, which the compile server binds"
        );
    }

    /// And one that cannot be made refuses startup, because a bind of nothing
    /// is every session failing to start.
    #[test]
    fn a_cache_directory_that_cannot_be_made_refuses_to_resolve() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a-file");
        std::fs::write(&file, "not a directory\n").unwrap();

        assert!(BuildCache::resolve(Some(&file.join("cache")), dir.path()).is_err());
    }

    /// The compile server is only ever the human's to have: switched off, there
    /// is nothing to serve and nothing is started.
    #[test]
    fn a_cache_switched_off_starts_no_compile_server() {
        let cache = BuildCache::at(
            PathBuf::from("/var/cache/verkstead"),
            Some(PathBuf::from("/nix/store/whatever/bin/sccache")),
            PathBuf::from("/var/lib/verkstead"),
        );

        cache.compiling(&RustBuildCache::of(false, None), None);

        assert!(cache.held().is_none());
    }

    /// And without an sccache there is nothing to start one *of* — which is the
    /// same machine the setup card warns on, still building and still sharing
    /// its downloads.
    #[test]
    fn without_an_sccache_there_is_no_compile_server_either() {
        let cache = BuildCache::at(
            PathBuf::from("/var/cache/verkstead"),
            None,
            PathBuf::from("/var/lib/verkstead"),
        );

        cache.compiling(&RustBuildCache::default(), None);

        assert!(cache.held().is_none());
    }

    /// All three platforms compile through an sccache, which is the one place
    /// that is decided — see [`compiles_through_an_sccache`].
    ///
    /// The third is the one worth asserting: it answered `false` for as long as
    /// a session there ran inside an AppContainer, and what changed is the
    /// boundary rather than anything about sccache.
    #[test]
    fn every_platform_compiles_through_an_sccache() {
        assert!(compiles_through_an_sccache(Platform::Linux));
        assert!(compiles_through_an_sccache(Platform::MacOs));
        assert!(
            compiles_through_an_sccache(Platform::Windows),
            "a session there runs as a local account of Verkstead's own, which \
             reaches the loopback its sccache client talks to the Compile \
             Server over",
        );
    }

    /// And what a cache hands out follows that answer rather than what it was
    /// handed: an sccache given to a machine that cannot compile through one is
    /// an sccache the cache does not have.
    ///
    /// Asked of [`Platform::HERE`] rather than of a platform, because this is
    /// the arm the constructors take — so what it asserts is whatever the
    /// machine running the suite really answers, which is the whole point of
    /// running it on all three.
    #[test]
    fn an_sccache_is_only_kept_where_a_session_could_reach_one() {
        let cache = BuildCache::at(
            PathBuf::from("/var/cache/verkstead"),
            Some(PathBuf::from("/nix/store/whatever/bin/sccache")),
            PathBuf::from("/var/lib/verkstead"),
        );
        let here = compiles_through_an_sccache(Platform::HERE);

        assert_eq!(cache.caches_compiles(), here);
        assert_eq!(
            cache
                .shared(&RustBuildCache::default())
                .expect("nothing configured is the feature on")
                .sccache()
                .is_some(),
            here,
            "which is what puts a RUSTC_WRAPPER in a session's environment, or \
             leaves it out",
        );
    }

    /// And on the platform whose boundary is an identity, a compile server with
    /// no account to be started as is refused rather than started as the human.
    ///
    /// The one thing about the Windows arm that can be asked without an account
    /// on the machine, and it is the thing that matters: what a refusal here
    /// costs is a session starting an sccache server of its own, and what
    /// starting it anyway would cost is every Rust dependency's proc macro
    /// running as whoever Verkstead runs as.
    #[test]
    #[cfg(windows)]
    fn a_compile_server_with_no_account_to_be_started_as_is_refused() {
        let dir = tempfile::tempdir().unwrap();

        let refused = compile_server(
            dir.path(),
            Path::new(r"C:\sccache\sccache.exe"),
            dir.path(),
            SIZE,
            None,
        )
        .expect_err("a Windows compile server names the account it runs as");

        assert!(
            refused.to_string().contains("local account"),
            "the refusal says which half of it is not there: {refused}",
        );
    }

    /// A Repo builds Rust where it has a manifest at its root, which is the one
    /// question both the compile server and the setup card's warning turn on.
    #[test]
    fn a_repo_builds_rust_where_it_has_a_manifest_at_its_root() {
        let dir = tempfile::tempdir().unwrap();

        assert!(
            !builds_rust(dir.path()),
            "an empty directory builds nothing"
        );

        std::fs::create_dir(dir.path().join("crates")).unwrap();
        std::fs::write(dir.path().join("crates/Cargo.toml"), "[package]\n").unwrap();

        assert!(
            !builds_rust(dir.path()),
            "a manifest somewhere underneath is not the root's"
        );

        std::fs::write(dir.path().join("Cargo.toml"), "[workspace]\n").unwrap();

        assert!(builds_rust(dir.path()));
    }
}
