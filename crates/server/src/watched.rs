//! The Watched Paths: the directories Verkstead is permitted to operate inside.
//!
//! A security boundary rather than a convenience. They are said rather than
//! discovered by scanning, and the decision about whether a path is inside one
//! is taken here — on the server, below every route — so that no request can
//! reach around it by asking differently.
//!
//! They are said in two places and the boundary is the union of both. The
//! installation says its own with `--watched-path`, which are resolved once at
//! startup and fail loudly: a directory that is not there is a misconfiguration
//! to report where it can be fixed. The human says theirs in `config.yaml`,
//! which are read and resolved at the moment an admission is decided and never
//! fail at all: an entry that will not resolve simply covers nothing, with a
//! word in the log. That is what lets a standalone install come up configured by
//! nobody and be set up from its own settings page.
//!
//! Everything is decided on the *resolved* path: `..` taken out, every symlink
//! followed, as the filesystem itself would. A path that merely reads as inside
//! a Watched Path is not inside it, and reading the text of a path is exactly
//! how a boundary gets walked through.
//!
//! Watching nothing is a legal state and a closed one: with no Watched Path
//! configured anywhere every path is outside, so nothing is admitted. That is
//! what a fresh standalone install is, and it is also what a boundary whose
//! configuration went missing has to be — a boundary whose empty case admitted
//! everything would open itself the moment a file did not read.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::settings::Settings;

/// The directories Verkstead may operate inside: what the installation said,
/// resolved once at startup, and a handle on the file the human says the rest in.
#[derive(Debug, Clone, Default)]
pub struct WatchedPaths {
    /// The installation's own, resolved and checked at startup.
    configured: Vec<PathBuf>,

    /// And where to read the human's own from, or `None` for a boundary with no
    /// settings file behind it at all — which is what the routers that watch
    /// nothing are, and what every test of the type below is.
    settings: Option<Settings>,
}

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
    /// The installation's Watched Paths as configured, resolved and checked.
    ///
    /// Resolving here rather than per request is what makes the check cheap and
    /// what makes a misconfiguration loud: a Watched Path that does not exist is
    /// refused at startup, where it can be fixed, rather than silently covering
    /// nothing and refusing every repo inside it.
    ///
    /// None of them is a legal thing to be given. A standalone install has no
    /// unit and no flags to be started with, and it has to be able to reach its
    /// own settings page before it has been configured at all — so an empty set
    /// here is a Verkstead that admits nothing yet rather than one that refuses
    /// to come up.
    pub fn resolve(paths: &[PathBuf]) -> Result<Self> {
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

        Ok(Self {
            configured: resolved,
            settings: None,
        })
    }

    /// The same boundary, widened by whatever `settings` holds at the moment
    /// each admission is decided.
    ///
    /// Attached rather than passed in at [`WatchedPaths::resolve`] because the
    /// settings file lives in the Data Directory, and the router is where the
    /// two are already in the same room — see [`crate::routed`].
    pub(crate) fn reading(self, settings: Settings) -> Self {
        Self {
            settings: Some(settings),
            ..self
        }
    }

    /// Watching nothing, which admits nothing.
    pub fn none() -> Self {
        Self::default()
    }

    /// The installation's own paths, for the line the server logs about what it
    /// may touch. Empty on a standalone install, which is a true thing to log.
    pub fn paths(&self) -> &[PathBuf] {
        &self.configured
    }

    /// Whether `path` is inside a Watched Path, and where it really is if so.
    ///
    /// The settings file is read here, so a Watched Path added to it admits from
    /// the next request on and one taken out of it stops admitting — which costs
    /// nothing already registered, because admission is asked at registration and
    /// never again.
    ///
    /// Blocking: resolving a path is a filesystem read, and so is reading the
    /// file.
    pub fn admit(&self, path: &Path) -> Admission {
        if !path.is_absolute() {
            return Admission::NotAbsolute;
        }

        let Ok(real) = path.canonicalize() else {
            return Admission::Missing;
        };

        if self.covers(&real) {
            Admission::Inside(real)
        } else {
            Admission::Outside
        }
    }

    /// Whether any Watched Path, from either side, holds `real` — which is
    /// already resolved.
    ///
    /// The installation's are asked first because they are already resolved and
    /// the file has not been opened yet: a machine configured by its unit never
    /// reads a settings file to admit a repo inside what the unit said.
    fn covers(&self, real: &Path) -> bool {
        // Component by component, which `starts_with` is: `/watched-elsewhere`
        // begins with the text of `/watched` and is not inside it.
        if self
            .configured
            .iter()
            .any(|watched| real.starts_with(watched))
        {
            return true;
        }

        self.settings_paths()
            .iter()
            .any(|watched| real.starts_with(watched))
    }

    /// What `config.yaml` holds now, resolved, with whatever will not resolve
    /// left out.
    ///
    /// **Nothing here is ever an error.** A relative entry, one naming a
    /// directory that was never made, and one naming a file are each skipped
    /// with a line in the log, and every other entry goes on covering what it
    /// covers. That is the settings side of the line the whole of
    /// [`crate::settings`] is on: the file is edited from a phone, a save lands
    /// whatever it was told, and a typo in it is a directory Verkstead cannot
    /// see rather than a server that will not come up. The flag keeps the other
    /// answer, because a flag is the installation's own word and nobody is
    /// watching when it is wrong.
    ///
    /// The boundary only ever widens from what is here, so a skipped entry
    /// admits nothing and refuses nothing: fail-closed is the failure mode, and
    /// it is the safe one.
    fn settings_paths(&self) -> Vec<PathBuf> {
        let Some(settings) = &self.settings else {
            return Vec::new();
        };

        settings
            .config()
            .watched_paths()
            .iter()
            .filter_map(|written| {
                let path = PathBuf::from(written);

                match resolved_dir(&path) {
                    Ok(real) => Some(real),
                    Err(error) => {
                        tracing::warn!(
                            watched_path = written,
                            error = %error,
                            "a watched path in the settings could not be resolved, so it \
                             covers nothing"
                        );

                        None
                    }
                }
            })
            .collect()
    }
}

