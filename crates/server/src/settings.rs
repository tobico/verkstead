//! The settings files: what Verkstead is told, rather than what it finds.
//!
//! GitHub auth used to be whatever happened to sit in the service's home — the
//! host's `~/.config/gh`, bound into every sandbox and hoped to be logged in.
//! That is credentials by accident: nobody says which account a session runs
//! as, nothing says whether one is configured at all, and the failure arrives
//! inside a sandbox as `gh` claiming it has never heard of the machine.
//!
//! So the credentials are said instead, in files of Verkstead's own under the
//! Data Directory beside the database. Two of them, split by whether what is in
//! them is secret rather than by what it configures. `secrets.yaml` is the one
//! with anything secret in it:
//!
//! ```yaml
//! github_token: ghp_...
//! ```
//!
//! and `config.yaml` is the one that could be read over anybody's shoulder:
//!
//! ```yaml
//! git_author:
//!   name: Tobias Cohen
//!   email: tobi@tobico.net
//! rust_build_cache:
//!   enabled: true
//!   size: 30G
//! share_viewer_url: https://ada.github.io/verkstead-share-viewer/
//! conflict_resolution: merge
//! sandbox_binds:
//!   - /var/cache/verkstead-node
//!   - verkstead=/var/cache/verkstead-cargo
//! watched_paths:
//!   - /home/tobi/src
//! ```
//!
//! Who a session commits as is said here for the reason the token is: it used
//! to be found — the host's `~/.gitconfig`, bound into every sandbox — and an
//! identity nobody chose is one nobody can see they have chosen. Both files are
//! read at the moment they are needed rather than held from startup, so
//! anything saved through the settings page applies to the next session without
//! a restart, and a running session keeps what it started with.
//!
//! **Nothing here is ever an error.** A file that is not there, one that is
//! empty, and one nothing can parse all come back as nothing configured: the
//! consequence of no token is `gh` inside saying it is not logged in, and of no
//! author is git inside asking to be told who you are, where the consequence of
//! refusing would be a session that never starts. The malformed case is logged,
//! because a file the human wrote and Verkstead cannot read is the one of the
//! three they would want telling about.
//!
//! Which is why `rust_build_cache` is written the way it is: an absent key, an
//! absent file and an unparseable one all mean the shared build cache is on at
//! its default size. The setting is here rather than on the command line
//! because it is the one sandbox control the human may reasonably want to reach
//! from a phone — see [`RustBuildCache`], and
//! [`crate::build_cache`] for what it switches.
//!
//! `conflict_resolution` is written that way too, and the default it falls back
//! to is the safe half of the choice: a conflicted pull request has its base
//! merged in rather than its branch rebased and force-pushed. One Repo may say
//! otherwise — that override is a fact about the Repo and lives in the store
//! beside it, not here.

use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::store::ConflictResolution;

/// What the secrets file is called inside the Data Directory. Fixed rather than
/// configurable, for the reason the database's name is: the directory is what an
/// operator points Verkstead at, and what is in it is Verkstead's to name.
const SECRETS: &str = "secrets.yaml";

/// And what the other one is called: everything configured that is nobody's
/// secret.
const CONFIG: &str = "config.yaml";

/// What `secrets.yaml` is written as: readable and writable by the account
/// Verkstead runs under, and by nothing else on the machine. A GitHub token is
/// a password to every repository the human can reach, and a file holding one
/// that any process could read would undo the point of saying it here rather
/// than leaving it in a home directory.
const SECRET_MODE: u32 = 0o600;

/// And what `config.yaml` is written as, which is the ordinary thing: a name and
/// an email address are on every commit either of them ever makes.
const ORDINARY_MODE: u32 = 0o644;

/// Where the settings files are: the Data Directory, and nothing else to hold.
///
/// A handle rather than the contents, because the contents are read afresh every
/// time they are asked for — see this module's own documentation.
#[derive(Debug, Clone)]
pub struct Settings {
    dir: PathBuf,
}

impl Settings {
    /// The settings files kept in `data_dir`, beside the database.
    pub fn in_data_dir(data_dir: &Path) -> Settings {
        Settings {
            dir: data_dir.to_owned(),
        }
    }

    /// Where `secrets.yaml` is, which is what writes it and what says so on a
    /// settings page.
    pub fn secrets_path(&self) -> PathBuf {
        self.dir.join(SECRETS)
    }

    /// Where `config.yaml` is, which is the same to a settings page.
    pub fn config_path(&self) -> PathBuf {
        self.dir.join(CONFIG)
    }

    /// What `secrets.yaml` holds now.
    ///
    /// Blocking, and called where blocking is allowed: a session's sandbox is
    /// built on a blocking thread already, because git is asked about the
    /// worktree there.
    pub fn secrets(&self) -> Secrets {
        let path = self.secrets_path();
        let Some(text) = text_of(&path) else {
            return Secrets::default();
        };

        Secrets::read(&text).unwrap_or_else(|error| {
            unreadable(&path, &error);
            Secrets::default()
        })
    }

    /// And what `config.yaml` holds now, read the same way and at the same
    /// moment: the pair is what a session's git is configured out of, so they
    /// are decided together.
    pub fn config(&self) -> Config {
        let path = self.config_path();
        let Some(text) = text_of(&path) else {
            return Config::default();
        };

        Config::read(&text).unwrap_or_else(|error| {
            unreadable(&path, &error);
            Config::default()
        })
    }

