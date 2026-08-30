//! The paths as the settings page reads them: every Watched Path and every
//! Sandbox Configuration bind, whoever said it, and whether the server can see
//! what it names.
//!
//! Both of them are said in two places — the installation's flags and
//! `config.yaml` — and Verkstead goes by the union, so a page that drew only one
//! of the two halves would be a page that could not explain what it was looking
//! at. What is composed here is the whole list, with each entry saying which of
//! the two said it: that is what makes an entry editable on the page or
//! read-only on it, because the installation's own are the unit's word and there
//! is nothing here that could rewrite a unit.
//!
//! And each entry says whether it resolves, asked afresh every time this is
//! read. That is a report rather than a check: nothing here refuses anything,
//! and a save lands whatever it was told — see [`crate::settings`], where the
//! same rule holds for the file itself. What it is *for* is the one thing a
//! human cannot see from a phone. A directory nobody has made yet, a path typed
//! with a letter missing, and — on a hardened nix unit — a directory that is
//! there and outside the namespace the service can see, all look identical in a
//! text field, and all three are an entry that does nothing. So the row says so,
//! in words, and on the nix install that is how somebody learns the installer
//! has to widen the unit before what they saved can work.
//!
//! The two lists resolve to different questions, because they are two different
//! permissions. A Watched Path has to be a directory — it bounds where Verkstead
//! may be pointed, and a boundary around a file is a boundary around nothing. A
//! bind only has to be there, which is what [`crate::sandbox`] asks of one as a
//! session spawns.

use std::path::Path;

use verkstead_render::{BindEntry, PathSource, PathsView, Resolution, WatchedPathEntry};

use crate::sandbox::SandboxConfig;
use crate::settings::Settings;
use crate::watched::{self, WatchedPaths};

/// Every path Verkstead has been told about: what `watched` and `binds` were
/// configured with at startup, and whatever `settings` holds at this moment.
///
/// Blocking: the settings file is read and every entry is resolved, which is a
/// handful of `stat` calls.
pub(crate) fn told(
    watched: &WatchedPaths,
    binds: &SandboxConfig,
    settings: &Settings,
) -> PathsView {
    let config = settings.config();

    PathsView {
        watched: watched_told(watched, config.watched_paths()),
        binds: binds_told(binds, config.sandbox_binds()),
    }
}

/// The Watched Paths: the installation's own, then the ones written in the
/// settings, in the order they were written.
///
/// The installation's are already resolved — they were resolved at startup, and
/// a directory that was not there refused to start — so what is asked of them
/// here is whether they are *still* there. A machine that lost one after it came
/// up is one whose page should say so rather than one that goes on drawing a
/// boundary that has stopped covering anything.
fn watched_told(watched: &WatchedPaths, written: &[String]) -> Vec<WatchedPathEntry> {
    let installed = watched.paths().iter().map(|path| WatchedPathEntry {
        path: path.display().to_string(),
        source: PathSource::Installation,
        resolution: resolved_dir(path),
    });

    let said = written.iter().map(|written| WatchedPathEntry {
        // As it was written rather than as it resolves: this is the value the
        // page sends back, and a row that came back resolved would rewrite the
        // human's own file every time they saved anything else.
        path: written.to_owned(),
        source: PathSource::Settings,
        resolution: resolved_dir(Path::new(written)),
    });

    installed.chain(said).collect()
}

/// And the binds, the same way round: the installation's parsed set first, then
/// the entries the settings hold as they were written.
///
/// An entry nothing can be read out of is drawn as itself, scoped to no Repo and
/// unresolved for the reason it could not be read. It is the one kind of row
/// whose path is not a path — and it has to be a row, because a typo that
/// vanished from the page would be a typo nobody could correct.
fn binds_told(binds: &SandboxConfig, written: &[String]) -> Vec<BindEntry> {
    let installed = binds.entries().into_iter().map(|(repo, path)| BindEntry {
        path: path.display().to_string(),
        repo: repo.map(str::to_owned),
        source: PathSource::Installation,
        resolution: there(path),
    });

    let said = written
        .iter()
        .map(|written| match crate::sandbox::read_bind(written) {
            Ok((repo, path)) => BindEntry {
                path: path.display().to_string(),
                repo,
                source: PathSource::Settings,
                resolution: there(&path),
            },
            Err(error) => BindEntry {
                path: written.to_owned(),
                repo: None,
                source: PathSource::Settings,
                resolution: Resolution::Unresolved {
                    why: format!("{error:#}"),
                },
            },
        });

    installed.chain(said).collect()
}

/// Whether `path` is a directory the server can see, in the words
/// [`crate::watched`] would log about it — the same three questions it asks of
/// an entry it is deciding an admission on, so that the page and the boundary
/// cannot come to disagree about what a Watched Path is.
fn resolved_dir(path: &Path) -> Resolution {
    match watched::resolved_dir(path) {
        Ok(_) => Resolution::Resolves,
        Err(error) => Resolution::Unresolved {
            why: format!("{error:#}"),
        },
    }
}

/// And whether `path` is there at all, which is the whole of what a bind asks —
/// see [`crate::sandbox::SandboxConfig::settings_binds`], which drops one that
/// is not as the session spawns.
fn there(path: &Path) -> Resolution {
    match path.exists() {
        true => Resolution::Resolves,
        false => Resolution::Unresolved {
            why: "the server cannot see it: there is nothing at that path".to_owned(),
        },
    }
}