/// `path` as the filesystem has it, or what is wrong with it.
///
/// The same three questions [`WatchedPaths::resolve`] asks of the flag's own —
/// absolute, there, a directory — asked here so that the two sides of the
/// boundary agree about what a Watched Path is and disagree only about what to
/// do when one is not.
fn resolved_dir(path: &Path) -> Result<PathBuf> {
    if !path.is_absolute() {
        bail!("it is relative, and a boundary has to name one directory");
    }

    let real = path
        .canonicalize()
        .with_context(|| format!("resolving {}", path.display()))?;

    if !real.is_dir() {
        bail!("{} is not a directory", real.display());
    }

    Ok(real)
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

    /// The standalone case: nothing said at the installation is a boundary
    /// around nothing rather than a refusal, because a server that would not
    /// start unconfigured could never be reached to configure.
    #[test]
    fn resolving_no_watched_paths_at_all_watches_nothing() {
        let dir = tempdir();
        let watched = WatchedPaths::resolve(&[]).unwrap();

        assert!(watched.paths().is_empty());
        assert_eq!(watched.admit(dir.path()), Admission::Outside);
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

    /// The settings side. A `config.yaml` in `data_dir` saying `written`, and a
    /// boundary reading it — which is what every router has, and what the tests
    /// above deliberately do not.
    fn settings_watching(data_dir: &Path, written: &[&Path]) -> WatchedPaths {
        write_watched(data_dir, written);

        WatchedPaths::none().reading(Settings::in_data_dir(data_dir))
    }

    fn write_watched(data_dir: &Path, written: &[&Path]) {
        let paths = written
            .iter()
            .map(|path| path.to_str().unwrap().to_owned())
            .collect();

        Settings::in_data_dir(data_dir)
            .save_config(&crate::settings::Config::of(
                crate::settings::GitAuthor::default(),
                crate::settings::RustBuildCache::default(),
                None,
                vec![],
                paths,
            ))
            .unwrap();
    }

    #[test]
    fn a_watched_path_in_the_settings_admits_what_is_inside_it() {
        let data_dir = tempdir();
        let dir = tempdir();
        let repo = dir.path().join("verkstead");
        std::fs::create_dir(&repo).unwrap();

        let watched = settings_watching(data_dir.path(), &[dir.path()]);

        assert_eq!(
            watched.admit(&repo),
            Admission::Inside(repo.canonicalize().unwrap())
        );
    }

    /// The union: the flag's set and the file's are both the boundary, and
    /// neither stands in for the other.
    #[test]
    fn the_two_sides_are_one_boundary() {
        let data_dir = tempdir();
        let configured = tempdir();
        let said = tempdir();

        let watched = watching(configured.path()).reading(Settings::in_data_dir(data_dir.path()));
        write_watched(data_dir.path(), &[said.path()]);

        assert!(matches!(
            watched.admit(configured.path()),
            Admission::Inside(_)
        ));
        assert!(matches!(watched.admit(said.path()), Admission::Inside(_)));
    }

    /// Read per admission rather than held: a directory added to the file
    /// admits from the next question on, and one taken out of it stops
    /// admitting.
    #[test]
    fn the_file_is_read_at_every_admission() {
        let data_dir = tempdir();
        let dir = tempdir();
        let watched = settings_watching(data_dir.path(), &[]);

        assert_eq!(watched.admit(dir.path()), Admission::Outside);

        write_watched(data_dir.path(), &[dir.path()]);
        assert!(matches!(watched.admit(dir.path()), Admission::Inside(_)));

        write_watched(data_dir.path(), &[]);
        assert_eq!(watched.admit(dir.path()), Admission::Outside);
    }

    /// Nothing in the file is ever fatal, and an entry that will not resolve
    /// costs the ones beside it nothing: it simply covers nothing.
    #[test]
    fn a_settings_watched_path_that_will_not_resolve_covers_nothing() {
        let data_dir = tempdir();
        let dir = tempdir();
        let file = dir.path().join("notes.md");
        std::fs::write(&file, "not a directory\n").unwrap();

        let watched = settings_watching(
            data_dir.path(),
            &[
                &dir.path().join("never-made"),
                Path::new("src"),
                &file,
                dir.path(),
            ],
        );

        assert_eq!(
            watched.admit(&dir.path().join("never-made")),
            Admission::Missing
        );
        assert_eq!(watched.admit(Path::new("src")), Admission::NotAbsolute);

        // The one that resolves goes on covering what it covers.
        assert!(matches!(watched.admit(dir.path()), Admission::Inside(_)));
    }
}