    /// When `secrets.yaml` was last written, or `None` where there is no file to
    /// have a time.
    ///
    /// The file's own modification time rather than a stamp kept beside the
    /// token, for the reason everything else here is read fresh: the file is the
    /// source of truth, and a stored stamp would go on claiming a day after a
    /// hand-edit moved the token.
    pub fn secrets_written_at(&self) -> Option<OffsetDateTime> {
        let written = std::fs::metadata(self.secrets_path())
            .ok()?
            .modified()
            .ok()?;

        Some(OffsetDateTime::from(written))
    }

    /// Write `secrets.yaml`, replacing whatever is there.
    ///
    /// Mode 0600 and atomically. The mode because a file holding a GitHub token
    /// has no business being readable by anything else on the machine, and
    /// atomically because the alternative is a window in which the file is
    /// truncated: a session spawning in that window would be one that quietly
    /// had no credentials, which is the failure this whole feature is about.
    ///
    /// Clearing writes an empty file rather than removing one. It says exactly
    /// what a missing file says — see [`Secrets::read`] — and leaving the file
    /// there keeps its mode, its ownership and the fact that this is where the
    /// token goes.
    pub fn save_secrets(&self, secrets: &Secrets) -> std::io::Result<()> {
        let text = match secrets.github_token() {
            Some(_) => yaml(secrets)?,
            None => String::new(),
        };

        write_atomically(&self.secrets_path(), &text, SECRET_MODE)
    }

    /// And write `config.yaml`, the same way but readable: there is nothing in
    /// it that is anybody's secret, and a name and an address the machine's
    /// owner cannot read back would be an odd thing to insist on.
    pub fn save_config(&self, config: &Config) -> std::io::Result<()> {
        write_atomically(&self.config_path(), &yaml(config)?, ORDINARY_MODE)
    }
}

/// One settings file as YAML, ready to be written.
///
/// Serialized rather than formatted by hand, because what goes in these files is
/// the human's own prose: a name with a colon in it, an address in angle
/// brackets, a token that begins with a character YAML reads as markup. A
/// serializer knows when to quote and a `format!` does not.
///
/// A value that will not serialize is an `io::Error` here rather than a kind of
/// its own. There is nothing in either of these files that can fail to become
/// YAML — two strings and a token — so the only caller worth writing is the one
/// that reports a file it could not write.
fn yaml<T: Serialize>(value: &T) -> std::io::Result<String> {
    serde_saphyr::to_string(value).map_err(std::io::Error::other)
}

/// Write `text` to `path` with mode `mode`, so that a reader sees either the old
/// file or the new one and never a half of either.
///
/// Through a neighbouring temporary file and a rename, which is atomic within a
/// directory. The mode is set as the temporary file is created rather than
/// afterwards, so there is no instant in which a file holding a token stands
/// world-readable — and the temporary is named for this process, so two
/// Verksteads pointed at one Data Directory would each replace the file rather
/// than half-write one between them.
///
/// A rename that fails leaves the temporary behind. It is named plainly enough
/// to be recognised for what it is, and the alternative — unwinding on the way
/// out of an error — is more that can go wrong on the path where something
/// already has.
fn write_atomically(path: &Path, text: &str, mode: u32) -> std::io::Result<()> {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "settings".to_owned());

    let temp = path.with_file_name(format!(".{name}.{}.new", std::process::id()));

    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(mode)
        .open(&temp)?;

    file.write_all(text.as_bytes())?;

    // Before the rename rather than after it: a rename makes the new file the
    // one everything reads, and a machine that lost power between the two would
    // have replaced the settings with a file of nothing.
    file.sync_all()?;
    drop(file);

    // An existing file's mode is the file's rather than the directory's default,
    // so a `secrets.yaml` written by an earlier Verkstead — or by hand, at
    // whatever mode the human's umask gave it — is brought to this one's by the
    // replacement.
    std::fs::rename(&temp, path)
}

/// What is in the settings file at `path`, or `None` where there is nothing to
/// read — which is a file nobody has written yet, and is what an installation
/// before the settings page looks like.
///
/// A file that is there and will not open is the odd one: logged, because
/// permissions nobody meant to set are worth saying out loud, and then treated
/// as the missing one for the reason this whole module refuses to fail.
fn text_of(path: &Path) -> Option<String> {
    match std::fs::read_to_string(path) {
        Ok(text) => Some(text),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            tracing::debug!(path = %path.display(), "no settings file here, so nothing from it");
            None
        }
        Err(error) => {
            tracing::warn!(
                error = ?error,
                path = %path.display(),
                "the settings file could not be read, so nothing is configured from it"
            );
            None
        }
    }
}

/// Say that a settings file is not YAML this understands. The one thing worth
/// telling the human about, because it is the one they can fix.
fn unreadable(path: &Path, error: &serde_saphyr::Error) {
    tracing::warn!(
        error = %error,
        path = %path.display(),
        "the settings file is not YAML this understands, so nothing is configured from it"
    );
}

/// What `secrets.yaml` says. Flat, because there is one secret in it.
///
/// Unknown keys are ignored rather than refused: the human hand-edits this file,
/// and a key from a later Verkstead — or a comment they left as a key by mistake
/// — is not worth taking a session's credentials away over.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Secrets {
    /// The GitHub token every session and every host-side `gh` authenticates
    /// with, or `None` where none is configured.
    ///
    /// Left out of what is written rather than written as `null`: the file this
    /// produces is one the human may open, and a key with nothing under it
    /// reads as a setting that went wrong rather than as one nobody has made.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    github_token: Option<String>,
}

