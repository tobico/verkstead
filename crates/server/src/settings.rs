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
//! cleanup:
//!   trim:
//!     enabled: true
//!     days: 3
//!   delete:
//!     enabled: false
//!     days: 30
//! conflict_resolution: merge
//! share_on_done: false
//! sandbox_binds:
//!   - /var/cache/verkstead-node
//!   - verkstead=/var/cache/verkstead-cargo
//! watched_paths:
//!   - /home/tobi/src
//! ignored_comments:
//!   - author: coderabbitai
//!     body: billing
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
//! `cleanup` is written that way as well, and it is the one section here whose
//! two halves fall back the two different ways: the trim is on at three days
//! with nothing said, because what it takes is what nobody opens twice, and the
//! delete is off at thirty, because it is the one thing in Verkstead that
//! forgets — see [`Cleanup`], and [`crate::cleanup`] for the sweep that reads
//! it on every pass.
//!
//! `conflict_resolution` is written that way too, and the default it falls back
//! to is the safe half of the choice: a conflicted pull request has its base
//! merged in rather than its branch rebased and force-pushed. One Repo may say
//! otherwise — that override is a fact about the Repo and lives in the store
//! beside it, not here.
//!
//! And `share_on_done` is written that way and defaults the other way about:
//! the three ways of saying nothing all mean **off**. The other defaults here
//! are the answer a human would have chosen anyway; this one publishes a gist
//! under their own account and comments on a pull request other people read,
//! and neither is a thing to do to somebody who has never been to the settings
//! page — see [`Config::share_on_done`].

use std::io::Write;
use std::path::{Path, PathBuf};

use regex::Regex;
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

    let mut options = std::fs::OpenOptions::new();

    options.write(true).create(true).truncate(true);
    no_more_readable_than(&mut options, mode);

    let mut file = options.open(&temp)?;

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

/// Create the file at `mode` — see [`SECRET_MODE`], which is the one that
/// matters.
#[cfg(unix)]
fn no_more_readable_than(options: &mut std::fs::OpenOptions, mode: u32) {
    use std::os::unix::fs::OpenOptionsExt;

    options.mode(mode);
}

