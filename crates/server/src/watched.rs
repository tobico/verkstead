//! The Watched Paths: the directories Verkstead is permitted to operate inside.
//!
//! A security boundary rather than a convenience. They are configured in the
//! environment at installation and never discovered by scanning, and the
//! decision about whether a path is inside one is taken here — on the server,
//! below every route — so that no request can reach around it by asking
//! differently.
//!
//! Everything is decided on the *resolved* path: `..` taken out, every symlink
//! followed, as the filesystem itself would. A path that merely reads as inside
//! a Watched Path is not inside it, and reading the text of a path is exactly
//! how a boundary gets walked through.
//!
//! Watching nothing is a legal state and a closed one: with no Watched Path
//! configured every path is outside, so nothing is admitted. The server refuses
//! to start that way — see [`crate::Config`] — but the type fails closed
//! regardless, because a boundary whose empty case admits everything is a
//! boundary that opens itself the moment configuration goes missing.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

/// The directories Verkstead may operate inside, resolved once at startup.
#[derive(Debug, Clone, Default)]
pub struct WatchedPaths(Vec<PathBuf>);

/// What the boundary made of a path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Admission {
    /// Inside a Watched Path. Carries the resolved path, which is the one to
    /// work with from here on: it names one directory, where the path as it was
    /// written may name several.
    Inside(PathBuf),

    /// Relative. Nothing here resolves one — the directory the server happens to
    /// be running in is not something a path should mean.
    NotAbsolute,

    /// Nothing is there to resolve.
    Missing,

    /// It resolves to somewhere no Watched Path covers.
    Outside,
}

impl WatchedPaths {
    /// The Watched Paths as configured, resolved and checked.
    ///
    /// Resolving here rather than per request is what makes the check cheap and
    /// what makes a misconfiguration loud: a Watched Path that does not exist is
    /// refused at startup, where it can be fixed, rather than silently covering
    /// nothing and refusing every repo inside it.
    pub fn resolve(paths: &[PathBuf]) -> Result<Self> {
        if paths.is_empty() {
            bail!(
                "no watched path is configured: Verkstead operates only inside the \
                 directories it is given, so it has nothing it may touch"
            );
        }

        let mut resolved = Vec::with_capacity(paths.len());
        for path in paths {
            if !path.is_absolute() {
                bail!(
                    "the watched path {} is relative: a security boundary has to name \
                     one directory, whichever directory the server was started in",
                    path.display()
                );
            }

            let real = path
                .canonicalize()
                .with_context(|| format!("resolving the watched path {}", path.display()))?;

            if !real.is_dir() {
                bail!("the watched path {} is not a directory", path.display());
            }

            resolved.push(real);
        }

        Ok(Self(resolved))
    }

    /// Watching nothing, which admits nothing.
    pub fn none() -> Self {
        Self(Vec::new())
    }

    /// The paths themselves, for the line the server logs about what it may
    /// touch.
    pub fn paths(&self) -> &[PathBuf] {
        &self.0
    }