impl Secrets {
    /// What `text` says, or what went wrong reading it.
    ///
    /// An empty file is not a parse failure and must not be logged as one: it is
    /// what a settings page leaves behind after the token is cleared, and it says
    /// exactly what a missing file says.
    fn read(text: &str) -> Result<Secrets, serde_saphyr::Error> {
        if text.trim().is_empty() {
            return Ok(Secrets::default());
        }

        let secrets: Secrets = serde_saphyr::from_str(text)?;

        Ok(Secrets {
            github_token: secrets.github_token.and_then(blank_is_nothing),
        })
    }

    /// The secrets a settings page has just been told: `token` as it was typed,
    /// or `None` where the human cleared it.
    ///
    /// Whitespace is nothing, as it is on the way in — see [`blank_is_nothing`].
    /// A token pasted with the newline that came with it is the ordinary case,
    /// and one that was only spaces is a cleared field spelled another way.
    pub fn of_token(token: Option<String>) -> Secrets {
        Secrets {
            github_token: token.and_then(blank_is_nothing),
        }
    }

    /// The configured GitHub token, or `None` where there is none.
    pub fn github_token(&self) -> Option<&str> {
        self.github_token.as_deref()
    }
}

/// What `config.yaml` says: everything told to Verkstead that is nobody's
/// secret.
///
/// Unknown keys are ignored for the reason [`Secrets`]'s are.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Config {
    /// Who a session commits as.
    #[serde(default)]
    git_author: GitAuthor,

    /// And how the shared Rust build cache is set: whether sessions get one at
    /// all, and how big its compiled half may grow — see [`crate::build_cache`].
    ///
    /// The one thing in either file that is about the sandbox rather than about
    /// an identity, and the first control the workbench has that is. A setting
    /// rather than a flag because it is the human's to change from a phone, and
    /// safe to be: what it opens is a directory of Verkstead's own making, and
    /// the switch is the one that *closes* it.
    #[serde(default)]
    rust_build_cache: RustBuildCache,

    /// Where the human put a **share viewer** of their own — the small page
    /// Verkstead ships that draws a Published Share in a browser, hosted on a
    /// public site of theirs.
    ///
    /// Configuration rather than a secret, and the plainest thing in either
    /// file: a URL, or nothing. Nothing is a Verkstead that has never been to
    /// that section, and it costs nothing — links are composed through the copy
    /// Verkstead itself hosts, `HOSTED` in [`crate::sharing`]. This is an
    /// override for a human who would rather serve the page themselves.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    share_viewer_url: Option<String>,

    /// And how a pull request that will not merge is resolved: the base merged
    /// in, which is what nobody choosing anything gets, or the branch rebased
    /// onto the base and force-pushed.
    ///
    /// Written the way `rust_build_cache` is, and for the same reason: an absent
    /// key, an absent file and one nothing can parse all mean a merge. A human
    /// should never have a worse experience for not having checked the settings,
    /// and the worse experience here is a branch rewritten under whoever was
    /// reading it.
    ///
    /// One Repo can say otherwise — that override is a fact about the Repo and
    /// lives in the store beside it, and this is what it falls back to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    conflict_resolution: Option<ConflictResolution>,

    /// And the Sandbox Configuration binds said here rather than at the
    /// installation: a flat list in the grammar `--sandbox-bind` takes,
    /// `/abs/path` for a bind every sandbox gets and `name=/abs/path` for one
    /// only the Repo registered under that name does.
    ///
    /// They compose with the installation's own set rather than replacing it,
    /// and they are read at the moment a session spawns, like the author above
    /// and the build cache beside it. Which is why nothing here is checked as it
    /// is read: an entry naming a directory that is not there is skipped at that
    /// moment with a word in the log, where a startup flag naming one refuses to
    /// start — see [`crate::sandbox::SandboxConfig::settings_binds`].
    #[serde(
        default,
        deserialize_with = "rows_written",
        skip_serializing_if = "Vec::is_empty"
    )]
    sandbox_binds: Vec<String>,

    /// And the Watched Paths said here rather than at the installation: a flat
    /// list of absolute directories, the same thing `--watched-path` names.
    ///
    /// They widen the boundary rather than standing in for the part of it the
    /// installation drew, and they are read at the moment an admission is
    /// decided, so a directory added here admits from the next request on. Which
    /// is why nothing here is checked as it is read: an entry naming a directory
    /// that is not there covers nothing at that moment, with a word in the log,
    /// where a startup flag naming one refuses to start — see
    /// [`crate::WatchedPaths::admit`].
    ///
    /// An empty list is what a standalone install starts with, and it is a
    /// closed boundary rather than an open one: the type fails closed whatever
    /// is here, so a Verkstead nobody has configured may touch nothing at all.
    #[serde(
        default,
        deserialize_with = "rows_written",
        skip_serializing_if = "Vec::is_empty"
    )]
    watched_paths: Vec<String>,
}

impl Config {
    /// What `text` says, or what went wrong reading it. An empty file is not a
    /// failure, for the reason it is not one in [`Secrets::read`].
    fn read(text: &str) -> Result<Config, serde_saphyr::Error> {
        if text.trim().is_empty() {
            return Ok(Config::default());
        }

        let config: Config = serde_saphyr::from_str(text)?;

        Ok(Config {
            git_author: GitAuthor {
                name: config.git_author.name.and_then(blank_is_nothing),
                email: config.git_author.email.and_then(blank_is_nothing),
            },
            rust_build_cache: RustBuildCache {
                enabled: config.rust_build_cache.enabled,
                size: config.rust_build_cache.size.and_then(blank_is_nothing),
            },
            share_viewer_url: config.share_viewer_url.and_then(blank_is_nothing),
            conflict_resolution: config.conflict_resolution,
            sandbox_binds: entries_written(config.sandbox_binds),
            watched_paths: entries_written(config.watched_paths),
        })
    }