/// And on Windows, where a file has no mode to be created at.
///
/// **What guards the token there is the directory, and it is worth saying what
/// that is and is not.** The Data Directory is `%APPDATA%\Verkstead` — inside
/// the account's own profile, which the operating system gives that account and
/// no other standard user, and which a file created inside inherits. So a
/// second person logged into the same machine cannot read the token, which is
/// what [`SECRET_MODE`] buys on Unix.
///
/// **What it does not buy is a narrower file than the directory it is in.** On
/// Unix the token's file is 0600 in a directory that is 0755, so a mode nobody
/// meant to widen is the only way it becomes readable; here it is exactly as
/// readable as the profile around it, and an administrator can read it as root
/// can on Unix. Narrowing it further would mean writing an access control list
/// by hand through the Win32 security API — a dependency and a body of code for
/// the difference between "this account" and "this account, and an
/// administrator who was already able to take ownership of it".
///
/// The `mode` is taken and dropped rather than not passed: it is what the
/// caller means, on the one platform that can say it, and a signature that
/// changed by platform would be a second thing to keep in step.
#[cfg(not(unix))]
fn no_more_readable_than(_options: &mut std::fs::OpenOptions, _mode: u32) {}

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

    /// And what the Cleanup does to an archived Conversation, and how long
    /// after the archiving it does it: the trim that takes the bulk, and the
    /// delete that takes the whole of it — see [`Cleanup`], and
    /// [`crate::cleanup`] for the sweep that reads this.
    ///
    /// Two switches and two durations, every one of them optional, and the two
    /// halves default the opposite ways about: a trim is on at three days
    /// because what it takes is what nobody opens twice, and a delete is off at
    /// thirty because it is the one thing here that forgets.
    #[serde(default)]
    cleanup: Cleanup,

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

    /// And whether a Conversation's record is shared to its pull request when
    /// the work settles to Done.
    ///
    /// Written the way the two above it are and defaulting the other way about:
    /// an absent key, an absent file and one nothing can parse all mean **off**.
    /// What the switch turns on writes to GitHub under the human's own account,
    /// which is not something to start doing to somebody who has never been to
    /// the settings page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    share_on_done: Option<bool>,

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

    /// And the comments Wrapping is never to address: a list of rules, each an
    /// optional regex over the author's login and an optional regex over the
    /// comment's body, matched anywhere in either.
    ///
    /// Rules combine with OR and a rule's own fields with AND: a comment is
    /// ignored where any one rule matches it, and a rule matches where every
    /// field it gives does. What it is for is a bot nobody can turn off — a
    /// review service filing the same word about billing on every pull request
    /// — where the alternative is a session spun up to address it each time.
    ///
    /// Read leniently, the way everything else in this file is, and with one
    /// refusal that is the reading's own rather than the settings page's: a
    /// rule giving neither field is dropped as it is read. A rule constraining
    /// nothing matches *everything*, so a hand-edit that left one behind would
    /// silence every comment on every pull request — which is the one way a
    /// misread of this file could take work away rather than leave it undone.
    /// A pattern that will not compile is kept exactly as it was written and
    /// matches nothing, with a line in the log — see [`IgnoreRule::matches`].
    #[serde(
        default,
        deserialize_with = "rules_written",
        skip_serializing_if = "Vec::is_empty"
    )]
    ignored_comments: Vec<IgnoreRule>,
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
            // Nothing to tidy on the way in: a switch is a switch, and a
            // duration that is not a whole number of days never became one —
            // see [`CleanupStep`].
            cleanup: config.cleanup,
            conflict_resolution: config.conflict_resolution,
            share_on_done: config.share_on_done,
            sandbox_binds: entries_written(config.sandbox_binds),
            watched_paths: entries_written(config.watched_paths),
            ignored_comments: rules_kept(config.ignored_comments),
        })
    }

    /// The config a settings page has just been told.
    ///
    /// One argument per section, which is what the file is: the page saves the
    /// whole of it in one request, so a constructor taking fewer would be one a
    /// caller could leave a section out of.
    #[allow(clippy::too_many_arguments)]
    pub fn of(
        git_author: GitAuthor,
        rust_build_cache: RustBuildCache,
        cleanup: Cleanup,
        conflict_resolution: ConflictResolution,
        share_on_done: bool,
        sandbox_binds: Vec<String>,
        watched_paths: Vec<String>,
        ignored_comments: Vec<IgnoreRule>,
    ) -> Config {
        Config {
            git_author,
            rust_build_cache,
            // As the page set it: both switches written down, and each duration
            // only where somebody typed one — see [`CleanupStep::of`], where an
            // empty box is the default asked for back rather than a duration of
            // nothing.
            cleanup,
            // Written down as it stands rather than left out where it is the
            // default, the way the build cache's switch is: what the page sends
            // is where the setting is to sit, and a key that appeared only for
            // one of the two answers would read as a file half-written.
            conflict_resolution: Some(conflict_resolution),
            // And the switch beside it, for the reason above it.
            share_on_done: Some(share_on_done),
            sandbox_binds: entries_written(sandbox_binds),
            watched_paths: entries_written(watched_paths),
            // Whole, and not put through the reading half's own drop above: what
            // reaches here has already been through [`IgnoreRule::trouble`] at
            // the endpoint, which refuses the rule the reading merely skips —
            // and dropping one here would be a save that quietly wrote fewer
            // rules than the page sent.
            ignored_comments,
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

    /// And what the Cleanup is to do after an archiving, which is a trim at
    /// three days and no delete at all where nobody has said otherwise.
    pub fn cleanup(&self) -> &Cleanup {
        &self.cleanup
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

    /// And whether the wrap-up shares the record to the pull request when the
    /// work settles to Done, which is **off** until somebody says otherwise.
    ///
    /// Read the way the two above it are: where the setting sits, rather than
    /// whether anybody has been here.
    pub fn share_on_done(&self) -> bool {
        self.share_on_done.unwrap_or(false)
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

    /// And the comments nothing is ever to be dispatched about, in the order
    /// they were written down. An empty list where nobody has added any, which
    /// is every comment on every pull request being somebody's to address.
    pub fn ignored_comments(&self) -> &[IgnoreRule] {
        &self.ignored_comments
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

/// What the Cleanup does to an archived Conversation, and how long after the
/// archiving it does it.
///
/// Two steps on two clocks, each counted from `archived_at` and neither waiting
/// on the other — see [`crate::cleanup`]. A **trim** takes the bulk: the full
/// agent output, the Transcripts and the session names, which is everything a
/// Share never carried. A **delete** takes the whole Conversation.
///
/// The two default the opposite ways about, and that is the whole shape of the
/// section. A trim is **on**, at [`crate::cleanup::TRIMMED_AFTER`] days: what it
/// takes is what nobody opens twice, and a human should not be keeping gigabytes
/// of session output for never having found this page. A delete is **off**, at
/// [`crate::cleanup::DELETED_AFTER`] days where it is turned on: it is the one
/// thing in Verkstead that forgets, and forgetting is not something to start
/// doing to somebody who has never said it should.
///
/// A delete sooner than a trim is not refused and is nothing to fix: the two
/// clocks are independent, so the Conversation is simply deleted before it was
/// ever trimmed, which is the reading of the two numbers a human typed.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Cleanup {
    #[serde(default)]
    trim: CleanupStep,

    #[serde(default)]
    delete: CleanupStep,
}

impl Cleanup {
    /// What a settings page has just been told: the two rows, in the order they
    /// are read down.
    pub fn of(trim: CleanupStep, delete: CleanupStep) -> Cleanup {
        Cleanup { trim, delete }
    }

    /// Whether an archived Conversation has its bulk taken. Nothing configured
    /// is **on**.
    pub fn trims(&self) -> bool {
        self.trim.enabled.unwrap_or(true)
    }

    /// And how many days after the archiving, which is
    /// [`crate::cleanup::TRIMMED_AFTER`] where nobody has typed one.
    pub fn trim_after(&self) -> u32 {
        self.trim.days.unwrap_or(crate::cleanup::TRIMMED_AFTER)
    }

    /// And the days exactly as they are written down, and `None` where nobody
    /// has written any: what a settings page draws as a placeholder rather than
    /// as a value somebody chose.
    pub fn trim_after_configured(&self) -> Option<u32> {
        self.trim.days
    }

    /// Whether an archived Conversation is deleted for good in the end. Nothing
    /// configured is **off**.
    pub fn deletes(&self) -> bool {
        self.delete.enabled.unwrap_or(false)
    }

    /// And how many days after the archiving, which is
    /// [`crate::cleanup::DELETED_AFTER`] where nobody has typed one.
    pub fn delete_after(&self) -> u32 {
        self.delete.days.unwrap_or(crate::cleanup::DELETED_AFTER)
    }

    /// And the days as they are written down, read the way the trim's are and
    /// drawn the same way.
    pub fn delete_after_configured(&self) -> Option<u32> {
        self.delete.days
    }
}

/// One of the Cleanup's two steps as the human left it: whether it happens, and
/// how long after the archiving.
///
/// Both halves optional and both absent on a machine nobody has been to the
/// settings page of, because what either of them falls back to is the *step's*
/// business rather than this type's — a trim and a delete are the same shape
/// and different answers, and [`Cleanup`] is where the two are told apart.
///
/// The days are a whole number of them and nothing else. A hand-edit that wrote
/// prose there leaves the duration unmade rather than the file unread, which is
/// this module's rule about everything it is told.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct CleanupStep {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    enabled: Option<bool>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    days: Option<u32>,
}

impl CleanupStep {
    /// What a settings page has just been told: the switch, and the days where
    /// a number was typed.
    ///
    /// An empty box is no duration configured, the way an empty build cache
    /// size is: clearing it is how the human asks for the default back. So is
    /// anything that is not a whole number of days — the page sends what was
    /// typed, and a duration nobody can read is a duration nobody set.
    pub fn of(enabled: bool, days: Option<String>) -> CleanupStep {
        CleanupStep {
            enabled: Some(enabled),
            days: days.and_then(days_typed),
        }
    }
}

/// The whole number of days a field holds, or `None` where it holds anything
/// else — an empty box, a space, a word, a fraction, a number of days nobody
/// could wait.
fn days_typed(days: String) -> Option<u32> {
    blank_is_nothing(days)?.parse().ok()
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

/// One class of comment nobody wants addressed: a regex over who wrote it, a
/// regex over what it says, or both.
///
/// Both halves are optional and each is a constraint only where it is given, so
/// a rule with an author and no body ignores everything that account writes and
/// one with a body and no author ignores that phrase from anybody. A rule that
/// gives neither would ignore every comment there is, which is why it is the
/// one thing here that is refused rather than read leniently — see
/// [`IgnoreRule::trouble`], and the [`Config::ignored_comments`] field for what
/// the reading half does with one that reached the file anyway.
///
/// The patterns are the regex crate's own syntax and are matched anywhere in
/// their text rather than against the whole of it: `billing` is what a human
/// means by *a comment about billing*, and an implicit anchor either side would
/// make the ordinary rule the surprising one. Case-sensitive, with `(?i)`
/// available at the front of a pattern for the human who wants otherwise.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct IgnoreRule {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    author: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    body: Option<String>,
}

impl IgnoreRule {
    /// The rule a settings page has just been told, each blank half no
    /// constraint at all — see [`blank_is_nothing`].
    pub fn of(author: Option<String>, body: Option<String>) -> IgnoreRule {
        IgnoreRule {
            author: author.and_then(blank_is_nothing),
            body: body.and_then(blank_is_nothing),
        }
    }

    /// The pattern the author's login is matched against, where one was given.
    pub fn author(&self) -> Option<&str> {
        self.author.as_deref()
    }

    /// And the one the comment's body is matched against.
    pub fn body(&self) -> Option<&str> {
        self.body.as_deref()
    }

    /// What would stop this rule being written down, or `None` where there is
    /// nothing wrong with it.
    ///
    /// The one refusal in either settings file, and it is here because the two
    /// ways a rule goes wrong are both ways it does something other than what
    /// was meant: a rule constraining nothing silences every comment, and a
    /// pattern that will not compile silences none while looking as though it
    /// does. Both are worth turning a save down over, where a bind naming a
    /// directory that is not there is not — that one is a row the human has yet
    /// to make, and this one is a row that cannot come right on its own.
    pub fn trouble(&self) -> Option<RuleTrouble> {
        if self.author.is_none() && self.body.is_none() {
            return Some(RuleTrouble::Empty);
        }

        if let Some(author) = self.author.as_deref()
            && let Err(error) = Regex::new(author)
        {
            return Some(RuleTrouble::Author(why(&error)));
        }

        if let Some(body) = self.body.as_deref()
            && let Err(error) = Regex::new(body)
        {
            return Some(RuleTrouble::Body(why(&error)));
        }

        None
    }

    /// Whether a comment by `author` reading `body` is one this rule ignores.
    ///
    /// Every field the rule gives has to match, and a field it does not give is
    /// no constraint — so a rule with both halves is narrower than either of
    /// them alone. A rule giving neither matches nothing here rather than
    /// everything: it is refused at the save and dropped at the read, and the
    /// one way to hold one is in memory somebody built by hand.
    ///
    /// A pattern that will not compile matches nothing, with a line in the log.
    /// That is this module's rule about the file it reads — a hand-edit nobody
    /// can parse leaves the setting unmade rather than refusing the read — and
    /// it fails in the safe direction: the comment goes on being somebody's to
    /// address, which is what would have happened with no rule at all.
    pub fn matches(&self, author: &str, body: &str) -> bool {
        match (self.author.as_deref(), self.body.as_deref()) {
            (None, None) => false,
            (rule_author, rule_body) => {
                rule_author.is_none_or(|pattern| found(pattern, author))
                    && rule_body.is_none_or(|pattern| found(pattern, body))
            }
        }
    }
}

/// What is wrong with a rule somebody tried to save.
///
/// Which of the two fields, for the pattern that would not compile: the page
/// draws the error at the box it is about, and a refusal that named the row and
/// not the field would leave the human reading both patterns to find out which.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleTrouble {
    /// It gives neither an author nor a body, so there is nothing it does not
    /// match.
    Empty,

    /// The author pattern is not a regex, in the engine's own words.
    Author(String),

    /// And the body pattern.
    Body(String),
}

/// Whether `pattern` is found anywhere in `text`, and `false` where it is not a
/// pattern at all.
///
/// Compiled here rather than held, because the rules are read fresh off the
/// file every time they are wanted — a rule added on a phone takes effect on
/// the next poll, and a compiled set held from startup would be one that did
/// not.
fn found(pattern: &str, text: &str) -> bool {
    match Regex::new(pattern) {
        Ok(regex) => regex.is_match(text),
        Err(error) => {
            tracing::warn!(
                pattern,
                error = %error,
                "an ignore rule's pattern is not a regex, so it ignores nothing"
            );

            false
        }
    }
}

/// A regex the engine would not take, in the words it refused it in, on one
/// line: the message is a small diagram of the pattern across three or four of
/// them, and what draws it is a box beside a text field on a phone.
fn why(error: &regex::Error) -> String {
    error
        .to_string()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// A list of rules as somebody left them, with the rows they emptied out taken
/// away.
///
/// Written for the reason [`rows_written`] is: a row with nothing after its `-`
/// is YAML's null, and a `Vec<IgnoreRule>` reading one would refuse the whole
/// file — which under this module's own rule would throw the author and the
/// build cache away over a half-deleted line.
fn rules_written<'de, D: serde::Deserializer<'de>>(rules: D) -> Result<Vec<IgnoreRule>, D::Error> {
    Ok(Vec::<Option<IgnoreRule>>::deserialize(rules)?
        .into_iter()
        .flatten()
        .collect())
}

/// A written list of rules with the blanks taken out of each and the ones that
/// came to nothing dropped.
///
/// The drop is the one place the reading half refuses anything, and it refuses
/// in the direction that leaves work to be done: a rule giving neither field
/// matches every comment there is, so a hand-edit that left one behind would
/// silence a whole pull request rather than merely failing to silence a bot.
fn rules_kept(rules: Vec<IgnoreRule>) -> Vec<IgnoreRule> {
    rules
        .into_iter()
        .map(|rule| IgnoreRule::of(rule.author, rule.body))
        .filter(|rule| rule.author.is_some() || rule.body.is_some())
        .collect()
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
    use super::{
        Cleanup, CleanupStep, Config, ConflictResolution, GitAuthor, IgnoreRule, RuleTrouble,
        RustBuildCache, Secrets, Settings,
    };

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
                Cleanup::default(),
                ConflictResolution::Rebase,
                false,
                vec![],
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
                Cleanup::default(),
                ConflictResolution::Merge,
                false,
                vec![],
                vec![],
                vec![],
            ))
            .unwrap();

        assert_eq!(
            settings.config().conflict_resolution(),
            ConflictResolution::Merge
        );
    }

    /// Whether Done shares the record to the pull request is the human's word
    /// for it too, and what they wrote is what comes back.
    #[test]
    fn sharing_on_done_is_what_the_config_file_says() {
        assert!(
            Config::read("share_on_done: true\n")
                .unwrap()
                .share_on_done()
        );
        assert!(
            !Config::read("share_on_done: false\n")
                .unwrap()
                .share_on_done()
        );
    }

    /// And the whole point of *its* shape, which is the other way about from
    /// the two above: nothing configured is off.
    ///
    /// An absent key, an absent file and one nothing can parse all say the same
    /// thing, because the alternative is a human who never found this switch
    /// having gists published under their account and comments left on pull
    /// requests other people are reading.
    #[test]
    fn sharing_nobody_has_said_anything_about_is_off() {
        for text in [
            "",
            "git_author:\n  name: Tobias Cohen\n",
            "share_on_done:\n",
        ] {
            assert!(
                !Config::read(text).unwrap().share_on_done(),
                "nothing said here is off: {text:?}",
            );
        }

        let dir = tempfile::tempdir().unwrap();
        let settings = Settings::in_data_dir(dir.path());

        assert!(
            !settings.config().share_on_done(),
            "and so is a Data Directory with no config file in it at all",
        );

        std::fs::write(settings.config_path(), "share_on_done: [oh\n").unwrap();

        assert!(
            !settings.config().share_on_done(),
            "and so is a file nothing can parse",
        );
    }

    /// The switch a save writes is the one the next read finds, which is what a
    /// setting surviving a restart amounts to: the file is read afresh every
    /// time, so a second reader of the same directory is what a restart is.
    #[test]
    fn sharing_on_done_goes_through_the_file_and_comes_back() {
        let dir = tempfile::tempdir().unwrap();
        let settings = Settings::in_data_dir(dir.path());

        settings
            .save_config(&Config::of(
                GitAuthor::default(),
                RustBuildCache::default(),
                Cleanup::default(),
                ConflictResolution::Merge,
                true,
                vec![],
                vec![],
                vec![],
            ))
            .unwrap();

        assert!(Settings::in_data_dir(dir.path()).config().share_on_done());

        settings
            .save_config(&Config::of(
                GitAuthor::default(),
                RustBuildCache::default(),
                Cleanup::default(),
                ConflictResolution::Merge,
                false,
                vec![],
                vec![],
                vec![],
            ))
            .unwrap();

        assert!(
            !Settings::in_data_dir(dir.path()).config().share_on_done(),
            "a switch that could only be turned on would be one nobody could undo",
        );
    }

    /// The Cleanup is four values in two rows, and this is that they are read.
    #[test]
    fn the_cleanup_is_what_the_config_file_says() {
        let config = Config::read(
            "cleanup:\n  trim:\n    enabled: false\n    days: 5\n  delete:\n    enabled: true\n    days: 90\n",
        )
        .unwrap();
        let cleanup = config.cleanup();

        assert!(!cleanup.trims());
        assert_eq!(cleanup.trim_after(), 5);
        assert!(cleanup.deletes());
        assert_eq!(cleanup.delete_after(), 90);
    }

    /// And the shape of the section, which is the one here whose two halves
    /// fall back the two different ways: nothing configured trims at three days
    /// and deletes never.
    ///
    /// An absent key, an absent file and one nothing can parse all say it. The
    /// trim is on for the reason the build cache is — a human should not be
    /// keeping gigabytes of session output for never having found this page —
    /// and the delete is off for the reason sharing on Done is: it is the one
    /// thing here that forgets.
    #[test]
    fn a_cleanup_nobody_has_said_anything_about_trims_and_never_deletes() {
        for text in ["", "git_author:\n  name: Tobias Cohen\n", "cleanup:\n"] {
            let config = Config::read(text).unwrap();
            let cleanup = config.cleanup();

            assert!(cleanup.trims(), "nothing said here trims: {text:?}");
            assert_eq!(cleanup.trim_after(), crate::cleanup::TRIMMED_AFTER);
            assert!(!cleanup.deletes(), "and deletes nothing: {text:?}");
            assert_eq!(cleanup.delete_after(), crate::cleanup::DELETED_AFTER);
        }

        let dir = tempfile::tempdir().unwrap();
        let settings = Settings::in_data_dir(dir.path());

        assert!(
            settings.config().cleanup().trims(),
            "and so is a Data Directory with no config file in it at all",
        );

        std::fs::write(settings.config_path(), "cleanup: [oh\n").unwrap();

        assert!(
            !settings.config().cleanup().deletes(),
            "and so is a file nothing can parse",
        );
    }

    /// A duration nobody has typed is the default *and says so*, which is what
    /// the page draws as a placeholder rather than as a value somebody chose.
    #[test]
    fn a_cleanup_duration_says_whether_anybody_chose_it() {
        let unset = Config::read("cleanup:\n  trim:\n    enabled: true\n").unwrap();

        assert_eq!(unset.cleanup().trim_after(), crate::cleanup::TRIMMED_AFTER);
        assert_eq!(unset.cleanup().trim_after_configured(), None);
        assert_eq!(unset.cleanup().delete_after_configured(), None);

        let typed =
            Config::read("cleanup:\n  trim:\n    days: 3\n  delete:\n    days: 30\n").unwrap();

        assert_eq!(
            typed.cleanup().trim_after_configured(),
            Some(3),
            "the same number a human typed is a number they typed",
        );
        assert_eq!(typed.cleanup().delete_after_configured(), Some(30));
    }

    /// And a duration that is not a whole number of days is nothing configured
    /// rather than anything to report — the page sends what was typed, and this
    /// module refuses nothing it is told.
    #[test]
    fn a_cleanup_duration_that_is_not_days_is_no_duration() {
        for days in ["", "   ", "a fortnight", "3.5", "-1"] {
            let step = CleanupStep::of(true, Some(days.to_owned()));
            let cleanup = Cleanup::of(step, CleanupStep::default());

            assert_eq!(
                cleanup.trim_after_configured(),
                None,
                "nothing readable in {days:?}",
            );
            assert_eq!(cleanup.trim_after(), crate::cleanup::TRIMMED_AFTER);
        }
    }

    /// The two rows a save writes are the ones the next read finds, a delete
    /// sooner than the trim included: the clocks run from the archiving
    /// independently, so there is nothing here to refuse.
    #[test]
    fn a_saved_cleanup_is_what_the_next_read_says() {
        let dir = tempfile::tempdir().unwrap();
        let settings = Settings::in_data_dir(dir.path());

        settings
            .save_config(&Config::of(
                GitAuthor::default(),
                RustBuildCache::default(),
                Cleanup::of(
                    CleanupStep::of(false, Some("14".to_owned())),
                    CleanupStep::of(true, Some("2".to_owned())),
                ),
                ConflictResolution::Merge,
                false,
                vec![],
                vec![],
                vec![],
            ))
            .unwrap();

        let config = Settings::in_data_dir(dir.path()).config();
        let cleanup = config.cleanup();

        assert!(!cleanup.trims());
        assert_eq!(cleanup.trim_after(), 14);
        assert!(cleanup.deletes());
        assert_eq!(
            cleanup.delete_after(),
            2,
            "a delete sooner than the trim is saved as it was typed",
        );
    }

    /// And a duration cleared is the default back, rather than a number of
    /// nothing written down: the field standing empty is how the human asks for
    /// it — see [`CleanupStep::of`].
    #[test]
    fn clearing_a_cleanup_duration_puts_the_default_back() {
        let dir = tempfile::tempdir().unwrap();
        let settings = Settings::in_data_dir(dir.path());

        settings
            .save_config(&Config::of(
                GitAuthor::default(),
                RustBuildCache::default(),
                Cleanup::of(
                    CleanupStep::of(true, Some(String::new())),
                    CleanupStep::of(false, Some("  ".to_owned())),
                ),
                ConflictResolution::Merge,
                false,
                vec![],
                vec![],
                vec![],
            ))
            .unwrap();

        let written = std::fs::read_to_string(settings.config_path()).unwrap();

        assert!(
            !written.contains("days"),
            "a duration nobody typed is not in the file: {written}"
        );

        let config = Settings::in_data_dir(dir.path()).config();

        assert_eq!(config.cleanup().trim_after(), crate::cleanup::TRIMMED_AFTER);
        assert_eq!(config.cleanup().trim_after_configured(), None);
    }

    /// Where the share viewer is hosted used to be said here, and a file
    /// written before it stopped being a setting still carries the key. It is
    /// read past like any other key this build has never heard of — the rest of
    /// the file is what the human configured, and refusing it would be a
    /// Verkstead that would not start for a line it no longer cares about.
    #[test]
    fn a_config_file_still_carrying_a_share_viewer_url_is_read_past_it() {
        let config = Config::read(
            "share_viewer_url: https://ada.github.io/shares/\ngit_author:\n  name: Tobias Cohen\n",
        )
        .unwrap();

        assert_eq!(config.git_author().name(), Some("Tobias Cohen"));
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

    /// On the platforms where a file has a mode to be written at — see
    /// [`super::no_more_readable_than`], which is where what Windows has
    /// instead is written down.
    #[cfg(unix)]
    #[test]
    fn the_secrets_file_is_readable_by_nobody_else_on_the_machine() {
        use std::os::unix::fs::PermissionsExt;

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

    /// The same platforms, for the same reason — see the test above.
    #[cfg(unix)]
    #[test]
    fn a_file_somebody_left_world_readable_is_brought_to_0600_by_a_save() {
        use std::os::unix::fs::PermissionsExt;

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
                Cleanup::default(),
                ConflictResolution::Merge,
                false,
                vec![],
                vec![],
                vec![],
            ))
            .unwrap();

        let config = settings.config();

        assert_eq!(config.git_author().name(), Some("Tobias Cohen"));
        assert_eq!(config.git_author().email(), Some("tobi@tobico.net"));
    }

    /// And the key goes when the file is next written: a save serializes the
    /// config as this build knows it, so the line the human never asked for
    /// stops being carried about forever.
    #[test]
    fn saving_drops_a_share_viewer_url_the_file_was_still_carrying() {
        let dir = tempfile::tempdir().unwrap();
        let settings = Settings::in_data_dir(dir.path());

        std::fs::write(
            settings.config_path(),
            "share_viewer_url: https://ada.github.io/shares/\n",
        )
        .unwrap();

        settings
            .save_config(&Config::of(
                GitAuthor::default(),
                RustBuildCache::default(),
                Cleanup::default(),
                ConflictResolution::Merge,
                false,
                vec![],
                vec![],
                vec![],
            ))
            .unwrap();

        let written = std::fs::read_to_string(settings.config_path()).unwrap();

        assert!(
            !written.contains("share_viewer_url"),
            "the key should be gone, not {written:?}"
        );
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
                Cleanup::default(),
                ConflictResolution::Merge,
                false,
                vec![],
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
                Cleanup::default(),
                ConflictResolution::Merge,
                false,
                vec![],
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
                Cleanup::default(),
                ConflictResolution::Merge,
                false,
                vec![],
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
                Cleanup::default(),
                ConflictResolution::Merge,
                false,
                vec![],
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
                Cleanup::default(),
                ConflictResolution::Merge,
                false,
                vec!["/var/cache/verkstead-node".to_owned()],
                vec![],
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
                Cleanup::default(),
                ConflictResolution::Merge,
                false,
                vec![],
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
                Cleanup::default(),
                ConflictResolution::Merge,
                false,
                vec![],
                vec!["/home/ada/src".to_owned()],
                vec![],
            ))
            .unwrap();

        assert_eq!(settings.config().watched_paths(), ["/home/ada/src"]);
    }

    #[test]
    fn the_ignore_rules_are_what_the_config_file_says() {
        let config = Config::read(
            "ignored_comments:\n  - author: coderabbitai\n    body: billing\n  - body: '^nit:'\n",
        )
        .unwrap();

        let rules = config.ignored_comments();

        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].author(), Some("coderabbitai"));
        assert_eq!(rules[0].body(), Some("billing"));
        assert_eq!(rules[1].author(), None);
        assert_eq!(rules[1].body(), Some("^nit:"));
    }

    #[test]
    fn a_file_with_no_ignore_rules_in_it_says_none() {
        assert!(
            Config::read("git_author:\n  name: Ada\n")
                .unwrap()
                .ignored_comments()
                .is_empty()
        );
        assert!(Config::read("").unwrap().ignored_comments().is_empty());
    }

    /// The one thing the reading half drops rather than keeps, and the reason is
    /// which way it fails: a rule constraining nothing matches every comment
    /// there is, so keeping one would silence a whole pull request.
    #[test]
    fn a_rule_that_constrains_nothing_is_not_a_rule() {
        let config = Config::read(
            "ignored_comments:\n  - author: ''\n    body: ''\n  -\n  - {}\n  - author: dependabot\n",
        )
        .unwrap();

        let rules = config.ignored_comments();

        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].author(), Some("dependabot"));
    }

    /// The other half of reading leniently: a pattern nothing can compile is
    /// kept as it was written, so the human can see it on the page and correct
    /// it, and it ignores nothing in the meantime.
    #[test]
    fn a_pattern_that_will_not_compile_is_kept_and_ignores_nothing() {
        let config = Config::read("ignored_comments:\n  - body: '[oh'\n").unwrap();

        let rules = config.ignored_comments();

        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].body(), Some("[oh"));
        assert!(!rules[0].matches("coderabbitai", "[oh"));
    }

    #[test]
    fn a_saved_ignore_rule_is_what_the_next_read_says() {
        let dir = tempfile::tempdir().unwrap();
        let settings = Settings::in_data_dir(dir.path());

        settings
            .save_config(&Config::of(
                GitAuthor::default(),
                RustBuildCache::default(),
                Cleanup::default(),
                ConflictResolution::Merge,
                false,
                vec![],
                vec![],
                vec![IgnoreRule::of(
                    Some("coderabbitai".to_owned()),
                    Some("billing".to_owned()),
                )],
            ))
            .unwrap();

        let config = settings.config();
        let rules = config.ignored_comments();

        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].author(), Some("coderabbitai"));
        assert_eq!(rules[0].body(), Some("billing"));
    }

    /// Every field the rule gives has to match, and one it does not give is no
    /// constraint at all.
    #[test]
    fn a_rule_matches_where_every_field_it_gives_does() {
        let both = IgnoreRule::of(Some("coderabbit".to_owned()), Some("billing".to_owned()));

        assert!(both.matches("coderabbitai[bot]", "your billing is not set up"));
        assert!(!both.matches("coderabbitai[bot]", "consider renaming this"));
        assert!(!both.matches("ada", "your billing is not set up"));

        let author_only = IgnoreRule::of(Some("coderabbit".to_owned()), None);

        assert!(author_only.matches("coderabbitai[bot]", "consider renaming this"));
        assert!(!author_only.matches("ada", "consider renaming this"));

        let body_only = IgnoreRule::of(None, Some("billing".to_owned()));

        assert!(body_only.matches("ada", "your billing is not set up"));
        assert!(!body_only.matches("ada", "consider renaming this"));
    }

    /// Anywhere in the text rather than the whole of it, and case-sensitive
    /// until the pattern says otherwise.
    #[test]
    fn a_pattern_is_found_anywhere_and_minds_its_case() {
        let rule = IgnoreRule::of(None, Some("billing".to_owned()));

        assert!(rule.matches("ada", "a word about billing, again"));
        assert!(!rule.matches("ada", "a word about Billing, again"));

        let either = IgnoreRule::of(None, Some("(?i)billing".to_owned()));

        assert!(either.matches("ada", "a word about Billing, again"));
    }

    /// What the settings page is refused over, which is the two ways a rule does
    /// something other than what was meant.
    #[test]
    fn a_rule_says_what_is_wrong_with_it() {
        assert_eq!(IgnoreRule::default().trouble(), Some(RuleTrouble::Empty));
        assert_eq!(
            IgnoreRule::of(Some("  ".to_owned()), Some(String::new())).trouble(),
            Some(RuleTrouble::Empty)
        );

        assert!(matches!(
            IgnoreRule::of(Some("[oh".to_owned()), None).trouble(),
            Some(RuleTrouble::Author(_))
        ));
        assert!(matches!(
            IgnoreRule::of(None, Some("[oh".to_owned())).trouble(),
            Some(RuleTrouble::Body(_))
        ));

        assert_eq!(
            IgnoreRule::of(Some("coderabbit".to_owned()), Some("billing".to_owned())).trouble(),
            None
        );
    }

    /// On one line, because what draws it is a box beside a text field on a
    /// phone and the engine's own message is a diagram across four.
    #[test]
    fn a_refused_pattern_is_reported_on_one_line() {
        let Some(RuleTrouble::Body(why)) = IgnoreRule::of(None, Some("[oh".to_owned())).trouble()
        else {
            panic!("a pattern that will not compile is a trouble to report");
        };

        assert!(!why.contains('\n'), "{why:?}");
        assert!(!why.is_empty());
    }
}