    /// Whether `path` is inside a Watched Path, and where it really is if so.
    ///
    /// Blocking: resolving a path is a filesystem read.
    pub fn admit(&self, path: &Path) -> Admission {
        if !path.is_absolute() {
            return Admission::NotAbsolute;
        }

        let Ok(real) = path.canonicalize() else {
            return Admission::Missing;
        };

        // Component by component, which `starts_with` is: `/watched-elsewhere`
        // begins with the text of `/watched` and is not inside it.
        if self.0.iter().any(|watched| real.starts_with(watched)) {
            Admission::Inside(real)
        } else {
            Admission::Outside
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A temporary directory, resolved: on macOS `/tmp` is itself a symlink, so
    /// a Watched Path built from an unresolved tempdir would never match the
    /// resolved paths coming back from [`WatchedPaths::admit`].
    fn tempdir() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    fn watching(dir: &Path) -> WatchedPaths {
        WatchedPaths::resolve(&[dir.to_owned()]).unwrap()
    }

    #[test]
    fn a_directory_inside_a_watched_path_is_admitted_as_its_resolved_self() {
        let dir = tempdir();
        let watched = watching(dir.path());
        let repo = dir.path().join("verkstead");
        std::fs::create_dir(&repo).unwrap();

        assert_eq!(
            watched.admit(&repo),
            Admission::Inside(repo.canonicalize().unwrap())
        );
    }

    #[test]
    fn the_watched_path_itself_is_inside_it() {
        let dir = tempdir();
        let watched = watching(dir.path());

        assert!(matches!(watched.admit(dir.path()), Admission::Inside(_)));
    }

    /// The lookalike: a sibling whose path begins with the Watched Path's text.
    #[test]
    fn a_sibling_whose_name_starts_with_the_watched_path_is_outside() {
        let root = tempdir();
        let watched_dir = root.path().join("watched");
        let lookalike = root.path().join("watched-elsewhere");
        std::fs::create_dir(&watched_dir).unwrap();
        std::fs::create_dir(&lookalike).unwrap();

        assert_eq!(watching(&watched_dir).admit(&lookalike), Admission::Outside);
    }

    /// A path that reads as inside a Watched Path and is not: the symlink is
    /// followed before the boundary is consulted.
    #[test]
    fn a_symlink_out_of_a_watched_path_is_outside_it() {
        let root = tempdir();
        let watched_dir = root.path().join("watched");
        let elsewhere = root.path().join("elsewhere");
        std::fs::create_dir(&watched_dir).unwrap();
        std::fs::create_dir(&elsewhere).unwrap();

        let escape = watched_dir.join("escape");
        std::os::unix::fs::symlink(&elsewhere, &escape).unwrap();

        assert_eq!(watching(&watched_dir).admit(&escape), Admission::Outside);
    }

    /// The other way of reading as inside one: `..` climbs back out, and is
    /// taken out of the path before the boundary is consulted.
    #[test]
    fn a_path_climbing_out_with_dot_dot_is_outside() {
        let root = tempdir();
        let watched_dir = root.path().join("watched");
        let elsewhere = root.path().join("elsewhere");
        std::fs::create_dir(&watched_dir).unwrap();
        std::fs::create_dir(&elsewhere).unwrap();

        let climbed = watched_dir.join("..").join("elsewhere");

        assert_eq!(watching(&watched_dir).admit(&climbed), Admission::Outside);
    }

    /// And `..` that goes nowhere is still inside: the point is where the path
    /// lands, not how it was spelled.
    #[test]
    fn a_path_that_climbs_and_comes_back_is_inside() {
        let root = tempdir();
        let watched_dir = root.path().join("watched");
        let repo = watched_dir.join("verkstead");
        std::fs::create_dir(&watched_dir).unwrap();
        std::fs::create_dir(&repo).unwrap();

        let roundabout = repo.join("..").join("verkstead");

        assert_eq!(
            watching(&watched_dir).admit(&roundabout),
            Admission::Inside(repo.canonicalize().unwrap())
        );
    }

    #[test]
    fn a_relative_path_is_refused_without_being_resolved() {
        let dir = tempdir();

        assert_eq!(
            watching(dir.path()).admit(Path::new("verkstead")),
            Admission::NotAbsolute
        );
    }

    #[test]
    fn a_path_with_nothing_at_it_is_missing() {
        let dir = tempdir();

        assert_eq!(
            watching(dir.path()).admit(&dir.path().join("never-made")),
            Admission::Missing
        );
    }

    /// Fail closed: configuration that went missing must not open the boundary.
    #[test]
    fn watching_nothing_admits_nothing() {
        let dir = tempdir();

        assert_eq!(WatchedPaths::none().admit(dir.path()), Admission::Outside);
    }

    #[test]
    fn resolving_no_watched_paths_at_all_is_refused() {
        assert!(WatchedPaths::resolve(&[]).is_err());
    }

    #[test]
    fn resolving_a_relative_watched_path_is_refused() {
        assert!(WatchedPaths::resolve(&[PathBuf::from("src")]).is_err());
    }

    #[test]
    fn resolving_a_watched_path_that_is_not_there_is_refused() {
        let dir = tempdir();

        assert!(WatchedPaths::resolve(&[dir.path().join("never-made")]).is_err());
    }
}