    /// The config a settings page has just been told.
    pub fn of(
        git_author: GitAuthor,
        rust_build_cache: RustBuildCache,
        share_viewer_url: Option<String>,
        conflict_resolution: ConflictResolution,
        sandbox_binds: Vec<String>,
        watched_paths: Vec<String>,
    ) -> Config {
        Config {
            git_author,
            rust_build_cache,
            share_viewer_url: share_viewer_url.and_then(blank_is_nothing),
            // Written down as it stands rather than left out where it is the
            // default, the way the build cache's switch is: what the page sends
            // is where the setting is to sit, and a key that appeared only for
            // one of the two answers would read as a file half-written.
            conflict_resolution: Some(conflict_resolution),
            sandbox_binds: entries_written(sandbox_binds),
            watched_paths: entries_written(watched_paths),
        }
    }

    /// Who a session commits as, which may be nobody.
    pub fn git_author(&self) -> &GitAuthor {
        &self.git_author
    }

    /// And how the build cache is set, which is on at the default size where
    /// nobody has said otherwise.
    pub fn rust_build_cache(&self) -> &RustBuildCache {
        &self.rust_build_cache
    }

    /// And where the human hosts a share viewer of their own, or `None` where
    /// they host none.
    ///
    /// `None` rather than the address links are actually composed through, and
    /// deliberately: this is what the settings page draws back into its field,
    /// and a field filled in with something nobody typed is a setting the human
    /// cannot tell they have not chosen. What a blank one *means* is
    /// [`crate::sharing::link`]'s to say.
    pub fn share_viewer_url(&self) -> Option<&str> {
        self.share_viewer_url.as_deref()
    }

    /// And how a conflict is resolved where the Repo it is in says nothing,
    /// which is a merge until somebody says otherwise.
    ///
    /// Where the switch above answers rather than the field beside it: there is
    /// no third state to draw, so what comes back is where the setting *sits*
    /// and not whether anybody has been here.
    pub fn conflict_resolution(&self) -> ConflictResolution {
        self.conflict_resolution
            .unwrap_or(ConflictResolution::Merge)
    }

    /// And the binds it holds, in the order they were written down. An empty
    /// list where nobody has added any, which is a sandbox with whatever the
    /// installation configured and nothing beside it.
    pub fn sandbox_binds(&self) -> &[String] {
        &self.sandbox_binds
    }

    /// And the Watched Paths it holds, in the order they were written down. An
    /// empty list where nobody has added any, which is a boundary drawn by
    /// whatever the installation configured — and, on an installation that
    /// configured none, a boundary around nothing.
    pub fn watched_paths(&self) -> &[String] {
        &self.watched_paths
    }
}

/// The shared Rust build cache as the human left it: whether sessions get one,
/// and how much disk its compiled half may take.
///
/// Both halves are optional and both are absent on a machine nobody has been to
/// the settings page of — which is **on**, at the default size. That is the
/// whole of the shape, and it is deliberate: a human should never have a worse
/// experience for not having checked the settings, so an unwritten file says
/// what a switch somebody turned on says.
///
/// The size is the human's own word rather than a number of bytes. It is
/// `SCCACHE_CACHE_SIZE`, which sccache reads as `10G`, `500M` and so on, and
/// nothing here parses it: what sccache makes of a word it cannot read is
/// sccache's to say, and a parser here would be a second opinion about the one
/// thing the value is for.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct RustBuildCache {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    enabled: Option<bool>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    size: Option<String>,
}

impl RustBuildCache {
    /// What a settings page has just been told: the switch, and the size where
    /// one was typed.
    pub fn of(enabled: bool, size: Option<String>) -> RustBuildCache {
        RustBuildCache {
            enabled: Some(enabled),
            size: size.and_then(blank_is_nothing),
        }
    }

    /// Whether a session gets one. Nothing configured is **on**.
    pub fn enabled(&self) -> bool {
        self.enabled.unwrap_or(true)
    }

    /// And how big its compiled half may get, which is
    /// [`crate::build_cache::SIZE`] where nobody has said.
    pub fn size(&self) -> &str {
        self.size.as_deref().unwrap_or(crate::build_cache::SIZE)
    }

    /// The size exactly as it is written down, and `None` where nobody has
    /// written one: what a settings page draws as a placeholder rather than as
    /// a value somebody chose.
    pub fn size_configured(&self) -> Option<&str> {
        self.size.as_deref()
    }
}

/// The name and the email address a session's commits are by.
///
/// Two halves, each on its own: a human who has filled in one and not the other
/// gets the one they filled in, and git says what is still missing. Nothing here
/// substitutes a default — a commit by `verkstead@localhost` is worse than a
/// commit that would not be made, because it is the one nobody notices.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct GitAuthor {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    name: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    email: Option<String>,
}

impl GitAuthor {
    /// The author a settings page has just been told, each half on its own and
    /// each blank half nobody — see [`blank_is_nothing`].
    pub fn of(name: Option<String>, email: Option<String>) -> GitAuthor {
        GitAuthor {
            name: name.and_then(blank_is_nothing),
            email: email.and_then(blank_is_nothing),
        }
    }

