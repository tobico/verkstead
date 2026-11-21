//! **A Plugin from a git repository** (ADR-0030, *Where plugins live*): a
//! clone under the Data Directory's `plugins/<uuid>/`, which every device makes
//! for itself, checked out at the revision the record pins.
//!
//! **`plugins/` is a sibling of `skills/`**, which `Skills::installed` wipes at
//! every start, and nothing here is ever under it.
//!
//! **Nothing fetches on its own.** A clone is made at **Install**, and on a
//! device the record reaches without one; a fetch is made at **Update**, the
//! press, and to find a pinned revision this device's clone has not seen. The
//! plugin tracks its remote's default branch, which is what an Update moves to.
//!
//! **Every git here reaches the remote the way a Repo's clone does** — see
//! [`crate::repos::reaching_as`] — and asks nothing of anybody: a clone that
//! wants a password fails, rather than waiting for one. Every function blocks,
//! and is called on `spawn_blocking`.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::github::Credential;
use crate::unseen::Unseen;

/// Where this device keeps its plugin clones.
pub(crate) fn home(data_dir: &Path) -> PathBuf {
    data_dir.join("plugins")
}

/// Where this device keeps the clone of the plugin `uuid`.
pub(crate) fn checkout(data_dir: &Path, uuid: &str) -> PathBuf {
    home(data_dir).join(uuid)
}

/// Whether what the human gave **Add a plugin…** is a git URL: one with a
/// scheme, or the `[user@]host:path` spelling of an SSH one. A Windows drive's
/// single letter before its colon is a path, as it is to git.
pub(crate) fn is_url(asked: &str) -> bool {
    if asked.contains("://") {
        return true;
    }

    match asked.split_once(':') {
        Some((host, _)) => !host.is_empty() && !host.contains(['/', '\\']) && host.len() > 1,
        None => false,
    }
}

/// A clone of `url` in a staging directory under `plugins/`, which is taken
/// back when this is dropped unless it [`Staged::lands`].
pub(crate) struct Staged {
    dir: tempfile::TempDir,
    revision: String,
}

impl Staged {
    pub(crate) fn path(&self) -> &Path {
        self.dir.path()
    }

    /// The commit it is checked out at: the remote's default branch, as the
    /// clone found it.
    pub(crate) fn revision(&self) -> &str {
        &self.revision
    }

    /// Move the clone to `to`, in place of anything a clone that never landed
    /// left there.
    pub(crate) fn lands(self, to: &Path) -> Result<(), String> {
        if to.symlink_metadata().is_ok() {
            std::fs::remove_dir_all(to)
                .map_err(|error| format!("The old checkout couldn't be cleared: {error}."))?;
        }

        let from = self.dir.keep();
        std::fs::rename(&from, to).map_err(|error| {
            let _ = std::fs::remove_dir_all(&from);
            format!("The clone couldn't be put in place: {error}.")
        })
    }
}

/// Clone `url` into a staging directory under `data_dir`'s `plugins/`, or say
/// why it couldn't be — a URL that isn't a repository being git's to say.
pub(crate) fn staged(
    data_dir: &Path,
    url: &str,
    credential: &Credential,
) -> Result<Staged, String> {
    let home = home(data_dir);
    std::fs::create_dir_all(&home)
        .map_err(|error| format!("The plugins directory couldn't be made: {error}."))?;

    let dir = tempfile::Builder::new()
        .prefix(".staging-")
        .tempdir_in(&home)
        .map_err(|error| format!("A directory to clone into couldn't be made: {error}."))?;

    reaching(
        dir.path(),
        credential,
        &["clone", "--quiet", "--", url, "."],
    )?;
    let revision = head(dir.path())?;

    Ok(Staged { dir, revision })
}

/// This device's clone of the plugin `uuid`, checked out at `revision`: cloned
/// from `url` where there is none yet, and fetched where the revision is one it
/// hasn't seen.
pub(crate) fn pinned(
    data_dir: &Path,
    uuid: &str,
    url: &str,
    revision: &str,
    credential: &Credential,
) -> Result<PathBuf, String> {
    let at = checkout(data_dir, uuid);

    if !at.join(".git").exists() {
        let staged = staged(data_dir, url, credential)?;
        check_out(staged.path(), revision)?;
        staged.lands(&at)?;
        return Ok(at);
    }

    if head(&at).is_ok_and(|head| head == revision) {
        return Ok(at);
    }

    if run(&at, &["cat-file", "-e", &format!("{revision}^{{commit}}")]).is_err() {
        reaching(&at, credential, &["fetch", "--quiet", "origin"])?;
    }
    check_out(&at, revision)?;

    Ok(at)
}

/// Fetch the remote's default branch into `dir`, answering the commit it is at
/// now.
pub(crate) fn fetched(dir: &Path, credential: &Credential) -> Result<String, String> {
    reaching(dir, credential, &["fetch", "--quiet", "origin", "HEAD"])?;
    said(dir, &["rev-parse", "FETCH_HEAD"])
}

/// Check `dir` out at `revision`, whatever its working tree held.
pub(crate) fn check_out(dir: &Path, revision: &str) -> Result<(), String> {
    run(
        dir,
        &["checkout", "--quiet", "--force", "--detach", revision],
    )
    .map_err(|why| {
        format!(
            "Revision {} couldn't be checked out: {why}",
            short(revision)
        )
    })
}

/// The commit `dir` is checked out at.
fn head(dir: &Path) -> Result<String, String> {
    said(dir, &["rev-parse", "HEAD"])
}

/// A revision as a row draws it.
pub(crate) fn short(revision: &str) -> &str {
    revision.get(..7).unwrap_or(revision)
}

/// A git that reaches the remote, in `dir`.
fn reaching(dir: &Path, credential: &Credential, args: &[&str]) -> Result<(), String> {
    let mut command = Command::new("git");
    crate::repos::reaching_as(&mut command, credential).args(args);
    finished(command, dir).map(drop)
}

/// A git that stays on this device, in `dir`.
fn run(dir: &Path, args: &[&str]) -> Result<(), String> {
    let mut command = Command::new("git");
    command.args(args);
    finished(command, dir).map(drop)
}

/// A git that stays on this device, in `dir`, answering its one line.
fn said(dir: &Path, args: &[&str]) -> Result<String, String> {
    let mut command = Command::new("git");
    command.args(args);
    finished(command, dir).map(|out| out.trim().to_owned())
}

fn finished(mut command: Command, dir: &Path) -> Result<String, String> {
    let output = command
        .unseen()
        .current_dir(dir)
        .stdin(Stdio::null())
        .output()
        .map_err(|error| format!("Git couldn't be run: {error}."))?;

    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).into_owned());
    }

    Err(crate::publishing::said(&String::from_utf8_lossy(
        &output.stderr,
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls_and_paths_are_told_apart() {
        assert!(is_url("https://gitlab.com/you/plugin.git"));
        assert!(is_url("file:///srv/plugin"));
        assert!(is_url("git@github.com:you/plugin"));
        assert!(!is_url("/home/you/src/plugin"));
        assert!(!is_url(r"C:\Users\you\plugin"));
        assert!(!is_url("relative/plugin"));
        assert!(!is_url("./a:b"));
    }
}
