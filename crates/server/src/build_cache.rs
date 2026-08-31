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
//! **Rust by name**, deliberately. Nothing here generalises over languages: a
//! node or a python cache would want its own directory, its own variables and
//! its own switch, and a sibling of this module is where one would go. Naming
//! this one for what it caches is what leaves room for that.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};

use crate::sandbox;
use crate::settings::RustBuildCache;

/// What is looked for on the server's own `PATH`, and the name it is mounted
/// under inside a sandbox — see [`crate::sandbox::SCCACHE_INSIDE`].
const SCCACHE: &str = "sccache";

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

/// The `HOME` the compile server is given, which is a directory made by the
/// bind and holding nothing.
///
/// It has one because sccache looks for a config file under it, and a process
/// with no `HOME` at all looks for one under `/`. Beside `/verkstead/bin`, for
/// the reason that is where it is: a directory of Verkstead's own rather than a
/// name inside one of the host's.
const COMPILING_HOME: &str = "/verkstead/home";

/// What the compile server's sandbox is called from inside it, which is the
/// only thing that tells it apart from a session's in a process listing.
const COMPILING_HOST: &str = "verkstead-compiling";

/// The build cache this server hands out: where it is, and what it can offer.
///
/// Resolved once at startup, like the Watched Paths and the Sandbox
/// Configuration, and for the reason those are: a cache directory that cannot
/// be made is a misconfiguration to report at startup rather than a session
/// that fails to start weeks later with nobody watching.
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

    /// The Worktrees directory, which is the whole of what the compile server
    /// is shown of the Data Directory — see [`BuildCache::compiling`].
    ///
    /// Every Conversation's checkout is under it and nothing else Verkstead
    /// keeps is, so one bind covers every Conversation there will ever be while
    /// leaving the database and the settings files outside.
    worktrees: Option<PathBuf>,

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
    server: Child,
    size: String,
}