    /// What `user.name` is inside a sandbox, where one is configured.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// And what `user.email` is.
    pub fn email(&self) -> Option<&str> {
        self.email.as_deref()
    }
}

/// A list of rows as somebody left them, with the ones they emptied out taken
/// away.
///
/// A row with nothing after its `-` is YAML's null rather than YAML's empty
/// string, and a `Vec<String>` reading one refuses the whole file — which under
/// this module's own rule would throw the author and the build cache away over a
/// half-deleted line. So the rows are read as nullable and the nulls dropped,
/// which is what an emptied row was always going to mean.
fn rows_written<'de, D: serde::Deserializer<'de>>(rows: D) -> Result<Vec<String>, D::Error> {
    Ok(Vec::<Option<String>>::deserialize(rows)?
        .into_iter()
        .flatten()
        .collect())
}

/// A written list with its blank entries taken out and the rest trimmed: a row
/// the human emptied rather than deleted says as little as a field they cleared
/// does, and an entry with a stray space around it is the path they meant.
fn entries_written(entries: Vec<String>) -> Vec<String> {
    entries.into_iter().filter_map(blank_is_nothing).collect()
}

/// A configured value that is only whitespace is no value: a field left empty by
/// hand reads as the human having cleared it, and a variable set to nothing at
/// all is a session that fails obscurely rather than one that says plainly what
/// it has not got — `GH_TOKEN=` is a login `gh` chokes on, and an empty
/// `user.name` is a commit by nobody that git makes without a word.
fn blank_is_nothing(value: String) -> Option<String> {
    let trimmed = value.trim();

    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::PermissionsExt;

    use super::{Config, ConflictResolution, GitAuthor, RustBuildCache, Secrets, Settings};

    #[test]
    fn the_token_is_what_the_file_says() {
        let secrets = Secrets::read("github_token: ghp_thetoken\n").unwrap();

        assert_eq!(secrets.github_token(), Some("ghp_thetoken"));
    }

    #[test]
    fn a_key_this_version_never_heard_of_is_not_the_end_of_the_file() {
        let secrets = Secrets::read("github_token: ghp_thetoken\nsomething_later: yes\n").unwrap();

        assert_eq!(secrets.github_token(), Some("ghp_thetoken"));
    }

    #[test]
    fn an_empty_file_configures_nothing_and_is_not_a_failure() {
        assert_eq!(Secrets::read("").unwrap().github_token(), None);
        assert_eq!(
            Secrets::read("\n# nothing yet\n").unwrap().github_token(),
            None
        );
    }

    #[test]
    fn a_blank_token_is_no_token() {
        assert_eq!(
            Secrets::read("github_token: ''\n").unwrap().github_token(),
            None
        );
        assert_eq!(
            Secrets::read("github_token:\n").unwrap().github_token(),
            None
        );
    }

    #[test]
    fn nothing_that_will_parse_is_a_failure_to_report() {
        assert!(Secrets::read("github_token: [oh\n").is_err());
    }

    #[test]
    fn a_missing_file_is_no_token_and_no_complaint() {
        let dir = tempfile::tempdir().unwrap();

        assert_eq!(
            Settings::in_data_dir(dir.path()).secrets().github_token(),
            None
        );
    }

    #[test]
    fn a_file_that_is_there_is_read_every_time_it_is_asked_for() {
        let dir = tempfile::tempdir().unwrap();
        let settings = Settings::in_data_dir(dir.path());

        std::fs::write(settings.secrets_path(), "github_token: the-first\n").unwrap();
        assert_eq!(settings.secrets().github_token(), Some("the-first"));

        // A rotation, which is what the settings page will do to this file.
        std::fs::write(settings.secrets_path(), "github_token: the-second\n").unwrap();
        assert_eq!(settings.secrets().github_token(), Some("the-second"));
    }

    #[test]
    fn a_file_nothing_can_parse_leaves_a_session_startable() {
        let dir = tempfile::tempdir().unwrap();
        let settings = Settings::in_data_dir(dir.path());

        std::fs::write(settings.secrets_path(), "github_token: [oh\n").unwrap();

        assert_eq!(settings.secrets().github_token(), None);
    }

    #[test]
    fn the_author_is_what_the_config_file_says() {
        let config =
            Config::read("git_author:\n  name: Tobias Cohen\n  email: tobi@tobico.net\n").unwrap();

        assert_eq!(config.git_author().name(), Some("Tobias Cohen"));
        assert_eq!(config.git_author().email(), Some("tobi@tobico.net"));
    }

    #[test]
    fn half_an_author_is_the_half_that_was_filled_in() {
        let config = Config::read("git_author:\n  name: Tobias Cohen\n  email: ''\n").unwrap();

        assert_eq!(config.git_author().name(), Some("Tobias Cohen"));
        assert_eq!(
            config.git_author().email(),
            None,
            "an empty address is one the human cleared, and git says so for itself"
        );
    }

    #[test]
    fn where_the_share_viewer_is_hosted_is_what_the_config_file_says() {
        let config = Config::read("share_viewer_url: https://ada.github.io/shares/\n").unwrap();

        assert_eq!(
            config.share_viewer_url(),
            Some("https://ada.github.io/shares/")
        );
    }

    /// How a conflict is resolved is the human's word for it, and what they
    /// wrote is what comes back.
    #[test]
    fn how_a_conflict_is_resolved_is_what_the_config_file_says() {
        assert_eq!(
            Config::read("conflict_resolution: rebase\n")
                .unwrap()
                .conflict_resolution(),
            ConflictResolution::Rebase,
        );
        assert_eq!(
            Config::read("conflict_resolution: merge\n")
                .unwrap()
                .conflict_resolution(),
            ConflictResolution::Merge,
        );
    }

    /// And the whole point of the shape: nothing configured is a merge.
    ///
    /// An absent key, an absent file and one nothing can parse all say the same
    /// thing, because the alternative is a human who never found this section
    /// having their branch rewritten and force-pushed under whoever was reading
    /// it.
    #[test]
    fn a_conflict_nobody_has_said_anything_about_is_merged() {
        for text in [
            "",
            "git_author:\n  name: Tobias Cohen\n",
            "conflict_resolution:\n",
        ] {
            assert_eq!(
                Config::read(text).unwrap().conflict_resolution(),
                ConflictResolution::Merge,
                "nothing said here is a merge: {text:?}",
            );
        }

        let dir = tempfile::tempdir().unwrap();
        let settings = Settings::in_data_dir(dir.path());

        assert_eq!(
            settings.config().conflict_resolution(),
            ConflictResolution::Merge,
            "and so is a Data Directory with no config file in it at all",
        );

        std::fs::write(settings.config_path(), "conflict_resolution: [oh\n").unwrap();

        assert_eq!(
            settings.config().conflict_resolution(),
            ConflictResolution::Merge,
            "and so is a file nothing can parse",
        );
    }

    /// The word a save writes is one the next read understands, which is what
    /// the settings page depends on: it saves and then draws what came back.
    #[test]
    fn how_a_conflict_is_resolved_goes_through_the_file_and_comes_back() {
        let dir = tempfile::tempdir().unwrap();
        let settings = Settings::in_data_dir(dir.path());

        settings
            .save_config(&Config::of(
                GitAuthor::default(),
                RustBuildCache::default(),
                None,
                ConflictResolution::Rebase,
                vec![],
                vec![],
            ))
            .unwrap();

        assert_eq!(
            settings.config().conflict_resolution(),
            ConflictResolution::Rebase,
        );

        settings
            .save_config(&Config::of(
                GitAuthor::default(),
                RustBuildCache::default(),
                None,
                ConflictResolution::Merge,
                vec![],
                vec![],
            ))
            .unwrap();

        assert_eq!(
            settings.config().conflict_resolution(),
            ConflictResolution::Merge
        );
    }

    /// There is no default, and there could not be one: nobody but the human
    /// knows where their own site is, and a Verkstead that guessed would put a
    /// link to somebody else's page on a pull request.
    #[test]
    fn a_share_viewer_nobody_has_hosted_is_nowhere() {
        for text in [
            "",
            "git_author:\n  name: Tobias Cohen\n",
            "share_viewer_url:\n",
        ] {
            assert_eq!(
                Config::read(text).unwrap().share_viewer_url(),
                None,
                "for {text:?}"
            );
        }
    }

    #[test]
    fn a_config_file_that_says_nothing_configures_nobody() {
        for text in ["", "\n# nothing yet\n", "git_author:\n"] {
            let config = Config::read(text).unwrap();

            assert_eq!(config.git_author().name(), None, "for {text:?}");
            assert_eq!(config.git_author().email(), None, "for {text:?}");
        }
    }

    #[test]
    fn a_config_file_nothing_can_parse_leaves_a_session_startable() {
        let dir = tempfile::tempdir().unwrap();
        let settings = Settings::in_data_dir(dir.path());

        assert!(Config::read("git_author: [oh\n").is_err());

        std::fs::write(settings.config_path(), "git_author: [oh\n").unwrap();

        assert_eq!(settings.config().git_author().name(), None);
    }

    #[test]
    fn the_two_files_are_read_apart_from_one_another() {
        let dir = tempfile::tempdir().unwrap();
        let settings = Settings::in_data_dir(dir.path());

        std::fs::write(settings.secrets_path(), "github_token: ghp_thetoken\n").unwrap();

        assert_eq!(settings.secrets().github_token(), Some("ghp_thetoken"));
        assert_eq!(
            settings.config().git_author().name(),
            None,
            "a token configured is not an author configured"
        );

        std::fs::write(
            settings.config_path(),
            "git_author:\n  name: Tobias Cohen\n",
        )
        .unwrap();

        assert_eq!(settings.config().git_author().name(), Some("Tobias Cohen"));
        assert_eq!(settings.secrets().github_token(), Some("ghp_thetoken"));
    }

    #[test]
    fn a_saved_token_is_what_the_next_read_says() {
        let dir = tempfile::tempdir().unwrap();
        let settings = Settings::in_data_dir(dir.path());

        settings
            .save_secrets(&Secrets::of_token(Some("ghp_thetoken".to_owned())))
            .unwrap();

        assert_eq!(settings.secrets().github_token(), Some("ghp_thetoken"));
    }

    #[test]
    fn the_secrets_file_is_readable_by_nobody_else_on_the_machine() {
        let dir = tempfile::tempdir().unwrap();
        let settings = Settings::in_data_dir(dir.path());

        settings
            .save_secrets(&Secrets::of_token(Some("ghp_thetoken".to_owned())))
            .unwrap();

        let mode = std::fs::metadata(settings.secrets_path())
            .unwrap()
            .permissions()
            .mode();

        assert_eq!(mode & 0o777, 0o600, "the mode of the file holding a token");
    }

    #[test]
    fn a_file_somebody_left_world_readable_is_brought_to_0600_by_a_save() {
        let dir = tempfile::tempdir().unwrap();
        let settings = Settings::in_data_dir(dir.path());

        // A `secrets.yaml` written by hand, at whatever mode the human's umask
        // gave it — which is the ordinary way one exists before there is a
        // settings page to write it.
        std::fs::write(settings.secrets_path(), "github_token: by-hand\n").unwrap();
        std::fs::set_permissions(
            settings.secrets_path(),
            std::fs::Permissions::from_mode(0o644),
        )
        .unwrap();

        settings
            .save_secrets(&Secrets::of_token(Some("ghp_thetoken".to_owned())))
            .unwrap();

        let mode = std::fs::metadata(settings.secrets_path())
            .unwrap()
            .permissions()
            .mode();

        assert_eq!(mode & 0o777, 0o600);
    }

    #[test]
    fn clearing_the_token_leaves_a_file_that_configures_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let settings = Settings::in_data_dir(dir.path());

        settings
            .save_secrets(&Secrets::of_token(Some("ghp_thetoken".to_owned())))
            .unwrap();
        settings.save_secrets(&Secrets::of_token(None)).unwrap();

        assert_eq!(settings.secrets().github_token(), None);
        assert_eq!(
            std::fs::read_to_string(settings.secrets_path()).unwrap(),
            "",
            "a cleared token leaves the file, saying what a missing one says"
        );
    }

    #[test]
    fn a_token_that_was_only_whitespace_is_no_token_saved() {
        let dir = tempfile::tempdir().unwrap();
        let settings = Settings::in_data_dir(dir.path());

        settings
            .save_secrets(&Secrets::of_token(Some("   \n".to_owned())))
            .unwrap();

        assert_eq!(settings.secrets().github_token(), None);
    }

    #[test]
    fn a_pasted_token_keeps_none_of_the_whitespace_that_came_with_it() {
        let dir = tempfile::tempdir().unwrap();
        let settings = Settings::in_data_dir(dir.path());

        settings
            .save_secrets(&Secrets::of_token(Some(" ghp_thetoken\n".to_owned())))
            .unwrap();

        assert_eq!(settings.secrets().github_token(), Some("ghp_thetoken"));
    }

    #[test]
    fn a_saved_author_is_what_the_next_read_says() {
        let dir = tempfile::tempdir().unwrap();
        let settings = Settings::in_data_dir(dir.path());

        settings
            .save_config(&Config::of(
                GitAuthor::of(
                    Some("Tobias Cohen".to_owned()),
                    Some("tobi@tobico.net".to_owned()),
                ),
                RustBuildCache::default(),
                None,
                ConflictResolution::Merge,
                vec![],
                vec![],
            ))
            .unwrap();

        let config = settings.config();

        assert_eq!(config.git_author().name(), Some("Tobias Cohen"));
        assert_eq!(config.git_author().email(), Some("tobi@tobico.net"));
    }

    #[test]
    fn a_saved_share_viewer_url_is_what_the_next_read_says() {
        let dir = tempfile::tempdir().unwrap();
        let settings = Settings::in_data_dir(dir.path());

        settings
            .save_config(&Config::of(
                GitAuthor::default(),
                RustBuildCache::default(),
                Some("https://ada.github.io/shares/".to_owned()),
                ConflictResolution::Merge,
                vec![],
                vec![],
            ))
            .unwrap();

        assert_eq!(
            settings.config().share_viewer_url(),
            Some("https://ada.github.io/shares/")
        );

        // And clearing the field is how it is taken away, which is the whole of
        // what an empty one means.
        settings
            .save_config(&Config::of(
                GitAuthor::default(),
                RustBuildCache::default(),
                Some(String::new()),
                ConflictResolution::Merge,
                vec![],
                vec![],
            ))
            .unwrap();

        assert_eq!(settings.config().share_viewer_url(), None);
    }

    /// The reason the files are serialized rather than formatted by hand: a name
    /// with YAML's own punctuation in it has to come back as itself.
    #[test]
    fn an_author_whose_name_reads_as_markup_survives_the_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let settings = Settings::in_data_dir(dir.path());

        settings
            .save_config(&Config::of(
                GitAuthor::of(
                    Some("Cohen, Tobias: #1".to_owned()),
                    Some("tobi@tobico.net".to_owned()),
                ),
                RustBuildCache::default(),
                None,
                ConflictResolution::Merge,
                vec![],
                vec![],
            ))
            .unwrap();

        assert_eq!(
            settings.config().git_author().name(),
            Some("Cohen, Tobias: #1"),
        );
    }

    #[test]
    fn half_a_saved_author_is_the_half_that_was_filled_in() {
        let dir = tempfile::tempdir().unwrap();
        let settings = Settings::in_data_dir(dir.path());

        settings
            .save_config(&Config::of(
                GitAuthor::of(Some("Tobias Cohen".to_owned()), Some(String::new())),
                RustBuildCache::default(),
                None,
                ConflictResolution::Merge,
                vec![],
                vec![],
            ))
            .unwrap();

        let config = settings.config();

        assert_eq!(config.git_author().name(), Some("Tobias Cohen"));
        assert_eq!(config.git_author().email(), None);
    }

    #[test]
    fn there_is_no_written_time_until_something_has_been_written() {
        let dir = tempfile::tempdir().unwrap();
        let settings = Settings::in_data_dir(dir.path());

        assert!(settings.secrets_written_at().is_none());

        settings
            .save_secrets(&Secrets::of_token(Some("ghp_thetoken".to_owned())))
            .unwrap();

        assert!(settings.secrets_written_at().is_some());
    }

    /// The two files are written apart, as they are read apart: saving an author
    /// must not take a token away.
    #[test]
    fn saving_one_file_leaves_the_other_alone() {
        let dir = tempfile::tempdir().unwrap();
        let settings = Settings::in_data_dir(dir.path());

        settings
            .save_secrets(&Secrets::of_token(Some("ghp_thetoken".to_owned())))
            .unwrap();
        settings
            .save_config(&Config::of(
                GitAuthor::of(Some("Tobias Cohen".to_owned()), None),
                RustBuildCache::default(),
                None,
                ConflictResolution::Merge,
                vec![],
                vec![],
            ))
            .unwrap();

        assert_eq!(settings.secrets().github_token(), Some("ghp_thetoken"));
        assert_eq!(settings.config().git_author().name(), Some("Tobias Cohen"));
    }

    /// Nothing is left beside the file a save was about: the temporary it went
    /// through is renamed onto the settings file rather than left in the Data
    /// Directory.
    #[test]
    fn a_save_leaves_nothing_behind_it() {
        let dir = tempfile::tempdir().unwrap();
        let settings = Settings::in_data_dir(dir.path());

        settings
            .save_secrets(&Secrets::of_token(Some("ghp_thetoken".to_owned())))
            .unwrap();
        settings
            .save_config(&Config::of(
                GitAuthor::of(Some("Tobias Cohen".to_owned()), None),
                RustBuildCache::default(),
                None,
                ConflictResolution::Merge,
                vec![],
                vec![],
            ))
            .unwrap();

        let mut left: Vec<String> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        left.sort();

        assert_eq!(left, vec!["config.yaml", "secrets.yaml"]);
    }

    #[test]
    fn the_binds_are_what_the_config_file_says_in_the_order_it_says_them() {
        let config = Config::read(
            "sandbox_binds:\n  - /var/cache/verkstead-node\n  \
             - verkstead=/var/cache/verkstead-cargo\n",
        )
        .unwrap();

        assert_eq!(
            config.sandbox_binds(),
            [
                "/var/cache/verkstead-node",
                "verkstead=/var/cache/verkstead-cargo"
            ],
        );
    }

    #[test]
    fn a_file_with_no_binds_in_it_configures_none() {
        assert!(
            Config::read("git_author:\n  name: Ada\n")
                .unwrap()
                .sandbox_binds()
                .is_empty()
        );
        assert!(Config::read("").unwrap().sandbox_binds().is_empty());
    }

    /// A row emptied rather than deleted is a row the human took out, and one
    /// with a stray space around it is the path they meant.
    #[test]
    fn a_blank_bind_is_no_bind_and_a_padded_one_is_the_path_inside_it() {
        let config = Config::read("sandbox_binds:\n  - ''\n  - '  /var/cache  '\n  -\n").unwrap();

        assert_eq!(config.sandbox_binds(), ["/var/cache"]);
    }

    #[test]
    fn a_saved_bind_is_what_the_next_read_says() {
        let dir = tempfile::tempdir().unwrap();
        let settings = Settings::in_data_dir(dir.path());

        settings
            .save_config(&Config::of(
                GitAuthor::default(),
                RustBuildCache::default(),
                None,
                ConflictResolution::Merge,
                vec!["/var/cache/verkstead-node".to_owned()],
                vec![],
            ))
            .unwrap();

        assert_eq!(
            settings.config().sandbox_binds(),
            ["/var/cache/verkstead-node"]
        );

        // And a save that was told none takes the ones that were there away,
        // which is how the last one is deleted.
        settings
            .save_config(&Config::of(
                GitAuthor::default(),
                RustBuildCache::default(),
                None,
                ConflictResolution::Merge,
                vec![],
                vec![],
            ))
            .unwrap();

        assert!(settings.config().sandbox_binds().is_empty());
    }

    #[test]
    fn the_watched_paths_are_what_the_config_file_says() {
        let config = Config::read("watched_paths:\n  - /home/ada/src\n  - /srv/repos\n").unwrap();

        assert_eq!(config.watched_paths(), ["/home/ada/src", "/srv/repos"]);
    }

    /// A boundary nobody has widened, which is every path outside whatever the
    /// installation said — see [`crate::WatchedPaths`].
    #[test]
    fn a_file_with_no_watched_paths_in_it_says_none() {
        assert!(
            Config::read("git_author:\n  name: Ada\n")
                .unwrap()
                .watched_paths()
                .is_empty()
        );
        assert!(Config::read("").unwrap().watched_paths().is_empty());
    }

    #[test]
    fn a_blank_watched_path_is_no_path_and_a_padded_one_is_the_path_inside_it() {
        let config =
            Config::read("watched_paths:\n  - ''\n  - '  /home/ada/src  '\n  -\n").unwrap();

        assert_eq!(config.watched_paths(), ["/home/ada/src"]);
    }

    #[test]
    fn a_saved_watched_path_is_what_the_next_read_says() {
        let dir = tempfile::tempdir().unwrap();
        let settings = Settings::in_data_dir(dir.path());

        settings
            .save_config(&Config::of(
                GitAuthor::default(),
                RustBuildCache::default(),
                None,
                ConflictResolution::Merge,
                vec![],
                vec!["/home/ada/src".to_owned()],
            ))
            .unwrap();

        assert_eq!(settings.config().watched_paths(), ["/home/ada/src"]);
    }
}
