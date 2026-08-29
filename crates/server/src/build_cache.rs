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
//! **Rust by name**, deliberately. Nothing here generalises over languages: a
//! node or a python cache would want its own directory, its own variables and
//! its own switch, and a sibling of this module is where one would go. Naming
//! this one for what it caches is what leaves room for that.

use std::path::{Path, PathBuf};

use crate::settings::RustBuildCache;

/// What is looked for on the server's own `PATH`, and the name it is mounted
/// under inside a sandbox — see [`crate::sandbox::SCCACHE_INSIDE`].
const SCCACHE: &str = "sccache";

/// The directory name Verkstead's own things go under inside the XDG cache
/// directory, which is where the build cache is when nobody has said otherwise.
const OURS: &str = "verkstead";

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
}

impl BuildCache {
    /// The cache at `configured`, or at the XDG cache directory where nothing
    /// was configured — see [`default_dir`].
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
    pub fn resolve(configured: Option<&Path>) -> anyhow::Result<BuildCache> {
        let dir = match configured {
            Some(dir) => dir.to_owned(),
            None => default_dir().ok_or_else(|| {
                anyhow::anyhow!(
                    "there is nowhere to put the shared Rust build cache: neither \
                     XDG_CACHE_HOME nor HOME is set, so say where it goes with \
                     --build-cache-dir"
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

        Ok(BuildCache {
            dir: Some(dir),
            sccache,
        })
    }

    /// One at `dir`, with `sccache` where the machine has one — which is what a
    /// test builds when the cache is the thing under test.
    pub fn at(dir: PathBuf, sccache: Option<PathBuf>) -> BuildCache {
        BuildCache {
            dir: Some(dir),
            sccache,
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

/// Where the build cache goes when nobody has said: `$XDG_CACHE_HOME/verkstead`,
/// or `~/.cache/verkstead` where the machine leaves the XDG variable unset —
/// which is the specification's own fallback and what most machines have.
///
/// `None` where neither variable is set and there is therefore no home to put a
/// cache under, which is a service unit that said nothing about either. The
/// server refuses to start on it rather than picking somewhere: a cache in a
/// directory nobody chose is one nobody will find to clear.
///
/// A relative `XDG_CACHE_HOME` is ignored rather than resolved, as the
/// specification says: it would otherwise resolve against whatever directory
/// the unit happened to start the server in.
fn default_dir() -> Option<PathBuf> {
    let xdg = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .filter(|dir| dir.is_absolute());

    let cache = match xdg {
        Some(dir) => dir,
        None => PathBuf::from(std::env::var_os("HOME")?).join(".cache"),
    };

    Some(cache.join(OURS))
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
        let cache = BuildCache::at(PathBuf::from("/var/cache/verkstead"), None);

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
        let cache = BuildCache::at(PathBuf::from("/var/cache/verkstead"), None);
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

        let resolved = BuildCache::resolve(Some(&cache)).expect("it is made rather than refused");

        assert_eq!(resolved.dir(), Some(cache.as_path()));
        assert!(cache.is_dir());
    }

    /// And one that cannot be made refuses startup, because a bind of nothing
    /// is every session failing to start.
    #[test]
    fn a_cache_directory_that_cannot_be_made_refuses_to_resolve() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a-file");
        std::fs::write(&file, "not a directory\n").unwrap();

        assert!(BuildCache::resolve(Some(&file.join("cache"))).is_err());
    }
}