impl Drop for Compiling {
    /// Stopping it is letting go of it.
    ///
    /// `--die-with-parent` is what makes the server Verkstead's rather than
    /// something left running on the machine, and it covers the case that
    /// matters — the server exiting. This covers the other one: a size the
    /// human changed, where the old server is replaced while Verkstead carries
    /// on, and where nothing else would ever tell it to go.
    fn drop(&mut self) {
        let _ = self.server.kill();
        let _ = self.server.wait();
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
                    "there is nowhere to put the shared Rust build cache: neither \
                     XDG_CACHE_HOME nor HOME is set to an absolute path, so say where \
                     it goes with --build-cache-dir"
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

        let sccache = on_the_path(SCCACHE);

        if sccache.is_none() {
            tracing::info!(
                cache = %dir.display(),
                "no sccache on the server's PATH, so compile caching is off: crate \
                 downloads are still shared between sessions, and dependencies are \
                 compiled once per session. Install sccache where the server can see \
                 it to cache the compiling too",
            );
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
            worktrees: Some(worktrees),
            compiling: Arc::default(),
        })
    }

    /// One at `dir`, with `sccache` where the machine has one, compiling in a
    /// sandbox holding `worktrees` — which is what a test builds when the cache
    /// is the thing under test.
    pub fn at(dir: PathBuf, sccache: Option<PathBuf>, worktrees: PathBuf) -> BuildCache {
        BuildCache {
            dir: Some(dir),
            sccache,
            worktrees: Some(worktrees),
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
    /// Nothing waits on it and nothing fails if it will not start: a session
    /// whose compile server is missing falls back to starting one of its own,
    /// which is what every session did before this existed.
    pub fn compiling(&self, settings: &RustBuildCache) {
        let (Some(dir), Some(sccache), Some(worktrees)) =
            (&self.dir, &self.sccache, &self.worktrees)
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
            let stopped = !matches!(one.server.try_wait(), Ok(None));

            if !stopped && one.size == settings.size() {
                return;
            }

            // Dropped, which stops it where it is still up — see [`Compiling`].
            *running = None;
        }

        match compile_server(dir, sccache, worktrees, settings.size()).spawn() {
            Ok(server) => {
                tracing::info!(
                    cache = %dir.display(),
                    size = settings.size(),
                    "the shared compile server is up: every session's rustc goes through \
                     this one, in a sandbox holding the worktrees and the cache",
                );

                *running = Some(Compiling {
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
/// whether it is still up, and nothing for `--die-with-parent` to hang off.
/// `SCCACHE_START_SERVER` with `SCCACHE_NO_DAEMON` is the server *as* the
/// process, so it is a child like any other: it dies when Verkstead does, and
/// [`BuildCache::compiling`] can ask whether it is alive.
///
/// `SCCACHE_IDLE_TIMEOUT` is nothing, because the default is ten minutes and
/// this one is meant to be there whenever a session wants it — an unattended
/// Conversation may go a long time between builds.
///
/// Not a [`crate::sandbox::Sandbox`], deliberately. That type is a
/// Conversation's — a Worktree, a Profile, a handoff directory, the credentials
/// a session commits with — and none of it applies to a process that serves
/// every Conversation and belongs to none. What the two do share is the system
/// binds and the `PATH`, which are facts about the machine, so they are read
/// from the one place that states them.
fn compile_server(dir: &Path, sccache: &Path, worktrees: &Path, size: &str) -> Command {
    let mut bwrap = Command::new("bwrap");

    // Nothing of the server's environment, for the reason a session gets none:
    // what is inside is what is said here.
    bwrap.env_clear();

    bwrap.args([
        "--die-with-parent",
        "--unshare-all",
        "--share-net",
        "--hostname",
        COMPILING_HOST,
    ]);

    for path in sandbox::SYSTEM.iter().map(Path::new).filter(|p| p.exists()) {
        bwrap.arg("--ro-bind").arg(path).arg(path);
    }

    bwrap.args(["--proc", "/proc", "--dev", "/dev", "--tmpfs", "/tmp"]);
    bwrap.arg("--dir").arg(COMPILING_HOME);

    bwrap
        // Every Conversation's checkout, writable: a compile writes its output
        // into the Worktree's own `target/`. One bind rather than one per
        // Conversation, because a Worktree made after this started would
        // otherwise be one this cannot see.
        .arg("--bind")
        .arg(worktrees)
        .arg(worktrees)
        // And the cache, which holds both what it reads — the dependency
        // sources under `CARGO_HOME` — and what it writes.
        .arg("--bind")
        .arg(dir)
        .arg(dir)
        .arg("--ro-bind")
        .arg(sccache)
        .arg(sandbox::SCCACHE_INSIDE);

    bwrap
        .arg("--setenv")
        .arg("HOME")
        .arg(COMPILING_HOME)
        .arg("--setenv")
        .arg("PATH")
        .arg(sandbox::PATH)
        .arg("--setenv")
        .arg("SCCACHE_DIR")
        .arg(dir.join(SCCACHE_DIR))
        .arg("--setenv")
        .arg("SCCACHE_CACHE_SIZE")
        .arg(size)
        .arg("--setenv")
        .arg("SCCACHE_START_SERVER")
        .arg("1")
        .arg("--setenv")
        .arg("SCCACHE_NO_DAEMON")
        .arg("1")
        .arg("--setenv")
        .arg("SCCACHE_IDLE_TIMEOUT")
        .arg("0")
        .arg(sandbox::SCCACHE_INSIDE);

    // Nothing to read and nothing to say: what it prints in the ordinary case
    // is nothing at all. Its errors are left to the server's own, which is
    // where somebody would go looking for why compiling stopped being cached.
    bwrap
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit());

    bwrap
}

/// Where `program` is on the server's own `PATH`, or `None` where it is on none
/// of it.
///
/// The server's environment rather than the sandbox's fixed `PATH`: what is
/// bound into a sandbox has to be a file on the host, and the packaged unit
/// puts sccache on the service's path precisely so that this finds it.
fn on_the_path(program: &str) -> Option<PathBuf> {
    std::env::split_paths(&std::env::var_os("PATH")?)
        // A `PATH` entry that is empty means the working directory, which is
        // not somewhere to go looking for a compiler wrapper.
        .filter(|dir| !dir.as_os_str().is_empty())
        .map(|dir| dir.join(program))
        .find(|path| path.is_file())
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
            PathBuf::from("/var/lib/verkstead/worktrees"),
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
            PathBuf::from("/var/lib/verkstead/worktrees"),
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
            PathBuf::from("/var/lib/verkstead/worktrees"),
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
            PathBuf::from("/var/lib/verkstead/worktrees"),
        );

        cache.compiling(&RustBuildCache::of(false, None));

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
            PathBuf::from("/var/lib/verkstead/worktrees"),
        );

        cache.compiling(&RustBuildCache::default());

        assert!(cache.held().is_none());
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
